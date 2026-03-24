use crate::detectors::Detector;
use crate::models::{ParsedAst, Problem, Severity};
use uuid::Uuid;

pub struct PerformanceDetector;

impl PerformanceDetector {
    pub fn new() -> Self { Self }
}

impl Detector for PerformanceDetector {
    fn detect(&self, data: &ParsedAst) -> Vec<Problem> {
        let mut problems = Vec::new();

        // 1. Nested Loops / High Complexity
        for func in &data.functions {
            if func.cyclomatic_complexity > 15 && func.nesting_depth > 3 {
                problems.push(Problem {
                    id: Uuid::new_v4(),
                    analysis_job_id: data.analysis_job_id,
                    problem_type: "performance.nested_loop".to_string(),
                    severity: Severity::Medium,
                    line_start: func.line_start,
                    line_end: func.line_end,
                    message: format!("Function '{}' has high complexity and deep nesting. Consider optimizing the algorithm.", func.name),
                    code_snippet: data.code.lines()
                        .skip(func.line_start.saturating_sub(1))
                        .take(func.line_end - func.line_start + 1)
                        .collect::<Vec<_>>()
                        .join("\n"),
                    created_at: chrono::Utc::now(),
                });
            }
        }

        // 2. Unoptimized Query Naive Check
        let code_lower = data.code.to_lowercase();
        if code_lower.contains("for") && (code_lower.contains("query") || code_lower.contains("select")) {
            let line_num = data.code.lines()
                .enumerate()
                .find(|(_, line)| line.to_lowercase().contains("for") && 
                                  (line.to_lowercase().contains("query") || line.to_lowercase().contains("select")))
                .map(|(idx, _)| idx + 1)
                .unwrap_or(1);

            problems.push(Problem {
                id: Uuid::new_v4(),
                analysis_job_id: data.analysis_job_id,
                problem_type: "performance.unoptimized_query".to_string(),
                severity: Severity::High,
                line_start: line_num,
                line_end: line_num,
                message: "Potential N+1 query detected in loop. Consider using batch queries or eager loading.".to_string(),
                code_snippet: data.code.lines().nth(line_num - 1).unwrap_or("").to_string(),
                created_at: chrono::Utc::now(),
            });
        }

        problems
    }
}