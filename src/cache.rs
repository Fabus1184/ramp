use crate::{config::Config, decoder, song::Song};
use std::{
    collections::HashMap,
    fs::Metadata,
    path::{Path, PathBuf},
};

use log::{trace, warn};

use walkdir::WalkDir;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Cache {
    root: HashMap<String, CacheEntry>,
}

impl Cache {
    pub fn songs(&self) -> impl Iterator<Item = (&Song, PathBuf)> {
        self.root.iter().flat_map(|(k, v)| {
            v.songs().map(|(s, p)| {
                let mut path = PathBuf::new().join(k.clone());
                path.extend(p);
                (s, path)
            })
        })
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

    pub fn build_from_config(config: &Config) -> Self {
        let mut cache = Cache {
            root: HashMap::new(),
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
            .inspect(|(e, _)| {
                trace!("Found file {}", e.path().display());
            })
            .filter_map(|(e, m)| {
                decoder::song_from_file(e.path())
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

    fn insert_file<P>(&mut self, path: P, meta: Metadata, song: Song) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let mut cs = path
            .as_ref()
            .components()
            .map(|c| {
                c.as_os_str().to_str().ok_or(anyhow::anyhow!(
                    "Failed to convert OsString to str: {}",
                    path.as_ref().display()
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let first = cs.drain(..1).next().ok_or(anyhow::anyhow!(
            "Failed to get first component from Path {}",
            path.as_ref().display()
        ))?;

        self.root
            .entry(first.to_string())
            .or_insert(CacheEntry::Directory {
                children: HashMap::new(),
            })
            .insert_file(cs, meta, song)?;

        Ok(())
    }

    pub fn get<P>(&self, path: P) -> anyhow::Result<Option<&CacheEntry>>
    where
        P: AsRef<Path>,
    {
        let mut cs = path
            .as_ref()
            .components()
            .map(|c| {
                c.as_os_str().to_str().ok_or(anyhow::anyhow!(
                    "Failed to convert OsString to str: {}",
                    path.as_ref().display()
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        if cs.is_empty() {
            anyhow::bail!(
                "Cache::get called with empty path: {:?}",
                path.as_ref().display()
            );
        } else {
            let dir = cs.drain(..1).next();
            let dir = dir.ok_or(anyhow::anyhow!(
                "Failed to drain 1 element from Vec with size >1: {:?}",
                cs
            ))?;

            if let Some(d) = self.root.get(dir).map(|d| d.get(cs)) {
                let d = d?;
                Ok(d)
            } else {
                Ok(None)
            }
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum CacheEntry {
    File {
        song: Song,
    },
    Directory {
        children: HashMap<String, CacheEntry>,
    },
}

impl CacheEntry {
    pub fn as_file(&self) -> anyhow::Result<&Song> {
        match self {
            CacheEntry::File { song } => Ok(song),
            CacheEntry::Directory { .. } => {
                anyhow::bail!("CacheEntry::into_song called on {:?}", self)
            }
        }
    }

    pub fn as_directory(&self) -> anyhow::Result<&HashMap<String, CacheEntry>> {
        match self {
            CacheEntry::File { .. } => anyhow::bail!("CacheEntry::into_song called on {:?}", self),
            CacheEntry::Directory { children } => Ok(children),
        }
    }

    fn songs(&self) -> Box<dyn Iterator<Item = (&Song, Vec<String>)> + '_> {
        match self {
            CacheEntry::File { .. } => panic!("CacheEntry::songs called on File"),
            CacheEntry::Directory { children, .. } => {
                Box::new(children.iter().flat_map(|(k, v)| {
                    if v.is_file() {
                        Box::new(std::iter::once((
                            match v {
                                CacheEntry::File { song } => song,
                                CacheEntry::Directory { .. } => unreachable!(),
                            },
                            vec![k.clone()],
                        )))
                    } else {
                        Box::new(v.songs().map(|(s, p)| {
                            let mut path = vec![k.clone()];
                            path.extend(p);
                            (s, path)
                        }))
                            as Box<dyn Iterator<Item = (&Song, Vec<String>)>>
                    }
                }))
            }
        }
    }

    fn is_file(&self) -> bool {
        match self {
            CacheEntry::File { .. } => true,
            CacheEntry::Directory { .. } => false,
        }
    }

    fn insert_file(
        &mut self,
        mut path: Vec<&str>,
        meta: Metadata,
        song: Song,
    ) -> anyhow::Result<()> {
        match self {
            CacheEntry::File { .. } => {
                anyhow::bail!("CacheEntry::insert_file called on {:?}", self)
            }
            CacheEntry::Directory { children, .. } => {
                if path.len() == 1 {
                    let filename = path.first().ok_or(anyhow::anyhow!(
                        "Failed to get first element from Vec with len 1: {:?}",
                        path,
                    ))?;
                    children.insert(filename.to_string(), CacheEntry::File { song });

                    Ok(())
                } else {
                    let dir = path.drain(..1).next();
                    let dir = dir.ok_or(anyhow::anyhow!(
                        "Failed to drain 1 element from Vec with size >1: {:?}",
                        path
                    ))?;

                    children
                        .entry(dir.to_string())
                        .or_insert_with(|| CacheEntry::Directory {
                            children: HashMap::new(),
                        })
                        .insert_file(path, meta, song)
                }
            }
        }
    }

    fn get(&self, mut path: Vec<&str>) -> anyhow::Result<Option<&CacheEntry>> {
        if path.is_empty() {
            Ok(Some(self))
        } else {
            let dir = path.drain(..1).next();
            let dir = dir.ok_or(anyhow::anyhow!(
                "Failed to drain 1 element from Vec with size >1: {:?}",
                path
            ))?;

            match self {
                CacheEntry::File { .. } => anyhow::bail!("CacheEntry::get called on {:?}", self),
                CacheEntry::Directory { children, .. } => {
                    if let Some(d) = children.get(dir).map(|d| d.get(path)) {
                        let d = d?;
                        Ok(d)
                    } else {
                        Ok(None)
                    }
                }
            }
        }
    }
}
