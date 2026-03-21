mod ml_engine;
mod models;
mod rabbitmq;

use axum::{routing::get, Json, Router};
use ml_engine::MLEngine;
use models::{ParsedAst, Problem};
use serde::Serialize;
use serde_json::Value;
use std::{env, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

pub struct AppState {
    pub ml_engine: MLEngine,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "ml_ranker_service",
    })
}

pub async fn process_ranking(state: &Arc<AppState>, problems_json: Vec<Value>) -> Result<Vec<Value>, String> {
    tracing::info!("Ranking {} problems", problems_json.len());

    let mut problems: Vec<Problem> = Vec::new();
    for p in problems_json {
        match serde_json::from_value::<Problem>(p) {
            Ok(problem) => problems.push(problem),
            Err(e) => tracing::error!("Failed to parse problem in ML Ranker: {}", e),
        }
    }

    if problems.is_empty() {
        return Ok(vec![]);
    }

    let analysis_job_id = problems[0].analysis_job_id;

    let dummy_ast = ParsedAst {
        analysis_job_id,
        language: models::Language::Python,
        code: "".to_string(),
        metrics: models::CodeMetrics {
            total_lines: 100,
            code_lines: 80,
            comment_lines: 10,
            blank_lines: 10,
            total_functions: problems.len() as usize,
            total_classes: 0,
            max_nesting_depth: 5,
            cyclomatic_complexity: 15,
        },
        functions: vec![],
        classes: vec![],
    };

    let ranked_problems = state.ml_engine.rank_problems(problems, &dummy_ast);

    let result_json: Vec<Value> = ranked_problems
        .into_iter()
        .map(|p| serde_json::to_value(p).unwrap())
        .collect();

    Ok(result_json)
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

    let state = Arc::new(AppState { ml_engine });

    let worker_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = rabbitmq::start_worker(worker_state).await {
            tracing::error!("RabbitMQ worker for ML Ranker crashed: {}", e);
        }
    });

    let port: u16 = env::var("ML_RANKER_SERVICE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8004);

    let app = Router::new()
        .route("/health", get(health))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("ml_ranker_service HTTP listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}