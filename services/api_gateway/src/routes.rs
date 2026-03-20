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
}

// Helper funkcija za konverziju Reqwest status koda u Axum status kod
fn convert_status(reqwest_status: reqwest::StatusCode) -> StatusCode {
    StatusCode::from_u16(reqwest_status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

// 1. Proxy za Auth Servis - Registracija
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

    // Konvertujemo status ovde
    let status = convert_status(res.status());
    let body: Value = res.json().await.unwrap_or_default();
    Ok((status, Json(body)))
}

// 2. Proxy za Auth Servis - Login
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

    // Konvertujemo status ovde
    let status = convert_status(res.status());
    let body: Value = res.json().await.unwrap_or_default();
    Ok((status, Json(body)))
}

// 3. Orkestracija Analize Koda (Glavni Pipeline)
pub async fn analyze_code_handler(
    State(state): State<GatewayState>,
    Extension(claims): Extension<Claims>,
    Json(mut payload): Json<Value>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let analysis_job_id = Uuid::new_v4();
    tracing::info!("Starting analysis job {} for user {}", analysis_job_id, claims.sub);

    // Bezbedno dodavanje novog ključa u JSON objekat
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("analysis_job_id".to_string(), json!(analysis_job_id));
    } else {
        return Err((
            StatusCode::BAD_REQUEST, 
            Json(json!({"error": "Payload must be a valid JSON object"}))
        ));
    }
    
    let parser_url = env::var("PARSER_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8002".to_string());
    let analysis_url = env::var("ANALYSIS_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8003".to_string());
    let ranker_url = env::var("ML_RANKER_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8004".to_string());
    let suggest_url = env::var("SUGGESTION_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8005".to_string());

    // KORAK 1: Parser Service
    let parser_res = state.http_client.post(&format!("{}/parse", parser_url))
        .json(&payload)
        .send()
        .await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "Parser service failed"}))))?;
        
    if !parser_res.status().is_success() {
        // Konvertujemo status ovde
        let error_status = convert_status(parser_res.status());
        return Err((error_status, Json(parser_res.json().await.unwrap_or_default())));
    }

    // KORAK 2: Analysis Service
    let analyze_payload = json!({
        "analysis_job_id": analysis_job_id,
        "user_id": claims.sub
    });
    
    let analysis_res = state.http_client.post(&format!("{}/analyze", analysis_url))
        .json(&analyze_payload)
        .send()
        .await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "Analysis service failed"}))))?;

    if !analysis_res.status().is_success() {
        // Konvertujemo status ovde
        let error_status = convert_status(analysis_res.status());
        return Err((error_status, Json(analysis_res.json().await.unwrap_or_default())));
    }
    
    let analysis_data: Value = analysis_res.json().await.unwrap_or_default();
    let problems = &analysis_data["problems"];

    if problems.as_array().map_or(true, |arr| arr.is_empty()) {
        return Ok((StatusCode::OK, Json(json!({
            "analysis_job_id": analysis_job_id,
            "status": "perfect_code",
            "message": "Nisu pronađeni problemi u kodu!"
        }))));
    }

    // KORAK 3: ML Ranker Service
    let ranker_payload = json!({
        "analysis_job_id": analysis_job_id,
        "problems": problems
    });

    let ranker_res = state.http_client.post(&format!("{}/rank", ranker_url))
        .json(&ranker_payload)
        .send()
        .await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "Ranker service failed"}))))?;

    let ranker_data: Value = ranker_res.json().await.unwrap_or_default();
    let ranked_problems = &ranker_data["ranked_problems"];

    // KORAK 4: Suggestion Generator
    let suggest_payload = json!({
        "analysis_job_id": analysis_job_id,
        "problems": ranked_problems
    });

    let suggest_res = state.http_client.post(&format!("{}/suggest", suggest_url))
        .json(&suggest_payload)
        .send()
        .await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "Suggestion service failed"}))))?;

    let suggest_data: Value = suggest_res.json().await.unwrap_or_default();

    Ok((StatusCode::OK, Json(json!({
        "analysis_job_id": analysis_job_id,
        "summary": analysis_data,
        "ranked_issues": ranked_problems,
        "suggestions": suggest_data["suggestions"]
    }))))
}