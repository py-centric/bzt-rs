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

#[test]
fn test_cli_option_override_applied() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("test.yaml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
    let csv_name = "test_existing_temp.csv";
    let _ = std::fs::File::create(csv_name).unwrap();

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
    - {}
    requests:
    - http://localhost
",
        csv_name
    )
    .unwrap();

    // With a valid scenario and existing data-source, dry-run succeeds
    let output_success = Command::new("cargo")
        .args(["run", "--", config_path.to_str().unwrap(), "--dry-run"])
        .output()
        .expect("failed to execute process");
    
    // Clean up temporary file
    let _ = std::fs::remove_file(csv_name);

    assert!(output_success.status.success());

    // Overriding the data-source to a non-existent file causes validation to fail
    let output_fail = Command::new("cargo")
        .args([
            "run",
            "--",
            config_path.to_str().unwrap(),
            "--dry-run",
            "-o",
            "scenarios.simple.data-sources.0=missing_override_temp.csv",
        ])
        .output()
        .expect("failed to execute process");
    assert!(!output_fail.status.success());
    let stdout = String::from_utf8_lossy(&output_fail.stdout);
    assert!(stdout.contains("Validation Failed."));
    assert!(stdout.contains("Missing data-source: missing_override_temp.csv"));
}

#[test]
fn test_cli_option_override_console_disabled() {
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

    // Disabling console output should silence standard validation print statements on stdout
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            config_path.to_str().unwrap(),
            "--dry-run",
            "-o",
            "modules.console.disable=true",
        ])
        .output()
        .expect("failed to execute process");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // When console is disabled, no prints to stdout should occur from pummel
    assert!(stdout.trim().is_empty(), "Stdout should be empty but got: {}", stdout);
}
