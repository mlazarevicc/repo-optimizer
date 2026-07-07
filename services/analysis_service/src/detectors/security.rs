use crate::detectors::Detector;
use crate::models::{ParsedAst, Problem, ProblemType, Severity};
use uuid::Uuid;
use regex::Regex;

pub struct SecurityDetector;

impl SecurityDetector {
    pub fn new() -> Self {
        Self
    }
}

impl Detector for SecurityDetector {
    fn detect(&self, data: &ParsedAst) -> Vec<Problem> {
        let mut problems = Vec::new();

        // 1. Hardcoded secrets (API keys, passwords, tokens)
        let secret_patterns = vec![
            (r#"(?i)(api[_-]?key|apikey)\s*=\s*['""][^'""]{20,}['""]"#, "API key"),
            (r#"(?i)(password|passwd|pwd)\s*=\s*['""][^'""]{8,}['""]"#, "Password"),
            (r#"(?i)(secret|token)\s*=\s*['""][^'""]{20,}['""]"#, "Secret/Token"),
            // Ranije je ovo bilo (aws_access_key|aws_secret) - to NIJE hvatalo "aws_secret_key"
            // (regex je trazio "aws_secret" odmah ispred "=", a ovde dolazi "_key" izmedju).
            // Sad pokriva proizvoljnu kombinaciju aws + access/secret/key/token u imenu promenljive.
            (r#"(?i)\baws[a-z_]*(access|secret|key|token)[a-z_]*\s*=\s*['""][^'""]+['""]"#, "AWS credential"),
        ];

        for (pattern_str, secret_type) in secret_patterns {
            let pattern = Regex::new(pattern_str).unwrap();
            for (idx, line) in data.code.lines().enumerate() {
                if pattern.is_match(line) {
                    problems.push(Problem {
                        id: Uuid::new_v4(),
                        analysis_job_id: data.analysis_job_id,
                        // problem_type: ProblemType::HardcodedSecret,
                        problem_type: "security.hardcoded_secret".to_string(),
                        severity: Severity::Critical,
                        line_start: idx + 1,
                        line_end: idx + 1,
                        message: format!(
                            "Hardcoded {} detected. Use environment variables or secret management.",
                            secret_type
                        ),
                        code_snippet: line.to_string(),
                        created_at: chrono::Utc::now(),
                        file_path: data.file_path.clone(),
                        language: data.language.to_string(),
                        autofix: None,
                        rank_score: None,
                    });
                }
            }
        }

        // 2. XSS (for JavaScript/TypeScript)
        if matches!(data.language, crate::models::Language::JavaScript | crate::models::Language::TypeScript) {
            let xss_patterns = vec![
                r#"innerHTML\s*=\s*[^'""]"#,
                r#"document\.write\s*\("#,
                r#"eval\s*\("#,
            ];

            for pattern_str in xss_patterns {
                let pattern = Regex::new(pattern_str).unwrap();
                for (idx, line) in data.code.lines().enumerate() {
                    if pattern.is_match(line) {
                        problems.push(Problem {
                            id: Uuid::new_v4(),
                            analysis_job_id: data.analysis_job_id,
                            // problem_type: ProblemType::XssVulnerability,
                            problem_type: "security.xss_vulnerability".to_string(),
                            severity: Severity::High,
                            line_start: idx + 1,
                            line_end: idx + 1,
                            message: "Potential XSS vulnerability. Sanitize user input before rendering.".to_string(),
                            code_snippet: line.to_string(),
                            created_at: chrono::Utc::now(),
                            file_path: data.file_path.clone(),
                            language: data.language.to_string(),
                            autofix: None,
                            rank_score: None,
                        });
                    }
                }
            }
        }


        // 3. Insecure random generation
        {
            let py_re = regex::Regex::new(
                r"random\.(random|randint|randrange|choice|choices|shuffle|sample|getrandbits)\s*\("
            ).unwrap();
            let js_re = regex::Regex::new(r"Math\.random\s*\(\)").unwrap();

            for (idx, line) in data.code.lines().enumerate() {
                let is_py = matches!(data.language, crate::models::Language::Python);
                let is_js = matches!(
                    data.language,
                    crate::models::Language::JavaScript | crate::models::Language::TypeScript
                );
                let matched = (is_py && py_re.is_match(line)) || (is_js && js_re.is_match(line));
                if !matched { continue; }

                let sensitive = is_security_sensitive_context(line);
                let severity = if sensitive { Severity::High } else { Severity::Low };
                let msg = if is_py {
                    "Python `random` module is not cryptographically secure. Use `secrets` module: secrets.token_hex(16)."
                } else {
                    "Math.random() is not cryptographically secure. Use crypto.getRandomValues() or crypto.randomUUID()."
                };
                problems.push(Problem {
                    id: Uuid::new_v4(),
                    analysis_job_id: data.analysis_job_id,
                    problem_type: "security.insecure_random".to_string(),
                    severity,
                    line_start: idx + 1,
                    line_end: idx + 1,
                    message: msg.to_string(),
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

fn is_security_sensitive_context(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("token") || lower.contains("secret") || lower.contains("password")
        || lower.contains("passwd") || lower.contains("nonce") || lower.contains("csrf")
        || lower.contains("session_id") || lower.contains("api_key") || lower.contains("auth")
        || lower.contains("otp") || lower.contains("salt") || lower.contains("sign")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CodeMetrics, Language, ParsedAst};

    fn ast_with_code(code: &str, language: Language) -> ParsedAst {
        ParsedAst {
            analysis_job_id: Uuid::new_v4(),
            language,
            code: code.to_string(),
            metrics: CodeMetrics {
                total_lines: code.lines().count(),
                code_lines: code.lines().count(),
                comment_lines: 0,
                blank_lines: 0,
                total_functions: 0,
                total_classes: 0,
                max_nesting_depth: 0,
                cyclomatic_complexity: 0,
            },
            functions: vec![],
            classes: vec![],
            file_path: None,
        }
    }

    #[test]
    fn detects_aws_access_key() {
        let code = "aws_access_key = \"AKIAIOSFODNN7EXAMPLE\"\n";
        let problems = SecurityDetector::new().detect(&ast_with_code(code, Language::Python));
        assert!(problems.iter().any(|p| p.problem_type == "security.hardcoded_secret"));
    }

    // Regresioni test za bug iz produkcije: regex je ranije trazio "aws_secret" odmah
    // ispred "=" i NIJE hvatao "aws_secret_key" (jer dolazi "_key" izmedju).
    #[test]
    fn detects_aws_secret_key_regression() {
        let code = "aws_secret_key = \"wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY\"\n";
        let problems = SecurityDetector::new().detect(&ast_with_code(code, Language::Python));
        assert!(
            problems.iter().any(|p| p.problem_type == "security.hardcoded_secret"),
            "aws_secret_key mora biti detektovan kao hardkodovan secret"
        );
    }

    #[test]
    fn detects_hardcoded_password() {
        let code = "db_password = \"super_secret_production_password\"\n";
        let problems = SecurityDetector::new().detect(&ast_with_code(code, Language::Python));
        assert!(problems.iter().any(|p| p.problem_type == "security.hardcoded_secret"));
    }

    #[test]
    fn short_password_value_not_flagged() {
        // Manje od 8 karaktera - ne treba da okine pattern za password.
        let code = "pwd = \"ab\"\n";
        let problems = SecurityDetector::new().detect(&ast_with_code(code, Language::Python));
        assert!(!problems.iter().any(|p| p.problem_type == "security.hardcoded_secret"));
    }

    #[test]
    fn detects_xss_in_javascript() {
        let code = "el.innerHTML = userInput;\n";
        let problems = SecurityDetector::new().detect(&ast_with_code(code, Language::JavaScript));
        assert!(problems.iter().any(|p| p.problem_type == "security.xss_vulnerability"));
    }

    #[test]
    fn xss_patterns_skipped_for_python() {
        // innerHTML/eval su JS-specificni - ne treba da se provericmo za Python.
        let code = "x.innerHTML = 5\n";
        let problems = SecurityDetector::new().detect(&ast_with_code(code, Language::Python));
        assert!(problems.iter().all(|p| p.problem_type != "security.xss_vulnerability"));
    }

    #[test]
    fn clean_code_triggers_nothing() {
        let code = "def add(a, b):\n    return a + b\n";
        let problems = SecurityDetector::new().detect(&ast_with_code(code, Language::Python));
        assert!(problems.is_empty());
    }

    #[test]
    fn insecure_random_python_sensitive_context_is_high() {
        let code = "def gen():\n    auth_token = random.random()\n";
        let problems = SecurityDetector::new().detect(&ast_with_code(code, Language::Python));
        let hit = problems.iter().find(|p| p.problem_type == "security.insecure_random");
        assert!(hit.is_some(), "deve detectar insecure random");
        assert_eq!(hit.unwrap().severity, Severity::High);
    }

    #[test]
    fn insecure_random_python_neutral_context_is_low() {
        let code = "def pick_color():\n    r = random.randint(0, 255)\n";
        let problems = SecurityDetector::new().detect(&ast_with_code(code, Language::Python));
        let hit = problems.iter().find(|p| p.problem_type == "security.insecure_random");
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().severity, Severity::Low);
    }

    #[test]
    fn insecure_random_js_token_context_is_high() {
        let code = "function gen() {\n    const token = Math.random();\n}\n";
        let problems = SecurityDetector::new().detect(&ast_with_code(code, Language::JavaScript));
        let hit = problems.iter().find(|p| p.problem_type == "security.insecure_random");
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().severity, Severity::High);
    }

    #[test]
    fn math_random_not_flagged_for_python() {
        // Math.random je JS pattern, Python fajl ne treba da ga hvata
        let code = "x = Math.random()\n";
        let problems = SecurityDetector::new().detect(&ast_with_code(code, Language::Python));
        assert!(!problems.iter().any(|p| p.problem_type == "security.insecure_random"));
    }

}
