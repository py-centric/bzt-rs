use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Http,
    Websocket,
    #[serde(alias = "ws")]
    Ws,
    Grpc,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum GrpcMode {
    #[serde(alias = "server")]
    ServerStreaming,
    #[serde(alias = "client")]
    ClientStreaming,
    #[serde(alias = "bidi")]
    BidiStreaming,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub execution: Vec<ExecutionPlan>,
    pub scenarios: HashMap<String, ScenarioDefinition>,
    #[serde(default)]
    pub reporting: Vec<ReportingDefinition>,
    #[serde(default)]
    pub services: Vec<ServiceDefinition>,
    #[serde(default)]
    pub api: Option<ApiConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPlan {
    pub concurrency: usize,
    #[serde(rename = "ramp-up")]
    pub ramp_up: String,
    #[serde(rename = "hold-for")]
    pub hold_for: String,
    pub scenario: String,
    pub throughput: Option<usize>,
    pub steps: Option<usize>,
    #[serde(default)]
    pub pacing: Option<PacingConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDefinition {
    pub requests: Vec<HTTPRequestDefinition>,
    #[serde(default = "default_weight")]
    pub weight: usize,
    #[serde(rename = "think-time")]
    pub think_time: Option<String>,
    #[serde(rename = "data-sources")]
    pub data_sources: Option<Vec<DataSourceDefinition>>,
    /// Scenario-level headers applied to all requests in this scenario.
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
}

fn default_weight() -> usize {
    1
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DataSourceDefinition {
    Simple(String),
    Structured(StructuredDataSource),
}

impl DataSourceDefinition {
    #[must_use]
    pub fn path(&self) -> &str {
        match self {
            DataSourceDefinition::Simple(p) => p,
            DataSourceDefinition::Structured(s) => &s.path,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct StructuredDataSource {
    pub path: String,
    #[serde(default)]
    pub delimiter: Option<String>,
    #[serde(default)]
    pub quoted: bool,
    #[serde(default)]
    pub loop_data: bool,
    #[serde(default)]
    pub ordered: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum HTTPRequestDefinition {
    Simple(String),
    Detailed(Box<DetailedRequest>),
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct DetailedRequest {
    pub url: String,
    #[serde(default)]
    pub method: Option<HttpMethod>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
    /// Human-readable label for metrics/reporting.
    pub label: Option<String>,
    /// File path to read request body from.
    #[serde(rename = "body-file")]
    pub body_file: Option<String>,
    /// Request timeout (e.g. "30s", "5000ms").
    pub timeout: Option<String>,
    #[serde(default)]
    pub on_start: bool,
    #[serde(rename = "think-time")]
    pub think_time: Option<String>,
    #[serde(rename = "extract-jsonpath")]
    pub extract_jsonpath: Option<HashMap<String, String>>,
    #[serde(rename = "extract-regexp")]
    pub extract_regexp: Option<HashMap<String, String>>,
    #[serde(rename = "extract-xpath")]
    pub extract_xpath: Option<HashMap<String, String>>,
    #[serde(default)]
    pub assert: Vec<AssertionDefinition>,
    /// Protocol override (e.g. "websocket", "grpc")
    #[serde(default)]
    pub protocol: Option<Protocol>,
    /// WebSocket message to send
    pub message: Option<String>,
    /// gRPC service method
    pub method_name: Option<String>,
    #[serde(rename = "grpc-mode")]
    #[serde(default)]
    pub grpc_mode: Option<GrpcMode>,
    #[serde(default)]
    pub messages: Vec<String>,
    /// Conditional execution: `variable_name` == value
    #[serde(rename = "if")]
    pub execute_if: Option<String>,
    /// Loop execution: while `variable_name` == value
    #[serde(rename = "loop")]
    pub loop_while: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PacingConfig {
    pub rate: usize,
    #[serde(default = "default_pacing_per")]
    pub per: String,
    #[serde(default)]
    pub randomize: bool,
}

fn default_pacing_per() -> String {
    "1s".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum SlaMetric {
    #[serde(rename = "fail-rate")]
    FailRate,
    #[serde(rename = "avg-response-time")]
    AvgResponseTime,
    #[serde(rename = "p90-response-time")]
    P90ResponseTime,
    #[serde(rename = "p95-response-time")]
    P95ResponseTime,
    #[serde(rename = "p99-response-time")]
    P99ResponseTime,
    #[serde(rename = "throughput")]
    Throughput,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum SlaAction {
    #[default]
    Stop,
    Warn,
    Continue,
    Exec(String),
}

impl<'de> serde::Deserialize<'de> for SlaAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "stop" => Ok(SlaAction::Stop),
            "warn" => Ok(SlaAction::Warn),
            "continue" => Ok(SlaAction::Continue),
            other => {
                if let Some(stripped) = other.strip_prefix("exec:") {
                    Ok(SlaAction::Exec(stripped.to_string()))
                } else {
                    Ok(SlaAction::Exec(other.to_string()))
                }
            }
        }
    }
}

impl serde::Serialize for SlaAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            SlaAction::Stop => serializer.serialize_str("stop"),
            SlaAction::Warn => serializer.serialize_str("warn"),
            SlaAction::Continue => serializer.serialize_str("continue"),
            SlaAction::Exec(cmd) => serializer.serialize_str(&format!("exec:{cmd}")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SlaCriterion {
    pub metric: SlaMetric,
    pub threshold: f32,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub duration: Option<String>,
    #[serde(default)]
    pub action: SlaAction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct AssertionDefinition {
    pub contains: Vec<String>,
    #[serde(default = "default_subject")]
    pub subject: String,
    #[serde(default)]
    pub regexp: bool,
    #[serde(default)]
    pub not: bool,
}

fn default_subject() -> String {
    "body".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ReportingDefinition {
    pub module: String,
    pub filename: Option<String>,
    pub url: Option<String>,
    pub token: Option<String>,
    pub org: Option<String>,
    pub bucket: Option<String>,
    pub interval: Option<String>,
    #[serde(rename = "failed-threshold")]
    pub failed_threshold: Option<f32>,
    #[serde(default)]
    pub sla: Vec<SlaCriterion>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ServiceDefinition {
    pub module: String,
    #[serde(default)]
    pub prepare: Vec<String>,
    #[serde(default)]
    pub startup: Vec<String>,
    #[serde(default)]
    pub shutdown: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ApiConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_api_port")]
    pub port: u16,
}

fn default_api_port() -> u16 {
    8000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pacing_config_default_per() {
        let config: PacingConfig = serde_yaml::from_str("rate: 10").unwrap();
        assert_eq!(config.rate, 10);
        assert_eq!(config.per, "1s");
        assert!(!config.randomize);
    }

    #[test]
    fn test_pacing_config_full() {
        let yaml = "rate: 50\nper: 1m\nrandomize: true";
        let config: PacingConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.rate, 50);
        assert_eq!(config.per, "1m");
        assert!(config.randomize);
    }

    #[test]
    fn test_sla_action_default_stop() {
        let criterion: SlaCriterion =
            serde_yaml::from_str("metric: fail-rate\nthreshold: 0.1").unwrap();
        assert!(matches!(criterion.action, SlaAction::Stop));
    }

    #[test]
    fn test_sla_criterion_full() {
        let yaml = "\
metric: avg-response-time
threshold: 500.0
subject: /api/status
duration: 30s
action: warn";
        let criterion: SlaCriterion = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(criterion.metric, SlaMetric::AvgResponseTime));
        assert_eq!(criterion.threshold, 500.0);
        assert_eq!(criterion.subject.unwrap(), "/api/status");
        assert!(matches!(criterion.action, SlaAction::Warn));
    }

    #[test]
    fn test_sla_metric_serde() {
        assert!(matches!(
            serde_yaml::from_str::<SlaMetric>("fail-rate").unwrap(),
            SlaMetric::FailRate
        ));
        assert!(matches!(
            serde_yaml::from_str::<SlaMetric>("throughput").unwrap(),
            SlaMetric::Throughput
        ));
    }

    #[test]
    fn test_data_source_definition_simple() {
        let ds: DataSourceDefinition = serde_yaml::from_str("users.csv").unwrap();
        let path = ds.path();
        match ds {
            DataSourceDefinition::Simple(ref p) => assert_eq!(p, "users.csv"),
            _ => panic!("Expected Simple variant"),
        }
        assert_eq!(path, "users.csv");
    }

    #[test]
    fn test_data_source_definition_structured() {
        let yaml = "\
path: data.csv
delimiter: ';'
quoted: true
loop_data: true
ordered: false";
        let ds: DataSourceDefinition = serde_yaml::from_str(yaml).unwrap();
        let path = ds.path();
        match ds {
            DataSourceDefinition::Structured(ref s) => {
                assert_eq!(s.path, "data.csv");
                assert_eq!(s.delimiter.as_deref().unwrap(), ";");
                assert!(s.quoted);
                assert!(s.loop_data);
                assert!(!s.ordered);
            }
            _ => panic!("Expected Structured variant"),
        }
        assert_eq!(path, "data.csv");
    }

    #[test]
    fn test_detailed_request_label() {
        let yaml = "url: /api/test\nlabel: login";
        let req: DetailedRequest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(req.label.unwrap(), "login");
    }

    #[test]
    fn test_detailed_request_timeout() {
        let yaml = "url: /api/test\ntimeout: 30s";
        let req: DetailedRequest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(req.timeout.unwrap(), "30s");
    }

    #[test]
    fn test_detailed_request_body_file() {
        let yaml = "url: /api/test\nbody-file: payload.json";
        let req: DetailedRequest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(req.body_file.unwrap(), "payload.json");
    }

    #[test]
    fn test_detailed_request_deny_unknown() {
        let yaml = "url: /api/test\nunknown_field: true";
        let result: Result<DetailedRequest, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_scenario_headers() {
        let yaml = "\
requests:
- /api/test
headers:
  Authorization: Bearer token123
  X-Custom: value";
        let scenario: ScenarioDefinition = serde_yaml::from_str(yaml).unwrap();
        let headers = scenario.headers.unwrap();
        assert_eq!(headers.get("Authorization").unwrap(), "Bearer token123");
        assert_eq!(headers.get("X-Custom").unwrap(), "value");
    }
}
