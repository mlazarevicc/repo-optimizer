use crate::{models::ParseRequest, AppState};
use futures_lite::stream::StreamExt;
use lapin::{options::*, types::FieldTable, BasicProperties, Connection, ConnectionProperties};
use serde_json::json;
use tokio::sync::Semaphore;
use std::{env, sync::Arc};

pub async fn start_worker(state: Arc<AppState>) -> Result<(), lapin::Error> {
    let amqp_url = env::var("AMQP_URL").unwrap_or_else(|_| "amqp://repo_optimizer:dev_password@localhost:5672/%2f".into());
    let conn = Connection::connect(&amqp_url, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;

    channel.queue_declare("parse_queue", QueueDeclareOptions::default(), FieldTable::default()).await?;
    channel.queue_declare("analyze_queue", QueueDeclareOptions::default(), FieldTable::default()).await?;

    let mut consumer = channel
        .basic_consume(
            "parse_queue",
            "parser_service",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    tracing::info!("Parser service worker listening on 'parse_queue'...");
    let semaphore = Arc::new(Semaphore::new(20));

    while let Some(delivery) = consumer.next().await {
        if let Ok(delivery) = delivery {
            let state_clone = state.clone();
            let channel_clone = channel.clone();

            let _permit = semaphore.clone().acquire_owned().await.unwrap();

            tokio::spawn(async move {
                let payload: Result<ParseRequest, _> = serde_json::from_slice(&delivery.data);

                match payload {
                    Ok(req) => {
                        let job_id = req.analysis_job_id.unwrap_or_default();
                        tracing::info!("Processing parsing job: {}", job_id);

                        match crate::process_and_save(&state_clone, req.clone()).await {
                            Ok(_) => {
                                let next_job = json!({
                                    "analysis_job_id": job_id,
                                    "file_path": req.file_path,
                                    "user_id": req.user_id,
                                    "language": req.language,
                                });

                                let _ = channel_clone
                                    .basic_publish(
                                        "",
                                        "analyze_queue",
                                        BasicPublishOptions::default(),
                                        &serde_json::to_vec(&next_job).unwrap(),
                                        BasicProperties::default(),
                                    )
                                    .await;
                                    
                                tracing::info!("Job {} successfully parsed and sent to analyze_queue", job_id);
                                
                                let _ = delivery.ack(BasicAckOptions::default()).await;
                            }
                            Err(e) => {
                                tracing::error!("Failed to process parsing job {}: {}", job_id, e);
                                let _ = delivery.nack(BasicNackOptions { multiple: false, requeue: false }).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to deserialize RabbitMQ message: {}", e);
                        let _ = delivery.nack(BasicNackOptions { multiple: false, requeue: false }).await;
                    }
                }
            });
        }
    }

    Ok(())
}