//! Event models and producer trait for Kafka integration.
//!
//! This module defines the event schemas used in the fingerprinting workflow
//! and the trait for extensible event production.

use std::time::Duration;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::{Deserialize, Serialize};
use crate::fingerprint::hashing::Fingerprint;

/// Event signaling that a song has been uploaded to storage.
///
/// This event is published when a new audio file becomes available for fingerprinting.
/// The Kafka worker listens for these events and initiates fingerprint generation.
#[derive(Deserialize, Serialize)]
pub struct SongUploaded {
    /// Unique identifier for the song
    pub(crate) song_id: String,
    /// S3 object key where the audio file is stored
    pub(crate) s3_key: String,
}

impl SongUploaded {
    /// Create a new `SongUploaded` event.
    ///
    /// # Arguments
    ///
    /// * `song_id` - Unique song identifier
    /// * `s3_key` - S3 object key for the audio file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use resonate::streaming::models::SongUploaded;
    /// let event = SongUploaded::new(
    ///     "song123".to_string(),
    ///     "songs/song123.mp3".to_string()
    /// );
    /// ```
    pub fn new(song_id: String, s3_key: String) -> Self {
        Self {
            song_id,
            s3_key,
        }
    }

    /// Serialize the event to JSON.
    ///
    /// # Returns
    ///
    /// JSON string representation of this event
    pub fn as_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

/// Event signaling that fingerprints have been generated for a song.
///
/// Published after fingerprinting completes. Provides metadata about the fingerprint set,
/// including the number of chunks that will follow.
#[derive(Serialize, Deserialize)]
pub struct FingerprintGenerated {
    /// Song ID being fingerprinted
    pub song_id: String,
    /// Total number of fingerprint chunks to expect
    pub total_chunks: usize,
}

/// Abstract trait for event production to Kafka.
///
/// Enables extensible event publishing, allowing mock implementations for testing
/// and different Kafka producer implementations.
#[async_trait::async_trait]
pub trait EventProducer {
    /// Send an event to a Kafka topic.
    ///
    /// # Arguments
    ///
    /// * `topic` - Kafka topic name
    /// * `key` - Message key (enables message ordering and partitioning)
    /// * `payload` - JSON-serialized event data
    ///
    /// # Returns
    ///
    /// - `Ok(())` if successfully sent
    /// - `Err(anyhow::Error)` if publishing fails
    async fn send(
        &mut self,
        topic: &str,
        key: &str,
        payload: String,
    ) -> anyhow::Result<()>;
}

/// Kafka-based implementation of `EventProducer`.
///
/// Uses rdkafka to publish events to Kafka topics with configured compression and batching.
pub struct KafkaProducer {
    /// Inner rdkafka FutureProducer
    pub inner: FutureProducer,
}

#[async_trait::async_trait]
impl EventProducer for KafkaProducer {
    async fn send(
        &mut self,
        topic: &str,
        key: &str,
        payload: String,
    ) -> anyhow::Result<()> {
        self.inner
            .send(
                FutureRecord::to(topic)
                    .key(key)
                    .payload(&payload),
                Duration::from_secs(0),
            )
            .await
            .map_err(|(e, _)| anyhow::anyhow!(e))?;

        Ok(())
    }
}

/// A chunk of fingerprints for a song.
///
/// Large fingerprint sets are split into chunks (max 1000 fingerprints each)
/// and sent as separate events to enable incremental processing.
#[derive(Deserialize, Serialize)]
pub struct FingerprintChunk {
    /// Song ID this chunk belongs to
    pub song_id: String,
    /// Sequential index of this chunk (0-based)
    pub index: u32,
    /// Fingerprint data for this chunk
    pub data: Vec<Fingerprint>,
}