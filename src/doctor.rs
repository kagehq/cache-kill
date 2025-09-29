use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::process::Command;

use crate::config::MergedConfig;
use crate::hf::HfCacheManager;
use crate::npx::NpxCacheManager;
use crate::torch::TorchCacheManager;

/// System diagnostic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDiagnostics {
    pub timestamp: DateTime<Utc>,
    pub platform: String,
    pub cachekill_version: String,
    pub integrations: IntegrationStatus,
    pub cache_directories: HashMap<String, CacheDirInfo>,
    pub environment: EnvironmentInfo,
    pub recommendations: Vec<String>,
}

/// Integration status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStatus {
    pub docker: bool,
    pub npx: bool,
    pub vercel: bool,
    pub cloudflare: bool,
    pub huggingface: bool,
    pub torch: bool,
}

/// Cache directory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheDirInfo {
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub size_human: Option<String>,
    pub entry_count: Option<usize>,
    pub last_modified: Option<DateTime<Utc>>,
}

/// Environment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub home_dir: Option<String>,
    pub temp_dir: Option<String>,
    pub cache_dir: Option<String>,
    pub environment_variables: HashMap<String, String>,
}

/// System doctor
pub struct SystemDoctor {
    config: MergedConfig,
}

impl SystemDoctor {
    pub fn new(config: MergedConfig) -> Self {
        Self { config }
    }

    /// Run comprehensive system diagnostics
    pub fn diagnose(&self) -> Result<SystemDiagnostics> {
        let timestamp = Utc::now();
        let platform = self.get_platform();
        let cachekill_version = env!("CARGO_PKG_VERSION").to_string();

        let integrations = self.check_integrations();
        let cache_directories = self.check_cache_directories();
        let environment = self.check_environment();
        let recommendations = self.generate_recommendations(&integrations, &cache_directories);

        Ok(SystemDiagnostics {
            timestamp,
            platform,
            cachekill_version,
            integrations,
            cache_directories,
            environment,
            recommendations,
        })
    }

    /// Get platform information
    fn get_platform(&self) -> String {
        format!("{} {}", env::consts::OS, env::consts::ARCH)
    }

    /// Check integration availability
    fn check_integrations(&self) -> IntegrationStatus {
        IntegrationStatus {
            docker: self.check_docker(),
            npx: self.check_npx(),
            vercel: self.check_vercel(),
            cloudflare: self.check_cloudflare(),
            huggingface: self.check_huggingface(),
            torch: self.check_torch(),
        }
    }

    /// Check Docker availability
    fn check_docker(&self) -> bool {
        Command::new("docker")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Check NPX availability
    fn check_npx(&self) -> bool {
        Command::new("npx")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Check Vercel availability
    fn check_vercel(&self) -> bool {
        Command::new("vercel")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Check Cloudflare availability
    fn check_cloudflare(&self) -> bool {
        Command::new("wrangler")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Check HuggingFace cache
    fn check_huggingface(&self) -> bool {
        HfCacheManager::new(self.config.clone()).cache_exists()
    }

    /// Check PyTorch cache
    fn check_torch(&self) -> bool {
        TorchCacheManager::new(self.config.clone()).cache_exists()
    }

    /// Check cache directories
    fn check_cache_directories(&self) -> HashMap<String, CacheDirInfo> {
        let mut directories = HashMap::new();

        // Check various cache directories
        let cache_dirs = vec![
            ("npx", NpxCacheManager::get_npx_cache_dir().ok()),
            ("huggingface", HfCacheManager::get_hf_cache_dir().ok()),
            ("torch", TorchCacheManager::get_torch_cache_dir().ok()),
        ];

        for (name, dir_path) in cache_dirs {
            if let Some(path) = dir_path {
                let info = self.analyze_cache_directory(&path);
                directories.insert(name.to_string(), info);
            }
        }

        directories
    }

    /// Analyze a cache directory
    fn analyze_cache_directory(&self, path: &std::path::Path) -> CacheDirInfo {
        let exists = path.exists();
        let mut size_bytes = None;
        let mut size_human = None;
        let mut entry_count = None;
        let mut last_modified = None;

        if exists {
            if let Ok(size) = crate::util::get_size(path) {
                size_bytes = Some(size);
                size_human = Some(humansize::format_size(size, humansize::DECIMAL));
            }

            if let Ok(count) = self.count_entries(path) {
                entry_count = Some(count);
            }

            if let Ok(mtime) = crate::util::get_most_recent_mtime(path) {
                last_modified = Some(mtime);
            }
        }

        CacheDirInfo {
            exists,
            size_bytes,
            size_human,
            entry_count,
            last_modified,
        }
    }

    /// Count entries in a directory
    fn count_entries(&self, path: &std::path::Path) -> Result<usize> {
        let mut count = 0;
        for entry in std::fs::read_dir(path)? {
            let _ = entry?;
            count += 1;
        }
        Ok(count)
    }

    /// Check environment
    fn check_environment(&self) -> EnvironmentInfo {
        let home_dir = dirs::home_dir().map(|p| p.to_string_lossy().to_string());
        let temp_dir = std::env::temp_dir().to_string_lossy().to_string().into();
        let cache_dir = dirs::cache_dir().map(|p| p.to_string_lossy().to_string());

        let mut env_vars = HashMap::new();
        for (key, value) in env::vars() {
            if key.starts_with("CACHEKILL_")
                || key == "VERCEL_TOKEN"
                || key == "CF_API_TOKEN"
                || key == "DOCKER_HOST"
            {
                env_vars.insert(key, value);
            }
        }

        EnvironmentInfo {
            home_dir,
            temp_dir: Some(temp_dir),
            cache_dir,
            environment_variables: env_vars,
        }
    }

    /// Generate recommendations
    fn generate_recommendations(
        &self,
        integrations: &IntegrationStatus,
        cache_dirs: &HashMap<String, CacheDirInfo>,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !integrations.docker {
            recommendations.push("Install Docker for container cache management".to_string());
        }

        if !integrations.npx {
            recommendations.push("Install Node.js and npm for NPX cache management".to_string());
        }

        if !integrations.vercel {
            recommendations.push("Install Vercel CLI for edge cache purging".to_string());
        }

        if !integrations.cloudflare {
            recommendations
                .push("Install Cloudflare CLI (wrangler) for edge cache purging".to_string());
        }

        if cache_dirs.is_empty() {
            recommendations.push(
                "No cache directories found - try running cachekill in a project directory"
                    .to_string(),
            );
        }

        if recommendations.is_empty() {
            recommendations.push("All integrations are ready!".to_string());
        }

        recommendations
    }
}

/// Handle doctor command
pub fn handle_doctor(config: &MergedConfig) -> Result<()> {
    let doctor = SystemDoctor::new(config.clone());
    let diagnostics = doctor.diagnose()?;

    if config.json {
        println!("{}", serde_json::to_string_pretty(&diagnostics)?);
    } else {
        println!("🔍 CacheKill System Diagnostics");
        println!("Version: {}", diagnostics.cachekill_version);
        println!("Platform: {}", diagnostics.platform);
        println!(
            "Timestamp: {}",
            diagnostics.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );

        println!("\n🔧 Integrations:");
        println!(
            "  Docker: {}",
            if diagnostics.integrations.docker {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  NPX: {}",
            if diagnostics.integrations.npx {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  Vercel: {}",
            if diagnostics.integrations.vercel {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  Cloudflare: {}",
            if diagnostics.integrations.cloudflare {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  HuggingFace: {}",
            if diagnostics.integrations.huggingface {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  PyTorch: {}",
            if diagnostics.integrations.torch {
                "✅"
            } else {
                "❌"
            }
        );

        if !diagnostics.cache_directories.is_empty() {
            println!("\n📁 Cache Directories:");
            for (name, info) in &diagnostics.cache_directories {
                let status = if info.exists { "✅" } else { "❌" };
                let size = info.size_human.as_deref().unwrap_or("N/A");
                let count = info
                    .entry_count
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                println!("  {}: {} ({} entries, {})", name, status, count, size);
            }
        }

        if !diagnostics.recommendations.is_empty() {
            println!("\n💡 Recommendations:");
            for rec in &diagnostics.recommendations {
                println!("  • {}", rec);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MergedConfig;

    #[test]
    fn test_system_diagnostics() {
        let config = MergedConfig::default();
        let doctor = SystemDoctor::new(config);
        let diagnostics = doctor.diagnose().unwrap();

        assert!(!diagnostics.cachekill_version.is_empty());
        assert!(!diagnostics.platform.is_empty());
    }

    #[test]
    fn test_integration_status() {
        let status = IntegrationStatus {
            docker: true,
            npx: false,
            vercel: true,
            cloudflare: false,
            huggingface: true,
            torch: false,
        };

        assert!(status.docker);
        assert!(!status.npx);
    }
}
