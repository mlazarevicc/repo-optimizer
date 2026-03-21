use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, path::PathBuf};

const GATEWAY_URL: &str = "http://localhost:8006/api";

#[derive(Parser)]
#[command(name = "repo-opt")]
#[command(about = "RepoOptimizer CLI - Code analysis from your terminal", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Login {
        #[arg(short, long)]
        email: String,
        #[arg(short, long)]
        password: String,
    },
    Logout,
    Analyze {
        file_path: PathBuf,
    },
}

#[derive(Serialize, Deserialize)]
struct AuthConfig {
    token: String,
    email: String,
}

fn get_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find the home directory")?;
    Ok(home.join(".repo-optimizer-auth.json"))
}

async fn handle_login(email: &str, password: &str) -> Result<()> {
    println!("{} Connecting to server...", "➔".blue());
    
    let client = Client::new();
    let res = client
        .post(&format!("{}/auth/login", GATEWAY_URL))
        .json(&serde_json::json!({
            "email": email,
            "password": password
        }))
        .send()
        .await?;

    if !res.status().is_success() {
        let err: Value = res.json().await?;
        println!("{} {}", "Login error:".red().bold(), err["error"].as_str().unwrap_or("Unknown error"));
        return Ok(());
    }

    let data: Value = res.json().await?;
    let token = data["token"].as_str().context("Missing token in response")?;

    let config = AuthConfig {
        token: token.to_string(),
        email: email.to_string(),
    };

    let config_path = get_config_path()?;
    fs::write(&config_path, serde_json::to_string(&config)?)?;

    println!("{} Successfully logged in as {}!", "✔".green().bold(), email.cyan());
    Ok(())
}

async fn handle_logout() -> Result<()> {
    let config_path = get_config_path()?;
    
    if config_path.exists() {
        fs::remove_file(&config_path).context("Failed to delete the authentication file")?;
        println!("{} Successfully logged out. Session cleared.", "✔".green().bold());
    } else {
        println!("{} You are not currently logged in.", "ℹ".yellow());
    }
    
    Ok(())
}

async fn handle_analyze(file_path: &PathBuf) -> Result<()> {
    // 1. Authentication check
    let config_path = get_config_path()?;
    if !config_path.exists() {
        println!("{} You are not logged in. Run: repo-opt login --email <e> --password <p>", "✖".red());
        return Ok(());
    }
    let config_data = fs::read_to_string(config_path)?;
    let config: AuthConfig = serde_json::from_str(&config_data)?;

    // 2. File reading and language detection
    let code = fs::read_to_string(file_path).context("Could not read the specified file")?;
    let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    
    let language = match extension {
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "rs" => "rust",
        "java" => "java",
        _ => {
            println!("{} Unsupported file extension: {}", "✖".red(), extension);
            return Ok(());
        }
    };

    println!("{} Analyzing {} (Language: {})...", "➔".blue(), file_path.display(), language.cyan());

    // 3. Sending to API Gateway
    let client = Client::new();
    let res = client
        .post(&format!("{}/analyze", GATEWAY_URL))
        .header("Authorization", format!("Bearer {}", config.token))
        .json(&serde_json::json!({
            "language": language,
            "code": code
        }))
        .send()
        .await?;

    if !res.status().is_success() {
        println!("{} Analysis failed (Status: {})", "✖".red(), res.status());
        return Ok(());
    }

    let result: Value = res.json().await?;

    // 4. Displaying results
    println!("\n{}", "=== ANALYSIS REPORT ===".bold());
    
    let summary = &result["summary"];
    let total = summary["total_problems"].as_u64().unwrap_or(0);
    
    if total == 0 {
        println!("{} Your code is perfect! No issues found.", "✔".green().bold());
        return Ok(());
    }

    println!("Issues found: {}\n", total.to_string().yellow().bold());

    let ranked_issues = result["ranked_issues"].as_array().unwrap();
    let suggestions = result["suggestions"].as_array().unwrap();

    for (i, issue) in ranked_issues.iter().enumerate() {
        let severity = issue["severity"].as_str().unwrap_or("low");
        let severity_colored = match severity {
            "critical" => "CRITICAL".red().bold(),
            "high" => "HIGH".truecolor(255, 165, 0).bold(), // Orange
            "medium" => "MEDIUM".yellow().bold(),
            _ => "LOW".blue().bold(),
        };

        println!("{}. [{}] {}", i + 1, severity_colored, issue["message"].as_str().unwrap().white().bold());
        println!("   {} Line: {}", "📍".cyan(), issue["line_start"]);
        
        // Find suggestion if it exists
        let issue_id = issue["id"].as_str().unwrap_or("");
        if let Some(sug) = suggestions.iter().find(|s| s["problem_id"].as_str().unwrap_or("") == issue_id) {
            println!("   {} {} (+{} Impact Score)", 
                "💡".yellow(), 
                "FIX SUGGESTION:".green(), 
                sug["impact_score"].as_u64().unwrap_or(0)
            );
            println!("   {}\n", sug["suggested_code"].as_str().unwrap().truecolor(150, 150, 150).italic());
        } else {
            println!();
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Login { email, password } => {
            handle_login(email, password).await?;
        }
        Commands::Logout => {
            handle_logout().await?;
        }
        Commands::Analyze { file_path } => {
            handle_analyze(file_path).await?;
        }
    }

    Ok(())
}