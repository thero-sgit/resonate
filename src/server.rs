//! HTTP route handlers for the Resonate service.
//!
//! Exposes the public API used by the binary to accept uploads and return
//! fingerprint results.

use axum::extract::Multipart;
use axum::{Json, Router};
use axum::routing::{get, post};
use serde::Serialize;
use crate::fingerprint::hashing::Fingerprint;
use crate::fingerprint::fingerprint_pipeline;

#[derive(Serialize)]
/// JSON response for the `/fingerprint` endpoint.
pub struct LookupResponse {
    fingerprints: Vec<Fingerprint>,
}

pub fn router() -> Router {
    Router::new()
        .route("/fingerprint", post(lookup))
        .route("/health", get(|| async { "healthy" }))
}

/// Handle multipart uploads and return generated fingerprints as JSON.
///
/// Expects a form field named `file` containing the audio payload.
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