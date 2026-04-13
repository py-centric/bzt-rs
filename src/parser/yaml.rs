use crate::models::config::Configuration;
use serde_yaml;
use std::fs;
use std::path::Path;

pub struct YamlParser;

impl YamlParser {
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<Configuration, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_yaml::from_str(&content).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_basic_yaml() {
        let yaml = r#"
execution:
- concurrency: 10
  ramp-up: 5s
  hold-for: 30s
  scenario: basic

scenarios:
  basic:
    requests:
    - http://localhost:8080/
    - url: http://localhost:8080/api
      method: POST
      body: '{"test": true}'
"#;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{}", yaml).unwrap();

        let config = YamlParser::parse(file.path()).unwrap();
        assert_eq!(config.execution.len(), 1);
        assert_eq!(config.execution[0].concurrency, 10);
        assert_eq!(config.scenarios.len(), 1);
        assert!(config.scenarios.contains_key("basic"));
    }
}
