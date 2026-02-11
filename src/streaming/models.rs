use std::time::Duration;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::{Deserialize, Serialize};
use crate::fingerprint::hashing::Fingerprint;

#[derive(Deserialize)]
pub struct SongUploaded {
    pub(crate) song_id: String,
    pub(crate) s3_key: String,
    pub(crate) uploaded_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct FingerprintGenerated {
    pub(crate) song_id: String,
    pub(crate) fingerprints: Vec<Fingerprint>,
    pub(crate) generated_at: i64,
}

#[async_trait::async_trait]
pub trait EventProducer {
    async fn send(
        &self,
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
        &self,
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