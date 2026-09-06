mod common;

use pummel::engine::goose;
use pummel::models::config::{
    AssertionDefinition, Configuration, DetailedRequest, ExecutionPlan, HttpMethod,
    HTTPRequestDefinition, ReportingDefinition, ScenarioDefinition, SlaAction, SlaCriterion,
    SlaMetric,
};
use ntest::timeout;
use std::collections::HashMap;

#[tokio::test]
#[timeout(30000)]
async fn test_sla_and_pacing_integration() {
    let addr = common::start_mock_server().await;
    let mut scenarios = HashMap::new();

    scenarios.insert(
        "sla_test".to_string(),
        ScenarioDefinition {
            requests: vec![HTTPRequestDefinition::Detailed(Box::new(DetailedRequest {
                url: format!("http://{}/", addr),
                method: Some(HttpMethod::Get),
                headers: None,
                body: None,
                label: None,
                body_file: None,
                timeout: None,
                on_start: false,
                think_time: None,
                extract_jsonpath: None,
                extract_regexp: None,
                assert: vec![AssertionDefinition {
                    contains: vec!["Hello".to_string()],
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
            }))],

            weight: 1,
            think_time: None,
            data_sources: None,
            headers: None,
        },
    );

    let config = Configuration {
        execution: vec![ExecutionPlan {
            concurrency: 10,
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
            filename: Some("/tmp/bzt-sla-test-junit.xml".to_string()),
            url: None,
            token: None,
            org: None,
            bucket: None,
            interval: None,
            // fail-rate threshold of 5% — mock always returns 200, so this should pass
            failed_threshold: Some(0.05),
            sla: vec![
                SlaCriterion {
                    metric: SlaMetric::FailRate,
                    threshold: 0.05,
                    subject: None,
                    duration: None,
                    action: SlaAction::Warn,
                },
                SlaCriterion {
                    metric: SlaMetric::AvgResponseTime,
                    threshold: 5000.0,
                    subject: None,
                    duration: None,
                    action: SlaAction::Warn,
                },
            ],
        }],
        services: vec![],
        api: None,
    };

    let result = goose::run_attack(config).await;
    assert!(result.is_ok());
}
