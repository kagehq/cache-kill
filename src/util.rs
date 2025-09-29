use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Timelike, Utc};
use dirs;
use std::fs;
use std::path::{Path, PathBuf};

/// Normalize a path by resolving any relative components and expanding home directory
#[allow(dead_code)]
pub fn normalize_path(path: &Path) -> Result<PathBuf> {
    let expanded = if path.starts_with("~") {
        if let Some(home) = dirs::home_dir() {
            home.join(path.strip_prefix("~").unwrap())
        } else {
            path.to_path_buf()
        }
    } else {
        path.to_path_buf()
    };

    Ok(expanded.canonicalize().unwrap_or(expanded))
}

/// Expand home directory in a path string
#[allow(dead_code)]
pub fn expand_home(path: &str) -> PathBuf {
    if path.starts_with("~") {
        if let Some(home) = dirs::home_dir() {
            home.join(path.strip_prefix("~").unwrap())
        } else {
            PathBuf::from(path)
        }
    } else {
        PathBuf::from(path)
    }
}

/// Get the current working directory
pub fn get_current_dir() -> Result<PathBuf> {
    std::env::current_dir().context("Failed to get current working directory")
}

/// Check if a path exists and is accessible
pub fn path_exists(path: &Path) -> bool {
    path.exists()
}

/// Check if a path is a file
#[allow(dead_code)]
pub fn is_file(path: &Path) -> bool {
    path.is_file()
}

/// Check if a path is a directory
pub fn is_dir(path: &Path) -> bool {
    path.is_dir()
}

/// Get the size of a file or directory
pub fn get_size(path: &Path) -> Result<u64> {
    if path.is_file() {
        Ok(fs::metadata(path)?.len())
    } else if path.is_dir() {
        let mut total_size = 0u64;
        for entry in walkdir::WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total_size += entry.metadata()?.len();
            }
        }
        Ok(total_size)
    } else {
        Ok(0)
    }
}

/// Get the modification time of a path
pub fn get_mtime(path: &Path) -> Result<DateTime<Utc>> {
    let metadata = fs::metadata(path)?;
    let system_time = metadata.modified()?;
    let datetime = DateTime::from(system_time);
    Ok(datetime)
}

/// Get the most recent modification time in a directory tree
pub fn get_most_recent_mtime(path: &Path) -> Result<DateTime<Utc>> {
    let mut most_recent = get_mtime(path)?;

    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let entry_mtime = get_mtime(entry.path())?;
            if entry_mtime > most_recent {
                most_recent = entry_mtime;
            }
        }
    }

    Ok(most_recent)
}

/// Check if a path is within a project directory (not following symlinks outside)
#[allow(dead_code)]
pub fn is_within_project(path: &Path, project_root: &Path) -> bool {
    path.canonicalize()
        .map(|canonical| {
            canonical.starts_with(
                project_root
                    .canonicalize()
                    .unwrap_or_else(|_| project_root.to_path_buf()),
            )
        })
        .unwrap_or(false)
}

/// Get the cachekill backup directory
pub fn get_backup_dir() -> PathBuf {
    get_current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".cachekill-backup")
}

/// Create a timestamped backup directory name
pub fn create_backup_dir_name() -> String {
    let now = Utc::now();
    format!(
        "{}-{:02}-{:02}_{:02}-{:02}-{:02}",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

/// Check if a path matches any of the given glob patterns
pub fn matches_any_glob(path: &Path, patterns: &[String]) -> bool {
    use globset::{Glob, GlobSetBuilder};

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            let _ = builder.add(glob);
        }
    }

    if let Ok(glob_set) = builder.build() {
        glob_set.is_match(path)
    } else {
        false
    }
}

/// Check if a path should be excluded based on glob patterns
pub fn should_exclude_path(path: &Path, exclude_patterns: &[String]) -> bool {
    matches_any_glob(path, exclude_patterns)
}

/// Check if a path should be included based on glob patterns
pub fn should_include_path(path: &Path, include_patterns: &[String]) -> bool {
    if include_patterns.is_empty() {
        return true;
    }
    matches_any_glob(path, include_patterns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_expand_home() {
        let _home = dirs::home_dir().unwrap();
        let expanded = expand_home("~/test");
        assert!(expanded.to_string_lossy().contains("test"));
    }

    #[test]
    fn test_path_exists() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test").unwrap();

        assert!(path_exists(&test_file));
        assert!(!path_exists(&temp_dir.path().join("nonexistent.txt")));
    }

    #[test]
    fn test_get_size() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let size = get_size(&test_file).unwrap();
        assert_eq!(size, 12); // "test content".len()
    }

    #[test]
    fn test_is_within_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let sub_path = project_root.join("subdir").join("file.txt");

        fs::create_dir_all(sub_path.parent().unwrap()).unwrap();
        fs::write(&sub_path, "test").unwrap();

        assert!(is_within_project(&sub_path, project_root));
        assert!(!is_within_project(&PathBuf::from("/tmp"), project_root));
    }

    #[test]
    fn test_create_backup_dir_name() {
        let name = create_backup_dir_name();
        assert!(name.contains("-"));
        assert!(name.contains("_"));
    }

    #[test]
    fn test_matches_any_glob() {
        let path = PathBuf::from("/tmp/test.txt");
        let patterns = vec!["*.txt".to_string(), "*.log".to_string()];

        assert!(matches_any_glob(&path, &patterns));

        let patterns2 = vec!["*.log".to_string(), "*.json".to_string()];
        assert!(!matches_any_glob(&path, &patterns2));
    }
}
