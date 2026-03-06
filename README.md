# Resonate

A high-performance audio fingerprinting service with both HTTP and Kafka-based event processing. Generates compact, noise-robust fingerprints from audio files for fast and efficient audio matching and identification.

## Overview

Resonate provides a complete audio fingerprinting pipeline that converts audio files into sparse spectral hashes. These fingerprints enable fast audio matching, even for degraded or partial audio segments, without requiring full audio comparison.

**Key capabilities:**
- **Multi-format support**: MP3, WAV, FLAC, AAC (via Symphonia)
- **HTTP API**: Direct upload and synchronous fingerprint generation
- **Kafka Streaming**: Event-driven asynchronous processing with S3 integration
- **Parallel Processing**: Multi-threaded fingerprinting via Rayon
- **Compact Representation**: 64-bit hashes with minimal storage requirements

## How It Works

### Audio Fingerprinting Algorithm

The fingerprinting process consists of three stages:

#### 1. **Decoding & Normalization**
- Decodes audio from various formats using Symphonia
- Converts multi-channel audio to mono
- Resamples to 11,025 Hz for consistent processing
- Result: Normalized mono PCM stream

#### 2. **Spectral Feature Extraction**
- Splits PCM into overlapping frames (1024 samples, 50% overlap ≈ 92.8 ms/frame)
- Applies Hann windowing to reduce spectral leakage
- Computes FFT to get magnitude spectrum for each frame
- Result: Time-frequency spectrogram

#### 3. **Hash Generation**
- Identifies local spectral peaks in the spectrogram
- Pairs peaks with adjacent peaks in time and frequency
- Encodes peak relationships into compact 64-bit hashes
- Result: Sparse fingerprint set with frame indices

### Why This Approach?

This algorithm creates **robust, match-tolerant fingerprints** because:
- **Sparse representation**: Only peaks matter, noise is filtered
- **Relative encodings**: Hashes encode frequency/time deltas, not absolute values
- **Time-localized**: Frame indices enable segment matching
- **Noise-tolerant**: Works with compressed, degraded, or partial audio
- **Efficient**: Each fingerprint is just 64 bits + frame index

## Features

- **HTTP API** — Direct audio upload and fingerprint generation
- **Kafka Streaming** — Event-driven fingerprinting with S3 integration
- **Parallel Processing** — Leverages Rayon for efficient multi-threaded fingerprint generation
- **Extensible Events** — Trait-based producer allows custom event handling

## API Endpoints

### POST /fingerprint
Upload an audio file to generate fingerprints.

**Request:**
- Format: multipart form data
- Field name: `file`
- Content: audio bytes (any supported format)

**Response:**
```json
{
  "fingerprints": [
    {
      "hash": 12345678901234567,
      "frame_index": 0
    },
    {
      "hash": 98765432109876543,
      "frame_index": 512
    }
  ]
}
```

**Example:**
```bash
curl -F "file=@sample.mp3" http://localhost:8080/fingerprint
```

### GET /health
Health check endpoint that returns `healthy`.

**Example:**
```bash
curl http://localhost:8080/health
# Returns: healthy
```

## Building

Requires Rust and Cargo (1.70+).

```bash
# Build release binary
cargo build --release

# Run the server
cargo run --release
```

The server binds to `0.0.0.0:8080` by default.

## Configuration

### Environment Variables

- **`KAFKA_BROKERS`** — Kafka broker endpoints (e.g., `localhost:9092`, `broker1:9092,broker2:9092`)
  - Required for event-driven processing
- **`S3_BUCKET`** — AWS S3 bucket name for storing/retrieving audio files
  - Required for Kafka worker
  - Uses AWS SDK default credential chain (env vars, IAM role, etc.)

### Example Setup

```bash
export KAFKA_BROKERS="localhost:9092"
export S3_BUCKET="audio-files"
cargo run --release
```

## Architecture

### Core Modules

#### `src/fingerprint/`
The fingerprinting pipeline core:

- **`mod.rs`** — Orchestrates the complete fingerprinting workflow
- **`decode.rs`** — Audio decoding and resampling to 11,025 Hz mono
  - Supports MP3, WAV, FLAC, AAC formats
  - Parallel resampling via Rayon
- **`extraction.rs`** — Spectral analysis via FFT
  - Frame windowing with Hann window
  - Parallel FFT computation across frames
- **`hashing.rs`** — Peak detection and hash generation
  - Local spectral peak finding (3×3 neighborhood)
  - Sparse hash encoding of peak pairs

#### `src/server.rs`
HTTP route handlers:
- `POST /fingerprint` — Receives multipart form data, generates fingerprints
- `GET /health` — Health check

#### `src/streaming/`
Event-driven fingerprinting via Kafka:

- **`mod.rs`** — Kafka consumer/worker
  - Listens for `song_uploaded` events
  - Retrieves audio from S3
  - Publishes fingerprints to `fingerprint_generated` and `fingerprint_chunk` topics
  - Automatically commits consumed messages

- **`models.rs`** — Event schemas and producer trait
  - `SongUploaded` — Input event
  - `FingerprintGenerated` — Metadata event
  - `FingerprintChunk` — Fingerprint data (chunked)
  - `EventProducer` trait — Extensible for testing and custom implementations

### Concurrency Model

The application runs two concurrent tasks:

1. **HTTP Server** — Handles synchronous fingerprinting requests
2. **Kafka Worker** — Processes asynchronous fingerprinting events

Both tasks run concurrently via `tokio::select!` and operate independently.

### Performance Optimizations

- **Parallel resampling**: Rayon processes audio in chunks across CPU cores
- **Parallel FFT**: Each frame's FFT computed independently
- **Blocking tasks**: CPU-intensive fingerprinting moved to `tokio::task::spawn_blocking`
- **Message batching**: Kafka producer batches messages over 50ms for compression efficiency
- **Chunked output**: Large fingerprint sets split into 1000-fingerprint chunks for incremental processing

## Usage Examples

### HTTP API

#### Generate fingerprints from a local file

```bash
curl -F "file=@song.mp3" http://localhost:8080/fingerprint | jq .
```

#### Save fingerprints to file

```bash
curl -F "file=@song.mp3" http://localhost:8080/fingerprint > fingerprints.json
```

### Kafka Event Flow

#### 1. Publish a song upload event

```bash
# Publish to song_uploaded topic
{
  "song_id": "abc123",
  "s3_key": "songs/abc123.mp3"
}
```

#### 2. Listen for results

The worker will:
1. Consume the `song_uploaded` event
2. Download audio from S3: `s3://bucket/songs/abc123.mp3`
3. Generate fingerprints
4. Publish `fingerprint_generated` event with metadata
5. Publish one `fingerprint_chunk` event per 1000 fingerprints

Example `fingerprint_generated` event:
```json
{
  "song_id": "abc123",
  "total_chunks": 2
}
```

Example `fingerprint_chunk` event:
```json
{
  "song_id": "abc123",
  "index": 0,
  "data": [
    {"hash": 12345678901234567, "frame_index": 0},
    {"hash": 98765432109876543, "frame_index": 512},
    ...
  ]
}
```

## Development

### Project Structure

```
resonate/
├── src/
│   ├── lib.rs                    # Library root
│   ├── main.rs                   # Binary entrypoint
│   ├── server.rs                 # HTTP handlers
│   ├── fingerprint/
│   │   ├── mod.rs                # Pipeline orchestration
│   │   ├── decode.rs             # Audio decoding
│   │   ├── extraction.rs         # Spectral analysis
│   │   └── hashing.rs            # Hash generation
│   └── streaming/
│       ├── mod.rs                # Kafka worker
│       └── models.rs             # Event schemas
├── tests/
│   ├── integration.rs            # Integration tests
│   └── common/                   # Test utilities
├── Cargo.toml                    # Manifest
├── Dockerfile                    # Container configuration
└── README.md                     # This file
```

### Running Tests

```bash
# All tests
cargo test

# Integration tests only
cargo test --test integration

# With output
cargo test -- --nocapture
```

### Testing with Kafka

The project includes integration tests using testcontainers that spin up Kafka and S3 (LocalStack) automatically.

```bash
# Run integration tests (requires Docker)
cargo test --test integration
```

### Code Documentation

Generate and view Rust documentation:

```bash
# Generate docs
cargo doc --no-deps

# Open in browser
cargo doc --open
```

All public APIs include comprehensive doc comments with examples.

## Dependencies

Key dependencies:

- **symphonia** — Multi-format audio decoding
- **rustfft** — FFT computation
- **rubato** — High-quality audio resampling
- **rayon** — Parallel processing
- **tokio** — Async runtime
- **axum** — HTTP framework
- **rdkafka** — Kafka client
- **aws-sdk-s3** — AWS S3 integration
- **serde_json** — JSON serialization

See `Cargo.toml` for complete dependency list and versions.

## Performance Characteristics

### Fingerprinting Speed

Typical performance on modern hardware:

- **3-minute song**: ~100-200 ms (including I/O)
- **Fingerprints generated**: 2,000-5,000 per song
- **Hash size**: 64 bits + 32-bit frame index per fingerprint
- **Storage per song**: ~20-50 KB

### Scalability

- **HTTP throughput**: ~5-10 concurrent uploads (with 4+ cores)
- **Kafka processing**: Hundreds of songs/hour (with sufficient resources)
- **Horizontal scaling**: Multiple instances can process independently

### Memory Usage

- **Per-song**: ~50-200 MB (depends on audio length and bitrate)
- **Working memory**: Minimal; audio freed after fingerprinting
- **Long-running**: Memory stable; no accumulation

## Troubleshooting

### Common Issues

**Error: "Unsupported format"**
- Ensure audio file format is supported: MP3, WAV, FLAC, AAC
- Check file is not corrupted

**Error: "KAFKA_BROKERS not set"**
- Set environment variable: `export KAFKA_BROKERS="localhost:9092"`
- Verify Kafka is running and reachable

**Error: "S3_BUCKET not set"**
- Set environment variable: `export S3_BUCKET="your-bucket-name"`
- Verify AWS credentials are configured

**Slow fingerprinting**
- Ensure sufficient CPU cores available (Rayon will utilize available cores)
- Check for CPU/memory contention from other processes

## Algorithm References

The fingerprinting algorithm is inspired by Shazam's patented approach:
- Spectral peak detection for robustness
- Relative encoding for invariance
- Time-frequency pairing for efficiency
