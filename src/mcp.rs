use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Command;

/// CacheKill MCP Server
/// Provides cache management tools through the Model Context Protocol
pub struct CacheKillMcpServer;

impl CacheKillMcpServer {
    pub fn new() -> Self {
        Self
    }

    pub async fn run(&mut self) -> Result<()> {
        // Simple MCP server that delegates to the main cachekill binary
        // This is a basic implementation that can be extended
        
        println!("CacheKill MCP Server starting...");
        println!("Available tools:");
        println!("- list_caches: List all cache entries with details");
        println!("- clean_caches: Clean cache entries");
        println!("- dry_run: Show what would be cleaned without doing it");
        println!("- npx_analysis: Analyze NPX cache with per-package details");
        println!("- docker_stats: Get Docker cache statistics");
        println!("- system_diagnostics: Run system diagnostics");
        println!("- restore_backup: Restore from last backup");
        
        // Keep the server running
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    // Tool implementations that delegate to the main cachekill binary
    pub async fn list_caches(&self, args: HashMap<String, Value>) -> Result<Value> {
        let mut cmd = Command::new("cachekill");
        cmd.arg("--list").arg("--json");
        
        // Add language filter if specified
        if let Some(lang) = args.get("lang").and_then(|v| v.as_str()) {
            cmd.arg("--lang").arg(lang);
        }
        
        // Add NPX if requested
        if let Some(npx) = args.get("npx").and_then(|v| v.as_bool()) {
            if npx {
                cmd.arg("--npx");
            }
        }
        
        // Add Docker if requested
        if let Some(docker) = args.get("docker").and_then(|v| v.as_bool()) {
            if docker {
                cmd.arg("--docker");
            }
        }

        let output = cmd.output()?;
        
        if output.status.success() {
            let result: Value = serde_json::from_slice(&output.stdout)?;
            Ok(result)
        } else {
            Ok(serde_json::json!({
                "error": String::from_utf8_lossy(&output.stderr),
                "success": false
            }))
        }
    }

    pub async fn clean_caches(&self, args: HashMap<String, Value>) -> Result<Value> {
        let mut cmd = Command::new("cachekill");
        cmd.arg("--json");
        
        // Add force flag if specified
        if let Some(force) = args.get("force").and_then(|v| v.as_bool()) {
            if force {
                cmd.arg("--force");
            }
        }
        
        // Add safe delete if specified
        if let Some(safe_delete) = args.get("safe_delete").and_then(|v| v.as_bool()) {
            if safe_delete {
                cmd.arg("--safe-delete");
            }
        }
        
        // Add language filter if specified
        if let Some(lang) = args.get("lang").and_then(|v| v.as_str()) {
            cmd.arg("--lang").arg(lang);
        }
        
        // Add NPX if requested
        if let Some(npx) = args.get("npx").and_then(|v| v.as_bool()) {
            if npx {
                cmd.arg("--npx");
            }
        }
        
        // Add Docker if requested
        if let Some(docker) = args.get("docker").and_then(|v| v.as_bool()) {
            if docker {
                cmd.arg("--docker");
            }
        }

        let output = cmd.output()?;
        
        if output.status.success() {
            let result: Value = serde_json::from_slice(&output.stdout)?;
            Ok(result)
        } else {
            Ok(serde_json::json!({
                "error": String::from_utf8_lossy(&output.stderr),
                "success": false
            }))
        }
    }

    pub async fn dry_run(&self, args: HashMap<String, Value>) -> Result<Value> {
        let mut cmd = Command::new("cachekill");
        cmd.arg("--dry-run").arg("--json");
        
        // Add language filter if specified
        if let Some(lang) = args.get("lang").and_then(|v| v.as_str()) {
            cmd.arg("--lang").arg(lang);
        }
        
        // Add NPX if requested
        if let Some(npx) = args.get("npx").and_then(|v| v.as_bool()) {
            if npx {
                cmd.arg("--npx");
            }
        }
        
        // Add Docker if requested
        if let Some(docker) = args.get("docker").and_then(|v| v.as_bool()) {
            if docker {
                cmd.arg("--docker");
            }
        }

        let output = cmd.output()?;
        
        if output.status.success() {
            let result: Value = serde_json::from_slice(&output.stdout)?;
            Ok(result)
        } else {
            Ok(serde_json::json!({
                "error": String::from_utf8_lossy(&output.stderr),
                "success": false
            }))
        }
    }

    pub async fn npx_analysis(&self, _args: HashMap<String, Value>) -> Result<Value> {
        let mut cmd = Command::new("cachekill");
        cmd.arg("--npx").arg("--list").arg("--json");

        let output = cmd.output()?;
        
        if output.status.success() {
            let result: Value = serde_json::from_slice(&output.stdout)?;
            Ok(result)
        } else {
            Ok(serde_json::json!({
                "error": String::from_utf8_lossy(&output.stderr),
                "success": false
            }))
        }
    }

    pub async fn docker_stats(&self, _args: HashMap<String, Value>) -> Result<Value> {
        let mut cmd = Command::new("cachekill");
        cmd.arg("--docker").arg("--list").arg("--json");

        let output = cmd.output()?;
        
        if output.status.success() {
            let result: Value = serde_json::from_slice(&output.stdout)?;
            Ok(result)
        } else {
            Ok(serde_json::json!({
                "error": String::from_utf8_lossy(&output.stderr),
                "success": false
            }))
        }
    }

    pub async fn system_diagnostics(&self, _args: HashMap<String, Value>) -> Result<Value> {
        let mut cmd = Command::new("cachekill");
        cmd.arg("--doctor").arg("--json");

        let output = cmd.output()?;
        
        if output.status.success() {
            let result: Value = serde_json::from_slice(&output.stdout)?;
            Ok(result)
        } else {
            Ok(serde_json::json!({
                "error": String::from_utf8_lossy(&output.stderr),
                "success": false
            }))
        }
    }

    pub async fn restore_backup(&self, _args: HashMap<String, Value>) -> Result<Value> {
        let mut cmd = Command::new("cachekill");
        cmd.arg("--restore-last").arg("--json");

        let output = cmd.output()?;
        
        if output.status.success() {
            let result: Value = serde_json::from_slice(&output.stdout)?;
            Ok(result)
        } else {
            Ok(serde_json::json!({
                "error": String::from_utf8_lossy(&output.stderr),
                "success": false
            }))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut server = CacheKillMcpServer::new();
    server.run().await
}