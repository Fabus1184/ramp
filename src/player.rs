use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, OutputCallbackInfo, Stream, StreamConfig,
};
use log::{error, info, warn};
use symphonia::core::{
    audio::{Channels, SampleBuffer, SignalSpec},
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    formats::FormatOptions,
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::MetadataOptions,
    probe::Hint,
};

use crate::{config::Config, Song};

use std::{collections::VecDeque, path::PathBuf, sync::mpsc::Receiver};

pub struct Player<'a> {
    playing: bool,
    current: Option<(&'a Song, Stream)>,
    next: VecDeque<(&'a Song, Receiver<f32>)>,
    config: &'a Config,
    device: Device,
    stream_config: StreamConfig,
}

impl<'a> Player<'a> {
    pub fn new(config: &'a Config) -> Result<Self, ()> {
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

        Ok(Player {
            playing: false,
            current: None,
            next: VecDeque::new(),
            config,
            device,
            stream_config,
        })
    }

    pub fn play(&mut self) -> Result<(), String> {
        match &self.current {
            Some((_, stream)) => stream
                .play()
                .map_err(|e| format!("Failed to play output stream {:?}", e))
                .map(|()| {
                    self.playing = true;
                }),
            None => {
                if let Some((song, rx)) = self.next.pop_front() {
                    let gain_factor = song.gain.unwrap_or(1.0);

                    let stream = self
                        .device
                        .build_output_stream(
                            &self.stream_config,
                            move |data: &mut [f32], _info: &OutputCallbackInfo| {
                                data.iter_mut().for_each(|d| {
                                    *d = rx.recv().map(|s| s * gain_factor).unwrap_or_else(|e| {
                                        warn!("Buffer underrun: {e:?}",);
                                        0.0
                                    })
                                });
                            },
                            |e| {
                                warn!("Output stream error {:?}", e);
                            },
                            None,
                        )
                        .map_err(|e| format!("Failed to build output stream {:?}", e))?;

                    self.current = Some((song, stream));

                    self.play()
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn stop(&mut self) {
        self.current = None;
        self.playing = false;
    }

    pub fn pause(&mut self) -> Result<(), String> {
        if let Some((_, stream)) = &self.current {
            stream
                .pause()
                .map_err(|e| format!("Failed to pause output stream {:?}", e))
                .map(|()| {
                    self.playing = false;
                })?;
        }

        Ok(())
    }

    pub fn queue(
        &mut self,
        song: &'a Song,
        path: &Vec<String>,
        file_name: &String,
    ) -> Result<(), String> {
        let path = path
            .into_iter()
            .chain(std::iter::once(file_name))
            .fold(PathBuf::new(), |acc, p| acc.join(p));

        let src = std::fs::File::open(path).map_err(|e| format!("{:?}", e))?;

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

        let (tx, rx) = std::sync::mpsc::sync_channel::<f32>(2 * 48_000);

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

                            for s in b.samples() {
                                match tx.send(*s) {
                                    Err(e) => {
                                        error!("Failed to send sample {:?}", e);
                                        break 'l;
                                    }
                                    _ => {}
                                }
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

        self.next.push_back((song, rx));

        if !self.playing {
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

    pub fn clear(&mut self) {
        self.next.clear();
        self.stop();
    }

    pub fn skip(&mut self) -> Result<(), String> {
        self.stop();
        self.play()
    }

    pub fn current(&self) -> Option<&Song> {
        self.current.as_ref().map(|(s, _)| *s)
    }

    pub fn nexts(&self) -> impl Iterator<Item = &Song> {
        self.next.iter().map(|(s, _)| *s)
    }
}
