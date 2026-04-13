use crate::models::config::{
    Configuration, DetailedRequest, ExecutionPlan, HTTPRequestDefinition, ScenarioDefinition,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ShorthandConfiguration {
    pub execution: ShorthandExecution,
    pub scenarios: HashMap<String, ShorthandScenario>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ShorthandExecution {
    Single(ShorthandExecutionPlan),
    Multiple(Vec<ShorthandExecutionPlan>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShorthandExecutionPlan {
    pub concurrency: usize,
    #[serde(default = "default_ramp_up")]
    pub ramp_up: String,
    #[serde(default = "default_hold_for")]
    pub hold_for: String,
    pub scenario: String,
}

fn default_ramp_up() -> String {
    "0s".to_string()
}
fn default_hold_for() -> String {
    "10s".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShorthandScenario {
    pub requests: Vec<String>,
}

pub struct SchemaNormalizer;

impl SchemaNormalizer {
    #[must_use]
    pub fn normalize_shorthand(shorthand: ShorthandConfiguration) -> Configuration {
        let execution = match shorthand.execution {
            ShorthandExecution::Single(plan) => vec![ExecutionPlan {
                concurrency: plan.concurrency,
                ramp_up: plan.ramp_up,
                hold_for: plan.hold_for,
                scenario: plan.scenario,
                throughput: None,
                steps: None,
            }],
            ShorthandExecution::Multiple(plans) => plans
                .into_iter()
                .map(|plan| ExecutionPlan {
                    concurrency: plan.concurrency,
                    ramp_up: plan.ramp_up,
                    hold_for: plan.hold_for,
                    scenario: plan.scenario,
                    throughput: None,
                    steps: None,
                })
                .collect(),
        };

        let mut scenarios = HashMap::new();

        // First pass: flat copy
        for (name, s) in &shorthand.scenarios {
            scenarios.insert(
                name.clone(),
                ScenarioDefinition {
                    requests: s
                        .requests
                        .iter()
                        .map(|r| HTTPRequestDefinition::Simple(r.clone()))
                        .collect(),
                    weight: 1,
                    think_time: None,
                    data_sources: None,
                },
            );
        }

        // Second pass: resolve hierarchies (dot notation)
        let keys: Vec<String> = scenarios.keys().cloned().collect();
        for name in keys {
            if name.contains('.') {
                let parts: Vec<&str> = name.split('.').collect();
                let parent_name = parts[0];
                if let (Some(parent), Some(child)) = (
                    scenarios.get(parent_name).cloned(),
                    scenarios.get_mut(&name),
                ) {
                    // Prepend parent requests as on_start if they aren't already
                    let mut new_requests = Vec::new();
                    for req in parent.requests {
                        match req {
                            HTTPRequestDefinition::Simple(url) => {
                                new_requests.push(HTTPRequestDefinition::Detailed(Box::new(
                                    DetailedRequest {
                                        url,
                                        method: None,
                                        headers: None,
                                        body: None,
                                        on_start: true,
                                        think_time: None,
                                        extract_jsonpath: None,
                                        extract_regexp: None,
                                        assert: Vec::new(),
                                        protocol: None,
                                        message: None,
                                        method_name: None,
                                        execute_if: None,
                                        loop_while: None,
                                    },
                                )));
                            }
                            HTTPRequestDefinition::Detailed(detailed) => {
                                let mut d = detailed.as_ref().clone();
                                d.on_start = true;
                                new_requests.push(HTTPRequestDefinition::Detailed(Box::new(d)));
                            }
                        }
                    }
                    new_requests.append(&mut child.requests);
                    child.requests = new_requests;
                }
            }
        }

        Configuration {
            execution,
            scenarios,
            reporting: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchical_resolution() {
        let mut scenarios = HashMap::new();
        scenarios.insert(
            "auth".to_string(),
            ShorthandScenario {
                requests: vec!["/login".to_string()],
            },
        );
        scenarios.insert(
            "auth.search".to_string(),
            ShorthandScenario {
                requests: vec!["/search".to_string()],
            },
        );

        let shorthand = ShorthandConfiguration {
            execution: ShorthandExecution::Single(ShorthandExecutionPlan {
                concurrency: 1,
                ramp_up: "0s".to_string(),
                hold_for: "1s".to_string(),
                scenario: "auth.search".to_string(),
            }),
            scenarios,
        };

        let config = SchemaNormalizer::normalize_shorthand(shorthand);
        let resolved = config.scenarios.get("auth.search").unwrap();

        assert_eq!(resolved.requests.len(), 2);
        match &resolved.requests[0] {
            HTTPRequestDefinition::Detailed(d) => {
                assert_eq!(d.url, "/login");
                assert!(d.on_start);
            }
            _ => panic!("Expected detailed request"),
        }
        match &resolved.requests[1] {
            HTTPRequestDefinition::Simple(url) => assert_eq!(url, "/search"),
            _ => panic!("Expected simple request"),
        }
    }

    #[test]
    fn test_multiple_execution_plans() {
        let shorthand = ShorthandConfiguration {
            execution: ShorthandExecution::Multiple(vec![
                ShorthandExecutionPlan {
                    concurrency: 5,
                    ramp_up: "10s".to_string(),
                    hold_for: "1m".to_string(),
                    scenario: "s1".to_string(),
                },
                ShorthandExecutionPlan {
                    concurrency: 10,
                    ramp_up: "5s".to_string(),
                    hold_for: "30s".to_string(),
                    scenario: "s2".to_string(),
                },
            ]),
            scenarios: HashMap::new(),
        };
        let config = SchemaNormalizer::normalize_shorthand(shorthand);
        assert_eq!(config.execution.len(), 2);
        assert_eq!(config.execution[0].concurrency, 5);
        assert_eq!(config.execution[1].concurrency, 10);
    }
}
