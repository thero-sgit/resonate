use futures::StreamExt;
use rdkafka::{
    ClientConfig,
    consumer::{Consumer, StreamConsumer},
    message::Message,
    producer::{FutureProducer, FutureRecord},
};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use crate::fingerprint::fingerprint_pipeline;
use crate::fingerprint::hashing::Fingerprint;

#[derive(Deserialize)]
struct SongUploaded {
    song_id: String,
    s3_key: String,
    uploaded_at: i64,
}

#[derive(Serialize)]
struct FingerprintGenerated {
    song_id: String,
    fingerprints: Vec<Fingerprint>,
    generated_at: i64,
}

pub fn create_consumer(brokers: &str, group_id: &str) -> StreamConsumer {
    ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("group.id", group_id)
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Consumer creation failed")
}

pub fn create_producer(brokers: &str) -> FutureProducer {
    ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .create()
        .expect("Producer creation failed")
}

pub async fn run_kafka_worker(
    consumer: StreamConsumer,
    producer: FutureProducer,
    s3: aws_sdk_s3::Client,
    bucket: String,
) -> anyhow::Result<()> {

    let mut stream = consumer.stream();

    while let Some(message) = stream.next().await {
        if let Ok(msg) = message {
            if let Some(payload) = msg.payload() {
                let event: SongUploaded =
                    serde_json::from_slice(payload)?;

                process_event(
                    event,
                    &producer,
                    &s3,
                    &bucket,
                ).await?;
            }
        }
    }

    Ok(())
}

async fn process_event(
    event: SongUploaded,
    producer: &FutureProducer,
    s3: &aws_sdk_s3::Client,
    bucket: &str,
) -> anyhow::Result<()> {

    // Download from S3
    let obj = s3
        .get_object()
        .bucket(std::env::var(bucket)?)
        .key(&event.s3_key)
        .send()
        .await?;

    let data = obj.body.collect().await?.into_bytes();

    // Fingerprint
    let fingerprints = tokio::task::spawn_blocking(move || {
        fingerprint_pipeline(data.to_vec())
    })
        .await?;

    // Produce result event
    let output = FingerprintGenerated {
        song_id: event.song_id,
        fingerprints,
        generated_at: chrono::Utc::now().timestamp(),
    };

    let payload = serde_json::to_string(&output)?;

    producer.send(
        FutureRecord::<(), _>::to("fingerprint_generated")
            .payload(&payload),
        Duration::from_secs(0),
    )
        .await
        .map_err(|(e, _msg)| anyhow::anyhow!("Kafka produce error: {}", e))?;

    Ok(())
}

