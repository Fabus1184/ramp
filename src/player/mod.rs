use crate::{
    cache::Cache,
    song::{Song, StandardTagKey},
};
use anyhow::Context;
use log::warn;
use souvlaki::{MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};
use std::{
    collections::VecDeque,
    io::Write,
    sync::{mpsc, Arc, RwLock},
};
use symphonia::core::meta::MetadataRevision;
use tempfile::NamedTempFile;

use self::{command::Command, facade::PlayerFacade, loader::LoadedSong, playback::Playback};

pub mod command;
pub mod facade;
mod loader;
mod playback;

#[allow(clippy::large_enum_variant)]
enum InternalPlayerStatus {
    PlayingOrPaused {
        song: Song,
        metadata: Option<MetadataRevision>,
        playback: Playback,
    },
    Stopped,
}

pub struct Player {
    cache: Arc<Cache>,
    status: InternalPlayerStatus,
    queue: VecDeque<Box<std::path::Path>>,
    media_controls: MediaControls,
    command_tx: mpsc::Sender<Command>,
}

impl Player {
    /// command player to continue playing or start playing the next song
    fn play(&mut self) -> anyhow::Result<()> {
        match &self.status {
            InternalPlayerStatus::PlayingOrPaused { playback, .. } => {
                if playback.pause.load(std::sync::atomic::Ordering::Relaxed) {
                    playback
                        .pause
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                }
            }
            InternalPlayerStatus::Stopped => {}
        }

        if matches!(self.status, InternalPlayerStatus::Stopped) {
            if let Some(path) = self.queue.pop_front() {
                let song = self
                    .cache
                    .get(path)
                    .context("Failed to get song from cache")?
                    .ok_or(anyhow::anyhow!("Song not found in cache"))?
                    .as_file()
                    .context("Song is not a file")?
                    .clone();

                let loaded_song = LoadedSong::load(song.clone()).context("Failed to load song")?;

                let metadata = loaded_song.metadata.clone();
                let playback = Playback::new(self.command_tx.clone(), loaded_song)?;

                self.status = InternalPlayerStatus::PlayingOrPaused {
                    song,
                    metadata,
                    playback,
                }
            }
        }

        Ok(())
    }

    /// command player to pause
    fn pause(&mut self) -> anyhow::Result<()> {
        match &self.status {
            InternalPlayerStatus::PlayingOrPaused { playback, .. } => {
                playback
                    .pause
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            InternalPlayerStatus::Stopped => {}
        }

        Ok(())
    }

    /// command player to play if paused or pause if playing
    fn play_pause(&mut self) -> anyhow::Result<()> {
        match &self.status {
            InternalPlayerStatus::PlayingOrPaused { playback, .. } => {
                playback
                    .pause
                    .fetch_xor(true, std::sync::atomic::Ordering::Relaxed);
            }
            InternalPlayerStatus::Stopped => {}
        }

        Ok(())
    }

    /// command player to stop
    fn stop(&mut self) -> anyhow::Result<()> {
        self.status = InternalPlayerStatus::Stopped;

        Ok(())
    }

    /// command player to skip to next song
    fn skip(&mut self) -> anyhow::Result<()> {
        self.stop()?;
        self.play()?;

        Ok(())
    }

    /// add a song to the queue
    /// if the player is stopped, the song will be played
    fn enqueue<P: AsRef<std::path::Path>>(&mut self, path: P) -> anyhow::Result<()> {
        self.queue.push_back(path.as_ref().into());

        if matches!(self.status, InternalPlayerStatus::Stopped) {
            self.play()?;
        }

        Ok(())
    }

    /// remove a song from the queue
    fn dequeue(&mut self, index: usize) -> anyhow::Result<()> {
        self.queue
            .remove(index)
            .ok_or(anyhow::anyhow!(format!("No song at index {}", index)))?;

        Ok(())
    }

    /// remove all songs from the queue and stop playing
    fn clear(&mut self) -> anyhow::Result<()> {
        self.queue.clear();
        self.stop()?;

        Ok(())
    }

    pub fn run(
        cache: Arc<Cache>,
    ) -> anyhow::Result<(mpsc::Sender<Command>, Arc<RwLock<PlayerFacade>>)> {
        let media_controls = MediaControls::new(PlatformConfig {
            display_name: "rcmp",
            dbus_name: "rcmp",
            hwnd: None,
        })
        .map_err(|e| anyhow::anyhow!(format!("{:?}", e)))
        .context("Failed to create media controls")?;

        let (tx, rx) = mpsc::channel();
        let facade = Arc::new(RwLock::new(PlayerFacade::default()));

        let tx2 = tx.clone();
        let facade2 = facade.clone();
        std::thread::Builder::new()
            .name("player thread".to_string())
            .spawn(move || {
                let mut player = Player {
                    cache,
                    status: InternalPlayerStatus::Stopped,
                    queue: VecDeque::new(),
                    media_controls,
                    command_tx: tx2.clone(),
                };

                let tx = tx2.clone();
                player
                    .media_controls
                    .attach(move |event| match event {
                        souvlaki::MediaControlEvent::Play => {
                            tx.send(Command::Play).unwrap();
                        }
                        souvlaki::MediaControlEvent::Pause => {
                            tx.send(Command::Pause).unwrap();
                        }
                        souvlaki::MediaControlEvent::Toggle => {
                            tx.send(Command::PlayPause).unwrap();
                        }
                        souvlaki::MediaControlEvent::Next => {
                            tx.send(Command::Skip).unwrap();
                        }
                        souvlaki::MediaControlEvent::Previous => warn!("Previous not implemented"),
                        souvlaki::MediaControlEvent::Stop => {
                            tx.send(Command::Stop).unwrap();
                        }
                        souvlaki::MediaControlEvent::Seek(dir) => {
                            warn!("Seek {dir:?} not implemented")
                        }
                        souvlaki::MediaControlEvent::SeekBy(dir, dur) => {
                            warn!("SeekBy {dir:?} {dur:?} not implemented")
                        }
                        souvlaki::MediaControlEvent::SetPosition(mp) => {
                            warn!("SetPosition {mp:?} not implemented")
                        }
                        souvlaki::MediaControlEvent::OpenUri(uri) => {
                            warn!("OpenUri {uri:?} not implemented")
                        }
                        souvlaki::MediaControlEvent::Raise => {}
                        souvlaki::MediaControlEvent::Quit => {
                            warn!("Quit not implemented")
                        }
                    })
                    .expect("Failed to attach media controls");

                let mut cover_tempfile;
                loop {
                    match rx.recv().expect("Failed to receive Command") {
                        Command::Play => player.play().unwrap(),
                        Command::Pause => player.pause().unwrap(),
                        Command::PlayPause => player.play_pause().unwrap(),
                        Command::Skip => player.skip().unwrap(),
                        Command::Stop => player.stop().unwrap(),
                        Command::Clear => player.clear().unwrap(),
                        Command::Enqueue(path) => player.enqueue(path).unwrap(),
                        Command::Dequeue(index) => player.dequeue(index).unwrap(),
                    }

                    *facade2.write().unwrap() = PlayerFacade::from_player(&player);

                    let facade = facade2.read().unwrap();

                    cover_tempfile = NamedTempFile::new().expect("Failed to create tempfile");
                    cover_tempfile
                        .write_all(facade.current_cover().unwrap_or(&[]))
                        .expect("Failed to write cover to tempfile");

                    player
                        .media_controls
                        .set_metadata(MediaMetadata {
                            title: facade
                                .current_song()
                                .and_then(|s| s.tag_string(StandardTagKey::TrackTitle)),
                            album: facade
                                .current_song()
                                .and_then(|s| s.tag_string(StandardTagKey::Album)),
                            artist: facade
                                .current_song()
                                .and_then(|s| s.tag_string(StandardTagKey::Artist)),
                            cover_url: Some(
                                format!("file://{}", cover_tempfile.path().display()).as_str(),
                            ),
                            duration: facade.current_song().map(|s| s.duration),
                        })
                        .expect("Failed to set metadata");

                    player
                        .media_controls
                        .set_playback(match &facade.status {
                            facade::PlayerStatus::PlayingOrPaused {
                                playing_duration,
                                paused,
                                ..
                            } => {
                                if paused.load(std::sync::atomic::Ordering::Relaxed) {
                                    MediaPlayback::Paused {
                                        progress: Some(MediaPosition(
                                            *playing_duration.read().unwrap(),
                                        )),
                                    }
                                } else {
                                    MediaPlayback::Playing {
                                        progress: Some(MediaPosition(
                                            *playing_duration.read().unwrap(),
                                        )),
                                    }
                                }
                            }
                            facade::PlayerStatus::Stopped => MediaPlayback::Stopped,
                        })
                        .expect("Failed to set playback");
                }
            })
            .context("Failed to create player thread")?;

        Ok((tx, facade))
    }
}
