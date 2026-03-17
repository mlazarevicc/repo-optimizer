use crate::detectors::Detector;
use crate::models::{ParsedAst, Problem, ProblemType, Severity};
use uuid::Uuid;

pub struct PerformanceDetector;

impl PerformanceDetector {
    pub fn new() -> Self {
        Self
    }
}

impl Detector for PerformanceDetector {
    fn detect(&self, data: &ParsedAst) -> Vec<Problem> {
        let mut problems = Vec::new();

        // Simple heuristic: detect nested loops (approximation)
        // Look for functions with high cyclomatic complexity + deep nesting
        for func in &data.functions {
            if func.cyclomatic_complexity > 15 && func.nesting_depth > 3 {
                problems.push(Problem {
                    id: Uuid::new_v4(),
                    analysis_job_id: data.analysis_job_id,
                    problem_type: ProblemType::NestedLoop,
                    severity: Severity::Medium,
                    line_start: func.line_start,
                    line_end: func.line_end,
                    message: format!(
                        "Function '{}' may contain nested loops. Consider optimizing algorithm complexity.",
                        func.name
                    ),
                    code_snippet: data.code.lines()
                        .skip(func.line_start.saturating_sub(1))
                        .take(func.line_end - func.line_start + 1)
                        .collect::<Vec<_>>()
                        .join("\n"),
                    created_at: chrono::Utc::now(),
                });
            }
        }

        // Pattern matching for common performance issues
        let code_lower = data.code.to_lowercase();

        // Check for potential N+1 query pattern (very naive)
        if code_lower.contains("for") && (code_lower.contains("query") || code_lower.contains("select")) {
            // Find approximate location
            let line_num = data.code.lines()
                .enumerate()
                .find(|(_, line)| line.to_lowercase().contains("for") && 
                                  (line.to_lowercase().contains("query") || line.to_lowercase().contains("select")))
                .map(|(idx, _)| idx + 1)
                .unwrap_or(1);

            problems.push(Problem {
                id: Uuid::new_v4(),
                analysis_job_id: data.analysis_job_id,
                problem_type: ProblemType::UnoptimizedQuery,
                severity: Severity::High,
                line_start: line_num,
                line_end: line_num,
                message: "Potential N+1 query detected. Consider using batch queries or eager loading.".to_string(),
                code_snippet: data.code.lines().nth(line_num - 1).unwrap_or("").to_string(),
                created_at: chrono::Utc::now(),
            });
        }

        problems
    }
}
