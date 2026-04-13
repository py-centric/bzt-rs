use crate::models::config::Configuration;
use crate::translator::StateTranslator;
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
                FileStatus::Found => println!("[OK] Verifying data-source: {}", file),
                FileStatus::Missing => println!("[ERROR] Missing data-source: {}", file),
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

pub fn validate_config(config: &Configuration) -> ValidationSummary {
    let mut summary = ValidationSummary {
        syntax_valid: true, // If we have the config object, it's syntactically valid
        normalization_successful: true,
        ..Default::default()
    };

    // Check data sources
    let mut files_to_check = Vec::new();
    for scenario in config.scenarios.values() {
        if let Some(sources) = &scenario.data_sources {
            for source in sources {
                files_to_check.push(source.clone());
            }
        }
    }
    summary.file_existence = validate_files(&files_to_check);

    // Try translation
    match StateTranslator::translate(config, None) {
        Ok(_) => summary.translation_successful = true,
        Err(e) => {
            summary.translation_successful = false;
            tracing::error!("Translation failed during validation: {}", e);
        }
    }

    summary
}

pub fn validate_files(files: &[String]) -> HashMap<String, FileStatus> {
    let mut existence = HashMap::new();
    for file in files {
        if std::path::Path::new(file).exists() {
            existence.insert(file.clone(), FileStatus::Found);
        } else {
            existence.insert(file.clone(), FileStatus::Missing);
        }
    }
    existence
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_file_existence_validation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.csv");
        File::create(&file_path).unwrap();

        let file_path_str = file_path.to_str().unwrap().to_string();
        let missing_path = "missing.csv".to_string();

        let existence = validate_files(&[file_path_str.clone(), missing_path.clone()]);

        assert!(matches!(
            existence.get(&file_path_str).unwrap(),
            FileStatus::Found
        ));
        assert!(matches!(
            existence.get(&missing_path).unwrap(),
            FileStatus::Missing
        ));
    }
}
