use axum::{
    extract::{Extension, State, Path, Multipart},
    http::StatusCode,
    Json,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use uuid::Uuid;
use crate::middleware::Claims;
use futures_lite::stream::StreamExt;
use mongodb::{bson::{doc, Binary}, Database};
use bson::spec::BinarySubtype;
use sqlx::PgPool;
use tempfile::tempdir;
use git2::Repository;

#[derive(Clone)]
pub struct GatewayState {
    pub http_client: Client,
    pub amqp_channel: lapin::Channel,
    pub db: Database,
    // Source-of-truth za status/vlasnistvo posla (vidi analysis_jobs u postgres_init.sql)
    pub pg_pool: PgPool,
    // Redis - kesiranje zavrsenih rezultata (TTL 1h) da se ne udara MongoDB na svaki GET
    pub redis_client: redis::Client,
}

#[derive(serde::Deserialize)]
struct FetchedProblem {
    id: Uuid,
    severity: String,
    problem_type: String,
    line_start: usize,
    line_end: usize,
    message: String,
    code_snippet: String,
    file_path: Option<String>,
    // Pravi ML rank_score koji upisuje ml_ranker_service nakon rangiranja.
    // Moze biti None ako iz nekog razloga rangiranje jos nije zavrseno.
    #[serde(default)]
    rank_score: Option<f64>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct FetchedSuggestion {
    id: mongodb::bson::Uuid, 
    problem_id: mongodb::bson::Uuid,
    explanation: String,
    original_code: String,
    suggested_code: String,
    impact_score: i32,
}

#[derive(serde::Deserialize)]
pub struct GitAnalyzeRequest {
    pub repo_url: String,
}

fn convert_status(reqwest_status: reqwest::StatusCode) -> StatusCode {
    StatusCode::from_u16(reqwest_status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn register_handler(
    State(state): State<GatewayState>,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let auth_url = env::var("AUTH_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8001".to_string());
    
    let res = state.http_client.post(&format!("{}/register", auth_url))
        .json(&payload)
        .send()
        .await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR, 
            Json(json!({"error": "Auth service is offline"}))
        ))?;

    let status = convert_status(res.status());
    let body: Value = res.json().await.unwrap_or_default();
    Ok((status, Json(body)))
}

pub async fn login_handler(
    State(state): State<GatewayState>,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let auth_url = env::var("AUTH_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8001".to_string());
    
    let res = state.http_client.post(&format!("{}/login", auth_url))
        .json(&payload)
        .send()
        .await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR, 
            Json(json!({"error": "Auth service is offline"}))
        ))?;

    let status = convert_status(res.status());
    let body: Value = res.json().await.unwrap_or_default();
    Ok((status, Json(body)))
}

pub async fn analyze_code_handler(
    State(state): State<GatewayState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let job_id = Uuid::new_v4();

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid user id in token"}))))?;

    let language = payload.get("language").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let file_name = payload.get("file_name").and_then(|v| v.as_str()).map(|s| s.to_string());

    let mut amqp_payload = payload.clone();
    amqp_payload["analysis_job_id"] = json!(job_id);
    amqp_payload["user_id"] = json!(claims.sub);

    sqlx::query(
        "INSERT INTO analysis_jobs (id, user_id, status, language, file_name, total_files, processed_files) \
         VALUES ($1, $2, 'processing', $3, $4, 1, 0)"
    )
    .bind(job_id)
    .bind(user_id)
    .bind(&language)
    .bind(&file_name)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("DB error: {}", e)}))))?;

    if let Err(e) = crate::rabbitmq::publish_job(&state.amqp_channel, &amqp_payload).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("RabbitMQ error: {}", e)}))));
    }

    Ok((StatusCode::ACCEPTED, Json(json!({
        "analysis_job_id": job_id,
        "status": "PROCESSING"
    }))))
}

pub async fn analyze_git_handler(
    State(state): State<GatewayState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GitAnalyzeRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let job_id = Uuid::new_v4();

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid user id in token"}))))?;

    let dir = tempdir().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to create temp directory"}))))?;
    let repo_url = payload.repo_url.clone();
    let path = dir.path().to_owned();

    let clone_result = tokio::task::spawn_blocking(move || {
        Repository::clone(&repo_url, &path)
    }).await.unwrap();

    if clone_result.is_err() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Failed to clone git repository. Ensure it is public."}))));
    }

    let files_sent = crate::scanner::scan_directory_and_publish(
        dir.path(),
        job_id,
        Some(claims.sub.clone()),
        &state.amqp_channel
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))))?;

    sqlx::query(
        "INSERT INTO analysis_jobs (id, user_id, status, language, file_name, total_files, processed_files) \
         VALUES ($1, $2, 'processing', 'mixed', $3, $4, 0)"
    )
    .bind(job_id)
    .bind(user_id)
    .bind(&payload.repo_url)
    .bind(files_sent as i32)
    .execute(&state.pg_pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "DB error"}))))?;

    Ok((StatusCode::ACCEPTED, Json(json!({
        "analysis_job_id": job_id,
        "status": "PROCESSING",
        "files_queued": files_sent
    }))))
}

pub async fn analyze_zip_handler(
    State(state): State<GatewayState>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let job_id = Uuid::new_v4();

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid user id in token"}))))?;

    let dir = tempdir().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to create temp directory"}))))?;
    let mut original_file_name: Option<String> = None;

    if let Some(field) = multipart.next_field().await.unwrap() {
        original_file_name = field.file_name().map(|s| s.to_string());
        let data = field.bytes().await.unwrap();
        let reader = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader).map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid ZIP"}))))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => dir.path().join(path),
                None => continue,
            };
            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath).unwrap();
            } else {
                if let Some(p) = outpath.parent() { std::fs::create_dir_all(p).unwrap(); }
                let mut outfile = std::fs::File::create(&outpath).unwrap();
                std::io::copy(&mut file, &mut outfile).unwrap();
            }
        }
    }

    let files_sent = crate::scanner::scan_directory_and_publish(dir.path(), job_id, Some(claims.sub), &state.amqp_channel).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))))?;

    sqlx::query(
        "INSERT INTO analysis_jobs (id, user_id, status, language, file_name, total_files, processed_files) \
         VALUES ($1, $2, 'processing', 'mixed', $3, $4, 0)"
    )
    .bind(job_id)
    .bind(user_id)
    .bind(original_file_name.unwrap_or_else(|| "upload.zip".to_string()))
    .bind(files_sent as i32)
    .execute(&state.pg_pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "DB error"}))))?;

    Ok((StatusCode::ACCEPTED, Json(json!({ "analysis_job_id": job_id, "status": "PROCESSING", "files_queued": files_sent }))))
}

#[derive(sqlx::FromRow)]
struct AnalysisJobRow {
    user_id: Uuid,
    status: String,
    total_files: i32,
    processed_files: i32,
}

pub async fn get_results_handler(
    Path(job_id_str): Path<String>,
    State(state): State<GatewayState>,
    Extension(claims): Extension<Claims>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let job_id = Uuid::parse_str(&job_id_str).map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid UUID"}))))?;
    let requester_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid user id in token"}))))?;

    let job_row: Option<AnalysisJobRow> = sqlx::query_as(
        "SELECT user_id, status, total_files, processed_files FROM analysis_jobs WHERE id = $1"
    )
    .bind(job_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("DB error: {}", e)}))))?;

    let job_row = match job_row {
        Some(row) => row,
        None => return Err((StatusCode::NOT_FOUND, Json(json!({"error": "Job not found"})))),
    };

    // Ownership provera - korisnik moze da vidi samo svoje poslove.
    if job_row.user_id != requester_id {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error": "You do not have access to this job"}))));
    }

    let is_completed = job_row.status == "completed" || job_row.processed_files >= job_row.total_files;

    if job_row.status == "failed" {
        return Ok((StatusCode::OK, Json(json!({ "status": "FAILED", "message": "Analysis failed" }))));
    }

    if !is_completed {
        let msg = format!("Analyzing codebase... ({}/{}) files processed", job_row.processed_files, job_row.total_files);
        return Ok((StatusCode::OK, Json(json!({ "status": "PROCESSING", "message": msg }))));
    }

    // Posao zavrsen — provi Redis pre MongoDB-a.
    // Kljuc: "results::{job_id}", TTL: 3600s (1h).
    // Best-effort: Redis greska = nastavljamo na MongoDB normalno.
    let cache_key = format!("results::{}", job_id);
    if let Ok(mut con) = state.redis_client.get_async_connection().await {
        let cached: redis::RedisResult<Option<String>> = redis::cmd("GET")
            .arg(&cache_key)
            .query_async(&mut con)
            .await;
        if let Ok(Some(json_str)) = cached {
            if let Ok(val) = serde_json::from_str::<Value>(&json_str) {
                tracing::debug!("Redis cache HIT za job {}", job_id);
                return Ok((StatusCode::OK, Json(val)));
            }
        }
    }
    tracing::debug!("Redis cache MISS za job {}", job_id);

    let uuid_bytes = job_id.into_bytes();
    let query = doc! { "analysis_job_id": Binary { subtype: BinarySubtype::Generic, bytes: uuid_bytes.to_vec() } };

    let prob_coll = state.db.collection::<FetchedProblem>("problems");
    let sugg_coll = state.db.collection::<mongodb::bson::Document>("suggestions");

    let mut problems = Vec::new();
    if let Ok(mut cursor) = prob_coll.find(query.clone()).await {
        while let Some(Ok(p)) = cursor.next().await { problems.push(p); }
    }

    let mut suggestions = Vec::new();
    if let Ok(mut cursor) = sugg_coll.find(query.clone()).await {
        while let Some(Ok(doc)) = cursor.next().await {
            match mongodb::bson::from_document::<FetchedSuggestion>(doc) {
                Ok(s) => suggestions.push(s),
                Err(e) => tracing::error!("Failed to deserialize suggestion: {}", e),
            }
        }
    }

    let mut critical_count = 0; let mut high_count = 0; let mut medium_count = 0; let mut low_count = 0;
    let ranked_issues: Vec<Value> = problems.iter().map(|p| {
        let fallback_score = match p.severity.to_lowercase().as_str() {
            "critical" => { 0.95 },
            "high" => { 0.75 },
            "medium" => { 0.50 },
            _ => { 0.25 },
        };
        match p.severity.to_lowercase().as_str() {
            "critical" => critical_count += 1,
            "high" => high_count += 1,
            "medium" => medium_count += 1,
            _ => low_count += 1,
        };
        let rank_score = p.rank_score.unwrap_or(fallback_score);
        json!({ "id": p.id, "problem_type": p.problem_type, "severity": p.severity, "line_start": p.line_start, "line_end": p.line_end, "message": p.message, "code_snippet": p.code_snippet, "rank_score": rank_score, "file_path": p.file_path.clone() })
    }).collect();

    let suggestions_json: Vec<Value> = suggestions.into_iter().map(|s| {
        json!({ "id": s.id.to_string(), "problem_id": s.problem_id.to_string(), "explanation": s.explanation, "original_code": s.original_code, "suggested_code": s.suggested_code, "impact_score": s.impact_score })
    }).collect();

    let response = json!({
        "analysis_job_id": job_id,
        "status": "COMPLETED",
        "summary": { "critical_count": critical_count, "high_count": high_count, "medium_count": medium_count, "low_count": low_count, "total_problems": problems.len() },
        "ranked_issues": ranked_issues,
        "suggestions": suggestions_json
    });

    // Upisi u Redis (best-effort — ne pucamo ako Redis nedostaje).
    if let Ok(json_str) = serde_json::to_string(&response) {
        if let Ok(mut con) = state.redis_client.get_async_connection().await {
            let _: redis::RedisResult<()> = redis::cmd("SETEX")
                .arg(&cache_key)
                .arg(3600u32)
                .arg(&json_str)
                .query_async(&mut con)
                .await;
        }
    }

    Ok((StatusCode::OK, Json(response)))
}

// ---------------------------------------------------------------------------
// GET /api/jobs — lista analiza ulogovanog korisnika
// ---------------------------------------------------------------------------
// Vraca sve analysis_jobs za trenutnog korisnika, sortirane od najnovijeg.
// Frontend koristi ovu rutu za prikaz historije analiza.
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct JobListItem {
    pub id: uuid::Uuid,
    pub status: String,
    pub language: String,
    pub file_name: Option<String>,
    pub total_files: i32,
    pub processed_files: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn list_jobs_handler(
    State(state): State<GatewayState>,
    Extension(claims): Extension<Claims>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid token"}))))?;

    let jobs: Vec<JobListItem> = sqlx::query_as(
        "SELECT id, status, language, file_name, total_files, processed_files,
                created_at, completed_at
         FROM analysis_jobs
         WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT 50"
    )
    .bind(user_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("DB error: {}", e)}))))?;

    let jobs_json: Vec<Value> = jobs.into_iter().map(|j| json!({
        "id": j.id,
        "status": j.status,
        "language": j.language,
        "file_name": j.file_name,
        "total_files": j.total_files,
        "processed_files": j.processed_files,
        "created_at": j.created_at,
        "completed_at": j.completed_at,
    })).collect();

    Ok((StatusCode::OK, Json(json!({ "jobs": jobs_json, "count": jobs_json.len() }))))
}

// ---------------------------------------------------------------------------
// DELETE /api/results/:job_id — GDPR brisanje rezultata analize
// ---------------------------------------------------------------------------
// Brise sve podatke vezane za jedan posao:
//   1. analysis_jobs row (Postgres) — CASCADE brise sve sto ima FK na njega
//   2. problems, suggestions, parsed_asts u MongoDB — manuelno (nema FK)
//   3. Redis cache — invalidira keširane rezultate
//
// Samo vlasnik posla moze da ga obrise (ista provjera kao u get_results_handler).
pub async fn delete_results_handler(
    Path(job_id_str): Path<String>,
    State(state): State<GatewayState>,
    Extension(claims): Extension<Claims>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let job_id = Uuid::parse_str(&job_id_str)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid UUID"}))))?;
    let requester_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid token"}))))?;

    // Provjera vlasnistva
    #[derive(sqlx::FromRow)]
    struct OwnerRow { user_id: Uuid }

    let row: Option<OwnerRow> = sqlx::query_as(
        "SELECT user_id FROM analysis_jobs WHERE id = $1"
    )
    .bind(job_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("DB error: {}", e)}))))?;

    match row {
        None => return Err((StatusCode::NOT_FOUND, Json(json!({"error": "Job not found"})))),
        Some(r) if r.user_id != requester_id =>
            return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Access denied"})))),
        _ => {}
    }

    // 1. Postgres — CASCADE automatski brise sve sa FK na analysis_jobs
    sqlx::query("DELETE FROM analysis_jobs WHERE id = $1")
        .bind(job_id)
        .execute(&state.pg_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("DB error: {}", e)}))))?;

    // 2. MongoDB — brisemo problems, suggestions, parsed_asts po analysis_job_id
    let uuid_bytes = job_id.into_bytes();
    let filter = doc! {
        "analysis_job_id": Binary {
            subtype: BinarySubtype::Generic,
            bytes: uuid_bytes.to_vec(),
        }
    };

    let _ = state.db.collection::<mongodb::bson::Document>("problems")
        .delete_many(filter.clone()).await;
    let _ = state.db.collection::<mongodb::bson::Document>("suggestions")
        .delete_many(filter.clone()).await;
    let _ = state.db.collection::<mongodb::bson::Document>("parsed_asts")
        .delete_many(filter).await;

    // 3. Redis — invalidiraj keširane rezultate
    let cache_key = format!("results::{}", job_id);
    if let Ok(mut con) = state.redis_client.get_async_connection().await {
        let _: redis::RedisResult<()> = redis::cmd("DEL")
            .arg(&cache_key)
            .query_async(&mut con)
            .await;
    }

    tracing::info!("Deleted all data for job {} (GDPR request by user {})", job_id, requester_id);

    Ok((StatusCode::OK, Json(json!({
        "message": "All data for this analysis job has been permanently deleted.",
        "job_id": job_id
    }))))
}
