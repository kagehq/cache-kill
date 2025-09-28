use std::process::Command;

// Minimal smoke test to ensure the new --js-pm flag is wired into the CLI
// and that it returns JSON (object) without errors.
#[test]
fn js_pm_list_json_succeeds_and_is_object() {
    let output = Command::new("cargo")
        .args(["run", "--", "--list", "--json", "--js-pm"])
        .output()
        .expect("failed to run cachekill");

    assert!(output.status.success(), "command failed: status={:?}\nstdout=\n{}\nstderr=\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let s = stdout.trim();
    assert!(s.starts_with("{"), "stdout should start with '{{' but was: {}", s);
    assert!(s.ends_with("}"), "stdout should end with '}}' but was: {}", s);
}
