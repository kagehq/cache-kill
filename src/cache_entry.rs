use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a cache entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Path to the cache directory or file
    pub path: PathBuf,
    /// Type of cache (js, py, rust, java, ml, npx, docker, generic)
    pub kind: CacheKind,
    /// Size in bytes
    pub size_bytes: u64,
    /// Last used timestamp
    pub last_used: DateTime<Utc>,
    /// Whether this cache is considered stale
    pub stale: bool,
    /// Planned action for this entry
    pub planned_action: Option<PlannedAction>,
}

/// Types of caches that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheKind {
    #[serde(rename = "js")]
    JavaScript,
    #[serde(rename = "py")]
    Python,
    #[serde(rename = "rust")]
    Rust,
    #[serde(rename = "java")]
    Java,
    #[serde(rename = "ml")]
    MachineLearning,
    #[serde(rename = "npx")]
    Npx,
    #[serde(rename = "docker")]
    Docker,
    #[serde(rename = "generic")]
    Generic,
}

impl std::fmt::Display for CacheKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheKind::JavaScript => write!(f, "js"),
            CacheKind::Python => write!(f, "py"),
            CacheKind::Rust => write!(f, "rust"),
            CacheKind::Java => write!(f, "java"),
            CacheKind::MachineLearning => write!(f, "ml"),
            CacheKind::Npx => write!(f, "npx"),
            CacheKind::Docker => write!(f, "docker"),
            CacheKind::Generic => write!(f, "generic"),
        }
    }
}

/// Planned action for a cache entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlannedAction {
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "backup")]
    Backup,
    #[serde(rename = "skip")]
    Skip,
}

impl std::fmt::Display for PlannedAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlannedAction::Delete => write!(f, "delete"),
            PlannedAction::Backup => write!(f, "backup"),
            PlannedAction::Skip => write!(f, "skip"),
        }
    }
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(
        path: PathBuf,
        kind: CacheKind,
        size_bytes: u64,
        last_used: DateTime<Utc>,
        stale: bool,
    ) -> Self {
        Self {
            path,
            kind,
            size_bytes,
            last_used,
            stale,
            planned_action: None,
        }
    }

    /// Set the planned action for this entry
    pub fn with_planned_action(mut self, action: PlannedAction) -> Self {
        self.planned_action = Some(action);
        self
    }

    /// Get a human-readable size string
    pub fn size_human(&self) -> String {
        humansize::format_size(self.size_bytes, humansize::DECIMAL)
    }

    /// Get a human-readable last used string
    pub fn last_used_human(&self) -> String {
        let now = Utc::now();
        let duration = now - self.last_used;

        if duration.num_days() > 0 {
            format!("{}d ago", duration.num_days())
        } else if duration.num_hours() > 0 {
            format!("{}h ago", duration.num_hours())
        } else if duration.num_minutes() > 0 {
            format!("{}m ago", duration.num_minutes())
        } else {
            "just now".to_string()
        }
    }

    /// Check if this entry should be included based on language filter
    #[allow(dead_code)]
    pub fn matches_lang_filter(&self, filter: &LanguageFilter) -> bool {
        match filter {
            LanguageFilter::Auto => true,
            LanguageFilter::JavaScript => matches!(self.kind, CacheKind::JavaScript),
            LanguageFilter::Python => matches!(self.kind, CacheKind::Python),
            LanguageFilter::Rust => matches!(self.kind, CacheKind::Rust),
            LanguageFilter::Java => matches!(self.kind, CacheKind::Java),
            LanguageFilter::MachineLearning => matches!(self.kind, CacheKind::MachineLearning),
        }
    }
}

/// Language filter for cache detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageFilter {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "js")]
    JavaScript,
    #[serde(rename = "py")]
    Python,
    #[serde(rename = "rust")]
    Rust,
    #[serde(rename = "java")]
    Java,
    #[serde(rename = "ml")]
    MachineLearning,
}

impl std::str::FromStr for LanguageFilter {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(LanguageFilter::Auto),
            "js" | "javascript" => Ok(LanguageFilter::JavaScript),
            "py" | "python" => Ok(LanguageFilter::Python),
            "rust" => Ok(LanguageFilter::Rust),
            "java" => Ok(LanguageFilter::Java),
            "ml" | "machinelearning" => Ok(LanguageFilter::MachineLearning),
            _ => Err(format!("Unknown language filter: {}", s)),
        }
    }
}

impl std::fmt::Display for LanguageFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LanguageFilter::Auto => write!(f, "auto"),
            LanguageFilter::JavaScript => write!(f, "js"),
            LanguageFilter::Python => write!(f, "py"),
            LanguageFilter::Rust => write!(f, "rust"),
            LanguageFilter::Java => write!(f, "java"),
            LanguageFilter::MachineLearning => write!(f, "ml"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_cache_kind_display() {
        assert_eq!(CacheKind::JavaScript.to_string(), "js");
        assert_eq!(CacheKind::Python.to_string(), "py");
        assert_eq!(CacheKind::Rust.to_string(), "rust");
    }

    #[test]
    fn test_language_filter_parsing() {
        assert_eq!(
            LanguageFilter::from_str("auto").unwrap(),
            LanguageFilter::Auto
        );
        assert_eq!(
            LanguageFilter::from_str("js").unwrap(),
            LanguageFilter::JavaScript
        );
        assert_eq!(
            LanguageFilter::from_str("python").unwrap(),
            LanguageFilter::Python
        );
        assert!(LanguageFilter::from_str("invalid").is_err());
    }

    #[test]
    fn test_cache_entry_creation() {
        let now = Utc::now();
        let entry = CacheEntry::new(
            PathBuf::from("/tmp/test"),
            CacheKind::JavaScript,
            1024,
            now,
            false,
        );

        assert_eq!(entry.kind, CacheKind::JavaScript);
        assert_eq!(entry.size_bytes, 1024);
        assert!(!entry.stale);
        assert!(entry.planned_action.is_none());
    }

    #[test]
    fn test_size_human_formatting() {
        let entry = CacheEntry::new(
            PathBuf::from("/tmp/test"),
            CacheKind::JavaScript,
            1024,
            Utc::now(),
            false,
        );

        let size_str = entry.size_human();
        assert!(size_str.contains("1.0") || size_str.contains("1024"));
    }
}
