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

#[derive(Clone)]
pub struct GatewayState {
    pub http_client: Client,
    pub amqp_channel: lapin::Channel,
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
        return Err((
            StatusCode::BAD_REQUEST, 
            Json(json!({"error": "Payload must be a valid JSON object"}))
        ));
    }

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