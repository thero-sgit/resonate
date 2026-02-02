use std::{collections::HashMap};

pub mod database;

pub fn elect_id(matches: Vec<(i64, i64)>) -> i64 {
    let mut vote_map: HashMap<(i64, i64), usize> = HashMap::new();

    for (song_id, delta) in matches {
        *vote_map.entry(
            (song_id, delta),
        ).or_default() += 1;
    }

    let (best_match, max_votes) = vote_map
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .unwrap();

    best_match.0
}