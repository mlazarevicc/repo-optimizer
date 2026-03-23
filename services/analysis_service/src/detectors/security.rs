// use crate::detectors::Detector;
// use crate::models::{ParsedAst, Problem, ProblemType, Severity};
// use uuid::Uuid;
// use regex::Regex;

// pub struct SecurityDetector;

// impl SecurityDetector {
//     pub fn new() -> Self {
//         Self
//     }
// }

// impl Detector for SecurityDetector {
//     fn detect(&self, data: &ParsedAst) -> Vec<Problem> {
//         let mut problems = Vec::new();

//         // 1. Hardcoded secrets (API keys, passwords, tokens)
//         let secret_patterns = vec![
//             (r#"(?i)(api[_-]?key|apikey)\s*=\s*['""][^'""]{20,}['""]"#, "API key"),
//             (r#"(?i)(password|passwd|pwd)\s*=\s*['""][^'""]{8,}['""]"#, "Password"),
//             (r#"(?i)(secret|token)\s*=\s*['""][^'""]{20,}['""]"#, "Secret/Token"),
//             (r#"(?i)(aws_access_key|aws_secret)\s*=\s*['""][^'""]+['""]"#, "AWS credential"),
//         ];

//         for (pattern_str, secret_type) in secret_patterns {
//             let pattern = Regex::new(pattern_str).unwrap();
//             for (idx, line) in data.code.lines().enumerate() {
//                 if pattern.is_match(line) {
//                     problems.push(Problem {
//                         id: Uuid::new_v4(),
//                         analysis_job_id: data.analysis_job_id,
//                         problem_type: ProblemType::HardcodedSecret,
//                         severity: Severity::Critical,
//                         line_start: idx + 1,
//                         line_end: idx + 1,
//                         message: format!(
//                             "Hardcoded {} detected. Use environment variables or secret management.",
//                             secret_type
//                         ),
//                         code_snippet: line.to_string(),
//                         created_at: chrono::Utc::now(),
//                     });
//                 }
//             }
//         }

//         // 2. SQL Injection (naive detection)
//         let sql_injection_patterns = vec![
//             r#"(?i)execute\s*\(\s*['""].*\+.*['""]"#,
//             r#"(?i)query\s*\(\s*f['""].*\{.*\}.*['""]"#,
//             r#"(?i)(select|insert|update|delete).*\+.*"#,
//         ];

//         for pattern_str in sql_injection_patterns {
//             let pattern = Regex::new(pattern_str).unwrap();
//             for (idx, line) in data.code.lines().enumerate() {
//                 if pattern.is_match(line) {
//                     problems.push(Problem {
//                         id: Uuid::new_v4(),
//                         analysis_job_id: data.analysis_job_id,
//                         problem_type: ProblemType::SqlInjection,
//                         severity: Severity::Critical,
//                         line_start: idx + 1,
//                         line_end: idx + 1,
//                         message: "Potential SQL injection vulnerability. Use parameterized queries.".to_string(),
//                         code_snippet: line.to_string(),
//                         created_at: chrono::Utc::now(),
//                     });
//                 }
//             }
//         }

//         // 3. XSS (for JavaScript/TypeScript)
//         if matches!(data.language, crate::models::Language::JavaScript | crate::models::Language::TypeScript) {
//             let xss_patterns = vec![
//                 r#"innerHTML\s*=\s*[^'""]"#,
//                 r#"document\.write\s*\("#,
//                 r#"eval\s*\("#,
//             ];

//             for pattern_str in xss_patterns {
//                 let pattern = Regex::new(pattern_str).unwrap();
//                 for (idx, line) in data.code.lines().enumerate() {
//                     if pattern.is_match(line) {
//                         problems.push(Problem {
//                             id: Uuid::new_v4(),
//                             analysis_job_id: data.analysis_job_id,
//                             problem_type: ProblemType::XssVulnerability,
//                             severity: Severity::High,
//                             line_start: idx + 1,
//                             line_end: idx + 1,
//                             message: "Potential XSS vulnerability. Sanitize user input before rendering.".to_string(),
//                             code_snippet: line.to_string(),
//                             created_at: chrono::Utc::now(),
//                         });
//                     }
//                 }
//             }
//         }

//         problems
//     }
// }
