# Resonate

A minimal service for generating compact audio fingerprints from uploaded audio files. Supports both HTTP uploads and event-driven processing via Kafka.

## Features

- **HTTP API** — Direct audio upload and fingerprint generation
- **Kafka Streaming** — Event-driven fingerprinting with S3 integration
- **Parallel Processing** — Leverages Rayon for efficient multi-threaded fingerprint generation

## Endpoints

- `POST /fingerprint` — Upload an audio file to generate fingerprints. Expects multipart form data with a field named `file` containing the audio bytes. Returns a JSON list of fingerprint objects.
- `GET /health` — Health check endpoint that returns `healthy`.

## Building

Requires Rust and Cargo. Build with:

```bash
cargo build --release
```

Run with:

```bash
cargo run --release
```

The server binds to `0.0.0.0:8080` by default.

### Environment Variables

- `KAFKA_BROKERS` — Kafka broker endpoints (e.g., `localhost:9092`)
- `S3_BUCKET` — AWS S3 bucket name for storing/retrieving audio files

## Architecture

The fingerprint pipeline is implemented in `src/fingerprint/`:

- **`decode.rs`** — Decodes audio formats (via Symphonia) and resamples to mono at 11,025 Hz.
- **`extraction.rs`** — Splits PCM into overlapping frames, applies a Hann window, and computes FFT magnitudes.
- **`hashing.rs`** — Finds local spectral peaks and generates compact 64-bit hashes by pairing peaks.
- **`mod.rs`** — Orchestrates the full pipeline.

The HTTP routes are in `src/server.rs`, and the main server setup is in `src/main.rs`.

### Kafka Integration

The `src/streaming/` module provides event-driven fingerprinting:

- **`mod.rs`** — Kafka consumer and worker that listens for `song_uploaded` events, retrieves audio from S3, generates fingerprints, and publishes `fingerprint_generated` events.
- **`models.rs`** — Event schemas (`SongUploaded`, `FingerprintGenerated`) and the `EventProducer` trait for extensible event handling.

## Usage Example

### HTTP Endpoint

```bash
curl -F "file=@sample.mp3" http://localhost:8080/fingerprint
```

Response:
```json
{
  "fingerprints": [
    {
      "hash": 12345678901234567,
      "frame_index": 0
    },
    ...
  ]
}
```

### Kafka Event Flow

1. A `song_uploaded` event is published to Kafka:
   ```json
   {
     "song_id": "abc123",
     "s3_key": "songs/abc123.mp3",
     "uploaded_at": 1707600000
   }
   ```

2. The Kafka worker retrieves the audio from S3, generates fingerprints, and publishes a `fingerprint_generated` event:
   ```json
   {
     "song_id": "abc123",
     "fingerprints": [
       {"hash": 12345678901234567, "frame_index": 0},
       ...
     ],
     "generated_at": 1707600005
   }
   ```

## Development

- Run tests: `cargo test`
- Format code: `cargo fmt`
- Lint: `cargo clippy`

## Dependencies

Key dependencies include:
- **Axum** — async web framework
- **Symphonia** — audio decoding
- **RustFFT** — Fast Fourier Transform
- **Rubato** — audio resampling
- **Rayon** — data parallelism
- **Serde** — JSON serialization
- **Tokio** — async runtime
- **rdkafka** — Kafka client
- **AWS SDK for Rust** — S3 integration
