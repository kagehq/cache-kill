use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::env;
use std::process::Command;

use crate::config::MergedConfig;

/// Vercel purge result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VercelPurgeResult {
    pub success: bool,
    pub message: String,
    pub project_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub method: String, // "api" or "cli"
}

/// Vercel cache manager
pub struct VercelCacheManager {
    config: MergedConfig,
}

impl VercelCacheManager {
    pub fn new(config: MergedConfig) -> Self {
        Self { config }
    }

    /// Check if Vercel CLI is available
    pub fn cli_available(&self) -> bool {
        Command::new("vercel").arg("--version").output().is_ok()
    }

    /// Check if Vercel token is available
    pub fn token_available(&self) -> bool {
        env::var("VERCEL_TOKEN").is_ok()
    }

    /// Get Vercel token from environment
    #[allow(dead_code)]
    fn get_token(&self) -> Result<String> {
        env::var("VERCEL_TOKEN").context("VERCEL_TOKEN environment variable not set")
    }

    /// Purge Vercel cache using CLI
    fn purge_via_cli(&self, project_id: Option<&str>) -> Result<VercelPurgeResult> {
        let mut cmd = Command::new("vercel");
        cmd.arg("deploy").arg("--prebuilt").arg("--force");

        if let Some(project) = project_id {
            cmd.arg("--project").arg(project);
        }

        let output = cmd
            .output()
            .context("Failed to execute vercel CLI command")?;

        let success = output.status.success();
        let message = if success {
            "Vercel cache purge initiated via CLI".to_string()
        } else {
            format!(
                "Vercel CLI failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        };

        Ok(VercelPurgeResult {
            success,
            message,
            project_id: project_id.map(|s| s.to_string()),
            timestamp: Utc::now(),
            method: "cli".to_string(),
        })
    }

    /// Purge Vercel cache using API
    fn purge_via_api(&self, project_id: Option<&str>) -> Result<VercelPurgeResult> {
        let token = self.get_token()?;

        // Use curl as a fallback for API calls
        let mut cmd = Command::new("curl");
        cmd.args(&[
            "-X",
            "POST",
            "-H",
            &format!("Authorization: Bearer {}", token),
            "-H",
            "Content-Type: application/json",
        ]);

        if let Some(project) = project_id {
            // Purge specific project
            let url = format!(
                "https://api.vercel.com/v1/integrations/deployments/{}/revalidate",
                project
            );
            cmd.arg(&url);
        } else {
            // Purge all projects (this might not be supported by Vercel API)
            return Ok(VercelPurgeResult {
                success: false,
                message:
                    "Project ID required for Vercel API purge. Use --project <id> or Vercel CLI."
                        .to_string(),
                project_id: None,
                timestamp: Utc::now(),
                method: "api".to_string(),
            });
        }

        let output = cmd
            .output()
            .context("Failed to execute curl command for Vercel API")?;

        let success = output.status.success();
        let message = if success {
            "Vercel cache purge initiated via API".to_string()
        } else {
            format!(
                "Vercel API failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        };

        Ok(VercelPurgeResult {
            success,
            message,
            project_id: project_id.map(|s| s.to_string()),
            timestamp: Utc::now(),
            method: "api".to_string(),
        })
    }

    /// Purge Vercel cache
    pub fn purge_cache(&self, project_id: Option<&str>, force: bool) -> Result<VercelPurgeResult> {
        if !force && self.config.dry_run {
            return Ok(VercelPurgeResult {
                success: true,
                message: "Dry run: Vercel cache purge would be executed".to_string(),
                project_id: project_id.map(|s| s.to_string()),
                timestamp: Utc::now(),
                method: "dry_run".to_string(),
            });
        }

        // Try CLI first, then API
        if self.cli_available() {
            self.purge_via_cli(project_id)
        } else if self.token_available() {
            self.purge_via_api(project_id)
        } else {
            Ok(VercelPurgeResult {
                success: false,
                message: "Neither Vercel CLI nor VERCEL_TOKEN available. Install Vercel CLI or set VERCEL_TOKEN environment variable.".to_string(),
                project_id: project_id.map(|s| s.to_string()),
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
            "token_source": if self.token_available() { serde_json::Value::String("VERCEL_TOKEN".to_string()) } else { serde_json::Value::Null },
            "recommendations": self.get_recommendations()
        })
    }

    /// Get recommendations for setup
    fn get_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !self.cli_available() {
            recommendations.push("Install Vercel CLI: npm install -g vercel".to_string());
        }

        if !self.token_available() {
            recommendations.push("Set VERCEL_TOKEN environment variable".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Vercel integration is ready".to_string());
        }

        recommendations
    }
}

/// Handle Vercel purge command
pub fn handle_vercel_purge(config: &MergedConfig, project_id: Option<&str>) -> Result<()> {
    let manager = VercelCacheManager::new(config.clone());

    let result = manager.purge_cache(project_id, config.force)?;

    if config.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        if result.success {
            println!("✅ {}", result.message);
            if let Some(project) = &result.project_id {
                println!("   Project: {}", project);
            }
            println!("   Method: {}", result.method);
        } else {
            println!("❌ {}", result.message);
        }
    }

    Ok(())
}

/// Handle Vercel status command
pub fn handle_vercel_status(config: &MergedConfig) -> Result<()> {
    let manager = VercelCacheManager::new(config.clone());
    let status = manager.get_status();

    if config.json {
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("🚀 Vercel Integration Status");
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
    fn test_vercel_purge_result() {
        let result = VercelPurgeResult {
            success: true,
            message: "Test message".to_string(),
            project_id: Some("test-project".to_string()),
            timestamp: Utc::now(),
            method: "cli".to_string(),
        };

        assert!(result.success);
        assert_eq!(result.message, "Test message");
        assert_eq!(result.project_id, Some("test-project".to_string()));
    }

    #[test]
    fn test_vercel_status() {
        let config = MergedConfig::default();
        let manager = VercelCacheManager::new(config);
        let status = manager.get_status();

        assert!(status.is_object());
        assert!(status["cli_available"].is_boolean());
        assert!(status["token_available"].is_boolean());
    }
}
