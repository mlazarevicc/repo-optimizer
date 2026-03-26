mod detectors;
mod models;
mod rabbitmq; 
mod semgrep;

use axum::{routing::get, Json, Router};
use detectors::{
    performance::PerformanceDetector, 
    security::SecurityDetector, 
    smells::SmellDetector, 
    Detector
};
use models::{ParsedAst, Problem, Severity};
use mongodb::{
    bson::{doc, Binary, Bson},
    Client, 
    Collection
};
use bson::spec::BinarySubtype;
use serde::Serialize;
use serde_json::Value;
use std::{env, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

pub struct AppState {
    pub parsed_asts: Collection<ParsedAst>,
    pub problems_collection: Collection<Problem>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "analysis_service",
    })
}

pub async fn process_analysis(
    state: &Arc<AppState>,
    analysis_job_id: Uuid,
    user_id: Option<String>,
) -> Result<Vec<Value>, String> {
    
    let uuid_bytes = analysis_job_id.into_bytes();
    let query = doc! { 
        "analysis_job_id": Binary { 
            subtype: BinarySubtype::Generic, 
            bytes: uuid_bytes.to_vec() 
        } 
    };

    let ast_doc = state
        .parsed_asts
        .find_one(query)
        .await
        .map_err(|e| format!("Database error while fetching AST: {}", e))?;

    let ast = match ast_doc {
        Some(doc) => doc,
        None => return Err(format!("AST not found for analysis_job_id: {}", analysis_job_id)),
    };

    let mut problems = Vec::new();

    let smell_detector = SmellDetector::new();
    problems.extend(smell_detector.detect(&ast));

    let perf_detector = PerformanceDetector::new();
    problems.extend(perf_detector.detect(&ast));

    let sec_detector = SecurityDetector::new();
    problems.extend(sec_detector.detect(&ast));

    if let Ok(semgrep_issues) = crate::semgrep::run_scan(&ast.code, &ast.language.to_string(), analysis_job_id) {
        problems.extend(semgrep_issues);
    }

    if !problems.is_empty() {
        state.problems_collection.insert_many(problems.clone()).await
            .map_err(|e| format!("Failed to save problems to DB: {}", e))?;
    }

    let result_json: Vec<Value> = problems
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

    let mongodb_url = env::var("MONGODB_URL").expect("MONGODB_URL must be set");
    let client = Client::with_uri_str(&mongodb_url).await?;
    let mongodb = client.database("repo_optimizer");

    let parsed_asts = mongodb.collection::<ParsedAst>("parsed_asts");
    let problems_collection = mongodb.collection::<Problem>("problems");

    tracing::info!("Connected to MongoDB");

    let state = Arc::new(AppState {
        parsed_asts,
        problems_collection,
    });

    let worker_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = rabbitmq::start_worker(worker_state).await {
            tracing::error!("RabbitMQ worker for analysis crashed: {}", e);
        }
    });

    let port: u16 = env::var("ANALYSIS_SERVICE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8003);

    let app = Router::new()
        .route("/health", get(health))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("analysis_service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}