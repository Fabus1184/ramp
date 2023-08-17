use crate::{config::Config, Song};
use std::{
    collections::HashMap,
    fs::Metadata,
    path::{Path, PathBuf},
    time::SystemTime,
};

use log::warn;
use symphonia::core::{
    formats::FormatOptions,
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::{MetadataOptions, StandardTagKey},
    probe::Hint,
};
use walkdir::WalkDir;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum Cache {
    File { modified: SystemTime, song: Song },
    Directory { children: HashMap<String, Cache> },
}

impl Clone for Cache {
    fn clone(&self) -> Cache {
        panic!("Cache::clone called")
    }
}

impl Cache {
    pub fn empty() -> Self {
        Cache::Directory {
            children: HashMap::new(),
        }
    }

    pub fn songs(&self) -> Box<dyn Iterator<Item = &Song> + '_> {
        match self {
            Cache::File { song, .. } => Box::new(std::iter::once(song)),
            Cache::Directory { children } => Box::new(children.values().flat_map(|c| c.songs())),
        }
    }

    fn insert_file(&mut self, path: &Path, meta: Metadata, song: Song) -> Result<(), String> {
        match self {
            Cache::File { .. } => panic!("Cache::insert_file called on Cache::File"),
            Cache::Directory { children } => {
                if path.components().count() == 1 {
                    let name = path
                        .file_name()
                        .ok_or_else(|| format!("Failed to get file name from path {:?}", path))?
                        .to_str()
                        .ok_or_else(|| format!("Failed to convert file name to string {:?}", path))?
                        .to_string();

                    let modified = meta.modified().unwrap_or_else(|e| {
                        warn!("Failed to read modified time {:?}", e);
                        SystemTime::UNIX_EPOCH
                    });

                    children.insert(name, Cache::File { modified, song });

                    Ok(())
                } else {
                    let next_dir = path
                        .components()
                        .next()
                        .ok_or_else(|| {
                            format!("Failed to get next directory from path {:?}", path)
                        })?
                        .as_os_str()
                        .to_str()
                        .ok_or_else(|| {
                            format!("Failed to convert next directory to string {:?}", path)
                        })?
                        .to_string();

                    let next_path = path
                        .components()
                        .skip(1)
                        .fold(PathBuf::new(), |acc, p| acc.join(p.as_os_str()));

                    children
                        .entry(next_dir)
                        .or_insert_with(|| Cache::Directory {
                            children: HashMap::new(),
                        })
                        .insert_file(&next_path, meta, song)
                }
            }
        }
    }

    pub fn load(config: &Config) -> Option<Self> {
        let s = std::fs::read_to_string(&config.cache_path)
            .map_err(|e| {
                warn!("Failed to read cache file {:?}", e);
            })
            .ok()?;

        let mut ds = serde_json::Deserializer::from_str(s.as_str());
        ds.disable_recursion_limit();
        let ds = serde_stacker::Deserializer::new(&mut ds);

        serde::Deserialize::deserialize(ds)
            .map_err(|e| {
                warn!("Failed to deserialize cache file {:?}", e);
            })
            .ok()
    }

    pub fn save(&self, config: &Config) -> Result<(), String> {
        let s = serde_json::to_string(&self)
            .map_err(|e| format!("Failed to serialize cache {:?}", e))?;
        std::fs::write(&config.cache_path, s)
            .map_err(|e| format!("Failed to write cache file {:?}", e))?;

        Ok(())
    }

    pub fn get(&self, path: &Vec<String>) -> Option<&Cache> {
        if path.is_empty() {
            Some(self)
        } else {
            let next = &path[0];
            let rest = path[1..].to_vec();

            match self {
                Cache::File { .. } => panic!("Cache::get called on Cache::File"),
                Cache::Directory { children } => children.get(next)?.get(&rest),
            }
        }
    }

    pub fn cache_files(&mut self, config: &Config) {
        config
            .search_directories
            .iter()
            .flat_map(|d| WalkDir::new(d))
            .flat_map(|d| (d.map_err(|e| warn!("Failed to read directory {:?}", e))))
            .filter(|d| d.file_type().is_file())
            .filter_map(|d| {
                d.metadata()
                    .map(|m| (d, m))
                    .map_err(|e| warn!("Failed to read metadata {:?}", e))
                    .ok()
            })
            .filter(|(d, _)| {
                d.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|e| config.extensions.contains(e))
                    .unwrap_or(false)
            })
            .filter_map(|(f, m)| {
                let path = f.path();

                let file = std::fs::File::open(path)
                    .map_err(|e| {
                        warn!("Failed to open file {:?} {:?}", path, e);
                    })
                    .ok()?;
                let mss =
                    MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

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

                let [title, artist, album, year, track, gain]: [Option<String>; 6] = [
                    StandardTagKey::TrackTitle,
                    StandardTagKey::Artist,
                    StandardTagKey::Album,
                    StandardTagKey::ReleaseDate,
                    StandardTagKey::TrackNumber,
                    StandardTagKey::ReplayGainTrackGain,
                ]
                .into_iter()
                .map(|t| {
                    meta.tags()
                        .into_iter()
                        .find(|t2| t2.std_key == Some(t))
                        .map(|t| t.value.to_string())
                })
                .collect::<Vec<_>>()
                .try_into()
                .expect("Failed to convert tags");

                let gain = gain.and_then(|g| parse_track_gain(&g));
                if gain.is_none() {
                    warn!(
                        "Failed to parse gain from {:?}",
                        meta.tags()
                            .into_iter()
                            .find(|t2| t2.std_key == Some(StandardTagKey::ReplayGainTrackGain))
                            .map(|t| t.value.to_string())
                    );
                }

                Some((
                    f,
                    m,
                    Song {
                        title,
                        artist,
                        album,
                        year,
                        track,
                        gain,
                    },
                ))
            })
            .for_each(|(f, m, s)| {
                self.insert_file(f.path(), m, s)
                    .unwrap_or_else(|e| warn!("{}", e))
            });
    }
}

fn parse_track_gain(gain: &str) -> Option<f32> {
    let gain = gain.strip_suffix(" dB")?;
    let db = gain.parse::<f32>().ok()?;
    Some(10f32.powf(db / 20.0))
}
