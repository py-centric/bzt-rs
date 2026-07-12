use regex::Regex;
use serde_json::Value;
use serde_json_path::JsonPath;
use std::collections::HashMap;
#[cfg(feature = "xpath")]
use sxd_document::parser;
#[cfg(feature = "xpath")]
use sxd_xpath::{Context, Factory, Value as XPathValue};

#[derive(Default, Debug)]
pub struct UserSession {
    pub variables: HashMap<String, String>,
}

pub struct ExtractionEngine;

impl ExtractionEngine {
    pub fn extract_jsonpath(
        body: &str,
        rules: &HashMap<String, String>,
        session: &mut UserSession,
    ) {
        if let Ok(json) = serde_json::from_str::<Value>(body) {
            for (var_name, path_str) in rules {
                if let Some(node) = JsonPath::parse(path_str)
                    .ok()
                    .and_then(|p| p.query(&json).first())
                {
                    let value = match node {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => node.to_string(),
                    };
                    session.variables.insert(var_name.clone(), value);
                }
            }
        }
    }

    pub fn extract_regex(body: &str, rules: &HashMap<String, String>, session: &mut UserSession) {
        for (var_name, regex_str) in rules {
            if let Some(val) = Regex::new(regex_str)
                .ok()
                .and_then(|re| re.captures(body))
                .and_then(|cap| cap.get(1).or_else(|| cap.get(0)))
            {
                session
                    .variables
                    .insert(var_name.clone(), val.as_str().to_string());
            }
        }
    }

    #[cfg(feature = "xpath")]
    pub fn extract_xpath(body: &str, rules: &HashMap<String, String>, session: &mut UserSession) {
        let package = match parser::parse(body) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("[XPATH] Failed to parse XML body: {}", e);
                return;
            }
        };
        let document = package.as_document();
        let factory = Factory::new();

        for (var_name, xpath_str) in rules {
            let xpath = match factory.build(xpath_str) {
                Ok(Some(x)) => x,
                Ok(None) => {
                    tracing::warn!("[XPATH] XPath expression returned None: {}", xpath_str);
                    continue;
                }
                Err(e) => {
                    tracing::warn!("[XPATH] Invalid XPath expression '{}': {}", xpath_str, e);
                    continue;
                }
            };

            let context = Context::new();
            match xpath.evaluate(&context, document.root()) {
                Ok(value) => {
                    let result = match value {
                        XPathValue::Nodeset(ns) => {
                            if let Some(node) = ns.document_order().first() {
                                node.string_value()
                            } else {
                                continue;
                            }
                        }
                        XPathValue::Boolean(b) => b.to_string(),
                        XPathValue::Number(n) => n.to_string(),
                        XPathValue::String(s) => s,
                    };
                    session.variables.insert(var_name.clone(), result);
                }
                Err(e) => {
                    tracing::warn!("[XPATH] Failed to evaluate XPath '{}': {}", xpath_str, e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_jsonpath() {
        let body = r#"{"id": 123, "name": "test"}"#;
        let mut rules = HashMap::new();
        rules.insert("userId".to_string(), "$.id".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_jsonpath(body, &rules, &mut session);
        assert_eq!(session.variables.get("userId").unwrap(), "123");
    }

    #[test]
    fn test_extract_regex() {
        let body = "Token: abc-123-def";
        let mut rules = HashMap::new();
        rules.insert("token".to_string(), "Token: ([a-z0-9-]+)".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_regex(body, &rules, &mut session);
        assert_eq!(session.variables.get("token").unwrap(), "abc-123-def");
    }

    #[cfg(feature = "xpath")]
    #[test]
    fn test_extract_xpath() {
        let body = r#"<bookstore><book category="cooking"><title lang="en">Everyday Italian</title><price>30.00</price></book></bookstore>"#;
        let mut rules = HashMap::new();
        rules.insert(
            "title".to_string(),
            "//book[@category='cooking']/title/text()".to_string(),
        );
        rules.insert("price".to_string(), "//price/text()".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_xpath(body, &rules, &mut session);
        assert_eq!(session.variables.get("title").unwrap(), "Everyday Italian");
        assert_eq!(session.variables.get("price").unwrap(), "30.00");
    }

    // --- JSONPath edge cases ---

    #[test]
    fn test_extract_jsonpath_empty_body() {
        let mut rules = HashMap::new();
        rules.insert("val".to_string(), "$.id".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_jsonpath("", &rules, &mut session);
        assert!(session.variables.is_empty(), "empty body should produce no variables");
    }

    #[test]
    fn test_extract_jsonpath_malformed_json() {
        let mut rules = HashMap::new();
        rules.insert("val".to_string(), "$.id".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_jsonpath("{not valid json", &rules, &mut session);
        assert!(session.variables.is_empty(), "malformed JSON should produce no variables");
    }

    #[test]
    fn test_extract_jsonpath_missing_field() {
        let body = r#"{"id": 123}"#;
        let mut rules = HashMap::new();
        rules.insert("missing".to_string(), "$.nonexistent".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_jsonpath(body, &rules, &mut session);
        assert!(session.variables.is_empty(), "missing field should produce no variable");
    }

    #[test]
    fn test_extract_jsonpath_empty_rules() {
        let body = r#"{"id": 123}"#;
        let rules = HashMap::new();
        let mut session = UserSession::default();
        ExtractionEngine::extract_jsonpath(body, &rules, &mut session);
        assert!(session.variables.is_empty(), "empty rules should produce no variables");
    }

    #[test]
    fn test_extract_jsonpath_string_value() {
        let body = r#"{"name": "hello"}"#;
        let mut rules = HashMap::new();
        rules.insert("n".to_string(), "$.name".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_jsonpath(body, &rules, &mut session);
        assert_eq!(session.variables.get("n").unwrap(), "hello");
    }

    #[test]
    fn test_extract_jsonpath_bool_value() {
        let body = r#"{"active": true}"#;
        let mut rules = HashMap::new();
        rules.insert("flag".to_string(), "$.active".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_jsonpath(body, &rules, &mut session);
        assert_eq!(session.variables.get("flag").unwrap(), "true");
    }

    // --- Regex edge cases ---

    #[test]
    fn test_extract_regex_no_match() {
        let body = "nothing here matches";
        let mut rules = HashMap::new();
        rules.insert("val".to_string(), r"token=([a-z]+)".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_regex(body, &rules, &mut session);
        assert!(session.variables.is_empty(), "no match should produce no variable");
    }

    #[test]
    fn test_extract_regex_invalid_regex() {
        let body = "some text";
        let mut rules = HashMap::new();
        rules.insert("val".to_string(), "[invalid".to_string()); // invalid regex
        let mut session = UserSession::default();
        ExtractionEngine::extract_regex(body, &rules, &mut session);
        assert!(session.variables.is_empty(), "invalid regex should produce no variable");
    }

    #[test]
    fn test_extract_regex_empty_body() {
        let mut rules = HashMap::new();
        rules.insert("val".to_string(), r"token=([a-z]+)".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_regex("", &rules, &mut session);
        assert!(session.variables.is_empty(), "empty body should produce no variable");
    }

    #[test]
    fn test_extract_regex_empty_rules() {
        let body = "Token: abc";
        let rules = HashMap::new();
        let mut session = UserSession::default();
        ExtractionEngine::extract_regex(body, &rules, &mut session);
        assert!(session.variables.is_empty(), "empty rules should produce no variables");
    }

    #[test]
    fn test_extract_regex_full_match_no_group() {
        // Pattern with no capture group — should use match(0)
        let body = "abc-123";
        let mut rules = HashMap::new();
        rules.insert("val".to_string(), r"[a-z]+-\d+".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_regex(body, &rules, &mut session);
        assert_eq!(session.variables.get("val").unwrap(), "abc-123");
    }

    // --- XPath edge cases ---

    #[cfg(feature = "xpath")]
    #[test]
    fn test_extract_xpath_malformed_xml() {
        let body = "<unclosed";
        let mut rules = HashMap::new();
        rules.insert("val".to_string(), "//root/text()".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_xpath(body, &rules, &mut session);
        assert!(session.variables.is_empty(), "malformed XML should produce no variables");
    }

    #[cfg(feature = "xpath")]
    #[test]
    fn test_extract_xpath_invalid_xpath_syntax() {
        let body = r#"<root><item>hello</item></root>"#;
        let mut rules = HashMap::new();
        rules.insert("val".to_string(), "[invalid-xpath".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_xpath(body, &rules, &mut session);
        assert!(session.variables.is_empty(), "invalid xpath should produce no variables");
    }

    #[cfg(feature = "xpath")]
    #[test]
    fn test_extract_xpath_no_match() {
        let body = r#"<root><item>hello</item></root>"#;
        let mut rules = HashMap::new();
        rules.insert("val".to_string(), "//missing/text()".to_string());
        let mut session = UserSession::default();
        ExtractionEngine::extract_xpath(body, &rules, &mut session);
        assert!(session.variables.is_empty(), "xpath with no match should produce no variables");
    }

    #[cfg(feature = "xpath")]
    #[test]
    fn test_extract_xpath_empty_rules() {
        let body = r#"<root><item>hello</item></root>"#;
        let rules = HashMap::new();
        let mut session = UserSession::default();
        ExtractionEngine::extract_xpath(body, &rules, &mut session);
        assert!(session.variables.is_empty(), "empty rules should produce no variables");
    }

    // --- UserSession ---

    #[test]
    fn test_user_session_default_is_empty() {
        let session = UserSession::default();
        assert!(session.variables.is_empty());
    }

    #[test]
    fn test_user_session_variable_overwrite() {
        let mut session = UserSession::default();
        session.variables.insert("key".to_string(), "val1".to_string());
        session.variables.insert("key".to_string(), "val2".to_string());
        assert_eq!(session.variables.get("key").unwrap(), "val2");
    }
}
