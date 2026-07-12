pub(crate) mod control_flow;
pub(crate) mod data_sources;
pub(crate) mod env;
pub(crate) mod extraction;
pub mod goose;
#[cfg(feature = "grpc")]
pub(crate) mod grpc_dynamic;
pub(crate) mod interpolation;
pub(crate) mod macros;
#[cfg(feature = "grpc")]
pub mod mock;
pub(crate) mod pacing;
#[cfg(feature = "grpc")]
pub(crate) mod proto;
pub mod reporting;
pub(crate) mod sla;
pub(crate) mod utils;
pub mod validation;

#[derive(Debug, thiserror::Error)]
pub enum BztError {
    #[error("[IO] {context}: {source}")]
    Io {
        source: std::io::Error,
        context: String,
    },
    #[error("[SERDE] {message}{}", file_path.as_ref().map(|p| format!(" in '{p}'")).unwrap_or_default())]
    Serde {
        message: String,
        file_path: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + 'static>>,
    },
    #[error("[GOOSE] {0}")]
    Goose(Box<dyn std::error::Error + Send>),
    #[error("[VALIDATION] Field '{field}': {reason}")]
    Validation { field: String, reason: String },
    #[error("[MOCK] {reason}")]
    Mock { reason: String },
    #[error("[NETWORK] {host}: {operation} failed — {details}")]
    Network {
        host: String,
        operation: String,
        details: String,
    },
    #[error("[SLA] {metric} {actual:.2} exceeds threshold {threshold:.2}")]
    SlaViolation {
        actual: f32,
        threshold: f32,
        metric: String,
    },
    #[error("[ENV] Variable '{var}': {reason}")]
    Environment { var: String, reason: String },
    #[error("[NOT IMPLEMENTED] {feature} is not yet available")]
    NotImplemented { feature: String },
    #[error("[INTERNAL] {0}")]
    Internal(String),
}

impl From<std::io::Error> for BztError {
    fn from(source: std::io::Error) -> Self {
        BztError::Io {
            source,
            context: "operation failed".to_string(),
        }
    }
}

impl From<serde_json::Error> for BztError {
    fn from(e: serde_json::Error) -> Self {
        BztError::Serde {
            message: e.to_string(),
            file_path: None,
            source: Some(Box::new(e)),
        }
    }
}

impl From<serde_yaml::Error> for BztError {
    fn from(e: serde_yaml::Error) -> Self {
        BztError::Serde {
            message: e.to_string(),
            file_path: None,
            source: None,
        }
    }
}

impl From<toml::de::Error> for BztError {
    fn from(e: toml::de::Error) -> Self {
        BztError::Serde {
            message: e.to_string(),
            file_path: None,
            source: Some(Box::new(e)),
        }
    }
}

pub fn init_logging() {
    tracing_subscriber::fmt::init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging() {
        init_logging();
    }

    #[test]
    fn test_not_implemented_display() {
        let err = BztError::NotImplemented {
            feature: "distributed mode".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("NOT IMPLEMENTED"));
        assert!(msg.contains("distributed mode"));
    }

    #[test]
    fn test_validation_display() {
        let err = BztError::Validation {
            field: "concurrency".to_string(),
            reason: "must be > 0".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("VALIDATION"));
        assert!(msg.contains("concurrency"));
    }

    #[test]
    fn test_sla_violation_display() {
        let err = BztError::SlaViolation {
            metric: "fail_rate".to_string(),
            actual: 0.15,
            threshold: 0.1,
        };
        let msg = err.to_string();
        assert!(msg.contains("SLA"));
        assert!(msg.contains("0.15"));
        assert!(msg.contains("0.10"));
    }

    #[test]
    fn test_env_error_display() {
        let err = BztError::Environment {
            var: "AWS_SECRET_KEY".to_string(),
            reason: "blocked".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("ENV"));
        assert!(msg.contains("AWS_SECRET_KEY"));
    }

    #[test]
    fn test_serde_error_from_json_has_source() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid:}").unwrap_err();
        let bzt_err: BztError = json_err.into();
        let msg = bzt_err.to_string();
        assert!(msg.contains("[SERDE]"));
        // source() should return Some for json errors
        assert!(std::error::Error::source(&bzt_err).is_some());
    }

    #[test]
    fn test_serde_error_from_toml_has_source() {
        let toml_err = toml::from_str::<toml::Value>("invalid = [").unwrap_err();
        let bzt_err: BztError = toml_err.into();
        let msg = bzt_err.to_string();
        assert!(msg.contains("[SERDE]"));
        assert!(std::error::Error::source(&bzt_err).is_some());
    }

    #[test]
    fn test_serde_error_from_yaml_no_source() {
        let yaml_err = serde_yaml::from_str::<serde_yaml::Value>("'unclosed").unwrap_err();
        let bzt_err: BztError = yaml_err.into();
        let msg = bzt_err.to_string();
        assert!(msg.contains("[SERDE]"));
        // serde_yaml::Error is Box<dyn Error> without Send, so source is None
        assert!(std::error::Error::source(&bzt_err).is_none());
    }

    #[test]
    fn test_io_error_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let bzt_err: BztError = io_err.into();
        let msg = bzt_err.to_string();
        assert!(msg.contains("[IO]"));
        assert!(std::error::Error::source(&bzt_err).is_some());
    }

    #[test]
    fn test_networking_error_display() {
        let err = BztError::Network {
            host: "api.example.com".to_string(),
            operation: "connect".to_string(),
            details: "connection refused".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("NETWORK"));
        assert!(msg.contains("api.example.com"));
    }

    #[test]
    fn test_mock_error_display() {
        let err = BztError::Mock {
            reason: "server not started".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("MOCK"));
        assert!(msg.contains("server not started"));
    }

    #[test]
    fn test_internal_error_display() {
        let err = BztError::Internal("unexpected state".to_string());
        let msg = err.to_string();
        assert!(msg.contains("INTERNAL"));
        assert!(msg.contains("unexpected state"));
    }

    #[test]
    fn test_goose_error_display() {
        let err = BztError::Goose(Box::new(std::io::Error::other("goose failed")));
        let msg = err.to_string();
        assert!(msg.contains("GOOSE"));
        assert!(msg.contains("goose failed"));
    }
}
