mod ml_engine;
mod models;
mod rabbitmq;

use axum::{routing::get, Json, Router};
use ml_engine::MLEngine;
use models::{ParsedAst, Problem};
use mongodb::{bson::doc, Client, Collection, bson::Binary, bson::spec::BinarySubtype};
use serde::Serialize;
use serde_json::Value;
use std::{env, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;


pub struct AppState {
    pub ml_engine: MLEngine,
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

    let actual_ast = match ast_doc {
        Some(ast) => ast,
        None => return Err(format!("AST not found for analysis_job_id: {}", analysis_job_id)),
    };

    let ranked_problems = state.ml_engine.rank_problems(problems, &actual_ast);

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

    let mongodb_url = env::var("MONGODB_URL")
        .unwrap_or_else(|_| "mongodb://repo_optimizer:dev_password@localhost:27017".into());
    
    tracing::info!("Connecting to MongoDB...");
    let db_client = Client::with_uri_str(&mongodb_url).await?;
    let mongodb = db_client.database("repo_optimizer");
    let parsed_asts = mongodb.collection::<ParsedAst>("parsed_asts");

    let ml_engine = ml_engine::MLEngine::new();
    tracing::info!("ML Ranker Service initialized");

    let state = Arc::new(AppState { 
        ml_engine,
        parsed_asts 
    });

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
    tracing::info!("ml_ranker_service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}