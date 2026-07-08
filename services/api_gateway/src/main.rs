mod middleware;
mod routes;
mod rabbitmq;
mod scanner;

use axum::{
    routing::{get, post, delete},
    Json, Router,
};
use serde::Serialize;
use std::env;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;
use mongodb::Client as MongoClient;
use sqlx::PgPool;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "api_gateway_service",
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let port: u16 = env::var("API_GATEWAY_SERVICE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8006);

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let amqp_channel = rabbitmq::setup_rabbitmq()
        .await
        .expect("Failed to connect to RabbitMQ");

    let mongodb_url = env::var("MONGODB_URL").unwrap_or_else(|_| "mongodb://repo_optimizer:dev_password@localhost:27017".into());
    let mongo_client = MongoClient::with_uri_str(&mongodb_url).await?;
    let db = mongo_client.database("repo_optimizer");
    tracing::info!("API Gateway connected to MongoDB");

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pg_pool = PgPool::connect(&database_url).await?;
    tracing::info!("API Gateway connected to PostgreSQL");

    // Redis - kesiranje COMPLETED rezultata (TTL 1h). Best-effort: startujemo
    // i ako Redis nije dostupan samo logujemo warning, ne pucamo ceo servis.
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis:6379".to_string());
    let redis_client = redis::Client::open(redis_url.as_str())
        .expect("Invalid REDIS_URL");
    tracing::info!("API Gateway Redis client initialized ({})", redis_url);

    let state = routes::GatewayState { 
        http_client,
        amqp_channel,
        db, 
        pg_pool,
        redis_client,
    };

    let public_routes = Router::new()
        .route("/api/auth/register", post(routes::register_handler))
        .route("/api/auth/login", post(routes::login_handler));

    let protected_routes = Router::new()
        .route("/api/analyze", post(routes::analyze_code_handler))
        .route("/api/analyze/git", post(routes::analyze_git_handler))
        .route("/api/analyze/zip", post(routes::analyze_zip_handler))
        .route("/api/results/:job_id", get(routes::get_results_handler))
        .route("/api/results/:job_id", delete(routes::delete_results_handler))
        .route("/api/jobs", get(routes::list_jobs_handler))
        .layer(axum::middleware::from_fn(middleware::auth_middleware));

    let app = Router::new()
        .route("/health", get(health))
        .merge(public_routes)
        .merge(protected_routes)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("api_gateway_service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}