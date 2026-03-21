use lapin::{
    options::*, types::FieldTable, BasicProperties, Channel, Connection, ConnectionProperties,
};
use serde::Serialize;
use std::env;

pub async fn setup_rabbitmq() -> Result<Channel, lapin::Error> {
    let amqp_url = env::var("AMQP_URL").unwrap_or_else(|_| "amqp://repo_optimizer:dev_password@localhost:5672/%2f".into());
    
    tracing::info!("Connecting to RabbitMQ...");
    let conn = Connection::connect(&amqp_url, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;

    channel
        .queue_declare(
            "parse_queue",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    tracing::info!("RabbitMQ channel created and parse_queue declared.");
    Ok(channel)
}

pub async fn publish_job<T: Serialize>(
    channel: &Channel,
    payload: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let serialized_payload = serde_json::to_vec(payload)?;

    channel
        .basic_publish(
            "",            // exchange
            "parse_queue", // routing key
            BasicPublishOptions::default(),
            &serialized_payload,
            BasicProperties::default(),
        )
        .await?;

    Ok(())
}