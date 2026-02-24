use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::client::Client;
use testcontainers_modules::{
    localstack::LocalStack,
    testcontainers::ImageExt,
    kafka::apache,
};
use testcontainers_modules::testcontainers::ContainerAsync;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

pub struct Environment {
    pub aws_client: Client,
    s3_container: ContainerAsync<LocalStack>,
    kafka_container: apache::Kafka
}

impl Environment {
    pub async fn setup() -> Environment {
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
        aws_client.create_bucket()
            .bucket("test-bucket")
            .send().await.expect("Failed to create bucket!");

        Environment{
            aws_client,
            s3_container,
            kafka_container: apache::Kafka::default()
        }
    }
}