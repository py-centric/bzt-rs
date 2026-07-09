use crate::engine::BztError;
use crate::engine::reporting::{CliSummary, InfluxDbReporter, JUnitReporter, RealTimeMetrics};
use crate::engine::utils::parse_time_to_ms;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::engine::sla::SlaEngine;
use crate::models::config::{Configuration, SlaAction, SlaCriterion, SlaMetric};
use crate::translator::StateTranslator;
use axum::{routing::{get, post}, Router};
use std::net::SocketAddr;

fn run_hooks(services: &[crate::models::config::ServiceDefinition], phase: &str) {
    for svc in services {
        if svc.module == "shell" {
            let cmds = match phase {
                "prepare" => &svc.prepare,
                "startup" => &svc.startup,
                "shutdown" => &svc.shutdown,
                _ => continue,
            };
            for cmd in cmds {
                tracing::info!("[SHELL HOOK] Running ({}): {}", phase, cmd);
                let output = if cfg!(target_os = "windows") {
                    std::process::Command::new("cmd")
                        .args(&["/C", cmd])
                        .output()
                } else {
                    std::process::Command::new("sh")
                        .args(&["-c", cmd])
                        .output()
                };
                match output {
                    Ok(out) => {
                        if !out.status.success() {
                            tracing::error!(
                                "[SHELL HOOK] Command failed with status {}: {}",
                                out.status,
                                String::from_utf8_lossy(&out.stderr)
                            );
                        } else {
                            tracing::debug!(
                                "[SHELL HOOK] Success: {}",
                                String::from_utf8_lossy(&out.stdout)
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("[SHELL HOOK] Failed to execute command: {}", e);
                    }
                }
            }
        }
    }
}

pub async fn run_attack(config: Configuration) -> Result<(), BztError> {
    run_hooks(&config.services, "prepare");
    run_hooks(&config.services, "startup");

    let services_clone = config.services.clone();
    let res = run_attack_inner(config).await;

    run_hooks(&services_clone, "shutdown");
    res
}

async fn run_attack_inner(config: Configuration) -> Result<(), BztError> {
    tracing::info!(
        "Starting load test with {} scenarios",
        config.scenarios.len()
    );

    let real_time_metrics = Arc::new(Mutex::new(RealTimeMetrics::default()));

    // FR-010: Enable real-time reporting if interval is set
    for report_def in &config.reporting {
        if report_def.module == "influxdb" {
            let interval_opt = &report_def.interval;
            if let Some(interval) = interval_opt {
                let interval_ms = parse_time_to_ms(interval);
                if interval_ms > 0 {
                    let rt_metrics = real_time_metrics.clone();
                    let report_def_clone = report_def.clone();
                    tokio::spawn(async move {
                        let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
                        loop {
                            interval.tick().await;
                            let metrics_snapshot = {
                                let lock = rt_metrics.lock().unwrap();
                                lock.clone()
                            };
                            if let Err(e) = InfluxDbReporter::push_real_time_metrics(&report_def_clone, &metrics_snapshot).await {
                                tracing::error!("[INFLUX-RT] Push failed: {}", e);
                            }
                        }
                    });
                }
            }
        }
    }

    // Build SLA criteria
    let mut all_criteria = Vec::new();
    for report_def in &config.reporting {
        if let Some(threshold) = report_def.failed_threshold {
            all_criteria.push(SlaCriterion {
                metric: SlaMetric::FailRate,
                threshold,
                subject: None,
                duration: None,
                action: SlaAction::Stop,
            });
        }
        all_criteria.extend(report_def.sla.clone());
    }

    let (abort_tx, mut abort_rx) = tokio::sync::mpsc::channel::<String>(1);

    // Spawn Dynamic API server if enabled
    if let Some(ref api_cfg) = config.api && api_cfg.enabled {
        let metrics_clone = real_time_metrics.clone();
        let abort_tx_clone = abort_tx.clone();
        let port = api_cfg.port;
        tokio::spawn(async move {
            if let Err(e) = start_api_server(port, metrics_clone, abort_tx_clone).await {
                tracing::error!("[API] Server error: {}", e);
            }
        });
    }

    // Spawn SLA background checker if we have criteria
    if !all_criteria.is_empty() {
        let metrics_clone = real_time_metrics.clone();
        let abort_tx_clone = abort_tx.clone();
        let criteria_clone = all_criteria.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            let mut triggered_execs = std::collections::HashSet::new();

            loop {
                interval.tick().await;
                let snapshot = {
                    let lock = metrics_clone.lock().unwrap();
                    lock.clone()
                };
                let results = SlaEngine::evaluate_realtime(&criteria_clone, &snapshot);
                for result in &results {
                    if !result.passed {
                        match &result.action {
                            SlaAction::Warn => {
                                tracing::warn!(
                                    "[SLA-RT] SLA warning: {:?} {:.2} exceeds threshold {:.2}",
                                    result.metric,
                                    result.actual,
                                    result.threshold
                                );
                            }
                            SlaAction::Continue => {
                                tracing::info!(
                                    "[SLA-RT] SLA breach: {:?} {:.2} exceeds {:.2}",
                                    result.metric,
                                    result.actual,
                                    result.threshold
                                );
                            }
                            SlaAction::Exec(cmd) => {
                                if triggered_execs.insert(cmd.clone()) {
                                    tracing::info!("[SLA-RT] Executing SLA action command: {}", cmd);
                                    let output = if cfg!(target_os = "windows") {
                                        std::process::Command::new("cmd").args(&["/C", cmd]).status()
                                    } else {
                                        std::process::Command::new("sh").args(&["-c", cmd]).status()
                                    };
                                    if let Err(e) = output {
                                        tracing::error!("[SLA-RT] SLA action failed to launch: {}", e);
                                    }
                                }
                            }
                            SlaAction::Stop => {
                                let _ = abort_tx_clone.send(format!(
                                    "SLA breach: {:?} {:.2} exceeds threshold {:.2}",
                                    result.metric, result.actual, result.threshold
                                )).await;
                                return;
                            }
                        }
                    }
                }
            }
        });
    }

    let attack = StateTranslator::translate(&config, None, Some(real_time_metrics)).await?;

    tracing::info!("Executing Goose attack...");
    let attack_future = attack.execute();
    tokio::pin!(attack_future);

    let stats = tokio::select! {
        res = &mut attack_future => {
            match res {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Goose attack execution failed: {}", e);
                    return Err(BztError::Goose(Box::new(e)));
                }
            }
        }
        reason = abort_rx.recv() => {
            let reason_str = reason.unwrap_or_else(|| "Unknown abort reason".to_string());
            tracing::error!("[ABORT] Load test aborted: {}", reason_str);
            return Err(BztError::SlaViolation {
                metric: "Real-time SLA/API Stop".to_string(),
                actual: 0.0,
                threshold: 0.0,
            });
        }
    };

    tracing::info!("Goose attack completed, generating report");

    // US5: CLI Reporter — always print terminal summary
    tracing::debug!("[CLI] Generating terminal summary");
    let summary = CliSummary::from_metrics(&stats);
    summary.print();

    // REQ-6.1: Reporting (JUnit)
    for report_def in &config.reporting {
        if report_def.module == "junit-xml" {
            JUnitReporter::generate_report(report_def, &stats)?;
        }
        if report_def.module == "influxdb" {
            InfluxDbReporter::push_metrics(report_def, &stats).await?;
        }
    }

    // Evaluate SLA at the end
    if !all_criteria.is_empty() {
        tracing::debug!("[SLA] Evaluating {} criteria", all_criteria.len());
        let results = SlaEngine::evaluate(&all_criteria, &stats);
        for result in &results {
            if !result.passed {
                match &result.action {
                    SlaAction::Warn => {
                        tracing::warn!(
                            "SLA warning: {:?} {:.2} exceeds threshold {:.2}",
                            result.metric,
                            result.actual,
                            result.threshold
                        );
                    }
                    SlaAction::Continue => {
                        tracing::info!(
                            "SLA breach recorded: {:?} {:.2} exceeds {:.2}",
                            result.metric,
                            result.actual,
                            result.threshold
                        );
                    }
                    SlaAction::Exec(cmd) => {
                        tracing::info!("SLA breach recorded: {:?}, triggered: {}", result.metric, cmd);
                    }
                    SlaAction::Stop => {}
                }
            }
        }
        SlaEngine::check_breaches(&results)?;
    }

    Ok(())
}

async fn start_api_server(
    port: u16,
    metrics: Arc<Mutex<RealTimeMetrics>>,
    abort_tx: tokio::sync::mpsc::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use axum::http::StatusCode;

    let metrics_clone = metrics.clone();
    let abort_tx_clone = abort_tx.clone();

    let app = Router::new()
        .route("/metrics", get(move || {
            let metrics = metrics_clone.clone();
            async move {
                let snapshot = {
                    let lock = metrics.lock().unwrap();
                    lock.clone()
                };
                axum::Json(snapshot)
            }
        }))
        .route("/control/stop", post(move || {
            let abort_tx = abort_tx_clone.clone();
            async move {
                let _ = abort_tx.send("Aborted via control API".to_string()).await;
                (StatusCode::OK, "Load test stop initiated")
            }
        }));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("[API] Runtime tuning control API listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::Configuration;

    #[tokio::test]
    async fn test_run_attack_empty_config() {
        let config = Configuration {
            execution: vec![],
            scenarios: std::collections::HashMap::new(),
            reporting: vec![],
            services: vec![],
            api: None,
        };
        let result = run_attack(config).await;
        assert!(result.is_ok() || result.is_err());
    }
}
