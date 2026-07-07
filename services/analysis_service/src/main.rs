mod detectors;
mod dedup;
mod models;
mod rabbitmq; 
mod semgrep;

use axum::{routing::get, Json, Router};
use detectors::{
    performance::PerformanceDetector, 
    security::SecurityDetector, 
    smells::SmellDetector, 
    duplication::DuplicationDetector,
    Detector
};
use models::{ParsedAst, Problem};
use mongodb::{
    bson::{doc, Binary},
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
use mongodb::options::{ClientOptions, IndexOptions};
use mongodb::IndexModel;
use std::time::Duration;

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
    file_path: Option<String>,
    _user_id: Option<String>,
    req_language: Option<String>,
) -> Result<Vec<Value>, String> {
    
    let uuid_bytes = analysis_job_id.into_bytes();
    let mut query = doc! { 
        "analysis_job_id": Binary { 
            subtype: BinarySubtype::Generic, 
            bytes: uuid_bytes.to_vec() 
        } 
    };
    
    if let Some(fp) = &file_path {
        query.insert("file_path", fp.clone());
    }

    let ast_doc = state
        .parsed_asts
        .find_one(query)
        .await
        .map_err(|e| format!("Database error while fetching AST: {}", e))?;

    let ast = match ast_doc {
        Some(doc) => doc,
        None => return Err(format!("AST not found for analysis_job_id: {}", analysis_job_id)),
    };

    let lang_str = req_language.unwrap_or_else(|| ast.language.to_string());
    let mut problems = Vec::new();

    let mut semgrep_problems = semgrep::run_scan(&ast.code, &lang_str, analysis_job_id, file_path.clone()).await?;
    problems.append(&mut semgrep_problems);

    let smell_detector = SmellDetector::new();
    problems.extend(smell_detector.detect(&ast));

    let perf_detector = PerformanceDetector::new();
    problems.extend(perf_detector.detect(&ast));

    let sec_detector = SecurityDetector::new();
    problems.extend(sec_detector.detect(&ast));

    let dup_detector = DuplicationDetector::new();
    problems.extend(dup_detector.detect(&ast));

    // Semgrep ("p/default") i nasi detektori (security/smells/performance) mogu prijaviti
    // ISTU stvar na istoj liniji (npr. hardkodovan password) - skloni duplikate pre upisa.
    let problems = dedup::dedupe_problems(problems);

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
    
    // 1. Konfiguracija Connection Pool-a i Timeout-a
    let mut client_options = ClientOptions::parse(&mongodb_url).await?;
    client_options.max_pool_size = Some(200); // Dozvoljavamo do 200 paralelnih konekcija
    client_options.min_pool_size = Some(10);
    client_options.connect_timeout = Some(Duration::from_secs(10));
    client_options.server_selection_timeout = Some(Duration::from_secs(10));
    client_options.retry_writes = Some(true); // Baza sama pokušava ponovo ako pukne
    client_options.retry_reads = Some(true);

    let client = Client::with_options(client_options)?;
    let mongodb = client.database("repo_optimizer");

    let parsed_asts = mongodb.collection::<ParsedAst>("parsed_asts");
    let _problems_collection = mongodb.collection::<Problem>("problems");

    // 2. KREIRANJE INDEKSA (Ključno za brzinu pretrage)
    let index_model = IndexModel::builder()
        .keys(doc! { "analysis_job_id": 1 }) // Pravimo brzi pretraživač za ovaj ID
        .options(IndexOptions::builder().background(true).build())
        .build();
        
    if let Err(e) = parsed_asts.create_index(index_model).await {
        tracing::warn!("Failed to create index for parsed_asts: {}", e);
    } else {
        tracing::info!("MongoDB Index for 'analysis_job_id' created/verified.");
    }

    let parsed_asts = mongodb.collection::<ParsedAst>("parsed_asts");
    let problems_collection = mongodb.collection::<Problem>("problems");

    tracing::info!("Connected to MongoDB");

    let state = Arc::new(AppState {
        parsed_asts,
        problems_collection,
    });

    let worker_state = state.clone();
    tokio::spawn(async move {
        let mut backoff = std::time::Duration::from_secs(2);
        loop {
            match rabbitmq::start_worker(worker_state.clone()).await {
                Ok(()) => {
                    tracing::warn!("RabbitMQ worker for analysis disconnected, reconnecting in {:?}...", backoff);
                }
                Err(e) => {
                    tracing::error!("RabbitMQ worker for analysis crashed: {} - reconnecting in {:?}...", e, backoff);
                }
            }
            tokio::time::sleep(backoff).await;
            backoff = std::cmp::min(backoff * 2, std::time::Duration::from_secs(30));
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