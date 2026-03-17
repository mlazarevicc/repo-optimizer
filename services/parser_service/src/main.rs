mod models;
mod parsers;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use models::{Language, ParsedAst, ParseRequest, ParseResponse};
use mongodb::{bson::doc, Client, Collection};
use serde::Serialize;
use std::env;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    parsed_asts: Collection<ParsedAst>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "parser_service",
    })
}

async fn parse_code(
    State(state): State<AppState>,
    Json(payload): Json<ParseRequest>,
) -> Result<Json<ParseResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Parse language
    let language = Language::from_str(&payload.language).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Unsupported language: {}", payload.language),
            }),
        )
    })?;

    // Get parser
    let mut parser = parsers::get_parser(&language);

    // Parse code
    let tree = parser.parse_code(&payload.code).map_err(|e| {
        tracing::error!("Parse error: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Parse error: {}", e),
            }),
        )
    })?;

    // Extract info
    let metrics = parser.calculate_metrics(&tree, &payload.code);
    let functions = parser.extract_functions(&tree, &payload.code);
    let classes = parser.extract_classes(&tree, &payload.code);
    let analysis_job_id = payload.analysis_job_id.unwrap_or_else(Uuid::new_v4);

    // Store in MongoDB
    let parsed_ast = ParsedAst {
        analysis_job_id,
        language: language.clone(),
        code: payload.code,
        metrics: metrics.clone(),
        functions: functions.clone(),
        classes: classes.clone(),
        created_at: chrono::Utc::now(),
    };

    state
        .parsed_asts
        .insert_one(&parsed_ast)
        .await
        .map_err(|e| {
            tracing::error!("MongoDB insert error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to store parsed data".to_string(),
                }),
            )
        })?;

    tracing::info!(
        "Parsed code: {} lines, {} functions, {} classes",
        metrics.total_lines,
        functions.len(),
        classes.len()
    );

    Ok(Json(ParseResponse {
        analysis_job_id,
        language,
        metrics,
        functions,
        classes,
        success: true,
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let mongodb_url = env::var("MONGODB_URL").expect("MONGODB_URL must be set");
    let client = Client::with_uri_str(&mongodb_url).await?;
    let db = client.database("repo_optimizer");
    let parsed_asts = db.collection::<ParsedAst>("parsed_asts");

    tracing::info!("Connected to MongoDB");

    let state = AppState { parsed_asts };

    let port: u16 = env::var("PARSER_SERVICE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8002);

    let app = Router::new()
        .route("/health", get(health))
        .route("/parse", post(parse_code))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("parser_service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
