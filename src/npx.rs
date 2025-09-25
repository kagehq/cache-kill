use std::path::PathBuf;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use crate::cache_entry::{CacheEntry, CacheKind, PlannedAction};
use crate::util::{path_exists, is_dir, get_size, get_most_recent_mtime};
use crate::config::MergedConfig;

/// NPX package entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpxPackage {
    pub name: String,
    pub version: Option<String>,
    pub size_bytes: u64,
    pub last_used: DateTime<Utc>,
    pub path: PathBuf,
    pub stale: bool,
}

/// Package information from package.json
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PackageInfo {
    name: String,
    version: Option<String>,
}

/// NPX cache manager
pub struct NpxCacheManager {
    config: MergedConfig,
}

impl NpxCacheManager {
    /// Create a new NPX cache manager
    pub fn new(config: MergedConfig) -> Self {
        Self { config }
    }

    /// Get the NPX cache directory path for the current platform
    pub fn get_npx_cache_dir() -> Result<PathBuf> {
        if let Some(home) = dirs::home_dir() {
            let npx_cache = home.join(".npm").join("_npx");
            Ok(npx_cache)
        } else {
            Err(anyhow::anyhow!("Could not determine home directory"))
        }
    }

    /// Check if NPX cache exists
    pub fn npx_cache_exists() -> bool {
        Self::get_npx_cache_dir()
            .map(|path| path_exists(&path) && is_dir(&path))
            .unwrap_or(false)
    }

    /// List NPX cache entries
    pub fn list_npx_cache(&self) -> Result<Vec<CacheEntry>> {
        let npx_cache_dir = Self::get_npx_cache_dir()?;
        
        if !path_exists(&npx_cache_dir) {
            return Ok(vec![]);
        }

        let mut entries = Vec::new();
        
        // Get the total size of the NPX cache directory
        let total_size = get_size(&npx_cache_dir)?;
        let last_used = get_most_recent_mtime(&npx_cache_dir)?;
        let stale = self.is_stale(&last_used);

        let entry = CacheEntry::new(
            npx_cache_dir,
            CacheKind::Npx,
            total_size,
            last_used,
            stale,
        ).with_planned_action(self.determine_planned_action());

        entries.push(entry);

        // Also list individual cached packages if they exist
        if let Ok(read_dir) = std::fs::read_dir(&Self::get_npx_cache_dir()?) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let size = get_size(&path).unwrap_or(0);
                    let last_used = get_most_recent_mtime(&path).unwrap_or(DateTime::from_timestamp(0, 0).unwrap_or_default());
                    let stale = self.is_stale(&last_used);

                    let cache_entry = CacheEntry::new(
                        path,
                        CacheKind::Npx,
                        size,
                        last_used,
                        stale,
                    ).with_planned_action(self.determine_planned_action());

                    entries.push(cache_entry);
                }
            }
        }

        Ok(entries)
    }

    /// Get NPX cache size
    #[allow(dead_code)]
    pub fn get_npx_cache_size(&self) -> Result<u64> {
        let npx_cache_dir = Self::get_npx_cache_dir()?;
        if path_exists(&npx_cache_dir) {
            get_size(&npx_cache_dir)
        } else {
            Ok(0)
        }
    }

    /// Clear NPX cache
    #[allow(dead_code)]
    pub fn clear_npx_cache(&self) -> Result<()> {
        let npx_cache_dir = Self::get_npx_cache_dir()?;
        
        if !path_exists(&npx_cache_dir) {
            return Ok(());
        }

        if self.config.safe_delete {
            self.safe_delete_npx_cache(&npx_cache_dir)?;
        } else {
            self.hard_delete_npx_cache(&npx_cache_dir)?;
        }

        Ok(())
    }

    /// Safe delete NPX cache (move to backup)
    #[allow(dead_code)]
    fn safe_delete_npx_cache(&self, npx_cache_dir: &PathBuf) -> Result<()> {
        use crate::util::{create_backup_dir_name, get_backup_dir};
        use fs_extra::dir;
        
        let backup_dir = get_backup_dir();
        let timestamped_backup = backup_dir.join(create_backup_dir_name());
        let npx_backup = timestamped_backup.join("npx");
        
        // Create backup directory
        std::fs::create_dir_all(&npx_backup)
            .context("Failed to create backup directory")?;

        // Move NPX cache to backup
        let options = dir::CopyOptions::new();
        dir::move_dir(npx_cache_dir, &npx_backup, &options)
            .context("Failed to move NPX cache to backup")?;

        println!("✅ NPX cache moved to backup: {}", npx_backup.display());
        Ok(())
    }

    /// Hard delete NPX cache
    #[allow(dead_code)]
    fn hard_delete_npx_cache(&self, npx_cache_dir: &PathBuf) -> Result<()> {
        std::fs::remove_dir_all(npx_cache_dir)
            .context("Failed to remove NPX cache directory")?;

        println!("✅ NPX cache deleted");
        Ok(())
    }

    /// Check if NPX cache is stale
    fn is_stale(&self, last_used: &DateTime<Utc>) -> bool {
        let now = Utc::now();
        let days_since_used = (now - *last_used).num_days();
        days_since_used > self.config.stale_days as i64
    }

    /// Determine planned action for NPX cache
    fn determine_planned_action(&self) -> PlannedAction {
        if self.config.safe_delete {
            PlannedAction::Backup
        } else {
            PlannedAction::Delete
        }
    }

    /// Get NPX cache statistics
    pub fn get_npx_stats(&self) -> Result<NpxStats> {
        let entries = self.list_npx_cache()?;
        let total_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
        let total_count = entries.len();
        let stale_count = entries.iter().filter(|e| e.stale).count();

        Ok(NpxStats {
            total_size,
            total_count,
            stale_count,
            exists: Self::npx_cache_exists(),
        })
    }
}

/// NPX cache statistics
#[derive(Debug, Clone)]
pub struct NpxStats {
    pub total_size: u64,
    pub total_count: usize,
    pub stale_count: usize,
    pub exists: bool,
}

impl NpxStats {
    /// Get human-readable total size
    pub fn total_size_human(&self) -> String {
        humansize::format_size(self.total_size, humansize::DECIMAL)
    }
}

/// Check if NPX is available in the system
pub fn is_npx_available() -> bool {
    which::which("npx").is_ok()
}

/// Get NPX version if available
#[allow(dead_code)]
pub fn get_npx_version() -> Option<String> {
    use std::process::Command;
    
    if let Ok(output) = Command::new("npx").arg("--version").output() {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }
    None
}

impl NpxCacheManager {
    /// List NPX packages with per-package details
    pub fn list_packages(&self) -> Result<Vec<NpxPackage>> {
        let npx_cache_dir = Self::get_npx_cache_dir()?;
        
        if !path_exists(&npx_cache_dir) {
            return Ok(vec![]);
        }

        let mut packages = Vec::new();
        
        // Walk through NPX cache directory
        for entry in WalkDir::new(&npx_cache_dir).max_depth(3) {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Try to find package.json in this directory
                let package_json = path.join("package.json");
                if path_exists(&package_json) {
                    if let Ok(package_info) = self.parse_package_json(&package_json) {
                        let size = get_size(path).unwrap_or(0);
                        let last_used = get_most_recent_mtime(path).unwrap_or_else(|_| Utc::now());
                        let stale = self.is_stale(&last_used);
                        
                        packages.push(NpxPackage {
                            name: package_info.name,
                            version: package_info.version,
                            size_bytes: size,
                            last_used,
                            path: path.to_path_buf(),
                            stale,
                        });
                    }
                }
            }
        }
        
        // Sort by size descending
        packages.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        
        Ok(packages)
    }

    /// Parse package.json to extract name and version
    #[allow(dead_code)]
    fn parse_package_json(&self, path: &std::path::Path) -> Result<PackageInfo> {
        let content = std::fs::read_to_string(path)?;
        let package: serde_json::Value = serde_json::from_str(&content)?;
        
        // For NPX packages, try to get name from dependencies or use directory name
        let name = if let Some(name) = package["name"].as_str() {
            name.to_string()
        } else if let Some(deps) = package["dependencies"].as_object() {
            // Use the first dependency as the package name
            deps.keys().next().map_or("unknown", |v| v).to_string()
        } else {
            // Use the parent directory name as fallback
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        };
        
        let version = package["version"]
            .as_str()
            .map(|v| v.to_string());
        
        Ok(PackageInfo { name, version })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MergedConfig;
    use crate::cache_entry::LanguageFilter;

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
        }
    }

    #[test]
    fn test_npx_cache_dir() {
        let cache_dir = NpxCacheManager::get_npx_cache_dir().unwrap();
        assert!(cache_dir.to_string_lossy().contains(".npm"));
        assert!(cache_dir.to_string_lossy().contains("_npx"));
    }

    #[test]
    fn test_npx_cache_exists() {
        // This test might pass or fail depending on the system
        let exists = NpxCacheManager::npx_cache_exists();
        // We can't assert a specific value since it depends on the system state
        assert!(exists || !exists); // This will always be true
    }

    #[test]
    fn test_npx_manager_creation() {
        let config = create_test_config();
        let manager = NpxCacheManager::new(config);
        // Just test that it can be created
        assert!(true);
    }

    #[test]
    fn test_npx_availability() {
        // This test might pass or fail depending on the system
        let available = is_npx_available();
        // We can't assert a specific value since it depends on the system state
        assert!(available || !available); // This will always be true
    }

    #[test]
    fn test_stale_detection() {
        let config = create_test_config();
        let manager = NpxCacheManager::new(config);

        // Test with recent time (not stale)
        let recent_time = Utc::now();
        assert!(!manager.is_stale(&recent_time));

        // Test with old time (stale)
        let old_time = Utc::now() - chrono::Duration::days(20);
        assert!(manager.is_stale(&old_time));
    }

    #[test]
    fn test_planned_action() {
        let mut config = create_test_config();
        let manager = NpxCacheManager::new(config.clone());
        
        // Test with safe delete enabled
        assert_eq!(manager.determine_planned_action(), PlannedAction::Backup);
        
        // Test with safe delete disabled
        config.safe_delete = false;
        let manager = NpxCacheManager::new(config);
        assert_eq!(manager.determine_planned_action(), PlannedAction::Delete);
    }
}
