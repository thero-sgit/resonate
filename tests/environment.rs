use std::fs;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::client::Client;
use aws_sdk_s3::primitives::ByteStream;
use testcontainers_modules::{
    localstack::LocalStack,
    testcontainers::ImageExt,
    kafka::apache,
};
use testcontainers_modules::testcontainers::ContainerAsync;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

pub struct Environment {
    pub aws_client: Client,
    pub kafka_node: ContainerAsync<apache::Kafka>,
    
    // private aws service container
    s3_container: ContainerAsync<LocalStack>,
}

impl Environment {
    pub async fn init() -> Environment {
        // AWS S3 CLIENT
        let s3_container = LocalStack::default().with_env_var("SERVICES", "s3").start().await.unwrap();

        let host_port = s3_container.get_host_port_ipv4(4566).await.unwrap();
        let entrypoint_url = format!("http://127.0.0.1:{}", host_port);

        let config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url(&entrypoint_url)
            .region(Region::new("us-east-1"))
            .credentials_provider(
                aws_sdk_s3::config::Credentials::new(
                    "test", "test", None, None, "localstack"
                )
            ).load().await;

        let aws_client = Client::new(&config);
        let bucket = "test-bucket";

        // create bucket
        aws_client.create_bucket()
            .bucket(bucket)
            .send().await.expect("Failed to create bucket!");

        // upload object
        let key = "aud.mp3";
        let audio = fs::read("tests/test_assets/aud.mp3").unwrap();
        aws_client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(ByteStream::from(audio))
            .send()
            .await.expect("Failed to upload object");

        // APACHE KAFKA
        let kafka_node = apache::Kafka::default()
            .start()
            .await
            .unwrap();

        // return env with all test services running
        Environment{
            aws_client,
            kafka_node,
            s3_container            
        }
    }
}