use crate::AppState;
use futures_lite::stream::StreamExt;
use lapin::{options::*, types::FieldTable, BasicProperties, Connection, ConnectionProperties};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{env, sync::Arc};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct AnalyzeJob {
    analysis_job_id: Uuid,
    user_id: Option<String>,
}

pub async fn start_worker(state: Arc<AppState>) -> Result<(), lapin::Error> {
    let amqp_url = env::var("AMQP_URL").unwrap_or_else(|_| "amqp://repo_optimizer:dev_password@localhost:5672/%2f".into());
    let conn = Connection::connect(&amqp_url, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;

    channel.queue_declare("analyze_queue", QueueDeclareOptions::default(), FieldTable::default()).await?;
    channel.queue_declare("rank_queue", QueueDeclareOptions::default(), FieldTable::default()).await?;

    let mut consumer = channel
        .basic_consume(
            "analyze_queue",
            "analysis_service",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    tracing::info!("Analysis service worker listening on 'analyze_queue'...");

    while let Some(delivery) = consumer.next().await {
        if let Ok(delivery) = delivery {
            let payload: Result<AnalyzeJob, _> = serde_json::from_slice(&delivery.data);

            match payload {
                Ok(job) => {
                    tracing::info!("Processing analysis job: {}", job.analysis_job_id);

                    match crate::process_analysis(&state, job.analysis_job_id, job.user_id.clone()).await {
                        Ok(problems) => {
                            let next_job = json!({
                                "analysis_job_id": job.analysis_job_id,
                                "problems": problems,
                            });

                            let _ = channel
                                .basic_publish(
                                    "",
                                    "rank_queue",
                                    BasicPublishOptions::default(),
                                    &serde_json::to_vec(&next_job).unwrap(),
                                    BasicProperties::default(),
                                )
                                .await;
                                
                            tracing::info!("Job {} successfully analyzed and sent to rank_queue", job.analysis_job_id);
                            let _ = delivery.ack(BasicAckOptions::default()).await;
                        }
                        Err(e) => {
                            tracing::error!("Failed to process analysis for job {}: {}", job.analysis_job_id, e);
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