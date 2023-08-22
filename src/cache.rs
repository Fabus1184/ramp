use crate::{audio, config::Config, song::Song};
use std::{
    collections::HashMap,
    fs::Metadata,
    path::{Path, PathBuf},
};

use log::warn;

use walkdir::WalkDir;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum Cache {
    File { song: Song },
    Directory { children: HashMap<String, Cache> },
}

impl Clone for Cache {
    fn clone(&self) -> Cache {
        panic!("Cache::clone called")
    }
}

impl Cache {
    pub fn songs(&self) -> Box<dyn Iterator<Item = &Song> + '_> {
        match self {
            Cache::File { song, .. } => Box::new(std::iter::once(song)),
            Cache::Directory { children, .. } => {
                Box::new(children.values().flat_map(|c| c.songs()))
            }
        }
    }

    fn insert_file(&mut self, path: &Path, meta: Metadata, song: Song) -> Result<(), String> {
        match self {
            Cache::File { .. } => panic!("Cache::insert_file called on Cache::File"),
            Cache::Directory { children, .. } => {
                if path.components().count() == 1 {
                    let name = path
                        .file_name()
                        .ok_or_else(|| format!("Failed to get file name from path {:?}", path))?
                        .to_str()
                        .ok_or_else(|| format!("Failed to convert file name to string {:?}", path))?
                        .to_string();

                    children.insert(name, Cache::File { song });

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

    pub fn load(config: &Config) -> anyhow::Result<(Self, Config)> {
        let s = std::fs::read(&config.cache_path)?;
        let config = bitcode::deserialize(&s)?;
        Ok(config)
    }

    pub fn save(&self, config: &Config) -> anyhow::Result<()> {
        let s = bitcode::serialize(&(self, config))?;
        std::fs::write(&config.cache_path, s)?;

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
                Cache::Directory { children, .. } => children.get(next)?.get(&rest),
            }
        }
    }

    pub fn build_from_config(config: &Config) -> Self {
        let mut cache = Cache::Directory {
            children: HashMap::new(),
        };
        config
            .search_directories
            .iter()
            .flat_map(|d| WalkDir::new(d))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|e| config.extensions.contains(e.to_str().unwrap_or("")))
                    .unwrap_or(false)
            })
            .filter_map(|e| {
                e.metadata()
                    .map(|m| (e, m))
                    .map_err(|e| warn!("Failed to read metadata {:?}", e))
                    .ok()
            })
            .filter_map(|(e, m)| {
                audio::song_from_file(e.path())
                    .map(|s| (e.path().to_path_buf(), m, s))
                    .map_err(|e| {
                        warn!("Failed to read song from {:?}: {}", e, e);
                    })
                    .ok()
            })
            .for_each(|(p, m, s)| {
                cache
                    .insert_file(&p, m, s)
                    .unwrap_or_else(|e| warn!("Failed to insert file {:?}: {}", p, e));
            });

        cache
    }
}
