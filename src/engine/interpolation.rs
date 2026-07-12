use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

static VAR_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([a-zA-Z0-9_]+)\}").unwrap());

pub struct Interpolator;

impl Interpolator {
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn interpolate(input: &str, variables: &HashMap<String, String>) -> String {
        let output = input.to_string();

        let mut new_output = output.clone();
        for cap in VAR_PATTERN.captures_iter(&output) {
            let var_name = &cap[1];
            if let Some(val) = variables.get(var_name) {
                new_output = new_output.replace(&cap[0], val);
            }
        }
        new_output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_variable() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        let result = Interpolator::interpolate("Hello ${name}!", &vars);
        assert_eq!(result, "Hello Alice!");
    }

    #[test]
    fn test_multiple_variables() {
        let mut vars = HashMap::new();
        vars.insert("first".to_string(), "John".to_string());
        vars.insert("last".to_string(), "Doe".to_string());
        let result = Interpolator::interpolate("${first} ${last}", &vars);
        assert_eq!(result, "John Doe");
    }

    #[test]
    fn test_missing_variable_unchanged() {
        let vars = HashMap::new();
        let result = Interpolator::interpolate("Hello ${name}!", &vars);
        assert_eq!(result, "Hello ${name}!");
    }

    #[test]
    fn test_no_variables() {
        let vars = HashMap::new();
        let result = Interpolator::interpolate("Hello world!", &vars);
        assert_eq!(result, "Hello world!");
    }

    #[test]
    fn test_same_variable_multiple_times() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), "foo".to_string());
        let result = Interpolator::interpolate("${x}_${x}_${x}", &vars);
        assert_eq!(result, "foo_foo_foo");
    }

    #[test]
    fn test_partial_match_no_replace() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        let result = Interpolator::interpolate("Hello ${name} and ${other}!", &vars);
        assert_eq!(result, "Hello Alice and ${other}!");
    }

    #[test]
    fn test_variable_name_with_numbers() {
        let mut vars = HashMap::new();
        vars.insert("var123".to_string(), "value".to_string());
        let result = Interpolator::interpolate("${var123}", &vars);
        assert_eq!(result, "value");
    }

    #[test]
    fn test_empty_input() {
        let vars = HashMap::new();
        let result = Interpolator::interpolate("", &vars);
        assert_eq!(result, "");
    }
}
