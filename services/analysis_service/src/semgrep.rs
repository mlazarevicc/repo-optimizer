use crate::models::{Problem, Severity};
use serde_json::Value;
use std::{io::Write, process::Command};
use tempfile::Builder;
use uuid::Uuid;

pub fn run_scan(code: &str, language: &str, analysis_job_id: Uuid) -> Result<Vec<Problem>, String> {
    let ext = match language.to_lowercase().as_str() {
        "python" => ".py",
        "javascript" => ".js",
        "typescript" => ".ts",
        "rust" => ".rs",
        "java" => ".java",
        _ => ".txt",
    };

    let mut temp_file = Builder::new()
        .suffix(ext)
        .tempfile()
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
        
    temp_file.write_all(code.as_bytes())
        .map_err(|e| format!("Failed to write to temp file: {}", e))?;
        
    let temp_path = temp_file.path().to_str().unwrap();

    tracing::info!("Running Semgrep on temp file: {}", temp_path);

    let output = Command::new("semgrep")
        .arg("scan")
        .arg("--json")
        .arg("--config")
        .arg("auto")
        .arg(temp_path)
        .output()
        .map_err(|e| format!("Failed to execute semgrep. Is it installed on the system? Error: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&stdout).unwrap_or(serde_json::json!({}));

    let mut problems = Vec::new();
    
    if let Some(results) = parsed.get("results").and_then(|r| r.as_array()) {
        for res in results {
            let check_id = res["check_id"].as_str().unwrap_or("unknown_rule").to_string();
            let message = res["extra"]["message"].as_str().unwrap_or("No description provided.").to_string();
            let severity_str = res["extra"]["severity"].as_str().unwrap_or("INFO");
            let line_start = res["start"]["line"].as_u64().unwrap_or(1) as usize;
            let line_end = res["end"]["line"].as_u64().unwrap_or(1) as usize;

            let severity = match severity_str {
                "ERROR" => Severity::Critical,
                "WARNING" => Severity::Medium,
                _ => Severity::Low,
            };

            let code_snippet = code.lines()
                .skip(line_start.saturating_sub(1))
                .take(line_end - line_start + 1)
                .collect::<Vec<_>>()
                .join("\n");

            problems.push(Problem {
                id: Uuid::new_v4(),
                analysis_job_id,
                problem_type: check_id,
                severity,
                line_start,
                line_end,
                message,
                code_snippet,
                created_at: chrono::Utc::now(),
            });
        }
    }

    Ok(problems)
}