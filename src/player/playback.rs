use std::{
    collections::VecDeque,
    sync::{atomic::AtomicBool, mpsc, Arc, RwLock},
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait},
    StreamConfig,
};
use log::{debug, warn};

use super::{command::Command, loader::LoadedSong};

pub struct Playback {
    _stream: cpal::Stream,
    pub pause: Arc<AtomicBool>,
    pub played_duration: Arc<RwLock<Duration>>,
}

impl Playback {
    pub fn new(cmd: mpsc::Sender<Command>, mut song: LoadedSong) -> anyhow::Result<Self> {
        let config = StreamConfig {
            channels: song.signal_spec.channels.count() as u16,
            sample_rate: cpal::SampleRate(song.signal_spec.rate),
            buffer_size: cpal::BufferSize::Default,
        };
        debug!("Stream config: {:?}", config);

        let mut buffer = VecDeque::<f32>::new();

        let pause = Arc::new(AtomicBool::new(false));
        let playing_duration = Arc::new(RwLock::new(Duration::from_secs(0)));

        let gain_factor = song.song.gain_factor;
        let pause_stream2 = pause.clone();
        let playing_duration2 = playing_duration.clone();

        let stream = cpal::default_host()
            .default_output_device()
            .expect("Failed to get default output device")
            .build_output_stream::<f32, _, _>(
                &config,
                move |dest, _info| {
                    if pause_stream2.load(std::sync::atomic::Ordering::Relaxed) {
                        dest.fill(0.0);
                        return;
                    }

                    let mut duration = playing_duration2.write().unwrap();

                    let mut byte_count = 0;
                    while byte_count < dest.len() {
                        if buffer.len() < dest.len() {
                            let (sample_buffer, eof) = (song.decoder)().unwrap_or_else(|e| {
                                warn!("Error in decoder: {:?}", e);
                                (None, false)
                            });

                            if let Some(s) = sample_buffer {
                                buffer.extend(s.samples());
                            }

                            if eof && buffer.is_empty() {
                                cmd.send(Command::Skip).unwrap();
                                break;
                            }
                        }

                        buffer
                            .drain(..(dest.len() - byte_count).min(buffer.len()))
                            .for_each(|sample| {
                                dest[byte_count] = sample * gain_factor;
                                byte_count += 1;
                            });
                    }

                    *duration += Duration::from_secs_f64(
                        dest.len() as f64 / config.channels as f64 / config.sample_rate.0 as f64,
                    );
                },
                |e| {
                    warn!("Error in playback stream: {:?}", e);
                },
                None,
            )
            .expect("Failed to build output stream");

        Ok(Self {
            _stream: stream,
            pause,
            played_duration: playing_duration,
        })
    }
}
