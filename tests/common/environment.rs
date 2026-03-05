use std::fs;
use std::sync::OnceLock;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::client::Client;
use aws_sdk_s3::primitives::ByteStream;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::ClientConfig;
use testcontainers_modules::{
    localstack::LocalStack,
    testcontainers::ImageExt,
    kafka::apache,
};
use testcontainers_modules::testcontainers::ContainerAsync;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use crate::common::orchestrator::Orchestrator;

static SHARED_ENV: OnceLock<Environment> = OnceLock::new();

pub async fn get_env() -> &'static Environment {
    if let Some(env) = SHARED_ENV.get() {
        return env;
    }

    let env = Environment::init().await;
    SHARED_ENV.get_or_init(|| env)
}

pub(crate) struct Environment {
    pub aws_client: Client,
    pub bootstrap_servers: String,
    pub orchestrator: Orchestrator,
    
    // 
    s3_container: ContainerAsync<LocalStack>,
    kafka_node: ContainerAsync<apache::Kafka>
}

impl Environment {
    pub async fn init() -> Environment {
        // AWS S3 CLIENT
        let s3_container: ContainerAsync<LocalStack> = LocalStack::default().with_env_var("SERVICES", "s3").start().await.unwrap();

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

        // upload objects
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

        // get host
        let host = kafka_node.get_host()
            .await
            .expect("Failed to get host");

        // get port
        let port = kafka_node.get_host_port_ipv4(9092)
            .await
            .expect("Failed to get port");

        let bootstrap_servers = format!("{}:{}", host, port);

        // manually create topics
        Self::create_kafka_topics(&bootstrap_servers).await;

        //  ORCHESTRATOR
        let orchestrator = Orchestrator::new(&bootstrap_servers).await;
        orchestrator.setup()
            .await;

        // return env with all test services running
        Environment{
            aws_client,
            bootstrap_servers,
            orchestrator,
            kafka_node,
            s3_container            
        }
    }

    async fn create_kafka_topics(bootstrap_servers: &String) {
        let admin: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", bootstrap_servers)
            .create()
            .expect("AdminClient creation failed!");

        let topics: Vec<NewTopic> = ["song_uploaded", "fingerprint_generated", "fingerprint_chunk"]
            .iter()
            .map(| topic | NewTopic::new(topic, 3, TopicReplication::Fixed(1)))
            .collect();

        admin
            .create_topics(&topics, &AdminOptions::new())
            .await
            .expect("Failed to create topics!");
    }
}