use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::cache_entry::{CacheEntry, CacheKind, PlannedAction};
use crate::config::MergedConfig;
use crate::util::{get_most_recent_mtime, get_size, is_dir, path_exists};

/// PyTorch cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorchCacheEntry {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub last_used: DateTime<Utc>,
    pub cache_type: String,
    pub version: Option<String>,
}

/// PyTorch cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorchStats {
    pub total_size_bytes: u64,
    pub total_size_human: String,
    pub entry_count: usize,
    pub cache_types: std::collections::HashMap<String, u64>,
    pub versions: std::collections::HashMap<String, u64>,
}

/// PyTorch cache manager
pub struct TorchCacheManager {
    config: MergedConfig,
}

impl TorchCacheManager {
    pub fn new(config: MergedConfig) -> Self {
        Self { config }
    }

    /// Get PyTorch cache directory
    pub fn get_torch_cache_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to get home directory")?;

        let cache_dir = home.join(".cache").join("torch");
        Ok(cache_dir)
    }

    /// Check if PyTorch cache exists
    pub fn cache_exists(&self) -> bool {
        match Self::get_torch_cache_dir() {
            Ok(dir) => path_exists(&dir) && is_dir(&dir),
            Err(_) => false,
        }
    }

    /// List PyTorch cache entries
    pub fn list_cache(&self) -> Result<Vec<TorchCacheEntry>> {
        let cache_dir = Self::get_torch_cache_dir()?;

        if !self.cache_exists() {
            return Ok(vec![]);
        }

        let mut entries = Vec::new();

        for entry in WalkDir::new(&cache_dir).max_depth(4) {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let size = get_size(path).unwrap_or(0);
                let last_used = get_most_recent_mtime(path).unwrap_or_else(|_| Utc::now());

                let (cache_type, version) = self.parse_torch_path(path);

                entries.push(TorchCacheEntry {
                    path: path.to_path_buf(),
                    size_bytes: size,
                    last_used,
                    cache_type,
                    version,
                });
            }
        }

        // Sort by size descending
        entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

        Ok(entries)
    }

    /// Get PyTorch cache statistics
    pub fn get_stats(&self) -> Result<TorchStats> {
        let entries = self.list_cache()?;

        let total_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
        let total_size_human = humansize::format_size(total_size, humansize::DECIMAL);

        // Group by cache type and version
        let mut cache_types: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        let mut versions: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

        for entry in &entries {
            *cache_types.entry(entry.cache_type.clone()).or_insert(0) += entry.size_bytes;
            if let Some(version) = &entry.version {
                *versions.entry(version.clone()).or_insert(0) += entry.size_bytes;
            }
        }

        Ok(TorchStats {
            total_size_bytes: total_size,
            total_size_human,
            entry_count: entries.len(),
            cache_types,
            versions,
        })
    }

    /// Clean PyTorch cache
    pub fn clean_cache(&self) -> Result<Vec<CacheEntry>> {
        let _cache_dir = Self::get_torch_cache_dir()?;

        if !self.cache_exists() {
            return Ok(vec![]);
        }

        let entries = self.list_cache()?;
        let mut cache_entries = Vec::new();

        for entry in entries {
            // Check if entry is stale
            let is_stale = self.is_stale(&entry.last_used);

            let planned_action = if is_stale {
                PlannedAction::Backup
            } else {
                PlannedAction::Skip
            };

            cache_entries.push(CacheEntry {
                path: entry.path,
                kind: CacheKind::MachineLearning,
                size_bytes: entry.size_bytes,
                last_used: entry.last_used,
                stale: is_stale,
                planned_action: Some(planned_action),
            });
        }

        Ok(cache_entries)
    }

    /// Parse PyTorch cache path to extract cache type and version
    fn parse_torch_path(&self, path: &Path) -> (String, Option<String>) {
        let path_str = path.to_string_lossy();
        let parts: Vec<&str> = path_str.split('/').collect();

        let mut cache_type = "unknown".to_string();
        let mut version = None;

        // Look for common PyTorch cache patterns
        for (_i, part) in parts.iter().enumerate() {
            if *part == "checkpoints" {
                cache_type = "checkpoints".to_string();
            } else if *part == "hub" {
                cache_type = "hub".to_string();
            } else if *part == "datasets" {
                cache_type = "datasets".to_string();
            } else if *part == "models" {
                cache_type = "models".to_string();
            } else if *part == "transformers" {
                cache_type = "transformers".to_string();
            } else if part.contains("torch") && part.contains("_") {
                // Try to extract version from directory names like "torch_1.12.0"
                if let Some(version_part) = part.strip_prefix("torch_") {
                    version = Some(version_part.to_string());
                }
            }
        }

        // If no specific type found, use parent directory name
        if cache_type == "unknown" {
            if let Some(parent) = path.parent() {
                if let Some(parent_name) = parent.file_name() {
                    cache_type = parent_name.to_string_lossy().to_string();
                }
            }
        }

        (cache_type, version)
    }

    /// Check if cache entry is stale
    fn is_stale(&self, last_used: &DateTime<Utc>) -> bool {
        let now = Utc::now();
        let days_since_used = (now - *last_used).num_days();
        days_since_used > self.config.stale_days as i64
    }
}

/// Handle PyTorch list command
pub fn handle_torch_list(config: &MergedConfig) -> Result<()> {
    let manager = TorchCacheManager::new(config.clone());

    if !manager.cache_exists() {
        if config.json {
            println!("{{\"error\": \"PyTorch cache not found\"}}");
        } else {
            println!("PyTorch cache not found at ~/.cache/torch");
        }
        return Ok(());
    }

    let stats = manager.get_stats()?;

    if config.json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    } else {
        println!("🔥 PyTorch Cache Statistics");
        println!("Total size: {}", stats.total_size_human);
        println!("Entries: {}", stats.entry_count);

        if !stats.cache_types.is_empty() {
            println!("\nCache Types:");
            let mut sorted_types: Vec<_> = stats.cache_types.iter().collect();
            sorted_types.sort_by(|a, b| b.1.cmp(a.1));

            for (cache_type, size) in sorted_types {
                println!(
                    "  {}: {}",
                    cache_type,
                    humansize::format_size(*size, humansize::DECIMAL)
                );
            }
        }

        if !stats.versions.is_empty() {
            println!("\nVersions:");
            let mut sorted_versions: Vec<_> = stats.versions.iter().collect();
            sorted_versions.sort_by(|a, b| b.1.cmp(a.1));

            for (version, size) in sorted_versions {
                println!(
                    "  {}: {}",
                    version,
                    humansize::format_size(*size, humansize::DECIMAL)
                );
            }
        }
    }

    Ok(())
}

/// Handle PyTorch clean command
pub fn handle_torch_clean(config: &MergedConfig) -> Result<()> {
    let manager = TorchCacheManager::new(config.clone());

    if !manager.cache_exists() {
        if config.json {
            println!("{{\"error\": \"PyTorch cache not found\"}}");
        } else {
            println!("PyTorch cache not found at ~/.cache/torch");
        }
        return Ok(());
    }

    let entries = manager.clean_cache()?;

    if entries.is_empty() {
        if config.json {
            println!("{{\"message\": \"No PyTorch cache entries to clean\"}}");
        } else {
            println!("No PyTorch cache entries to clean");
        }
        return Ok(());
    }

    if config.json {
        let result = serde_json::json!({
            "entries": entries,
            "total_size": entries.iter().map(|e| e.size_bytes).sum::<u64>(),
            "stale_count": entries.iter().filter(|e| e.stale).count()
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("🔥 PyTorch Cache Cleanup");
        println!("Found {} entries", entries.len());

        let total_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
        let stale_count = entries.iter().filter(|e| e.stale).count();

        println!(
            "Total size: {}",
            humansize::format_size(total_size, humansize::DECIMAL)
        );
        println!("Stale entries: {}", stale_count);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_torch_cache_dir() {
        let cache_dir = TorchCacheManager::get_torch_cache_dir().unwrap();
        assert!(cache_dir.to_string_lossy().contains(".cache/torch"));
    }

    #[test]
    fn test_parse_torch_path() {
        let manager = TorchCacheManager::new(MergedConfig::default());
        let path = Path::new("/home/user/.cache/torch/hub/checkpoints/model.pth");

        let (cache_type, _version) = manager.parse_torch_path(path);
        assert_eq!(cache_type, "checkpoints");
    }

    #[test]
    fn test_torch_stats_creation() {
        let stats = TorchStats {
            total_size_bytes: 2048,
            total_size_human: "2.0 KB".to_string(),
            entry_count: 3,
            cache_types: std::collections::HashMap::new(),
            versions: std::collections::HashMap::new(),
        };

        assert_eq!(stats.total_size_bytes, 2048);
        assert_eq!(stats.entry_count, 3);
    }
}
