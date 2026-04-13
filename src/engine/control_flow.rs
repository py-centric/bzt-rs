use crate::models::config::AssertionDefinition;
use regex::Regex;

pub struct AssertionEngine;

impl AssertionEngine {
    pub fn check_assertion(
        body: &str,
        status: u16,
        assertion: &AssertionDefinition,
    ) -> Result<(), String> {
        let subject_val = match assertion.subject.as_str() {
            "body" => body,
            "http-code" => return check_status(status, assertion),
            _ => body,
        };

        for pattern in &assertion.contains {
            let found = if assertion.regexp {
                Regex::new(pattern)
                    .map(|re| re.is_match(subject_val))
                    .unwrap_or(false)
            } else {
                subject_val.contains(pattern)
            };

            if found == assertion.not {
                return Err(format!(
                    "Assertion failed: expected {} to {} contain '{}'",
                    assertion.subject,
                    if assertion.not { "not" } else { "" },
                    pattern
                ));
            }
        }
        Ok(())
    }
}

fn check_status(status: u16, assertion: &AssertionDefinition) -> Result<(), String> {
    let status_str = status.to_string();
    for pattern in &assertion.contains {
        let found = status_str == *pattern;
        if found == assertion.not {
            return Err(format!(
                "Assertion failed: expected status to {} be {}",
                if assertion.not { "not" } else { "" },
                pattern
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_body_assertion() {
        let assertion = AssertionDefinition {
            contains: vec!["Hello".to_string()],
            subject: "body".to_string(),
            regexp: false,
            not: false,
        };
        assert!(AssertionEngine::check_assertion("Hello world", 200, &assertion).is_ok());
        assert!(AssertionEngine::check_assertion("Goodbye", 200, &assertion).is_err());
    }

    #[test]
    fn test_status_assertion() {
        let assertion = AssertionDefinition {
            contains: vec!["200".to_string()],
            subject: "http-code".to_string(),
            regexp: false,
            not: false,
        };
        assert!(AssertionEngine::check_assertion("", 200, &assertion).is_ok());
        assert!(AssertionEngine::check_assertion("", 404, &assertion).is_err());
    }

    #[test]
    fn test_not_assertion() {
        let assertion = AssertionDefinition {
            contains: vec!["Error".to_string()],
            subject: "body".to_string(),
            regexp: false,
            not: true,
        };
        assert!(AssertionEngine::check_assertion("Success", 200, &assertion).is_ok());
        assert!(AssertionEngine::check_assertion("Error found", 500, &assertion).is_err());
    }

    #[test]
    fn test_regex_assertion() {
        let assertion = AssertionDefinition {
            contains: vec!["[0-9]{3}".to_string()],
            subject: "body".to_string(),
            regexp: true,
            not: false,
        };
        assert!(AssertionEngine::check_assertion("Code 123", 200, &assertion).is_ok());
        assert!(AssertionEngine::check_assertion("No code", 200, &assertion).is_err());
    }
}
