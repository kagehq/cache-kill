use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::env;
use std::process::Command;

use crate::config::MergedConfig;

/// Cloudflare purge result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflarePurgeResult {
    pub success: bool,
    pub message: String,
    pub zone_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub method: String, // "api" or "cli"
}

/// Cloudflare cache manager
pub struct CloudflareCacheManager {
    config: MergedConfig,
}

impl CloudflareCacheManager {
    pub fn new(config: MergedConfig) -> Self {
        Self { config }
    }

    /// Check if Cloudflare CLI (wrangler) is available
    pub fn cli_available(&self) -> bool {
        Command::new("wrangler").arg("--version").output().is_ok()
    }

    /// Check if Cloudflare API token is available
    pub fn token_available(&self) -> bool {
        env::var("CF_API_TOKEN").is_ok()
    }

    /// Get Cloudflare API token from environment
    #[allow(dead_code)]
    fn get_token(&self) -> Result<String> {
        env::var("CF_API_TOKEN").context("CF_API_TOKEN environment variable not set")
    }

    /// Purge Cloudflare cache using CLI
    fn purge_via_cli(&self, zone_id: Option<&str>) -> Result<CloudflarePurgeResult> {
        let mut cmd = Command::new("wrangler");
        cmd.arg("pages").arg("purge");

        if let Some(zone) = zone_id {
            cmd.arg("--zone").arg(zone);
        }

        let output = cmd
            .output()
            .context("Failed to execute wrangler CLI command")?;

        let success = output.status.success();
        let message = if success {
            "Cloudflare cache purge initiated via CLI".to_string()
        } else {
            format!(
                "Cloudflare CLI failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        };

        Ok(CloudflarePurgeResult {
            success,
            message,
            zone_id: zone_id.map(|s| s.to_string()),
            timestamp: Utc::now(),
            method: "cli".to_string(),
        })
    }

    /// Purge Cloudflare cache using API
    fn purge_via_api(&self, zone_id: Option<&str>) -> Result<CloudflarePurgeResult> {
        let token = self.get_token()?;

        if let Some(zone) = zone_id {
            // Use curl for Cloudflare API calls
            let mut cmd = Command::new("curl");
            let url = format!(
                "https://api.cloudflare.com/client/v4/zones/{}/purge_cache",
                zone
            );
            let data = r#"{"purge_everything":true}"#;

            cmd.args(&[
                "-X",
                "POST",
                &url,
                "-H",
                &format!("Authorization: Bearer {}", token),
                "-H",
                "Content-Type: application/json",
                "--data",
                data,
            ]);

            let output = cmd
                .output()
                .context("Failed to execute curl command for Cloudflare API")?;

            let success = output.status.success();
            let message = if success {
                format!("Cloudflare cache purge initiated for zone {}", zone)
            } else {
                format!(
                    "Cloudflare API failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
            };

            Ok(CloudflarePurgeResult {
                success,
                message,
                zone_id: Some(zone.to_string()),
                timestamp: Utc::now(),
                method: "api".to_string(),
            })
        } else {
            Ok(CloudflarePurgeResult {
                success: false,
                message:
                    "Zone ID required for Cloudflare API purge. Use --zone <id> or wrangler CLI."
                        .to_string(),
                zone_id: None,
                timestamp: Utc::now(),
                method: "api".to_string(),
            })
        }
    }

    /// Purge Cloudflare cache
    pub fn purge_cache(&self, zone_id: Option<&str>, force: bool) -> Result<CloudflarePurgeResult> {
        if !force && self.config.dry_run {
            return Ok(CloudflarePurgeResult {
                success: true,
                message: "Dry run: Cloudflare cache purge would be executed".to_string(),
                zone_id: zone_id.map(|s| s.to_string()),
                timestamp: Utc::now(),
                method: "dry_run".to_string(),
            });
        }

        // Try CLI first, then API
        if self.cli_available() {
            self.purge_via_cli(zone_id)
        } else if self.token_available() {
            self.purge_via_api(zone_id)
        } else {
            Ok(CloudflarePurgeResult {
                success: false,
                message: "Neither Cloudflare CLI (wrangler) nor CF_API_TOKEN available. Install wrangler or set CF_API_TOKEN environment variable.".to_string(),
                zone_id: zone_id.map(|s| s.to_string()),
                timestamp: Utc::now(),
                method: "none".to_string(),
            })
        }
    }

    /// Get system status
    pub fn get_status(&self) -> serde_json::Value {
        serde_json::json!({
            "cli_available": self.cli_available(),
            "token_available": self.token_available(),
            "token_source": if self.token_available() { serde_json::Value::String("CF_API_TOKEN".to_string()) } else { serde_json::Value::Null },
            "recommendations": self.get_recommendations()
        })
    }

    /// Get recommendations for setup
    fn get_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !self.cli_available() {
            recommendations.push("Install Cloudflare CLI: npm install -g wrangler".to_string());
        }

        if !self.token_available() {
            recommendations.push("Set CF_API_TOKEN environment variable".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Cloudflare integration is ready".to_string());
        }

        recommendations
    }
}

/// Handle Cloudflare purge command
pub fn handle_cloudflare_purge(config: &MergedConfig, zone_id: Option<&str>) -> Result<()> {
    let manager = CloudflareCacheManager::new(config.clone());

    let result = manager.purge_cache(zone_id, config.force)?;

    if config.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        if result.success {
            println!("✅ {}", result.message);
            if let Some(zone) = &result.zone_id {
                println!("   Zone: {}", zone);
            }
            println!("   Method: {}", result.method);
        } else {
            println!("❌ {}", result.message);
        }
    }

    Ok(())
}

/// Handle Cloudflare status command
pub fn handle_cloudflare_status(config: &MergedConfig) -> Result<()> {
    let manager = CloudflareCacheManager::new(config.clone());
    let status = manager.get_status();

    if config.json {
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("☁️ Cloudflare Integration Status");
        println!("CLI Available: {}", status["cli_available"]);
        println!("Token Available: {}", status["token_available"]);

        if let Some(token_source) = status["token_source"].as_str() {
            println!("Token Source: {}", token_source);
        }

        if let Some(recommendations) = status["recommendations"].as_array() {
            if !recommendations.is_empty() {
                println!("\nRecommendations:");
                for rec in recommendations {
                    if let Some(rec_str) = rec.as_str() {
                        println!("  • {}", rec_str);
                    }
                }
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
    fn test_cloudflare_purge_result() {
        let result = CloudflarePurgeResult {
            success: true,
            message: "Test message".to_string(),
            zone_id: Some("test-zone".to_string()),
            timestamp: Utc::now(),
            method: "cli".to_string(),
        };

        assert!(result.success);
        assert_eq!(result.message, "Test message");
        assert_eq!(result.zone_id, Some("test-zone".to_string()));
    }

    #[test]
    fn test_cloudflare_status() {
        let config = MergedConfig::default();
        let manager = CloudflareCacheManager::new(config);
        let status = manager.get_status();

        assert!(status.is_object());
        assert!(status["cli_available"].is_boolean());
        assert!(status["token_available"].is_boolean());
    }
}
