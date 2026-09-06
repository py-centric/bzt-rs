#![cfg(feature = "grpc")]

use pummel::engine;
use pummel::engine::PummelError;
use pummel::models::config::{
    Configuration, DetailedRequest, ExecutionPlan, HTTPRequestDefinition, ScenarioDefinition,
};
use std::collections::HashMap;

#[tokio::test]
async fn test_grpc_integration() -> Result<(), PummelError> {
    let mut scenarios = HashMap::new();
    scenarios.insert(
        "grpc_test".to_string(),
        ScenarioDefinition {
            requests: vec![HTTPRequestDefinition::Detailed(Box::new(DetailedRequest {
                url: "unused".to_string(),
                protocol: Some("grpc".to_string()),
                method_name: Some("bzt_mock.MockService/Call".to_string()),
                body: Some(r#"{"method": "test", "payload": "data"}"#.to_string()),
                ..Default::default()
            }))],
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
            scenario: "grpc_test".to_string(),
            throughput: None,
            steps: None,
            pacing: None,
        }],
        scenarios,
        reporting: vec![],
        services: vec![],
        api: None,
    };

    // Start mock server
    let addrs = engine::mock::start_mock_server(config.clone()).await?;
    let host_override = format!("http://{}", addrs.grpc_addr);

    // Run attack
    let attack =
        pummel::translator::StateTranslator::translate(&config, Some(host_override), None).await?;
    let stats = attack
        .execute()
        .await
        .map_err(|e| PummelError::Goose(Box::new(e)))?;

    assert!(stats.duration > 0);
    Ok(())
}
