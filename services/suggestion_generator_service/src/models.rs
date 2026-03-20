use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProblemType {
    LongMethod, LongParameterList, DeepNesting, LargeClass, DuplicateCode,
    ComplexMethod, UnusedVariable, MagicNumber, LongFile, NestedLoop,
    UnoptimizedQuery, HardcodedSecret, SqlInjection, XssVulnerability,
}

#[derive(Debug, Deserialize)]
pub struct RankedProblemPayload {
    pub id: Uuid,
    pub problem_type: ProblemType,
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
    pub problem_type: ProblemType,
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