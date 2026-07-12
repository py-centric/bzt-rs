use crate::engine::reporting::RealTimeMetrics;
use std::sync::{Arc, RwLock};

/// Records request metrics for real-time reporting.
///
/// Safely locks the metrics, updates endpoint stats (count, timing, failures),
/// and returns silently if metrics are unavailable or the lock is poisoned.
pub(crate) fn record_request(
    metrics: Option<&Arc<RwLock<RealTimeMetrics>>>,
    endpoint_name: &str,
    duration_ms: usize,
    success: bool,
) {
    if let Some(rtm) = metrics
        && let Ok(mut lock) = rtm.write()
    {
        let stats = lock
            .endpoints
            .entry(endpoint_name.to_string())
            .or_default();
        stats.count += 1;
        stats.total_time_ms += duration_ms;
        stats.push_time(duration_ms);
        if !success {
            stats.failures += 1;
        }
    }
}
