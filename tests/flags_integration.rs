use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_removed_manager_flag_is_rejected() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("test.yaml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
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
        .args(["run", "--", config_path.to_str().unwrap(), "--manager"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected argument") || stderr.contains("error:"),
        "CLI should reject --manager flag, got stderr: {}",
        stderr
    );
}

#[test]
fn test_removed_worker_flag_is_rejected() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("test.yaml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
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
        .args(["run", "--", config_path.to_str().unwrap(), "--worker"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected argument") || stderr.contains("error:"),
        "CLI should reject --worker flag, got stderr: {}",
        stderr
    );
}

#[test]
fn test_removed_metrics_flag_is_rejected() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("test.yaml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
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
        .args(["run", "--", config_path.to_str().unwrap(), "--metrics"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected argument") || stderr.contains("error:"),
        "CLI should reject --metrics flag, got stderr: {}",
        stderr
    );
}
