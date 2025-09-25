use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process;
use chrono::{DateTime, Utc};

use crate::config::MergedConfig;
use crate::actions::ActionExecutor;
use crate::discover::DiscoveryResult;
use crate::inspect::CacheInspector;

/// CI mode for non-interactive cache management
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CiMode {
    #[serde(rename = "prebuild")]
    Prebuild,
    #[serde(rename = "postbuild")]
    Postbuild,
}

/// CI execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiResult {
    pub mode: CiMode,
    pub entries_processed: usize,
    pub freed_bytes: u64,
    pub backup_dir: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub status: String,
    pub summary: HashMap<String, String>,
}

/// CI cache manager
pub struct CiCacheManager {
    config: MergedConfig,
    mode: CiMode,
}

impl CiCacheManager {
    pub fn new(config: MergedConfig, mode: CiMode) -> Self {
        Self { config, mode }
    }

    /// Execute CI cache operations
    pub fn execute(&self) -> Result<CiResult> {
        let start_time = Utc::now();
        
        // Discover caches
        let discovery = DiscoveryResult::discover(&self.config)
            .context("Failed to discover caches")?;
        
        if discovery.cache_entries.is_empty() {
            return Ok(CiResult {
                mode: self.mode,
                entries_processed: 0,
                freed_bytes: 0,
                backup_dir: None,
                timestamp: start_time,
                status: "nothing_to_do".to_string(),
                summary: self.create_summary(0, 0, None),
            });
        }

        // Inspect caches
        let inspector = CacheInspector::new(self.config.clone());
        let entries = inspector.inspect_caches(&discovery.cache_entries)
            .context("Failed to inspect caches")?;

        if entries.is_empty() {
            return Ok(CiResult {
                mode: self.mode,
                entries_processed: 0,
                freed_bytes: 0,
                backup_dir: None,
                timestamp: start_time,
                status: "nothing_to_do".to_string(),
                summary: self.create_summary(0, 0, None),
            });
        }

        // Execute actions based on mode
        let (freed_bytes, backup_dir) = match self.mode {
            CiMode::Prebuild => {
                // In prebuild mode, we typically just analyze
                let _total_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
                (0, None)
            }
            CiMode::Postbuild => {
                // In postbuild mode, we clean up
                let executor = ActionExecutor::new(self.config.clone());
                
                if self.config.dry_run {
                    // Dry run - just calculate what would be freed
                    let total_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
                    (total_size, None)
                } else {
                    // Actually clean up
                    let result = executor.safe_delete(&entries)
                        .context("Failed to execute safe delete")?;
                    
                    (result.total_size, Some(result.backup_dir.to_string_lossy().to_string()))
                }
            }
        };

        let status = if freed_bytes > 0 { "success" } else { "no_action" };
        
        Ok(CiResult {
            mode: self.mode,
            entries_processed: entries.len(),
            freed_bytes,
            backup_dir: backup_dir.clone(),
            timestamp: start_time,
            status: status.to_string(),
            summary: self.create_summary(entries.len(), freed_bytes, backup_dir.as_deref()),
        })
    }

    /// Create summary for CI output
    fn create_summary(&self, entries: usize, freed_bytes: u64, backup_dir: Option<&str>) -> HashMap<String, String> {
        let mut summary = HashMap::new();
        
        summary.insert("mode".to_string(), format!("{:?}", self.mode).to_lowercase());
        summary.insert("entries_processed".to_string(), entries.to_string());
        summary.insert("freed_bytes".to_string(), freed_bytes.to_string());
        summary.insert("freed_human".to_string(), humansize::format_size(freed_bytes, humansize::DECIMAL));
        
        if let Some(backup) = backup_dir {
            summary.insert("backup_dir".to_string(), backup.to_string());
        }
        
        summary.insert("timestamp".to_string(), Utc::now().to_rfc3339());
        
        summary
    }

    /// Print CI summary in machine-readable format
    pub fn print_summary(&self, result: &CiResult, json_mode: bool) -> Result<()> {
        if json_mode {
            println!("{}", serde_json::to_string_pretty(result)?);
        } else {
            // Print one-line summary for CI logs
            println!("CACHEKILL_CI: mode={:?} entries={} freed={} status={}", 
                result.mode, result.entries_processed, result.freed_bytes, result.status);
            
            if let Some(backup) = &result.backup_dir {
                println!("CACHEKILL_BACKUP: {}", backup);
            }
        }
        
        Ok(())
    }
}

/// Exit codes for CI operations
pub mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const PARTIAL_SUCCESS: i32 = 2;
    pub const NOTHING_TO_DO: i32 = 3;
    #[allow(dead_code)]
    pub const CONFIG_ERROR: i32 = 4;
    pub const FATAL_ERROR: i32 = 5;
}

/// Handle CI mode execution with proper exit codes
pub fn handle_ci_mode(config: &MergedConfig, mode: CiMode) -> Result<()> {
    let manager = CiCacheManager::new(config.clone(), mode);
    let result = manager.execute()?;
    
    // Print summary
    manager.print_summary(&result, config.json)?;
    
    // Set appropriate exit code
    let exit_code = match result.status.as_str() {
        "success" => exit_codes::SUCCESS,
        "nothing_to_do" => exit_codes::NOTHING_TO_DO,
        "partial" => exit_codes::PARTIAL_SUCCESS,
        _ => exit_codes::FATAL_ERROR,
    };
    
    if exit_code != exit_codes::SUCCESS {
        process::exit(exit_code);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MergedConfig;

    #[test]
    fn test_ci_result_creation() {
        let mut config = MergedConfig::default();
        config.dry_run = true;
        
        let result = CiResult {
            mode: CiMode::Prebuild,
            entries_processed: 5,
            freed_bytes: 1024,
            backup_dir: Some("/tmp/backup".to_string()),
            timestamp: Utc::now(),
            status: "success".to_string(),
            summary: HashMap::new(),
        };
        
        assert_eq!(result.entries_processed, 5);
        assert_eq!(result.freed_bytes, 1024);
    }

    #[test]
    fn test_ci_mode_serialization() {
        let prebuild = CiMode::Prebuild;
        let postbuild = CiMode::Postbuild;
        
        assert_eq!(format!("{:?}", prebuild), "Prebuild");
        assert_eq!(format!("{:?}", postbuild), "Postbuild");
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(exit_codes::SUCCESS, 0);
        assert_eq!(exit_codes::PARTIAL_SUCCESS, 2);
        assert_eq!(exit_codes::NOTHING_TO_DO, 3);
        assert_eq!(exit_codes::CONFIG_ERROR, 4);
        assert_eq!(exit_codes::FATAL_ERROR, 5);
    }
}
