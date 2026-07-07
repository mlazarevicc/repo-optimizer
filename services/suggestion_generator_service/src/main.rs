mod suggestions;
mod models;
mod rabbitmq;

use axum::{routing::get, Json, Router};
use suggestions::SuggestionEngine;
use models::{RankedProblemPayload, Suggestion};
use mongodb::{Client, Collection};
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;
use std::{env, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;
use mongodb::bson::{doc, Binary}; 
use mongodb::bson::spec::BinarySubtype;

pub struct AppState {
    pub suggestion_engine: SuggestionEngine,
    pub suggestions_collection: Collection<Suggestion>,
    // Status/napredak posla zivi u Postgres `analysis_jobs` (izvor istine - ima FK na users
    // pa moze da se proveri vlasnistvo); Mongo cuva samo same Suggestion/Problem podatke.
    pub pg_pool: PgPool,
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
    
    tracing::info!("Suggestion Service: Received {} problems for job {}", problems_json.len(), analysis_job_id);

    let mut ranked_problems: Vec<RankedProblemPayload> = Vec::new();
    for p in problems_json {
        match serde_json::from_value::<RankedProblemPayload>(p.clone()) {
            Ok(problem) => ranked_problems.push(problem),
            Err(e) => tracing::error!("Failed to deserialize problem from RabbitMQ: {}. Payload: {}", e, p),
        }
    }

    tracing::info!("Successfully parsed {} problems", ranked_problems.len());

    if !ranked_problems.is_empty() {
        let mut suggestion_docs = Vec::new();
        for problem in ranked_problems {
            let suggestion = state.suggestion_engine.generate(analysis_job_id, &problem);

            match mongodb::bson::to_document(&suggestion) {
                Ok(mut doc) => {
                    doc.insert("analysis_job_id", Binary {
                        subtype: BinarySubtype::Generic,
                        bytes: analysis_job_id.into_bytes().to_vec()
                    });

                    doc.insert("id", Binary {
                        subtype: BinarySubtype::Uuid,
                        bytes: suggestion.id.into_bytes().to_vec()
                    });

                    doc.insert("problem_id", Binary {
                        subtype: BinarySubtype::Uuid,
                        bytes: suggestion.problem_id.into_bytes().to_vec()
                    });

                    suggestion_docs.push(doc);
                },
                Err(e) => tracing::error!("Failed to convert suggestion to BSON document: {}", e),
            }
        }

        if !suggestion_docs.is_empty() {
            let suggestion_count = suggestion_docs.len();
            let raw_coll = state.suggestions_collection.clone_with_type::<mongodb::bson::Document>();
            if let Err(e) = raw_coll.insert_many(suggestion_docs).await {
                tracing::error!("Failed to save suggestion documents to MongoDB: {}", e);
                return Err(format!("Database error: {}", e));
            } else {
                tracing::info!("Successfully saved {} suggestions to DB!", suggestion_count);
            }
        }
    }

    // Azuriraj napredak posla u Postgres (izvor istine za status/vlasnistvo) umesto u Mongo.
    // Kad processed_files dostigne total_files, posao prelazi u 'completed'.
    let result = sqlx::query(
        r#"
        UPDATE analysis_jobs
        SET processed_files = processed_files + 1,
            status = CASE
                WHEN processed_files + 1 >= total_files THEN 'completed'
                ELSE 'processing'
            END,
            completed_at = CASE
                WHEN processed_files + 1 >= total_files THEN NOW()
                ELSE completed_at
            END
        WHERE id = $1
        "#
    )
    .bind(analysis_job_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| format!("Failed to update job status in Postgres: {}", e))?;

    if result.rows_affected() == 0 {
        tracing::warn!("analysis_jobs row not found for job {} (status update had no effect)", analysis_job_id);
    }

    tracing::info!("Incremented processed_files for job {}", analysis_job_id);


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

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pg_pool = PgPool::connect(&database_url).await?;
    tracing::info!("Connected to Postgres for job-status updates");

    let suggestion_engine = SuggestionEngine::new();
    tracing::info!("Suggestion Generator Service initialized");

    let state = Arc::new(AppState { 
        suggestion_engine,
        suggestions_collection,
        pg_pool,
    });

    let worker_state = state.clone();
    tokio::spawn(async move {
        let mut backoff = std::time::Duration::from_secs(2);
        loop {
            match rabbitmq::start_worker(worker_state.clone()).await {
                Ok(()) => {
                    tracing::warn!("RabbitMQ worker for Suggestion Generator disconnected, reconnecting in {:?}...", backoff);
                }
                Err(e) => {
                    tracing::error!("RabbitMQ worker for Suggestion Generator crashed: {} - reconnecting in {:?}...", e, backoff);
                }
            }
            tokio::time::sleep(backoff).await;
            backoff = std::cmp::min(backoff * 2, std::time::Duration::from_secs(30));
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
    tracing::info!("suggestion_generator_service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}