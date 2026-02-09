# Resonate

A minimal HTTP service for generating compact audio fingerprints from uploaded audio files.

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

## Architecture

The fingerprint pipeline is implemented in `src/fingerprint/`:

- **`decode.rs`** — Decodes audio formats (via Symphonia) and resamples to mono at 11,025 Hz.
- **`extraction.rs`** — Splits PCM into overlapping frames, applies a Hann window, and computes FFT magnitudes.
- **`hashing.rs`** — Finds local spectral peaks and generates compact 64-bit hashes by pairing peaks.
- **`mod.rs`** — Orchestrates the full pipeline.

The HTTP routes are in `src/routes.rs`, and the main server setup is in `src/main.rs`.

## Usage Example

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
