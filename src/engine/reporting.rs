use crate::models::config::ReportingDefinition;
use std::fs::File;
use std::io::Write;

pub struct JUnitReporter;

impl JUnitReporter {
    pub fn generate_report(
        reporting: &ReportingDefinition,
        stats: &goose::metrics::GooseMetrics,
    ) -> Result<(), String> {
        if reporting.module != "junit-xml" {
            return Ok(());
        }

        let filename = reporting.filename.as_deref().unwrap_or("junit.xml");
        let mut file = File::create(filename).map_err(|e| e.to_string())?;

        let total_requests = stats.requests.len();
        let mut failures = 0;
        for request in stats.requests.values() {
            failures += request.fail_count;
        }

        writeln!(file, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>").unwrap();
        writeln!(file, "<testsuites>").unwrap();
        writeln!(
            file,
            "  <testsuite name=\"bzt-rs\" tests=\"{}\" failures=\"{}\">",
            total_requests, failures
        )
        .unwrap();

        for (name, request) in &stats.requests {
            let avg_time = if request.raw_data.counter > 0 {
                request.raw_data.total_time as f32 / request.raw_data.counter as f32
            } else {
                0.0
            };
            writeln!(
                file,
                "    <testcase name=\"{}\" time=\"{:.3}\">",
                name,
                avg_time / 1000.0
            )
            .unwrap();
            if request.fail_count > 0 {
                writeln!(
                    file,
                    "      <failure message=\"Request failed\" type=\"Error\" />"
                )
                .unwrap();
            }
            writeln!(file, "    </testcase>").unwrap();
        }

        writeln!(file, "  </testsuite>").unwrap();
        writeln!(file, "</testsuites>").unwrap();

        Ok(())
    }
}
