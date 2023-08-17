use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, OutputCallbackInfo, StreamConfig,
};
use log::{error, info, trace, warn};
use souvlaki::{MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use symphonia::core::{
    audio::{Channels, SampleBuffer, SignalSpec},
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    formats::FormatOptions,
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::{MetadataOptions, StandardVisualKey},
    probe::Hint,
};
use tempfile::NamedTempFile;

use crate::Song;

use std::{
    collections::VecDeque,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, SyncSender},
    time::Duration,
};

enum StreamCommand {
    Play,
    Pause,
}

pub struct Player<'a> {
    playing: bool,
    current: Option<(
        &'a Song,
        PathBuf,
        SyncSender<StreamCommand>,
        Option<(Box<[u8]>, String)>,
    )>,
    next: VecDeque<(&'a Song, PathBuf, Receiver<SampleBuffer<f32>>)>,
    device: Device,
    stream_config: StreamConfig,
    pub media_controls: MediaControls,
    pub tempfile: NamedTempFile,
}

fn song_cover(path: &Path) -> Option<(Box<[u8]>, String)> {
    let file = std::fs::File::open(path)
        .map_err(|e| {
            warn!("Failed to open file {:?} {:?}", path, e);
        })
        .ok()?;
    let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();
    let hint = Hint::new();

    let mut result = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| {
            warn!("Failed to read metadata from {:?} {:?}", path, e);
        })
        .ok()?;

    let mut metadata = result.format.metadata();

    let meta = metadata
        .skip_to_latest()
        .ok_or_else(|| {
            warn!("Failed to skip to latest metadata from {:?}", path);
        })
        .ok()?;

    meta.visuals()
        .into_iter()
        .filter(|v| v.usage == Some(StandardVisualKey::FrontCover))
        .filter(|v| ["image/png", "image/jpeg"].contains(&v.media_type.as_str()))
        .map(|v| v.clone())
        .map(|v| (v.data, v.media_type))
        .next()
}

impl<'a> Player<'a> {
    pub fn new() -> Result<Player<'a>, ()> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or_else(|| {
                warn!("Failed to get default output device");
            })?;

        let stream_config = device
            .default_output_config()
            .map_err(|e| {
                warn!("Failed to get default output config {:?}", e);
            })?
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

    pub fn play(&mut self) -> Result<(), String> {
        trace!("play: current is_some: {:?}", self.current.is_some());

        match self.current.as_ref() {
            Some((song, path, tx, cover)) => {
                let cover_url = if let Some((data, _filetype)) = cover {
                    trace!(
                        "play: writing cover of {} (checksum {}) to tempfile",
                        path.display(),
                        data.iter().fold(0, |acc, x| acc ^ x)
                    );

                    self.tempfile = NamedTempFile::new().expect("Failed to create tempfile");

                    self.tempfile
                        .write_all(&data)
                        .expect("Failed to write cover to tempfile");

                    self.tempfile.flush().expect("Failed to flush tempfile");

                    self.tempfile
                        .as_file()
                        .sync_all()
                        .expect("Failed to sync tempfile");

                    trace!("play: wrote cover to {:?}", self.tempfile.path());
                    Some(format!("file://{}", self.tempfile.path().display()))
                } else {
                    None
                };

                tx.send(StreamCommand::Play)
                    .map_err(|e| format!("Failed to play output stream {:?}", e))
                    .and_then(|_| {
                        self.playing = true;
                        self.media_controls
                            .set_playback(MediaPlayback::Playing { progress: None })
                            .map_err(|e| format!("Failed to set playback {:?}", e))?;

                        self.media_controls
                            .set_metadata(MediaMetadata {
                                title: song.title.as_ref().map(|s| s.as_str()),
                                album: song.album.as_ref().map(|s| s.as_str()),
                                artist: song.artist.as_ref().map(|s| s.as_str()),
                                cover_url: cover_url.as_ref().map(|s| s.as_str()),
                                duration: None,
                            })
                            .map_err(|e| format!("Failed to set metadata {:?}", e))
                    })
            }
            None => {
                trace!("play: no current stream, trying to get next");

                if let Some((song, path, rx)) = self.next.pop_front() {
                    trace!("play: got next song {:?}", song);
                    let gain_factor = song.gain.unwrap_or(1.0);
                    let mut buf = Vec::new();

                    let (ctx, crx) = std::sync::mpsc::sync_channel::<StreamCommand>(0);

                    let stream_config = self.stream_config.clone();
                    let device =
                        unsafe { std::mem::transmute::<&'_ Device, &'static Device>(&self.device) };

                    std::thread::spawn(move || {
                        let stream = device
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

                                data.copy_from_slice(buf.drain(0..data.len()).as_slice());
                            },
                            |e| {
                                warn!("Output stream error {:?}", e);
                            },
                            Some(Duration::from_secs_f32(1.0)),
                        )
                        .map_err(|e| format!("Failed to build output stream {:?}", e)).expect("Failed to build output stream");

                        loop {
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
                                    return;
                                }
                            }
                        }
                    });

                    let cover = song_cover(&path);

                    self.current = Some((song, path, ctx, cover));
                    self.play()
                } else {
                    trace!("play: no next song");
                    Ok(())
                }
            }
        }
    }

    pub fn stop(&mut self) -> Result<(), String> {
        trace!("stopping player");
        self.current = None;
        self.playing = false;
        self.media_controls
            .set_playback(MediaPlayback::Stopped)
            .map_err(|e| format!("Failed to set playback {:?}", e))
    }

    pub fn pause(&mut self) -> Result<(), String> {
        trace!("pausing player");
        if let Some((_, _, ctx, _)) = self.current.as_ref() {
            trace!("pause: pausing stream");
            ctx.send(StreamCommand::Pause)
                .map_err(|e| format!("Failed to pause output stream {:?}", e))
                .and_then(|()| {
                    self.playing = false;
                    self.media_controls
                        .set_playback(MediaPlayback::Paused { progress: None })
                        .map_err(|e| format!("Failed to set playback {:?}", e))
                })?;
        } else {
            trace!("pause: no stream to pause");
        }

        Ok(())
    }

    pub fn queue(
        &mut self,
        song: &'a Song,
        path: &Vec<String>,
        file_name: &String,
    ) -> Result<(), String> {
        trace!("queueing song {:?}", song);

        let path = path
            .into_iter()
            .chain(std::iter::once(file_name))
            .fold(PathBuf::new(), |acc, p| acc.join(p));

        let src = std::fs::File::open(path.clone()).map_err(|e| format!("{:?}", e))?;

        let mss = MediaSourceStream::new(Box::new(src), MediaSourceStreamOptions::default());
        let probed = symphonia::default::get_probe()
            .format(
                &Hint::new(),
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| format!("{:?}", e))?;

        let mut format_reader = probed.format;
        let track = format_reader
            .tracks()
            .into_iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| "No valid tracks found".to_string())?;

        let track_id = track.id;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .unwrap_or_else(|e| {
                warn!("Failed to create decoder {:?}", e);
                std::process::exit(1);
            });

        let (tx, rx) = std::sync::mpsc::sync_channel::<SampleBuffer<f32>>(512);

        let handle = std::thread::spawn(move || {
            'l: loop {
                match format_reader.next_packet() {
                    Ok(packet) => {
                        if packet.track_id() == track_id {
                            let data = match decoder.decode(&packet) {
                                Ok(d) => d,
                                Err(e) => {
                                    error!("Failed to decode packet {:?}", e);
                                    break 'l;
                                }
                            };

                            let mut b = SampleBuffer::new(
                                data.capacity() as u64,
                                SignalSpec::new(
                                    48_000,
                                    Channels::FRONT_LEFT.union(Channels::FRONT_RIGHT),
                                ),
                            );
                            b.copy_interleaved_ref(data);

                            match tx.send(b) {
                                Err(e) => {
                                    error!("Failed to send sample {:?}", e);
                                    break 'l;
                                }
                                _ => {}
                            }
                        }
                    }

                    Err(e) => {
                        warn!("Error reading packet {:?}", e);
                        break;
                    }
                }
            }

            info!("thread {:?} finished", std::thread::current().id());
        });
        info!("thread {:?} started, detaching now", handle.thread().id());
        drop(handle);

        self.next.push_back((song, path, rx));

        if self.current.is_none() {
            self.play()?;
        }

        Ok(())
    }

    pub fn play_pause(&mut self) -> Result<(), String> {
        match self.playing {
            true => self.pause(),
            false => self.play(),
        }
    }

    pub fn clear(&mut self) -> Result<(), String> {
        self.next.clear();
        self.stop()
    }

    pub fn skip(&mut self) -> Result<(), String> {
        trace!("skipping song");
        self.stop()?;
        self.play()
    }

    pub fn current(&self) -> Option<(&'a Song, Option<&(Box<[u8]>, String)>)> {
        self.current
            .as_ref()
            .map(|&(s, _, _, ref c)| (s, c.as_ref()))
    }

    pub fn nexts(&self) -> impl Iterator<Item = &'a Song> + '_ {
        self.next.iter().map(|&(s, _, _)| s)
    }
}
