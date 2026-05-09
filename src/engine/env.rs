use crate::engine::BztError;
use std::collections::HashMap;

pub struct EnvironmentLoader;

impl EnvironmentLoader {
    pub fn resolve(
        input: &str,
        variables: &HashMap<String, String>,
        blocklist: &[&str],
    ) -> Result<String, BztError> {
        tracing::debug!("[ENV] Resolving env vars in: '{}'", input);
        let mut output = input.to_string();
        let re = regex::Regex::new(r"\$\{env\.([A-Za-z0-9_]+)\}").unwrap();

        for cap in re.captures_iter(input) {
            let var_name = &cap[1];
            if blocklist.iter().any(|p| var_name.contains(p)) {
                tracing::debug!(
                    "[ENV] Blocked sensitive var '{}' from resolving in '{}'",
                    var_name,
                    input
                );
                return Err(BztError::Environment {
                    var: var_name.to_string(),
                    reason: "access to sensitive environment variable is blocked".to_string(),
                });
            }
            let val = variables
                .get(var_name)
                .cloned()
                .or_else(|| std::env::var(var_name).ok())
                .unwrap_or_default();
            tracing::debug!("[ENV] Resolved '${{env.{}}}' -> '{}'", var_name, val);
            output = output.replace(&cap[0], &val);
        }

        tracing::debug!("[ENV] Resolved output: '{}'", output);
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
        assert!(matches!(result.unwrap_err(), BztError::Environment { .. }));
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
