use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "repo-optimizer")]
#[command(about = "CLI for RepoOptimizer platform")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check if API gateway is reachable
    Ping {
        #[arg(long, default_value = "http://localhost:8006")]
        base_url: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Commands::Ping { base_url } => {
            let url = format!("{}/health", base_url);
            let resp = reqwest::get(&url).await?;
            println!("Ping {} → status {}", url, resp.status());
        }
    }

    Ok(())
}
