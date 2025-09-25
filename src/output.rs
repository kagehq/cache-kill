use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::cache_entry::{CacheEntry, CacheKind, PlannedAction};
use crate::inspect::CacheSummary;
use crate::actions::{DryRunResult, SafeDeleteResult, HardDeleteResult, RestoreResult};
use crate::npx::NpxStats;
use crate::docker::DockerStats;

/// Output formatter for human and JSON output
pub struct OutputFormatter {
    json_mode: bool,
}

impl OutputFormatter {
    /// Create a new output formatter
    pub fn new(json_mode: bool) -> Self {
        Self { json_mode }
    }

    /// Print cache entries in a table format
    pub fn print_cache_table(&self, entries: &[CacheEntry]) -> Result<(), Box<dyn std::error::Error>> {
        if self.json_mode {
            self.print_json_output(entries)?;
        } else {
            self.print_human_table(entries)?;
        }
        Ok(())
    }

    /// Print human-readable table
    fn print_human_table(&self, entries: &[CacheEntry]) -> Result<(), Box<dyn std::error::Error>> {
        if entries.is_empty() {
            println!("No cache entries found.");
            return Ok(());
        }

        // Calculate column widths
        let mut path_width = 4; // "PATH"
        let mut kind_width = 4; // "KIND"
        let mut size_width = 4; // "SIZE"
        let mut last_used_width = 9; // "LAST USED"
        let stale_width = 6; // "STALE?"

        for entry in entries {
            path_width = path_width.max(entry.path.to_string_lossy().len());
            kind_width = kind_width.max(entry.kind.to_string().len());
            size_width = size_width.max(entry.size_human().len());
            last_used_width = last_used_width.max(entry.last_used_human().len());
        }

        // Print header
        println!("{:<path_width$} | {:<kind_width$} | {:<size_width$} | {:<last_used_width$} | {:<stale_width$}", 
                 "PATH", "KIND", "SIZE", "LAST USED", "STALE?");
        println!("{:-<path_width$}-+-{:-<kind_width$}-+-{:-<size_width$}-+-{:-<last_used_width$}-+-{:-<stale_width$}", 
                 "", "", "", "", "");

        // Print entries
        for entry in entries {
            let stale_str = if entry.stale { "Yes" } else { "No" };
            println!("{:<path_width$} | {:<kind_width$} | {:<size_width$} | {:<last_used_width$} | {:<stale_width$}", 
                     entry.path.to_string_lossy(),
                     entry.kind.to_string(),
                     entry.size_human(),
                     entry.last_used_human(),
                     stale_str);
        }

        Ok(())
    }

    /// Print JSON output
    fn print_json_output(&self, entries: &[CacheEntry]) -> Result<(), Box<dyn std::error::Error>> {
        let output = JsonOutput {
            mode: "list".to_string(),
            entries: entries.to_vec(),
            totals: self.calculate_totals(entries),
        };

        println!("{}", serde_json::to_string_pretty(&output)?);
        Ok(())
    }

    /// Print summary information
    pub fn print_summary(&self, summary: &CacheSummary) -> Result<(), Box<dyn std::error::Error>> {
        if self.json_mode {
            let output = JsonSummary {
                total_size_bytes: summary.total_size,
                total_size_human: summary.total_size_human(),
                total_count: summary.total_count,
                stale_count: summary.stale_count,
                size_by_kind: summary.size_by_kind_human(),
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("\n📊 Cache Summary:");
            println!("  Total size: {}", summary.total_size_human());
            println!("  Total entries: {}", summary.total_count);
            println!("  Stale entries: {}", summary.stale_count);
            
            if !summary.size_by_kind.is_empty() {
                println!("\n  Size by kind:");
                for (kind, size) in &summary.size_by_kind_human() {
                    println!("    {}: {}", kind, size);
                }
            }
        }
        Ok(())
    }

    /// Print dry run results
    pub fn print_dry_run(&self, result: &DryRunResult) -> Result<(), Box<dyn std::error::Error>> {
        if self.json_mode {
            let output = JsonDryRun {
                mode: "dry-run".to_string(),
                to_delete: result.to_delete.clone(),
                to_backup: result.to_backup.clone(),
                to_skip: result.to_skip.clone(),
                total_size_bytes: result.total_size,
                total_size_human: result.total_size_human(),
                total_count: result.total_count,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("\n🔍 Dry Run Results:");
            println!("  Total entries: {}", result.total_count);
            println!("  Total size: {}", result.total_size_human());
            println!("  To delete: {}", result.to_delete.len());
            println!("  To backup: {}", result.to_backup.len());
            println!("  To skip: {}", result.to_skip.len());

            if !result.to_delete.is_empty() {
                println!("\n  🗑️  Will DELETE:");
                for entry in &result.to_delete {
                    println!("    {} ({})", entry.path.display(), entry.size_human());
                }
            }

            if !result.to_backup.is_empty() {
                println!("\n  📦 Will BACKUP:");
                for entry in &result.to_backup {
                    println!("    {} ({})", entry.path.display(), entry.size_human());
                }
            }

            if !result.to_skip.is_empty() {
                println!("\n  ⏭️  Will SKIP:");
                for entry in &result.to_skip {
                    println!("    {} ({})", entry.path.display(), entry.size_human());
                }
            }
        }
        Ok(())
    }

    /// Print safe delete results
    pub fn print_safe_delete_result(&self, result: &SafeDeleteResult) -> Result<(), Box<dyn std::error::Error>> {
        if self.json_mode {
            let output = JsonSafeDelete {
                mode: "safe-delete".to_string(),
                backed_up: result.backed_up.clone(),
                failed: result.failed.clone(),
                total_size_bytes: result.total_size,
                total_size_human: humansize::format_size(result.total_size, humansize::DECIMAL),
                backup_dir: result.backup_dir.to_string_lossy().to_string(),
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("\n✅ Safe Delete Results:");
            println!("  Backup directory: {}", result.backup_dir.display());
            println!("  Total size backed up: {}", humansize::format_size(result.total_size, humansize::DECIMAL));
            println!("  Successfully backed up: {}", result.backed_up.len());
            println!("  Failed: {}", result.failed.len());

            if !result.backed_up.is_empty() {
                println!("\n  📦 Backed up:");
                for entry in &result.backed_up {
                    println!("    {} -> {}", entry.original_path.display(), entry.backup_path.display());
                }
            }

            if !result.failed.is_empty() {
                println!("\n  ❌ Failed:");
                for entry in &result.failed {
                    println!("    {}: {}", entry.path.display(), entry.error);
                }
            }
        }
        Ok(())
    }

    /// Print hard delete results
    pub fn print_hard_delete_result(&self, result: &HardDeleteResult) -> Result<(), Box<dyn std::error::Error>> {
        if self.json_mode {
            let output = JsonHardDelete {
                mode: "hard-delete".to_string(),
                deleted: result.deleted.iter().map(|p| p.to_string_lossy().to_string()).collect(),
                failed: result.failed.clone(),
                total_size_bytes: result.total_size,
                total_size_human: humansize::format_size(result.total_size, humansize::DECIMAL),
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("\n🗑️  Hard Delete Results:");
            println!("  Total size freed: {}", humansize::format_size(result.total_size, humansize::DECIMAL));
            println!("  Successfully deleted: {}", result.deleted.len());
            println!("  Failed: {}", result.failed.len());

            if !result.deleted.is_empty() {
                println!("\n  ✅ Deleted:");
                for path in &result.deleted {
                    println!("    {}", path.display());
                }
            }

            if !result.failed.is_empty() {
                println!("\n  ❌ Failed:");
                for entry in &result.failed {
                    println!("    {}: {}", entry.path.display(), entry.error);
                }
            }
        }
        Ok(())
    }

    /// Print restore results
    pub fn print_restore_result(&self, result: &RestoreResult) -> Result<(), Box<dyn std::error::Error>> {
        if self.json_mode {
            let output = JsonRestore {
                mode: "restore".to_string(),
                restored: result.restored.iter().map(|p| p.to_string_lossy().to_string()).collect(),
                failed: result.failed.clone(),
                backup_dir: result.backup_dir.to_string_lossy().to_string(),
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("\n🔄 Restore Results:");
            println!("  Backup directory: {}", result.backup_dir.display());
            println!("  Successfully restored: {}", result.restored.len());
            println!("  Failed: {}", result.failed.len());

            if !result.restored.is_empty() {
                println!("\n  ✅ Restored:");
                for path in &result.restored {
                    println!("    {}", path.display());
                }
            }

            if !result.failed.is_empty() {
                println!("\n  ❌ Failed:");
                for entry in &result.failed {
                    println!("    {}: {}", entry.path.display(), entry.error);
                }
            }
        }
        Ok(())
    }

    /// Print NPX cache information
    pub fn print_npx_info(&self, stats: &NpxStats) -> Result<(), Box<dyn std::error::Error>> {
        if self.json_mode {
            let output = JsonNpxStats {
                total_size_bytes: stats.total_size,
                total_size_human: stats.total_size_human(),
                total_count: stats.total_count,
                stale_count: stats.stale_count,
                exists: stats.exists,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            if stats.exists {
                println!("\n📦 NPX Cache Information:");
                println!("  Total size: {}", stats.total_size_human());
                println!("  Total entries: {}", stats.total_count);
                println!("  Stale entries: {}", stats.stale_count);
            } else {
                println!("\n📦 NPX Cache: Not found");
            }
        }
        Ok(())
    }

    /// Print Docker information
    pub fn print_docker_info(&self, stats: &DockerStats) -> Result<(), Box<dyn std::error::Error>> {
        if self.json_mode {
            let output = JsonDockerStats {
                total_size_bytes: stats.total_size,
                total_size_human: stats.total_size_human(),
                images_size_bytes: stats.images_size,
                containers_size_bytes: stats.containers_size,
                volumes_size_bytes: stats.volumes_size,
                build_cache_size_bytes: stats.build_cache_size,
                available: stats.available,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            if stats.available {
                println!("\n🐳 Docker Information:");
                println!("  Total size: {}", stats.total_size_human());
                println!("  Images: {}", humansize::format_size(stats.images_size, humansize::DECIMAL));
                println!("  Containers: {}", humansize::format_size(stats.containers_size, humansize::DECIMAL));
                println!("  Volumes: {}", humansize::format_size(stats.volumes_size, humansize::DECIMAL));
                println!("  Build cache: {}", humansize::format_size(stats.build_cache_size, humansize::DECIMAL));
            } else {
                println!("\n🐳 Docker: Not available");
            }
        }
        Ok(())
    }

    /// Calculate totals for JSON output
    fn calculate_totals(&self, entries: &[CacheEntry]) -> JsonTotals {
        let total_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
        let count = entries.len();
        let freed_bytes: u64 = entries.iter()
            .filter(|e| matches!(e.planned_action, Some(PlannedAction::Delete) | Some(PlannedAction::Backup)))
            .map(|e| e.size_bytes)
            .sum();

        JsonTotals {
            size_bytes: total_size,
            count,
            freed_bytes,
        }
    }
}

/// JSON output structure
#[derive(Serialize, Deserialize)]
struct JsonOutput {
    mode: String,
    entries: Vec<CacheEntry>,
    totals: JsonTotals,
}

/// JSON summary structure
#[derive(Serialize, Deserialize)]
struct JsonSummary {
    total_size_bytes: u64,
    total_size_human: String,
    total_count: usize,
    stale_count: usize,
    size_by_kind: HashMap<CacheKind, String>,
}

/// JSON dry run structure
#[derive(Serialize, Deserialize)]
struct JsonDryRun {
    mode: String,
    to_delete: Vec<CacheEntry>,
    to_backup: Vec<CacheEntry>,
    to_skip: Vec<CacheEntry>,
    total_size_bytes: u64,
    total_size_human: String,
    total_count: usize,
}

/// JSON safe delete structure
#[derive(Serialize, Deserialize)]
struct JsonSafeDelete {
    mode: String,
    backed_up: Vec<crate::actions::BackupEntry>,
    failed: Vec<crate::actions::FailedEntry>,
    total_size_bytes: u64,
    total_size_human: String,
    backup_dir: String,
}

/// JSON hard delete structure
#[derive(Serialize, Deserialize)]
struct JsonHardDelete {
    mode: String,
    deleted: Vec<String>,
    failed: Vec<crate::actions::FailedEntry>,
    total_size_bytes: u64,
    total_size_human: String,
}

/// JSON restore structure
#[derive(Serialize, Deserialize)]
struct JsonRestore {
    mode: String,
    restored: Vec<String>,
    failed: Vec<crate::actions::FailedEntry>,
    backup_dir: String,
}

/// JSON NPX stats structure
#[derive(Serialize, Deserialize)]
struct JsonNpxStats {
    total_size_bytes: u64,
    total_size_human: String,
    total_count: usize,
    stale_count: usize,
    exists: bool,
}

/// JSON Docker stats structure
#[derive(Serialize, Deserialize)]
struct JsonDockerStats {
    total_size_bytes: u64,
    total_size_human: String,
    images_size_bytes: u64,
    containers_size_bytes: u64,
    volumes_size_bytes: u64,
    build_cache_size_bytes: u64,
    available: bool,
}

/// JSON totals structure
#[derive(Serialize, Deserialize)]
struct JsonTotals {
    size_bytes: u64,
    count: usize,
    freed_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache_entry::{CacheEntry, CacheKind, PlannedAction};
    use chrono::Utc;

    #[test]
    fn test_output_formatter_creation() {
        let formatter = OutputFormatter::new(false);
        assert!(!formatter.json_mode);
        
        let json_formatter = OutputFormatter::new(true);
        assert!(json_formatter.json_mode);
    }

    #[test]
    fn test_calculate_totals() {
        let formatter = OutputFormatter::new(false);
        let entries = vec![
            CacheEntry::new(
                std::path::PathBuf::from("test1"),
                CacheKind::JavaScript,
                1000,
                Utc::now(),
                false,
            ).with_planned_action(PlannedAction::Delete),
            CacheEntry::new(
                std::path::PathBuf::from("test2"),
                CacheKind::Python,
                2000,
                Utc::now(),
                false,
            ).with_planned_action(PlannedAction::Backup),
        ];

        let totals = formatter.calculate_totals(&entries);
        assert_eq!(totals.size_bytes, 3000);
        assert_eq!(totals.count, 2);
        assert_eq!(totals.freed_bytes, 3000);
    }

    #[test]
    fn test_json_output_serialization() {
        let _formatter = OutputFormatter::new(true);
        let entries = vec![
            CacheEntry::new(
                std::path::PathBuf::from("test1"),
                CacheKind::JavaScript,
                1000,
                Utc::now(),
                false,
            ),
        ];

        let output = JsonOutput {
            mode: "list".to_string(),
            entries: entries.clone(),
            totals: JsonTotals {
                size_bytes: 1000,
                count: 1,
                freed_bytes: 0,
            },
        };

        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("test1"));
        assert!(json.contains("test1"));
    }
}
