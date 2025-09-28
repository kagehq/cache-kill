use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use anyhow::{Context, Result};
use crate::cache_entry::LanguageFilter;
use crate::util::{get_current_dir, expand_home};

/// Configuration loaded from .cachekillrc file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default language filter
    pub default_lang: Option<LanguageFilter>,
    /// Default stale days threshold
    pub stale_days: Option<u32>,
    /// Whether safe delete is enabled by default
    pub safe_delete: Option<bool>,
    /// Default backup directory
    pub backup_dir: Option<String>,
    /// Include patterns for additional paths
    pub include_paths: Option<Vec<String>>,
    /// Exclude patterns for paths to skip
    pub exclude_paths: Option<Vec<String>>,
    /// Whether to include Docker cleanup by default
    pub include_docker: Option<bool>,
    /// Whether to include NPX cache by default
    pub include_npx: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_lang: Some(LanguageFilter::Auto),
            stale_days: Some(14),
            safe_delete: Some(true),
            backup_dir: Some(".cachekill-backup".to_string()),
            include_paths: None,
            exclude_paths: Some(vec![
                ".git".to_string(),
                ".cachekill-backup".to_string(),
                "node_modules/.cache".to_string(),
            ]),
            include_docker: Some(false),
            include_npx: Some(false),
        }
    }
}

/// CLI arguments that override config
#[derive(Debug, Clone)]
pub struct CliArgs {
    pub list: bool,
    pub dry_run: bool,
    pub force: bool,
    pub json: bool,
    pub lang: Option<LanguageFilter>,
    pub paths: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub stale_days: Option<u32>,
    pub safe_delete: Option<bool>,
    pub backup_dir: Option<String>,
    pub docker: bool,
    pub npx: bool,
    pub restore_last: bool,
    pub all: bool,
    pub js_pm: bool,
}

/// Merged configuration combining config file and CLI args
#[derive(Debug, Clone)]
pub struct MergedConfig {
    pub list: bool,
    pub dry_run: bool,
    pub force: bool,
    pub json: bool,
    pub lang: LanguageFilter,
    pub paths: Vec<String>,
    pub exclude: Vec<String>,
    pub stale_days: u32,
    pub safe_delete: bool,
    #[allow(dead_code)]
    pub backup_dir: String,
    pub docker: bool,
    pub npx: bool,
    pub restore_last: bool,
    pub all: bool,
    pub js_pm: bool,
}

impl Config {
    /// Load configuration from .cachekillrc file
    pub fn load() -> Result<Self> {
        let config_path = Self::find_config_file()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .context("Failed to read .cachekillrc file")?;
            
            let config: Config = toml::from_str(&content)
                .context("Failed to parse .cachekillrc file")?;
            
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Find the .cachekillrc file in the current directory or parent directories
    fn find_config_file() -> Result<PathBuf> {
        let mut current_dir = get_current_dir()?;
        
        loop {
            let config_path = current_dir.join(".cachekillrc");
            if config_path.exists() {
                return Ok(config_path);
            }
            
            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                break;
            }
        }
        
        // Return a non-existent path if no config file found
        Ok(PathBuf::from(".cachekillrc"))
    }

    /// Merge config with CLI arguments
    pub fn merge_with_cli(&self, cli_args: &CliArgs) -> MergedConfig {
        MergedConfig {
            list: cli_args.list,
            dry_run: cli_args.dry_run,
            force: cli_args.force,
            json: cli_args.json,
            lang: cli_args.lang.unwrap_or(
                self.default_lang.unwrap_or(LanguageFilter::Auto)
            ),
            paths: cli_args.paths.clone().unwrap_or_else(|| {
                self.include_paths.clone().unwrap_or_default()
            }),
            exclude: cli_args.exclude.clone().unwrap_or_else(|| {
                self.exclude_paths.clone().unwrap_or_default()
            }),
            stale_days: cli_args.stale_days.unwrap_or(
                self.stale_days.unwrap_or(14)
            ),
            safe_delete: cli_args.safe_delete.unwrap_or(
                self.safe_delete.unwrap_or(true)
            ),
            backup_dir: cli_args.backup_dir.clone().unwrap_or_else(|| {
                self.backup_dir.clone().unwrap_or_else(|| ".cachekill-backup".to_string())
            }),
            docker: cli_args.docker || self.include_docker.unwrap_or(false),
            npx: cli_args.npx || self.include_npx.unwrap_or(false),
            restore_last: cli_args.restore_last,
            all: cli_args.all,
            js_pm: cli_args.js_pm,
        }
    }
}

impl Default for MergedConfig {
    fn default() -> Self {
        Self {
            list: false,
            dry_run: false,
            force: false,
            json: false,
            lang: LanguageFilter::Auto,
            paths: Vec::new(),
            exclude: Vec::new(),
            stale_days: 14,
            safe_delete: true,
            backup_dir: "~/.cachekill-backup".to_string(),
            docker: false,
            npx: false,
            restore_last: false,
            all: false,
            js_pm: false,
        }
    }
}

impl MergedConfig {
    /// Get the backup directory path
    #[allow(dead_code)]
    pub fn get_backup_dir(&self) -> PathBuf {
        expand_home(&self.backup_dir)
    }

    /// Check if a path should be included based on include patterns
    pub fn should_include_path(&self, path: &std::path::Path) -> bool {
        use crate::util::should_include_path;
        should_include_path(path, &self.paths)
    }

    /// Check if a path should be excluded based on exclude patterns
    pub fn should_exclude_path(&self, path: &std::path::Path) -> bool {
        use crate::util::should_exclude_path;
        should_exclude_path(path, &self.exclude)
    }

    /// Check if a path should be processed (included and not excluded)
    pub fn should_process_path(&self, path: &std::path::Path) -> bool {
        self.should_include_path(path) && !self.should_exclude_path(path)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.stale_days, Some(14));
        assert_eq!(config.safe_delete, Some(true));
        assert!(config.exclude_paths.is_some());
    }

    #[test]
    fn test_config_merge() {
        let config = Config::default();
        let cli_args = CliArgs {
            list: false,
            dry_run: true,
            force: false,
            json: false,
            lang: Some(LanguageFilter::JavaScript),
            paths: None,
            exclude: None,
            stale_days: Some(7),
            safe_delete: Some(false),
            backup_dir: None,
            docker: true,
            npx: false,
            restore_last: false,
            all: false,
            js_pm: false,
        };

        let merged = config.merge_with_cli(&cli_args);
        assert!(merged.dry_run);
        assert_eq!(merged.lang, LanguageFilter::JavaScript);
        assert_eq!(merged.stale_days, 7);
        assert!(!merged.safe_delete);
        assert!(merged.docker);
    }

    #[test]
    fn test_config_file_loading() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".cachekillrc");
        
        let config_content = r#"
stale_days = 7
safe_delete = false
include_docker = true
"#;
        
        fs::write(&config_path, config_content).unwrap();
        
        // Change to temp directory to test config loading
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let config = Config::load().unwrap();
        assert_eq!(config.stale_days, Some(7));
        assert_eq!(config.safe_delete, Some(false));
        assert_eq!(config.include_docker, Some(true));
    }

    #[test]
    fn test_should_process_path() {
        let config = MergedConfig {
            list: false,
            dry_run: false,
            force: false,
            json: false,
            lang: LanguageFilter::Auto,
            paths: vec!["**/node_modules".to_string()],
            exclude: vec!["**/test".to_string()],
            stale_days: 14,
            safe_delete: true,
            backup_dir: ".cachekill-backup".to_string(),
            docker: false,
            npx: false,
            restore_last: false,
            all: false,
            js_pm: false,
        };

        let node_modules = std::path::Path::new("/project/node_modules");
        let test_dir = std::path::Path::new("/project/test");
        let other_dir = std::path::Path::new("/project/other");

        assert!(config.should_process_path(node_modules));
        assert!(!config.should_process_path(test_dir));
        assert!(!config.should_process_path(other_dir));
    }
}
