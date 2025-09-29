use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::cache_entry::{CacheEntry, CacheKind, PlannedAction};
use crate::config::MergedConfig;
use crate::util::{get_most_recent_mtime, get_size, is_dir, path_exists};

/// HuggingFace cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfCacheEntry {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub last_used: DateTime<Utc>,
    pub repo_name: Option<String>,
    pub model_id: Option<String>,
    pub file_type: String,
}

/// HuggingFace cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfStats {
    pub total_size_bytes: u64,
    pub total_size_human: String,
    pub entry_count: usize,
    pub repo_count: usize,
    pub model_count: usize,
    pub top_repos: Vec<(String, u64)>,
    pub top_models: Vec<(String, u64)>,
}

/// HuggingFace cache manager
pub struct HfCacheManager {
    config: MergedConfig,
}

impl HfCacheManager {
    pub fn new(config: MergedConfig) -> Self {
        Self { config }
    }

    /// Get HuggingFace cache directory
    pub fn get_hf_cache_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to get home directory")?;

        let cache_dir = home.join(".cache").join("huggingface");
        Ok(cache_dir)
    }

    /// Check if HuggingFace cache exists
    pub fn cache_exists(&self) -> bool {
        match Self::get_hf_cache_dir() {
            Ok(dir) => path_exists(&dir) && is_dir(&dir),
            Err(_) => false,
        }
    }

    /// List HuggingFace cache entries
    pub fn list_cache(&self) -> Result<Vec<HfCacheEntry>> {
        let cache_dir = Self::get_hf_cache_dir()?;

        if !self.cache_exists() {
            return Ok(vec![]);
        }

        let mut entries = Vec::new();

        for entry in WalkDir::new(&cache_dir).max_depth(3) {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let size = get_size(path).unwrap_or(0);
                let last_used = get_most_recent_mtime(path).unwrap_or_else(|_| Utc::now());

                let (repo_name, model_id, file_type) = self.parse_hf_path(path);

                entries.push(HfCacheEntry {
                    path: path.to_path_buf(),
                    size_bytes: size,
                    last_used,
                    repo_name,
                    model_id,
                    file_type,
                });
            }
        }

        // Sort by size descending
        entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

        Ok(entries)
    }

    /// Get HuggingFace cache statistics
    pub fn get_stats(&self) -> Result<HfStats> {
        let entries = self.list_cache()?;

        let total_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
        let total_size_human = humansize::format_size(total_size, humansize::DECIMAL);

        // Group by repo and model
        let mut repo_sizes: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        let mut model_sizes: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();

        for entry in &entries {
            if let Some(repo) = &entry.repo_name {
                *repo_sizes.entry(repo.clone()).or_insert(0) += entry.size_bytes;
            }
            if let Some(model) = &entry.model_id {
                *model_sizes.entry(model.clone()).or_insert(0) += entry.size_bytes;
            }
        }

        // Get top repos and models
        let mut top_repos: Vec<(String, u64)> = repo_sizes.clone().into_iter().collect();
        top_repos.sort_by(|a, b| b.1.cmp(&a.1));
        top_repos.truncate(10);

        let mut top_models: Vec<(String, u64)> = model_sizes.clone().into_iter().collect();
        top_models.sort_by(|a, b| b.1.cmp(&a.1));
        top_models.truncate(10);

        Ok(HfStats {
            total_size_bytes: total_size,
            total_size_human,
            entry_count: entries.len(),
            repo_count: repo_sizes.len(),
            model_count: model_sizes.len(),
            top_repos,
            top_models,
        })
    }

    /// Clean HuggingFace cache
    pub fn clean_cache(&self, model_id: Option<&str>) -> Result<Vec<CacheEntry>> {
        let _cache_dir = Self::get_hf_cache_dir()?;

        if !self.cache_exists() {
            return Ok(vec![]);
        }

        let entries = self.list_cache()?;
        let mut cache_entries = Vec::new();

        for entry in entries {
            // Filter by model if specified
            if let Some(target_model) = model_id {
                if let Some(entry_model) = &entry.model_id {
                    if entry_model != target_model {
                        continue;
                    }
                } else {
                    continue;
                }
            }

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

    /// Parse HuggingFace cache path to extract repo and model info
    fn parse_hf_path(&self, path: &Path) -> (Option<String>, Option<String>, String) {
        let path_str = path.to_string_lossy();
        let parts: Vec<&str> = path_str.split('/').collect();

        // Look for patterns like hub/models--repo-name or datasets/repo-name
        let mut repo_name = None;
        let mut model_id = None;
        let file_type = path
            .extension()
            .map(|ext| ext.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        for (i, part) in parts.iter().enumerate() {
            if *part == "hub" && i + 1 < parts.len() {
                let hub_part = parts[i + 1];
                if hub_part.starts_with("models--") {
                    repo_name = Some(hub_part.replace("models--", "").replace("--", "/"));
                    model_id = Some(hub_part.replace("models--", "").replace("--", "/"));
                }
            } else if *part == "datasets" && i + 1 < parts.len() {
                repo_name = Some(parts[i + 1].to_string());
            }
        }

        (repo_name, model_id, file_type)
    }

    /// Check if cache entry is stale
    fn is_stale(&self, last_used: &DateTime<Utc>) -> bool {
        let now = Utc::now();
        let days_since_used = (now - *last_used).num_days();
        days_since_used > self.config.stale_days as i64
    }
}

/// Handle HuggingFace list command
pub fn handle_hf_list(config: &MergedConfig) -> Result<()> {
    let manager = HfCacheManager::new(config.clone());

    if !manager.cache_exists() {
        if config.json {
            println!("{{\"error\": \"HuggingFace cache not found\"}}");
        } else {
            println!("HuggingFace cache not found at ~/.cache/huggingface");
        }
        return Ok(());
    }

    let stats = manager.get_stats()?;

    if config.json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    } else {
        println!("🤗 HuggingFace Cache Statistics");
        println!("Total size: {}", stats.total_size_human);
        println!("Entries: {}", stats.entry_count);
        println!("Repositories: {}", stats.repo_count);
        println!("Models: {}", stats.model_count);

        if !stats.top_repos.is_empty() {
            println!("\nTop Repositories:");
            for (repo, size) in &stats.top_repos[..5] {
                println!(
                    "  {}: {}",
                    repo,
                    humansize::format_size(*size, humansize::DECIMAL)
                );
            }
        }

        if !stats.top_models.is_empty() {
            println!("\nTop Models:");
            for (model, size) in &stats.top_models[..5] {
                println!(
                    "  {}: {}",
                    model,
                    humansize::format_size(*size, humansize::DECIMAL)
                );
            }
        }
    }

    Ok(())
}

/// Handle HuggingFace clean command
pub fn handle_hf_clean(config: &MergedConfig, model_id: Option<&str>) -> Result<()> {
    let manager = HfCacheManager::new(config.clone());

    if !manager.cache_exists() {
        if config.json {
            println!("{{\"error\": \"HuggingFace cache not found\"}}");
        } else {
            println!("HuggingFace cache not found at ~/.cache/huggingface");
        }
        return Ok(());
    }

    let entries = manager.clean_cache(model_id)?;

    if entries.is_empty() {
        if config.json {
            println!("{{\"message\": \"No HuggingFace cache entries to clean\"}}");
        } else {
            println!("No HuggingFace cache entries to clean");
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
        println!("🤗 HuggingFace Cache Cleanup");
        println!("Found {} entries", entries.len());

        let total_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
        let stale_count = entries.iter().filter(|e| e.stale).count();

        println!(
            "Total size: {}",
            humansize::format_size(total_size, humansize::DECIMAL)
        );
        println!("Stale entries: {}", stale_count);

        if let Some(model) = model_id {
            println!("Targeting model: {}", model);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hf_cache_dir() {
        let cache_dir = HfCacheManager::get_hf_cache_dir().unwrap();
        assert!(cache_dir.to_string_lossy().contains(".cache/huggingface"));
    }

    #[test]
    fn test_parse_hf_path() {
        let manager = HfCacheManager::new(MergedConfig::default());
        let path =
            Path::new("/home/user/.cache/huggingface/hub/models--microsoft--DialoGPT-medium");

        let (repo, model, _file_type) = manager.parse_hf_path(path);
        assert_eq!(repo, Some("microsoft/DialoGPT-medium".to_string()));
        assert_eq!(model, Some("microsoft/DialoGPT-medium".to_string()));
    }

    #[test]
    fn test_hf_stats_creation() {
        let stats = HfStats {
            total_size_bytes: 1024,
            total_size_human: "1.0 KB".to_string(),
            entry_count: 5,
            repo_count: 2,
            model_count: 3,
            top_repos: vec![("repo1".to_string(), 512)],
            top_models: vec![("model1".to_string(), 256)],
        };

        assert_eq!(stats.total_size_bytes, 1024);
        assert_eq!(stats.entry_count, 5);
    }
}
