use std::fs;

use crate::{aud::fingerprints, persistance::{database::Database, elect_id}};

mod aud;
mod persistance;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let db = Database::init().await?;
    let song = "assets/Isibusiso_10s_Snippet.mp3";


    let audio_bytes: Vec<u8> = fs::read(song)?;

    println!("Fingerprinting");
    let fingerprints = fingerprints(audio_bytes);
    println!("Done");

    let stripped = &fingerprints[20_000..20_150];
    
    println!("DB Look-up..");
    let matches = db.select(stripped).await.unwrap();
    println!("Done");


    let best_match_id = elect_id(matches);

    let title = db.get_song(best_match_id).await.unwrap();

    println!("Song = {}", title);

    Ok(())  
}