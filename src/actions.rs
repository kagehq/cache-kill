use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use fs_extra::dir;
use crate::cache_entry::{CacheEntry, PlannedAction};
use crate::config::MergedConfig;
use crate::util::{create_backup_dir_name, get_backup_dir, path_exists, is_dir};

/// Action executor for cache operations
pub struct ActionExecutor {
    #[allow(dead_code)]
    config: MergedConfig,
}

impl ActionExecutor {
    /// Create a new action executor
    pub fn new(config: MergedConfig) -> Self {
        Self { config }
    }

    /// Execute dry run - show what would be done
    pub fn dry_run(&self, entries: &[CacheEntry]) -> Result<DryRunResult> {
        let mut result = DryRunResult {
            to_delete: Vec::new(),
            to_backup: Vec::new(),
            to_skip: Vec::new(),
            total_size: 0,
            total_count: 0,
        };

        for entry in entries {
            match entry.planned_action {
                Some(PlannedAction::Delete) => {
                    result.to_delete.push(entry.clone());
                    result.total_size += entry.size_bytes;
                }
                Some(PlannedAction::Backup) => {
                    result.to_backup.push(entry.clone());
                    result.total_size += entry.size_bytes;
                }
                Some(PlannedAction::Skip) => {
                    result.to_skip.push(entry.clone());
                }
                None => {
                    result.to_skip.push(entry.clone());
                }
            }
            result.total_count += 1;
        }

        Ok(result)
    }

    /// Execute safe delete - move to backup
    pub fn safe_delete(&self, entries: &[CacheEntry]) -> Result<SafeDeleteResult> {
        let backup_dir = get_backup_dir();
        let timestamped_backup = backup_dir.join(create_backup_dir_name());
        
        // Create backup directory
        std::fs::create_dir_all(&timestamped_backup)
            .context("Failed to create backup directory")?;

        let mut result = SafeDeleteResult {
            backed_up: Vec::new(),
            failed: Vec::new(),
            total_size: 0,
            backup_dir: timestamped_backup.clone(),
        };

        for entry in entries {
            if let Some(PlannedAction::Backup) = entry.planned_action {
                match self.move_to_backup(&entry.path, &timestamped_backup) {
                    Ok(backup_path) => {
                        result.backed_up.push(BackupEntry {
                            original_path: entry.path.clone(),
                            backup_path,
                            size: entry.size_bytes,
                        });
                        result.total_size += entry.size_bytes;
                    }
                    Err(e) => {
                        result.failed.push(FailedEntry {
                            path: entry.path.clone(),
                            error: e.to_string(),
                        });
                    }
                }
            }
        }

        Ok(result)
    }

    /// Move a path to backup directory
    fn move_to_backup(&self, source: &Path, backup_dir: &Path) -> Result<PathBuf> {
        if !path_exists(source) {
            return Err(anyhow::anyhow!("Source path does not exist: {}", source.display()));
        }

        let backup_path = backup_dir.join(source.file_name().unwrap_or_else(|| source.as_os_str()));
        
        if is_dir(source) {
            // Move directory
            let options = dir::CopyOptions::new();
            dir::move_dir(source, &backup_path, &options)
                .context("Failed to move directory to backup")?;
        } else {
            // Move file
            std::fs::rename(source, &backup_path)
                .context("Failed to move file to backup")?;
        }

        Ok(backup_path)
    }

    /// Execute hard delete - permanently remove
    pub fn hard_delete(&self, entries: &[CacheEntry]) -> Result<HardDeleteResult> {
        let mut result = HardDeleteResult {
            deleted: Vec::new(),
            failed: Vec::new(),
            total_size: 0,
        };

        for entry in entries {
            if let Some(PlannedAction::Delete) = entry.planned_action {
                match self.delete_path(&entry.path) {
                    Ok(()) => {
                        result.deleted.push(entry.path.clone());
                        result.total_size += entry.size_bytes;
                    }
                    Err(e) => {
                        result.failed.push(FailedEntry {
                            path: entry.path.clone(),
                            error: e.to_string(),
                        });
                    }
                }
            }
        }

        Ok(result)
    }

    /// Delete a path (file or directory)
    fn delete_path(&self, path: &Path) -> Result<()> {
        if !path_exists(path) {
            return Ok(()); // Already deleted
        }

        if is_dir(path) {
            std::fs::remove_dir_all(path)
                .context("Failed to remove directory")?;
        } else {
            std::fs::remove_file(path)
                .context("Failed to remove file")?;
        }

        Ok(())
    }

    /// Restore from last backup
    pub fn restore_last_backup(&self) -> Result<RestoreResult> {
        let backup_dir = get_backup_dir();
        
        if !path_exists(&backup_dir) {
            return Err(anyhow::anyhow!("No backup directory found"));
        }

        // Find the most recent backup
        let mut backup_dirs: Vec<_> = std::fs::read_dir(&backup_dir)
            .context("Failed to read backup directory")?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .collect();

        backup_dirs.sort_by(|a, b| {
            b.metadata().unwrap().modified().unwrap()
                .cmp(&a.metadata().unwrap().modified().unwrap())
        });

        if backup_dirs.is_empty() {
            return Err(anyhow::anyhow!("No backup directories found"));
        }

        let latest_backup = backup_dirs[0].path();
        self.restore_from_backup(&latest_backup)
    }

    /// Restore from a specific backup directory
    fn restore_from_backup(&self, backup_dir: &Path) -> Result<RestoreResult> {
        let mut result = RestoreResult {
            restored: Vec::new(),
            failed: Vec::new(),
            backup_dir: backup_dir.to_path_buf(),
        };

        // Read backup directory contents
        for entry in std::fs::read_dir(backup_dir)
            .context("Failed to read backup directory")?
        {
            let entry = entry?;
            let backup_path = entry.path();
            let original_path = self.get_original_path_from_backup(&backup_path)?;

            match self.restore_path(&backup_path, &original_path) {
                Ok(()) => {
                    result.restored.push(original_path);
                }
                Err(e) => {
                    result.failed.push(FailedEntry {
                        path: original_path,
                        error: e.to_string(),
                    });
                }
            }
        }

        Ok(result)
    }

    /// Get original path from backup path
    fn get_original_path_from_backup(&self, backup_path: &Path) -> Result<PathBuf> {
        // This is a simplified implementation
        // In practice, you might want to store metadata about original paths
        let file_name = backup_path.file_name()
            .context("Failed to get file name from backup path")?;
        Ok(PathBuf::from(file_name))
    }

    /// Restore a single path from backup
    fn restore_path(&self, backup_path: &Path, original_path: &Path) -> Result<()> {
        if !path_exists(backup_path) {
            return Err(anyhow::anyhow!("Backup path does not exist: {}", backup_path.display()));
        }

        if is_dir(backup_path) {
            // Restore directory
            let options = dir::CopyOptions::new();
            dir::move_dir(backup_path, original_path, &options)
                .context("Failed to restore directory from backup")?;
        } else {
            // Restore file
            std::fs::rename(backup_path, original_path)
                .context("Failed to restore file from backup")?;
        }

        Ok(())
    }

    /// Clean old backups (older than specified days)
    #[allow(dead_code)]
    pub fn clean_old_backups(&self, days: u32) -> Result<CleanupResult> {
        let backup_dir = get_backup_dir();
        
        if !path_exists(&backup_dir) {
            return Ok(CleanupResult {
                removed: Vec::new(),
                total_freed: 0,
            });
        }

        let cutoff_time = Utc::now() - chrono::Duration::days(days as i64);
        let mut result = CleanupResult {
            removed: Vec::new(),
            total_freed: 0,
        };

        for entry in std::fs::read_dir(&backup_dir)
            .context("Failed to read backup directory")?
        {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                let metadata = entry.metadata()?;
                let modified_time: DateTime<Utc> = DateTime::from(metadata.modified()?);
                
                if modified_time < cutoff_time {
                    let size = crate::util::get_size(&path).unwrap_or(0);
                    std::fs::remove_dir_all(&path)
                        .context("Failed to remove old backup")?;
                    
                    result.removed.push(path);
                    result.total_freed += size;
                }
            }
        }

        Ok(result)
    }
}

/// Dry run result
#[derive(Debug, Clone)]
pub struct DryRunResult {
    pub to_delete: Vec<CacheEntry>,
    pub to_backup: Vec<CacheEntry>,
    pub to_skip: Vec<CacheEntry>,
    pub total_size: u64,
    pub total_count: usize,
}

impl DryRunResult {
    /// Get human-readable total size
    pub fn total_size_human(&self) -> String {
        humansize::format_size(self.total_size, humansize::DECIMAL)
    }
}

/// Safe delete result
#[derive(Debug, Clone)]
pub struct SafeDeleteResult {
    pub backed_up: Vec<BackupEntry>,
    pub failed: Vec<FailedEntry>,
    pub total_size: u64,
    pub backup_dir: PathBuf,
}

/// Backup entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    pub original_path: PathBuf,
    pub backup_path: PathBuf,
    pub size: u64,
}

/// Failed entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedEntry {
    pub path: PathBuf,
    pub error: String,
}

/// Hard delete result
#[derive(Debug, Clone)]
pub struct HardDeleteResult {
    pub deleted: Vec<PathBuf>,
    pub failed: Vec<FailedEntry>,
    pub total_size: u64,
}

/// Restore result
#[derive(Debug, Clone)]
pub struct RestoreResult {
    pub restored: Vec<PathBuf>,
    pub failed: Vec<FailedEntry>,
    pub backup_dir: PathBuf,
}

/// Cleanup result
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CleanupResult {
    pub removed: Vec<PathBuf>,
    pub total_freed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use crate::config::MergedConfig;
    use crate::cache_entry::{LanguageFilter, CacheKind, PlannedAction};

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
    fn test_action_executor_creation() {
        let config = create_test_config();
        let executor = ActionExecutor::new(config);
        assert!(true); // Just test that it can be created
    }

    #[test]
    fn test_dry_run() {
        let config = create_test_config();
        let executor = ActionExecutor::new(config);

        let entries = vec![
            CacheEntry::new(
                PathBuf::from("test1"),
                CacheKind::JavaScript,
                1000,
                Utc::now(),
                false,
            ).with_planned_action(PlannedAction::Delete),
            CacheEntry::new(
                PathBuf::from("test2"),
                CacheKind::Python,
                2000,
                Utc::now(),
                false,
            ).with_planned_action(PlannedAction::Backup),
        ];

        let result = executor.dry_run(&entries).unwrap();
        assert_eq!(result.to_delete.len(), 1);
        assert_eq!(result.to_backup.len(), 1);
        assert_eq!(result.total_size, 3000);
    }

    #[test]
    fn test_safe_delete() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let config = create_test_config();
        let executor = ActionExecutor::new(config);

        // Create test files
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let entries = vec![
            CacheEntry::new(
                test_file.clone(),
                CacheKind::JavaScript,
                12,
                Utc::now(),
                false,
            ).with_planned_action(PlannedAction::Backup),
        ];

        let result = executor.safe_delete(&entries).unwrap();
        assert_eq!(result.backed_up.len(), 1);
        assert_eq!(result.total_size, 12);
    }

    #[test]
    fn test_hard_delete() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let config = create_test_config();
        let executor = ActionExecutor::new(config);

        // Create test files
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let entries = vec![
            CacheEntry::new(
                test_file.clone(),
                CacheKind::JavaScript,
                12,
                Utc::now(),
                false,
            ).with_planned_action(PlannedAction::Delete),
        ];

        let result = executor.hard_delete(&entries).unwrap();
        assert_eq!(result.deleted.len(), 1);
        assert_eq!(result.total_size, 12);
    }
}
