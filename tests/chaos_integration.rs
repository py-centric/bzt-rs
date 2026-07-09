mod common;

use bzt_rs::engine::goose;
use bzt_rs::models::config::{
    Configuration, ExecutionPlan, HTTPRequestDefinition, ScenarioDefinition,
    ServiceDefinition, ApiConfig, SlaCriterion, SlaMetric, SlaAction, DetailedRequest
};
use ntest::timeout;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

async fn send_get(addr: &str, path: &str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let req = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, addr);
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut resp = String::new();
    stream.read_to_string(&mut resp).await.unwrap();
    resp
}

async fn send_post(addr: &str, path: &str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let req = format!("POST {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n", path, addr);
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut resp = String::new();
    stream.read_to_string(&mut resp).await.unwrap();
    resp
}

#[tokio::test]
#[timeout(30000)]
async fn test_chaos_hooks_and_api() {
    let addr = common::start_mock_server().await;
    let mut scenarios = HashMap::new();

    scenarios.insert(
        "chaos_test".to_string(),
        ScenarioDefinition {
            requests: vec![HTTPRequestDefinition::Detailed(Box::new(DetailedRequest {
                url: format!("http://{}/", addr),
                method: Some("GET".to_string()),
                ..Default::default()
            }))],
            weight: 1,
            think_time: None,
            data_sources: None,
            headers: None,
        },
    );

    // Setup temporary files for hook verification
    let prepare_file = "/tmp/chaos_prepare.txt";
    let startup_file = "/tmp/chaos_startup.txt";
    let shutdown_file = "/tmp/chaos_shutdown.txt";
    let sla_file = "/tmp/chaos_sla.txt";

    let _ = fs::remove_file(prepare_file);
    let _ = fs::remove_file(startup_file);
    let _ = fs::remove_file(shutdown_file);
    let _ = fs::remove_file(sla_file);

    let services = vec![ServiceDefinition {
        module: "shell".to_string(),
        prepare: vec![format!("echo 'prepare' > {}", prepare_file)],
        startup: vec![format!("echo 'startup' > {}", startup_file)],
        shutdown: vec![format!("echo 'shutdown' > {}", shutdown_file)],
    }];

    // API enabled on port 8123 (but disabled so it doesn't conflict)
    let api = Some(ApiConfig {
        enabled: false,
        port: 8123,
    });

    let config = Configuration {
        execution: vec![ExecutionPlan {
            concurrency: 1,
            ramp_up: "0s".to_string(),
            hold_for: "2s".to_string(),
            scenario: "chaos_test".to_string(),
            throughput: None,
            steps: None,
            pacing: None,
        }],
        scenarios,
        reporting: vec![
            bzt_rs::models::config::ReportingDefinition {
                module: "dummy".to_string(), // sets up real-time metrics without connecting to InfluxDB
                filename: None,
                url: None,
                token: None,
                org: None,
                bucket: None,
                interval: Some("1s".to_string()),
                failed_threshold: None,
                sla: vec![
                    SlaCriterion {
                        metric: SlaMetric::AvgResponseTime,
                        threshold: 0.001, // extremely low, guaranteed to breach
                        subject: None,
                        duration: None,
                        action: SlaAction::Exec(format!("echo 'sla_breached' > {}", sla_file)),
                    }
                ],
            }
        ],
        services,
        api,
    };

    // Run the attack!
    let run_res = goose::run_attack(config).await;
    assert!(run_res.is_ok());

    // Verify prepare, startup, and shutdown hooks executed
    assert!(Path::new(prepare_file).exists());
    assert!(Path::new(startup_file).exists());
    assert!(Path::new(shutdown_file).exists());

    // Verify prepare content
    let prepare_content = fs::read_to_string(prepare_file).unwrap();
    assert_eq!(prepare_content.trim(), "prepare");

    // Verify startup content
    let startup_content = fs::read_to_string(startup_file).unwrap();
    assert_eq!(startup_content.trim(), "startup");

    // Verify shutdown content
    let shutdown_content = fs::read_to_string(shutdown_file).unwrap();
    assert_eq!(shutdown_content.trim(), "shutdown");

    // Verify SLA action command was triggered
    assert!(Path::new(sla_file).exists());
    let sla_content = fs::read_to_string(sla_file).unwrap();
    assert_eq!(sla_content.trim(), "sla_breached");

    // Clean up
    let _ = fs::remove_file(prepare_file);
    let _ = fs::remove_file(startup_file);
    let _ = fs::remove_file(shutdown_file);
    let _ = fs::remove_file(sla_file);
}

#[tokio::test]
#[timeout(30000)]
async fn test_chaos_api_control() {
    let addr = common::start_mock_server().await;
    let mut scenarios = HashMap::new();

    scenarios.insert(
        "api_test".to_string(),
        ScenarioDefinition {
            requests: vec![HTTPRequestDefinition::Detailed(Box::new(DetailedRequest {
                url: format!("http://{}/", addr),
                method: Some("GET".to_string()),
                ..Default::default()
            }))],
            weight: 1,
            think_time: None,
            data_sources: None,
            headers: None,
        },
    );

    // API enabled on port 8124
    let api = Some(ApiConfig {
        enabled: true,
        port: 8124,
    });

    let config = Configuration {
        execution: vec![ExecutionPlan {
            concurrency: 1,
            ramp_up: "0s".to_string(),
            hold_for: "20s".to_string(), // hold for 20s, but we will abort early
            scenario: "api_test".to_string(),
            throughput: None,
            steps: None,
            pacing: None,
        }],
        scenarios,
        reporting: vec![],
        services: vec![],
        api,
    };

    // Run the attack in a background task
    let handle = tokio::spawn(async move {
        goose::run_attack(config).await
    });

    // Wait for the server to start up
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Check GET /metrics
    let metrics_resp = send_get("127.0.0.1:8124", "/metrics").await;
    assert!(metrics_resp.contains("endpoints"));

    // Check POST /control/stop
    let stop_resp = send_post("127.0.0.1:8124", "/control/stop").await;
    assert!(stop_resp.contains("200 OK"));

    // Ensure the attack stops immediately and returns a validation error representing SLA stop
    let run_res = handle.await.unwrap();
    assert!(run_res.is_err());
    let err = run_res.unwrap_err();
    assert!(matches!(err, bzt_rs::engine::BztError::SlaViolation { .. }));
}
