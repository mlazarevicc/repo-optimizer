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
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

// Uklonili smo #[derive(Clone)] jer stanje sada delimo preko Arc-a
pub struct AppState {
    pub db: PgPool,
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

// OVO JE NOVA FUNKCIJA KOJU POZIVA RABBITMQ RADNIK
pub async fn process_analysis(
    state: &Arc<AppState>,
    analysis_job_id: Uuid,
    user_id: Option<String>,
) -> Result<Vec<Value>, String> {
    
    let uuid_bytes = analysis_job_id.as_bytes();
    
    // 1. Fetch parsed AST from MongoDB
    let parsed_ast = state
        .parsed_asts
        .find_one(
            doc! {
                "analysis_job_id": Bson::Binary(Binary {
                    subtype: BinarySubtype::Generic,
                    bytes: uuid_bytes.to_vec(),
                })
            }
        )
        .await
        .map_err(|e| format!("MongoDB query error: {}", e))?
        .ok_or_else(|| "Analysis job not found".to_string())?;

    // 2. Run detectors
    let language_str = parsed_ast.language.to_string();
    let mut all_problems = match semgrep::run_scan(&parsed_ast.code, &language_str, analysis_job_id) {
        Ok(problems) => problems,
        Err(e) => {
            tracing::warn!("Semgrep failed, proceeding with AST detectors only. Error: {}", e);
            Vec::new() 
        }
    };

    // 3. Adding our AST Detectors for architectural "Code Smells", performance, security
    let smell_detector = SmellDetector::new();
    let performance_detector = PerformanceDetector::new();
    let security_detector = detectors::security::SecurityDetector::new();

    all_problems.extend(smell_detector.detect(&parsed_ast));
    all_problems.extend(performance_detector.detect(&parsed_ast));
    all_problems.extend(security_detector.detect(&parsed_ast));

    let critical_count = all_problems.iter().filter(|p| matches!(p.severity, Severity::Critical)).count();
    let high_count = all_problems.iter().filter(|p| matches!(p.severity, Severity::High)).count();
    let medium_count = all_problems.iter().filter(|p| matches!(p.severity, Severity::Medium)).count();
    let low_count = all_problems.iter().filter(|p| matches!(p.severity, Severity::Low)).count();

    tracing::info!(
        "Analysis complete: {} problems (Critical: {}, High: {}, Medium: {}, Low: {})",
        all_problems.len(), critical_count, high_count, medium_count, low_count
    );

    // 4. Store in databases with proper FK handling
    if !all_problems.is_empty() {
        state
            .problems_collection
            .insert_many(&all_problems)
            .await
            .map_err(|e| format!("MongoDB insert error: {}", e))?;

        let mut tx = state.db.begin().await.map_err(|e| format!("Transaction begin error: {}", e))?;

        let db_user_id = user_id
            .and_then(|id| Uuid::parse_str(&id).ok())
            .unwrap_or_else(|| Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap());

        sqlx::query(
            r#"
            INSERT INTO analysis_jobs (id, user_id, status, language, completed_at)
            VALUES ($1, $2, 'completed', $3, NOW())
            ON CONFLICT (id) DO UPDATE SET
                status = 'completed',
                completed_at = NOW()
            "#,
        )
        .bind(analysis_job_id)
        .bind(db_user_id)
        .bind(parsed_ast.language.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Postgres analysis_jobs insert error: {}", e))?;

        for problem in &all_problems {
            sqlx::query(
                r#"
                INSERT INTO problems (id, analysis_job_id, problem_type, severity, line_start, line_end)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(problem.id)
            .bind(problem.analysis_job_id)
            .bind(problem.problem_type.clone())
            .bind(format!("{:?}", problem.severity))
            .bind(problem.line_start as i32)
            .bind(problem.line_end as i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Postgres problems insert error: {}", e))?;
        }

        tx.commit().await.map_err(|e| format!("Transaction commit error: {}", e))?;
        tracing::info!("Successfully stored {} problems in databases", all_problems.len());
    }

    // 5. Konvertujemo pronadjene probleme u JSON kako bi ih RabbitMQ poslao dalje
    let problems_json: Vec<Value> = all_problems
        .into_iter()
        .map(|p| serde_json::to_value(p).unwrap())
        .collect();

    Ok(problems_json)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let mongodb_url = env::var("MONGODB_URL").expect("MONGODB_URL must be set");
    let client = Client::with_uri_str(&mongodb_url).await?;
    let mongodb = client.database("repo_optimizer");

    let parsed_asts = mongodb.collection::<ParsedAst>("parsed_asts");
    let problems_collection = mongodb.collection::<Problem>("problems");

    tracing::info!("Connected to databases");

    let state = Arc::new(AppState {
        db,
        parsed_asts,
        problems_collection,
    });

    // POKRETANJE RABBITMQ RADNIKA
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

    // Ostaje samo health ruta
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