//! Resonate binary: HTTP audio fingerprinting service with Kafka integration
//!
//! This binary starts an HTTP server that accepts audio uploads and generates fingerprints,
//! while concurrently running a Kafka worker that processes fingerprinting events from a
//! message queue and stores results in S3.
//!
//! # Startup
//!
//! The server:
//! - Initializes distributed tracing via `tracing_subscriber`
//! - Configures AWS S3 client for retrieving audio files
//! - Creates a Kafka consumer listening to `song_uploaded` topic
//! - Starts the HTTP server on `0.0.0.0:8080`
//! - Runs the Kafka worker and HTTP server concurrently
//!
//! # Environment Variables
//!
//! - `KAFKA_BROKERS`: Kafka broker endpoints (e.g., `localhost:9092`)
//! - `S3_BUCKET`: AWS S3 bucket name for storing/retrieving audio files
//!
//! # Dependencies
//!
//! - Axum for HTTP routing
//! - Tokio for async runtime and task spawning
//! - rdkafka for Kafka integration
//! - AWS SDK for S3 access

use aws_config::BehaviorVersion;
use rdkafka::consumer::Consumer;
use crate::streaming::{create_consumer, create_producer, run_kafka_worker};
use crate::streaming::models::KafkaProducer;

mod fingerprint;
mod server;
mod streaming;

/// Application entrypoint. Initializes Kafka and HTTP server, running both concurrently.
///
/// # Errors
///
/// Returns an error if:
/// - Environment variables `KAFKA_BROKERS` or `S3_BUCKET` are not set
/// - Kafka consumer creation or subscription fails
/// - TCP listener binding fails
/// - AWS configuration fails
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
        s3_client,
        s3_bucket,
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
