#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]

use crate::engine::utils::parse_time_to_ms;
use crate::models::config::PacingConfig;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct PacingEngine;

impl PacingEngine {
    pub fn calculate_delay(config: &PacingConfig) -> Duration {
        let period_ms = parse_time_to_ms(&config.per);
        if config.rate == 0 {
            tracing::debug!("[PACING] Rate=0, returning zero delay");
            return Duration::from_millis(0);
        }
        let base_delay_ms = period_ms as f64 / config.rate as f64;
        let delay = if config.randomize {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos();
            let r = nanos as f64 / 1_000_000_000.0;
            let exp_delay = -base_delay_ms * (1.0 - r).ln();
            Duration::from_millis(exp_delay as u64).clamp(
                Duration::from_millis(1),
                Duration::from_millis(base_delay_ms as u64 * 10),
            )
        } else {
            Duration::from_millis(base_delay_ms as u64)
        };
        tracing::debug!(
            "[PACING] rate={}/{} randomize={} delay={:?}",
            config.rate,
            config.per,
            config.randomize,
            delay
        );
        delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_delay_fixed_rate() {
        let config = PacingConfig {
            rate: 10,
            per: "1s".to_string(),
            randomize: false,
        };
        let delay = PacingEngine::calculate_delay(&config);
        assert_eq!(delay.as_millis(), 100);
    }

    #[test]
    fn test_calculate_delay_rate_zero() {
        let config = PacingConfig {
            rate: 0,
            per: "1s".to_string(),
            randomize: false,
        };
        let delay = PacingEngine::calculate_delay(&config);
        assert_eq!(delay.as_millis(), 0);
    }

    #[test]
    fn test_calculate_delay_per_minute() {
        let config = PacingConfig {
            rate: 60,
            per: "1m".to_string(),
            randomize: false,
        };
        let delay = PacingEngine::calculate_delay(&config);
        assert_eq!(delay.as_millis(), 1000);
    }

    #[test]
    fn test_calculate_delay_randomized_nonzero() {
        let config = PacingConfig {
            rate: 10,
            per: "1s".to_string(),
            randomize: true,
        };
        let delay = PacingEngine::calculate_delay(&config);
        assert!(delay.as_millis() > 0);
    }
}
