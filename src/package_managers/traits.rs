use anyhow::Result;
use crate::cache_entry::CacheEntry;

/// Trait implemented by each package manager cache handler
pub trait CacheManager {
    /// A short static name (e.g., "npm", "pnpm", "yarn")
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    /// List cache entries for this package manager
    fn list(&self) -> Result<Vec<CacheEntry>>;

    /// Paths/patterns that should be excluded from backups
    #[allow(dead_code)]
    fn exclude_patterns(&self) -> Vec<String>;
}
