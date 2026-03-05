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
        .set("compression.type", "gzip")  // Enable gzip compression for better compression
        .set("linger.ms", "50")  // Batch messages for better compression
        .create()
        .expect("Producer creation failed")
}

pub async fn run_kafka_worker<P: EventProducer>(
    consumer: StreamConsumer,
    mut producer: P,
    s3: aws_sdk_s3::Client,
    // actual bucket name (not an env var key). the caller should read any
    // environment variable and pass the resulting string.
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
                    .bucket(&bucket)
                    .key(&event.s3_key)
                    .send()
                    .await?;

                let data = obj.body.collect().await?.into_bytes().to_vec();

                process_event(
                    event,
                    &mut producer,
                    data
                ).await?;
            }
        }
    }

    Ok(())
}

async fn process_event<P: EventProducer>(
    event: SongUploaded,
    producer: &mut P,
    data: Vec<u8>,
) -> anyhow::Result<()> {
    // Fingerprint
    let fingerprints = tokio::task::spawn_blocking(move || {
        fingerprint_pipeline(data)
    })
        .await?;


    let chunks: Vec<&[Fingerprint]> = fingerprints
        .chunks(1000)
        .collect();

    // Produce result event
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

async fn send_fingerprint_chunks<P: EventProducer>(
    producer: &mut P,
    event: FingerprintGenerated,
    chunks: Vec<&[Fingerprint]>,
) -> anyhow::Result<()> {
    let song_id = event.song_id.clone();

    for (index, chuck) in chunks.iter().enumerate() {
        let fingerprint_chunk = FingerprintChunk {
            song_id: song_id.clone(),
            index: index as u32,
            data: chuck.to_vec()
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

// Testing
pub struct MockProducer {
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