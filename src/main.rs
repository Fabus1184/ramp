use std::{
    fs::File,
    sync::{Arc, Mutex},
};

use cache::Cache;
use crossterm::terminal::disable_raw_mode;
use log::{error, trace, warn, LevelFilter};
use player::Player;
use simplelog::WriteLogger;
use souvlaki::MediaControlEvent;

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

    let _logger = WriteLogger::init(
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

        let player2 = player.clone();

        // passt
        let player2 = unsafe {
            std::mem::transmute::<Arc<Mutex<Player<'_>>>, Arc<Mutex<Player<'static>>>>(player2)
        };
        {
            player
                .lock()
                .unwrap()
                .media_controls
                .attach(move |event: MediaControlEvent| {
                    trace!("media control event {:?}", event);

                    match event {
                        MediaControlEvent::Play => player2.lock().unwrap().play(),
                        MediaControlEvent::Pause => player2.lock().unwrap().pause(),
                        MediaControlEvent::Toggle => player2.lock().unwrap().play_pause(),
                        MediaControlEvent::Next => todo!(),
                        MediaControlEvent::Previous => todo!(),
                        MediaControlEvent::Stop => todo!(),
                        MediaControlEvent::Seek(_) => todo!(),
                        MediaControlEvent::SeekBy(_, _) => todo!(),
                        MediaControlEvent::SetPosition(_) => todo!(),
                        MediaControlEvent::OpenUri(_) => todo!(),
                        MediaControlEvent::Raise => todo!(),
                        MediaControlEvent::Quit => todo!(),
                    }
                    .expect("Failed to handle media control event");
                })
                .expect("Failed to attach");
        }

        tui(&config, &cache, player).expect("Failed to run tui");
    })
    .map_err(|e| {
        error!("Panic: {e:?}");
        disable_raw_mode().unwrap_or(());
        std::process::exit(1);
    })
    .unwrap_or_else(|()| {});
}
