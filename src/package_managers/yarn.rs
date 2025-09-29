use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::cache_entry::CacheEntry;
use crate::config::MergedConfig;

use super::common::{existing_dir, make_entry};
use super::traits::CacheManager;

pub struct YarnManager {
    pub(crate) config: MergedConfig,
}

impl YarnManager {
    pub fn new(config: MergedConfig) -> Self {
        Self { config }
    }
}

fn global_cache_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA").map(|local| Path::new(&local).join("Yarn").join("Cache"))
    }
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|home| home.join("Library").join("Caches").join("Yarn"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        dirs::home_dir().map(|home| home.join(".cache").join("yarn"))
    }
}

fn project_cache_dir() -> Option<PathBuf> {
    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join(".yarn").join("cache");
        if p.exists() && p.is_dir() {
            return Some(p);
        }
    }
    None
}

impl CacheManager for YarnManager {
    fn name(&self) -> &'static str {
        "yarn"
    }

    fn list(&self) -> Result<Vec<CacheEntry>> {
        let mut entries = Vec::new();
        if let Some(dir) = global_cache_dir() {
            if existing_dir(&dir) {
                entries.push(make_entry(dir, &self.config)?);
            }
        }
        if let Some(dir) = project_cache_dir() {
            if existing_dir(&dir) {
                entries.push(make_entry(dir, &self.config)?);
            }
        }
        Ok(entries)
    }

    fn exclude_patterns(&self) -> Vec<String> {
        let mut patterns = Vec::new();
        if let Some(d) = global_cache_dir() {
            patterns.push(d.to_string_lossy().to_string());
        }
        if let Some(d) = project_cache_dir() {
            patterns.push(d.to_string_lossy().to_string());
        }
        patterns
    }
}
