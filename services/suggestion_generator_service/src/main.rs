mod suggestions;
mod models;
mod rabbitmq;

use axum::{routing::get, Json, Router};
use suggestions::SuggestionEngine;
use models::{RankedProblemPayload, Suggestion};
use mongodb::{Client, Collection};
use serde::Serialize;
use serde_json::Value;
use std::{env, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

pub struct AppState {
    pub suggestion_engine: SuggestionEngine,
    pub suggestions_collection: Collection<Suggestion>,
    pub job_status_collection: Collection<mongodb::bson::Document>
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "suggestion_generator_service",
    })
}

pub async fn process_suggestions(
    state: &Arc<AppState>,
    analysis_job_id: Uuid,
    problems_json: Vec<Value>,
) -> Result<(), String> {
    
    let mut ranked_problems: Vec<RankedProblemPayload> = Vec::new();
    for p in problems_json {
        if let Ok(problem) = serde_json::from_value::<RankedProblemPayload>(p) {
            ranked_problems.push(problem);
        }
    }

    if !ranked_problems.is_empty() {
        let mut suggestions = Vec::new();
        for problem in ranked_problems {
            let suggestion = state.suggestion_engine.generate(analysis_job_id, &problem);
            suggestions.push(suggestion);
        }

        if !suggestions.is_empty() {
            state
                .suggestions_collection
                .insert_many(&suggestions)
                .await
                .map_err(|e| format!("MongoDB insert error: {}", e))?;
        }
    }

    let query = mongodb::bson::doc! { "analysis_job_id": analysis_job_id.to_string() };
    let update = mongodb::bson::doc! { "$set": { "status": "COMPLETED" } };
    let _ = state.job_status_collection.update_one(query, update).await;

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
    let db_client = Client::with_uri_str(&mongodb_url).await?;
    let mongodb = db_client.database("repo_optimizer");
    let suggestions_collection = mongodb.collection::<Suggestion>("suggestions");
    let job_status_collection = mongodb.collection::<mongodb::bson::Document>("job_status");

    let suggestion_engine = SuggestionEngine::new();
    tracing::info!("Suggestion Generator Service initialized");

    let state = Arc::new(AppState { 
        suggestion_engine,
        suggestions_collection,
        job_status_collection,
    });

    let worker_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = rabbitmq::start_worker(worker_state).await {
            tracing::error!("RabbitMQ worker for Suggestion Generator crashed: {}", e);
        }
    });

    let port: u16 = env::var("SUGGESTION_SERVICE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8005);

    let app = Router::new()
        .route("/health", get(health))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("suggestion_generator_service HTTP listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}