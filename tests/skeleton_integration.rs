mod common;

use pummel::engine::goose;
use pummel::parser::yaml::YamlParser;
use ntest::timeout;
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::test]
#[timeout(30000)]
async fn test_walking_skeleton_integration() {
    let addr = common::start_mock_server().await;
    let yaml = format!(
        r#"
execution:
- concurrency: 1
  ramp-up: 0s
  hold-for: 1s
  scenario: skeleton

scenarios:
  skeleton:
    requests:
    - http://{}/
"#,
        addr
    );
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "{}", yaml).unwrap();

    let config = YamlParser::parse(file.path()).unwrap();
    let result = goose::run_attack(config).await;
    assert!(result.is_ok());
}
