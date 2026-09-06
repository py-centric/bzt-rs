#![cfg(feature = "grpc")]

use pummel::engine;
use pummel::engine::PummelError;
use pummel::models::config::{
    Configuration, ExecutionPlan, HTTPRequestDefinition, ReportingDefinition, ScenarioDefinition,
    SlaAction, SlaCriterion, SlaMetric,
};
use std::collections::HashMap;

#[tokio::test]
async fn test_sla_per_subject_integration() -> Result<(), PummelError> {
    let mut scenarios = HashMap::new();
    scenarios.insert(
        "sla_test".to_string(),
        ScenarioDefinition {
            requests: vec![
                HTTPRequestDefinition::Simple("/fast".to_string()),
                HTTPRequestDefinition::Simple("/slow".to_string()),
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
            scenario: "sla_test".to_string(),
            throughput: None,
            steps: None,
            pacing: None,
        }],
        scenarios,
        reporting: vec![ReportingDefinition {
            module: "junit-xml".to_string(),
            filename: Some("/tmp/sla_test.xml".to_string()),
            url: None,
            token: None,
            org: None,
            bucket: None,
            interval: None,
            failed_threshold: None,
            sla: vec![
                SlaCriterion {
                    metric: SlaMetric::AvgResponseTime,
                    threshold: 50.0,
                    subject: Some("/fast".to_string()),
                    duration: None,
                    action: SlaAction::Stop,
                },
                SlaCriterion {
                    metric: SlaMetric::AvgResponseTime,
                    threshold: 500.0,
                    subject: Some("/slow".to_string()),
                    duration: None,
                    action: SlaAction::Stop,
                },
            ],
        }],
        services: vec![],
        api: None,
    };

    // Start mock server
    let addrs = engine::mock::start_mock_server(config.clone()).await?;
    let host_override = format!("http://{}", addrs.http_addr);

    // Run attack
    let attack =
        pummel::translator::StateTranslator::translate(&config, Some(host_override), None).await?;
    let stats = attack
        .execute()
        .await
        .map_err(|e| PummelError::Goose(Box::new(e)))?;

    // Evaluate SLAs manually to verify the engine logic
    let results = engine::sla::SlaEngine::evaluate(&config.reporting[0].sla, &stats);

    // Both should pass as mock returns instantly
    assert!(results[0].passed, "Fast request SLA should pass");
    assert!(results[1].passed, "Slow request SLA should pass");

    Ok(())
}
