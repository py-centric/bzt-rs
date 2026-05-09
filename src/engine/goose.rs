use crate::engine::BztError;
use crate::engine::reporting::{CliSummary, JUnitReporter};
use crate::engine::sla::SlaEngine;
use crate::models::config::{Configuration, SlaAction, SlaCriterion, SlaMetric};
use crate::translator::StateTranslator;

pub async fn run_attack(config: Configuration) -> Result<(), BztError> {
    tracing::info!(
        "Starting load test with {} scenarios",
        config.scenarios.len()
    );
    let attack = StateTranslator::translate(&config, None)?;

    // FR-004: Enable HTML report generation via Goose environment variable
    for report_def in &config.reporting {
        if report_def.module == "html" {
            let filename = report_def.filename.as_deref().unwrap_or("report.html");
            unsafe {
                std::env::set_var("GOOSE_REPORT_FILE", filename);
            }
        }
    }

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
