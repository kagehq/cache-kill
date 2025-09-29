use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::cache_entry::CacheEntry;
use crate::config::MergedConfig;

use super::common::{existing_dir, make_entry};
use super::traits::CacheManager;

pub struct NpmManager {
    pub(crate) config: MergedConfig,
}

impl NpmManager {
    pub fn new(config: MergedConfig) -> Self {
        Self { config }
    }

    fn cache_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var_os("LOCALAPPDATA").map(|local| Path::new(&local).join("npm-cache"))
        }
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|home| home.join(".npm"))
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            dirs::home_dir().map(|home| home.join(".npm"))
        }
    }
}

impl CacheManager for NpmManager {
    fn name(&self) -> &'static str {
        "npm"
    }

    fn list(&self) -> Result<Vec<CacheEntry>> {
        let mut entries = Vec::new();
        if let Some(dir) = Self::cache_dir() {
            if existing_dir(&dir) {
                entries.push(make_entry(dir, &self.config)?);
            }
        }
        Ok(entries)
    }

    fn exclude_patterns(&self) -> Vec<String> {
        let mut patterns = Vec::new();
        if let Some(dir) = Self::cache_dir() {
            patterns.push(dir.to_string_lossy().to_string());
        }
        patterns
    }
}
