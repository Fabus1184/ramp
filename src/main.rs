use std::fs::File;

use cache::Cache;
use crossterm::terminal::disable_raw_mode;
use log::{error, warn, LevelFilter};
use player::Player;
use simplelog::WriteLogger;

use crate::{config::Config, tui::tui};

mod cache;
mod config;
mod player;
mod tui;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Song {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    year: Option<String>,
    track: Option<String>,
    gain: Option<f32>,
}

fn main() {
    let config = Config::load("./config.json").expect("Failed to load config");

    let _ = WriteLogger::init(
        LevelFilter::Trace,
        simplelog::Config::default(),
        File::create(&config.log_path).expect("Failed to create log file"),
    )
    .expect("Failed to initialize logger");

    std::panic::catch_unwind(|| {
        let cache = Cache::load(&config).unwrap_or_else(|| {
            warn!("Failed to load cache, rebuilding");
            let mut cache = Cache::empty();
            cache.cache_files(&config);
            cache
                .save(&config)
                .unwrap_or_else(|e| warn!("Failed to save cache {e:?}"));
            cache
        });

        let player = Player::new(&config).expect("Failed to initialize player");

        tui(&config, &cache, player).expect("Failed to run tui");
    })
    .map_err(|e| {
        error!("Panic: {e:?}");
        disable_raw_mode().unwrap_or(());
        std::process::exit(1);
    })
    .unwrap_or_else(|()| {});
}
