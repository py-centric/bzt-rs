mod common;

use pummel::engine::goose;
use pummel::models::config::{
    Configuration, ExecutionPlan, HTTPRequestDefinition, ScenarioDefinition,
};
use ntest::timeout;
use std::collections::HashMap;

#[tokio::test]
#[timeout(30000)]
async fn test_branching_and_weights_integration() {
    let addr = common::start_mock_server().await;
    let mut scenarios = HashMap::new();

    // Scenario 1: weight 80
    scenarios.insert(
        "high_traffic".to_string(),
        ScenarioDefinition {
            requests: vec![HTTPRequestDefinition::Simple(format!(
                "http://{}/high",
                addr
            ))],
            weight: 80,
            think_time: None,
            data_sources: None,
            headers: None,
        },
    );

    // Scenario 2: weight 20
    scenarios.insert(
        "low_traffic".to_string(),
        ScenarioDefinition {
            requests: vec![HTTPRequestDefinition::Simple(format!(
                "http://{}/low",
                addr
            ))],
            weight: 20,
            think_time: None,
            data_sources: None,
            headers: None,
        },
    );

    let config = Configuration {
        execution: vec![
            ExecutionPlan {
                concurrency: 10,
                ramp_up: "0s".to_string(),
                hold_for: "2s".to_string(),
                scenario: "high_traffic".to_string(),
                throughput: None,
                steps: None,
                pacing: None,
            },
            ExecutionPlan {
                concurrency: 10,
                ramp_up: "0s".to_string(),
                hold_for: "2s".to_string(),
                scenario: "low_traffic".to_string(),
                throughput: None,
                steps: None,
                pacing: None,
            },
        ],
        scenarios,
        reporting: vec![],
        services: vec![],
        api: None,
    };

    let result = goose::run_attack(config).await;
    assert!(result.is_ok());
}
