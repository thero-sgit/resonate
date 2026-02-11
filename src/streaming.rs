use std::str::Bytes;
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

#[derive(Serialize, Deserialize)]
struct FingerprintGenerated {
    song_id: String,
    fingerprints: Vec<Fingerprint>,
    generated_at: i64,
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

pub async fn run_kafka_worker<P: EventProducer>(
    consumer: StreamConsumer,
    producer: P,
    s3: aws_sdk_s3::Client,
    bucket: String,
) -> anyhow::Result<()> {

    let mut stream = consumer.stream();

    while let Some(message) = stream.next().await {
        if let Ok(msg) = message {
            if let Some(payload) = msg.payload() {
                let event: SongUploaded =
                    serde_json::from_slice(payload)?;

                // Download from S3
                let obj = s3
                    .get_object()
                    .bucket(std::env::var(&bucket)?)
                    .key(&event.s3_key)
                    .send()
                    .await?;

                let data = obj.body.collect().await?.into_bytes().to_vec();

                process_event(
                    event,
                    &producer,
                    data
                ).await?;
            }
        }
    }

    Ok(())
}

async fn process_event<P: EventProducer>(
    event: SongUploaded,
    producer: &P,
    data: Vec<u8>,
) -> anyhow::Result<()> {
    // Fingerprint
    let fingerprints = tokio::task::spawn_blocking(move || {
        fingerprint_pipeline(data)
    })
        .await?;

    // Produce result event
    let output = FingerprintGenerated {
        song_id: event.song_id.clone(),
        fingerprints,
        generated_at: chrono::Utc::now().timestamp(),
    };

    let payload = serde_json::to_string(&output)?;

    producer.send(
        "fingerprint_generated",
        &event.song_id,
        payload,
    )
        .await?;

    Ok(())
}

// Testing
pub struct MockProducer {
    pub messages: std::sync::Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl EventProducer for MockProducer {
    async fn send(
        &self,
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
    use std::fs;
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_process_event_produces_fingerprint_event() {
        // set up
        let mock = MockProducer {
            messages: std::sync::Mutex::new(vec![]),
        };

        let event = SongUploaded {
            song_id: "test123".into(),
            s3_key: "dummy".into(),
            uploaded_at: 0,
        };

        // use small dummy audio input
        let audio = fs::read("assets/fma_small/000/000002.mp3").unwrap();

        // act
        process_event(
            event,
            &mock,
            audio
        ).await.unwrap();

        // assert
        let messages = mock.messages.lock().unwrap();
        assert_eq!(messages.len(), 1);

        let produced: FingerprintGenerated =
            serde_json::from_str(&messages[0]).unwrap();

        assert_eq!(produced.song_id, "test123");
        assert!(!produced.fingerprints.is_empty());
    }
}