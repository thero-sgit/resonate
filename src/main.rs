//! Small HTTP server exposing the fingerprint API.
//!
//! The binary provides a minimal Axum-based API that accepts audio uploads
//! and returns generated fingerprints.

use aws_config::BehaviorVersion;
use rdkafka::consumer::Consumer;
use crate::streaming::{create_consumer, create_producer, run_kafka_worker, KafkaProducer};

mod fingerprint;
mod server;
mod streaming;

/// Application entrypoint. Binds to `0.0.0.0:8080` and serves routes.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // kafka setup
    let brokers = std::env::var("KAFKA_BROKERS")?;
    let s3_bucket = std::env::var("S3_BUCKET")?;

    let aws_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    let consumer = create_consumer(&brokers, "fingerprint-group");
    consumer.subscribe(&["song_uploaded"])?;

    let producer = KafkaProducer {
        inner: create_producer(&brokers)
    };

    // spawn kafka worker
    let kafka_handle = tokio::spawn(run_kafka_worker(
        consumer,
        producer,
        s3_client.clone(),
        s3_bucket.clone(),
    ));

    // http server
    let app = server::router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    let server = axum::serve(listener, app);

    // run server and kafka concurrently
    tokio::select! {
        _ = kafka_handle => {},
        _ = server => {},
    }

    Ok(())
}
