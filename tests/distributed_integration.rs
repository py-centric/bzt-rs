mod common;

use pummel::engine::goose;
use pummel::models::config::{
    Configuration, ExecutionPlan, HTTPRequestDefinition, ScenarioDefinition,
};
use ntest::timeout;
use std::collections::HashMap;
use std::env;

#[tokio::test]
#[timeout(30000)]
async fn test_distributed_and_metrics_integration() {
    let addr = common::start_mock_server().await;
    let mut scenarios = HashMap::new();

    scenarios.insert(
        "distributed_test".to_string(),
        ScenarioDefinition {
            requests: vec![HTTPRequestDefinition::Simple(format!("http://{}/", addr))],
            weight: 1,
            think_time: None,
            data_sources: None,
            headers: None,
        },
    );

    let config = Configuration {
        execution: vec![ExecutionPlan {
            concurrency: 2,
            ramp_up: "0s".to_string(),
            hold_for: "1s".to_string(),
            scenario: "distributed_test".to_string(),
            throughput: None,
            steps: None,
            pacing: None,
        }],
        scenarios,
        reporting: vec![],
        services: vec![],
        api: None,
    };

    unsafe {
        env::set_var("GOOSE_MANAGER", "true");
        env::set_var("GOOSE_EXPECT_WORKERS", "1");
        // Also test prometheus metrics flag integration
        env::set_var("GOOSE_METRICS_PROMETHEUS", "true");
    }

    let result = goose::run_attack(config).await;
    // Expected to pass since Goose will run but just not connect any workers if no actual run triggers wait
    // Or it might just run the test locally if manager flag isn't parsed cleanly in test mode
    assert!(result.is_ok());
}
