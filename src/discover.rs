use std::path::{Path, PathBuf};
use std::fs;
use anyhow::Result;
use crate::cache_entry::{CacheKind, LanguageFilter};
use crate::util::{get_current_dir, path_exists, is_dir};
use crate::config::MergedConfig;

/// Detected project type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectType {
    JavaScript,
    Python,
    Rust,
    Java,
    MachineLearning,
    Mixed,
    Unknown,
}

/// Cache discovery result
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    #[allow(dead_code)]
    pub project_type: ProjectType,
    pub cache_entries: Vec<PathBuf>,
    #[allow(dead_code)]
    pub project_root: PathBuf,
}

impl ProjectType {
    /// Detect project type from directory contents
    pub fn detect(project_root: &Path) -> Result<Self> {
        let mut types = Vec::new();

        // Check for JavaScript/TypeScript projects
        if project_root.join("package.json").exists() ||
           project_root.join("yarn.lock").exists() ||
           project_root.join("pnpm-lock.yaml").exists() ||
           project_root.join("bun.lockb").exists() {
            types.push(ProjectType::JavaScript);
        }

        // Check for Python projects
        if project_root.join("pyproject.toml").exists() ||
           project_root.join("requirements.txt").exists() ||
           project_root.join("setup.py").exists() ||
           project_root.join("Pipfile").exists() ||
           project_root.join("poetry.lock").exists() {
            types.push(ProjectType::Python);
        }

        // Check for Rust projects
        if project_root.join("Cargo.toml").exists() {
            types.push(ProjectType::Rust);
        }

        // Check for Java projects
        if project_root.join("pom.xml").exists() ||
           project_root.join("build.gradle").exists() ||
           project_root.join("build.gradle.kts").exists() ||
           project_root.join("gradlew").exists() {
            types.push(ProjectType::Java);
        }

        // Check for ML/AI projects
        if project_root.join("requirements.txt").exists() && 
           (fs::read_to_string(project_root.join("requirements.txt"))
               .unwrap_or_default()
               .contains("torch") ||
            fs::read_to_string(project_root.join("requirements.txt"))
                .unwrap_or_default()
                .contains("tensorflow") ||
            fs::read_to_string(project_root.join("requirements.txt"))
                .unwrap_or_default()
                .contains("huggingface")) ||
           project_root.join(".dvc").exists() {
            types.push(ProjectType::MachineLearning);
        }

        match types.len() {
            0 => Ok(ProjectType::Unknown),
            1 => Ok(types[0].clone()),
            _ => Ok(ProjectType::Mixed),
        }
    }

    /// Get cache kinds for this project type
    #[allow(dead_code)]
    pub fn get_cache_kinds(&self) -> Vec<CacheKind> {
        match self {
            ProjectType::JavaScript => vec![
                CacheKind::JavaScript,
                CacheKind::Generic,
            ],
            ProjectType::Python => vec![
                CacheKind::Python,
                CacheKind::Generic,
            ],
            ProjectType::Rust => vec![
                CacheKind::Rust,
                CacheKind::Generic,
            ],
            ProjectType::Java => vec![
                CacheKind::Java,
                CacheKind::Generic,
            ],
            ProjectType::MachineLearning => vec![
                CacheKind::MachineLearning,
                CacheKind::Python,
                CacheKind::Generic,
            ],
            ProjectType::Mixed => vec![
                CacheKind::JavaScript,
                CacheKind::Python,
                CacheKind::Rust,
                CacheKind::Java,
                CacheKind::MachineLearning,
                CacheKind::Generic,
            ],
            ProjectType::Unknown => vec![CacheKind::Generic],
        }
    }
}

impl DiscoveryResult {
    /// Discover cache entries in the project
    pub fn discover(config: &MergedConfig) -> Result<Self> {
        let project_root = get_current_dir()?;
        let project_type = ProjectType::detect(&project_root)?;
        
        let mut cache_entries = Vec::new();
        
        // Discover caches based on project type and language filter
        // Always include project-type-specific caches when lang is Auto
        let should_discover_js = config.lang == LanguageFilter::Auto || config.lang == LanguageFilter::JavaScript;
        let should_discover_py = config.lang == LanguageFilter::Auto || config.lang == LanguageFilter::Python;
        let should_discover_rust = config.lang == LanguageFilter::Auto || config.lang == LanguageFilter::Rust;
        let should_discover_java = config.lang == LanguageFilter::Auto || config.lang == LanguageFilter::Java;
        let should_discover_ml = config.lang == LanguageFilter::Auto || config.lang == LanguageFilter::MachineLearning;
        
        // For Auto mode, also include project-type-specific caches
        if config.lang == LanguageFilter::Auto {
            match project_type {
                ProjectType::JavaScript => {
                    cache_entries.extend(Self::discover_js_caches(&project_root, config)?);
                }
                ProjectType::Python => {
                    cache_entries.extend(Self::discover_py_caches(&project_root, config)?);
                }
                ProjectType::Rust => {
                    cache_entries.extend(Self::discover_rust_caches(&project_root, config)?);
                }
                ProjectType::Java => {
                    cache_entries.extend(Self::discover_java_caches(&project_root, config)?);
                }
                ProjectType::MachineLearning => {
                    cache_entries.extend(Self::discover_ml_caches(&project_root, config)?);
                }
                ProjectType::Mixed => {
                    // For mixed projects, include all relevant caches
                    cache_entries.extend(Self::discover_js_caches(&project_root, config)?);
                    cache_entries.extend(Self::discover_py_caches(&project_root, config)?);
                    cache_entries.extend(Self::discover_rust_caches(&project_root, config)?);
                    cache_entries.extend(Self::discover_java_caches(&project_root, config)?);
                    cache_entries.extend(Self::discover_ml_caches(&project_root, config)?);
                }
                ProjectType::Unknown => {
                    // For unknown projects, try to discover all caches
                    cache_entries.extend(Self::discover_js_caches(&project_root, config)?);
                    cache_entries.extend(Self::discover_py_caches(&project_root, config)?);
                    cache_entries.extend(Self::discover_rust_caches(&project_root, config)?);
                    cache_entries.extend(Self::discover_java_caches(&project_root, config)?);
                    cache_entries.extend(Self::discover_ml_caches(&project_root, config)?);
                }
            }
        } else {
            // For specific language filters, only include those caches
            if should_discover_js {
                cache_entries.extend(Self::discover_js_caches(&project_root, config)?);
            }
            if should_discover_py {
                cache_entries.extend(Self::discover_py_caches(&project_root, config)?);
            }
            if should_discover_rust {
                cache_entries.extend(Self::discover_rust_caches(&project_root, config)?);
            }
            if should_discover_java {
                cache_entries.extend(Self::discover_java_caches(&project_root, config)?);
            }
            if should_discover_ml {
                cache_entries.extend(Self::discover_ml_caches(&project_root, config)?);
            }
        }
        
        if config.all {
            cache_entries.extend(Self::discover_generic_caches(&project_root, config)?);
        }
        
        // Add custom paths if specified
        if !config.paths.is_empty() {
            cache_entries.extend(Self::discover_custom_paths(&project_root, config)?);
        }

        Ok(Self {
            project_type,
            cache_entries,
            project_root,
        })
    }

    /// Discover JavaScript/TypeScript caches
    fn discover_js_caches(project_root: &Path, config: &MergedConfig) -> Result<Vec<PathBuf>> {
        let mut caches = Vec::new();
        
        let js_cache_patterns = vec![
            "node_modules",
            ".next",
            ".nuxt",
            ".vite",
            ".cache",
            "dist",
            "coverage",
            ".turbo",
            ".parcel-cache",
            "build",
            "out",
            ".next/cache",
            ".nuxt/dist",
        ];

        for pattern in js_cache_patterns {
            let cache_path = project_root.join(pattern);
            if path_exists(&cache_path) && is_dir(&cache_path) {
                if config.should_process_path(&cache_path) {
                    caches.push(cache_path);
                }
            }
        }

        Ok(caches)
    }

    /// Discover Python caches
    fn discover_py_caches(project_root: &Path, config: &MergedConfig) -> Result<Vec<PathBuf>> {
        let mut caches = Vec::new();
        
        let py_cache_patterns = vec![
            "__pycache__",
            ".pytest_cache",
            ".venv",
            "venv",
            ".tox",
            ".mypy_cache",
            ".ruff_cache",
            ".pip-cache",
            ".coverage",
            "htmlcov",
            ".pytest_cache",
        ];

        for pattern in py_cache_patterns {
            let cache_path = project_root.join(pattern);
            if path_exists(&cache_path) && is_dir(&cache_path) {
                if config.should_process_path(&cache_path) {
                    caches.push(cache_path);
                }
            }
        }

        // Also check for __pycache__ directories in subdirectories
        if let Ok(entries) = fs::read_dir(project_root) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let pycache_path = entry.path().join("__pycache__");
                    if path_exists(&pycache_path) && is_dir(&pycache_path) {
                        if config.should_process_path(&pycache_path) {
                            caches.push(pycache_path);
                        }
                    }
                }
            }
        }

        Ok(caches)
    }

    /// Discover Rust caches
    fn discover_rust_caches(project_root: &Path, config: &MergedConfig) -> Result<Vec<PathBuf>> {
        let mut caches = Vec::new();
        
        let rust_cache_patterns = vec![
            "target",
            ".cargo",
        ];

        for pattern in rust_cache_patterns {
            let cache_path = project_root.join(pattern);
            if path_exists(&cache_path) && is_dir(&cache_path) {
                if config.should_process_path(&cache_path) {
                    caches.push(cache_path);
                }
            }
        }

        Ok(caches)
    }

    /// Discover Java caches
    fn discover_java_caches(project_root: &Path, config: &MergedConfig) -> Result<Vec<PathBuf>> {
        let mut caches = Vec::new();
        
        let java_cache_patterns = vec![
            ".gradle",
            "build",
            "target",
            ".m2",
        ];

        for pattern in java_cache_patterns {
            let cache_path = project_root.join(pattern);
            if path_exists(&cache_path) && is_dir(&cache_path) {
                if config.should_process_path(&cache_path) {
                    caches.push(cache_path);
                }
            }
        }

        // Check for Maven repository in home directory
        if let Some(home) = dirs::home_dir() {
            let m2_repo = home.join(".m2").join("repository");
            if path_exists(&m2_repo) && is_dir(&m2_repo) {
                if config.should_process_path(&m2_repo) {
                    caches.push(m2_repo);
                }
            }
        }

        Ok(caches)
    }

    /// Discover ML/AI caches
    fn discover_ml_caches(project_root: &Path, config: &MergedConfig) -> Result<Vec<PathBuf>> {
        let mut caches = Vec::new();
        
        let ml_cache_patterns = vec![
            ".dvc/cache",
            ".dvc/tmp",
            "wandb",
            ".wandb",
        ];

        for pattern in ml_cache_patterns {
            let cache_path = project_root.join(pattern);
            if path_exists(&cache_path) && is_dir(&cache_path) {
                if config.should_process_path(&cache_path) {
                    caches.push(cache_path);
                }
            }
        }

        // Check for global ML caches
        if let Some(home) = dirs::home_dir() {
            let ml_caches = vec![
                home.join(".cache").join("huggingface"),
                home.join(".cache").join("torch"),
                home.join(".cache").join("transformers"),
            ];

            for cache_path in ml_caches {
                if path_exists(&cache_path) && is_dir(&cache_path) {
                    if config.should_process_path(&cache_path) {
                        caches.push(cache_path);
                    }
                }
            }
        }

        Ok(caches)
    }

    /// Discover generic caches
    fn discover_generic_caches(project_root: &Path, config: &MergedConfig) -> Result<Vec<PathBuf>> {
        let mut caches = Vec::new();
        
        let generic_cache_patterns = vec![
            "tmp",
            "temp",
            ".cache",
            "cache",
            ".tmp",
        ];

        for pattern in generic_cache_patterns {
            let cache_path = project_root.join(pattern);
            if path_exists(&cache_path) && is_dir(&cache_path) {
                if config.should_process_path(&cache_path) {
                    caches.push(cache_path);
                }
            }
        }

        Ok(caches)
    }

    /// Discover custom paths specified in config
    fn discover_custom_paths(project_root: &Path, config: &MergedConfig) -> Result<Vec<PathBuf>> {
        let mut caches = Vec::new();
        
        for pattern in &config.paths {
            // Handle glob patterns
            if pattern.contains('*') || pattern.contains('?') {
                use globset::{Glob, GlobSetBuilder};
                
                let mut builder = GlobSetBuilder::new();
                if let Ok(glob) = Glob::new(pattern) {
                    let _ = builder.add(glob);
                }
                
                if let Ok(glob_set) = builder.build() {
                    for entry in walkdir::WalkDir::new(project_root) {
                        if let Ok(entry) = entry {
                            if glob_set.is_match(entry.path()) {
                                if config.should_process_path(entry.path()) {
                                    caches.push(entry.path().to_path_buf());
                                }
                            }
                        }
                    }
                }
            } else {
                // Handle simple path
                let cache_path = if pattern.starts_with('/') {
                    PathBuf::from(pattern)
                } else {
                    project_root.join(pattern)
                };
                
                if path_exists(&cache_path) {
                    if config.should_process_path(&cache_path) {
                        caches.push(cache_path);
                    }
                }
            }
        }

        Ok(caches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_project_type_detection() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test JavaScript project
        fs::write(temp_dir.path().join("package.json"), "{}").unwrap();
        assert_eq!(ProjectType::detect(temp_dir.path()).unwrap(), ProjectType::JavaScript);
        
        // Test Python project
        fs::write(temp_dir.path().join("requirements.txt"), "requests").unwrap();
        assert_eq!(ProjectType::detect(temp_dir.path()).unwrap(), ProjectType::Mixed);
        
        // Test Rust project
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]").unwrap();
        assert_eq!(ProjectType::detect(temp_dir.path()).unwrap(), ProjectType::Mixed);
    }

    #[test]
    fn test_cache_kinds_for_project_type() {
        let js_kinds = ProjectType::JavaScript.get_cache_kinds();
        assert!(js_kinds.contains(&CacheKind::JavaScript));
        assert!(js_kinds.contains(&CacheKind::Generic));
        
        let rust_kinds = ProjectType::Rust.get_cache_kinds();
        assert!(rust_kinds.contains(&CacheKind::Rust));
        assert!(rust_kinds.contains(&CacheKind::Generic));
    }

    #[test]
    fn test_discovery_result() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Create a simple JavaScript project
        fs::write("package.json", "{}").unwrap();
        fs::create_dir_all("node_modules").unwrap();
        fs::create_dir_all("dist").unwrap();
        
        let config = MergedConfig {
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
        };
        
        let result = DiscoveryResult::discover(&config).unwrap();
        assert_eq!(result.project_type, ProjectType::JavaScript);
        assert!(!result.cache_entries.is_empty());
    }
}
