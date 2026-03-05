use std::time::Duration;
use futures::StreamExt;
use rdkafka::consumer::{Consumer, DefaultConsumerContext, MessageStream, StreamConsumer};
use rdkafka::error::KafkaError;
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::producer::future_producer::Delivery;
use resonate::streaming::{create_consumer, create_producer};
use resonate::streaming::models::{FingerprintChunk, FingerprintGenerated, SongUploaded};

pub struct FingerprintGeneratedResult {
    pub event: FingerprintGenerated,
    pub chucks: Vec<FingerprintChunk>,
}

pub struct Orchestrator {
    pub producer: FutureProducer,
    pub consumer: StreamConsumer,
    buffer_consumer: StreamConsumer
}

impl Orchestrator {
    pub async fn new(brokers: &String) -> Orchestrator {
        let producer = create_producer(brokers);
        let consumer: StreamConsumer = create_consumer(brokers, "assert-group");
        let buffer_consumer = create_consumer(brokers, "assert-group");

        Orchestrator { producer, consumer, buffer_consumer }
    }

    pub async fn setup(&self) {
        self.consumer
            .subscribe(&["fingerprint_generated"])
            .expect("Assert Group Consumer: Failed to subscribe to kafka topic!");

        self.buffer_consumer
            .subscribe(&["fingerprint_chunk"])
            .expect("Assert Group Consumer (Buffer Consumer): Failed to subscribe to kafka topic!");
    }

    pub async fn send(&self, id: &str, key: &str) -> Result<Delivery, (KafkaError, OwnedMessage)> {
        let payload = SongUploaded::new(id.to_string(), key.to_string()).as_json();

        let sent = self.producer
            .send(
                FutureRecord::to("song_uploaded")
                    .key(key)
                    .payload(&payload),
                Duration::from_secs(0)
            )
        .await?;

        Ok(sent)
    }

    pub async fn receive(&self) -> FingerprintGeneratedResult {
        let mut stream = self.consumer.stream();
        let mut event: FingerprintGenerated = FingerprintGenerated {song_id: "none".to_string(), total_chunks: 0};
        let mut chunks: Vec<FingerprintChunk> = vec![];

        // listen to fingerprint generated event
        while let Some(Ok(event_message)) = stream.next().await {
            if let Some(payload) = event_message.payload() {
                event = serde_json::from_slice(payload).unwrap();
                chunks = self.collect_chunks(event.song_id.clone(), event.total_chunks).await;
                 if Some(&chunks).is_some() {break};
            }
        }

        FingerprintGeneratedResult { event, chucks: chunks }
    }

    async fn collect_chunks(&self, event_song_id: String, total_chunks: usize) -> Vec<FingerprintChunk> {
        let mut buffer: Vec<FingerprintChunk> = Vec::new();
        let mut buffer_stream = self.buffer_consumer.stream();

        while let Some(Ok(record)) = buffer_stream.next().await {
            if let Some(record) = record.payload() {
                let chunk: FingerprintChunk = serde_json::from_slice(record).unwrap();

                if chunk.song_id == event_song_id {
                    buffer.push(chunk);
                }

                // check if we have all chunks
                if buffer.len() == total_chunks {
                    buffer.sort_by_key(|chunk| chunk.index);
                    break;
                }

            }
        }

        buffer
    }
}