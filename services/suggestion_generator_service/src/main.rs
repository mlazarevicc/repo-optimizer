mod models;
mod suggestions;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use models::{SuggestRequest, SuggestResponse, Suggestion};
use mongodb::{Client, Collection};
use serde::Serialize;
use std::{env, net::SocketAddr};
use suggestions::SuggestionEngine;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    suggestions_collection: Collection<Suggestion>,
    engine: std::sync::Arc<SuggestionEngine>,
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
        service: "suggestion_generator_service",
    })
}

async fn generate_suggestions(
    State(state): State<AppState>,
    Json(payload): Json<SuggestRequest>,
) -> Result<Json<SuggestResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("Generating suggestions for {} problems", payload.problems.len());

    let mut suggestions = Vec::new();

    for problem in payload.problems {
        let suggestion = state.engine.generate(payload.analysis_job_id, &problem);
        suggestions.push(suggestion);
    }

    // Čuvanje u MongoDB bazi
    if !suggestions.is_empty() {
        if let Err(e) = state.suggestions_collection.insert_many(&suggestions).await {
            tracing::error!("Failed to save suggestions to db: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "Database error".to_string() })
            ));
        }
    }

    Ok(Json(SuggestResponse {
        analysis_job_id: payload.analysis_job_id,
        suggestions,
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
    let suggestions_collection = db.collection::<Suggestion>("suggestions");

    tracing::info!("Connected to MongoDB");

    let state = AppState {
        suggestions_collection,
        engine: std::sync::Arc::new(SuggestionEngine::new()),
    };

    // Ispravljeno ime environment varijable (dodato 'C' u SERVICE)
    let port: u16 = env::var("SUGGESTION_GENERATOR_SERVICE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8005);

    let app = Router::new()
        .route("/health", get(health))
        .route("/suggest", post(generate_suggestions))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("suggestion_generator_service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}