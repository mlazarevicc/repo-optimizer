use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Java,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Language::Python => write!(f, "python"),
            Language::JavaScript => write!(f, "javascript"),
            Language::TypeScript => write!(f, "typescript"),
            Language::Rust => write!(f, "rust"),
            Language::Java => write!(f, "java"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProblemType {
    LongMethod,
    LongParameterList,
    DeepNesting,
    LargeClass,
    DuplicateCode,
    ComplexMethod,
    UnusedVariable,
    MagicNumber,
    LongFile,
    // Performance
    NestedLoop,
    UnoptimizedQuery,
    // Security
    HardcodedSecret,
    SqlInjection,
    XssVulnerability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl Severity {
    pub fn to_score(&self) -> u8 {
        match self {
            Severity::Critical => 90,
            Severity::High => 70,
            Severity::Medium => 50,
            Severity::Low => 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub id: Uuid,
    pub analysis_job_id: Uuid,
    pub problem_type: String, 
    pub severity: Severity,
    pub line_start: usize,
    pub line_end: usize,
    pub message: String,
    pub code_snippet: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzeRequest {
    pub analysis_job_id: Uuid,
    #[serde(default)]
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeResponse {
    pub analysis_job_id: Uuid,
    pub problems: Vec<Problem>,
    pub total_problems: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
}

// From Parser Service
#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub params_count: usize,
    pub lines_of_code: usize,
    pub cyclomatic_complexity: usize,
    pub nesting_depth: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub method_count: usize,
    pub lines_of_code: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ParsedAst {
    pub analysis_job_id: Uuid,
    pub language: Language,
    pub code: String,
    pub metrics: CodeMetrics,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub file_path: Option<String>,
}
