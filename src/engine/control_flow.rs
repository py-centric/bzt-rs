use crate::engine::BztError;
use crate::models::config::AssertionDefinition;
use regex::Regex;
use std::collections::HashMap;

pub struct AssertionEngine;

impl AssertionEngine {
    #[allow(clippy::missing_errors_doc)]
    pub fn check_assertion(
        body: &str,
        status: u16,
        assertion: &AssertionDefinition,
    ) -> Result<(), BztError> {
        let subject_val = match assertion.subject.as_str() {
            "http-code" => return check_status(status, assertion),
            _ => body,
        };

        for pattern in &assertion.contains {
            // NOTE: `pattern` is dynamic per-assertion so cannot be cached in a
            // static OnceLock.  The regex crate's internal DFA caches compiled
            // patterns in a thread-local LRU, so repeated compilations of the
            // same pattern string are cheap after the first call.
            let found = if assertion.regexp {
                Regex::new(pattern).is_ok_and(|re| re.is_match(subject_val))
            } else {
                subject_val.contains(pattern)
            };

            if found == assertion.not {
                return Err(BztError::Validation {
                    field: assertion.subject.clone(),
                    reason: format!(
                        "expected to {} contain '{pattern}'",
                        if assertion.not { "not" } else { "" },
                    ),
                });
            }
        }
        Ok(())
    }
}

fn check_status(status: u16, assertion: &AssertionDefinition) -> Result<(), BztError> {
    let status_str = status.to_string();
    for pattern in &assertion.contains {
        let found = status_str == *pattern;
        if found == assertion.not {
            return Err(BztError::Validation {
                field: "http-code".to_string(),
                reason: format!(
                    "expected status to {} be {pattern}",
                    if assertion.not { "not" } else { "" },
                ),
            });
        }
    }
    Ok(())
}

pub struct ControlFlowEngine;

impl ControlFlowEngine {
    /// Evaluates a condition string against a set of variables.
    /// Supports:
    /// - Comparisons: ==, !=, >, <, >=, <=
    /// - Logical operators: &&, ||
    /// - Basic numeric and string values
    #[must_use]
    pub fn evaluate_condition(cond: &str, variables: &HashMap<String, String>) -> bool {
        let cond = cond.trim();
        if cond.is_empty() {
            return true;
        }

        // Handle OR
        if cond.contains("||") {
            return cond
                .split("||")
                .any(|part| Self::evaluate_condition(part, variables));
        }

        // Handle AND
        if cond.contains("&&") {
            return cond
                .split("&&")
                .all(|part| Self::evaluate_condition(part, variables));
        }

        // Handle basic comparisons
        if let Some((left, op, right)) = parse_comparison(cond) {
            let left_val = variables.get(left).map_or(left, String::as_str);
            let right_val = right.trim_matches('"');

            match op {
                "==" => left_val == right_val,
                "!=" => left_val != right_val,
                ">" | "<" | ">=" | "<=" => {
                    if let (Ok(l), Ok(r)) = (left_val.parse::<f64>(), right_val.parse::<f64>()) {
                        match op {
                            ">" => l > r,
                            "<" => l < r,
                            ">=" => l >= r,
                            "<=" => l <= r,
                            _ => false,
                        }
                    } else {
                        // Fallback to string comparison if not numeric
                        match op {
                            ">" => left_val > right_val,
                            "<" => left_val < right_val,
                            ">=" => left_val >= right_val,
                            "<=" => left_val <= right_val,
                            _ => false,
                        }
                    }
                }
                _ => false,
            }
        } else {
            // Truthy check for single variable
            variables.get(cond).is_some_and(|v| !v.is_empty())
        }
    }
}

fn parse_comparison(cond: &str) -> Option<(&str, &str, &str)> {
    let ops = ["==", "!=", ">=", "<=", ">", "<"];
    for op in ops {
        if let Some(idx) = cond.find(op) {
            let left = cond[..idx].trim();
            let right = cond[idx + op.len()..].trim();
            return Some((left, op, right));
        }
    }
    None
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
    fn test_evaluate_condition_basic() {
        let mut vars = HashMap::new();
        vars.insert("status".to_string(), "200".to_string());
        assert!(ControlFlowEngine::evaluate_condition(
            "status == 200",
            &vars
        ));
        assert!(ControlFlowEngine::evaluate_condition(
            "status != 404",
            &vars
        ));
    }

    #[test]
    fn test_evaluate_condition_numeric() {
        let mut vars = HashMap::new();
        vars.insert("count".to_string(), "10".to_string());
        assert!(ControlFlowEngine::evaluate_condition("count > 5", &vars));
        assert!(ControlFlowEngine::evaluate_condition("count >= 10", &vars));
        assert!(ControlFlowEngine::evaluate_condition("count < 20", &vars));
        assert!(!ControlFlowEngine::evaluate_condition("count < 5", &vars));
    }

    #[test]
    fn test_evaluate_condition_logical() {
        let mut vars = HashMap::new();
        vars.insert("status".to_string(), "200".to_string());
        vars.insert("count".to_string(), "10".to_string());
        assert!(ControlFlowEngine::evaluate_condition(
            "status == 200 && count > 5",
            &vars
        ));
        assert!(ControlFlowEngine::evaluate_condition(
            "status == 500 || count == 10",
            &vars
        ));
        assert!(!ControlFlowEngine::evaluate_condition(
            "status == 200 && count < 5",
            &vars
        ));
    }

    #[test]
    fn test_evaluate_condition_truthy() {
        let mut vars = HashMap::new();
        vars.insert("exists".to_string(), "true".to_string());
        assert!(ControlFlowEngine::evaluate_condition("exists", &vars));
        assert!(!ControlFlowEngine::evaluate_condition("missing", &vars));
    }
}
