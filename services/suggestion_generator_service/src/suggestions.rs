use crate::models::{ProblemType, RankedProblemPayload, Suggestion};
use uuid::Uuid;
use chrono::Utc;

pub struct SuggestionEngine;

impl SuggestionEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn generate(&self, analysis_job_id: Uuid, problem: &RankedProblemPayload) -> Suggestion {
        let (explanation, suggested_code, impact_score) = match problem.problem_type {
            ProblemType::HardcodedSecret => ( 
                "Hardcoded secrets (passwords, API keys) are a big security risk. Use environment variables.".to_string(), 
                "let secret = std::env::var(\"SECRET_KEY\").expect(\"Missing SECRET_KEY\");".to_string(), 
                95 
            ), 
                ProblemType::SqlInjection => ( 
                "Potential SQL injection detected. Always use prepared statements instead of string concatenation.".to_string(), 
                "// Example (Rust/SQLx):\nsqlx::query(\"SELECT * FROM users WHERE email = $1\").bind(email_input)".to_string(), 
                90 
            ), 
                ProblemType::LongMethod => ( 
                "This function is too long and violates the Single Responsibility principle. Try to separate the logic into smaller helper functions.".to_string(), 
                "// Extract the function parts into smaller ones:\nfn process_data() {\n validate_input();\n calculate_metrics();\n save_results();\n}".to_string(), 
                60 
            ), 
                ProblemType::NestedLoop => ( 
                "Nested loops drastically reduce performance (O(n^2) or worse complexity). Consider using Hash Maps or optimizing the algorithm.".to_string(), 
                "// Consider caching data in HashMap before main loop".to_string(), 
                75 
            ), 
                _ => ( 
                "This part of the code needs refactoring to improve quality and readability.".to_string(), 
                "// Consider applying standard design patterns to this problem.".to_string(), 
                50 
            ),
        };

        Suggestion {
            id: Uuid::new_v4(),
            problem_id: problem.id,
            analysis_job_id,
            problem_type: problem.problem_type.clone(),
            explanation,
            original_code: problem.code_snippet.clone(),
            suggested_code,
            impact_score,
            created_at: Utc::now(),
        }
    }
}