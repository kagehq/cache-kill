use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::cache_entry::CacheEntry;
use crate::config::MergedConfig;

use super::common::{existing_dir, make_entry};
use super::traits::CacheManager;

pub struct PnpmManager {
    pub(crate) config: MergedConfig,
}

impl PnpmManager {
    pub fn new(config: MergedConfig) -> Self {
        Self { config }
    }
}

fn store_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(|local| Path::new(&local).join("pnpm").join("store").join("v3"))
    }
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|home| home.join("Library").join("pnpm").join("store").join("v3"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        dirs::home_dir().map(|home| {
            home.join(".local")
                .join("share")
                .join("pnpm")
                .join("store")
                .join("v3")
        })
    }
}

fn meta_cache_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA").map(|local| Path::new(&local).join("pnpm-cache"))
    }
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|home| home.join("Library").join("Caches").join("pnpm"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        dirs::home_dir().map(|home| home.join(".cache").join("pnpm"))
    }
}

impl CacheManager for PnpmManager {
    fn name(&self) -> &'static str {
        "pnpm"
    }

    fn list(&self) -> Result<Vec<CacheEntry>> {
        let mut entries = Vec::new();
        if let Some(dir) = store_dir() {
            if existing_dir(&dir) {
                entries.push(make_entry(dir, &self.config)?);
            }
        }
        if let Some(dir) = meta_cache_dir() {
            if existing_dir(&dir) {
                entries.push(make_entry(dir, &self.config)?);
            }
        }
        Ok(entries)
    }

    fn exclude_patterns(&self) -> Vec<String> {
        let mut patterns = Vec::new();
        if let Some(d) = store_dir() {
            patterns.push(d.to_string_lossy().to_string());
        }
        if let Some(d) = meta_cache_dir() {
            patterns.push(d.to_string_lossy().to_string());
        }
        patterns
    }
}
