use clap::Parser;
use anyhow::Result;
use std::process;

mod cache_entry;
mod config;
mod discover;
mod inspect;
mod actions;
mod output;
mod npx;
mod docker;
mod util;
mod ci;
mod hf;
mod torch;
mod edge;
mod doctor;

use config::{Config, CliArgs, MergedConfig};
use discover::DiscoveryResult;
use inspect::CacheInspector;
use actions::ActionExecutor;
use output::OutputFormatter;
use npx::NpxCacheManager;
use docker::DockerCacheManager;
use ci::{CiMode, handle_ci_mode};
use hf::{handle_hf_list, handle_hf_clean};
use torch::{handle_torch_list, handle_torch_clean};
use edge::{handle_vercel_purge, handle_vercel_status, handle_cloudflare_purge, handle_cloudflare_status};
use doctor::handle_doctor;

/// CacheKill - A production-ready CLI tool to safely nuke development and build caches
#[derive(Parser)]
#[command(name = "cachekill")]
    #[command(version = "0.1.2")] // Version bump with NPX improvements
#[command(about = "Safely nuke development and build caches")]
#[command(long_about = r#"
CacheKill is a production-ready CLI tool that helps you safely clean up
development and build caches. It supports multiple languages and frameworks,
provides safe deletion with backup functionality, and offers detailed insights
into your cache usage.

Examples:
  cachekill                    # Clean detected caches with confirmation
  cachekill --dry-run          # Show what would be cleaned
  cachekill --list             # List all cache entries with details
  cachekill --lang js --force  # Clean JavaScript caches without confirmation
  cachekill --docker           # Include Docker cleanup
  cachekill --npx --list       # List NPX cache contents
  cachekill --ci prebuild      # CI mode for prebuild
  cachekill --hf --list        # List HuggingFace cache
  cachekill --torch            # Clean PyTorch cache
  cachekill --vercel --list    # Check Vercel integration status
  cachekill --cloudflare       # Purge Cloudflare edge cache
  cachekill --doctor           # System diagnostics
"#)]
struct Cli {
    /// List cache entries with size, last-used, and stale information
    #[arg(long)]
    list: bool,

    /// Show what would be removed without actually doing it
    #[arg(long)]
    dry_run: bool,

    /// Proceed without interactive prompt
    #[arg(short = 'f', long)]
    force: bool,

    /// Alias for --force
    #[arg(short = 'y', long)]
    yes: bool,

    /// Output in JSON format
    #[arg(long)]
    json: bool,

    /// Language filter (auto, js, py, rust, java, ml)
    #[arg(long, value_name = "LANG")]
    lang: Option<String>,

    /// Additional include paths (glob patterns)
    #[arg(long, value_name = "PATTERNS")]
    paths: Option<String>,

    /// Additional exclude paths (glob patterns)
    #[arg(long, value_name = "PATTERNS")]
    exclude: Option<String>,

    /// Days threshold for marking caches as stale
    #[arg(long, value_name = "DAYS")]
    stale_days: Option<u32>,

    /// Enable safe delete (move to backup before deletion)
    #[arg(long)]
    safe_delete: Option<bool>,

    /// Backup directory for safe delete
    #[arg(long, value_name = "PATH")]
    backup_dir: Option<String>,

    /// Include Docker cleanup
    #[arg(long)]
    docker: bool,

    /// Include NPX cache cleanup
    #[arg(long)]
    npx: bool,

    /// Restore from last backup
    #[arg(long)]
    restore_last: bool,

    /// Clean all common caches regardless of project type
    #[arg(long)]
    all: bool,

    /// System diagnostics
    #[arg(long)]
    doctor: bool,

    /// CI mode for non-interactive cache management
    #[arg(long, value_name = "MODE")]
    ci: Option<String>,

    /// HuggingFace cache operations
    #[arg(long)]
    hf: bool,

    /// PyTorch cache operations
    #[arg(long)]
    torch: bool,

    /// Vercel edge cache operations
    #[arg(long)]
    vercel: bool,

    /// Cloudflare edge cache operations
    #[arg(long)]
    cloudflare: bool,

    /// Target specific model ID for HuggingFace
    #[arg(long, value_name = "MODEL_ID")]
    model: Option<String>,

    /// Target specific project ID for Vercel
    #[arg(long, value_name = "PROJECT_ID")]
    project: Option<String>,

    /// Target specific zone ID for Cloudflare
    #[arg(long, value_name = "ZONE_ID")]
    zone: Option<String>,

    /// API token for edge cache purging
    #[arg(long, value_name = "TOKEN")]
    token: Option<String>,
}


impl Cli {
    /// Convert CLI arguments to CliArgs struct
    fn to_cli_args(&self) -> CliArgs {
        CliArgs {
            list: self.list,
            dry_run: self.dry_run,
            force: self.force || self.yes,
            json: self.json,
            lang: self.lang.as_ref().and_then(|s| s.parse().ok()),
            paths: self.paths.as_ref().map(|s| s.split(',').map(|s| s.trim().to_string()).collect()),
            exclude: self.exclude.as_ref().map(|s| s.split(',').map(|s| s.trim().to_string()).collect()),
            stale_days: self.stale_days,
            safe_delete: self.safe_delete,
            backup_dir: self.backup_dir.clone(),
            docker: self.docker,
            npx: self.npx,
            restore_last: self.restore_last,
            all: self.all,
        }
    }
}

fn main() {
    let cli = Cli::parse();


    // Run the main application
    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    // Load configuration
    let config = Config::load().unwrap_or_else(|_| Config::default());
    let cli_args = cli.to_cli_args();
    let merged_config = config.merge_with_cli(&cli_args);

    // Create output formatter
    let formatter = OutputFormatter::new(merged_config.json);

    // Handle restore last backup
    if merged_config.restore_last {
        return handle_restore_last(&merged_config, &formatter);
    }

    // Handle doctor command
    if cli.doctor {
        return handle_doctor(&merged_config);
    }

    // Handle CI mode
    if let Some(mode) = cli.ci {
        let ci_mode = match mode.as_str() {
            "prebuild" => CiMode::Prebuild,
            "postbuild" => CiMode::Postbuild,
            _ => {
                eprintln!("Invalid CI mode: {}. Use 'prebuild' or 'postbuild'", mode);
                process::exit(4);
            }
        };
        return handle_ci_mode(&merged_config, ci_mode);
    }

    // Handle specialized integrations
    if cli.hf {
        if merged_config.list {
            return handle_hf_list(&merged_config);
        } else {
            return handle_hf_clean(&merged_config, cli.model.as_deref());
        }
    }

    if cli.torch {
        if merged_config.list {
            return handle_torch_list(&merged_config);
        } else {
            return handle_torch_clean(&merged_config);
        }
    }

    if cli.vercel {
        if merged_config.list {
            return handle_vercel_status(&merged_config);
        } else {
            return handle_vercel_purge(&merged_config, cli.project.as_deref());
        }
    }

    if cli.cloudflare {
        if merged_config.list {
            return handle_cloudflare_status(&merged_config);
        } else {
            return handle_cloudflare_purge(&merged_config, cli.zone.as_deref());
        }
    }

    // Handle NPX list mode (must be before general list mode)
    if merged_config.npx && merged_config.list {
        return handle_npx_list_mode(&merged_config, &formatter);
    }

    // Handle Docker list mode (must be before general list mode)
    if merged_config.docker && merged_config.list {
        return handle_docker_list_mode(&merged_config, &formatter);
    }

    // Handle list mode
    if merged_config.list {
        return handle_list_mode(&merged_config, &formatter);
    }

    // Handle dry run mode
    if merged_config.dry_run {
        return handle_dry_run_mode(&merged_config, &formatter);
    }

    // Handle normal cleanup mode
    handle_cleanup_mode(&merged_config, &formatter)
}

fn handle_restore_last(config: &MergedConfig, formatter: &OutputFormatter) -> Result<()> {
    let executor = ActionExecutor::new(config.clone());
    
    match executor.restore_last_backup() {
        Ok(result) => {
            if let Err(e) = formatter.print_restore_result(&result) {
                eprintln!("Error printing restore result: {}", e);
            }
            if result.failed.is_empty() {
                println!("✅ Successfully restored from backup");
                process::exit(0);
            } else {
                println!("⚠️  Restore completed with some failures");
                process::exit(2);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to restore from backup: {}", e);
            process::exit(3);
        }
    }
}

fn handle_list_mode(config: &MergedConfig, formatter: &OutputFormatter) -> Result<()> {
    // Discover cache entries
    let discovery = DiscoveryResult::discover(config)?;
    
    if discovery.cache_entries.is_empty() {
        if !config.json {
            println!("No cache entries found.");
        }
        return Ok(());
    }

    // Inspect cache entries
    let inspector = CacheInspector::new(config.clone());
    let entries = inspector.inspect_caches(&discovery.cache_entries)?;

    // Print cache table
    if let Err(e) = formatter.print_cache_table(&entries) {
        eprintln!("Error printing cache table: {}", e);
    }

    // Print summary
    let summary = inspector.get_summary(&entries);
    if let Err(e) = formatter.print_summary(&summary) {
        eprintln!("Error printing summary: {}", e);
    }

    Ok(())
}

fn handle_npx_list_mode(config: &MergedConfig, formatter: &OutputFormatter) -> Result<()> {
    let npx_manager = NpxCacheManager::new(config.clone());
    
    if !npx::is_npx_available() {
        if !config.json {
            println!("NPX is not available on this system.");
        }
        return Ok(());
    }

    // Use per-package functionality for detailed NPX cache analysis
    if let Ok(packages) = npx_manager.list_packages() {
        if config.json {
            println!("{}", serde_json::to_string_pretty(&packages)?);
        } else {
            println!("📦 NPX Package Cache Analysis");
            println!("Found {} cached packages:", packages.len());
            println!();
            
            if packages.is_empty() {
                println!("No NPX packages found in cache.");
                return Ok(());
            }
            
            println!("{:<30} | {:<15} | {:<12} | {:<15} | {:<8}", 
                "Package", "Version", "Size", "Last Used", "Stale?");
            println!("{:-<30} | {:-<15} | {:-<12} | {:-<15} | {:-<8}", 
                "", "", "", "", "");
            
            for package in &packages {
                let version = package.version.as_deref().unwrap_or("unknown");
                let size = humansize::format_size(package.size_bytes, humansize::DECIMAL);
                let last_used = package.last_used.format("%Y-%m-%d %H:%M").to_string();
                let stale = if package.stale { "Yes" } else { "No" };
                
                println!("{:<30} | {:<15} | {:<12} | {:<15} | {:<8}", 
                    package.name, version, size, last_used, stale);
            }
            
            let total_size: u64 = packages.iter().map(|p| p.size_bytes).sum();
            let stale_count = packages.iter().filter(|p| p.stale).count();
            
            println!();
            println!("📊 Summary:");
            println!("  Total packages: {}", packages.len());
            println!("  Total size: {}", humansize::format_size(total_size, humansize::DECIMAL));
            println!("  Stale packages: {}", stale_count);
        }
    } else {
        // Fallback to basic stats if per-package analysis fails
        let stats = npx_manager.get_npx_stats()?;
        if let Err(e) = formatter.print_npx_info(&stats) {
            eprintln!("Error printing NPX info: {}", e);
        }
    }

    Ok(())
}

fn handle_docker_list_mode(config: &MergedConfig, formatter: &OutputFormatter) -> Result<()> {
    let docker_manager = DockerCacheManager::new(config.clone());
    
    if !DockerCacheManager::is_docker_available() {
        if !config.json {
            println!("Docker is not available on this system.");
        }
        return Ok(());
    }

    let stats = docker_manager.get_docker_stats()?;
    if let Err(e) = formatter.print_docker_info(&stats) {
        eprintln!("Error printing Docker info: {}", e);
    }

    Ok(())
}

fn handle_dry_run_mode(config: &MergedConfig, formatter: &OutputFormatter) -> Result<()> {
    // Discover and inspect cache entries
    let discovery = DiscoveryResult::discover(config)?;
    let inspector = CacheInspector::new(config.clone());
    let entries = inspector.inspect_caches(&discovery.cache_entries)?;

    // Add NPX cache if requested
    let mut all_entries = entries;
    if config.npx {
        let npx_manager = NpxCacheManager::new(config.clone());
        if let Ok(npx_entries) = npx_manager.list_npx_cache() {
            all_entries.extend(npx_entries);
        }
    }

    // Add Docker cache if requested
    if config.docker {
        let docker_manager = DockerCacheManager::new(config.clone());
        if let Ok(docker_entries) = docker_manager.list_docker_cache() {
            all_entries.extend(docker_entries);
        }
    }

    // Execute dry run
    let executor = ActionExecutor::new(config.clone());
    let result = executor.dry_run(&all_entries)?;

    // Print results
    if let Err(e) = formatter.print_dry_run(&result) {
        eprintln!("Error printing dry run results: {}", e);
    }

    Ok(())
}

fn handle_cleanup_mode(config: &MergedConfig, formatter: &OutputFormatter) -> Result<()> {
    // Discover and inspect cache entries
    let discovery = DiscoveryResult::discover(config)?;
    let inspector = CacheInspector::new(config.clone());
    let mut entries = inspector.inspect_caches(&discovery.cache_entries)?;

    // Add NPX cache if requested
    if config.npx {
        let npx_manager = NpxCacheManager::new(config.clone());
        if let Ok(npx_entries) = npx_manager.list_npx_cache() {
            entries.extend(npx_entries);
        }
    }

    // Add Docker cache if requested
    if config.docker {
        let docker_manager = DockerCacheManager::new(config.clone());
        if let Ok(docker_entries) = docker_manager.list_docker_cache() {
            entries.extend(docker_entries);
        }
    }

    if entries.is_empty() {
        if !config.json {
            println!("No cache entries found to clean.");
        }
        return Ok(());
    }

    // Show summary
    let summary = inspector.get_summary(&entries);
    if !config.json {
        println!("🔍 Found {} cache entries ({} total)", 
                 summary.total_count, 
                 summary.total_size_human());
        
        let largest = inspector.get_largest_entries(&entries, 5);
        if !largest.is_empty() {
            println!("\n📊 Top 5 largest caches:");
            for (i, entry) in largest.iter().enumerate() {
                println!("  {}. {} ({})", 
                         i + 1, 
                         entry.path.display(), 
                         entry.size_human());
            }
        }
    }

    // Ask for confirmation unless forced
    if !config.force {
        let action = if config.safe_delete { "SAFE DELETE (move to backup)" } else { "DELETE" };
        let prompt = format!("Proceed with {}? (y/N)", action);
        
        match inquire::Confirm::new(&prompt).prompt() {
            Ok(true) => {},
            Ok(false) => {
                if !config.json {
                    println!("Operation cancelled.");
                }
                return Ok(());
            }
            Err(_) => {
                if !config.json {
                    println!("Operation cancelled.");
                }
                return Ok(());
            }
        }
    }

    // Execute cleanup
    let executor = ActionExecutor::new(config.clone());
    
    if config.safe_delete {
        // Safe delete (move to backup)
        let result = executor.safe_delete(&entries)?;
        if let Err(e) = formatter.print_safe_delete_result(&result) {
            eprintln!("Error printing safe delete results: {}", e);
        }
        
        if result.failed.is_empty() {
            if !config.json {
                println!("✅ Successfully moved {} entries to backup", result.backed_up.len());
            }
            process::exit(0);
        } else {
            if !config.json {
                println!("⚠️  Cleanup completed with some failures");
            }
            process::exit(2);
        }
    } else {
        // Hard delete
        let result = executor.hard_delete(&entries)?;
        if let Err(e) = formatter.print_hard_delete_result(&result) {
            eprintln!("Error printing hard delete results: {}", e);
        }
        
        if result.failed.is_empty() {
            if !config.json {
                println!("✅ Successfully deleted {} entries", result.deleted.len());
            }
            process::exit(0);
        } else {
            if !config.json {
                println!("⚠️  Cleanup completed with some failures");
            }
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let args = vec!["cachekill", "--list", "--json"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(cli.list);
        assert!(cli.json);
    }

    #[test]
    fn test_cli_args_conversion() {
        let cli = Cli {
            list: true,
            dry_run: false,
            force: false,
            yes: false,
            json: true,
            lang: Some("js".to_string()),
            paths: Some("**/node_modules".to_string()),
            exclude: None,
            stale_days: Some(7),
            safe_delete: Some(true),
            backup_dir: Some(".backup".to_string()),
            docker: false,
            npx: false,
            restore_last: false,
            all: false,
            doctor: false,
            ci: None,
            hf: false,
            torch: false,
            vercel: false,
            cloudflare: false,
            model: None,
            project: None,
            zone: None,
            token: None,
        };

        let cli_args = cli.to_cli_args();
        assert!(cli_args.list);
        assert!(cli_args.json);
        use crate::cache_entry::LanguageFilter;
        assert_eq!(cli_args.lang, Some(LanguageFilter::JavaScript));
        assert_eq!(cli_args.stale_days, Some(7));
    }
}
