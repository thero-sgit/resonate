use std::sync::OnceLock;
use std::time::Duration;
use aws_sdk_s3::config::http::HttpResponse;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::list_buckets::{ListBucketsError, ListBucketsOutput};
use rdkafka::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use crate::environment::Environment;

mod environment;

pub(crate) static SHARED_ENV: OnceLock<Environment> = OnceLock::new();

async fn get_env() -> &'static Environment {
    if let Some(env) =  SHARED_ENV.get() {
        return env;
    }

    let env = Environment::init().await;
    SHARED_ENV.get_or_init(|| env)
}

#[tokio::test]
async fn test_s3local_stack_starts() {
    let env = get_env().await;

    let result: Result<ListBucketsOutput, SdkError<ListBucketsError, HttpResponse>> = env.aws_client
        .list_buckets()
        .send()
        .await;

    assert!(result.is_ok());

    if let Some(buckets) = result.unwrap().buckets {
        let bucket_name = buckets[0]
            .name
            .clone()
            .unwrap();
        assert_eq!(bucket_name, "test-bucket");
    }
}

#[tokio::test]
async fn test_s3_upload_exists() {
    let env = get_env().await;

    let object = env.aws_client
        .get_object()
        .bucket("test-bucket")
        .key("aud.mp3")
        .send()
        .await;

    assert!(object.is_ok());
}

#[tokio::test]
async fn test_kafka_connection() {
    let env = get_env().await;
    let node = &env.kafka_node;

    let host = node.get_host()
        .await
        .expect("Failed to get host");

    let port = node.get_host_port_ipv4(9092)
        .await
        .expect("Failed to get port");

    let bootstrap_servers = format!("{}:{}", host, port);

    // set up config
    let mut client_config = ClientConfig::new();
    client_config.set("bootstrap.servers", &bootstrap_servers);
    client_config.set("client.id", "connection-test");
    client_config.set("metadata.request.timeout.ms", "5000");

    let consumer: BaseConsumer = client_config
        .create()
        .expect("Failed to create consumer");

    let metadata = consumer
        .fetch_metadata(None, Duration::from_secs(5))
        .expect("Failed to fetch metadata");

    assert!(!metadata.brokers().is_empty(), "No brokers found in the cluster");
}