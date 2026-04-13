use crate::engine::reporting::JUnitReporter;
use crate::models::config::Configuration;
use crate::translator::StateTranslator;

pub async fn run_attack(config: Configuration) -> Result<(), String> {
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

    let stats = attack.execute().await.map_err(|e| e.to_string())?;

    // REQ-6.1: Reporting (JUnit)
    for report_def in &config.reporting {
        if report_def.module == "junit-xml" {
            JUnitReporter::generate_report(report_def, &stats)?;
        }
    }

    // REQ-6.2: Pass/Fail Criteria (SLA enforcement)
    for report_def in &config.reporting {
        if let Some(threshold) = report_def.failed_threshold {
            let mut total_fails = 0;
            let mut total_reqs = 0;
            for req in stats.requests.values() {
                total_fails += req.fail_count;
                total_reqs += req.success_count + req.fail_count;
            }
            if total_reqs > 0 {
                let fail_rate = total_fails as f32 / total_reqs as f32;
                if fail_rate > threshold {
                    return Err(format!(
                        "SLA breached: fail rate {:.2}% > threshold {:.2}%",
                        fail_rate * 100.0,
                        threshold * 100.0
                    ));
                }
            }
        }
    }

    Ok(())
}
