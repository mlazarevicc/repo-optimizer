use crate::rabbitmq;
use lapin::Channel;
use serde::Serialize;
use std::{fs, path::Path};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Serialize)]
pub struct FileParseRequest {
    pub code: String,
    pub language: String,
    pub file_path: String,
    pub analysis_job_id: Uuid,
    pub user_id: Option<String>,
}

pub async fn scan_directory_and_publish(
    dir_path: &Path,
    job_id: Uuid,
    user_id: Option<String>,
    channel: &Channel,
) -> Result<usize, String> {
    let mut files_sent = 0;

    for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_dir() {
            continue;
        }

        let path_str = path.to_string_lossy();
        if path_str.contains("/.git/") 
            || path_str.contains("/node_modules/") 
            || path_str.contains("/target/") 
            || path_str.contains("/venv/") 
        {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let language = match ext {
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
            "rs" => "rust",
            "java" => "java",
            _ => continue,
        };

        if let Ok(code) = fs::read_to_string(path) {
            let relative_path = path.strip_prefix(dir_path)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            let payload = FileParseRequest {
                code,
                language: language.to_string(),
                file_path: relative_path,
                analysis_job_id: job_id,
                user_id: user_id.clone(),
            };

            if rabbitmq::publish_job(channel, &payload).await.is_ok() {
                files_sent += 1;
            }
        }
    }

    Ok(files_sent)
}