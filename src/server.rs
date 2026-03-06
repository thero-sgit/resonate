//! HTTP route handlers for the Resonate service.
//!
//! Exposes the public API used by the binary to accept audio uploads and return
//! fingerprint results. The module provides two endpoints:
//!
//! - `POST /fingerprint` — Upload audio file and get fingerprints
//! - `GET /health` — Health check endpoint

use axum::extract::Multipart;
use axum::{Json, Router};
use axum::routing::{get, post};
use serde::Serialize;
use crate::fingerprint::hashing::Fingerprint;
use crate::fingerprint::fingerprint_pipeline;

/// JSON response structure for the fingerprint API endpoint.
///
/// Contains a list of all fingerprints generated from the uploaded audio.
#[derive(Serialize)]
pub struct LookupResponse {
    /// Vector of fingerprints, each containing a hash and frame index
    fingerprints: Vec<Fingerprint>,
}

/// Constructs the HTTP router with all service endpoints.
///
/// # Returns
///
/// An Axum `Router` configured with POST and GET routes.
///
/// # Routes
///
/// - `POST /fingerprint` - Audio fingerprinting endpoint
/// - `GET /health` - Health check endpoint
pub fn router() -> Router {
    Router::new()
        .route("/fingerprint", post(lookup))
        .route("/health", get(|| async { "healthy" }))
}

/// Handle multipart file uploads and generate fingerprints.
///
/// Processes audio files uploaded via multipart form data and runs the
/// fingerprinting pipeline in a blocking task to avoid blocking the async runtime.
///
/// # Arguments
///
/// - `audio` — Multipart form data containing the audio file in a `file` field
///
/// # Returns
///
/// - `Ok(Json<LookupResponse>)` — Successfully generated fingerprints
/// - `Err(StatusCode)` — Error processing the request (returns 400)
///
/// # Request Format
///
/// Expects multipart form data with a field named `file` containing the audio bytes.
/// Audio formats supported: MP3, WAV, FLAC, AAC (via Symphonia decoder).
///
/// # Example
///
/// ```bash
/// curl -F "file=@sample.mp3" http://localhost:8080/fingerprint
/// ```
///
/// # Response
///
/// ```json
/// {
///   "fingerprints": [
///     {"hash": 12345678901234567, "frame_index": 0},
///     {"hash": 98765432109876543, "frame_index": 512}
///   ]
/// }
/// ```
async fn lookup(
    mut audio: Multipart,
) -> Result<Json<LookupResponse>, axum::http::StatusCode> {

    let mut audio_bytes = Vec::new();

    while let Some(field) = audio.next_field().await.unwrap() {
        if let Some(name) = field.name() {
            if name == "file" {
                audio_bytes = field.bytes().await.unwrap().to_vec();
            }
        }
    }

    let hashes = tokio::task::spawn_blocking(move || {
        fingerprint_pipeline(audio_bytes)
    })
        .await
        .unwrap();

    Ok(Json(LookupResponse {fingerprints: hashes}))
}