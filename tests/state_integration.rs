mod common;

use bzt_rs::engine::goose;
use bzt_rs::models::config::{
    AssertionDefinition, Configuration, DetailedRequest, ExecutionPlan, HTTPRequestDefinition,
    ScenarioDefinition,
};
use ntest::timeout;
use std::collections::HashMap;

#[tokio::test]
#[timeout(30000)]
async fn test_state_integration() {
    let addr = common::start_mock_server().await;
    let mut scenarios = HashMap::new();

    let mut extract_rules = HashMap::new();
    extract_rules.insert("testVar".to_string(), r#"$.hello"#.to_string());

    scenarios.insert(
        "state_test".to_string(),
        ScenarioDefinition {
            requests: vec![
                HTTPRequestDefinition::Detailed(Box::new(DetailedRequest {
                    url: format!("http://{}/user/123", addr),
                    method: Some("GET".to_string()),
                    headers: None,
                    body: None,
                    label: None,
                    body_file: None,
                    timeout: None,
                    on_start: false,
                    think_time: None,
                    extract_jsonpath: Some(extract_rules),
                    extract_regexp: None,
                    assert: vec![AssertionDefinition {
                        contains: vec!["123".to_string()],
                        subject: "body".to_string(),
                        regexp: false,
                        not: false,
                    }],
                    protocol: None,
                    message: None,
                    method_name: None,
                    execute_if: None,
                    loop_while: None,
                    ..Default::default()
                })),
                HTTPRequestDefinition::Detailed(Box::new(DetailedRequest {
                    url: format!("http://{}/user/${{testVar}}", addr),
                    method: Some("GET".to_string()),
                    headers: None,
                    body: None,
                    label: None,
                    body_file: None,
                    timeout: None,
                    on_start: false,
                    think_time: None,
                    extract_jsonpath: None,
                    extract_regexp: None,
                    assert: vec![],
                    protocol: None,
                    message: None,
                    method_name: None,
                    execute_if: None,
                    loop_while: None,
                    ..Default::default()
                })),
            ],
            weight: 1,
            think_time: None,
            data_sources: None,
            headers: None,
        },
    );

    let config = Configuration {
        execution: vec![ExecutionPlan {
            concurrency: 1,
            ramp_up: "0s".to_string(),
            hold_for: "1s".to_string(),
            scenario: "state_test".to_string(),
            throughput: None,
            steps: None,
            pacing: None,
        }],
        scenarios,
        reporting: vec![],
    };

    let result = goose::run_attack(config).await;
    assert!(result.is_ok());
}
