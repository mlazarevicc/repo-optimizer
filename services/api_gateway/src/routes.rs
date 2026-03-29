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
}

#[derive(serde::Deserialize, serde::Serialize)]
struct FetchedSuggestion {
    id: Uuid,
    problem_id: Uuid,
    explanation: String,
    original_code: String,
    suggested_code: String,
    impact_score: u8,
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
    Json(mut payload): Json<Value>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let analysis_job_id = Uuid::new_v4();
    tracing::info!("Queuing async analysis job {} for user {}", analysis_job_id, claims.sub);

    if let Some(obj) = payload.as_object_mut() {
        obj.insert("analysis_job_id".to_string(), json!(analysis_job_id));
        obj.insert("user_id".to_string(), json!(claims.sub));
    } else {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Payload must be a valid JSON object"}))));
    }

    let status_coll = state.db.collection::<mongodb::bson::Document>("job_status");
    let _ = status_coll.insert_one(doc! {
        "analysis_job_id": analysis_job_id.to_string(),
        "status": "PROCESSING"
    }, None).await;

    match crate::rabbitmq::publish_job(&state.amqp_channel, &payload).await {
        Ok(_) => {
            Ok((StatusCode::ACCEPTED, Json(json!({
                "analysis_job_id": analysis_job_id,
                "status": "PROCESSING",
                "message": "The analysis was successfully queued for processing."
            }))))
        },
        Err(e) => {
            tracing::error!("Failed to publish job to RabbitMQ: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to queue the analysis job"}))
            ))
        }
    }
}

pub async fn analyze_git_handler(
    State(state): State<GatewayState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GitAnalyzeRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let job_id = Uuid::new_v4();
    
    let dir = tempdir().map_err(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to create temp directory"})))
    })?;

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

    Ok((StatusCode::ACCEPTED, Json(json!({
        "analysis_job_id": job_id,
        "status": "PROCESSING",
        "files_queued": files_sent,
        "message": format!("Successfully queued {} files for analysis.", files_sent)
    }))))
}

pub async fn analyze_zip_handler(
    State(state): State<GatewayState>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let job_id = Uuid::new_v4();
    let dir = tempdir().map_err(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to create temp directory"})))
    })?;

    if let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();
        let reader = std::io::Cursor::new(data);
        
        let mut archive = zip::ZipArchive::new(reader).map_err(|_| {
            (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid ZIP file format"})))
        })?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => dir.path().join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath).unwrap();
            } else {
                if let Some(p) = outpath.parent() {
                    std::fs::create_dir_all(p).unwrap();
                }
                let mut outfile = std::fs::File::create(&outpath).unwrap();
                std::io::copy(&mut file, &mut outfile).unwrap();
            }
        }
    } else {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "No file uploaded"}))));
    }

    let files_sent = crate::scanner::scan_directory_and_publish(
        dir.path(),
        job_id,
        Some(claims.sub),
        &state.amqp_channel
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))))?;

    Ok((StatusCode::ACCEPTED, Json(json!({
        "analysis_job_id": job_id,
        "status": "PROCESSING",
        "files_queued": files_sent,
        "message": format!("Successfully queued {} files for analysis.", files_sent)
    }))))
}

pub async fn get_results_handler(
    State(state): State<GatewayState>,
    Path(job_id_str): Path<String>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    
    let job_id = match Uuid::parse_str(&job_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid job_id format. Must be a valid UUID." })),
            ));
        }
    };

    let prob_coll = state.db.collection::<mongodb::bson::Document>("problems");
    let sugg_coll = state.db.collection::<mongodb::bson::Document>("suggestions");

    let uuid_bytes = job_id.into_bytes();
    let query = doc! { 
        "analysis_job_id": Binary { 
            subtype: BinarySubtype::Generic, 
            bytes: uuid_bytes.to_vec() 
        } 
    };

    let mut problems: Vec<FetchedProblem> = Vec::new();
    let mut suggestions: Vec<FetchedSuggestion> = Vec::new();

    match prob_coll.find(query.clone(), None).await {
        Ok(mut prob_cursor) => {
            while let Some(Ok(doc)) = prob_cursor.next().await {
                if let Ok(p) = mongodb::bson::from_document::<FetchedProblem>(doc) {
                    problems.push(p);
                }
            }
        }
        Err(e) => {
            tracing::error!("Database error while fetching problems: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" }))));
        }
    }

    if problems.is_empty() {
        return Ok((
            StatusCode::OK,
            Json(json!({
                "status": "PROCESSING",
                "message": "Job is still processing or no problems were found."
            })),
        ));
    }

    match sugg_coll.find(query.clone(), None).await {
        Ok(mut sugg_cursor) => {
            while let Some(Ok(doc)) = sugg_cursor.next().await {
                if let Ok(s) = mongodb::bson::from_document::<FetchedSuggestion>(doc) {
                    suggestions.push(s);
                }
            }
        }
        Err(e) => {
            tracing::error!("Database error while fetching suggestions: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" }))));
        }
    }

    if suggestions.len() < problems.len() {
        return Ok((
            StatusCode::OK,
            Json(json!({
                "status": "PROCESSING",
                "message": format!("Analysis completed. Generating fix suggestions... ({}/{})", suggestions.len(), problems.len())
            })),
        ));
    }

    let mut critical_count = 0;
    let mut high_count = 0;
    let mut medium_count = 0;
    let mut low_count = 0;

    let ranked_issues: Vec<Value> = problems.iter().map(|p| {
        let rank_score = match p.severity.to_lowercase().as_str() {
            "critical" => { critical_count += 1; 0.95 },
            "high" => { high_count += 1; 0.75 },
            "medium" => { medium_count += 1; 0.50 },
            _ => { low_count += 1; 0.25 },
        };
        
        json!({
            "id": p.id,
            "problem_type": p.problem_type,
            "severity": p.severity,
            "line_start": p.line_start,
            "line_end": p.line_end,
            "message": p.message,
            "code_snippet": p.code_snippet,
            "rank_score": rank_score
        })
    }).collect();

    let suggestions_json: Vec<Value> = suggestions.into_iter().map(|s| {
        json!({
            "id": s.id,
            "problem_id": s.problem_id,
            "explanation": s.explanation,
            "original_code": s.original_code,
            "suggested_code": s.suggested_code,
            "impact_score": s.impact_score
        })
    }).collect();

    Ok((
        StatusCode::OK,
        Json(json!({
            "analysis_job_id": job_id,
            "status": "COMPLETED",
            "summary": {
                "critical_count": critical_count,
                "high_count": high_count,
                "medium_count": medium_count,
                "low_count": low_count,
                "total_problems": problems.len()
            },
            "ranked_issues": ranked_issues,
            "suggestions": suggestions_json
        })),
    ))
}