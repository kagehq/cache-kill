use toml;

fn main() {
    let content = r#"# CacheKill Configuration File
# This file configures the default behavior of cachekill

# Default language filter (auto, js, py, rust, java, ml)
default_lang = "auto"

# Number of days after which caches are considered stale (default: 14)
stale_days = 14

# Enable safe delete by default (moves to backup before deletion)
safe_delete = true

# Default backup directory (relative to project root)
backup_dir = ".cachekill-backup"

# Additional include patterns (glob patterns)
# include_paths = [
#     "**/custom-cache",
#     "**/build-artifacts"
# ]

# Exclude patterns (glob patterns)
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
include_npx = false"#;

    match toml::from_str::<toml::Value>(content) {
        Ok(_) => println!("TOML parsing successful"),
        Err(e) => println!("TOML parsing error: {}", e),
    }
}
