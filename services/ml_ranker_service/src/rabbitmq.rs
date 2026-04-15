use crate::AppState;
use futures_lite::stream::StreamExt;
use lapin::{options::*, types::FieldTable, BasicProperties, Connection, ConnectionProperties};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, sync::Arc};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct RankJob {
    analysis_job_id: Uuid,
    problems: Vec<Value>,
}

pub async fn start_worker(state: Arc<AppState>) -> Result<(), lapin::Error> {
    let amqp_url = env::var("AMQP_URL").unwrap_or_else(|_| "amqp://repo_optimizer:dev_password@localhost:5672/%2f".into());
    let conn = Connection::connect(&amqp_url, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;

    channel.queue_declare("rank_queue", QueueDeclareOptions::default(), FieldTable::default()).await?;
    channel.queue_declare("suggest_queue", QueueDeclareOptions::default(), FieldTable::default()).await?;

    let mut consumer = channel
        .basic_consume(
            "rank_queue",
            "ml_ranker_service",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    tracing::info!("ML Ranker worker listening on 'rank_queue'...");

    while let Some(delivery) = consumer.next().await {
        if let Ok(delivery) = delivery {
            let state_clone = state.clone();
            let channel_clone = channel.clone();

            tokio::spawn(async move {
                let payload: Result<RankJob, _> = serde_json::from_slice(&delivery.data);

                match payload {
                    Ok(job) => {
                        tracing::info!("Processing rank job: {}", job.analysis_job_id);

                        match crate::process_ranking(&state_clone,  job.problems).await {
                            Ok(ranked_problems) => {
                                let next_job = json!({
                                    "analysis_job_id": job.analysis_job_id,
                                    "problems": ranked_problems,
                                });

                                let _ = channel_clone
                                    .basic_publish(
                                        "",
                                        "suggest_queue",
                                        BasicPublishOptions::default(),
                                        &serde_json::to_vec(&next_job).unwrap(),
                                        BasicProperties::default(),
                                    )
                                    .await;
                                    
                                tracing::info!("Job {} successfully ranked and sent to suggest_queue", job.analysis_job_id);
                                
                                let _ = delivery.ack(BasicAckOptions::default()).await;
                            }
                            Err(e) => {
                                tracing::error!("Failed to process ranking for job {}: {}", job.analysis_job_id, e);
                            
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