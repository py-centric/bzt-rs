#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use crate::engine::BztError;
use crate::models::config::ReportingDefinition;
use goose::metrics::GooseMetrics;
#[cfg(feature = "influxdb-reporter")]
use influxdb::{Client, InfluxDbWriteable};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::Write;
#[cfg(feature = "influxdb-reporter")]
use std::sync::OnceLock;
#[cfg(feature = "influxdb-reporter")]
use uuid::Uuid;

/// Maximum number of timing samples to retain per endpoint.
/// Prevents unbounded memory growth under high request rates (e.g. 10K RPS for 10 min = 6M entries).
/// When the cap is reached, the oldest half of entries are dropped.
const MAX_TIMINGS: usize = 10_000;

/// Trait abstraction for report generators.
///
/// Each reporter identifies itself by module name and produces a report
/// from aggregate metrics. The dispatch in [`crate::engine::goose`] uses
/// this trait to avoid string-matching on module names at the call site.
pub trait Reporter: Send + Sync {
    /// Returns the module identifier this reporter handles
    /// (e.g. `"junit-xml"`, `"influxdb"`).
    fn module_name(&self) -> &str;

    /// Produce the report for the given reporting definition and metrics.
    ///
    /// # Errors
    ///
    /// Returns `BztError` if report generation or writing fails.
    fn report(
        &self,
        definition: &ReportingDefinition,
        stats: &GooseMetrics,
    ) -> Result<(), BztError>;
}

#[cfg(feature = "influxdb-reporter")]
static WORKER_ID: OnceLock<String> = OnceLock::new();

#[cfg(feature = "influxdb-reporter")]
fn get_worker_id() -> &'static str {
    WORKER_ID.get_or_init(|| Uuid::new_v4().to_string())
}

/// Escapes a string for safe inclusion in XML text content and attributes.
/// Prevents XML injection from user-controlled request names, methods, etc.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub struct JUnitReporter;

impl JUnitReporter {
    #[allow(clippy::missing_errors_doc)]
    pub fn generate_report(
        reporting: &ReportingDefinition,
        stats: &GooseMetrics,
    ) -> Result<(), BztError> {
        if reporting.module != "junit-xml" {
            return Ok(());
        }

        let filename = reporting.filename.as_deref().unwrap_or("junit.xml");
        let mut file = File::create(filename)?;

        let total_requests = stats.requests.len();
        let mut failures = 0;
        for request in stats.requests.values() {
            failures += request.fail_count;
        }

        writeln!(file, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")
            .map_err(|e| BztError::Internal(e.to_string()))?;
        writeln!(file, "<testsuites>").map_err(|e| BztError::Internal(e.to_string()))?;
        writeln!(
            file,
            "  <testsuite name=\"bzt-rs\" tests=\"{total_requests}\" failures=\"{failures}\">"
        )
        .map_err(|e| BztError::Internal(e.to_string()))?;

        for (name, request) in &stats.requests {
            let avg_time = if request.raw_data.counter > 0 {
                request.raw_data.total_time as f32 / request.raw_data.counter as f32
            } else {
                0.0
            };
            // Security: escape request names to prevent XML injection
            let safe_name = xml_escape(name);
            let safe_method = xml_escape(&format!("{:?}", request.method).to_uppercase());
            writeln!(
                file,
                "    <testcase name=\"{}\" time=\"{:.3}\">",
                safe_name,
                avg_time / 1000.0
            )
            .map_err(|e| BztError::Internal(e.to_string()))?;
            if request.fail_count > 0 {
                writeln!(
                    file,
                    "      <failure message=\"Request failed: {safe_method} {safe_name}\" type=\"Error\" />",
                )
                .map_err(|e| BztError::Internal(e.to_string()))?;
            }
            writeln!(file, "    </testcase>").map_err(|e| BztError::Internal(e.to_string()))?;
        }

        writeln!(file, "  </testsuite>").map_err(|e| BztError::Internal(e.to_string()))?;
        writeln!(file, "</testsuites>").map_err(|e| BztError::Internal(e.to_string()))?;

        Ok(())
    }
}

impl Reporter for JUnitReporter {
    fn module_name(&self) -> &'static str {
        "junit-xml"
    }

    fn report(
        &self,
        definition: &ReportingDefinition,
        stats: &GooseMetrics,
    ) -> Result<(), BztError> {
        Self::generate_report(definition, stats)
    }
}

#[cfg(feature = "influxdb-reporter")]
pub struct InfluxDbReporter;

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct RealTimeEndpointStats {
    pub count: usize,
    pub failures: usize,
    pub total_time_ms: usize,
    pub times: Vec<usize>,
}

impl RealTimeEndpointStats {
    /// Push a timing sample, capping at `MAX_TIMINGS` to prevent memory leaks.
    /// When the cap is reached, the oldest half of entries are dropped.
    pub fn push_time(&mut self, time: usize) {
        if self.times.len() >= MAX_TIMINGS {
            // Compact: drop the oldest half to amortize the cost.
            // At 10K RPS this triggers once every 0.5 s and copies ~40 KB.
            self.times.drain(..MAX_TIMINGS / 2);
        }
        self.times.push(time);
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RealTimeMetrics {
    pub endpoints: HashMap<String, RealTimeEndpointStats>,
    #[serde(skip)]
    pub start_time: std::time::Instant,
}

impl Default for RealTimeMetrics {
    fn default() -> Self {
        Self {
            endpoints: HashMap::new(),
            start_time: std::time::Instant::now(),
        }
    }
}

#[cfg(feature = "influxdb-reporter")]
impl InfluxDbReporter {
    #[allow(clippy::missing_errors_doc)]
    pub async fn push_metrics(
        reporting: &ReportingDefinition,
        stats: &GooseMetrics,
    ) -> Result<(), BztError> {
        if reporting.module != "influxdb" {
            return Ok(());
        }

        let url = reporting.url.as_deref().unwrap_or("http://localhost:8086");
        let bucket = reporting.bucket.as_deref().unwrap_or("bzt");
        let token = reporting.token.as_deref().unwrap_or("");

        let client = Client::new(url, bucket).with_token(token);
        let points = Self::generate_points(stats)?;
        let point_count = points.len();

        for point in points {
            client
                .query(point)
                .await
                .map_err(|e| BztError::Internal(format!("InfluxDB push failed: {e}")))?;
        }

        tracing::info!("[INFLUX] Pushed {} points to {}", point_count, url);
        Ok(())
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn push_real_time_metrics(
        reporting: &ReportingDefinition,
        metrics: &RealTimeMetrics,
    ) -> Result<(), BztError> {
        if reporting.module != "influxdb" {
            return Ok(());
        }

        let url = reporting.url.as_deref().unwrap_or("http://localhost:8086");
        let bucket = reporting.bucket.as_deref().unwrap_or("bzt");
        let token = reporting.token.as_deref().unwrap_or("");

        let client = Client::new(url, bucket).with_token(token);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let timestamp = influxdb::Timestamp::Milliseconds(now);

        let mut points = Vec::new();
        for (name, stats) in &metrics.endpoints {
            if stats.count == 0 {
                continue;
            }

            let avg_time = stats.total_time_ms as f64 / stats.count as f64;
            let mut sorted_times = stats.times.clone();
            sorted_times.sort_unstable();

            let p95 = if sorted_times.is_empty() {
                0.0
            } else {
                let val: f64 = 0.95 * (sorted_times.len() as f64 - 1.0);
                let idx = val.round() as usize;
                sorted_times[idx] as f32
            };

            let point = timestamp
                .try_into_query("request_metrics_realtime")
                .map_err(|e| BztError::Internal(format!("InfluxDB query build failed: {e}")))?
                .add_tag("path", name.clone())
                .add_tag("worker_id", get_worker_id())
                .add_field("count", stats.count as i64)
                .add_field("failures", stats.failures as i64)
                .add_field("avg_ms", avg_time)
                .add_field("p95_ms", p95);
            points.push(point);
        }

        for point in points {
            client
                .query(point)
                .await
                .map_err(|e| BztError::Internal(format!("InfluxDB real-time push failed: {e}")))?;
        }

        Ok(())
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn generate_points(stats: &GooseMetrics) -> Result<Vec<influxdb::WriteQuery>, BztError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let timestamp = influxdb::Timestamp::Milliseconds(now);

        let mut points = Vec::new();
        for (name, agg) in &stats.requests {
            let count = agg.raw_data.counter;
            let avg_time = if count > 0 {
                agg.raw_data.total_time as f64 / count as f64
            } else {
                0.0
            };

            let point = timestamp
                .try_into_query("request_metrics")
                .map_err(|e| BztError::Internal(format!("InfluxDB query build failed: {e}")))?
                .add_tag("path", name.clone())
                .add_tag("method", format!("{:?}", agg.method).to_uppercase())
                .add_tag("worker_id", get_worker_id())
                .add_field("count", count as i64)
                .add_field("failures", agg.fail_count as i64)
                .add_field("avg_ms", avg_time)
                .add_field("p95_ms", percentile(&agg.raw_data.times, 95.0))
                .add_field("p99_ms", percentile(&agg.raw_data.times, 99.0));
            points.push(point);
        }

        // Add a global point
        let global_point = timestamp
            .try_into_query("test_summary")
            .map_err(|e| BztError::Internal(format!("InfluxDB query build failed: {e}")))?
            .add_tag("worker_id", get_worker_id())
            .add_field(
                "total_requests",
                stats
                    .requests
                    .values()
                    .map(|r| r.raw_data.counter)
                    .sum::<usize>() as i64,
            )
            .add_field(
                "total_failures",
                stats.requests.values().map(|r| r.fail_count).sum::<usize>() as i64,
            )
            .add_field("duration_secs", stats.duration as i64)
            .add_field("max_users", stats.maximum_users as i64);
        points.push(global_point);

        Ok(points)
    }
}

#[cfg(feature = "influxdb-reporter")]
impl Reporter for InfluxDbReporter {
    fn module_name(&self) -> &'static str {
        "influxdb"
    }

    fn report(
        &self,
        definition: &ReportingDefinition,
        stats: &GooseMetrics,
    ) -> Result<(), BztError> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(Self::push_metrics(definition, stats))
        })
    }
}

/// Computed summary data for a single endpoint.
#[derive(Debug, Default)]
pub struct EndpointSummary {
    pub method: String,
    pub path: String,
    pub count: usize,
    pub failures: usize,
    pub avg_response_time_ms: f64,
    pub min_response_time_ms: usize,
    pub max_response_time_ms: usize,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub requests_per_sec: f64,
}

/// Terminal-friendly summary of a completed load test.
#[derive(Debug, Default)]
pub struct CliSummary {
    pub total_requests: usize,
    pub total_failures: usize,
    pub duration_secs: usize,
    pub max_users: usize,
    pub endpoints: Vec<EndpointSummary>,
}

/// Computes the p-th percentile from a `BTreeMap` of response time -> count.
/// Returns the response time value at the given percentile.
#[must_use]
pub fn percentile(times: &BTreeMap<usize, usize>, p: f64) -> f64 {
    if times.is_empty() {
        return 0.0;
    }
    let total: usize = times.values().sum();
    if total == 0 {
        return 0.0;
    }
    let target = (total as f64 * p / 100.0).ceil() as usize;
    let mut cumulative = 0;
    for (time, count) in times {
        cumulative += count;
        if cumulative >= target {
            return *time as f64;
        }
    }
    *times.keys().last().unwrap_or(&0) as f64
}

impl CliSummary {
    /// Computes a [`CliSummary`] from [`GooseMetrics`] after a test run.
    #[must_use]
    pub fn from_metrics(metrics: &GooseMetrics) -> Self {
        tracing::debug!(
            "[CLI] Computing summary from {} endpoints, duration={}s",
            metrics.requests.len(),
            metrics.duration
        );
        let mut total_requests = 0;
        let mut total_failures = 0;
        let mut endpoints = Vec::new();

        for (name, agg) in &metrics.requests {
            let count = agg.raw_data.counter;
            let failures = agg.fail_count;
            total_requests += count;
            total_failures += failures;

            let avg_time = if count > 0 {
                agg.raw_data.total_time as f64 / count as f64
            } else {
                0.0
            };

            let rps = if metrics.duration > 0 {
                count as f64 / metrics.duration as f64
            } else {
                0.0
            };

            endpoints.push(EndpointSummary {
                method: format!("{:?}", agg.method).to_uppercase(),
                path: if name.len() > 40 {
                    format!("{}…", &name[..37])
                } else {
                    name.clone()
                },
                count,
                failures,
                avg_response_time_ms: avg_time,
                min_response_time_ms: agg.raw_data.minimum_time,
                max_response_time_ms: agg.raw_data.maximum_time,
                p95_response_time_ms: percentile(&agg.raw_data.times, 95.0),
                p99_response_time_ms: percentile(&agg.raw_data.times, 99.0),
                requests_per_sec: rps,
            });
        }

        endpoints.sort_by_key(|b| std::cmp::Reverse(b.count));

        CliSummary {
            total_requests,
            total_failures,
            duration_secs: metrics.duration,
            max_users: metrics.maximum_users,
            endpoints,
        }
    }

    /// Formats the summary as an ASCII table printed to stdout.
    pub fn print(&self) {
        tracing::debug!("[CLI] Printing summary: {} endpoints", self.endpoints.len());
        println!("\n{}", "=".repeat(90));
        println!("  BZT-RS LOAD TEST SUMMARY");
        println!("{}", "=".repeat(90));
        println!(
            "  Duration: {}s  |  Max Users: {}  |  Total Requests: {}  |  Failures: {} ({:.1}%)",
            self.duration_secs,
            self.max_users,
            self.total_requests,
            self.total_failures,
            if self.total_requests > 0 {
                self.total_failures as f64 / self.total_requests as f64 * 100.0
            } else {
                0.0
            },
        );
        println!("{}", "-".repeat(90));
        println!(
            "  {:<8} {:<40} {:>8} {:>8} {:>10} {:>10} {:>10}",
            "Method", "Path", "Requests", "Fails", "Avg(ms)", "p95(ms)", "p99(ms)"
        );
        println!("{}", "-".repeat(90));
        for ep in &self.endpoints {
            println!(
                "  {:<8} {:<40} {:>8} {:>8} {:>10.1} {:>10.1} {:>10.1}",
                ep.method,
                if ep.path.len() > 40 {
                    format!("{}…", &ep.path[..37])
                } else {
                    ep.path.clone()
                },
                ep.count,
                ep.failures,
                ep.avg_response_time_ms,
                ep.p95_response_time_ms,
                ep.p99_response_time_ms,
            );
        }
        println!("{}", "-".repeat(90));
        println!();
    }
}

impl Reporter for CliSummary {
    fn module_name(&self) -> &'static str {
        "cli-summary"
    }

    fn report(
        &self,
        _definition: &ReportingDefinition,
        stats: &GooseMetrics,
    ) -> Result<(), BztError> {
        let summary = Self::from_metrics(stats);
        summary.print();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose::metrics::{GooseRequestMetricAggregate, GooseRequestMetricTimingData};
    use goose::prelude::GooseMethod;
    use std::collections::HashMap;

    // --- xml_escape tests ---

    #[test]
    fn test_xml_escape_ampersand() {
        assert_eq!(xml_escape("a&b"), "a&amp;b");
    }

    #[test]
    fn test_xml_escape_less_than() {
        assert_eq!(xml_escape("a<b"), "a&lt;b");
    }

    #[test]
    fn test_xml_escape_greater_than() {
        assert_eq!(xml_escape("a>b"), "a&gt;b");
    }

    #[test]
    fn test_xml_escape_double_quote() {
        assert_eq!(xml_escape(r#"a"b"#), "a&quot;b");
    }

    #[test]
    fn test_xml_escape_single_quote() {
        assert_eq!(xml_escape("a'b"), "a&apos;b");
    }

    #[test]
    fn test_xml_escape_all_special_chars() {
        let input = "a&b<c>d\"e'f";
        let expected = "a&amp;b&lt;c&gt;d&quot;e&apos;f";
        assert_eq!(xml_escape(input), expected);
    }

    #[test]
    fn test_xml_escape_empty_string() {
        assert_eq!(xml_escape(""), "");
    }

    #[test]
    fn test_xml_escape_no_special_chars() {
        assert_eq!(xml_escape("Hello World 123"), "Hello World 123");
    }

    #[test]
    fn test_xml_escape_multiple_ampersands() {
        assert_eq!(xml_escape("&&"), "&amp;&amp;");
    }

    #[test]
    fn test_xml_escape_xml_injection_prevention() {
        let malicious = "<script>alert('xss')</script>";
        let escaped = xml_escape(malicious);
        assert!(!escaped.contains("<script>"));
        assert!(escaped.contains("&lt;script&gt;"));
    }

    fn make_timing(times: Vec<(usize, usize)>) -> GooseRequestMetricTimingData {
        let mut data = GooseRequestMetricTimingData {
            times: BTreeMap::new(),
            minimum_time: usize::MAX,
            maximum_time: 0,
            total_time: 0,
            counter: 0,
        };
        let mut total = 0usize;
        let mut count = 0usize;
        for (time, cnt) in &times {
            data.times.insert(*time, *cnt);
            total += time * cnt;
            count += cnt;
        }
        data.total_time = total;
        data.counter = count;
        data.minimum_time = *times.first().map(|(t, _)| t).unwrap_or(&0);
        data.maximum_time = *times.last().map(|(t, _)| t).unwrap_or(&0);
        data
    }

    fn make_aggregate(
        name: &str,
        method: GooseMethod,
        success: usize,
        fail: usize,
        timing: GooseRequestMetricTimingData,
    ) -> (String, GooseRequestMetricAggregate) {
        let agg = GooseRequestMetricAggregate {
            path: name.to_string(),
            method,
            raw_data: timing,
            coordinated_omission_data: None,
            status_code_counts: HashMap::new(),
            success_count: success,
            fail_count: fail,
            load_test_hash: 0,
        };
        (name.to_string(), agg)
    }

    fn make_metrics(endpoints: Vec<(String, GooseRequestMetricAggregate)>) -> GooseMetrics {
        let mut requests = HashMap::new();
        for (name, agg) in endpoints {
            requests.insert(name, agg);
        }
        let mut metrics = GooseMetrics::default();
        metrics.duration = 10;
        metrics.maximum_users = 5;
        metrics.total_users = 5;
        metrics.requests = requests;
        metrics
    }

    #[test]
    fn test_percentile_empty() {
        let times = BTreeMap::new();
        assert_eq!(percentile(&times, 95.0), 0.0);
    }

    #[test]
    fn test_percentile_exact() {
        let mut times = BTreeMap::new();
        times.insert(100, 10); // 10 requests at 100ms
        times.insert(200, 10); // 10 requests at 200ms
        times.insert(300, 10); // 10 requests at 300ms
        // 30 total, p95 target = 29th request -> highest value
        assert!((percentile(&times, 95.0) - 300.0).abs() < 0.1);
    }

    #[test]
    fn test_percentile_single_value() {
        let mut times = BTreeMap::new();
        times.insert(42, 100);
        assert!((percentile(&times, 95.0) - 42.0).abs() < 0.1);
        assert!((percentile(&times, 99.0) - 42.0).abs() < 0.1);
    }

    #[test]
    fn test_cli_summary_computation() {
        let timing = make_timing(vec![(50, 5), (100, 5)]);
        let (name, agg) = make_aggregate("/api/test", GooseMethod::Get, 10, 0, timing);
        let metrics = make_metrics(vec![(name, agg)]);

        let summary = CliSummary::from_metrics(&metrics);
        assert_eq!(summary.total_requests, 10);
        assert_eq!(summary.total_failures, 0);
        assert_eq!(summary.duration_secs, 10);
        assert_eq!(summary.max_users, 5);
        assert_eq!(summary.endpoints.len(), 1);
        assert!((summary.endpoints[0].avg_response_time_ms - 75.0).abs() < 0.1);
    }

    #[test]
    fn test_cli_summary_with_failures() {
        let timing = make_timing(vec![(100, 20)]);
        let (name, agg) = make_aggregate("/api/fail", GooseMethod::Post, 15, 5, timing);
        let metrics = make_metrics(vec![(name, agg)]);

        let summary = CliSummary::from_metrics(&metrics);
        assert_eq!(summary.total_requests, 20);
        assert_eq!(summary.total_failures, 5);
        assert_eq!(summary.endpoints[0].failures, 5);
    }

    #[test]
    fn test_cli_summary_empty() {
        let metrics = make_metrics(vec![]);
        let summary = CliSummary::from_metrics(&metrics);
        assert_eq!(summary.total_requests, 0);
        assert_eq!(summary.total_failures, 0);
        assert!(summary.endpoints.is_empty());
    }

    #[cfg(feature = "influxdb-reporter")]
    #[test]
    fn test_influxdb_point_generation() {
        let timing = make_timing(vec![(100, 10)]);
        let (name, agg) = make_aggregate("/api/influx", GooseMethod::Get, 10, 0, timing);
        let metrics = make_metrics(vec![(name, agg)]);

        let points = InfluxDbReporter::generate_points(&metrics).unwrap();
        // Expect one per endpoint + one global
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn test_percentile_zero_total_count() {
        // BTreeMap with entries that all have 0 count
        let mut times = BTreeMap::new();
        times.insert(100, 0);
        times.insert(200, 0);
        // total = 0, so percentile should return 0.0
        assert_eq!(percentile(&times, 95.0), 0.0);
    }

    #[test]
    fn test_percentile_p50() {
        let mut times = BTreeMap::new();
        times.insert(10, 5);
        times.insert(20, 5);
        // 10 total, p50 target = 5th request -> should be 10
        assert!((percentile(&times, 50.0) - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_cli_summary_long_path_truncation() {
        let long_path = "/api/v1/users/".repeat(5); // 75 chars
        let timing = make_timing(vec![(100, 1)]);
        let (name, agg) = make_aggregate(&long_path, GooseMethod::Get, 1, 0, timing);
        let metrics = make_metrics(vec![(name, agg)]);
        let summary = CliSummary::from_metrics(&metrics);
        // Path should be truncated to 39 chars + ellipsis
        assert!(summary.endpoints[0].path.len() <= 40);
    }

    #[test]
    fn test_cli_summary_multiple_endpoints_sorted_by_count() {
        let t1 = make_timing(vec![(50, 5)]);
        let t2 = make_timing(vec![(100, 20)]);
        let (n1, a1) = make_aggregate("/api/low", GooseMethod::Get, 5, 0, t1);
        let (n2, a2) = make_aggregate("/api/high", GooseMethod::Post, 20, 0, t2);
        let metrics = make_metrics(vec![(n1, a1), (n2, a2)]);
        let summary = CliSummary::from_metrics(&metrics);
        // Should be sorted by count descending
        assert_eq!(summary.endpoints[0].path, "/api/high");
        assert_eq!(summary.endpoints[1].path, "/api/low");
    }

    #[cfg(feature = "influxdb-reporter")]
    #[test]
    fn test_influxdb_multiple_endpoints() {
        let t1 = make_timing(vec![(100, 5)]);
        let t2 = make_timing(vec![(200, 3)]);
        let (n1, a1) = make_aggregate("/api/a", GooseMethod::Get, 5, 0, t1);
        let (n2, a2) = make_aggregate("/api/b", GooseMethod::Post, 3, 0, t2);
        let metrics = make_metrics(vec![(n1, a1), (n2, a2)]);
        let points = InfluxDbReporter::generate_points(&metrics).unwrap();
        // 2 endpoints + 1 global = 3
        assert_eq!(points.len(), 3);
    }

    #[test]
    fn test_real_time_metrics_default() {
        let metrics = RealTimeMetrics::default();
        assert!(metrics.endpoints.is_empty());
        // start_time should be approximately now
        assert!(metrics.start_time.elapsed().as_secs() < 5);
    }

    #[test]
    fn test_real_time_endpoint_stats_default() {
        let stats = RealTimeEndpointStats::default();
        assert_eq!(stats.count, 0);
        assert_eq!(stats.failures, 0);
        assert_eq!(stats.total_time_ms, 0);
        assert!(stats.times.is_empty());
    }
}
