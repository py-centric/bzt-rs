/// Parses a time string (e.g. "30s", "5000ms", "5m") into milliseconds.
/// Returns 0 if the string cannot be parsed.
#[must_use]
pub fn parse_time_to_ms(s: &str) -> u64 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }

    if s.ends_with("ms") {
        s.trim_end_matches("ms").parse::<u64>().unwrap_or(0)
    } else if s.ends_with('s') {
        s.trim_end_matches('s')
            .parse::<u64>()
            .map(|v| v * 1000)
            .unwrap_or(0)
    } else if s.ends_with('m') {
        s.trim_end_matches('m')
            .parse::<u64>()
            .map(|v| v * 60_000)
            .unwrap_or(0)
    } else {
        s.parse::<u64>().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_to_ms() {
        assert_eq!(parse_time_to_ms("30s"), 30_000);
        assert_eq!(parse_time_to_ms("500ms"), 500);
        assert_eq!(parse_time_to_ms("1m"), 60_000);
        assert_eq!(parse_time_to_ms("100"), 100);
        assert_eq!(parse_time_to_ms(""), 0);
        assert_eq!(parse_time_to_ms("invalid"), 0);
    }
}
