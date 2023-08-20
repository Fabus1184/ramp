use crate::{audio, config::Config, song::Song};
use std::{
    collections::{HashMap, HashSet},
    fs::Metadata,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::SystemTime,
};

use log::warn;

use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
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
        let s = std::fs::read(&config.cache_path)
            .map_err(|e| {
                warn!("Failed to read cache file {:?}", e);
            })
            .ok()?;

        bitcode::deserialize(&s)
            .map_err(|e| {
                warn!("Failed to deserialize cache file {:?}", e);
            })
            .ok()
    }

    pub fn save(&self, config: &Config) -> Result<(), String> {
        let s =
            bitcode::serialize(self).map_err(|e| format!("Failed to serialize cache {:?}", e))?;
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

    fn merge_into(&mut self, cache: Cache) {
        match (self, cache) {
            (Cache::Directory { children: c1 }, Cache::Directory { children: c2 }) => {
                for (k, v) in c2 {
                    if let Some(c) = c1.get_mut(&k) {
                        c.merge_into(v);
                    } else {
                        c1.insert(k, v);
                    }
                }
            }
            (a, b) => panic!("Cache::merge called on {:?} and {:?}", a, b),
        }
    }

    pub fn build_from_config(config: &Config) -> Self {
        let n = config
            .search_directories
            .iter()
            .flat_map(|d| WalkDir::new(d))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count();

        let i = AtomicUsize::new(0);

        config
            .search_directories
            .iter()
            .flat_map(|d| WalkDir::new(d))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
            .collect::<Vec<_>>()
            .par_iter()
            .map(|p| Cache::build_from_path(p.path(), &config.extensions, n, &i))
            .reduce(Cache::empty, |mut a, b| {
                a.merge_into(b);
                a
            })
    }

    fn build_from_path<P>(path: P, extensions: &HashSet<String>, n: usize, i: &AtomicUsize) -> Cache
    where
        P: AsRef<std::path::Path>,
    {
        let mut cache = Cache::empty();

        WalkDir::new(path)
            .into_iter()
            .flat_map(|d| (d.map_err(|e| warn!("Failed to read file {:?}", e))))
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
                    .map(|e| extensions.contains(e))
                    .unwrap_or(false)
            })
            .inspect(|(d, _)| {
                let i = i.fetch_add(1, Ordering::SeqCst);
                println!(
                    "{}/{} ({:.3}%): {:?}",
                    i,
                    n,
                    i as f32 / n as f32 * 100.0,
                    d.path()
                );
            })
            .filter_map(|(f, m)| {
                audio::song_from_file(f.path())
                    .map(|s| (s, m, f))
                    .map_err(|e| warn!("Failed to read song from file {:?}", e))
                    .ok()
            })
            .for_each(|(s, m, f)| {
                cache
                    .insert_file(f.path(), m, s)
                    .unwrap_or_else(|e| warn!("{}", e))
            });

        cache
    }
}
