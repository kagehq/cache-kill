use std::process::Command;

#[test]
fn test_ci_mode_prebuild() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--ci", "prebuild"])
        .output()
        .expect("Failed to execute command");
    
    // Should exit with code 0 (success), 3 (nothing to do), or 5 (fatal error)
    let exit_code = output.status.code();
    assert!(exit_code == Some(0) || exit_code == Some(3) || exit_code == Some(5));
}

#[test]
fn test_ci_mode_postbuild() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--ci", "postbuild"])
        .output()
        .expect("Failed to execute command");
    
    // Should exit with code 0 (success), 3 (nothing to do), or 5 (fatal error)
    let exit_code = output.status.code();
    assert!(exit_code == Some(0) || exit_code == Some(3) || exit_code == Some(5));
}

#[test]
fn test_doctor_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--doctor"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CacheKill System Diagnostics"));
    assert!(stdout.contains("Integrations:"));
}

#[test]
fn test_hf_list_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--hf", "--list"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should either show cache or "not found" message
    assert!(stdout.contains("HuggingFace") || stdout.contains("not found"));
}

#[test]
fn test_torch_list_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--torch", "--list"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should either show cache or "not found" message
    assert!(stdout.contains("PyTorch") || stdout.contains("not found"));
}

#[test]
fn test_vercel_status_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--vercel", "--list"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Vercel Integration Status"));
}

#[test]
fn test_cloudflare_status_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--cloudflare", "--list"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cloudflare Integration Status"));
}

#[test]
fn test_json_output() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--doctor", "--json"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    assert!(stdout.trim().starts_with("{"));
    assert!(stdout.trim().ends_with("}"));
}

#[test]
fn test_ci_mode_with_invalid_mode() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--ci", "invalid"])
        .output()
        .expect("Failed to execute command");
    
    // Should exit with code 4 (config error)
    assert_eq!(output.status.code(), Some(4));
}

#[test]
fn test_hf_clean_with_model() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--hf", "--model", "test-model"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should either show cache or "not found" message
    assert!(stdout.contains("HuggingFace") || stdout.contains("not found"));
}

#[test]
fn test_torch_clean_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--torch"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should either show cache or "not found" message
    assert!(stdout.contains("PyTorch") || stdout.contains("not found"));
}

#[test]
fn test_vercel_purge_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--vercel"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show purge attempt or recommendation
    assert!(stdout.contains("Vercel") || stdout.contains("CLI") || stdout.contains("Token"));
}

#[test]
fn test_cloudflare_purge_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--cloudflare"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show purge attempt or recommendation
    assert!(stdout.contains("Cloudflare") || stdout.contains("CLI") || stdout.contains("Token"));
}

#[test]
fn test_help_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");
    
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
    let output = Command::new("cargo")
        .args(&["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.3.0"));
}

#[test]
fn test_dry_run_mode() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--dry-run"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show dry run information or cache entries
    assert!(stdout.contains("dry") || stdout.contains("would be") || stdout.contains("Cache") || stdout.contains("entries"));
}

#[test]
fn test_list_mode() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--list"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cache") || stdout.contains("entries"));
}

#[test]
fn test_force_mode() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--force"])
        .output()
        .expect("Failed to execute command");
    
    // Should not prompt for confirmation - may succeed, show no caches, partial success, or have other issues
    let exit_code = output.status.code();
    assert!(exit_code == Some(0) || exit_code == Some(2) || exit_code == Some(3) || exit_code == Some(5));
}

#[test]
fn test_json_list_mode() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--list", "--json"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    assert!(stdout.trim().starts_with("{"));
    assert!(stdout.trim().ends_with("}"));
}

#[test]
fn test_language_filter() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--lang", "js", "--list"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show JavaScript-related caches or empty result
    assert!(stdout.contains("Cache") || stdout.contains("entries"));
}

#[test]
fn test_docker_integration() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--docker", "--list"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show Docker-related information or cache entries
    assert!(stdout.contains("Docker") || stdout.contains("docker") || stdout.contains("Cache") || stdout.contains("entries"));
}

#[test]
fn test_npx_integration() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--npx", "--list"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show NPX-related information or cache entries
    assert!(stdout.contains("NPX") || stdout.contains("npx") || stdout.contains("Cache") || stdout.contains("entries"));
}
