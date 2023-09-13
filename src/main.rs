use std::{
    fs::File,
    io::{Read, Write},
    sync::{atomic::Ordering, Arc},
};

use cache::Cache;
use log::{trace, warn, LevelFilter};
use player::Player;
use simplelog::{CombinedLogger, WriteLogger};

use crate::{config::Config, tui::tui};

mod cache;
mod config;
mod decoder;
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

    let _logger = CombinedLogger::init(vec![WriteLogger::new(
        #[cfg(debug_assertions)]
        LevelFilter::Trace,
        #[cfg(not(debug_assertions))]
        LevelFilter::Info,
        simplelog::ConfigBuilder::new()
            .set_target_level(LevelFilter::Error)
            .set_location_level(LevelFilter::Error)
            .set_thread_level(LevelFilter::Error)
            .add_filter_ignore_str("symphonia")
            .build(),
        File::create(&config.log_path).expect("Failed to create log file"),
    )])
    .expect("Failed to initialize logger");

    let quit = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut f = File::open(&config.log_path).expect("Failed to open log file");
    let _quit = quit.clone();
    let handle = std::thread::spawn(move || {
        while !_quit.load(Ordering::SeqCst) {
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).unwrap();
            std::io::stdout().write_all(&buf).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
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

    let mut cache = if *config != old_config {
        trace!("config changed, rebuilding");
        Cache::build_from_config(&config)
    } else {
        cache
    };
    cache.validate();
    let cache = Arc::new(cache);

    trace!("initializing player");
    let player = Player::new(cache.clone()).expect("Failed to initialize player");

    {
        trace!("attaching media controls");
        player
            .lock()
            .unwrap()
            .attach_media_controls()
            .unwrap_or_else(|e| {
                warn!("Failed to attach media controls: {e:?}");
            });
    }

    quit.store(true, Ordering::SeqCst);
    handle.join().expect("Failed to join log thread");

    trace!("entering tui");
    tui(config.clone(), cache.clone(), player.clone()).expect("Failed to run tui");
    trace!("tui exited");

    trace!("quitting");
}
