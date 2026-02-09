use axum::extract::Multipart;
use axum::Json;
use serde::Serialize;
use crate::fingerprint::hashing::Fingerprint;
use crate::fingerprint::pipeline;

#[derive(Serialize)]
pub struct FingerprintResponse {
    fingerprints: Vec<Fingerprint>,
}

pub async fn fingerprint(
    mut audio: Multipart,
) -> Result<Json<FingerprintResponse>, axum::http::StatusCode> {

    let mut audio_bytes = Vec::new();

    while let Some(field) = audio.next_field().await.unwrap() {
        if let Some(name) = field.name() {
            if name == "file" {
                audio_bytes = field.bytes().await.unwrap().to_vec();
            }
        }
    }

    let hashes = tokio::task::spawn_blocking(move || {
        pipeline(audio_bytes)
    })
        .await
        .unwrap();

    Ok(
        Json(FingerprintResponse {fingerprints: hashes})
    )
}