use std::fs;
use std::sync::OnceLock;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::client::Client;
use aws_sdk_s3::config::http::HttpResponse;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::list_buckets::{ListBucketsError, ListBucketsOutput};
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

        Environment{
            aws_client,
            s3_container,
            kafka_container: apache::Kafka::default()
        }
    }
}

pub(crate) static SHARED_ENV: OnceLock<Environment> = OnceLock::new();

async fn get_env() -> &'static Environment {
    if let Some(env) =  SHARED_ENV.get() {
        return env;
    }

    let env = Environment::setup().await;
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