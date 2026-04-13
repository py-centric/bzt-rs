mod common;

use bzt_rs::engine::goose;
use bzt_rs::normalizer::{SchemaNormalizer, ShorthandConfiguration};
use ntest::timeout;
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::test]
#[timeout(30000)]
async fn test_shorthand_integration() {
    let addr = common::start_mock_server().await;
    let toml_content = format!(
        r#"
[execution]
concurrency = 1
scenario = "shorthand"

[scenarios.shorthand]
requests = ["http://{}/"]
"#,
        addr
    );
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "{}", toml_content).unwrap();

    let shorthand: ShorthandConfiguration = toml::from_str(&toml_content).unwrap();
    let config = SchemaNormalizer::normalize_shorthand(shorthand);

    assert_eq!(config.execution[0].concurrency, 1);
    assert_eq!(config.scenarios.len(), 1);

    let result = goose::run_attack(config).await;
    assert!(result.is_ok());
}
