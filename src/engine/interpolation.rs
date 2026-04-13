use regex::Regex;
use std::collections::HashMap;

pub struct Interpolator;

impl Interpolator {
    #[must_use]
    pub fn interpolate(input: &str, variables: &HashMap<String, String>) -> String {
        let output = input.to_string();
        let re = Regex::new(r"\$\{([a-zA-Z0-9_]+)\}").unwrap();

        let mut new_output = output.clone();
        for cap in re.captures_iter(&output) {
            let var_name = &cap[1];
            if let Some(val) = variables.get(var_name) {
                new_output = new_output.replace(&cap[0], val);
            }
        }
        new_output
    }
}
