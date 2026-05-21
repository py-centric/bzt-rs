use crate::engine::macros::is_sensitive_var;
use crate::models::config::{Configuration, DataSourceDefinition, HTTPRequestDefinition};
use crate::translator::StateTranslator;
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Copy)]
pub enum FileStatus {
    #[default]
    Found,
    Missing,
}

#[derive(Debug, Default)]
pub struct ValidationSummary {
    pub file_existence: HashMap<String, FileStatus>,
    pub syntax_valid: bool,
    pub normalization_successful: bool,
    pub translation_successful: bool,
}

impl ValidationSummary {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.syntax_valid
            && self.normalization_successful
            && self.translation_successful
            && self
                .file_existence
                .values()
                .all(|s| matches!(s, FileStatus::Found))
    }

    pub fn report(&self) {
        println!("[OK] Parsing configuration");
        if self.normalization_successful {
            println!("[OK] Normalizing configuration");
        } else {
            println!("[ERROR] Normalizing configuration");
        }

        for (file, status) in &self.file_existence {
            match status {
                FileStatus::Found => println!("[OK] Verifying data-source: {file}"),
                FileStatus::Missing => println!("[ERROR] Missing data-source: {file}"),
            }
        }

        if self.translation_successful {
            println!("[OK] Translating to Goose Attack");
        } else {
            println!("[ERROR] Translating to Goose Attack");
        }

        if self.is_valid() {
            println!("Validation Successful.");
        } else {
            println!("Validation Failed.");
        }
    }
}

pub async fn validate_config(config: &Configuration) -> ValidationSummary {
    let mut summary = ValidationSummary {
        syntax_valid: true, // If we have the config object, it's syntactically valid
        normalization_successful: true,
        ..Default::default()
    };

    // Check data sources
    let mut files_to_check: Vec<&DataSourceDefinition> = Vec::new();
    for scenario in config.scenarios.values() {
        if let Some(sources) = &scenario.data_sources {
            for source in sources {
                files_to_check.push(source);
            }
        }
    }
    summary.file_existence = validate_files(&files_to_check);

    // Validate environment variable references for sensitive patterns
    let env_issues = validate_env_refs(config);
    for issue in &env_issues {
        tracing::warn!("[SECURITY] {}", issue);
    }

    // Try translation
    match StateTranslator::translate(config, None, None).await {
        Ok(_) => summary.translation_successful = true,
        Err(e) => {
            summary.translation_successful = false;
            tracing::error!("Translation failed during validation: {}", e);
        }
    }

    summary
}

/// Scans configuration for `${env.VAR}` references and warns about
/// variables that match sensitive patterns.
#[must_use]
pub fn validate_env_refs(config: &Configuration) -> Vec<String> {
    let mut warnings = Vec::new();
    let re = Regex::new(r"\$\{env\.([A-Za-z0-9_]+)\}").unwrap();

    for (scenario_name, scenario) in &config.scenarios {
        for (idx, req) in scenario.requests.iter().enumerate() {
            let text = match req {
                HTTPRequestDefinition::Simple(u) => u.clone(),
                HTTPRequestDefinition::Detailed(d) => {
                    let mut s = d.url.clone();
                    if let Some(b) = &d.body {
                        s.push(' ');
                        s.push_str(b);
                    }
                    if let Some(bf) = &d.body_file {
                        s.push(' ');
                        s.push_str(bf);
                    }
                    s
                }
            };
            for cap in re.captures_iter(&text) {
                let var_name = &cap[1];
                if is_sensitive_var(var_name) {
                    warnings.push(format!(
                        "Scenario '{}', request {}: env var '${{env.{}}}' matches sensitive patterns",
                        scenario_name, idx, var_name
                    ));
                }
            }
        }
    }
    warnings
}

#[must_use]
pub fn validate_files(files: &[&DataSourceDefinition]) -> HashMap<String, FileStatus> {
    let mut existence = HashMap::new();
    for source in files {
        let path = source.path();
        if std::path::Path::new(path).exists() {
            existence.insert(path.to_string(), FileStatus::Found);
        } else {
            existence.insert(path.to_string(), FileStatus::Missing);
        }
    }
    existence
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::{
        Configuration, DataSourceDefinition, DetailedRequest, HTTPRequestDefinition,
        ScenarioDefinition,
    };
    use std::collections::HashMap;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_validate_env_refs_detects_sensitive_var_in_simple_request() {
        let mut scenarios = HashMap::new();
        scenarios.insert(
            "test_scenario".to_string(),
            ScenarioDefinition {
                requests: vec![HTTPRequestDefinition::Simple(
                    "https://${env.AWS_SECRET_KEY}.example.com/api".to_string(),
                )],
                weight: 1,
                think_time: None,
                data_sources: None,
                headers: None,
            },
        );
        let config = Configuration {
            execution: vec![],
            scenarios,
            reporting: vec![],
        };
        let warnings = validate_env_refs(&config);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("AWS_SECRET_KEY"));
        assert!(warnings[0].contains("test_scenario"));
        assert!(warnings[0].contains("request 0"));
    }

    #[test]
    fn test_validate_env_refs_skips_safe_vars() {
        let mut scenarios = HashMap::new();
        scenarios.insert(
            "safe".to_string(),
            ScenarioDefinition {
                requests: vec![HTTPRequestDefinition::Simple(
                    "https://${env.DEBUG}.example.com/api".to_string(),
                )],
                weight: 1,
                think_time: None,
                data_sources: None,
                headers: None,
            },
        );
        let config = Configuration {
            execution: vec![],
            scenarios,
            reporting: vec![],
        };
        let warnings = validate_env_refs(&config);
        assert!(
            warnings.is_empty(),
            "DEBUG should not be flagged as sensitive"
        );
    }

    #[test]
    fn test_validate_env_refs_checks_detailed_request_body_and_url() {
        let mut scenarios = HashMap::new();
        scenarios.insert(
            "detailed".to_string(),
            ScenarioDefinition {
                requests: vec![HTTPRequestDefinition::Detailed(Box::new(DetailedRequest {
                    url: "https://api.example.com/data".to_string(),
                    method: Some("POST".to_string()),
                    headers: None,
                    body: Some("{\"token\": \"${env.SECRET_TOKEN}\"}".to_string()),
                    label: None,
                    body_file: None,
                    timeout: None,
                    on_start: false,
                    think_time: None,
                    extract_jsonpath: None,
                    extract_regexp: None,
                    assert: vec![],
                    protocol: None,
                    message: None,
                    method_name: None,
                    execute_if: None,
                    loop_while: None,
                    ..Default::default()
                }))],

                weight: 1,
                think_time: None,
                data_sources: None,
                headers: None,
            },
        );
        let config = Configuration {
            execution: vec![],
            scenarios,
            reporting: vec![],
        };
        let warnings = validate_env_refs(&config);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("SECRET_TOKEN"));
    }

    #[test]
    fn test_validate_env_refs_multiple_requests_multiple_warnings() {
        let mut scenarios = HashMap::new();
        scenarios.insert(
            "multi".to_string(),
            ScenarioDefinition {
                requests: vec![
                    HTTPRequestDefinition::Simple("${env.API_KEY}/endpoint".to_string()),
                    HTTPRequestDefinition::Simple("${env.DB_PASSWORD}/db".to_string()),
                    HTTPRequestDefinition::Simple("https://${env.HOST}/health".to_string()),
                ],
                weight: 1,
                think_time: None,
                data_sources: None,
                headers: None,
            },
        );
        let config = Configuration {
            execution: vec![],
            scenarios,
            reporting: vec![],
        };
        let warnings = validate_env_refs(&config);
        // API_KEY and DB_PASSWORD should be flagged; HOST should not
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn test_file_existence_validation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.csv");
        File::create(&file_path).unwrap();

        let file_path_str = file_path.to_str().unwrap().to_string();
        let found_ds = DataSourceDefinition::Simple(file_path_str.clone());
        let missing_ds = DataSourceDefinition::Simple("missing.csv".to_string());

        let existence = validate_files(&[&found_ds, &missing_ds]);

        assert!(matches!(
            existence.get(file_path_str.as_str()).unwrap(),
            FileStatus::Found
        ));
        assert!(matches!(
            existence.get("missing.csv").unwrap(),
            FileStatus::Missing
        ));
    }
}
