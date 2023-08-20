use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, OutputCallbackInfo, StreamConfig,
};
use log::{trace, warn};
use souvlaki::{MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use symphonia::core::audio::{Channels, SampleBuffer, SignalSpec};
use tempfile::NamedTempFile;

use std::{
    collections::VecDeque,
    io::Write,
    path::PathBuf,
    sync::{
        mpsc::{Receiver, SyncSender},
        Arc, Mutex,
    },
    time::Duration,
};

use crate::{song::{Song, StandardTagKey}, audio};

enum StreamCommand {
    Play,
    Pause,
}

pub struct Player {
    playing: bool,
    current: Option<(Song, PathBuf, SyncSender<StreamCommand>, Duration)>,
    next: VecDeque<(Song, PathBuf, Receiver<SampleBuffer<f32>>)>,
    device: Device,
    stream_config: StreamConfig,
    pub media_controls: MediaControls,
    pub tempfile: NamedTempFile,
}

impl Player {
    pub fn new() -> anyhow::Result<Player> {
        let device = cpal::default_host().default_output_device().ok_or(anyhow::anyhow!("Failed to get default output device"))?;

        let stream_config = device
            .default_output_config()
            ?
            .into();

        let media_controls = MediaControls::new(PlatformConfig {
            display_name: "rcmp",
            dbus_name: "rcmp",
            hwnd: None,
        })
        .expect("Failed to create media controls");

        Ok(Player {
            playing: false,
            current: None,
            next: VecDeque::new(),
            device,
            stream_config,
            media_controls,
            tempfile: NamedTempFile::new().expect("Failed to create tempfile"),
        })
    }

    pub fn play(player: Arc<Mutex<Player>>) -> anyhow::Result<()> {
        {
            trace!("locking player");
            trace!(
                "play: current is_some: {:?}",
                player.lock().unwrap().current.is_some()
            );
        }

        trace!("locking player");
        if player.lock().unwrap().current.is_some() {
            let (song, tx, cover_url) = {
                trace!("locking player");
                let mut player = player.lock().unwrap();

                let (song, _, _, _) = player.current.clone().unwrap();
                let cover_url = song.front_cover().map(|v| {
                    player.tempfile = NamedTempFile::new().expect("Failed to create tempfile");

                    player
                        .tempfile
                        .write_all(&v.data)
                        .expect("Failed to write cover to tempfile");
                    player.tempfile.flush().expect("Failed to flush tempfile");
                    player
                        .tempfile
                        .as_file()
                        .sync_all()
                        .expect("Failed to sync tempfile");

                    trace!("play: wrote cover to {:?}", player.tempfile.path());
                    format!("file://{}", player.tempfile.path().display())
                });

                let (_, _, tx, _) = player.current.clone().unwrap();
                (song, tx, cover_url)
            };

            tx.send(StreamCommand::Play)?;
        
            let mut player = player.lock().unwrap();
            player.playing = true;
            player
                .media_controls
                .set_playback(MediaPlayback::Playing { progress: None })
                .map_err(|e| anyhow::anyhow!("Failed to set playback: {:?}", e))
                ?;

            let title = song
                .standard_tags
                .get(&StandardTagKey::TrackTitle)
                .map(|x| x.to_string());
            let album = song
                .standard_tags
                .get(&StandardTagKey::Album)
                .map(|x| x.to_string());
            let artist = song
                .standard_tags
                .get(&StandardTagKey::TrackTitle)
                .map(|x| x.to_string());

            player
                .media_controls
                .set_metadata(MediaMetadata {
                    title: title.as_ref().map(|x| x.as_str()),
                    album: album.as_ref().map(|x| x.as_str()),
                    artist: artist.as_ref().map(|x| x.as_str()),
                    cover_url: cover_url.as_ref().map(|x| x.as_str()),
                    duration: None,
                })
                .map_err(|e| anyhow::anyhow!("Failed to set metadata: {:?}", e))
                ?;

            trace!("play: sent play command");
            Ok(())
        } else {
            trace!("play: no current stream, trying to get next");

            trace!("locking player");
            let mut x = player.lock().unwrap();
            if let Some((song, path, rx)) = x.next.pop_front() {
                let gain_factor = song
                    .standard_tags
                    .get(&StandardTagKey::ReplayGainTrackGain)
                    .map(|x| x.to_string())
                    .and_then(|x| x.strip_suffix(" dB").map(|x| x.to_string()))
                    .and_then(|x| x.parse::<f32>().ok())
                    .map(|x| 10.0f32.powf(x / 20.0))
                    .unwrap_or(1.0);

                trace!("play: gain_factor: {}", gain_factor);

                let mut buf = Vec::new();

                let (ctx, crx) = std::sync::mpsc::sync_channel::<StreamCommand>(0);

                let stream_config = x.stream_config.clone();
                trace!("play: stream_config: {:?}", stream_config);

                let player2 = player.clone();
                let player3 = player.clone();
                std::thread::spawn(move || {
                    let mut n = 0;
                    let stream = {
                        trace!("locking player");
                        player2.lock().unwrap().device
                        .build_output_stream(
                            &stream_config,
                                move |data: &mut [f32], _info: &OutputCallbackInfo| {
                                    while buf.len() < data.len() {
                                        match rx.recv() {
                                            Ok(s) => buf.extend(
                                                s.samples().into_iter().map(|x| x * gain_factor),
                                            ),
                                            Err(e) => {
                                                warn!("Failed to receive sample, sender disconnected {:?}",e);
                                                return;
                                            }
                                        }
                                    }
                                    
                                    {
                                        n += data.len();
                                        if let Some((_, _, _, ref mut d)) = player3.lock().unwrap().current.as_mut() { 
                                            *d = Duration::from_secs_f32(n as f32 /  (2.0 * 48_000.0));
                                        }
                                    }
                                    
                                    data.copy_from_slice(buf.drain(0..data.len()).as_slice());
                                },
                                |e| {
                                    warn!("Output stream error {:?}", e);
                                },
                                Some(Duration::from_secs_f32(1.0)),
                            )
                            .expect("Failed to build output stream")
                    };

                    loop {
                        trace!(
                            "thread {:?} waiting for command",
                            std::thread::current().id()
                        );
                        match crx.recv() {
                            Ok(s) => match s {
                                StreamCommand::Play => {
                                    stream.play().expect("Failed to play output stream")
                                }
                                StreamCommand::Pause => {
                                    stream.pause().expect("Failed to pause output stream")
                                }
                            },
                            Err(e) => {
                                warn!("Failed to receive sample, sender disconnected {:?}", e);
                                // double free possible
                                // player2.lock().unwrap().current = None;
                                // player2.lock().unwrap().playing = false;
                                break;
                            }
                        }
                    }

                    trace!("play: stream thread exiting");
                });

                drop(x);
                {
                    trace!("locking player");
                    player.clone().lock().unwrap().current =
                        Some((song, path, ctx, Duration::from_secs(0)))
                };
                Player::play(player.clone())
            } else {
                trace!("play: no next song");
                Ok(())
            }
        }
    }

    pub fn stop(player: Arc<Mutex<Player>>) -> anyhow::Result<()> {
        trace!("stopping player");
        trace!("locking player");
        player.lock().unwrap().current = None;
        trace!("locking player");
        player.lock().unwrap().playing = false;
        trace!("locking player");
        player
            .lock()
            .unwrap()
            .media_controls
            .set_playback(MediaPlayback::Stopped)
            .map_err(|e| anyhow::anyhow!("Failed to set playback: {:?}", e))?;

        Ok(())
    }

    pub fn pause(player: Arc<Mutex<Player>>) -> anyhow::Result<()> {
        trace!("pausing player");
        trace!("locking player");
        let mut player = player.lock().unwrap();
        if let Some((_, _, ref ctx, _)) = player.current {
            trace!("pause: pausing stream");
            ctx.send(StreamCommand::Pause)?;
            player.playing = false;
            player
                .media_controls
                .set_playback(MediaPlayback::Paused { progress: None })
                .map_err(|e| anyhow::anyhow!("Failed to set playback: {:?}", e))?;
                
        } else {
            trace!("pause: no stream to pause");
        }

        trace!("pause: done");

        Ok(())
    }

    pub fn queue(
        player: Arc<Mutex<Player>>,
        song: Song,
        path: &Vec<String>,
        file_name: &String,
    ) -> anyhow::Result<()> {
        let path = path
            .into_iter()
            .chain(std::iter::once(file_name))
            .fold(PathBuf::new(), |acc, p| acc.join(p));

        let (tx, rx) = std::sync::mpsc::sync_channel::<SampleBuffer<f32>>(512);

        audio::read_audio(&path, move |data| {
            trace!("read_audio: got {} frames", data.frames());
            let mut b = SampleBuffer::new(
                data.capacity() as u64,
                SignalSpec::new(
                    48_000,
                    Channels::FRONT_LEFT.union(Channels::FRONT_RIGHT),
                ),
            );
            b.copy_interleaved_ref(data);
            tx.send(b)?;

            Ok(())
        })?;


        let is_none = {
            trace!("locking player");
            let mut player = player.lock().unwrap();
            player.next.push_back((song, path, rx));
            player.current.is_none()
        };

        if is_none {
            trace!("queue: playing queued song");
            Player::play(player)
        } else {
            Ok(())
        }
    }

    pub fn play_pause(player: Arc<Mutex<Player>>) -> anyhow::Result<()> {
        trace!("locking player");
        let playing = { player.clone().lock().unwrap().playing };
        match playing {
            true => Player::pause(player),
            false => Player::play(player),
        }
    }

    pub fn clear(player: Arc<Mutex<Player>>) -> anyhow::Result<()> {
        trace!("clearing queue");
        trace!("locking player");
        player.lock().unwrap().next.clear();
        Player::stop(player)
    }

    pub fn skip(player: Arc<Mutex<Player>>) -> anyhow::Result<()> {
        trace!("skipping song");
        Player::stop(player.clone())?;
        Player::play(player)
    }

    pub fn current(&self) -> Option<&Song> {
        self.current.as_ref().map(|(s, _, _, _)| s)
    }

    pub fn current_time(&self) -> Option<&Duration> {
        self.current.as_ref().map(|(_, _, _, t)| t)
    }

    pub fn nexts(&self) -> impl Iterator<Item = &Song> {
        self.next.iter().map(|(s, _, _)| s)
    }
}
