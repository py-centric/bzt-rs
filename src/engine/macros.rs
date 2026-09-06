use crate::engine::PummelError;
use crate::engine::env::EnvironmentLoader;
#[cfg(feature = "faker-macros")]
use fake::Fake;
#[cfg(feature = "faker-macros")]
use fake::faker::internet::en::SafeEmail;
#[cfg(feature = "faker-macros")]
use fake::faker::lorem::en::Word;
#[cfg(feature = "faker-macros")]
use fake::faker::name::en::Name;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::sync::LazyLock;
#[cfg(feature = "faker-macros")]
use uuid::Uuid;

static ENV_SIMPLE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([A-Z0-9_]+)\}").unwrap());

static FIELD_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([a-z0-9_]+)\}").unwrap());

/// Unified sensitive environment variable patterns.
///
/// Matching is performed by converting the variable name to uppercase and checking:
/// - Exact matches (e.g., "KEY", "`DATABASE_URL`")
/// - Prefix matches (e.g., starts with "AWS_", "SECRET", "DB_", "PRIVATE")
/// - Suffix matches (e.g., ends with "_KEY", "_TOKEN", "_PASSWORD")
/// - Contains matches (e.g., contains "PASSWORD")
///
/// This single list is used for both **rejection** (macro evaluation) and
/// **warnings** (validation scanning).
const SENSITIVE_PATTERNS: &[SensitivePattern] = &[
    SensitivePattern::StartsWith("AWS_"),
    SensitivePattern::StartsWith("SECRET"),
    SensitivePattern::Exact("KEY"),
    SensitivePattern::EndsWith("_KEY"),
    SensitivePattern::EndsWith("_TOKEN"),
    SensitivePattern::EndsWith("_PASSWORD"),
    SensitivePattern::Contains("PASSWORD"),
    SensitivePattern::StartsWith("PRIVATE"),
    SensitivePattern::Exact("DATABASE_URL"),
    SensitivePattern::StartsWith("DB_"),
];

#[derive(Debug, Clone, Copy)]
enum SensitivePattern {
    /// Name must equal the pattern (case-insensitive, uppercased comparison).
    Exact(&'static str),
    /// Upper-cased name starts with the pattern.
    StartsWith(&'static str),
    /// Upper-cased name ends with the pattern.
    EndsWith(&'static str),
    /// Upper-cased name contains the pattern.
    Contains(&'static str),
}

/// Checks if an environment variable name matches any sensitive pattern.
/// Sensitive vars are **rejected** during resolution — they are never injected.
#[must_use]
pub fn is_sensitive_var(name: &str) -> bool {
    matches_any_pattern(name)
}

/// Returns the blocklist of patterns used to match sensitive env var names.
/// Any variable whose uppercased name matches any of these patterns is rejected.
///
/// This function returns `Vec<String>` for backward compatibility with code that
/// calls `.iter().any(|p| var_name.contains(p))`. The patterns are the raw
/// strings from `SENSITIVE_PATTERNS`; callers should use `is_sensitive_var()`
/// for precise matching.
#[must_use]
pub fn sensitive_blocklist() -> Vec<&'static str> {
    SENSITIVE_PATTERNS
        .iter()
        .map(|p| match p {
            SensitivePattern::Exact(s)
            | SensitivePattern::StartsWith(s)
            | SensitivePattern::EndsWith(s)
            | SensitivePattern::Contains(s) => *s,
        })
        .collect()
}

/// Internal: checks if `name` matches any pattern in `SENSITIVE_PATTERNS`.
fn matches_any_pattern(name: &str) -> bool {
    let upper = name.to_uppercase();
    SENSITIVE_PATTERNS.iter().any(|p| match p {
        SensitivePattern::Exact(s) => upper == *s,
        SensitivePattern::StartsWith(s) => upper.starts_with(s),
        SensitivePattern::EndsWith(s) => upper.ends_with(s),
        SensitivePattern::Contains(s) => upper.contains(s),
    })
}

/// Masks a sensitive variable value for safe logging.
/// Returns `"[REDACTED]"` if the variable name is sensitive, otherwise returns the value.
#[must_use]
pub fn mask_env_value(name: &str, value: &str) -> String {
    if is_sensitive_var(name) {
        "[REDACTED]".to_string()
    } else {
        value.to_string()
    }
}

pub struct MacroEvaluator;

impl MacroEvaluator {
    /// Evaluates macros in `input` and returns the resolved string.
    ///
    /// # Errors
    ///
    /// Returns `PummelError::Environment` if a sensitive/blocked env var is referenced.
    #[allow(clippy::missing_panics_doc)]
    pub fn evaluate(
        input: &str,
        record: Option<&HashMap<String, String>>,
    ) -> Result<String, PummelError> {
        let mut output = input.to_string();

        // 1. Evaluate Dynamic Macros
        #[cfg(feature = "faker-macros")]
        {
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
        }

        // 2. Evaluate Environment Variables ${env.VAR_NAME} via EnvironmentLoader (with blocklist)
        let blocklist = sensitive_blocklist();
        let vars = HashMap::new(); // empty — EnvironmentLoader falls through to std::env
        output = EnvironmentLoader::resolve(&output, &vars, &blocklist)?;

        // 2b. Evaluate simple ${VAR_NAME} env vars (with blocklist rejection)
        let mut new_output = output.clone();
        for cap in ENV_SIMPLE_PATTERN.captures_iter(&output) {
            let var_name = &cap[1];
            if is_sensitive_var(var_name) {
                return Err(PummelError::Environment {
                    var: var_name.to_string(),
                    reason: "access to sensitive environment variable is blocked".to_string(),
                });
            }
            if let Ok(val) = env::var(var_name) {
                new_output = new_output.replace(&cap[0], &val);
            }
        }
        output = new_output;

        // 3. Evaluate CSV record fields ${field_name}
        if let Some(r) = record {
            let mut new_output = output.clone();
            for cap in FIELD_PATTERN.captures_iter(&output) {
                let field_name = &cap[1];
                if let Some(val) = r.get(field_name) {
                    new_output = new_output.replace(&cap[0], val);
                }
            }
            output = new_output;
        }

        Ok(output)
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
        assert_eq!(
            mask_env_value("AWS_SECRET_KEY", "super-secret"),
            "[REDACTED]"
        );
        assert_eq!(mask_env_value("DB_PASSWORD", "hunter2"), "[REDACTED]");
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
        assert_eq!(
            MacroEvaluator::evaluate(input, None).unwrap(),
            "Value is hello"
        );
    }

    #[test]
    fn test_record_injection() {
        let mut record = HashMap::new();
        record.insert("id".to_string(), "123".to_string());
        let input = "User ${id}";
        assert_eq!(
            MacroEvaluator::evaluate(input, Some(&record)).unwrap(),
            "User 123"
        );
    }

    #[cfg(feature = "faker-macros")]
    #[test]
    fn test_faker_macro() {
        let input = "${faker.email}";
        let output = MacroEvaluator::evaluate(input, None).unwrap();
        assert!(output.contains('@'));
        assert_ne!(output, "${faker.email}");
    }

    #[cfg(feature = "faker-macros")]
    #[test]
    fn test_uuid_macro() {
        let input = "${uuid}";
        let output = MacroEvaluator::evaluate(input, None).unwrap();
        assert_eq!(output.len(), 36);
        assert_ne!(output, "${uuid}");
    }

    #[test]
    fn test_sensitive_env_var_blocked() {
        unsafe {
            env::set_var("AWS_SECRET_KEY", "should-not-resolve");
        }
        let input = "${AWS_SECRET_KEY}";
        let result = MacroEvaluator::evaluate(input, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("blocked"));
    }

    #[test]
    fn test_sensitive_env_var_env_prefix_blocked() {
        unsafe {
            env::set_var("DB_PASSWORD", "hunter2");
        }
        let input = "${env.DB_PASSWORD}";
        let result = MacroEvaluator::evaluate(input, None);
        assert!(result.is_err());
    }
}
