use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub search_directories: Vec<String>,
    pub extensions: HashSet<String>,
    pub cache_path: String,
    pub log_path: String,
    pub gain: f32,
}

impl Config {
    pub fn load<P>(path: P) -> Result<Self, String>
    where
        P: AsRef<std::path::Path>,
    {
        let contents = std::fs::read_to_string(path).map_err(|s| s.to_string())?;
        serde_json::from_str(&contents).map_err(|s| s.to_string())
    }
}
