//! Analysis Service
//! 
//! Development Note: 
//! - Uses test user_id (00000000-0000-0000-0000-000000000001) until auth_service is integrated
//! - TODO: Replace with actual user_id from JWT token after auth implementation

mod detectors;
mod models;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use detectors::{
    performance::PerformanceDetector, 
    security::SecurityDetector, 
    smells::SmellDetector, 
    Detector
};
use models::{AnalyzeRequest, AnalyzeResponse, ParsedAst, Problem, Severity};
use mongodb::{
    bson::{doc, Binary, Bson},
    Client, 
    Collection
};
use bson::spec::BinarySubtype;
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: PgPool,
    parsed_asts: Collection<ParsedAst>,
    problems_collection: Collection<Problem>,
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
        service: "analysis_service",
    })
}

async fn analyze(
    State(state): State<AppState>,
    Json(payload): Json<AnalyzeRequest>,
) -> Result<Json<AnalyzeResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 1. Fetch parsed AST from MongoDB
    let uuid_bytes = payload.analysis_job_id.as_bytes();
    
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
        .map_err(|e| {
            tracing::error!("MongoDB query error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch parsed data".to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Analysis job not found".to_string(),
                }),
            )
        })?;

    // 2. Run detectors
    let smell_detector = SmellDetector::new();
    let performance_detector = PerformanceDetector::new();
    let security_detector = SecurityDetector::new();

    let mut all_problems: Vec<Problem> = Vec::new();
    all_problems.extend(smell_detector.detect(&parsed_ast));
    all_problems.extend(performance_detector.detect(&parsed_ast));
    all_problems.extend(security_detector.detect(&parsed_ast));

    // 3. Count by severity
    let critical_count = all_problems
        .iter()
        .filter(|p| matches!(p.severity, Severity::Critical))
        .count();
    let high_count = all_problems
        .iter()
        .filter(|p| matches!(p.severity, Severity::High))
        .count();
    let medium_count = all_problems
        .iter()
        .filter(|p| matches!(p.severity, Severity::Medium))
        .count();
    let low_count = all_problems
        .iter()
        .filter(|p| matches!(p.severity, Severity::Low))
        .count();

    tracing::info!(
        "Analysis complete: {} problems (Critical: {}, High: {}, Medium: {}, Low: {})",
        all_problems.len(),
        critical_count,
        high_count,
        medium_count,
        low_count
    );

    // 4. Store in databases with proper FK handling
    if !all_problems.is_empty() {
        // MongoDB: Store full problem data
        state
            .problems_collection
            .insert_many(&all_problems)
            .await
            .map_err(|e| {
                tracing::error!("MongoDB insert error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to store problems".to_string(),
                    }),
                )
            })?;

        // Postgres: Use transaction for atomicity
        let mut tx = state.db.begin().await.map_err(|e| {
            tracing::error!("Transaction begin error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database transaction error".to_string(),
                }),
            )
        })?;

        // TEST USER ID - za development pre implementacije auth sistema
        let test_user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")
            .expect("Invalid test user UUID");

        // Step 1: Insert or update analysis_jobs record
        sqlx::query(
            r#"
            INSERT INTO analysis_jobs (id, user_id, status, language, completed_at)
            VALUES ($1, $2, 'completed', $3, NOW())
            ON CONFLICT (id) DO UPDATE SET
                status = 'completed',
                completed_at = NOW()
            "#,
        )
        .bind(payload.analysis_job_id)
        .bind(test_user_id)
        .bind(parsed_ast.language.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Postgres analysis_jobs insert error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to store analysis job".to_string(),
                }),
            )
        })?;

        // Step 2: Insert problems metadata (FK will pass now)
        for problem in &all_problems {
            sqlx::query(
                r#"
                INSERT INTO problems (id, analysis_job_id, problem_type, severity, line_start, line_end)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(problem.id)
            .bind(problem.analysis_job_id)
            .bind(format!("{:?}", problem.problem_type))
            .bind(format!("{:?}", problem.severity))
            .bind(problem.line_start as i32)
            .bind(problem.line_end as i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Postgres problems insert error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to store problem metadata".to_string(),
                    }),
                )
            })?;
        }

        // Commit transaction
        tx.commit().await.map_err(|e| {
            tracing::error!("Transaction commit error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to commit transaction".to_string(),
                }),
            )
        })?;

        tracing::info!(
            "Successfully stored {} problems in both databases", 
            all_problems.len()
        );
    }

    let total_problems = all_problems.len();

    Ok(Json(AnalyzeResponse {
        analysis_job_id: payload.analysis_job_id,
        problems: all_problems,
        total_problems,
        critical_count,
        high_count,
        medium_count,
        low_count,
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

    let state = AppState {
        db,
        parsed_asts,
        problems_collection,
    };

    let port: u16 = env::var("ANALYSIS_SERVICE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8003);

    let app = Router::new()
        .route("/health", get(health))
        .route("/analyze", post(analyze))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("analysis_service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
