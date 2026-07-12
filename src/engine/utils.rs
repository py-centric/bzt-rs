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
            .map_or(0, |v| v * 1000)
    } else if s.ends_with('m') {
        s.trim_end_matches('m')
            .parse::<u64>()
            .map_or(0, |v| v * 60_000)
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

    #[test]
    fn test_parse_time_zero_values() {
        assert_eq!(parse_time_to_ms("0s"), 0);
        assert_eq!(parse_time_to_ms("0ms"), 0);
        assert_eq!(parse_time_to_ms("0m"), 0);
        assert_eq!(parse_time_to_ms("0"), 0);
    }

    #[test]
    fn test_parse_time_large_values() {
        assert_eq!(parse_time_to_ms("999999s"), 999_999_000);
        assert_eq!(parse_time_to_ms("999999m"), 59_999_940_000);
        assert_eq!(parse_time_to_ms("999999ms"), 999_999);
    }

    #[test]
    fn test_parse_time_whitespace_handling() {
        assert_eq!(parse_time_to_ms(" 30s "), 30_000);
        assert_eq!(parse_time_to_ms("\t5m\t"), 300_000);
        assert_eq!(parse_time_to_ms("  100ms  "), 100);
    }

    #[test]
    fn test_parse_time_unsupported_suffixes() {
        // "h" and "d" are not supported — should fall through to raw parse
        assert_eq!(parse_time_to_ms("1h"), 0); // "1h" can't parse as u64
        assert_eq!(parse_time_to_ms("1d"), 0); // "1d" can't parse as u64
    }

    #[test]
    fn test_parse_time_boundary_values() {
        assert_eq!(parse_time_to_ms("1ms"), 1);
        assert_eq!(parse_time_to_ms("1s"), 1_000);
        assert_eq!(parse_time_to_ms("1m"), 60_000);
    }

    #[test]
    fn test_parse_time_non_numeric_after_suffix() {
        assert_eq!(parse_time_to_ms("abc s"), 0);
        assert_eq!(parse_time_to_ms("xms"), 0); // "x" before "ms" fails parse
        assert_eq!(parse_time_to_ms("abcms"), 0);
    }

    #[test]
    fn test_parse_time_just_suffix_no_number() {
        assert_eq!(parse_time_to_ms("s"), 0);
        assert_eq!(parse_time_to_ms("ms"), 0);
        assert_eq!(parse_time_to_ms("m"), 0);
    }
}
