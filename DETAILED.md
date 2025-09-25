# CacheKill - Detailed Documentation 📚

A guide to the CacheKill CLI tool for safely managing development and build caches.

## Table of Contents

1. [Overview](#overview)
2. [Installation](#installation)
3. [Architecture](#architecture)
4. [Core Concepts](#core-concepts)
5. [CLI Reference](#cli-reference)
6. [Configuration](#configuration)
7. [Cache Detection](#cache-detection)
8. [Safety Features](#safety-features)
9. [Testing Guide](#testing-guide)
10. [Development](#development)
11. [Troubleshooting](#troubleshooting)
12. [API Reference](#api-reference)

## Overview

CacheKill is a CLI tool designed to safely clean up development and build caches across multiple languages and frameworks. It provides intelligent cache detection, safe deletion with backup functionality, and detailed insights into cache usage.

### Key Features

- **Multi-language Support**: JavaScript, Python, Rust, Java, Machine Learning
- **Smart Detection**: Automatic project type detection and relevant cache identification
- **Safe Operations**: Timestamped backups with restore capability
- **Flexible Output**: Human-readable tables and machine-readable JSON
- **Cross-platform**: macOS, Linux, Windows support
- **Configurable**: TOML configuration file support
- **Advanced NPX Analysis**: Per-package visibility with detailed breakdown
- **Enhanced Edge Purging**: Improved API integration for Vercel and Cloudflare
- **Integration**: Docker and NPX cache management

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/kagehq/cachekill.git
cd cachekill

# Build in release mode
cargo build --release

# The binary will be available at ./target/release/cachekill
```

### Using Cargo

```bash
# Install from crates.io (when published)
cargo install cachekill

# Install from local source
cargo install --path .
```

### Binary Distribution

```bash
# Download pre-built binaries (when available)
# Check releases page for your platform
```

### Data Flow

1. **CLI Parsing**: Arguments parsed and validated
2. **Configuration**: Load `.cachekillrc` and merge with CLI args
3. **Discovery**: Detect project type and find cache candidates
4. **Inspection**: Analyze cache sizes, modification times, staleness
5. **Planning**: Determine actions (delete, backup, skip)
6. **Execution**: Perform operations with safety checks
7. **Output**: Format and display results

## Core Concepts

### Cache Entry

A `CacheEntry` represents a discovered cache with metadata:

```rust
pub struct CacheEntry {
    pub path: PathBuf,           // Path to cache
    pub kind: CacheKind,         // Type of cache
    pub size_bytes: u64,         // Size in bytes
    pub last_used: DateTime<Utc>, // Last modification time
    pub stale: bool,             // Whether cache is stale
    pub planned_action: Option<PlannedAction>, // What to do with it
}
```

### Cache Kinds

- **JavaScript**: `node_modules/`, `.next/`, `.vite/`, etc.
- **Python**: `__pycache__/`, `.pytest_cache/`, `.venv/`, etc.
- **Rust**: `target/`, `.cargo/`
- **Java**: `.gradle/`, `build/`, `~/.m2/repository`
- **Machine Learning**: `~/.cache/huggingface`, `~/.cache/torch`
- **NPX**: `~/.npm/_npx`
- **Docker**: Images, containers, volumes, build cache
- **Generic**: `tmp/`, `temp/`, `.cache/`

### Project Types

- **JavaScript**: Detected by `package.json`
- **Python**: Detected by `requirements.txt`, `pyproject.toml`
- **Rust**: Detected by `Cargo.toml`
- **Java**: Detected by `build.gradle`, `pom.xml`
- **Machine Learning**: Detected by ML-specific files
- **Mixed**: Multiple project types detected
- **Unknown**: No clear project type

## CLI Reference

### Global Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--list` | | List cache entries with details | |
| `--dry-run` | | Show what would be cleaned | |
| `--force` | `-f` | Proceed without confirmation | |
| `--yes` | `-y` | Alias for --force | |
| `--json` | | Output in JSON format | |
| `--lang <LANG>` | | Language filter | `auto` |
| `--paths <PATTERNS>` | | Include patterns (glob) | |
| `--exclude <PATTERNS>` | | Exclude patterns (glob) | |
| `--stale-days <DAYS>` | | Stale threshold in days | `14` |
| `--safe-delete <BOOL>` | | Enable safe delete | `true` |
| `--backup-dir <PATH>` | | Backup directory | `.cachekill-backup` |
| `--docker` | | Include Docker cleanup | |
| `--npx` | | Include NPX cache cleanup and per-package analysis | |
| `--restore-last` | | Restore from last backup | |
| `--all` | | Clean all common caches | |
| `--help` | `-h` | Show help | |
| `--version` | `-V` | Show version | |

### Language Filters

- `auto`: Auto-detect project type
- `js`: JavaScript/TypeScript only
- `py`: Python only
- `rust`: Rust only
- `java`: Java/Kotlin only
- `ml`: Machine Learning only

### Examples

```bash
# Basic usage
./target/release/cachekill

# List all caches
./target/release/cachekill --list

# Dry run with specific language
./target/release/cachekill --lang js --dry-run

# Include Docker and NPX
./target/release/cachekill --docker --npx --dry-run

# Advanced NPX analysis with per-package details
./target/release/cachekill --npx --list

# NPX analysis with JSON output for automation
./target/release/cachekill --npx --list --json

# Custom paths and exclusions
./target/release/cachekill --paths "**/custom-cache" --exclude "**/test" --dry-run

# JSON output for scripting
./target/release/cachekill --list --json | jq .

# Force mode without confirmation
./target/release/cachekill --force

# Restore from backup
./target/release/cachekill --restore-last
```

## Configuration

### Configuration File

Create a `.cachekillrc` file in your project root:

```toml
# Default language filter
default_lang = "auto"

# Stale threshold in days
stale_days = 14

# Enable safe delete by default
safe_delete = true

# Backup directory (relative to project root)
backup_dir = ".cachekill-backup"

# Additional include patterns
include_paths = [
    "**/custom-cache",
    "**/build-artifacts"
]

# Exclude patterns
exclude_paths = [
    ".git",
    ".cachekill-backup",
    "node_modules/.cache",
    "**/test-results",
    "**/coverage"
]

# Include Docker cleanup by default
include_docker = false

# Include NPX cache cleanup by default
include_npx = false
```

### Configuration Precedence

1. CLI arguments (highest priority)
2. Configuration file (`.cachekillrc`)
3. Default values (lowest priority)

### Environment Variables

- `CACHEKILL_CONFIG`: Path to configuration file
- `CACHEKILL_BACKUP_DIR`: Default backup directory
- `CACHEKILL_STALE_DAYS`: Default stale threshold

## Cache Detection

### Detection Algorithm

1. **Project Type Detection**:
   - Scan for project files (`package.json`, `Cargo.toml`, etc.)
   - Determine primary project type
   - Handle mixed projects

2. **Cache Discovery**:
   - Use project type to determine relevant cache patterns
   - Scan for common cache directories
   - Apply include/exclude patterns

3. **Cache Analysis**:
   - Calculate directory sizes
   - Determine last modification time
   - Check staleness based on threshold

### Supported Cache Patterns

#### JavaScript/TypeScript
- `node_modules/`
- `.next/`, `.nuxt/`
- `.vite/`, `.cache/`
- `dist/`, `coverage/`
- `.turbo/`, `.parcel-cache/`
- `build/`, `out/`

#### Python
- `__pycache__/`
- `.pytest_cache/`
- `.venv/`, `venv/`
- `.tox/`, `.mypy_cache/`
- `.ruff_cache/`, `.pip-cache/`
- `.coverage/`

#### Rust
- `target/`
- `.cargo/`
- `target/debug/`, `target/release/`

#### Java
- `.gradle/`
- `build/`
- `~/.m2/repository`
- `target/` (Maven)

#### Machine Learning
- `~/.cache/huggingface`
- `~/.cache/torch`
- `.dvc/cache`
- `~/.cache/transformers`

#### Generic
- `tmp/`, `temp/`
- `.cache/`
- `build/`, `dist/`

#### NPX
- `~/.npm/_npx`
- Platform-specific NPX cache locations

#### Docker
- Images, containers, volumes, build cache
- Managed via `docker system prune`

## Advanced NPX Analysis

CacheKill provides comprehensive per-package analysis for NPX caches, offering detailed insights into package usage and optimization opportunities.

### NPX Per-Package Analysis

The `--npx --list` command provides detailed analysis of NPX cached packages:

```bash
# Analyze NPX packages with detailed breakdown
./target/release/cachekill --npx --list

# JSON output for automation
./target/release/cachekill --npx --list --json
```

### Analysis Features

#### Package Details
- **Name**: Package name extracted from `package.json`
- **Version**: Package version (when available)
- **Size**: Disk usage in human-readable format
- **Last Used**: Modification timestamp
- **Stale Status**: Based on configurable threshold (default: 14 days)

#### Output Format
```
📦 NPX Package Cache Analysis
Found 791 cached packages:

Package                        | Version         | Size         | Last Used       | Stale?  
------------------------------ | --------------- | ------------ | --------------- | --------
prisma                         | unknown         | 85.59 MB     | 2025-02-03 00:13 | Yes     
@nestjs/cli                    | unknown         | 60.86 MB     | 2025-08-02 13:43 | Yes     
typescript                     | 5.9.2           | 23.62 MB     | 2025-09-12 20:19 | No      
```

#### Summary Statistics
- **Total packages**: Count of cached packages
- **Total size**: Combined disk usage
- **Stale packages**: Count of packages exceeding stale threshold
- **Size optimization**: Identify largest packages for cleanup

### NPX Cache Structure

NPX caches are stored in platform-specific locations:
- **macOS/Linux**: `~/.npm/_npx/`
- **Windows**: `%APPDATA%\npm-cache\_npx\`

Each package is stored in a hash-named directory containing:
- `package.json`: Package metadata
- `node_modules/`: Package dependencies
- `package-lock.json`: Lock file (when available)

### Stale Detection

Packages are marked as stale based on:
- **Last modification time**: When the package was last accessed
- **Configurable threshold**: Default 14 days, configurable via `--stale-days`
- **Size consideration**: Large packages may have different thresholds

### JSON Output

For automation and scripting, use `--json` flag:

```json
[
  {
    "name": "prisma",
    "version": null,
    "size_bytes": 89738240,
    "last_used": "2025-02-03T00:13:00Z",
    "path": "/Users/user/.npm/_npx/1d6e82a4126006c4",
    "stale": true
  }
]
```

### Optimization Recommendations

Based on analysis results:
1. **Remove stale packages**: Clean packages not used recently
2. **Size optimization**: Focus on largest packages first
3. **Version management**: Remove duplicate versions of same package
4. **Regular cleanup**: Schedule periodic NPX cache maintenance

## Safety Features

### Safe Delete (Default)

When `--safe-delete` is enabled (default):

1. **Backup Creation**: Move caches to timestamped backup directory
2. **Backup Structure**: `{backup_dir}/{timestamp}/`
3. **Restore Capability**: Use `--restore-last` to restore from backup
4. **Backup Cleanup**: Old backups can be automatically cleaned

### Stale Detection

- **Threshold**: Configurable via `--stale-days` (default: 14)
- **Calculation**: Based on last modification time
- **Purpose**: Avoid deleting recently used caches
- **Override**: Use `--force` to ignore staleness

### Project Boundary Safety

- **Symlink Protection**: Don't follow symlinks outside project
- **Path Validation**: Ensure operations stay within project bounds
- **Permission Checks**: Verify write permissions before operations

### Backup Management

```bash
# Backup directory structure
.cachekill-backup/
├── 2024-01-15_14-30-25/
│   ├── node_modules/
│   └── target/
├── 2024-01-16_09-15-10/
│   └── .next/
└── metadata.json
```

## Testing Guide

### Unit Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test cache_entry

# Run tests with output
cargo test -- --nocapture
```

### Integration Tests

```bash
# Test basic functionality
./target/release/cachekill --help
./target/release/cachekill --version
./target/release/cachekill --list

# Test dry run
./target/release/cachekill --dry-run

# Test JSON output
./target/release/cachekill --list --json | jq .

# Test language filters
./target/release/cachekill --lang js --dry-run
./target/release/cachekill --lang rust --dry-run
```

### Performance Tests

```bash
# Test large directory scanning
./target/release/cachekill --all --dry-run

# Test memory usage
time ./target/release/cachekill --list

# Test JSON performance
./target/release/cachekill --list --json > /dev/null
```

### Error Handling Tests

```bash
# Test invalid flags
./target/release/cachekill --invalid-flag

# Test invalid language
./target/release/cachekill --lang invalid --dry-run

# Test invalid paths
./target/release/cachekill --paths "invalid[pattern" --dry-run
```

## Development

### Development Setup

```bash
# Clone and build
git clone https://github.com/kagehq/cachekill.git
cd cachekill
cargo build --release
```

### Development Commands

```bash
# Build
make build
# or
cargo build --release

# Test
make test
# or
cargo test

# Format
make fmt
# or
cargo fmt

# Lint
make clippy
# or
cargo clippy -- -D warnings

# Run
make run
# or
./target/release/cachekill --help
```

### Code Structure

#### Main Entry Point (`main.rs`)
- CLI argument parsing
- Command dispatch
- Error handling
- Exit codes

#### Cache Entry (`cache_entry.rs`)
- `CacheEntry` struct
- `CacheKind` enum
- `LanguageFilter` enum
- `PlannedAction` enum

#### Configuration (`config.rs`)
- `Config` struct (file-based)
- `CliArgs` struct (CLI arguments)
- `MergedConfig` struct (merged configuration)

#### Discovery (`discover.rs`)
- `ProjectType` enum
- `DiscoveryResult` struct
- Project type detection
- Cache candidate discovery

#### Inspection (`inspect.rs`)
- `CacheInspector` struct
- Size calculation
- Staleness detection
- Cache analysis

#### Actions (`actions.rs`)
- `ActionExecutor` struct
- Dry run simulation
- Safe delete operations
- Backup management
- Restore functionality

#### Output (`output.rs`)
- `OutputFormatter` struct
- Table formatting
- JSON serialization
- Summary generation

#### NPX Management (`npx.rs`)
- `NpxCacheManager` struct
- NPX cache detection
- NPX cache operations

#### Docker Management (`docker.rs`)
- `DockerCacheManager` struct
- Docker system analysis
- Docker cleanup operations

#### Utilities (`util.rs`)
- Path utilities
- File system operations
- Time handling
- Backup management

### Adding New Features

1. **New Cache Type**:
   - Add to `CacheKind` enum
   - Update detection patterns
   - Add to project type mapping

2. **New Project Type**:
   - Add to `ProjectType` enum
   - Update detection logic
   - Add cache kind mapping

3. **New CLI Option**:
   - Add to `Cli` struct
   - Update `CliArgs` struct
   - Update `MergedConfig` struct
   - Update help text

### Testing New Features

```bash
# Add unit tests
# Add integration tests
# Update documentation
# Test cross-platform compatibility
```

## Troubleshooting

### Common Issues

#### "No cache entries found"
- **Cause**: No caches detected in current directory
- **Solution**: 
  - Check if you're in the right directory
  - Try `--all` to include generic caches
  - Use `--list` to see what's detected
  - Check include/exclude patterns

#### "Permission denied"
- **Cause**: Insufficient permissions for file operations
- **Solution**:
  - Run with appropriate permissions
  - Check file ownership
  - Use `--force` to skip interactive prompts

#### "Docker not available"
- **Cause**: Docker CLI not found or not running
- **Solution**:
  - Install Docker CLI
  - Check if Docker is running
  - Use `--docker` only when needed

#### "Configuration error"
- **Cause**: Invalid `.cachekillrc` file
- **Solution**:
  - Check TOML syntax
  - Validate configuration values
  - Use `--help` to see valid options

### Debug Mode

```bash
# Enable debug output
RUST_LOG=debug ./target/release/cachekill --list

# Verbose output
./target/release/cachekill --list --json | jq .
```

### Log Files

CacheKill doesn't create log files by default. For debugging:

```bash
# Redirect output to file
./target/release/cachekill --list > cachekill.log 2>&1

# Use system logging
./target/release/cachekill --list 2>&1 | tee cachekill.log
```

## API Reference

### Exit Codes

- `0`: Success
- `2`: Partial success (some operations failed)
- `3`: Nothing to do
- `4`: Configuration error
- `5`: Fatal error

### JSON Output Schema

```json
{
  "mode": "list|dry-run|delete|restore",
  "entries": [
    {
      "path": "string",
      "kind": "js|py|rust|java|ml|npx|docker|generic",
      "size_bytes": "number",
      "last_used": "ISO8601 datetime",
      "stale": "boolean",
      "planned_action": "delete|backup|skip"
    }
  ],
  "totals": {
    "size_bytes": "number",
    "count": "number",
    "freed_bytes": "number"
  }
}
```

### Configuration Schema

```toml
# .cachekillrc
default_lang = "auto|js|py|rust|java|ml"
stale_days = "number"
safe_delete = "boolean"
backup_dir = "string"
include_paths = ["string"]
exclude_paths = ["string"]
include_docker = "boolean"
include_npx = "boolean"
```

### Environment Variables

- `CACHEKILL_CONFIG`: Path to configuration file
- `CACHEKILL_BACKUP_DIR`: Default backup directory
- `CACHEKILL_STALE_DAYS`: Default stale threshold
- `RUST_LOG`: Log level for debugging

### Performance Characteristics

- **Memory Usage**: ~10-50MB depending on cache size
- **Scan Speed**: ~100-1000 directories/second
- **JSON Output**: ~1-10MB for large projects
- **Backup Size**: Same as original cache size

### Limitations

- **Symlinks**: Limited support for complex symlink structures
- **Permissions**: Requires appropriate file system permissions
- **Large Files**: May be slow with very large cache directories
- **Network**: No support for remote cache management

### Future Enhancements

- **Remote Caches**: Support for cloud-based caches
- **Cache Analysis**: Detailed cache usage analytics

---

For more information, see the [README.md](README.md) for quick start instructions.
