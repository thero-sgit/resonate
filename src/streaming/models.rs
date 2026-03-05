use std::time::Duration;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::{Deserialize, Serialize};
use crate::fingerprint::hashing::Fingerprint;

#[derive(Deserialize, Serialize)]
pub struct SongUploaded {
    pub(crate) song_id: String,
    pub(crate) s3_key: String,
}

impl SongUploaded {
    pub fn new(song_id: String, s3_key: String) -> Self {
        Self {
            song_id,
            s3_key,
        }
    }

    pub fn as_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct FingerprintGenerated {
    pub song_id: String,
    pub fingerprints: Vec<Fingerprint>,
}

#[async_trait::async_trait]
pub trait EventProducer {
    async fn send(
        &mut self,
        topic: &str,
        key: &str,
        payload: String,
    ) -> anyhow::Result<()>;
}

pub struct KafkaProducer {
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

#[derive(Deserialize, Serialize)]
struct FingerprintChunk {
    pub song_id: String,
    pub index: u32,
    pub fingerprints: Vec<Fingerprint>,
}