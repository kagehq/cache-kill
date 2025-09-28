use std::path::Path;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rayon::prelude::*;
use crate::cache_entry::{CacheEntry, CacheKind, PlannedAction};
use crate::util::{get_size, get_most_recent_mtime, path_exists, is_dir};
use crate::config::MergedConfig;

/// Inspect cache entries and calculate metadata
pub struct CacheInspector {
    config: MergedConfig,
}

impl CacheInspector {
    /// Create a new cache inspector
    pub fn new(config: MergedConfig) -> Self {
        Self { config }
    }

    /// Inspect a list of cache paths and return cache entries
    pub fn inspect_caches(&self, cache_paths: &[std::path::PathBuf]) -> Result<Vec<CacheEntry>> {
        let entries: Result<Vec<_>> = cache_paths
            .par_iter()
            .map(|path| self.inspect_single_cache(path))
            .collect();
        
        entries
    }

    /// Inspect a single cache path
    fn inspect_single_cache(&self, path: &Path) -> Result<CacheEntry> {
        if !path_exists(path) {
            return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
        }

        let kind = self.determine_cache_kind(path);
        let size_bytes = self.calculate_size(path)?;
        let last_used = self.get_last_used_time(path)?;
        let stale = self.is_stale(&last_used);

        let mut entry = CacheEntry::new(
            path.to_path_buf(),
            kind,
            size_bytes,
            last_used,
            stale,
        );

        // Determine planned action
        entry.planned_action = Some(self.determine_planned_action(&entry));

        Ok(entry)
    }

    /// Determine the cache kind for a path
    fn determine_cache_kind(&self, path: &Path) -> CacheKind {
        let path_str = path.to_string_lossy().to_lowercase();
        
        // JavaScript/TypeScript caches
        if path_str.contains("node_modules") || 
           path_str.contains(".next") || 
           path_str.contains(".nuxt") ||
           path_str.contains(".vite") ||
           path_str.contains(".turbo") ||
           path_str.contains(".parcel-cache") {
            return CacheKind::JavaScript;
        }

        // Python caches
        if path_str.contains("__pycache__") ||
           path_str.contains(".pytest_cache") ||
           path_str.contains(".venv") ||
           path_str.contains("venv") ||
           path_str.contains(".tox") ||
           path_str.contains(".mypy_cache") ||
           path_str.contains(".ruff_cache") ||
           path_str.contains(".pip-cache") {
            return CacheKind::Python;
        }

        // Rust caches
        if path_str.contains("target") || path_str.contains("cargo") {
            return CacheKind::Rust;
        }

        // Java caches
        if path_str.contains(".gradle") ||
           path_str.contains("build") && path_str.contains("gradle") ||
           path_str.contains(".m2") {
            return CacheKind::Java;
        }

        // ML/AI caches
        if path_str.contains("huggingface") ||
           path_str.contains("torch") ||
           path_str.contains("transformers") ||
           path_str.contains(".dvc") ||
           path_str.contains("wandb") {
            return CacheKind::MachineLearning;
        }

        // NPX caches
        if path_str.contains("_npx") {
            return CacheKind::Npx;
        }

        // Docker caches
        if path_str.contains("docker") {
            return CacheKind::Docker;
        }

        CacheKind::Generic
    }

    /// Calculate the size of a cache path
    fn calculate_size(&self, path: &Path) -> Result<u64> {
        if !path_exists(path) {
            return Ok(0);
        }

        if is_dir(path) {
            self.calculate_directory_size(path)
        } else {
            get_size(path).context("Failed to get file size")
        }
    }

    /// Calculate directory size with progress indication
    fn calculate_directory_size(&self, path: &Path) -> Result<u64> {
        let mut total_size = 0u64;
        
        for entry in walkdir::WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            }
        }
        
        Ok(total_size)
    }

    /// Get the last used time for a cache path
    fn get_last_used_time(&self, path: &Path) -> Result<DateTime<Utc>> {
        if !path_exists(path) {
            return Ok(DateTime::from_timestamp(0, 0).unwrap_or_default());
        }

        if is_dir(path) {
            get_most_recent_mtime(path)
                .context("Failed to get most recent modification time")
        } else {
            use crate::util::get_mtime;
            get_mtime(path)
                .context("Failed to get file modification time")
        }
    }

    /// Check if a cache entry is stale
    fn is_stale(&self, last_used: &DateTime<Utc>) -> bool {
        let now = Utc::now();
        let days_since_used = (now - *last_used).num_days();
        days_since_used > self.config.stale_days as i64
    }

    /// Determine the planned action for a cache entry
    fn determine_planned_action(&self, entry: &CacheEntry) -> PlannedAction {
        // Skip if path should be excluded
        if self.config.should_exclude_path(&entry.path) {
            return PlannedAction::Skip;
        }

        // Skip if not included in paths
        if !self.config.paths.is_empty() && !self.config.should_include_path(&entry.path) {
            return PlannedAction::Skip;
        }

        // Use safe delete if enabled
        if self.config.safe_delete {
            PlannedAction::Backup
        } else {
            PlannedAction::Delete
        }
    }

    /// Get summary statistics for cache entries
    pub fn get_summary(&self, entries: &[CacheEntry]) -> CacheSummary {
        let total_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
        let total_count = entries.len();
        let stale_count = entries.iter().filter(|e| e.stale).count();
        let to_delete_count = entries.iter()
            .filter(|e| matches!(e.planned_action, Some(PlannedAction::Delete) | Some(PlannedAction::Backup)))
            .count();
        let to_skip_count = entries.iter()
            .filter(|e| matches!(e.planned_action, Some(PlannedAction::Skip)))
            .count();

        let size_by_kind = entries.iter()
            .fold(std::collections::HashMap::new(), |mut acc, entry| {
                *acc.entry(entry.kind).or_insert(0) += entry.size_bytes;
                acc
            });

        CacheSummary {
            total_size,
            total_count,
            stale_count,
            to_delete_count,
            to_skip_count,
            size_by_kind,
        }
    }

    /// Get the top N largest cache entries
    pub fn get_largest_entries<'a>(&self, entries: &'a [CacheEntry], n: usize) -> Vec<&'a CacheEntry> {
        let mut sorted_entries: Vec<_> = entries.iter().collect();
        sorted_entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        sorted_entries.into_iter().take(n).collect()
    }
}

/// Summary statistics for cache entries
#[derive(Debug, Clone)]
pub struct CacheSummary {
    pub total_size: u64,
    pub total_count: usize,
    pub stale_count: usize,
    #[allow(dead_code)]
    pub to_delete_count: usize,
    #[allow(dead_code)]
    pub to_skip_count: usize,
    pub size_by_kind: std::collections::HashMap<CacheKind, u64>,
}

impl CacheSummary {
    /// Get human-readable total size
    pub fn total_size_human(&self) -> String {
        humansize::format_size(self.total_size, humansize::DECIMAL)
    }

    /// Get human-readable size by kind
    pub fn size_by_kind_human(&self) -> std::collections::HashMap<CacheKind, String> {
        self.size_by_kind
            .iter()
            .map(|(kind, size)| (*kind, humansize::format_size(*size, humansize::DECIMAL)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use crate::config::MergedConfig;
    use crate::cache_entry::LanguageFilter;
    use chrono::Utc;

    fn create_test_config() -> MergedConfig {
        MergedConfig {
            list: false,
            dry_run: false,
            force: false,
            json: false,
            lang: LanguageFilter::Auto,
            paths: vec![],
            exclude: vec![],
            stale_days: 14,
            safe_delete: true,
            backup_dir: ".cachekill-backup".to_string(),
            docker: false,
            npx: false,
            restore_last: false,
            all: false,
            js_pm: false,
        }
    }

    #[test]
    fn test_cache_kind_detection() {
        let config = create_test_config();
        let inspector = CacheInspector::new(config);

        assert_eq!(
            inspector.determine_cache_kind(Path::new("node_modules")),
            CacheKind::JavaScript
        );
        assert_eq!(
            inspector.determine_cache_kind(Path::new("__pycache__")),
            CacheKind::Python
        );
        assert_eq!(
            inspector.determine_cache_kind(Path::new("target")),
            CacheKind::Rust
        );
    }

    #[test]
    fn test_size_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let config = create_test_config();
        let inspector = CacheInspector::new(config);
        
        let size = inspector.calculate_size(&test_file).unwrap();
        assert_eq!(size, 12); // "test content".len()
    }

    #[test]
    fn test_stale_detection() {
        let config = create_test_config();
        let inspector = CacheInspector::new(config);

        // Test with recent time (not stale)
        let recent_time = Utc::now();
        assert!(!inspector.is_stale(&recent_time));

        // Test with old time (stale)
        let old_time = Utc::now() - chrono::Duration::days(20);
        assert!(inspector.is_stale(&old_time));
    }

    #[test]
    fn test_cache_summary() {
        let config = create_test_config();
        let inspector = CacheInspector::new(config);

        let entries = vec![
            CacheEntry::new(
                std::path::PathBuf::from("test1"),
                CacheKind::JavaScript,
                1000,
                Utc::now(),
                false,
            ),
            CacheEntry::new(
                std::path::PathBuf::from("test2"),
                CacheKind::Python,
                2000,
                Utc::now(),
                true,
            ),
        ];

        let summary = inspector.get_summary(&entries);
        assert_eq!(summary.total_size, 3000);
        assert_eq!(summary.total_count, 2);
        assert_eq!(summary.stale_count, 1);
    }

    #[test]
    fn test_largest_entries() {
        let config = create_test_config();
        let inspector = CacheInspector::new(config);

        let entries = vec![
            CacheEntry::new(
                std::path::PathBuf::from("small"),
                CacheKind::JavaScript,
                100,
                Utc::now(),
                false,
            ),
            CacheEntry::new(
                std::path::PathBuf::from("large"),
                CacheKind::Python,
                1000,
                Utc::now(),
                false,
            ),
            CacheEntry::new(
                std::path::PathBuf::from("medium"),
                CacheKind::Rust,
                500,
                Utc::now(),
                false,
            ),
        ];

        let largest = inspector.get_largest_entries(&entries, 2);
        assert_eq!(largest.len(), 2);
        assert_eq!(largest[0].path.to_string_lossy(), "large");
        assert_eq!(largest[1].path.to_string_lossy(), "medium");
    }
}
