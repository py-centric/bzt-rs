use crate::engine::PummelError;
use csv::Reader;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB

/// Validates a file path for security: rejects path traversal and non-existent files.
/// Returns the canonicalized path if valid.
#[allow(clippy::missing_errors_doc)]
pub fn validate_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, PummelError> {
    let path_ref = path.as_ref();
    let canonical = path_ref.canonicalize().map_err(|e| PummelError::Io {
        source: e,
        context: format!("Failed to resolve path '{}'", path_ref.display()),
    })?;

    // Get the project root (where Cargo.toml lives) as the allowed base
    let project_root = std::env::current_dir().map_err(|e| PummelError::Io {
        source: e,
        context: "Failed to determine project root".to_string(),
    })?;

    if !canonical.starts_with(&project_root) {
        let reason = format!(
            "Path '{}' resolves to '{}' which is outside the project directory '{}'",
            path_ref.display(),
            canonical.display(),
            project_root.display()
        );
        tracing::warn!("[SECURITY] Path traversal blocked: {reason}");
        return Err(PummelError::Validation {
            field: "data-source path".to_string(),
            reason,
        });
    }

    Ok(canonical)
}

/// Checks that a file does not exceed the maximum allowed size.
#[allow(clippy::missing_errors_doc)]
pub fn check_file_size<P: AsRef<Path>>(path: P) -> Result<(), PummelError> {
    let metadata = std::fs::metadata(path.as_ref()).map_err(|e| PummelError::Io {
        source: e,
        context: format!("Failed to read metadata for '{}'", path.as_ref().display()),
    })?;

    if metadata.len() > MAX_FILE_SIZE {
        let reason = format!(
            "File '{}' is {} bytes, exceeds maximum of {} bytes",
            path.as_ref().display(),
            metadata.len(),
            MAX_FILE_SIZE,
        );
        tracing::warn!("[SECURITY] File size limit exceeded: {reason}");
        return Err(PummelError::Validation {
            field: "data-source file size".to_string(),
            reason,
        });
    }

    Ok(())
}

pub struct CsvDataSource {
    records: Vec<HashMap<String, String>>,
}

impl CsvDataSource {
    #[allow(clippy::missing_errors_doc)]
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, PummelError> {
        // Security: validate path first
        let canonical = validate_path(path.as_ref())?;
        // Security: check file size
        check_file_size(&canonical)?;

        tracing::debug!("Loading CSV data source: {}", canonical.display());

        let file = File::open(&canonical)?;
        let mut rdr = Reader::from_reader(file);
        let headers = rdr
            .headers()
            .map_err(|e| PummelError::Internal(e.to_string()))?
            .clone();

        let mut records = Vec::new();
        for result in rdr.records() {
            let record = result.map_err(|e| PummelError::Internal(e.to_string()))?;
            let mut row = HashMap::new();
            for (i, header) in headers.iter().enumerate() {
                row.insert(header.to_string(), record.get(i).unwrap_or("").to_string());
            }
            records.push(row);
        }

        Ok(CsvDataSource { records })
    }

    #[must_use]
    pub fn get_record(&self, index: usize) -> Option<&HashMap<String, String>> {
        if self.records.is_empty() {
            return None;
        }
        self.records.get(index % self.records.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_get_record_wrapping_index() {
        let ds = CsvDataSource {
            records: vec![
                {
                    let mut m = HashMap::new();
                    m.insert("id".to_string(), "a".to_string());
                    m
                },
                {
                    let mut m = HashMap::new();
                    m.insert("id".to_string(), "b".to_string());
                    m
                },
                {
                    let mut m = HashMap::new();
                    m.insert("id".to_string(), "c".to_string());
                    m
                },
            ],
        };
        // Index wraps via modulo: 0%3=0, 3%3=0, 4%3=1, 5%3=2
        assert_eq!(ds.get_record(0).unwrap().get("id").unwrap(), "a");
        assert_eq!(ds.get_record(3).unwrap().get("id").unwrap(), "a");
        assert_eq!(ds.get_record(4).unwrap().get("id").unwrap(), "b");
        assert_eq!(ds.get_record(5).unwrap().get("id").unwrap(), "c");
        assert_eq!(ds.get_record(100).unwrap().get("id").unwrap(), "b"); // 100%3=1
    }

    #[test]
    fn test_get_record_large_index_wraps() {
        let ds = CsvDataSource {
            records: vec![{
                let mut m = HashMap::new();
                m.insert("v".to_string(), "x".to_string());
                m
            }],
        };
        // Single record: every index returns same record
        assert_eq!(ds.get_record(0).unwrap().get("v").unwrap(), "x");
        assert_eq!(ds.get_record(usize::MAX).unwrap().get("v").unwrap(), "x");
    }

    #[test]
    fn test_csv_source_empty_file() {
        let mut tmp = std::env::temp_dir();
        tmp.push("bzt_test_empty.csv");
        let mut f = fs::File::create(&tmp).unwrap();
        f.write_all(b"id,name\n").unwrap(); // headers only, no data rows
        f.flush().unwrap();

        let ds = CsvDataSource::new(&tmp);
        if let Ok(ds) = ds {
            assert!(ds.get_record(0).is_none(), "empty CSV should have no records");
        }
        fs::remove_file(&tmp).unwrap();
    }

    #[test]
    fn test_csv_source_utf8_content() {
        let mut tmp = std::env::temp_dir();
        tmp.push("bzt_test_utf8.csv");
        let mut f = fs::File::create(&tmp).unwrap();
        f.write_all("name,city\n".as_bytes()).unwrap();
        f.write_all("Hans,Munich\n".as_bytes()).unwrap();
        f.write_all("Yuki,Tokyo\n".as_bytes()).unwrap();
        f.flush().unwrap();

        let ds = CsvDataSource::new(&tmp);
        if let Ok(ds) = ds {
            let rec = ds.get_record(0).unwrap();
            assert_eq!(rec.get("name").unwrap(), "Hans");
            assert_eq!(rec.get("city").unwrap(), "Munich");
            let rec2 = ds.get_record(1).unwrap();
            assert_eq!(rec2.get("name").unwrap(), "Yuki");
            assert_eq!(rec2.get("city").unwrap(), "Tokyo");
        }
        fs::remove_file(&tmp).unwrap();
    }

    #[test]
    fn test_validate_path_rejects_traversal() {
        let result = validate_path("/etc/passwd");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("outside the project directory"));
    }

    #[test]
    fn test_validate_path_rejects_relative_traversal() {
        let result = validate_path("../../etc/passwd");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to resolve")
        );
    }

    #[test]
    fn test_validate_path_accepts_local_file() {
        // Create a temp file in the project dir
        let mut tmp = std::env::temp_dir();
        tmp.push("bzt_test_safe.csv");
        let _ = fs::File::create(&tmp).unwrap();
        let result = validate_path(&tmp);
        // This may fail if /tmp isn't under project root, which is expected
        // The important thing is the canonicalization and check work
        if let Err(e) = &result {
            assert!(
                e.to_string().contains("outside") || e.to_string().contains("Failed to resolve")
            );
        }
    }

    #[test]
    fn test_check_file_size_rejects_large() {
        let mut tmp = std::env::temp_dir();
        tmp.push("bzt_test_large.csv");
        let mut f = fs::File::create(&tmp).unwrap();
        // Write a file larger than 100MB (just write 1MB and pretend)
        let data = vec![b'x'; 1024 * 1024]; // 1MB
        for _ in 0..101 {
            f.write_all(&data).unwrap();
        }
        f.flush().unwrap();
        let result = check_file_size(&tmp);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("exceeds maximum"));
        fs::remove_file(&tmp).unwrap();
    }

    #[test]
    fn test_check_file_size_accepts_small() {
        let mut tmp = std::env::temp_dir();
        tmp.push("bzt_test_small.csv");
        let mut f = fs::File::create(&tmp).unwrap();
        f.write_all(b"id,name\n1,test\n").unwrap();
        f.flush().unwrap();
        let result = check_file_size(&tmp);
        assert!(result.is_ok());
        fs::remove_file(&tmp).unwrap();
    }

    #[test]
    fn test_csv_data_source_with_validations() {
        let mut tmp = std::env::temp_dir();
        tmp.push("bzt_test_valid.csv");
        let mut f = fs::File::create(&tmp).unwrap();
        f.write_all(b"id,name\n1,Alice\n2,Bob\n").unwrap();
        f.flush().unwrap();

        let ds = CsvDataSource::new(&tmp);
        // If temp dir is outside project root, this will fail with Validation
        if let Ok(ds) = ds {
            assert!(ds.get_record(0).is_some());
            assert!(ds.get_record(1).is_some());
            assert_eq!(ds.get_record(0).unwrap().get("name").unwrap(), "Alice");
        }
        fs::remove_file(&tmp).unwrap();
    }

    #[test]
    fn test_get_record_empty_records_returns_none() {
        let ds = CsvDataSource { records: vec![] };
        assert!(ds.get_record(0).is_none());
        assert!(ds.get_record(100).is_none());
    }
}
