use bzt_rs::engine;
use bzt_rs::engine::BztError;
use bzt_rs::models::config::{
    AssertionDefinition, Configuration, DetailedRequest, ExecutionPlan, HTTPRequestDefinition,
    ScenarioDefinition,
};
use std::collections::HashMap;

#[tokio::test]
async fn test_grpc_server_streaming_integration() -> Result<(), BztError> {
    let mut scenarios = HashMap::new();
    scenarios.insert(
        "grpc_stream".to_string(),
        ScenarioDefinition {
            requests: vec![HTTPRequestDefinition::Detailed(Box::new(DetailedRequest {
                url: "unused".to_string(),
                protocol: Some("grpc".to_string()),
                method_name: Some("bzt_mock.MockService/ServerStream".to_string()),
                grpc_mode: Some("server-streaming".to_string()),
                body: Some(r#"{"method": "test-stream", "payload": "data"}"#.to_string()),
                assert: vec![AssertionDefinition {
                    contains: vec!["Part".to_string()],
                    subject: "body".to_string(),
                    regexp: false,
                    not: false,
                }],
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
            scenario: "grpc_stream".to_string(),
            throughput: None,
            steps: None,
            pacing: None,
        }],
        scenarios,
        reporting: vec![],
    };

    // Start mock server
    let addrs = engine::mock::start_mock_server(config.clone()).await?;
    let host_override = format!("http://{}", addrs.grpc_addr);

    // Run attack
    let attack = bzt_rs::translator::StateTranslator::translate(&config, Some(host_override), None).await?;
    let stats = attack
        .execute()
        .await
        .map_err(|e| BztError::Goose(Box::new(e)))?;

    assert!(stats.duration > 0);
    Ok(())
}

#[tokio::test]
async fn test_grpc_bidi_streaming_integration() -> Result<(), BztError> {
    let mut scenarios = HashMap::new();
    scenarios.insert(
        "grpc_bidi".to_string(),
        ScenarioDefinition {
            requests: vec![HTTPRequestDefinition::Detailed(Box::new(DetailedRequest {
                url: "unused".to_string(),
                protocol: Some("grpc".to_string()),
                method_name: Some("bzt_mock.MockService/BidiStream".to_string()),
                grpc_mode: Some("bidi-streaming".to_string()),
                messages: vec![
                    r#"{"method": "msg1", "payload": "p1"}"#.to_string(),
                    r#"{"method": "msg2", "payload": "p2"}"#.to_string(),
                ],
                assert: vec![AssertionDefinition {
                    contains: vec!["Echo".to_string()],
                    subject: "body".to_string(),
                    regexp: false,
                    not: false,
                }],
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
            scenario: "grpc_bidi".to_string(),
            throughput: None,
            steps: None,
            pacing: None,
        }],
        scenarios,
        reporting: vec![],
    };

    // Start mock server
    let addrs = engine::mock::start_mock_server(config.clone()).await?;
    let host_override = format!("http://{}", addrs.grpc_addr);

    // Run attack
    let attack = bzt_rs::translator::StateTranslator::translate(&config, Some(host_override), None).await?;
    let stats = attack
        .execute()
        .await
        .map_err(|e| BztError::Goose(Box::new(e)))?;

    assert!(stats.duration > 0);
    Ok(())
}
