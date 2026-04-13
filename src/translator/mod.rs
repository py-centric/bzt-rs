use crate::engine::control_flow::AssertionEngine;
use crate::engine::data_sources::CsvDataSource;
use crate::engine::extraction::{ExtractionEngine, UserSession};
use crate::engine::interpolation::Interpolator;
use crate::engine::macros::MacroEvaluator;
use crate::models::config::{Configuration, HTTPRequestDefinition};
use goose::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub struct StateTranslator;

impl StateTranslator {
    pub fn translate(config: &Configuration) -> Result<GooseAttack, String> {
        let configuration = goose::config::GooseConfiguration::default();
        let mut attack =
            GooseAttack::initialize_with_config(configuration).map_err(|e| e.to_string())?;

        // Load data sources
        let mut data_sources = HashMap::new();
        for (name, scenario_def) in &config.scenarios {
            if let Some(sources) = &scenario_def.data_sources {
                let mut loaded = Vec::new();
                for path in sources {
                    let ds = CsvDataSource::new(path)?;
                    loaded.push(Arc::new(ds));
                }
                data_sources.insert(name.clone(), loaded);
            }
        }
        let data_sources = Arc::new(data_sources);

        for exec in &config.execution {
            let hatch_rate = parse_hatch_rate(&exec.ramp_up, exec.concurrency);

            attack = *attack
                .set_default(GooseDefault::Users, exec.concurrency)
                .map_err(|e| e.to_string())?;
            attack = *attack
                .set_default(GooseDefault::RunTime, parse_duration(&exec.hold_for))
                .map_err(|e| e.to_string())?;
            attack = *attack
                .set_default(GooseDefault::HatchRate, hatch_rate.as_str())
                .map_err(|e| e.to_string())?;
            attack = *attack
                .set_default(GooseDefault::Host, "http://localhost")
                .map_err(|e| e.to_string())?;

            if let Some(scenario_def) = config.scenarios.get(&exec.scenario) {
                let scenario_name = exec.scenario.clone();
                let mut scenario = Scenario::new(&scenario_name);

                if scenario_def.weight > 1 {
                    scenario = scenario
                        .set_weight(scenario_def.weight)
                        .map_err(|e| e.to_string())?;
                }

                // Handle scenario-level think-time
                if let Some(tt) = &scenario_def.think_time {
                    let (min, max) = parse_think_time(tt);
                    scenario = scenario
                        .set_wait_time(
                            Duration::from_millis(min as u64),
                            Duration::from_millis(max as u64),
                        )
                        .map_err(|e| e.to_string())?;
                }

                // Add a setup transaction to initialize UserSession
                scenario = scenario.register_transaction(
                    Transaction::new(Arc::new(|user| {
                        Box::pin(async move {
                            user.set_session_data(UserSession::default());
                            Ok(())
                        })
                    }))
                    .set_on_start(),
                );

                let ds_map = Arc::clone(&data_sources);
                let scenario_name_clone = scenario_name.clone();

                for req in &scenario_def.requests {
                    let req_clone = req.clone();
                    let ds_map = Arc::clone(&ds_map);
                    let scenario_name = scenario_name_clone.clone();

                    let mut transaction = Transaction::new(Arc::new(move |user| {
                        let req = req_clone.clone();
                        let ds_map = Arc::clone(&ds_map);
                        let scenario_name = scenario_name.clone();

                        Box::pin(async move {
                            let mut variables = HashMap::new();
                            if let Some(session) = user.get_session_data::<UserSession>() {
                                variables = session.variables.clone();
                            }

                            // Get record from data source if available
                            let mut record = None;
                            if let Some(ds_list) = ds_map.get(&scenario_name)
                                && !ds_list.is_empty()
                            {
                                record = ds_list[0].get_record(user.weighted_users_index);
                            }

                            match req {
                                HTTPRequestDefinition::Simple(url) => {
                                    let url = Interpolator::interpolate(&url, &variables);
                                    let final_url = MacroEvaluator::evaluate(&url, record);
                                    let _ = user.get(&final_url).await?;
                                }
                                HTTPRequestDefinition::Detailed(d) => {
                                    // REQ-5.4: Control Flow (if)
                                    if let Some(cond) = &d.execute_if
                                        && !evaluate_condition(cond, &variables)
                                    {
                                        return Ok(());
                                    }

                                    let url = Interpolator::interpolate(&d.url, &variables);
                                    let final_url = MacroEvaluator::evaluate(&url, record);

                                    // REQ-5.4: Control Flow (loop)
                                    let mut loop_count = 0;
                                    loop {
                                        if let Some(cond) = &d.loop_while
                                            && (!evaluate_condition(cond, &variables)
                                                || loop_count > 100)
                                        {
                                            break;
                                        }

                                        match d.protocol.as_deref() {
                                            Some("websocket") | Some("ws") => {
                                                if let Some(msg) = &d.message {
                                                    let final_msg =
                                                        Interpolator::interpolate(msg, &variables);
                                                    let final_msg = MacroEvaluator::evaluate(
                                                        &final_msg, record,
                                                    );
                                                    println!(
                                                        "WS: Sending {} to {}",
                                                        final_msg, final_url
                                                    );
                                                }
                                            }
                                            Some("grpc") => {
                                                if let Some(method) = &d.method_name {
                                                    println!(
                                                        "gRPC: Calling {} on {}",
                                                        method, final_url
                                                    );
                                                }
                                            }
                                            _ => {
                                                let method = d
                                                    .method
                                                    .clone()
                                                    .unwrap_or_else(|| "GET".to_string());
                                                let body = d
                                                    .body
                                                    .as_ref()
                                                    .map(|b| {
                                                        Interpolator::interpolate(b, &variables)
                                                    })
                                                    .unwrap_or_default();
                                                let final_body =
                                                    MacroEvaluator::evaluate(&body, record);

                                                let goose_response = if method == "POST" {
                                                    user.post(&final_url, final_body).await?
                                                } else {
                                                    user.get(&final_url).await?
                                                };

                                                if let Ok(response) = &goose_response.response {
                                                    let status = response.status().as_u16();
                                                    let text = goose_response
                                                        .response
                                                        .unwrap()
                                                        .text()
                                                        .await
                                                        .unwrap_or_default();

                                                    for a in &d.assert {
                                                        if let Err(e) =
                                                            AssertionEngine::check_assertion(
                                                                &text, status, a,
                                                            )
                                                        {
                                                            let mut req =
                                                                goose_response.request.clone();
                                                            let _ = user.set_failure(
                                                                "Assertion Failed",
                                                                &mut req,
                                                                None,
                                                                Some(&e),
                                                            );
                                                        }
                                                    }

                                                    if let Some(session) =
                                                        user.get_session_data_mut::<UserSession>()
                                                    {
                                                        if let Some(rules) = &d.extract_jsonpath {
                                                            ExtractionEngine::extract_jsonpath(
                                                                &text, rules, session,
                                                            );
                                                        }
                                                        if let Some(rules) = &d.extract_regexp {
                                                            ExtractionEngine::extract_regex(
                                                                &text, rules, session,
                                                            );
                                                        }
                                                        // Update local variables for loop condition
                                                        variables = session.variables.clone();
                                                    }
                                                }
                                            }
                                        }

                                        if d.loop_while.is_none() {
                                            break;
                                        }
                                        loop_count += 1;
                                    }
                                }
                            }
                            Ok(())
                        })
                    }));

                    if let HTTPRequestDefinition::Detailed(d) = req
                        && d.on_start
                    {
                        transaction = transaction.set_on_start();
                    }

                    scenario = scenario.register_transaction(transaction);
                }
                attack = attack.register_scenario(scenario);
            }
        }

        Ok(attack)
    }
}

fn evaluate_condition(cond: &str, variables: &HashMap<String, String>) -> bool {
    // Basic condition evaluation: "var_name == value" or "var_name != value"
    if cond.contains("==") {
        let parts: Vec<&str> = cond.split("==").collect();
        if parts.len() == 2 {
            let var = parts[0].trim();
            let val = parts[1]
                .trim()
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(parts[1].trim());
            return variables.get(var).map(|v| v == val).unwrap_or(false);
        }
    } else if cond.contains("!=") {
        let parts: Vec<&str> = cond.split("!=").collect();
        if parts.len() == 2 {
            let var = parts[0].trim();
            let val = parts[1]
                .trim()
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(parts[1].trim());
            return variables.get(var).map(|v| v != val).unwrap_or(true);
        }
    }
    false
}

fn parse_duration(s: &str) -> usize {
    if s.ends_with('s') {
        s.trim_end_matches('s').parse().unwrap_or(0)
    } else if s.ends_with('m') {
        s.trim_end_matches('m').parse::<usize>().unwrap_or(0) * 60
    } else {
        s.parse().unwrap_or(0)
    }
}

fn parse_hatch_rate(ramp_up: &str, concurrency: usize) -> String {
    let seconds = parse_duration(ramp_up);
    if seconds == 0 {
        concurrency.to_string()
    } else {
        (concurrency as f32 / seconds as f32).to_string()
    }
}

fn parse_think_time(s: &str) -> (usize, usize) {
    if s.contains('-') {
        let parts: Vec<&str> = s.split('-').collect();
        let min = parse_ms(parts[0]);
        let max = parse_ms(parts[1]);
        (min, max)
    } else {
        let ms = parse_ms(s);
        (ms, ms)
    }
}

fn parse_ms(s: &str) -> usize {
    if s.ends_with("ms") {
        s.trim_end_matches("ms").parse().unwrap_or(0)
    } else if s.ends_with('s') {
        s.trim_end_matches('s').parse::<usize>().unwrap_or(0) * 1000
    } else {
        s.parse().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::{DetailedRequest, ExecutionPlan, ScenarioDefinition};

    #[test]
    fn test_translate_basic() {
        let mut scenarios = HashMap::new();
        scenarios.insert(
            "test".to_string(),
            ScenarioDefinition {
                requests: vec![HTTPRequestDefinition::Simple(
                    "http://localhost".to_string(),
                )],
                weight: 1,
                think_time: None,
                data_sources: None,
            },
        );
        let config = Configuration {
            execution: vec![ExecutionPlan {
                concurrency: 1,
                ramp_up: "0s".to_string(),
                hold_for: "1s".to_string(),
                scenario: "test".to_string(),
                throughput: None,
                steps: None,
            }],
            scenarios,
            reporting: vec![],
        };
        let result = StateTranslator::translate(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_translate_protocols() {
        let mut scenarios = HashMap::new();
        scenarios.insert(
            "proto".to_string(),
            ScenarioDefinition {
                requests: vec![
                    HTTPRequestDefinition::Detailed(Box::new(DetailedRequest {
                        url: "ws://localhost".to_string(),
                        method: None,
                        headers: None,
                        body: None,
                        on_start: false,
                        think_time: None,
                        extract_jsonpath: None,
                        extract_regexp: None,
                        assert: vec![],
                        protocol: Some("websocket".to_string()),
                        message: Some("hello".to_string()),
                        method_name: None,
                        execute_if: None,
                        loop_while: None,
                    })),
                    HTTPRequestDefinition::Detailed(Box::new(DetailedRequest {
                        url: "grpc://localhost".to_string(),
                        method: None,
                        headers: None,
                        body: None,
                        on_start: false,
                        think_time: None,
                        extract_jsonpath: None,
                        extract_regexp: None,
                        assert: vec![],
                        protocol: Some("grpc".to_string()),
                        message: None,
                        method_name: Some("SayHello".to_string()),
                        execute_if: None,
                        loop_while: None,
                    })),
                ],
                weight: 1,
                think_time: None,
                data_sources: None,
            },
        );
        let config = Configuration {
            execution: vec![ExecutionPlan {
                concurrency: 1,
                ramp_up: "0s".to_string(),
                hold_for: "1s".to_string(),
                scenario: "proto".to_string(),
                throughput: None,
                steps: None,
            }],
            scenarios,
            reporting: vec![],
        };
        let result = StateTranslator::translate(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("10s"), 10);
        assert_eq!(parse_duration("1m"), 60);
        assert_eq!(parse_duration("invalid"), 0);
    }

    #[test]
    fn test_evaluate_condition() {
        let mut vars = HashMap::new();
        vars.insert("status".to_string(), "success".to_string());
        assert!(evaluate_condition("status == success", &vars));
        assert!(evaluate_condition("status == \"success\"", &vars));
        assert!(!evaluate_condition("status == fail", &vars));
        assert!(evaluate_condition("status != fail", &vars));
    }
}
