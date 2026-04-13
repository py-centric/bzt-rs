mod common;

use bzt_rs::engine::goose;
use bzt_rs::models::config::{
    Configuration, ExecutionPlan, HTTPRequestDefinition, ScenarioDefinition,
};
use ntest::timeout;
use std::collections::HashMap;

#[tokio::test]
#[timeout(30000)]
async fn test_csv_parameterization_integration() {
    let addr = common::start_mock_server().await;
    let mut scenarios = HashMap::new();

    scenarios.insert(
        "csv_test".to_string(),
        ScenarioDefinition {
            requests: vec![HTTPRequestDefinition::Simple(format!(
                "http://{}/user/${{id}}",
                addr
            ))],
            weight: 1,
            think_time: Some("100ms".to_string()),
            data_sources: Some(vec!["users.csv".to_string()]),
        },
    );

    let config = Configuration {
        execution: vec![ExecutionPlan {
            concurrency: 2,
            ramp_up: "0s".to_string(),
            hold_for: "1s".to_string(),
            scenario: "csv_test".to_string(),
            throughput: None,
            steps: None,
        }],
        scenarios,
        reporting: vec![],
    };

    let result = goose::run_attack(config).await;
    assert!(result.is_ok());
}
