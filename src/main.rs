use std::{
    fs::File,
    sync::{Arc, Mutex},
};

use cache::Cache;
use log::{trace, warn, LevelFilter};
use player::Player;
use simplelog::WriteLogger;

use crate::{config::Config, tui::tui};

mod audio;
mod cache;
mod config;
mod player;
mod song;
mod tui;

fn main() {
    let config_dir = dirs::config_dir()
        .expect("Unable to find config directory")
        .join("ramp");

    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).unwrap_or_else(|e| {
            eprintln!("Failed to create config directory: {e:?}");
        });
    }

    let config = Arc::new(
        Config::load(&config_dir.join("config.json")).unwrap_or_else(|e| {
            eprintln!("Failed to load config, using default: {e:?}");
            let config = Config::default_from_config_dir(&config_dir);
            config
                .save(&config_dir.join("config.json"))
                .unwrap_or_else(|e| {
                    eprintln!("Failed to save config: {e:?}");
                });
            config
        }),
    );

    let _logger = WriteLogger::init(
        //#[cfg(debug_assertions)]
        LevelFilter::Trace,
        //#[cfg(not(debug_assertions))]
        //LevelFilter::Info,
        simplelog::ConfigBuilder::new()
            .set_target_level(LevelFilter::Error)
            .set_location_level(LevelFilter::Error)
            .set_thread_level(LevelFilter::Error)
            .add_filter_ignore_str("symphonia")
            .build(),
        File::create(&config.log_path).expect("Failed to create log file"),
    )
    .unwrap_or_else(|e| {
        eprintln!("Failed to initialize logger: {e:?}");
    });

    trace!("loading cache");
    let (cache, old_config) = Cache::load(&config).unwrap_or_else(|e| {
        warn!("Failed to load cache: {e:?}, using default");

        let cache = Cache::build_from_config(&config);

        trace!("saving cache");
        cache
            .save(&config)
            .unwrap_or_else(|e| warn!("Failed to save cache {e:?}"));

        (cache, (*config).clone())
    });

    let cache = if *config != old_config {
        trace!("config changed, rebuilding");
        Cache::build_from_config(&config)
    } else {
        cache
    };
    let cache = Arc::new(cache);

    trace!("initializing player");
    let player = Arc::new(Mutex::new(
        Player::new().expect("Failed to initialize player"),
    ));
    let player_weak = Arc::downgrade(&player);

    {
        trace!("locking player");
        let mut player = player.lock().unwrap();

        trace!("attachin weak ref");
        player.attach_arc(player_weak);

        trace!("attaching media controls");
        player.attach_media_controls().unwrap_or_else(|e| {
            warn!("Failed to attach media controls: {e:?}");
        });
    }

    trace!("entering tui");
    tui(config.clone(), cache.clone(), player.clone()).expect("Failed to run tui");
    trace!("tui exited");

    trace!("quitting");
}
