use crate::models::{RankedProblemPayload, Suggestion};
use uuid::Uuid;
use chrono::Utc;

pub struct SuggestionEngine;

impl SuggestionEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn generate(&self, analysis_job_id: Uuid, problem: &RankedProblemPayload) -> Suggestion {
        let p_type = problem.problem_type.to_lowercase();

        let (explanation, suggested_code, impact_score) = if p_type.contains("secret") || p_type.contains("password") || p_type.contains("key") || p_type.contains("credential") {
            ( 
                "Hardcoded secrets (passwords, API keys) are a big security risk. Use environment variables or a secret management system.".to_string(), 
                "// Expose secrets via environment variables:\nlet secret = std::env::var(\"SECRET_KEY\").expect(\"Missing SECRET_KEY\");".to_string(), 
                95 
            )
        } else if p_type.contains("sql") || p_type.contains("injection") {
            ( 
                "Potential SQL injection detected. Always use prepared statements or parameterized queries instead of string concatenation.".to_string(), 
                "// Example of safe parameterized query:\nsqlx::query(\"SELECT * FROM users WHERE email = $1\").bind(email_input)".to_string(), 
                90 
            )
        } else if p_type.contains("xss") || p_type.contains("cross-site") {
             ( 
                "Cross-Site Scripting (XSS) vulnerability. User input must be sanitized before being injected into the DOM/HTML.".to_string(), 
                "// Use libraries that automatically escape HTML, or sanitize explicitly: \nlet safe_html = sanitize(user_input);".to_string(), 
                90 
            )
        } else if p_type.contains("long") || p_type.contains("complex") {
            ( 
                "This function is too long and violates the Single Responsibility principle. Try to separate the logic into smaller helper functions.".to_string(), 
                "// Extract the function parts into smaller ones:\nfn process_data() {\n    validate_input();\n    calculate_metrics();\n    save_results();\n}".to_string(), 
                60 
            )
        } else if p_type.contains("loop") || p_type.contains("nesting") {
            ( 
                "Nested loops or deep nesting drastically reduce performance and readability (O(n^2) or worse). Consider using Hash Maps or optimizing the algorithm.".to_string(), 
                "// Consider caching data in HashMap before main loop, or extracting inner loops into functions.".to_string(), 
                75 
            )
        } else if p_type.contains("query") || p_type.contains("unoptimized") {
            ( 
                "Unoptimized query logic found (e.g., N+1 query problem). Fetch required data in batches.".to_string(), 
                "// Try eager loading or fetching IDs in bulk before executing the operation.".to_string(), 
                70 
            )
        } else {
            ( 
                "This part of the code needs refactoring to improve quality, security, and readability.".to_string(), 
                "// Consider applying standard design patterns to this problem based on the provided warning.".to_string(), 
                50 
            )
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