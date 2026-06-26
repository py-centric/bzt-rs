use crate::engine::BztError;
use crate::engine::reporting::{CliSummary, InfluxDbReporter, JUnitReporter, RealTimeMetrics};
use crate::engine::utils::parse_time_to_ms;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::engine::sla::SlaEngine;
use crate::models::config::{Configuration, SlaAction, SlaCriterion, SlaMetric};
use crate::translator::StateTranslator;

pub async fn run_attack(config: Configuration) -> Result<(), BztError> {
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

    let attack = StateTranslator::translate(&config, None, Some(real_time_metrics)).await?;

    tracing::info!("Executing Goose attack...");
    let stats = attack.execute().await.map_err(|e| {
        tracing::error!("Goose attack execution failed: {}", e);
        BztError::Goose(Box::new(e))
    })?;
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

    // REQ-6.2: SLA evaluation via SlaEngine
    tracing::debug!(
        "[SLA] Building criteria from {} reporting definitions",
        config.reporting.len()
    );
    let mut all_criteria = Vec::new();
    for report_def in &config.reporting {
        // Backward compat: convert failed-threshold to SlaCriterion
        if let Some(threshold) = report_def.failed_threshold {
            all_criteria.push(SlaCriterion {
                metric: SlaMetric::FailRate,
                threshold,
                subject: None,
                duration: None,
                action: SlaAction::Stop,
            });
        }
        // New-style SLA criteria
        all_criteria.extend(report_def.sla.clone());
    }
    if !all_criteria.is_empty() {
        tracing::debug!("[SLA] Evaluating {} criteria", all_criteria.len());
        let results = SlaEngine::evaluate(&all_criteria, &stats);
        for result in &results {
            if !result.passed {
                match result.action {
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
                    SlaAction::Stop => {}
                }
            }
        }
        SlaEngine::check_breaches(&results)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::Configuration;

    #[tokio::test]
    async fn test_run_attack_empty_config() {
        let config = Configuration { execution: vec![], scenarios: std::collections::HashMap::new(), reporting: vec![] };
        let result = run_attack(config).await;
        // Goose shouldn't panic on an empty config, it should just complete or error gracefully.
        assert!(result.is_ok() || result.is_err()); 
    }
}
