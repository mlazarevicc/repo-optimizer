mod models;
mod parsers;
mod rabbitmq;

use axum::{routing::get, Json, Router};
use models::{ParsedAst, ParseRequest};
use mongodb::{Client, Collection};
use serde::Serialize;
use std::{env, net::SocketAddr, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

pub struct AppState {
    pub parsed_asts: Collection<ParsedAst>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "parser_service",
    })
}

pub async fn process_and_save(state: &Arc<AppState>, payload: ParseRequest) -> Result<(), String> {
    let language = models::Language::from_str(&payload.language)
        .ok_or_else(|| format!("Unsupported language: {}", payload.language))?;

    let mut parser = parsers::get_parser(&language);
    
    let tree = parser.parse_code(&payload.code)?;
    let metrics = parser.calculate_metrics(&tree, &payload.code);
    let functions = parser.extract_functions(&tree, &payload.code);
    let classes = parser.extract_classes(&tree, &payload.code);

    let analysis_job_id = payload.analysis_job_id.unwrap_or_else(uuid::Uuid::new_v4);

    let parsed_ast = ParsedAst {
        analysis_job_id,
        language,
        code: payload.code.clone(),
        metrics: metrics.clone(),
        functions,
        classes,
        created_at: chrono::Utc::now(),
    };

    state.parsed_asts.insert_one(parsed_ast).await.map_err(|e| e.to_string())?;

    Ok(())
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

    // We use Arc (Atomically Reference Counted) to share state between the HTTP server and RabbitMQ workers
    let state = Arc::new(AppState { parsed_asts });

    let worker_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = rabbitmq::start_worker(worker_state).await {
            tracing::error!("RabbitMQ worker crashed: {}", e);
        }
    });

    let port: u16 = env::var("PARSER_SERVICE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8002);

    let app = Router::new()
        .route("/health", get(health))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("parser_service HTTP listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}