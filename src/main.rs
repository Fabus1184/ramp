use std::{
    fs::File,
    sync::{Arc, Mutex},
};

use cache::Cache;
use log::{trace, warn, LevelFilter};
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
    let config = Arc::new(Config::load("./config.json").expect("Failed to load config"));

    let _logger = WriteLogger::init(
        LevelFilter::Trace,
        simplelog::Config::default(),
        File::create(&config.log_path).expect("Failed to create log file"),
    )
    .expect("Failed to initialize logger");

    trace!("loading cache");
    let cache = Cache::load(&config).unwrap_or_else(|| {
        warn!("Failed to load cache, rebuilding");
        let mut cache = Cache::empty();
        cache.cache_files(&config);
        cache
            .save(&config)
            .unwrap_or_else(|e| warn!("Failed to save cache {e:?}"));
        cache
    });

    trace!("initializing player");
    let player = Mutex::new(Player::new().expect("Failed to initialize player"));
    let player2 = unsafe {
        std::mem::transmute::<&'_ Mutex<Player<'_>>, &'static Mutex<Player<'static>>>(&player)
    };

    {
        trace!("attaching media controls: lock");
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
                    MediaControlEvent::Next => player2.lock().unwrap().skip(),
                    MediaControlEvent::Previous => Ok(()),
                    MediaControlEvent::Stop => player2.lock().unwrap().stop(),
                    MediaControlEvent::Seek(_) => todo!(),
                    MediaControlEvent::SeekBy(_, _) => todo!(),
                    MediaControlEvent::SetPosition(_) => todo!(),
                    MediaControlEvent::OpenUri(_) => Ok(()),
                    MediaControlEvent::Raise => Ok(()),
                    MediaControlEvent::Quit => Ok(()),
                }
                .expect("Failed to handle media control event");
            })
            .expect("Failed to attach");
    }
    trace!("attached media controls: unlock");

    trace!("running tui");
    tui(&config, &cache, &player).expect("Failed to run tui");
    trace!("tui exited");

    std::fs::remove_file(player.lock().unwrap().tempfile.path())
        .expect("Failed to remove tempfile");

    trace!("quitting");
}
