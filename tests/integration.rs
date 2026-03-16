use std::time::Duration;
use rdkafka::consumer::Consumer;
use rdkafka::producer::future_producer::Delivery;
use resonate::streaming::{create_consumer, create_producer, run_kafka_worker};
use resonate::streaming::models::KafkaProducer;
use common::get_env;

mod common;
#[tokio::test]
async fn test_end_to_end_kafka_worker() {
    println!("Starting test");
    // set up
    let env = get_env().await;


    let own_producer = KafkaProducer {
        inner: create_producer(&env.bootstrap_servers),
    };
    let consumer = create_consumer(&env.bootstrap_servers, "fingerprint_group");
    consumer.subscribe(&["song_uploaded"]).expect("Error occurred while subscribing");

    // spawn kafka worker and keep the handle so we can inspect errors
    let kafka_handle = tokio::spawn(run_kafka_worker(
        consumer,
        own_producer,
        env.aws_client.clone(),
        "test-bucket".to_string(),
    ));

    // orchestrator send event
    let delivery: Delivery = env.orchestrator
        .send("test-song-id", "aud.mp3")
        .await
        .expect("Error while sending message");

    // test
    let result = tokio::time::timeout(
        Duration::from_secs(30), async {
            return Some(env.orchestrator.receive().await);
        }
    )
    .await;

    // check worker for errors, using select to race the worker check with message wait
    let worker_error: Option<String> = tokio::time::timeout(Duration::from_secs(2), async {
        match kafka_handle.await {
            Ok(Err(e)) => Some(format!("kafka worker error: {}", e)),
            Err(e) => Some(format!("kafka worker join error: {:?}", e)),
            Ok(Ok(())) => None,
        }
    })
    .await
    .ok()
    .flatten();

    if let Some(err_msg) = worker_error {
        panic!("{}", err_msg);
    }

    let timeout_result = result.expect("timeout waiting for fingerprint_generated event");
    let event = timeout_result.expect("did not receive fingerprint_generated event");
    assert_eq!("test-song-id", event.event.song_id);
    assert_eq!(event.event.total_chunks as u64, event.chucks.len() as u64);
}