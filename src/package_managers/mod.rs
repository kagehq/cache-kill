pub mod common;
pub mod npm;
pub mod pnpm;
pub mod traits;
pub mod yarn;

use crate::cache_entry::CacheEntry;
use crate::config::MergedConfig;
use anyhow::Result;
use traits::CacheManager;

pub struct PackageManagers {
    config: MergedConfig,
}

impl PackageManagers {
    pub fn new(config: MergedConfig) -> Self {
        Self { config }
    }

    fn managers(&self) -> Vec<Box<dyn CacheManager>> {
        vec![
            Box::new(npm::NpmManager::new(self.config.clone())),
            Box::new(pnpm::PnpmManager::new(self.config.clone())),
            Box::new(yarn::YarnManager::new(self.config.clone())),
        ]
    }

    pub fn list_all(&self) -> Result<Vec<CacheEntry>> {
        let managers: Vec<Box<dyn CacheManager>> = self.managers();
        let mut all = Vec::new();
        for m in managers {
            let mut entries = m.list()?;
            all.append(&mut entries);
        }
        Ok(all)
    }
}

/*
Utils for `handle_cleanup_mode`, `handle_dry_run_mode` & `handle_list_mode`
Purpose: readability - the function is too cluttered and need refactoring for future.
 */
pub fn add_js_pm_entries(entries: &mut Vec<CacheEntry>, config: &MergedConfig) -> Result<()> {
    if !config.js_pm {
        return Ok(());
    }
    let pm = PackageManagers::new(config.clone());
    let mut pm_entries = pm.list_all()?;
    entries.append(&mut pm_entries);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_js_pm_entries_is_noop_when_flag_is_false() {
        let mut entries = Vec::new();
        let mut cfg = MergedConfig::default();
        cfg.js_pm = false;
        let res = add_js_pm_entries(&mut entries, &cfg);
        assert!(res.is_ok());
        assert!(entries.is_empty());
    }
}
