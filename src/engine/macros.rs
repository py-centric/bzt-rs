use fake::Fake;
use fake::faker::internet::en::SafeEmail;
use fake::faker::lorem::en::Word;
use fake::faker::name::en::Name;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use uuid::Uuid;

/// Checks if an environment variable name matches sensitive patterns.
/// Sensitive vars are still resolved for substitution but their values
/// are masked in logs and error messages.
#[must_use]
pub fn is_sensitive_var(name: &str) -> bool {
    let upper = name.to_uppercase();
    if upper.starts_with("AWS_")
        || upper.starts_with("SECRET")
        || upper == "KEY"
        || upper.ends_with("_KEY")
        || upper.ends_with("_TOKEN")
        || upper.ends_with("_PASSWORD")
        || upper.contains("PASSWORD")
        || upper.starts_with("PRIVATE")
        || upper == "DATABASE_URL"
        || upper.starts_with("DB_")
    {
        return true;
    }
    false
}

/// Masks a sensitive variable value for safe logging.
/// Returns `"***"` if the variable name is sensitive, otherwise returns the value.
#[must_use]
pub fn mask_env_value(name: &str, value: &str) -> String {
    if is_sensitive_var(name) {
        "***".to_string()
    } else {
        value.to_string()
    }
}

pub struct MacroEvaluator;

impl MacroEvaluator {
    #[must_use]
    pub fn evaluate(input: &str, record: Option<&HashMap<String, String>>) -> String {
        let mut output = input.to_string();

        // 1. Evaluate Dynamic Macros
        if output.contains("${faker.email}") {
            let email: String = SafeEmail().fake();
            output = output.replace("${faker.email}", &email);
        }
        if output.contains("${faker.name}") {
            let name: String = Name().fake();
            output = output.replace("${faker.name}", &name);
        }
        if output.contains("${faker.word}") {
            let word: String = Word().fake();
            output = output.replace("${faker.word}", &word);
        }
        if output.contains("${uuid}") {
            let my_uuid = Uuid::new_v4().to_string();
            output = output.replace("${uuid}", &my_uuid);
        }

        // 2. Evaluate Environment Variables ${env.VAR_NAME} or ${VAR_NAME}
        let env_regex = Regex::new(r"\$\{env\.([A-Z0-9_]+)\}").unwrap();
        let mut new_output = output.clone();
        for cap in env_regex.captures_iter(&output) {
            let var_name = &cap[1];
            if let Ok(val) = env::var(var_name) {
                if is_sensitive_var(var_name) {
                    tracing::warn!(
                        "[SECURITY] Sensitive env var '{}' resolved from config",
                        var_name
                    );
                }
                new_output = new_output.replace(&cap[0], &val);
            }
        }
        output = new_output;

        let env_simple_regex = Regex::new(r"\$\{([A-Z0-9_]+)\}").unwrap();
        let mut new_output = output.clone();
        for cap in env_simple_regex.captures_iter(&output) {
            let var_name = &cap[1];
            if let Ok(val) = env::var(var_name) {
                if is_sensitive_var(var_name) {
                    tracing::warn!(
                        "[SECURITY] Sensitive env var '{}' resolved from config",
                        var_name
                    );
                }
                new_output = new_output.replace(&cap[0], &val);
            }
        }
        output = new_output;

        // 3. Evaluate CSV record fields ${field_name}
        if let Some(r) = record {
            let field_regex = Regex::new(r"\$\{([a-z0-9_]+)\}").unwrap();
            let mut new_output = output.clone();
            for cap in field_regex.captures_iter(&output) {
                let field_name = &cap[1];
                if let Some(val) = r.get(field_name) {
                    new_output = new_output.replace(&cap[0], val);
                }
            }
            output = new_output;
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sensitive_var_aws() {
        assert!(is_sensitive_var("AWS_SECRET_KEY"));
        assert!(is_sensitive_var("AWS_ACCESS_KEY_ID"));
        assert!(is_sensitive_var("aws_secret_key"));
    }

    #[test]
    fn test_is_sensitive_var_secret_prefix() {
        assert!(is_sensitive_var("SECRET_KEY"));
        assert!(is_sensitive_var("SECRET_TOKEN"));
        assert!(is_sensitive_var("secret_value"));
    }

    #[test]
    fn test_is_sensitive_var_key_suffix() {
        assert!(is_sensitive_var("API_KEY"));
        assert!(is_sensitive_var("STRIPE_SECRET_KEY"));
    }

    #[test]
    fn test_is_sensitive_var_token_password() {
        assert!(is_sensitive_var("GITHUB_TOKEN"));
        assert!(is_sensitive_var("DB_PASSWORD"));
        assert!(is_sensitive_var("DATABASE_URL"));
    }

    #[test]
    fn test_is_sensitive_var_private_db() {
        assert!(is_sensitive_var("PRIVATE_KEY"));
        assert!(is_sensitive_var("DB_HOST"));
        assert!(is_sensitive_var("DB_NAME"));
    }

    #[test]
    fn test_is_sensitive_var_safe() {
        assert!(!is_sensitive_var("HOME"));
        assert!(!is_sensitive_var("PATH"));
        assert!(!is_sensitive_var("LANG"));
        assert!(!is_sensitive_var("USER"));
        assert!(!is_sensitive_var("DEBUG"));
    }

    #[test]
    fn test_mask_env_value_sensitive() {
        assert_eq!(mask_env_value("AWS_SECRET_KEY", "super-secret"), "***");
        assert_eq!(mask_env_value("DB_PASSWORD", "hunter2"), "***");
    }

    #[test]
    fn test_mask_env_value_safe() {
        assert_eq!(mask_env_value("HOME", "/home/user"), "/home/user");
        assert_eq!(mask_env_value("PATH", "/usr/bin"), "/usr/bin");
    }

    #[test]
    fn test_env_var_injection() {
        unsafe {
            env::set_var("TEST_VAR", "hello");
        }
        let input = "Value is ${TEST_VAR}";
        assert_eq!(MacroEvaluator::evaluate(input, None), "Value is hello");
    }

    #[test]
    fn test_record_injection() {
        let mut record = HashMap::new();
        record.insert("id".to_string(), "123".to_string());
        let input = "User ${id}";
        assert_eq!(MacroEvaluator::evaluate(input, Some(&record)), "User 123");
    }

    #[test]
    fn test_faker_macro() {
        let input = "${faker.email}";
        let output = MacroEvaluator::evaluate(input, None);
        assert!(output.contains('@'));
        assert_ne!(output, "${faker.email}");
    }

    #[test]
    fn test_uuid_macro() {
        let input = "${uuid}";
        let output = MacroEvaluator::evaluate(input, None);
        assert_eq!(output.len(), 36);
        assert_ne!(output, "${uuid}");
    }
}
