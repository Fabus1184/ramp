use std::{collections::HashSet, path::PathBuf};

use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Config {
    pub search_directories: Vec<PathBuf>,
    pub extensions: HashSet<String>,
    pub cache_path: PathBuf,
    pub log_path: PathBuf,
    pub gain: OrderedFloat<f32>,
}

impl Config {
    pub fn load<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let contents = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&contents)?;

        Ok(config)
    }

    pub fn save<P>(&self, path: P) -> anyhow::Result<()>
    where
        P: AsRef<std::path::Path>,
    {
        let file = std::fs::File::create(&path)?;
        let mut ser = serde_json::Serializer::pretty(file);
        self.serialize(&mut ser)?;

        Ok(())
    }

    pub fn default_from_config_dir<P: AsRef<std::path::Path>>(config_dir: P) -> Self {
        Self {
            search_directories: vec![],
            extensions: HashSet::new(),
            cache_path: config_dir.as_ref().join("ramp.cache"),
            log_path: config_dir.as_ref().join("ramp.log"),
            gain: OrderedFloat(0.0),
        }
    }
}
