use regex::Regex;
use serde_json::Value;
use serde_json_path::JsonPath;
use std::collections::HashMap;

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
}
