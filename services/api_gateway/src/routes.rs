use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use uuid::Uuid;
use crate::middleware::Claims;
use axum::extract::Path;
use futures_lite::stream::StreamExt;
use mongodb::{bson::{doc, Bson, Binary}, Database};
use bson::spec::BinarySubtype;

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

pub async fn get_results_handler(
    State(state): State<GatewayState>,
    Extension(_claims): Extension<Claims>,
    Path(job_id): Path<Uuid>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    
    let status_coll = state.db.collection::<mongodb::bson::Document>("job_status");
    if let Ok(Some(status_doc)) = status_coll.find_one(doc! { "analysis_job_id": job_id.to_string() }, None).await {
        if status_doc.get_str("status").unwrap_or("") == "PROCESSING" {
            tracing::info!("Job still processing in background. Returning PROCESSING.");
            return Ok((StatusCode::OK, Json(json!({
                "status": "PROCESSING",
                "message": "Semgrep security analysis is in progress, please wait..."
            }))));
        }
    }
    
    let uuid_bytes = job_id.as_bytes();
    let query = doc! {
        "analysis_job_id": Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: uuid_bytes.to_vec(),
        })
    };

    let ast_coll = state.db.collection::<mongodb::bson::Document>("parsed_asts");
    if ast_coll.find_one(query.clone(), None).await.unwrap_or(None).is_none() {
        return Ok((StatusCode::OK, Json(json!({
            "status": "PROCESSING",
            "message": "The code is being loaded and parsed..."
        }))));
    }

    let mut problems = Vec::new();
    let prob_coll = state.db.collection::<mongodb::bson::Document>("problems");
    let mut cursor = prob_coll.find(query.clone(), None).await.unwrap();
    
    let mut critical_count = 0; let mut high_count = 0;
    let mut medium_count = 0; let mut low_count = 0;

    while let Some(Ok(doc)) = cursor.next().await {
        if let Ok(p) = mongodb::bson::from_document::<FetchedProblem>(doc) {
            match p.severity.to_lowercase().as_str() {
                "critical" => critical_count += 1,
                "high" => high_count += 1,
                "medium" => medium_count += 1,
                _ => low_count += 1,
            }
            problems.push(p);
        }
    }

    let mut suggestions = Vec::new();
    let sugg_coll = state.db.collection::<mongodb::bson::Document>("suggestions");
    let mut sugg_cursor = sugg_coll.find(query.clone(), None).await.unwrap();
    
    while let Some(Ok(doc)) = sugg_cursor.next().await {
        if let Ok(s) = mongodb::bson::from_document::<FetchedSuggestion>(doc) {
            suggestions.push(s);
        }
    }

    if !problems.is_empty() && suggestions.len() < problems.len() {
        return Ok((StatusCode::OK, Json(json!({
            "status": "PROCESSING",
            "message": "Analysis in progress, generating fix suggestions..."
        }))));
    }

    let ranked_issues: Vec<Value> = problems.into_iter().map(|p| {
        let rank_score = match p.severity.to_lowercase().as_str() {
            "critical" => 0.95, "high" => 0.75, "medium" => 0.50, _ => 0.25,
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

    Ok((StatusCode::OK, Json(json!({
        "analysis_job_id": job_id,
        "status": "COMPLETED",
        "summary": {
            "critical_count": critical_count,
            "high_count": high_count,
            "medium_count": medium_count,
            "low_count": low_count,
            "total_problems": ranked_issues.len()
        },
        "ranked_issues": ranked_issues,
        "suggestions": suggestions
    }))))
}