use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
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
    NestedLoop,
    UnoptimizedQuery,
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
    pub problem_type: ProblemType,
    pub severity: Severity,
    pub line_start: usize,
    pub line_end: usize,
    pub message: String,
    pub code_snippet: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RankRequest {
    pub analysis_job_id: Uuid,
    pub problems: Vec<Problem>,
}

#[derive(Debug, Serialize)]
pub struct RankResponse {
    pub analysis_job_id: Uuid,
    pub ranked_problems: Vec<RankedProblem>,
    pub model_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedProblem {
    pub id: Uuid,
    pub problem_type: ProblemType,
    pub severity: Severity,
    pub rank_score: f64,
    pub line_start: usize,
    pub line_end: usize,
    pub message: String,
    pub code_snippet: String,
    pub feature_importance: Vec<f64>,
}

// Dummy ParsedAst za MVP
#[derive(Debug, Clone)]
pub struct ParsedAst {
    pub analysis_job_id: Uuid,
    pub language: Language,
    pub code: String,
    pub metrics: CodeMetrics,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Java,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub params_count: usize,
    pub lines_of_code: usize,
    pub cyclomatic_complexity: usize,
    pub nesting_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub method_count: usize,
    pub lines_of_code: usize,
}

#[derive(Debug, Clone)]
pub struct ProblemFeatures {
    pub loc: f64,
    pub cyclomatic_complexity: f64,
    pub nesting_depth: f64,
    pub params_count: f64,
    pub is_security: f64,
    pub is_performance: f64,
    pub file_size: f64,
    pub position: f64,
    pub severity_base: f64,
}
