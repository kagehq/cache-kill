# Cache Kill

A CLI tool to safely nuke development and build caches across multiple languages and frameworks.

## Community & Support

Join our Discord community for discussions, support, and updates:

[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-7289DA?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/KqdBcqRk5E)

## Features
- **One command** to safely clean dev/build caches
- **Smart detection** of project types and relevant caches
- **Safe operations** with timestamped backups
- **Cross-platform** (macOS, Linux, Windows)
- **JSON output** for scripting and automation
- **CI/CD integration** with GitHub Actions, GitLab CI, and CircleCI
- **Specialized integrations** for HuggingFace, PyTorch, Vercel, and Cloudflare
- **Advanced NPX analysis** with per-package visibility and stale detection
- **JavaScript package managers**: npm, pnpm, yarn global and project caches (opt-in via `--js-pm`)
- **Enhanced edge cache purging** with improved API integration
- **System diagnostics** with `--doctor` command
- **MCP Server** for AI assistant integration via Model Context Protocol

## Installation

### From Source
```bash
git clone https://github.com/kagehq/cachekill.git
cd cachekill
cargo build --release
```

### Using Cargo
```bash
cargo install cachekill
```

## Quick Start

```bash
# List all caches
./target/release/cachekill --list

# Show what would be cleaned
./target/release/cachekill --dry-run

# Clean with confirmation
./target/release/cachekill

# Clean specific language
./target/release/cachekill --lang js

# Include Docker and NPX
./target/release/cachekill --docker --npx

# JSON output for scripting
./target/release/cachekill --list --json

# System diagnostics
./target/release/cachekill --doctor

# CI mode for automation
./target/release/cachekill --ci prebuild
./target/release/cachekill --ci postbuild

# Specialized integrations
./target/release/cachekill --hf --list
./target/release/cachekill --torch
./target/release/cachekill --vercel --list
./target/release/cachekill --cloudflare

# Advanced NPX analysis with per-package details
./target/release/cachekill --npx --list

# NPX cache management
./target/release/cachekill --npx --dry-run          # Preview what would be cleaned
./target/release/cachekill --npx --force            # Nuclear option - clear all NPX caches
./target/release/cachekill --npx --stale-days 7 --force  # Surgical - only stale packages
```

### JavaScript Package Managers usage
```bash
# Include JavaScript package manager caches (npm, pnpm, yarn)
./target/release/cachekill --list --js-pm

# JSON output including JS PM caches
./target/release/cachekill --list --json --js-pm

# Dry run including JS PM caches
./target/release/cachekill --dry-run --js-pm
```

## MCP Server

CacheKill includes an MCP server that allows AI assistants to interact with cache management tools programmatically.

### Running the MCP Server

```bash
# Build the MCP server
cargo build --release --bin mcp

# Run the MCP server
./target/release/mcp
```

### Available MCP Tools

The MCP server provides the following tools for AI assistants:

- **`list_caches`** - List all cache entries with details
- **`clean_caches`** - Clean cache entries with various options
- **`dry_run`** - Show what would be cleaned without doing it
- **`npx_analysis`** - Analyze NPX cache with per-package details
- **`docker_stats`** - Get Docker cache statistics
- **`system_diagnostics`** - Run system diagnostics
- **`restore_backup`** - Restore from last backup

### MCP Server Configuration

The MCP server accepts the same configuration options as the CLI tool:

```json
{
  "lang": "js",
  "force": true,
  "safe_delete": true,
  "docker": true,
  "npx": true
}
```

### MCP Server Implementation

The MCP server is a simple wrapper around the main `cachekill` binary that provides JSON output for AI assistants. It delegates all operations to the main CLI tool, ensuring consistency and reliability.

## Supported Languages

- **JavaScript/TypeScript**: `node_modules/`, `.next/`, `.vite/`
- **Python**: `__pycache__/`, `.venv/`, `.pytest_cache/`
- **Rust**: `target/`, `.cargo/`
- **Java**: `.gradle/`, `build/`, `~/.m2/repository`
- **Machine Learning**: `~/.cache/huggingface`, `~/.cache/torch`
- **JavaScript package managers**: npm (`~/.npm` or `%LOCALAPPDATA%\npm-cache`), pnpm (store + meta caches), yarn (global + project `.yarn/cache`)
- **NPX**: `~/.npm/_npx`
- **Docker**: Images, containers, volumes

## Specialized Integrations

- **HuggingFace**: Model caches, datasets, and repositories with detailed analysis
- **PyTorch**: Checkpoints, hub models, and datasets with version tracking
- **NPX**: Per-package analysis with name, version, size, and stale detection
- **Vercel**: Enhanced edge cache purging with improved API integration
- **Cloudflare**: Enhanced edge cache purging with zone-specific targeting

## Advanced NPX Analysis

CacheKill provides detailed per-package analysis for NPX caches:

```bash
# Analyze NPX packages with detailed breakdown
./target/release/cachekill --npx --list

# Output shows:
# - Package name and version
# - Size and last-used timestamp
# - Stale detection (configurable threshold)
# - Sorted by size (largest first)
# - Summary statistics
```

## Configuration

Create a `.cachekillrc` file in your project root:

```toml
default_lang = "auto"
stale_days = 14
safe_delete = true
backup_dir = ".cachekill-backup"
exclude_paths = [".git", ".cachekill-backup"]
```

## CI/CD Integration

### GitHub Actions
```yaml
- uses: ./.github/actions/cachekill
  with:
    mode: 'postbuild'
    args: '--docker --npx'
  env:
    VERCEL_TOKEN: ${{ secrets.VERCEL_TOKEN }}
    CF_API_TOKEN: ${{ secrets.CF_API_TOKEN }}
```

### GitLab CI
```yaml
include:
  - local: 'ci/gitlab-cachekill.yml'

variables:
  CACHEKILL_ARGS: "--docker --npx"
```

### CircleCI
```yaml
orbs:
  cachekill: cachekill/cachekill@1.0.0

workflows:
  build:
    jobs:
      - cachekill/prebuild
      - build
      - cachekill/postbuild
```

## Safety Features

- **Safe Delete**: Moves caches to timestamped backup directory
- **Stale Detection**: Configurable threshold (default: 14 days)
- **Project Detection**: Automatically detects project type
- **Restore**: Use `--restore-last` to restore from backup

## Tips

1. **Always use `--dry-run` first** to see what will be cleaned
2. **Use `--list` to understand your cache usage** before cleaning
3. **Configure `.cachekillrc`** for project-specific settings
4. **Use `--restore-last`** if a build fails after cleanup

## Documentation

For detailed documentation, see [DETAILED.md](DETAILED.md).

## License

This project is licensed under the FSL-1.1-MIT License. See the LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run `make test`
6. Submit a pull request
