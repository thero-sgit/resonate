use std::fs;

use sqlx::SqlitePool;
use crate::aud::{fingerprints, hashing::Fingerprint};

pub struct Database {
    connection: SqlitePool
}

impl Database {
    pub async fn init() -> Result<Self, sqlx::Error> {
        let url = "sqlite:project.db";
        let connection = SqlitePool::connect(url).await?;

        sqlx::query!("PRAGMA foreign_keys = ON").execute(&connection).await?;

        Ok(
            Self {
                connection
            }
        )
    }

    pub async fn insert_song(&self, title: String) -> Result<i64, sqlx::Error> {
        Ok(sqlx::query!(
                    r#"
                    INSERT INTO songs (title)
                    VALUES (?)
                    RETURNING id
                    "#, title
                )
            .fetch_one(&self.connection)
            .await?
            .id
        )
    }

    pub async fn insert_fingerprints(&self, song_id: i64, fingerprints: &[Fingerprint]) 
    -> Result<(), sqlx::Error> {
        let mut tx = self.connection.begin().await?;

        for fingerprint in fingerprints {
            let hash = fingerprint.hash as i64;
            let frame_index = fingerprint.frame_index as i64;
            sqlx::query!(
                r#"
                INSERT INTO fingerprints (hash, song_id, time_offset)
                VALUES (?, ?, ?)
                "#,
                hash,
                song_id,
                frame_index
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn select(&self, query_fingerprints: &[Fingerprint]) -> Result<Vec<(i64, i64)>, sqlx::Error> {
        let mut matches = Vec::new();

        for (index, fingerprint) in query_fingerprints.iter().enumerate() {
            let hash = fingerprint.hash as i64;

            let database_matches = sqlx::query!(
                r#"
                SELECT song_id, time_offset
                FROM fingerprints
                WHERE hash = ?
                "#,
                hash
            )
            .fetch_all(&self.connection)
            .await?;

            for m in database_matches {
                matches.push(
                    (m.song_id, m.time_offset as i64 - fingerprint.frame_index as i64)
                );
            }

            println!("{} of {}", index, query_fingerprints.len())
        }

        Ok(matches)
    }

    pub async fn get_song(&self, song_id: i64) -> Result<String, sqlx::Error> {
        Ok(sqlx::query!(
                "SELECT title FROM songs WHERE id = ?", song_id
            )
            .fetch_one(&self.connection)
            .await?
            .title
        )
    }


    pub async fn persist(&self, path: &str, title: String) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let audio_bytes: Vec<u8> = fs::read(path).expect("Here");
        let fingerprints = fingerprints(audio_bytes);

        let song_id = self.insert_song(title).await.unwrap();

        self.insert_fingerprints(song_id, &fingerprints).await.unwrap();

        Ok(())
    }
    
}
