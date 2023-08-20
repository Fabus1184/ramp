use std::collections::HashMap;

use log::warn;
use symphonia::core::{
    audio::AudioBufferRef,
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    formats::FormatOptions,
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::{MetadataOptions, MetadataRevision},
    probe::Hint,
};

use crate::song::Song;

pub fn read_audio<P, F>(path: P, mut f: F) -> anyhow::Result<Option<MetadataRevision>>
where
    P: AsRef<std::path::Path>,
    F: FnMut(AudioBufferRef) -> anyhow::Result<()> + Send + 'static,
{
    let src = std::fs::File::open(path)?;

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

    let track_id = track.id;

    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    std::thread::spawn(move || loop {
        match format_reader.next_packet() {
            Ok(packet) => {
                if packet.track_id() == track_id {
                    let data = match decoder.decode(&packet) {
                        Ok(d) => d,
                        Err(e) => {
                            warn!("Failed to decode packet {:?}", e);
                            break;
                        }
                    };

                    if f(data).is_err() {
                        break;
                    }
                }
            }
            Err(symphonia::core::errors::Error::IoError(e)) => match e.kind() {
                std::io::ErrorKind::UnexpectedEof => break,
                _ => {
                    warn!("Failed to read packet {:?}", e);
                    break;
                }
            },
            Err(e) => {
                warn!("Failed to read packet {:?}", e);
                break;
            }
        };
    });

    Ok(metadata)
}

pub fn song_from_file<P>(path: P) -> anyhow::Result<Song>
where
    P: AsRef<std::path::Path>,
{
    let src = std::fs::File::open(path)?;

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

    let track = probed
        .format
        .tracks()
        .into_iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(anyhow::anyhow!("No audio tracks found"))?;

    let duration = track
        .codec_params
        .time_base
        .ok_or(anyhow::anyhow!(
            "No time base found for track {:?}",
            track.id
        ))?
        .calc_time(track.codec_params.n_frames.ok_or(anyhow::anyhow!(
            "No frame count found for track {:?}",
            track.id
        ))?);

    let duration = duration.seconds as f32 + duration.frac as f32;

    let (standard_tags, other_tags, visuals) = metadata
        .map(|m| {
            let s = m
                .tags()
                .into_iter()
                .filter_map(|t| t.std_key.map(|k| (k.into(), t.value.clone().into())))
                .collect::<HashMap<_, _>>();

            let o = m
                .tags()
                .into_iter()
                .filter(|t| t.std_key == None)
                .map(|t| (t.key.clone(), t.value.clone().into()))
                .collect::<HashMap<_, _>>();

            let v = m
                .visuals()
                .into_iter()
                .map(|x| x.clone().into())
                .collect::<Vec<_>>();

            (s, o, v)
        })
        .unwrap_or_default();

    Ok(Song {
        standard_tags,
        other_tags,
        visuals,
        duration,
    })
}
