use crate::AppState;
use futures_lite::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, sync::Arc};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct SuggestJob {
    analysis_job_id: Uuid,
    problems: Vec<Value>,
}

pub async fn start_worker(state: Arc<AppState>) -> Result<(), lapin::Error> {
    let amqp_url = env::var("AMQP_URL").unwrap_or_else(|_| "amqp://repo_optimizer:dev_password@localhost:5672/%2f".into());
    let conn = Connection::connect(&amqp_url, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;

    channel.queue_declare("suggest_queue", QueueDeclareOptions::default(), FieldTable::default()).await?;

    let mut consumer = channel
        .basic_consume(
            "suggest_queue",
            "suggestion_service",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    tracing::info!("Suggestion Generator worker listening on 'suggest_queue'...");

    while let Some(delivery) = consumer.next().await {
        if let Ok(delivery) = delivery {
            let payload: Result<SuggestJob, _> = serde_json::from_slice(&delivery.data);

            match payload {
                Ok(job) => {
                    tracing::info!("Generating suggestions for job: {}", job.analysis_job_id);

                    match crate::process_suggestions(&state, job.analysis_job_id, job.problems).await {
                        Ok(_) => {
                            tracing::info!("Job {} fully completed and saved to DB!", job.analysis_job_id);
                            let _ = delivery.ack(BasicAckOptions::default()).await;
                        }
                        Err(e) => {
                            tracing::error!("Failed to process suggestions for job {}: {}", job.analysis_job_id, e);
                            let _ = delivery.nack(BasicNackOptions { multiple: false, requeue: false }).await;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to deserialize RabbitMQ message: {}", e);
                    let _ = delivery.nack(BasicNackOptions { multiple: false, requeue: false }).await;
                }
            }
        }
    }

    Ok(())
}