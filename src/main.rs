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

mod audio;
mod cache;
mod config;
mod player;
mod song;
mod tui;

pub const UNKNOWN_STRING: &'static str = "<unknown>";

fn main() {
    let config = Arc::new(Config::load("./config.json").expect("Failed to load config"));

    let _logger = WriteLogger::init(
        LevelFilter::Trace,
        simplelog::Config::default(),
        File::create(&config.log_path).expect("Failed to create log file"),
    )
    .expect("Failed to initialize logger");

    trace!("loading cache");
    let cache = Arc::new(Cache::load(&config).unwrap_or_else(|| {
        warn!("Failed to load cache, rebuilding");
        let cache = Cache::build_from_config(&config);
        cache
            .save(&config)
            .unwrap_or_else(|e| warn!("Failed to save cache {e:?}"));
        cache
    }));

    trace!("initializing player");
    let player = Arc::new(Mutex::new(
        Player::new().expect("Failed to initialize player"),
    ));

    {
        let player = player.clone();
        trace!("locking player");
        player
            .clone()
            .lock()
            .unwrap()
            .media_controls
            .attach(move |event: MediaControlEvent| {
                trace!("media control event {:?}", event);

                match event {
                    MediaControlEvent::Play => Player::play(player.clone()),
                    MediaControlEvent::Pause => Player::pause(player.clone()),
                    MediaControlEvent::Toggle => Player::play_pause(player.clone()),
                    MediaControlEvent::Next => Player::skip(player.clone()),
                    MediaControlEvent::Previous => Ok(()),
                    MediaControlEvent::Stop => Player::stop(player.clone()),
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
    tui(&config, cache, player.clone()).expect("Failed to run tui");
    trace!("tui exited");

    trace!("locking player");
    std::fs::remove_file(player.lock().unwrap().tempfile.path())
        .expect("Failed to remove tempfile");

    trace!("quitting");
}
