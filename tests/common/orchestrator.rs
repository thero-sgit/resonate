use std::time::Duration;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::error::KafkaError;
use rdkafka::message::OwnedMessage;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::producer::future_producer::Delivery;
use resonate::streaming::{create_consumer, create_producer};
use resonate::streaming::models::SongUploaded;

pub struct Orchestrator {
    pub producer: FutureProducer,
    pub consumer: StreamConsumer
}

impl Orchestrator {
    pub async fn new(brokers: &String) -> Orchestrator {
        let producer = create_producer(brokers);
        let consumer: StreamConsumer = create_consumer(brokers, "assert-group");

        Orchestrator { producer, consumer }
    }

    pub async fn setup(&self) {
        self.consumer
            .subscribe(&["fingerprint_generated"])
            .expect("Assert Group Consumer: Failed to subscribe to kafka topic!");
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
}