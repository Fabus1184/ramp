use std::{fs::File, sync::Arc};

use anyhow::Context;
use cache::Cache;
use log::{info, trace, warn, LevelFilter};
use simplelog::{CombinedLogger, WriteLogger};

use crate::{config::Config, player::Player, tui::tui};

mod cache;
mod config;
mod player;
mod song;
mod tui;

fn main() -> anyhow::Result<()> {
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

    CombinedLogger::init(vec![WriteLogger::new(
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
    .context("Failed to initialize logger")?;
    info!("Logger initialized");

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

    let mut cache = if config.search_directories != old_config.search_directories
        || config.extensions != old_config.extensions
    {
        info!("config changed, rebuilding");
        let cache = Cache::build_from_config(&config);
        cache
            .save(&config)
            .unwrap_or_else(|e| warn!("Failed to save cache {e:?}"));
        cache
    } else {
        cache
    };
    cache.validate();
    let cache = Arc::new(cache);

    trace!("initializing player");
    let (cmd, player) = Player::run(cache.clone()).context("Failed to initialize player")?;

    trace!("entering tui");
    tui(config.clone(), cache.clone(), cmd, player).context("Error in tui")?;
    trace!("tui exited");

    Ok(())
}
