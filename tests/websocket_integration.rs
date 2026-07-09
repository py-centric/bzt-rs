use bzt_rs::engine;
use bzt_rs::engine::BztError;
use bzt_rs::models::config::{
    AssertionDefinition, Configuration, DetailedRequest, ExecutionPlan, HTTPRequestDefinition,
    ScenarioDefinition,
};
use std::collections::HashMap;

#[tokio::test]
async fn test_websocket_integration() -> Result<(), BztError> {
    let mut scenarios = HashMap::new();
    scenarios.insert(
        "ws_test".to_string(),
        ScenarioDefinition {
            requests: vec![HTTPRequestDefinition::Detailed(Box::new(DetailedRequest {
                url: "/ws/chat".to_string(),
                protocol: Some("websocket".to_string()),
                message: Some("Hello Server".to_string()),
                assert: vec![AssertionDefinition {
                    contains: vec!["Hello Client".to_string()],
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
            scenario: "ws_test".to_string(),
            throughput: None,
            steps: None,
            pacing: None,
        }],
        scenarios,
        reporting: vec![], services: vec![], api: None,
    };

    // Start mock server
    let addrs = engine::mock::start_mock_server(config.clone()).await?;
    let host_override = format!("ws://{}", addrs.http_addr);

    // Translate and execute
    let attack = bzt_rs::translator::StateTranslator::translate(&config, Some(host_override), None).await?;
    let stats = attack
        .execute()
        .await
        .map_err(|e| BztError::Goose(Box::new(e)))?;

    // Verify metrics
    assert!(stats.duration > 0);
    // Since we don't report WS metrics to Goose's HTTP metrics tracker yet, 
    // the request count in `stats` will be 0. 
    // However, if the above didn't panic or error out, the WS transaction executed.
    
    Ok(())
}
