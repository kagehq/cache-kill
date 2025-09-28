use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

use crate::cache_entry::{CacheEntry, CacheKind, PlannedAction};
use crate::config::MergedConfig;
use crate::util::{get_most_recent_mtime, get_size, is_dir, path_exists};

pub fn planned_action(_config: &MergedConfig) -> PlannedAction {
    /*
     * These are not backed up.
     * Reason: No direct effect on any project, safe to delete
     */

    PlannedAction::Delete
}

pub fn make_entry(dir: PathBuf, config: &MergedConfig) -> Result<CacheEntry> {
    let size = get_size(&dir)?;
    let last_used: DateTime<Utc> = get_most_recent_mtime(&dir)?;
    let stale = (Utc::now() - last_used).num_days() > config.stale_days as i64;
    Ok(
        CacheEntry::new(dir, CacheKind::JavaScript, size, last_used, stale)
            .with_planned_action(planned_action(config)),
    )
}

pub fn existing_dir(p: &Path) -> bool {
    path_exists(p) && is_dir(p)
}
