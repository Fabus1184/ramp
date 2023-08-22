use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, OutputCallbackInfo, StreamConfig,
};
use log::{error, trace, warn};
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use symphonia::core::{
    audio::SignalSpec,
    meta::{MetadataRevision, StandardVisualKey},
};
use tempfile::NamedTempFile;

use std::{
    collections::VecDeque,
    io::Write,
    path::PathBuf,
    sync::{
        mpsc::{Receiver, SyncSender},
        Mutex, Weak,
    },
    time::Duration,
};

use crate::{
    audio,
    song::{Song, StandardTagKey},
};

enum StreamCommand {
    Play,
    Pause,
}

struct QueuedSong {
    song: Song,
    metadata: Option<MetadataRevision>,
    signal_spec: SignalSpec,
    receiver: Receiver<Vec<f32>>,
}

fn front_cover<'a>(metadata: &'a Option<MetadataRevision>) -> Option<&[u8]> {
    metadata
        .as_ref()
        .and_then(|m| {
            m.visuals()
                .iter()
                .find(|v| v.usage == Some(StandardVisualKey::FrontCover))
        })
        .map(|v| v.data.as_ref())
}

struct CurrentSong {
    song: Song,
    metadata: Option<MetadataRevision>,
    stream_command_sender: SyncSender<StreamCommand>,
    elapsed: Duration,
    cover_tempfile: NamedTempFile,
}

pub struct Player {
    playing: bool,
    current: Option<CurrentSong>,
    next: VecDeque<QueuedSong>,
    arc: Weak<Mutex<Player>>,
    device: Device,
    pub media_controls: MediaControls,
    pub tempfile: NamedTempFile,
}

impl Player {
    pub fn new() -> anyhow::Result<Player> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(anyhow::anyhow!("Failed to get default output device"))?;

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
            media_controls,
            tempfile: NamedTempFile::new().expect("Failed to create tempfile"),
            arc: Weak::new(),
        })
    }

    pub fn attach_arc(&mut self, weak: Weak<Mutex<Player>>) {
        self.arc = weak;
    }

    pub fn update_media_controls(
        &mut self,
        song: &Song,
        cover_tempfile: &NamedTempFile,
    ) -> anyhow::Result<()> {
        let [title, album, artist] = [
            StandardTagKey::TrackTitle,
            StandardTagKey::Album,
            StandardTagKey::TrackTitle,
        ]
        .map(|k| song.standard_tags.get(&k).map(|x| x.to_string()));

        self.media_controls
            .set_playback(MediaPlayback::Playing { progress: None })
            .map_err(|e| anyhow::anyhow!("Failed to set playback: {:?}", e))?;

        self.media_controls
            .set_metadata(MediaMetadata {
                title: title.as_ref().map(|x| x.as_str()),
                album: album.as_ref().map(|x| x.as_str()),
                artist: artist.as_ref().map(|x| x.as_str()),
                cover_url: cover_tempfile.path().to_str(),
                duration: None,
            })
            .map_err(|e| anyhow::anyhow!("Failed to set metadata: {:?}", e))?;

        Ok(())
    }

    pub fn play(&mut self) -> anyhow::Result<()> {
        if let Some(CurrentSong {
            song,
            metadata,
            stream_command_sender,
            elapsed: duration,
            cover_tempfile,
        }) = self.current.take()
        {
            trace!("play: playing current stream");

            stream_command_sender.send(StreamCommand::Play)?;
            trace!("play: sent play command");

            self.playing = true;
            self.update_media_controls(&song, &cover_tempfile)?;

            self.current = Some(CurrentSong {
                song,
                metadata,
                stream_command_sender,
                elapsed: duration,
                cover_tempfile,
            });

            Ok(())
        } else {
            trace!("play: no current stream, trying to get next");

            if let Some(QueuedSong {
                song,
                metadata,
                signal_spec,
                receiver,
            }) = self.next.pop_front()
            {
                let sender = self.spawn_stream_thread(receiver, &signal_spec, &song);

                let mut cover_tempfile = NamedTempFile::new().expect("Failed to create tempfile");
                if let Some(data) = front_cover(&metadata) {
                    cover_tempfile.write_all(data).unwrap_or_else(|e| {
                        warn!("Failed to write cover to tempfile: {:?}", e);
                    });
                }

                self.current = Some(CurrentSong {
                    song,
                    metadata,
                    stream_command_sender: sender,
                    elapsed: Duration::from_secs(0),
                    cover_tempfile,
                });

                self.play()
            } else {
                trace!("play: no next stream");
                Ok(())
            }
        }
    }

    fn spawn_stream_thread(
        &mut self,
        receiver: Receiver<Vec<f32>>,
        signal_spec: &SignalSpec,
        song: &Song,
    ) -> SyncSender<StreamCommand> {
        let gain_factor = song.gain_factor().unwrap_or(1.0);
        trace!("play: gain_factor: {}", gain_factor);

        let mut buf = Vec::new();
        let (command_tx, command_rx) = std::sync::mpsc::sync_channel::<StreamCommand>(1);

        let stream_config = StreamConfig {
            channels: signal_spec.channels.count() as u16,
            sample_rate: cpal::SampleRate(signal_spec.rate),
            buffer_size: cpal::BufferSize::Fixed(signal_spec.rate * 2),
        };
        trace!("play: stream_config: {:?}", stream_config);

        let arc = self.arc.upgrade().expect("Failed to upgrade weak player");
        let arc2 = arc.clone();

        std::thread::spawn(move || {
            trace!("locking player");
            let player = arc.lock().expect("Failed to lock player");
            let mut n = 0;

            let stream = player
                .device
                .build_output_stream(
                    &stream_config,
                    move |data: &mut [f32], _info: &OutputCallbackInfo| {
                        let now = std::time::Instant::now();
                        while buf.len() < data.len() {
                            match receiver.recv() {
                                Ok(s) => buf.extend(s.into_iter().map(|x| x * gain_factor)),
                                Err(e) => {
                                    warn!("Failed to receive sample, sender disconnected {:?}", e);

                                    {
                                        trace!("locking player");
                                        let mut player =
                                            arc2.lock().expect("Failed to lock player");
                                        player.skip().unwrap_or_else(|e| {
                                            warn!("Failed to skip song: {:?}", e);
                                        });
                                    }

                                    return;
                                }
                            }
                        }
                        trace!(
                            "receiver.recv() got {} frames, took {:?}",
                            buf.len(),
                            now.elapsed()
                        );

                        {
                            trace!("locking player");
                            let mut player = arc2.lock().expect("Failed to lock player");
                            n += data.len();
                            if let Some(CurrentSong {
                                elapsed: ref mut duration,
                                ..
                            }) = player.current.as_mut()
                            {
                                *duration = Duration::from_secs_f32(n as f32 / (2.0 * 48_000.0));
                            }
                        }

                        data.copy_from_slice(buf.drain(0..data.len()).as_slice());
                    },
                    |e| {
                        warn!("Output stream error {:?}", e);
                    },
                    Some(Duration::from_secs_f32(1.0)),
                )
                .expect("Failed to build output stream");

            drop(player);

            loop {
                trace!(
                    "thread {:?} waiting for command",
                    std::thread::current().id()
                );
                match command_rx.recv() {
                    Ok(s) => match s {
                        StreamCommand::Play => stream.play().expect("Failed to play output stream"),
                        StreamCommand::Pause => {
                            stream.pause().expect("Failed to pause output stream")
                        }
                    },
                    Err(e) => {
                        warn!("Failed to receive command, sender disconnected {:?}", e);
                        {
                            trace!("locking player");
                            let mut player = arc.lock().expect("Failed to lock player");
                            player.current = None;
                            player.playing = false;
                        }
                        break;
                    }
                }
            }

            trace!("play: stream thread exiting");
        });

        command_tx
    }

    pub fn stop(&mut self) -> anyhow::Result<()> {
        trace!("stopping player");
        self.current = None;
        self.playing = false;
        self.media_controls
            .set_playback(MediaPlayback::Stopped)
            .map_err(|e| anyhow::anyhow!("Failed to set playback: {:?}", e))?;

        Ok(())
    }

    pub fn pause(&mut self) -> anyhow::Result<()> {
        trace!("pausing player");
        if let Some(CurrentSong {
            stream_command_sender,
            ..
        }) = self.current.as_ref()
        {
            trace!("pause: pausing stream");
            stream_command_sender.send(StreamCommand::Pause)?;
            self.playing = false;
            self.media_controls
                .set_playback(MediaPlayback::Paused { progress: None })
                .map_err(|e| anyhow::anyhow!("Failed to set playback: {:?}", e))?;
        } else {
            trace!("pause: no stream to pause");
        }

        trace!("pause: done");

        Ok(())
    }

    pub fn queue(
        &mut self,
        song: Song,
        path: &Vec<String>,
        file_name: &String,
    ) -> anyhow::Result<()> {
        let path = path
            .into_iter()
            .chain(std::iter::once(file_name))
            .fold(PathBuf::new(), |acc, p| acc.join(p));

        // TODO: adaptively choose buffer size based on signal spec and config
        let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<f32>>(32);

        let mut last = std::time::Instant::now();
        let (metadata, signal_spec, _handle) = audio::read_audio(&path, move |data| {
            trace!(
                "read_audio: got {} frames, took {:?}",
                data.len(),
                last.elapsed()
            );
            tx.send(data.to_vec())?;
            last = std::time::Instant::now();
            Ok(())
        })?;

        self.next.push_back(QueuedSong {
            song,
            metadata,
            signal_spec,
            receiver: rx,
        });

        if self.current.is_none() {
            trace!("queue: playing queued song");
            self.play()
        } else {
            Ok(())
        }
    }

    pub fn play_pause(&mut self) -> anyhow::Result<()> {
        match self.playing {
            true => self.pause(),
            false => self.play(),
        }
    }

    pub fn clear(&mut self) -> anyhow::Result<()> {
        trace!("clearing queue");
        self.next.clear();
        self.stop()
    }

    pub fn skip(&mut self) -> anyhow::Result<()> {
        trace!("skipping song");
        self.stop()?;
        self.play()?;

        Ok(())
    }

    pub fn current(&self) -> Option<&Song> {
        self.current
            .as_ref()
            .map(|CurrentSong { ref song, .. }| song)
    }

    pub fn current_time(&self) -> Option<&Duration> {
        self.current.as_ref().map(
            |CurrentSong {
                 elapsed: ref duration,
                 ..
             }| duration,
        )
    }

    pub fn current_cover(&self) -> Option<&[u8]> {
        self.current
            .as_ref()
            .and_then(|cs| front_cover(&cs.metadata))
    }

    pub fn nexts(&self) -> impl Iterator<Item = &Song> {
        self.next.iter().map(|QueuedSong { ref song, .. }| song)
    }

    pub fn attach_media_controls(&mut self) -> anyhow::Result<()> {
        let weak = self.arc.clone();
        self.media_controls
            .attach(move |event: MediaControlEvent| {
                trace!("media control event {:?}", event);

                let arc = weak.upgrade().expect("Failed to upgrade weak player");
                let mut player = arc.lock().expect("Failed to lock player");

                match event {
                    MediaControlEvent::Play => player.play(),
                    MediaControlEvent::Pause => player.pause(),
                    MediaControlEvent::Toggle => player.play_pause(),
                    MediaControlEvent::Next => player.skip(),
                    MediaControlEvent::Previous => Ok(()),
                    MediaControlEvent::Stop => player.stop(),
                    MediaControlEvent::Seek(_) => todo!(),
                    MediaControlEvent::SeekBy(_, _) => todo!(),
                    MediaControlEvent::SetPosition(_) => todo!(),
                    MediaControlEvent::OpenUri(_) => Ok(()),
                    MediaControlEvent::Raise => Ok(()),
                    MediaControlEvent::Quit => Ok(()),
                }
                .unwrap_or_else(|e| {
                    error!("Failed to handle media control event: {:?}", e);
                });
            })
            .map_err(|e| anyhow::anyhow!("Failed to attach media controls: {:?}", e))?;

        Ok(())
    }
}
