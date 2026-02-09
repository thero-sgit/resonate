-- Add migration script here

-- create songs table
CREATE TABLE songs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL UNIQUE
);

-- create fingerprints table
CREATE TABLE fingerprints (
    hash INTEGER NOT NULL,
    song_id INTEGER NOT NULL,
    time_offset INTEGER NOT NULL,
    FOREIGN KEY (song_id) REFERENCES songs (id)
);

CREATE INDEX index_fingerprints_hash ON fingerprints (hash);
CREATE INDEX index_fingerprints_song ON fingerprints (song_id);