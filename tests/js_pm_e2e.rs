use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};
use tempfile::TempDir;

fn manifest_path() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    p.to_string_lossy().to_string()
}

fn run_with_env_and_cwd(
    cwd: &std::path::Path,
    envs: &[(&str, &std::path::Path)],
    extra_args: &[&str],
) -> (bool, String, String) {
    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("--bin")
        .arg("cachekill")
        .arg("--manifest-path")
        .arg(manifest_path())
        .arg("--");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let output = cmd.output().expect("failed to run cachekill");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
#[cfg(target_os = "windows")]
fn js_pm_end_to_end_windows() {
    // Fake global caches under LOCALAPPDATA
    let td = TempDir::new().unwrap();
    let base = td.path();
    fs::create_dir_all(base.join("npm-cache")).unwrap();
    fs::create_dir_all(base.join("pnpm").join("store").join("v3")).unwrap();
    fs::create_dir_all(base.join("pnpm-cache")).unwrap();
    fs::create_dir_all(base.join("Yarn").join("Cache")).unwrap();

    // Fake project local Yarn cache by setting CWD
    let proj = TempDir::new().unwrap();
    fs::create_dir_all(proj.path().join(".yarn").join("cache")).unwrap();

    // With --js-pm we should see the seeded paths
    let (ok, out, err) = run_with_env_and_cwd(
        proj.path(),
        &[("LOCALAPPDATA", base)],
        &["--list", "--json", "--js-pm"],
    );
    assert!(
        ok,
        "command failed on Windows. stderr=\n{}\nstdout=\n{}",
        err, out
    );

    assert!(
        out.contains("npm-cache"),
        "missing npm-cache entry. out=\n{}",
        out
    );
    assert!(
        out.contains("pnpm\\store\\v3"),
        "missing pnpm store v3 entry. out=\n{}",
        out
    );
    assert!(
        out.contains("pnpm-cache"),
        "missing pnpm-cache entry. out=\n{}",
        out
    );
    assert!(
        out.contains("Yarn\\Cache") || out.contains("Yarn/Cache"),
        "missing Yarn global cache. out=\n{}",
        out
    );
    assert!(
        out.contains(".yarn\\cache") || out.contains(".yarn/cache"),
        "missing Yarn project cache. out=\n{}",
        out
    );

    // Without --js-pm these specific entries must not appear
    let (ok2, out2, err2) = run_with_env_and_cwd(
        proj.path(),
        &[("LOCALAPPDATA", base)],
        &["--list", "--json"],
    );
    assert!(
        ok2,
        "command (no --js-pm) failed on Windows. stderr=\n{}\nstdout=\n{}",
        err2, out2
    );

    for needle in [
        "npm-cache",
        "pnpm\\store\\v3",
        "pnpm-cache",
        "Yarn\\Cache",
        ".yarn\\cache",
    ] {
        assert!(
            !out2.contains(needle),
            "unexpected JS PM entry '{}' found without --js-pm. out=\n{}",
            needle,
            out2
        );
    }
}

#[test]
#[cfg(target_os = "macos")]
fn js_pm_end_to_end_macos() {
    let td = TempDir::new().unwrap();
    let home = td.path();
    // Seed expected macOS locations
    fs::create_dir_all(home.join(".npm")).unwrap();
    fs::create_dir_all(home.join("Library").join("pnpm").join("store").join("v3")).unwrap();
    fs::create_dir_all(home.join("Library").join("Caches").join("pnpm")).unwrap();
    fs::create_dir_all(home.join("Library").join("Caches").join("Yarn")).unwrap();

    let proj = TempDir::new().unwrap();
    fs::create_dir_all(proj.path().join(".yarn").join("cache")).unwrap();

    let (ok, out, err) = run_with_env_and_cwd(
        proj.path(),
        &[("HOME", home)],
        &["--list", "--json", "--js-pm"],
    );
    assert!(
        ok,
        "command failed on macOS. stderr=\n{}\nstdout=\n{}",
        err, out
    );

    assert!(
        out.contains("/.npm") || out.contains("\\.npm"),
        "missing ~/.npm entry. out=\n{}",
        out
    );
    assert!(
        out.contains("Library/pnpm/store/v3") || out.contains("Library\\pnpm\\store\\v3"),
        "missing pnpm store v3 entry. out=\n{}",
        out
    );
    assert!(
        out.contains("Library/Caches/pnpm") || out.contains("Library\\Caches\\pnpm"),
        "missing pnpm meta cache. out=\n{}",
        out
    );
    assert!(
        out.contains("Library/Caches/Yarn") || out.contains("Library\\Caches\\Yarn"),
        "missing Yarn global cache. out=\n{}",
        out
    );
    assert!(
        out.contains(".yarn/cache") || out.contains(".yarn\\cache"),
        "missing Yarn project cache. out=\n{}",
        out
    );

    let (ok2, out2, err2) =
        run_with_env_and_cwd(proj.path(), &[("HOME", home)], &["--list", "--json"]);
    assert!(
        ok2,
        "command (no --js-pm) failed on macOS. stderr=\n{}\nstdout=\n{}",
        err2, out2
    );

    for needle in [
        "/.npm",
        "Library/pnpm/store/v3",
        "Library/Caches/pnpm",
        "Library/Caches/Yarn",
        ".yarn/cache",
    ] {
        if out2.contains(needle) {
            panic!(
                "unexpected JS PM entry '{}' found without --js-pm. out=\n{}",
                needle, out2
            );
        }
    }
}

#[test]
#[cfg(all(unix, not(target_os = "macos")))]
fn js_pm_end_to_end_linux() {
    let td = TempDir::new().unwrap();
    let home = td.path();
    // Seed expected Linux locations
    fs::create_dir_all(home.join(".npm")).unwrap();
    fs::create_dir_all(
        home.join(".local")
            .join("share")
            .join("pnpm")
            .join("store")
            .join("v3"),
    )
    .unwrap();
    fs::create_dir_all(home.join(".cache").join("pnpm")).unwrap();
    fs::create_dir_all(home.join(".cache").join("yarn")).unwrap();

    let proj = TempDir::new().unwrap();
    fs::create_dir_all(proj.path().join(".yarn").join("cache")).unwrap();

    let (ok, out, err) = run_with_env_and_cwd(
        proj.path(),
        &[("HOME", home)],
        &["--list", "--json", "--js-pm"],
    );
    assert!(
        ok,
        "command failed on Linux. stderr=\n{}\nstdout=\n{}",
        err, out
    );

    assert!(
        out.contains("/.npm") || out.contains("\\.npm"),
        "missing ~/.npm entry. out=\n{}",
        out
    );
    assert!(
        out.contains(".local/share/pnpm/store/v3")
            || out.contains(".local\\share\\pnpm\\store\\v3"),
        "missing pnpm store v3 entry. out=\n{}",
        out
    );
    assert!(
        out.contains(".cache/pnpm") || out.contains(".cache\\pnpm"),
        "missing pnpm meta cache. out=\n{}",
        out
    );
    assert!(
        out.contains(".cache/yarn") || out.contains(".cache\\yarn"),
        "missing Yarn global cache. out=\n{}",
        out
    );
    assert!(
        out.contains(".yarn/cache") || out.contains(".yarn\\cache"),
        "missing Yarn project cache. out=\n{}",
        out
    );

    let (ok2, out2, err2) =
        run_with_env_and_cwd(proj.path(), &[("HOME", home)], &["--list", "--json"]);
    assert!(
        ok2,
        "command (no --js-pm) failed on Linux. stderr=\n{}\nstdout=\n{}",
        err2, out2
    );

    for needle in [
        "/.npm",
        ".local/share/pnpm/store/v3",
        ".cache/pnpm",
        ".cache/yarn",
        ".yarn/cache",
    ] {
        if out2.contains(needle) {
            panic!(
                "unexpected JS PM entry '{}' found without --js-pm. out=\n{}",
                needle, out2
            );
        }
    }
}
