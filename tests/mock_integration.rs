#![cfg(feature = "grpc")]

use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::time::Duration;
use tempfile::tempdir;
use tokio::time::sleep;

#[tokio::test]
async fn test_mock_server_standalone() {
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
    - url: /api/hello
      assert:
      - contains: [\"Hello Mock\"]
"
    )
    .unwrap();

    let mut child = Command::new("cargo")
        .args(["run", "--features", "grpc", "--", config_path.to_str().unwrap(), "--mock"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to execute process");

    // Poll until the process exits or 5 seconds elapses
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                panic!("mock server exited prematurely with status: {status}");
            }
            Ok(None) => {
                // Still running — what we want. Wait a bit then recheck.
                if std::time::Instant::now() >= deadline {
                    break;
                }
                sleep(Duration::from_millis(200)).await;
            }
            Err(e) => {
                panic!("error checking mock server status: {e}");
            }
        }
    }

    assert!(
        child.try_wait().unwrap().is_none(),
        "mock server should still be running"
    );
    child.kill().expect("failed to kill mock server");
}

#[test]
fn test_mock_run_full_lifecycle() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("test.yaml");
    let mut config_file = File::create(&config_path).unwrap();

    writeln!(
        config_file,
        "
execution:
- concurrency: 1
  ramp-up: 0s
  hold-for: 2s
  scenario: simple
scenarios:
  simple:
    requests:
    - url: /api/test
      assert:
      - contains: [\"Success\"]
"
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(["run", "--features", "grpc", "--", config_path.to_str().unwrap(), "--mock-run"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Mock server started"));
}
