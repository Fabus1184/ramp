use ratatui::{
    style::{Modifier, Stylize},
    widgets::Row,
};

use crate::{
    cache::CacheEntry,
    song::{Song, StandardTagKey},
};

use super::UNKNOWN_STRING;

pub const HEADER: fn() -> Row<'static> = || {
    Row::new(["Track #️⃣ ", "Artist 🧑‍🎤 ", "Title / File 🎶 ", "Album 🖼️ "])
        .add_modifier(Modifier::BOLD)
};

const KEYS: [StandardTagKey; 4] = [
    StandardTagKey::TrackNumber,
    StandardTagKey::Artist,
    StandardTagKey::TrackTitle,
    StandardTagKey::Album,
];

pub fn cache_row<'a>(key: &str, value: &CacheEntry) -> Row<'a> {
    Row::new(match value {
        CacheEntry::File { ref song, .. } => {
            let track = song
                .standard_tags
                .get(&StandardTagKey::TrackNumber)
                .map(|s| s.to_string())
                .unwrap_or(UNKNOWN_STRING.to_string());

            let artist = song
                .standard_tags
                .get(&StandardTagKey::Artist)
                .map(|s| s.to_string())
                .unwrap_or(UNKNOWN_STRING.to_string());

            let title = song
                .standard_tags
                .get(&StandardTagKey::TrackTitle)
                .map(|s| s.to_string())
                .unwrap_or(key.to_string());

            let album = song
                .standard_tags
                .get(&StandardTagKey::Album)
                .map(|s| s.to_string())
                .unwrap_or(UNKNOWN_STRING.to_string());

            [track, artist, title, album]
        }
        CacheEntry::Directory { .. } => ["", "", key, ""].map(|s| s.to_string()),
    })
}

pub fn song_row<'a>(song: &Song) -> Row<'a> {
    Row::new(KEYS.map(|k| {
        song.standard_tags
            .get(&k)
            .map(|v| v.to_string())
            .unwrap_or(UNKNOWN_STRING.to_string())
    }))
}
