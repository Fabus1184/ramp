use anyhow::Context;

use log::{debug, trace};
use symphonia::core::{
    audio::{SampleBuffer, SignalSpec},
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    errors::Error,
    formats::FormatOptions,
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::{MetadataOptions, MetadataRevision},
    probe::Hint,
};

use crate::song::Song;

pub struct LoadedSong {
    pub song: Song,
    pub metadata: Option<MetadataRevision>,
    pub signal_spec: SignalSpec,
    pub decoder: Box<dyn FnMut() -> anyhow::Result<(Option<SampleBuffer<f32>>, bool)> + Send>,
}

impl LoadedSong {
    pub fn load(song: Song) -> anyhow::Result<Self> {
        let src = std::fs::File::open(song.path.as_ref()).context(format!(
            "Failed to open file {}",
            song.path.to_string_lossy()
        ))?;

        let mss = MediaSourceStream::new(Box::new(src), MediaSourceStreamOptions::default());
        let mut probed = symphonia::default::get_probe().format(
            &Hint::new(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let metadata = {
            let mut meta = probed.format.metadata();
            meta.skip_to_latest().cloned()
        };

        let mut format_reader = probed.format;

        let track = format_reader
            .tracks()
            .into_iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(anyhow::anyhow!("No audio tracks found"))?;

        let codec_params = track.codec_params.clone();
        debug!("Codec params: {:?}", codec_params);
        let track_id = track.id;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())?;

        let signal_spec = SignalSpec::new(
            codec_params
                .sample_rate
                .ok_or(anyhow::anyhow!("No sample rate"))?,
            codec_params
                .channels
                .ok_or(anyhow::anyhow!("No channels"))?,
        );
        debug!("Signal spec: {:?}", signal_spec);

        let signal_spec2 = signal_spec.clone();
        let decoder = move || match format_reader.next_packet() {
            Ok(packet) => {
                if packet.track_id() == track_id {
                    let data = match decoder.decode(&packet) {
                        Ok(d) => d,
                        Err(e) => {
                            anyhow::bail!("Failed to decode packet {:?}", e);
                        }
                    };

                    let mut sample_buffer = SampleBuffer::new(data.capacity() as u64, signal_spec2);
                    sample_buffer.copy_interleaved_ref(data);

                    trace!(
                        "Decoded packet for track {} ({} bytes)",
                        packet.track_id(),
                        packet.data.len()
                    );

                    Ok((Some(sample_buffer), false))
                } else {
                    trace!(
                        "Skipping packet for track {} ({} bytes)",
                        packet.track_id(),
                        packet.data.len()
                    );
                    Ok((None, false))
                }
            }
            Err(Error::IoError(e)) if e.to_string() == "end of stream" => Ok((None, true)),
            Err(e) => {
                anyhow::bail!("Failed to read packet {:?}", e);
            }
        };

        Ok(Self {
            song,
            metadata,
            signal_spec,
            decoder: Box::new(decoder),
        })
    }
}
