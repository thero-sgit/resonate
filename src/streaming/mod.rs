//! Kafka event-driven fingerprinting worker.
//!
//! This module provides event streaming infrastructure for processing audio files
//! through Kafka topics. It listens for song upload events, retrieves audio from S3,
//! generates fingerprints, and publishes results back to Kafka.
//!
//! # Event Flow
//!
//! 1. **Input**: `song_uploaded` topic containing `SongUploaded` events
//! 2. **Processing**: Retrieve audio from S3, generate fingerprints
//! 3. **Output**: Publish fingerprints as `fingerprint_generated` and `fingerprint_chunk` events
//!
//! # Message Batching
//!
//! Large fingerprint sets (>1000 entries) are split into chunks for efficient Kafka publishing.
//! Each chunk is sent independently to allow downstream consumers to process incrementally.

pub mod models;

use crate::fingerprint::fingerprint_pipeline;
use crate::streaming::models::{EventProducer, FingerprintChunk, FingerprintGenerated, SongUploaded};
use futures::StreamExt;
use rdkafka::{
    consumer::StreamConsumer,
    message::Message,
    producer::FutureProducer,
    ClientConfig,
};
use crate::fingerprint::hashing::Fingerprint;

/// Create a Kafka consumer configured for fingerprinting.
///
/// # Arguments
///
/// * `brokers` - Comma-separated Kafka broker addresses (e.g., "localhost:9092")
/// * `group_id` - Consumer group ID for coordinating message consumption
///
/// # Returns
///
/// A configured `StreamConsumer` with the following settings:
/// - Auto-commits enabled
/// - Starts from earliest available offset
///
/// # Panics
///
/// Panics if consumer creation fails (invalid brokers, etc.)
pub fn create_consumer(brokers: &str, group_id: &str) -> StreamConsumer {
    ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("group.id", group_id)
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Consumer creation failed")
}

/// Create a Kafka producer configured for fingerprinting.
///
/// # Arguments
///
/// * `brokers` - Comma-separated Kafka broker addresses
///
/// # Returns
///
/// A configured `FutureProducer` with the following settings:
/// - Gzip compression for efficient message storage
/// - Message batching (50ms linger) for throughput optimization
///
/// # Panics
///
/// Panics if producer creation fails (invalid brokers, etc.)
pub fn create_producer(brokers: &str) -> FutureProducer {
    ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("compression.type", "gzip")
        .set("linger.ms", "50")
        .create()
        .expect("Producer creation failed")
}

/// Run the Kafka worker that processes fingerprinting events.
///
/// This is the main entry point for event-driven fingerprinting. It:
/// 1. Consumes `song_uploaded` events from Kafka
/// 2. Downloads audio from S3
/// 3. Generates fingerprints
/// 4. Produces results to `fingerprint_generated` and `fingerprint_chunk` topics
///
/// # Arguments
///
/// * `consumer` - Kafka consumer subscribed to `song_uploaded` topic
/// * `producer` - Event producer for publishing results
/// * `s3` - AWS S3 client for audio file retrieval
/// * `bucket` - S3 bucket name containing audio files
///
/// # Returns
///
/// - `Ok(())` if the worker runs successfully
/// - `Err(anyhow::Error)` if Kafka/S3 operations fail
///
/// # Errors
///
/// Returns errors from:
/// - Kafka consumer/producer operations
/// - S3 object retrieval
/// - JSON serialization
/// - Fingerprint generation failures
///
/// # Behavior
///
/// The worker runs indefinitely, processing messages as they arrive.
/// It automatically commits consumed messages to Kafka.
pub async fn run_kafka_worker<P: EventProducer>(
    consumer: StreamConsumer,
    mut producer: P,
    s3: aws_sdk_s3::Client,
    bucket: String,
) -> anyhow::Result<()> {

    let mut stream = consumer.stream();

    while let Some(message) = stream.next().await {
        if let Ok(msg) = message {
            if let Some(payload) = msg.payload() {
                let event: SongUploaded = match serde_json::from_slice(payload) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("Failed to deserialize event: {:?}", e);
                        continue;
                    }
                };

                let obj = match s3
                    .get_object()
                    .bucket(&bucket)
                    .key(&event.s3_key)
                    .send()
                    .await
                {
                    Ok(o) => o,
                    Err(e) => {
                        tracing::error!("Failed to get object from S3: {:?}", e);
                        continue;
                    }
                };

                let data = match obj.body.collect().await {
                    Ok(d) => d.into_bytes().to_vec(),
                    Err(e) => {
                        tracing::error!("Failed to read S3 body: {:?}", e);
                        continue;
                    }
                };

                if let Err(e) = process_event(event, &mut producer, data).await {
                    tracing::error!("Failed to process event: {:?}", e);
                }
            }
        }
    }

    Ok(())
}

/// Process a single song upload event and produce fingerprints.
///
/// # Arguments
///
/// * `event` - The `SongUploaded` event containing song metadata
/// * `producer` - Event producer for publishing results
/// * `data` - Raw audio file bytes
///
/// # Returns
///
/// - `Ok(())` if processing succeeds
/// - `Err(anyhow::Error)` if fingerprinting or publishing fails
///
/// # Events Published
///
/// - `fingerprint_generated`: Metadata event with total chunk count
/// - `fingerprint_chunk`: One event per 1000 fingerprints
async fn process_event<P: EventProducer>(
    event: SongUploaded,
    producer: &mut P,
    data: Vec<u8>,
) -> anyhow::Result<()> {
    // Fingerprint in blocking task to avoid blocking the async runtime
    let fingerprints = tokio::task::spawn_blocking(move || {
        fingerprint_pipeline(data)
    })
        .await?;

    // Split into chunks for efficient Kafka publishing
    let chunks: Vec<&[Fingerprint]> = fingerprints
        .chunks(1000)
        .collect();

    // Produce metadata event
    let fingerprint_generated_event = FingerprintGenerated {
        song_id: event.song_id.clone(),
        total_chunks: chunks.len(),
    };

    let payload = serde_json::to_string(&fingerprint_generated_event)?;

    let sent = send_fingerprint_chunks(producer, fingerprint_generated_event, chunks).await;

    if Some(sent).is_some() {
        producer.send(
            "fingerprint_generated",
            &event.song_id,
            payload
        ).await?;
    }

    Ok(())
}

/// Send fingerprint chunks to Kafka.
///
/// Publishes each fingerprint chunk as a separate message to enable
/// incremental processing by downstream consumers.
///
/// # Arguments
///
/// * `producer` - Event producer for publishing
/// * `event` - Metadata about the fingerprinting result
/// * `chunks` - Fingerprint data split into chunks
///
/// # Returns
///
/// - `Ok(())` if all chunks are published
/// - `Err(anyhow::Error)` if any publish operation fails
async fn send_fingerprint_chunks<P: EventProducer>(
    producer: &mut P,
    event: FingerprintGenerated,
    chunks: Vec<&[Fingerprint]>,
) -> anyhow::Result<()> {
    let song_id = event.song_id.clone();

    for (index, chunk) in chunks.iter().enumerate() {
        let fingerprint_chunk = FingerprintChunk {
            song_id: song_id.clone(),
            index: index as u32,
            data: chunk.to_vec()
        };

        let payload = serde_json::to_string(&fingerprint_chunk)?;

        producer.send(
            "fingerprint_chunk",
            &song_id,
            payload,
        ).await.expect("Failed to send fingerprint_chunk");
    }

    Ok(())
}

/// Mock event producer for testing.
///
/// Stores published events in an in-memory queue instead of sending to Kafka.
/// Useful for unit testing fingerprinting logic without a running Kafka broker.
pub struct MockProducer {
    /// Thread-safe queue of JSON-serialized events
    pub messages: std::sync::Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl EventProducer for MockProducer {
    async fn send(
        &mut self,
        _topic: &str,
        _key: &str,
        payload: String,
    ) -> anyhow::Result<()> {
        self.messages.lock().unwrap().push(payload);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tokio;

    #[tokio::test]
    async fn test_process_event_produces_fingerprint_event() {
        // set up
        let mut mock = MockProducer {
            messages: std::sync::Mutex::new(vec![]),
        };

        let event = SongUploaded {
            song_id: "test123".into(),
            s3_key: "dummy".into(),
        };

        // use small dummy audio input
        let audio = fs::read("assets/fma_small/000/000002.mp3").unwrap();

        // act
        process_event(
            event,
            &mut mock,
            audio
        ).await.unwrap();

        // assert
        let messages = mock.messages.lock().unwrap();
        assert_eq!(messages.len(), 1);

        let produced: FingerprintGenerated =
            serde_json::from_str(&messages[0]).unwrap();

        assert_eq!(produced.song_id, "test123");
    }
}