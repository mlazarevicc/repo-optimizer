mod ml_engine;
mod models;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use ml_engine::MLEngine;
use models::{RankRequest, RankResponse, ParsedAst};
use serde::Serialize;
use std::env;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    ml_engine: MLEngine,
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
        service: "ml_ranker_service",
    })
}

async fn rank_problems(
    State(state): State<AppState>,
    Json(payload): Json<RankRequest>,
) -> Result<Json<RankResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("Ranking {} problems", payload.problems.len());

    // Dummy AST data (kasnije fetch iz MongoDB)
    let dummy_ast = ParsedAst {
        analysis_job_id: payload.analysis_job_id,
        language: models::Language::Python,
        code: "".to_string(),
        metrics: models::CodeMetrics {
            total_lines: 100,
            code_lines: 80,
            comment_lines: 10,
            blank_lines: 10,
            total_functions: payload.problems.len() as usize,
            total_classes: 0,
            max_nesting_depth: 5,
            cyclomatic_complexity: 15,
        },
        functions: vec![],
        classes: vec![],
    };

    let ranked_problems = state.ml_engine.rank_problems(payload.problems, &dummy_ast);

    Ok(Json(RankResponse {
        analysis_job_id: payload.analysis_job_id,
        ranked_problems,
        model_confidence: 0.92,
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

    let ml_engine = ml_engine::MLEngine::new();

    tracing::info!("ML Ranker Service initialized");

    let state = AppState { ml_engine };

    let port: u16 = env::var("ML_RANKER_SERVICE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8004);

    let app = Router::new()
        .route("/health", get(health))
        .route("/rank", post(rank_problems))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("ml_ranker_service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
