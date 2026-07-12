use crate::engine::BztError;
use crate::models::config::Configuration;
use serde_json;
use std::fs;
use std::path::Path;

pub struct JsonParser;

impl JsonParser {
    #[allow(clippy::missing_errors_doc)]
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<Configuration, BztError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(|e| {
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
    fn test_parse_json() {
        let json = r#"
{
  "execution": [
    {
      "concurrency": 5,
      "ramp-up": "10s",
      "hold-for": "1m",
      "scenario": "json-test"
    }
  ],
  "scenarios": {
    "json-test": {
      "requests": ["http://localhost:8080/"]
    }
  }
}
"#;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{}", json).unwrap();

        let config = JsonParser::parse(file.path()).unwrap();
        assert_eq!(config.execution[0].concurrency, 5);
    }
}
