use ratatui::{
    style::{Modifier, Style},
    widgets::Row,
};

use crate::{
    cache::Cache,
    song::{Song, StandardTagKey},
    UNKNOWN_STRING,
};

pub const HEADER: fn() -> Row<'static> = || {
    Row::new(["Track #️⃣ ", "Artist 🧑‍🎤 ", "File / Title 🎶 ", "Album 🖼️ "])
        .style(Style::default().add_modifier(Modifier::BOLD))
};

const KEYS: [StandardTagKey; 4] = [
    StandardTagKey::TrackNumber,
    StandardTagKey::Artist,
    StandardTagKey::TrackTitle,
    StandardTagKey::Album,
];

pub fn cache_row<'a>(key: &String, value: &Cache) -> Row<'a> {
    Row::new(match value {
        Cache::File { ref song, .. } => KEYS.map(|k| {
            song.standard_tags
                .get(&k)
                .map(|v| v.to_string())
                .unwrap_or(UNKNOWN_STRING.to_string())
        }),
        Cache::Directory { .. } => ["", "", key.as_str(), ""].map(|s| s.to_string()),
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
