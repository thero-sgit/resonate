use std::fs;


use crate::{aud::{fingerprints, hashing::Fingerprint}};

mod aud;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let song = "assets/Isibusiso_10s_Snippet.mp3";
    let audio_bytes = fs::read(song).unwrap();

    let _fingerprints: Vec<Fingerprint> = fingerprints(audio_bytes);

    Ok(())  
}