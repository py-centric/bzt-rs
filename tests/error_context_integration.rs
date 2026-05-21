use std::io::Write;
use tempfile::NamedTempFile;

/// Test that parser errors carry `[CATEGORY]` format and file path context.
#[test]
fn test_parser_error_includes_category_and_file_path() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "execution:").unwrap();
    writeln!(file, "  - concurrency: not_a_number").unwrap();
    writeln!(file, "    ramp-up: 5s").unwrap();
    writeln!(file, "    hold-for: 30s").unwrap();
    writeln!(file, "    scenario: basic").unwrap();
    writeln!(file, "scenarios:").unwrap();
    writeln!(file, "  basic:").unwrap();
    writeln!(file, "    requests:").unwrap();
    writeln!(file, "      - http://localhost:8080/").unwrap();

    let result = bzt_rs::parser::yaml::YamlParser::parse(file.path());
    let err = result.unwrap_err();
    let msg = err.to_string();

    assert!(
        msg.contains("[SERDE]"),
        "error should contain [SERDE] category, got: {}",
        msg
    );
    assert!(
        msg.contains(file.path().to_str().unwrap()),
        "error should contain file path '{}', got: {}",
        file.path().display(),
        msg
    );
}

/// Test that `deny_unknown_fields` errors include the field name and file path.
#[test]
fn test_deny_unknown_fields_error_includes_context() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "execution:").unwrap();
    writeln!(file, "  - concurrency: 10").unwrap();
    writeln!(file, "    ramp-up: 5s").unwrap();
    writeln!(file, "    hold-for: 30s").unwrap();
    writeln!(file, "    scenario: basic").unwrap();
    writeln!(file, "    nonexistent_field: true").unwrap();
    writeln!(file, "scenarios:").unwrap();
    writeln!(file, "  basic:").unwrap();
    writeln!(file, "    requests:").unwrap();
    writeln!(file, "      - http://localhost:8080/").unwrap();

    let result = bzt_rs::parser::yaml::YamlParser::parse(file.path());
    let err = result.unwrap_err();
    let msg = err.to_string();

    assert!(
        msg.contains("[SERDE]"),
        "error should contain [SERDE] category, got: {}",
        msg
    );
    assert!(
        msg.to_lowercase().contains("nonexistent_field"),
        "error should mention the unknown field, got: {}",
        msg
    );
    assert!(
        msg.contains(file.path().to_str().unwrap()),
        "error should contain file path, got: {}",
        msg
    );
}

/// Test that broken YAML syntax produces a Serde error with file path.
#[test]
fn test_broken_yaml_syntax_error_includes_context() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "execution:").unwrap();
    writeln!(file, "  - concurrency: ten").unwrap(); // invalid: string instead of usize
    writeln!(file, "    ramp-up: 5s").unwrap();
    writeln!(file, "    hold-for: 30s").unwrap();
    writeln!(file, "    scenario: basic").unwrap();
    writeln!(file, "scenarios:").unwrap();
    writeln!(file, "  basic:").unwrap();
    writeln!(file, "    requests:").unwrap();
    writeln!(file, "      - http://localhost:8080/").unwrap();

    let result = bzt_rs::parser::yaml::YamlParser::parse(file.path());
    let err = result.unwrap_err();
    let msg = err.to_string();

    assert!(
        msg.contains("[SERDE]"),
        "error should contain [SERDE] category, got: {}",
        msg
    );
    assert!(
        msg.contains(file.path().to_str().unwrap()),
        "error should contain file path, got: {}",
        msg
    );
}

/// Test that translator errors propagate with context.
#[tokio::test]
async fn test_translator_data_source_missing_returns_error() {
    let yaml = r#"
execution:
  - concurrency: 10
    ramp-up: 5s
    hold-for: 30s
    scenario: data_test

scenarios:
  data_test:
    data-sources:
      - nonexistent_file.csv
    requests:
      - http://localhost:8080/
"#;
    let config: bzt_rs::models::config::Configuration = serde_yaml::from_str(yaml).unwrap();

    let result = bzt_rs::translator::StateTranslator::translate(&config, None, None).await;
    let msg = match result {
        Ok(_) => panic!("expected translation to fail for missing data source"),
        Err(e) => e.to_string(),
    };

    assert!(
        msg.contains("[IO]") || msg.contains("[VALIDATION]"),
        "error should contain [IO] or [VALIDATION] category, got: {}",
        msg
    );
    assert!(
        msg.contains("nonexistent_file.csv"),
        "error should mention the missing file, got: {}",
        msg
    );
}
