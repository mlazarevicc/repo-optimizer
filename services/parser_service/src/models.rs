use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Java,
}

impl Language {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "python" | "py" => Some(Language::Python),
            "javascript" | "js" => Some(Language::JavaScript),
            "typescript" | "ts" => Some(Language::TypeScript),
            "rust" | "rs" => Some(Language::Rust),
            "java" => Some(Language::Java),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ParseRequest {
    pub code: String,
    pub language: String,
    pub analysis_job_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParseResponse {
    pub analysis_job_id: Uuid,
    pub language: Language,
    pub metrics: CodeMetrics,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeMetrics {
    pub total_lines: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
    pub total_functions: usize,
    pub total_classes: usize,
    pub max_nesting_depth: usize,
    pub cyclomatic_complexity: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionInfo {
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub params_count: usize,
    pub lines_of_code: usize,
    pub cyclomatic_complexity: usize,
    pub nesting_depth: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClassInfo {
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub method_count: usize,
    pub lines_of_code: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedAst {
    pub analysis_job_id: Uuid,
    pub language: Language,
    pub code: String,
    pub metrics: CodeMetrics,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub file_path: Option<String>,
}
