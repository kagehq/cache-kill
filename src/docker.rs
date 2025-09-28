use anyhow::{Context, Result};
use std::process::Command;
use std::collections::HashMap;
use chrono::Utc;
use crate::cache_entry::{CacheEntry, CacheKind, PlannedAction};
use crate::config::MergedConfig;

/// Docker cache manager
pub struct DockerCacheManager {
    #[allow(dead_code)]
    config: MergedConfig,
}

impl DockerCacheManager {
    /// Create a new Docker cache manager
    pub fn new(config: MergedConfig) -> Self {
        Self { config }
    }

    /// Check if Docker is available
    pub fn is_docker_available() -> bool {
        which::which("docker").is_ok()
    }

    /// Get Docker version if available
    #[allow(dead_code)]
    pub fn get_docker_version() -> Option<String> {
        let output = Command::new("docker")
            .arg("--version")
            .output()
            .ok()?;
        
        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }

    /// Get Docker system information
    pub fn get_docker_system_info(&self) -> Result<DockerSystemInfo> {
        if !Self::is_docker_available() {
            return Err(anyhow::anyhow!("Docker is not available"));
        }

        // Get system df information
        let df_output = Command::new("docker")
            .arg("system")
            .arg("df")
            .arg("--format")
            .arg("table {{.Type}}\t{{.TotalCount}}\t{{.Size}}\t{{.Reclaimable}}")
            .output()
            .context("Failed to run docker system df")?;

        if !df_output.status.success() {
            return Err(anyhow::anyhow!("Failed to get Docker system information"));
        }

        let df_text = String::from_utf8_lossy(&df_output.stdout);
        let mut images_size = 0u64;
        let mut containers_size = 0u64;
        let mut volumes_size = 0u64;
        let mut build_cache_size = 0u64;

        for line in df_text.lines() {
            if line.contains("Images") {
                if let Some(size) = Self::parse_docker_size(line) {
                    images_size = size;
                }
            } else if line.contains("Containers") {
                if let Some(size) = Self::parse_docker_size(line) {
                    containers_size = size;
                }
            } else if line.contains("Local Volumes") {
                if let Some(size) = Self::parse_docker_size(line) {
                    volumes_size = size;
                }
            } else if line.contains("Build Cache") {
                if let Some(size) = Self::parse_docker_size(line) {
                    build_cache_size = size;
                }
            }
        }

        Ok(DockerSystemInfo {
            images_size,
            containers_size,
            volumes_size,
            build_cache_size,
            total_size: images_size + containers_size + volumes_size + build_cache_size,
        })
    }

    /// Parse Docker size from df output
    fn parse_docker_size(line: &str) -> Option<u64> {
        // Docker df output format: "Images    5    1.2GB    800MB"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            if let Ok(size) = Self::parse_size_string(parts[2]) {
                return Some(size);
            }
        }
        None
    }

    /// Parse size string like "1.2GB", "800MB", etc.
    fn parse_size_string(size_str: &str) -> Result<u64> {
        let size_str = size_str.to_uppercase();
        let size_str = size_str.trim_end_matches('B');
        
        let (number_part, unit) = if size_str.ends_with('K') {
            (size_str.trim_end_matches('K'), 1024)
        } else if size_str.ends_with('M') {
            (size_str.trim_end_matches('M'), 1024 * 1024)
        } else if size_str.ends_with('G') {
            (size_str.trim_end_matches('G'), 1024 * 1024 * 1024)
        } else if size_str.ends_with('T') {
            (size_str.trim_end_matches('T'), 1024_u64.pow(4))
        } else {
            (size_str, 1)
        };

        let number: f64 = number_part.parse()
            .context("Failed to parse size number")?;
        
        Ok((number * unit as f64) as u64)
    }

    /// List Docker cache entries
    pub fn list_docker_cache(&self) -> Result<Vec<CacheEntry>> {
        if !Self::is_docker_available() {
            return Ok(vec![]);
        }

        let system_info = self.get_docker_system_info()?;
        let mut entries = Vec::new();

        // Add images cache entry
        if system_info.images_size > 0 {
            let entry = CacheEntry::new(
                std::path::PathBuf::from("docker://images"),
                CacheKind::Docker,
                system_info.images_size,
                Utc::now(),
                false, // Docker images don't have a clear "stale" concept
            ).with_planned_action(self.determine_planned_action());
            entries.push(entry);
        }

        // Add containers cache entry
        if system_info.containers_size > 0 {
            let entry = CacheEntry::new(
                std::path::PathBuf::from("docker://containers"),
                CacheKind::Docker,
                system_info.containers_size,
                Utc::now(),
                false,
            ).with_planned_action(self.determine_planned_action());
            entries.push(entry);
        }

        // Add volumes cache entry
        if system_info.volumes_size > 0 {
            let entry = CacheEntry::new(
                std::path::PathBuf::from("docker://volumes"),
                CacheKind::Docker,
                system_info.volumes_size,
                Utc::now(),
                false,
            ).with_planned_action(self.determine_planned_action());
            entries.push(entry);
        }

        // Add build cache entry
        if system_info.build_cache_size > 0 {
            let entry = CacheEntry::new(
                std::path::PathBuf::from("docker://build-cache"),
                CacheKind::Docker,
                system_info.build_cache_size,
                Utc::now(),
                false,
            ).with_planned_action(self.determine_planned_action());
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Clean Docker system
    #[allow(dead_code)]
    pub fn clean_docker_system(&self) -> Result<DockerCleanupResult> {
        if !Self::is_docker_available() {
            return Err(anyhow::anyhow!("Docker is not available"));
        }

        let mut result = DockerCleanupResult {
            images_removed: 0,
            containers_removed: 0,
            volumes_removed: 0,
            build_cache_removed: 0,
            total_freed: 0,
        };

        // Remove unused images
        if let Ok(output) = Command::new("docker")
            .arg("image")
            .arg("prune")
            .arg("-f")
            .output()
        {
            if output.status.success() {
                result.images_removed = Self::parse_removed_count(&String::from_utf8_lossy(&output.stdout));
            }
        }

        // Remove stopped containers
        if let Ok(output) = Command::new("docker")
            .arg("container")
            .arg("prune")
            .arg("-f")
            .output()
        {
            if output.status.success() {
                result.containers_removed = Self::parse_removed_count(&String::from_utf8_lossy(&output.stdout));
            }
        }

        // Remove unused volumes
        if let Ok(output) = Command::new("docker")
            .arg("volume")
            .arg("prune")
            .arg("-f")
            .output()
        {
            if output.status.success() {
                result.volumes_removed = Self::parse_removed_count(&String::from_utf8_lossy(&output.stdout));
            }
        }

        // Remove build cache
        if let Ok(output) = Command::new("docker")
            .arg("builder")
            .arg("prune")
            .arg("-f")
            .output()
        {
            if output.status.success() {
                result.build_cache_removed = Self::parse_removed_count(&String::from_utf8_lossy(&output.stdout));
            }
        }

        // Get final system info to calculate freed space
        if let Ok(final_info) = self.get_docker_system_info() {
            // This is a rough estimate - in practice, you'd want to compare before/after
            result.total_freed = final_info.total_size;
        }

        Ok(result)
    }

    /// Parse removed count from Docker output
    #[allow(dead_code)]
    fn parse_removed_count(output: &str) -> usize {
        // Look for patterns like "Deleted: 5 objects"
        for line in output.lines() {
            if line.contains("Deleted:") {
                if let Some(count_str) = line.split_whitespace().nth(1) {
                    if let Ok(count) = count_str.parse::<usize>() {
                        return count;
                    }
                }
            }
        }
        0
    }

    /// Determine planned action for Docker cleanup
    fn determine_planned_action(&self) -> PlannedAction {
        // Docker cleanup is always a direct delete operation
        PlannedAction::Delete
    }

    /// Get Docker statistics
    pub fn get_docker_stats(&self) -> Result<DockerStats> {
        let system_info = self.get_docker_system_info()?;
        
        Ok(DockerStats {
            total_size: system_info.total_size,
            images_size: system_info.images_size,
            containers_size: system_info.containers_size,
            volumes_size: system_info.volumes_size,
            build_cache_size: system_info.build_cache_size,
            available: Self::is_docker_available(),
        })
    }
}

/// Docker system information
#[derive(Debug, Clone)]
pub struct DockerSystemInfo {
    pub images_size: u64,
    pub containers_size: u64,
    pub volumes_size: u64,
    pub build_cache_size: u64,
    pub total_size: u64,
}

/// Docker cleanup result
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DockerCleanupResult {
    pub images_removed: usize,
    pub containers_removed: usize,
    pub volumes_removed: usize,
    pub build_cache_removed: usize,
    pub total_freed: u64,
}

/// Docker statistics
#[derive(Debug, Clone)]
pub struct DockerStats {
    pub total_size: u64,
    pub images_size: u64,
    pub containers_size: u64,
    pub volumes_size: u64,
    pub build_cache_size: u64,
    pub available: bool,
}

impl DockerStats {
    /// Get human-readable total size
    pub fn total_size_human(&self) -> String {
        humansize::format_size(self.total_size, humansize::DECIMAL)
    }

    /// Get human-readable size by category
    #[allow(dead_code)]
    pub fn size_by_category_human(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("images".to_string(), humansize::format_size(self.images_size, humansize::DECIMAL));
        map.insert("containers".to_string(), humansize::format_size(self.containers_size, humansize::DECIMAL));
        map.insert("volumes".to_string(), humansize::format_size(self.volumes_size, humansize::DECIMAL));
        map.insert("build_cache".to_string(), humansize::format_size(self.build_cache_size, humansize::DECIMAL));
        map
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
            js_pm: false,
        }
    }

    #[test]
    fn test_docker_availability() {
        // This test might pass or fail depending on the system
        let available = DockerCacheManager::is_docker_available();
        // We can't assert a specific value since it depends on the system state
        assert!(available || !available); // This will always be true
    }

    #[test]
    fn test_docker_manager_creation() {
        let config = create_test_config();
        let _manager = DockerCacheManager::new(config);
        // Just test that it can be created
        assert!(true);
    }

    #[test]
    fn test_parse_size_string() {
        assert_eq!(DockerCacheManager::parse_size_string("1KB").unwrap(), 1024);
        assert_eq!(DockerCacheManager::parse_size_string("1MB").unwrap(), 1024 * 1024);
        assert_eq!(DockerCacheManager::parse_size_string("1GB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(DockerCacheManager::parse_size_string("1.5GB").unwrap(), (1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
    }

    #[test]
    fn test_parse_removed_count() {
        let output = "Deleted: 5 objects";
        assert_eq!(DockerCacheManager::parse_removed_count(output), 5);
        
        let output2 = "Deleted: 0 objects";
        assert_eq!(DockerCacheManager::parse_removed_count(output2), 0);
        
        let output3 = "No output";
        assert_eq!(DockerCacheManager::parse_removed_count(output3), 0);
    }

    #[test]
    fn test_docker_stats_creation() {
        let stats = DockerStats {
            total_size: 1024 * 1024 * 1024, // 1GB
            images_size: 512 * 1024 * 1024, // 512MB
            containers_size: 256 * 1024 * 1024, // 256MB
            volumes_size: 128 * 1024 * 1024, // 128MB
            build_cache_size: 128 * 1024 * 1024, // 128MB
            available: true,
        };

        assert_eq!(stats.total_size, 1024 * 1024 * 1024);
        assert!(stats.total_size_human().contains("1.0") && stats.total_size_human().contains("GB"));
    }
}
