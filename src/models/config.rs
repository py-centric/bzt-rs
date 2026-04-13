use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Configuration {
    pub execution: Vec<ExecutionPlan>,
    pub scenarios: HashMap<String, ScenarioDefinition>,
    #[serde(default)]
    pub reporting: Vec<ReportingDefinition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionPlan {
    pub concurrency: usize,
    #[serde(rename = "ramp-up")]
    pub ramp_up: String,
    #[serde(rename = "hold-for")]
    pub hold_for: String,
    pub scenario: String,
    pub throughput: Option<usize>,
    pub steps: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScenarioDefinition {
    pub requests: Vec<HTTPRequestDefinition>,
    #[serde(default = "default_weight")]
    pub weight: usize,
    #[serde(rename = "think-time")]
    pub think_time: Option<String>,
    #[serde(rename = "data-sources")]
    pub data_sources: Option<Vec<String>>,
}

fn default_weight() -> usize {
    1
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum HTTPRequestDefinition {
    Simple(String),
    Detailed(Box<DetailedRequest>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetailedRequest {
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
    #[serde(default)]
    pub on_start: bool,
    #[serde(rename = "think-time")]
    pub think_time: Option<String>,
    #[serde(rename = "extract-jsonpath")]
    pub extract_jsonpath: Option<HashMap<String, String>>,
    #[serde(rename = "extract-regexp")]
    pub extract_regexp: Option<HashMap<String, String>>,
    #[serde(default)]
    pub assert: Vec<AssertionDefinition>,
    /// Protocol override (e.g. "websocket", "grpc")
    pub protocol: Option<String>,
    /// WebSocket message to send
    pub message: Option<String>,
    /// gRPC service method
    pub method_name: Option<String>,
    /// Conditional execution: variable_name == value
    #[serde(rename = "if")]
    pub execute_if: Option<String>,
    /// Loop execution: while variable_name == value
    #[serde(rename = "loop")]
    pub loop_while: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
pub struct ReportingDefinition {
    pub module: String,
    pub filename: Option<String>,
    #[serde(rename = "failed-threshold")]
    pub failed_threshold: Option<f32>,
}
