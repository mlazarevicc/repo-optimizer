use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};


#[derive(Debug, Deserialize)]
pub struct RankedProblemPayload {
    pub id: Uuid,
    pub problem_type: String,
    pub code_snippet: String,
}

#[derive(Debug, Deserialize)]
pub struct SuggestRequest {
    pub analysis_job_id: Uuid,
    pub problems: Vec<RankedProblemPayload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: Uuid,
    pub problem_id: Uuid,
    pub analysis_job_id: Uuid,
    pub problem_type: String,
    pub explanation: String,
    pub original_code: String,
    pub suggested_code: String,
    pub impact_score: u8,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SuggestResponse {
    pub analysis_job_id: Uuid,
    pub suggestions: Vec<Suggestion>,
    pub success: bool,
}