use crate::detectors::Detector;
use crate::models::{ParsedAst, Problem, ProblemType, Severity};
use uuid::Uuid;

pub struct SmellDetector;

impl SmellDetector {
    pub fn new() -> Self {
        Self
    }

    fn extract_snippet(code: &str, line_start: usize, line_end: usize) -> String {
        code.lines()
            .skip(line_start.saturating_sub(1))
            .take(line_end - line_start + 1)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Detector for SmellDetector {
    fn detect(&self, data: &ParsedAst) -> Vec<Problem> {
        let mut problems = Vec::new();

        // 1. Long Method (> 50 LOC)
        for func in &data.functions {
            if func.lines_of_code > 50 {
                let severity = if func.lines_of_code > 100 {
                    Severity::High
                } else {
                    Severity::Medium
                };

                problems.push(Problem {
                    id: Uuid::new_v4(),
                    analysis_job_id: data.analysis_job_id,
                    problem_type: "code_smell.long_method".to_string(),
                    severity,
                    line_start: func.line_start,
                    line_end: func.line_end,
                    message: format!(
                        "Function '{}' is too long ({} lines). Consider breaking it into smaller functions.",
                        func.name, func.lines_of_code
                    ),
                    code_snippet: Self::extract_snippet(&data.code, func.line_start, func.line_end),
                    created_at: chrono::Utc::now(),
                });
            }
        }

        // 2. Long Parameter List (> 5 params)
        for func in &data.functions {
            if func.params_count > 5 {
                let severity = if func.params_count > 7 {
                    Severity::High
                } else {
                    Severity::Medium
                };

                problems.push(Problem {
                    id: Uuid::new_v4(),
                    analysis_job_id: data.analysis_job_id,
                    // problem_type: ProblemType::LongParameterList,
                    problem_type: "code_smell.long_parameter_list".to_string(),
                    severity,
                    line_start: func.line_start,
                    line_end: func.line_end,
                    message: format!(
                        "Function '{}' has too many parameters ({}). Consider using a parameter object.",
                        func.name, func.params_count
                    ),
                    code_snippet: Self::extract_snippet(&data.code, func.line_start, func.line_start),
                    created_at: chrono::Utc::now(),
                });
            }
        }

        // 3. Deep Nesting (> 4 levels)
        for func in &data.functions {
            if func.nesting_depth > 4 {
                let severity = if func.nesting_depth > 6 {
                    Severity::High
                } else {
                    Severity::Medium
                };

                problems.push(Problem {
                    id: Uuid::new_v4(),
                    analysis_job_id: data.analysis_job_id,
                    // problem_type: ProblemType::DeepNesting,
                    problem_type: "code_smell.deep_nesting".to_string(),
                    severity,
                    line_start: func.line_start,
                    line_end: func.line_end,
                    message: format!(
                        "Function '{}' has deep nesting ({} levels). Consider early returns or extracting logic.",
                        func.name, func.nesting_depth
                    ),
                    code_snippet: Self::extract_snippet(&data.code, func.line_start, func.line_end),
                    created_at: chrono::Utc::now(),
                });
            }
        }

        // 4. Complex Method (cyclomatic complexity > 10)
        for func in &data.functions {
            if func.cyclomatic_complexity > 10 {
                let severity = if func.cyclomatic_complexity > 20 {
                    Severity::High
                } else {
                    Severity::Medium
                };

                problems.push(Problem {
                    id: Uuid::new_v4(),
                    analysis_job_id: data.analysis_job_id,
                    // problem_type: ProblemType::ComplexMethod,
                    problem_type: "code_smell.complex_method".to_string(),
                    severity,
                    line_start: func.line_start,
                    line_end: func.line_end,
                    message: format!(
                        "Function '{}' is too complex (cyclomatic complexity: {}). Consider simplifying.",
                        func.name, func.cyclomatic_complexity
                    ),
                    code_snippet: Self::extract_snippet(&data.code, func.line_start, func.line_end),
                    created_at: chrono::Utc::now(),
                });
            }
        }

        // 5. Large Class (> 500 LOC or > 20 methods)
        for class in &data.classes {
            if class.lines_of_code > 500 || class.method_count > 20 {
                let severity = if class.lines_of_code > 1000 {
                    Severity::High
                } else {
                    Severity::Medium
                };

                problems.push(Problem {
                    id: Uuid::new_v4(),
                    analysis_job_id: data.analysis_job_id,
                    // problem_type: ProblemType::LargeClass,
                    problem_type: "code_smell.large_class".to_string(),
                    severity,
                    line_start: class.line_start,
                    line_end: class.line_end,
                    message: format!(
                        "Class '{}' is too large ({} lines, {} methods). Consider splitting into smaller classes.",
                        class.name, class.lines_of_code, class.method_count
                    ),
                    code_snippet: Self::extract_snippet(&data.code, class.line_start, class.line_end),
                    created_at: chrono::Utc::now(),
                });
            }
        }

        // 6. Long File (> 500 LOC)
        if data.metrics.code_lines > 500 {
            let severity = if data.metrics.code_lines > 1000 {
                Severity::High
            } else {
                Severity::Medium
            };

            problems.push(Problem {
                id: Uuid::new_v4(),
                analysis_job_id: data.analysis_job_id,
                // problem_type: ProblemType::LongFile,
                problem_type: "code_smell.long_file".to_string(),
                severity,
                line_start: 1,
                line_end: data.metrics.total_lines,
                message: format!(
                    "File is too long ({} lines of code). Consider splitting into multiple files.",
                    data.metrics.code_lines
                ),
                code_snippet: "".to_string(),
                created_at: chrono::Utc::now(),
            });
        }

        problems
    }
}

// use crate::detectors::Detector;
// use crate::models::{ParsedAst, Problem, Severity};
// use uuid::Uuid;

// pub struct SmellDetector;

// impl SmellDetector {
//     pub fn new() -> Self { Self }

//     fn extract_snippet(code: &str, line_start: usize, line_end: usize) -> String {
//         code.lines()
//             .skip(line_start.saturating_sub(1))
//             .take(line_end - line_start + 1)
//             .collect::<Vec<_>>()
//             .join("\n")
//     }
// }

// impl Detector for SmellDetector {
//     fn detect(&self, data: &ParsedAst) -> Vec<Problem> {
//         let mut problems = Vec::new();

//         // 1. Long Method
//         for func in &data.functions {
//             if func.lines_of_code > 50 {
//                 let severity = if func.lines_of_code > 100 { Severity::High } else { Severity::Medium };
//                 problems.push(Problem {
//                     id: Uuid::new_v4(),
//                     analysis_job_id: data.analysis_job_id,
//                     problem_type: "code_smell.long_method".to_string(),
//                     severity,
//                     line_start: func.line_start,
//                     line_end: func.line_end,
//                     message: format!("Function '{}' is too long ({} lines). Consider breaking it into smaller functions.", func.name, func.lines_of_code),
//                     code_snippet: Self::extract_snippet(&data.code, func.line_start, func.line_end),
//                     created_at: chrono::Utc::now(),
//                 });
//             }
//         }

//         // 2. Large Class
//         for class in &data.classes {
//             if class.lines_of_code > 200 || class.method_count > 10 {
//                 let severity = if class.lines_of_code > 500 { Severity::High } else { Severity::Medium };
//                 problems.push(Problem {
//                     id: Uuid::new_v4(),
//                     analysis_job_id: data.analysis_job_id,
//                     problem_type: "code_smell.large_class".to_string(),
//                     severity,
//                     line_start: class.line_start,
//                     line_end: class.line_end,
//                     message: format!("Class '{}' is too large ({} lines, {} methods). Consider splitting it.", class.name, class.lines_of_code, class.method_count),
//                     code_snippet: Self::extract_snippet(&data.code, class.line_start, class.line_end),
//                     created_at: chrono::Utc::now(),
//                 });
//             }
//         }

//         // 3. Long File
//         if data.metrics.code_lines > 500 {
//             let severity = if data.metrics.code_lines > 1000 { Severity::High } else { Severity::Medium };
//             problems.push(Problem {
//                 id: Uuid::new_v4(),
//                 analysis_job_id: data.analysis_job_id,
//                 problem_type: "code_smell.long_file".to_string(),
//                 severity,
//                 line_start: 1,
//                 line_end: data.metrics.total_lines,
//                 message: format!("File is too long ({} lines of code). Consider splitting into multiple files.", data.metrics.code_lines),
//                 code_snippet: "".to_string(),
//                 created_at: chrono::Utc::now(),
//             });
//         }

//         problems
//     }
// }