use std::path::Path;
use std::process::Command;

fn get_binary_path() -> String {
    let target_dir = if cfg!(debug_assertions) {
        "target/debug"
    } else {
        "target/release"
    };

    let binary_name = if cfg!(target_os = "windows") {
        "cachekill.exe"
    } else {
        "cachekill"
    };

    let binary_path = format!("{}/{}", target_dir, binary_name);

    // Check if the binary exists, if not try cargo run
    if Path::new(&binary_path).exists() {
        binary_path
    } else {
        // Fallback to cargo run for development
        "cargo".to_string()
    }
}

fn run_cachekill(args: &[&str]) -> std::process::Output {
    let binary_path = get_binary_path();
    let mut cmd = Command::new(&binary_path);

    if binary_path == "cargo" {
        cmd.args(&["run", "--"]);
        cmd.args(args);
    } else {
        cmd.args(args);
    }

    cmd.output().expect("Failed to execute command")
}

#[test]
fn test_ci_mode_prebuild() {
    let output = run_cachekill(&["--ci", "prebuild"]);

    // Should exit with code 0 (success), 3 (nothing to do), or 5 (fatal error)
    let exit_code = output.status.code();
    assert!(exit_code == Some(0) || exit_code == Some(3) || exit_code == Some(5));
}

#[test]
fn test_ci_mode_postbuild() {
    let output = run_cachekill(&["--ci", "postbuild"]);

    // Should exit with code 0 (success), 3 (nothing to do), or 5 (fatal error)
    let exit_code = output.status.code();
    assert!(exit_code == Some(0) || exit_code == Some(3) || exit_code == Some(5));
}

#[test]
fn test_doctor_command() {
    let output = run_cachekill(&["--doctor"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CacheKill System Diagnostics"));
    assert!(stdout.contains("Integrations:"));
}

#[test]
fn test_hf_list_command() {
    let output = run_cachekill(&["--hf", "--list"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should either show cache or "not found" message
    assert!(stdout.contains("HuggingFace") || stdout.contains("not found"));
}

#[test]
fn test_torch_list_command() {
    let output = run_cachekill(&["--torch", "--list"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should either show cache or "not found" message
    assert!(stdout.contains("PyTorch") || stdout.contains("not found"));
}

#[test]
fn test_vercel_status_command() {
    let output = run_cachekill(&["--vercel", "--list"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Vercel Integration Status"));
}

#[test]
fn test_cloudflare_status_command() {
    let output = run_cachekill(&["--cloudflare", "--list"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cloudflare Integration Status"));
}

#[test]
fn test_json_output() {
    let output = run_cachekill(&["--doctor", "--json"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    assert!(stdout.trim().starts_with("{"));
    assert!(stdout.trim().ends_with("}"));
}

#[test]
fn test_ci_mode_with_invalid_mode() {
    let output = run_cachekill(&["--ci", "invalid"]);

    // Should exit with code 4 (config error)
    assert_eq!(output.status.code(), Some(4));
}

#[test]
fn test_hf_clean_with_model() {
    let output = run_cachekill(&["--hf", "--model", "test-model"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should either show cache or "not found" message
    assert!(stdout.contains("HuggingFace") || stdout.contains("not found"));
}

#[test]
fn test_torch_clean_command() {
    let output = run_cachekill(&["--torch"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should either show cache or "not found" message
    assert!(stdout.contains("PyTorch") || stdout.contains("not found"));
}

#[test]
fn test_vercel_purge_command() {
    let output = run_cachekill(&["--vercel"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show purge attempt or recommendation
    assert!(stdout.contains("Vercel") || stdout.contains("CLI") || stdout.contains("Token"));
}

#[test]
fn test_cloudflare_purge_command() {
    let output = run_cachekill(&["--cloudflare"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show purge attempt or recommendation
    assert!(stdout.contains("Cloudflare") || stdout.contains("CLI") || stdout.contains("Token"));
}

#[test]
fn test_help_command() {
    let output = run_cachekill(&["--help"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CacheKill"));
    assert!(stdout.contains("Options:"));
    assert!(stdout.contains("--ci"));
    assert!(stdout.contains("--hf"));
    assert!(stdout.contains("--torch"));
    assert!(stdout.contains("--vercel"));
    assert!(stdout.contains("--cloudflare"));
}

#[test]
fn test_version_command() {
    let output = run_cachekill(&["--version"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.7"));
}

#[test]
fn test_dry_run_mode() {
    let output = run_cachekill(&["--dry-run"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show dry run information or cache entries
    assert!(
        stdout.contains("dry")
            || stdout.contains("would be")
            || stdout.contains("Cache")
            || stdout.contains("entries")
    );
}

#[test]
fn test_list_mode() {
    let output = run_cachekill(&["--list"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cache") || stdout.contains("entries"));
}

#[test]
fn test_force_mode() {
    let output = run_cachekill(&["--force"]);

    // Should not prompt for confirmation - may succeed, show no caches, partial success, or have other issues
    let exit_code = output.status.code();
    assert!(
        exit_code == Some(0)
            || exit_code == Some(2)
            || exit_code == Some(3)
            || exit_code == Some(5)
    );
}

#[test]
fn test_json_list_mode() {
    let output = run_cachekill(&["--list", "--json"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    assert!(stdout.trim().starts_with("{"));
    assert!(stdout.trim().ends_with("}"));
}

#[test]
fn test_language_filter() {
    let output = run_cachekill(&["--lang", "js", "--list"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show JavaScript-related caches or empty result
    assert!(stdout.contains("Cache") || stdout.contains("entries"));
}

#[test]
fn test_docker_integration() {
    let output = run_cachekill(&["--docker", "--list"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show Docker-related information or cache entries
    assert!(
        stdout.contains("Docker")
            || stdout.contains("docker")
            || stdout.contains("Cache")
            || stdout.contains("entries")
    );
}

#[test]
fn test_npx_integration() {
    let output = run_cachekill(&["--npx", "--list"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show NPX-related information or cache entries
    assert!(
        stdout.contains("NPX")
            || stdout.contains("npx")
            || stdout.contains("Cache")
            || stdout.contains("entries")
    );
}
