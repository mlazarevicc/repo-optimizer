use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Jezik fajla koji je analiziran. Dolazi iz ml_ranker_service
/// koji ga prosledjuje sa svakim problemom, a izvorno dolazi iz parser_service AST-a.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Java,
    Go,
    #[serde(other)]
    Unknown,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
            Language::Rust => "rust",
            Language::Java => "java",
            Language::Go => "go",
            Language::Unknown => "unknown",
        }
    }

    /// Za prikaz u predlogu - "Python", "JavaScript"...
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::Python => "Python",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Rust => "Rust",
            Language::Java => "Java",
            Language::Go => "Go",
            Language::Unknown => "Generic",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RankedProblemPayload {
    pub id: Uuid,
    pub problem_type: String,
    pub code_snippet: String,
    /// Jezik fajla - prenosi se iz ml_ranker-a da bismo mogli da generisemo
    /// language-specific predloge. Nije bio prisutan ranije, sto je znacilo
    /// da svi predlozi prikazu isti genericni kod bez obzira na jezik.
    #[serde(default = "default_language")]
    pub language: Language,
}

fn default_language() -> Language { Language::Unknown }

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct SuggestResponse {
    pub analysis_job_id: Uuid,
    pub suggestions: Vec<Suggestion>,
    pub success: bool,
}