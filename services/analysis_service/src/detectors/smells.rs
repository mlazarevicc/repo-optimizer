use crate::detectors::Detector;
use crate::models::{ParsedAst, Problem, ProblemType, Severity};
use regex::Regex;
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
                    file_path: data.file_path.clone(),
                    language: data.language.to_string(),
                    autofix: None,
                    rank_score: None,
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
                    file_path: data.file_path.clone(),
                    language: data.language.to_string(),
                    autofix: None,
                    rank_score: None,
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
                    file_path: data.file_path.clone(),
                    language: data.language.to_string(),
                    autofix: None,
                    rank_score: None,
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
                    file_path: data.file_path.clone(),
                    language: data.language.to_string(),
                    autofix: None,
                    rank_score: None,
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
                    file_path: data.file_path.clone(),
                    language: data.language.to_string(),
                    autofix: None,
                    rank_score: None,
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
                file_path: data.file_path.clone(),
                language: data.language.to_string(),
                autofix: None,
                rank_score: None,
            });
        }

        // 7. Magic Numbers - brojevni literali van "named constant" konteksta.
        // Heuristika: 0/1/-1/2 su uobicajeni i nisu magic (indeksi, koraci petlje);
        // brojevi unutar string literala se ignorisu (navodnici se skidaju pre provere);
        // linije koje izgledaju kao deklaracija SCREAMING_CASE konstante se preskacu
        // (vrednost je vec imenovana, to je tacno suprotno od "magic number" problema).
        let const_decl_re = Regex::new(r"^[A-Z_][A-Z0-9_]*\s*[:=]").unwrap();
        let string_strip_re = Regex::new(r#""[^"]*"|'[^']*'"#).unwrap();
        // \b\d{2,}\b - celi brojevi sa 2+ cifara (10, 42, 500...); jednocifreni (0-9)
        // se NE hvataju da ne zatrpamo nalazima svaki "for i in range(5)".
        // \b\d+\.\d+\b - bilo koji float (i 0.1, 3.14) - manje "ocigledni" od malih celih.
        let number_re = Regex::new(r"\b\d+\.\d+\b|\b\d{2,}\b").unwrap();

        for (idx, line) in data.code.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with('#')
                || trimmed.starts_with("//")
                || trimmed.starts_with('*')
                || trimmed.starts_with("/*")
            {
                continue;
            }
            if const_decl_re.is_match(trimmed) {
                continue;
            }
            let stripped = string_strip_re.replace_all(trimmed, "\"\"");
            if number_re.is_match(&stripped) {
                problems.push(Problem {
                    id: Uuid::new_v4(),
                    analysis_job_id: data.analysis_job_id,
                    problem_type: "code_smell.magic_number".to_string(),
                    severity: Severity::Low,
                    line_start: idx + 1,
                    line_end: idx + 1,
                    message: "Magic number detected. Consider extracting it into a named constant for clarity.".to_string(),
                    code_snippet: line.to_string(),
                    created_at: chrono::Utc::now(),
                    file_path: data.file_path.clone(),
                    language: data.language.to_string(),
                    autofix: None,
                    rank_score: None,
                });
            }
        }

        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ClassInfo, CodeMetrics, FunctionInfo, Language};

    fn empty_metrics() -> CodeMetrics {
        CodeMetrics {
            total_lines: 10,
            code_lines: 10,
            comment_lines: 0,
            blank_lines: 0,
            total_functions: 1,
            total_classes: 0,
            max_nesting_depth: 0,
            cyclomatic_complexity: 0,
        }
    }

    fn make_function(overrides: FunctionInfo) -> ParsedAst {
        ParsedAst {
            analysis_job_id: Uuid::new_v4(),
            language: Language::Python,
            code: "def f():\n    pass\n".repeat(5),
            metrics: empty_metrics(),
            functions: vec![overrides],
            classes: vec![],
            file_path: None,
        }
    }

    fn base_function() -> FunctionInfo {
        FunctionInfo {
            name: "f".to_string(),
            line_start: 1,
            line_end: 5,
            params_count: 1,
            lines_of_code: 5,
            cyclomatic_complexity: 1,
            nesting_depth: 1,
        }
    }

    #[test]
    fn long_method_fires_above_50_loc() {
        let mut f = base_function();
        f.lines_of_code = 51;
        let ast = make_function(f);
        let problems = SmellDetector::new().detect(&ast);
        assert!(problems.iter().any(|p| p.problem_type == "code_smell.long_method"));
    }

    #[test]
    fn long_method_does_not_fire_at_50_loc() {
        let mut f = base_function();
        f.lines_of_code = 50;
        let ast = make_function(f);
        let problems = SmellDetector::new().detect(&ast);
        assert!(!problems.iter().any(|p| p.problem_type == "code_smell.long_method"));
    }

    #[test]
    fn long_method_severity_high_above_100_loc() {
        let mut f = base_function();
        f.lines_of_code = 101;
        let ast = make_function(f);
        let problems = SmellDetector::new().detect(&ast);
        let p = problems.iter().find(|p| p.problem_type == "code_smell.long_method").unwrap();
        assert_eq!(p.severity, Severity::High);
    }

    #[test]
    fn long_parameter_list_fires_above_5_params() {
        let mut f = base_function();
        f.params_count = 6;
        let ast = make_function(f);
        let problems = SmellDetector::new().detect(&ast);
        assert!(problems.iter().any(|p| p.problem_type == "code_smell.long_parameter_list"));
    }

    #[test]
    fn deep_nesting_fires_above_4_levels() {
        let mut f = base_function();
        f.nesting_depth = 5;
        let ast = make_function(f);
        let problems = SmellDetector::new().detect(&ast);
        assert!(problems.iter().any(|p| p.problem_type == "code_smell.deep_nesting"));
    }

    #[test]
    fn deep_nesting_does_not_fire_at_4_levels() {
        let mut f = base_function();
        f.nesting_depth = 4;
        let ast = make_function(f);
        let problems = SmellDetector::new().detect(&ast);
        assert!(!problems.iter().any(|p| p.problem_type == "code_smell.deep_nesting"));
    }

    #[test]
    fn complex_method_fires_above_10() {
        let mut f = base_function();
        f.cyclomatic_complexity = 11;
        let ast = make_function(f);
        let problems = SmellDetector::new().detect(&ast);
        assert!(problems.iter().any(|p| p.problem_type == "code_smell.complex_method"));
    }

    #[test]
    fn large_class_fires_above_20_methods() {
        let ast = ParsedAst {
            analysis_job_id: Uuid::new_v4(),
            language: Language::Python,
            code: "class Foo: pass\n".to_string(),
            metrics: empty_metrics(),
            functions: vec![],
            classes: vec![ClassInfo {
                name: "Foo".to_string(),
                line_start: 1,
                line_end: 100,
                method_count: 21,
                lines_of_code: 100,
            }],
            file_path: None,
        };
        let problems = SmellDetector::new().detect(&ast);
        assert!(problems.iter().any(|p| p.problem_type == "code_smell.large_class"));
    }

    #[test]
    fn long_file_fires_above_500_code_lines() {
        let mut metrics = empty_metrics();
        metrics.code_lines = 501;
        let ast = ParsedAst {
            analysis_job_id: Uuid::new_v4(),
            language: Language::Python,
            code: "x = 1\n".to_string(),
            metrics,
            functions: vec![],
            classes: vec![],
            file_path: None,
        };
        let problems = SmellDetector::new().detect(&ast);
        assert!(problems.iter().any(|p| p.problem_type == "code_smell.long_file"));
    }

    #[test]
    fn clean_function_triggers_nothing() {
        let ast = make_function(base_function());
        let problems = SmellDetector::new().detect(&ast);
        assert!(problems.is_empty());
    }

    fn ast_with_raw_code(code: &str) -> ParsedAst {
        ParsedAst {
            analysis_job_id: Uuid::new_v4(),
            language: Language::Python,
            code: code.to_string(),
            metrics: empty_metrics(),
            functions: vec![],
            classes: vec![],
            file_path: None,
        }
    }

    #[test]
    fn magic_number_detected_for_multi_digit_literal() {
        let code = "def f():\n    timeout = 86400\n";
        let problems = SmellDetector::new().detect(&ast_with_raw_code(code));
        assert!(problems.iter().any(|p| p.problem_type == "code_smell.magic_number"));
    }

    #[test]
    fn magic_number_detected_for_float_literal() {
        let code = "def f():\n    discount = 0.15\n";
        let problems = SmellDetector::new().detect(&ast_with_raw_code(code));
        assert!(problems.iter().any(|p| p.problem_type == "code_smell.magic_number"));
    }

    #[test]
    fn single_digit_numbers_not_flagged() {
        // 0/1/2/9 - uobicajeni indeksi/koraci, ne treba ih tretirati kao "magic".
        let code = "def f():\n    for i in range(9):\n        x = i + 1\n";
        let problems = SmellDetector::new().detect(&ast_with_raw_code(code));
        assert!(problems.iter().all(|p| p.problem_type != "code_smell.magic_number"));
    }

    #[test]
    fn named_constant_declaration_not_flagged() {
        let code = "MAX_RETRIES = 42\n";
        let problems = SmellDetector::new().detect(&ast_with_raw_code(code));
        assert!(problems.iter().all(|p| p.problem_type != "code_smell.magic_number"));
    }

    #[test]
    fn number_inside_string_literal_not_flagged() {
        let code = "def f():\n    print(\"Server running on port 8080\")\n";
        let problems = SmellDetector::new().detect(&ast_with_raw_code(code));
        assert!(problems.iter().all(|p| p.problem_type != "code_smell.magic_number"));
    }
}