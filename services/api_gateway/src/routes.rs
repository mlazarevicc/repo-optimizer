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
use mongodb::{bson::{doc, Bson, Binary}, Database};
use bson::spec::BinarySubtype;
use tempfile::tempdir;
use git2::Repository;

#[derive(Clone)]
pub struct GatewayState {
    pub http_client: Client,
    pub amqp_channel: lapin::Channel,
    pub db: Database,
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
    
    let mut amqp_payload = payload.clone();
    amqp_payload["analysis_job_id"] = json!(job_id);
    amqp_payload["user_id"] = json!(claims.sub);

    let status_coll = state.db.collection::<mongodb::bson::Document>("job_status");
    let uuid_bytes = job_id.into_bytes();
    status_coll.insert_one(doc! {
        "analysis_job_id": Binary { subtype: BinarySubtype::Generic, bytes: uuid_bytes.to_vec() },
        "status": "PROCESSING",
        "total_files": 1,
        "processed_files": 0
    }, None).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("DB error: {}", e)}))))?;

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
        Some(claims.sub),
        &state.amqp_channel
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))))?;

    let status_coll = state.db.collection::<mongodb::bson::Document>("job_status");
    let uuid_bytes = job_id.into_bytes();
    status_coll.insert_one(doc! {
        "analysis_job_id": Binary { subtype: BinarySubtype::Generic, bytes: uuid_bytes.to_vec() },
        "status": "PROCESSING",
        "total_files": files_sent as i32,
        "processed_files": 0
    }, None).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "DB error"}))))?;

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
    let dir = tempdir().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to create temp directory"}))))?;

    if let Some(field) = multipart.next_field().await.unwrap() {
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

    let status_coll = state.db.collection::<mongodb::bson::Document>("job_status");
    let uuid_bytes = job_id.into_bytes();
    status_coll.insert_one(doc! {
        "analysis_job_id": Binary { subtype: BinarySubtype::Generic, bytes: uuid_bytes.to_vec() },
        "status": "PROCESSING",
        "total_files": files_sent as i32,
        "processed_files": 0
    }, None).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "DB error"}))))?;

    Ok((StatusCode::ACCEPTED, Json(json!({ "analysis_job_id": job_id, "status": "PROCESSING", "files_queued": files_sent }))))
}

pub async fn get_results_handler(
    Path(job_id_str): Path<String>,
    State(state): State<GatewayState>,
    Extension(_claims): Extension<Claims>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let job_id = Uuid::parse_str(&job_id_str).map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid UUID"}))))?;
    let uuid_bytes = job_id.into_bytes();
    let query = doc! { "analysis_job_id": Binary { subtype: BinarySubtype::Generic, bytes: uuid_bytes.to_vec() } };

    let status_coll = state.db.collection::<mongodb::bson::Document>("job_status");
    let job_status_doc = status_coll.find_one(query.clone(), None).await.ok().flatten();
    
    let is_completed = job_status_doc.as_ref().map(|d| {
        let total = d.get_i32("total_files").unwrap_or(1);
        let processed = d.get_i32("processed_files").unwrap_or(0);
        processed >= total || d.get_str("status").unwrap_or("PROCESSING") == "COMPLETED"
    }).unwrap_or(false);

    if !is_completed {
        let msg = if let Some(d) = &job_status_doc {
            let total = d.get_i32("total_files").unwrap_or(1);
            let processed = d.get_i32("processed_files").unwrap_or(0);
            format!("Analyzing codebase... ({}/{}) files processed", processed, total)
        } else {
            "Initializing analysis...".to_string()
        };
        return Ok((StatusCode::OK, Json(json!({ "status": "PROCESSING", "message": msg }))));
    }

    let prob_coll = state.db.collection::<FetchedProblem>("problems");
    let sugg_coll = state.db.collection::<mongodb::bson::Document>("suggestions");

    let mut problems = Vec::new();
    if let Ok(mut cursor) = prob_coll.find(query.clone(), None).await {
        while let Some(Ok(p)) = cursor.next().await { problems.push(p); }
    }

    let mut suggestions = Vec::new();
    if let Ok(mut cursor) = sugg_coll.find(query.clone(), None).await {
        while let Some(Ok(doc)) = cursor.next().await {
            match mongodb::bson::from_document::<FetchedSuggestion>(doc) {
                Ok(s) => suggestions.push(s),
                Err(e) => tracing::error!("Failed to deserialize suggestion: {}", e),
            }
        }
    }

    let mut critical_count = 0; let mut high_count = 0; let mut medium_count = 0; let mut low_count = 0;
    let ranked_issues: Vec<Value> = problems.iter().map(|p| {
        let rank_score = match p.severity.to_lowercase().as_str() {
            "critical" => { critical_count += 1; 0.95 },
            "high" => { high_count += 1; 0.75 },
            "medium" => { medium_count += 1; 0.50 },
            _ => { low_count += 1; 0.25 },
        };
        json!({ "id": p.id, "problem_type": p.problem_type, "severity": p.severity, "line_start": p.line_start, "line_end": p.line_end, "message": p.message, "code_snippet": p.code_snippet, "rank_score": rank_score, "file_path": p.file_path.clone() })
    }).collect();

    let suggestions_json: Vec<Value> = suggestions.into_iter().map(|s| {
        json!({ "id": s.id.to_string(), "problem_id": s.problem_id.to_string(), "explanation": s.explanation, "original_code": s.original_code, "suggested_code": s.suggested_code, "impact_score": s.impact_score })
    }).collect();

    Ok((StatusCode::OK, Json(json!({
        "analysis_job_id": job_id,
        "status": "COMPLETED",
        "summary": { "critical_count": critical_count, "high_count": high_count, "medium_count": medium_count, "low_count": low_count, "total_problems": problems.len() },
        "ranked_issues": ranked_issues,
        "suggestions": suggestions_json
    }))))
}