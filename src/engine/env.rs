use crate::engine::PummelError;
use crate::engine::macros::is_sensitive_var;
use crate::engine::macros::mask_env_value;
use std::collections::HashMap;

pub struct EnvironmentLoader;

impl EnvironmentLoader {
    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub fn resolve(
        input: &str,
        variables: &HashMap<String, String>,
        _blocklist: &[&str],
    ) -> Result<String, PummelError> {
        tracing::debug!("[ENV] Resolving env vars in: '{}'", input);
        let mut output = input.to_string();
        let re = regex::Regex::new(r"\$\{env\.([A-Za-z0-9_]+)\}").unwrap();

        for cap in re.captures_iter(input) {
            let var_name = &cap[1];
            // Security: use unified is_sensitive_var() for precise pattern matching
            // instead of substring .contains() which over-blocks (e.g., KEYBOARD, TOKENIZER)
            if is_sensitive_var(var_name) {
                tracing::debug!("[ENV] Blocked sensitive var '{}' from resolving", var_name,);
                return Err(PummelError::Environment {
                    var: var_name.to_string(),
                    reason: "access to sensitive environment variable is blocked".to_string(),
                });
            }
            let val = variables
                .get(var_name)
                .cloned()
                .or_else(|| std::env::var(var_name).ok())
                .unwrap_or_default();
            // Security: mask sensitive values in debug output to prevent credential leakage
            let masked = mask_env_value(var_name, &val);
            tracing::debug!("[ENV] Resolved '${{env.{}}}' -> '{}'", var_name, masked);
            output = output.replace(&cap[0], &val);
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_env_var() {
        let mut vars = HashMap::new();
        vars.insert("DEBUG".to_string(), "true".to_string());
        let result = EnvironmentLoader::resolve("${env.DEBUG}", &vars, &[]).unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_resolve_missing_var_returns_empty() {
        let vars = HashMap::new();
        let result = EnvironmentLoader::resolve("${env.MISSING}", &vars, &[]).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_blocklist_rejects_sensitive_var() {
        let mut vars = HashMap::new();
        vars.insert("AWS_SECRET_KEY".to_string(), "secret123".to_string());
        let result = EnvironmentLoader::resolve("${env.AWS_SECRET_KEY}", &vars, &["SECRET"]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PummelError::Environment { .. }));
    }

    #[test]
    fn test_resolve_with_prefix_and_suffix() {
        let mut vars = HashMap::new();
        vars.insert("HOST".to_string(), "localhost".to_string());
        let result =
            EnvironmentLoader::resolve("http://${env.HOST}:8080/path", &vars, &[]).unwrap();
        assert_eq!(result, "http://localhost:8080/path");
    }

    #[test]
    fn test_no_env_var_no_change() {
        let vars = HashMap::new();
        let result = EnvironmentLoader::resolve("plain text", &vars, &[]).unwrap();
        assert_eq!(result, "plain text");
    }

    #[test]
    fn test_multiple_env_vars() {
        let mut vars = HashMap::new();
        vars.insert("A".to_string(), "hello".to_string());
        vars.insert("B".to_string(), "world".to_string());
        let result = EnvironmentLoader::resolve("${env.A} ${env.B}", &vars, &[]).unwrap();
        assert_eq!(result, "hello world");
    }
}
