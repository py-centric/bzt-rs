use fake::Fake;
use fake::faker::internet::en::SafeEmail;
use fake::faker::lorem::en::Word;
use fake::faker::name::en::Name;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use uuid::Uuid;

pub struct MacroEvaluator;

impl MacroEvaluator {
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
                new_output = new_output.replace(&cap[0], &val);
            }
        }
        output = new_output;

        let env_simple_regex = Regex::new(r"\$\{([A-Z0-9_]+)\}").unwrap();
        let mut new_output = output.clone();
        for cap in env_simple_regex.captures_iter(&output) {
            let var_name = &cap[1];
            if let Ok(val) = env::var(var_name) {
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
