use crate::engine::BztError;
use crate::models::config::Configuration;
use std::fs;
use std::path::Path;
use toml;

pub struct TomlParser;

impl TomlParser {
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<Configuration, BztError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| {
            let msg = e.to_string();
            BztError::Serde {
                message: msg,
                file_path: Some(path.to_string_lossy().to_string()),
                source: Some(Box::new(e)),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
[[execution]]
concurrency = 5
ramp-up = "10s"
hold-for = "1m"
scenario = "toml-test"

[scenarios.toml-test]
requests = ["http://localhost:8080/"]
"#;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{}", toml_str).unwrap();

        let config = TomlParser::parse(file.path()).unwrap();
        assert_eq!(config.execution[0].concurrency, 5);
    }
}
