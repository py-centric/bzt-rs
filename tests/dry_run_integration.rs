use std::fs::File;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_dry_run_success() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("test.yaml");
    let mut config_file = File::create(&config_path).unwrap();

    writeln!(
        config_file,
        "
execution:
- concurrency: 1
  ramp-up: 1s
  hold-for: 1s
  scenario: simple
scenarios:
  simple:
    requests:
    - http://localhost
"
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", config_path.to_str().unwrap(), "--dry-run"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Validation Successful"));
}

#[test]
fn test_dry_run_missing_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("test.yaml");
    let mut config_file = File::create(&config_path).unwrap();

    writeln!(
        config_file,
        "
execution:
- concurrency: 1
  ramp-up: 1s
  hold-for: 1s
  scenario: simple
scenarios:
  simple:
    data-sources:
    - missing.csv
    requests:
    - http://localhost
"
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", config_path.to_str().unwrap(), "--dry-run"])
        .output()
        .expect("failed to execute process");

    // It should fail because missing.csv doesn't exist
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Validation Failed"));
    assert!(stdout.contains("Missing data-source: missing.csv"));
}
