use crate::engine::BztError;
use crate::engine::control_flow::AssertionEngine;
use crate::engine::data_sources::CsvDataSource;
use crate::engine::extraction::{ExtractionEngine, UserSession};
use crate::engine::interpolation::Interpolator;
use crate::engine::macros::MacroEvaluator;
use crate::engine::pacing::PacingEngine;
use crate::models::config::{Configuration, HTTPRequestDefinition};
use goose::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub struct StateTranslator;

impl StateTranslator {
    pub fn translate(
        config: &Configuration,
        host_override: Option<String>,
    ) -> Result<GooseAttack, BztError> {
        let configuration = goose::config::GooseConfiguration::default();
        let mut attack = GooseAttack::initialize_with_config(configuration)
            .map_err(|e| BztError::Goose(Box::new(e)))?;

        // Load data sources
        let mut data_sources = HashMap::new();
        for (name, scenario_def) in &config.scenarios {
            if let Some(sources) = &scenario_def.data_sources {
                let mut loaded = Vec::new();
                for source in sources {
                    let ds = CsvDataSource::new(source.path())?;
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
                .map_err(|e| BztError::Goose(Box::new(e)))?;
            attack = *attack
                .set_default(GooseDefault::RunTime, parse_duration(&exec.hold_for))
                .map_err(|e| BztError::Goose(Box::new(e)))?;
            attack = *attack
                .set_default(GooseDefault::HatchRate, hatch_rate.as_str())
                .map_err(|e| BztError::Goose(Box::new(e)))?;

            let host = host_override
                .clone()
                .unwrap_or_else(|| "http://localhost".to_string());
            attack = *attack
                .set_default(GooseDefault::Host, host.as_str())
                .map_err(|e| BztError::Goose(Box::new(e)))?;

            if let Some(throughput) = exec.throughput {
                attack = *attack
                    .set_default(GooseDefault::ThrottleRequests, throughput)
                    .map_err(|e| BztError::Goose(Box::new(e)))?;
            }

            if let Some(scenario_def) = config.scenarios.get(&exec.scenario) {
                let scenario_name = exec.scenario.clone();
                let mut scenario = Scenario::new(&scenario_name);

                if scenario_def.weight > 1 {
                    scenario = scenario
                        .set_weight(scenario_def.weight)
                        .map_err(|e| BztError::Goose(Box::new(e)))?;
                }

                // Handle scenario-level think-time
                if let Some(tt) = &scenario_def.think_time {
                    let (min, max) = parse_think_time(tt);
                    scenario = scenario
                        .set_wait_time(
                            Duration::from_millis(min as u64),
                            Duration::from_millis(max as u64),
                        )
                        .map_err(|e| BztError::Goose(Box::new(e)))?;
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

                let scenario_headers = scenario_def.headers.clone();
                let ds_map = Arc::clone(&data_sources);
                let scenario_name_clone = scenario_name.clone();

                for req in &scenario_def.requests {
                    let req_clone = req.clone();
                    let ds_map = Arc::clone(&ds_map);
                    let scenario_name = scenario_name_clone.clone();
                    let scenario_headers = scenario_headers.clone();
                    let pacing = exec.pacing.clone();

                    let mut transaction = Transaction::new(Arc::new(move |user| {
                        let req = req_clone.clone();
                        let ds_map = Arc::clone(&ds_map);
                        let scenario_name = scenario_name.clone();
                        let scenario_headers = scenario_headers.clone();
                        let pacing = pacing.clone();

                        Box::pin(async move {
                            // Apply pacing delay before each request if configured
                            if let Some(ref pc) = pacing {
                                let delay = PacingEngine::calculate_delay(pc);
                                tokio::time::sleep(delay).await;
                            }

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
                                            Some("websocket" | "ws") => {
                                                tracing::warn!(
                                                    "[NOT IMPLEMENTED] WebSocket protocol is not yet available — skipping request to {final_url}"
                                                );
                                                return Ok(());
                                            }
                                            Some("grpc") => {
                                                tracing::warn!(
                                                    "[NOT IMPLEMENTED] gRPC protocol is not yet available — skipping request to {final_url}"
                                                );
                                                return Ok(());
                                            }
                                            _ => {
                                                let method = d
                                                    .method
                                                    .clone()
                                                    .unwrap_or_else(|| "GET".to_string());

                                                // Body: prefer body_file if set, fall back to inline body
                                                let body = if let Some(ref bf) = d.body_file {
                                                    let body_path =
                                                        MacroEvaluator::evaluate(bf, record);
                                                    match tokio::fs::read_to_string(&body_path)
                                                        .await
                                                    {
                                                        Ok(content) => content,
                                                        Err(e) => {
                                                            tracing::warn!(
                                                                "[BODY_FILE] Failed to read body file '{}': {}",
                                                                body_path,
                                                                e
                                                            );
                                                            String::new()
                                                        }
                                                    }
                                                } else {
                                                    d.body
                                                        .as_ref()
                                                        .map(|b| {
                                                            Interpolator::interpolate(b, &variables)
                                                        })
                                                        .unwrap_or_default()
                                                };
                                                let final_body =
                                                    MacroEvaluator::evaluate(&body, record);

                                                let url = user.build_url(&final_url)?;
                                                let method_enum = match method.as_str() {
                                                    "POST" => GooseMethod::Post,
                                                    "PUT" => GooseMethod::Put,
                                                    "DELETE" => GooseMethod::Delete,
                                                    "PATCH" => GooseMethod::Patch,
                                                    "HEAD" => GooseMethod::Head,
                                                    _ => GooseMethod::Get,
                                                };

                                                // Build the request name from label or URL
                                                let request_name = d
                                                    .label
                                                    .as_deref()
                                                    .map(|l| MacroEvaluator::evaluate(l, record))
                                                    .unwrap_or_else(|| final_url.clone());
                                                tracing::debug!(
                                                    "[CONFIG] request_name={}, timeout={:?}, body_file={}, headers={}",
                                                    request_name,
                                                    d.timeout,
                                                    d.body_file.as_deref().unwrap_or("none"),
                                                    d.headers.as_ref().map_or(0, |h| h.len())
                                                );

                                                let mut reqwest_builder = match method_enum {
                                                    GooseMethod::Post => user.client.post(&url),
                                                    GooseMethod::Put => user.client.put(&url),
                                                    GooseMethod::Delete => user.client.delete(&url),
                                                    GooseMethod::Patch => user.client.patch(&url),
                                                    GooseMethod::Head => user.client.head(&url),
                                                    _ => user.client.get(&url),
                                                };

                                                // Apply scenario-level + per-request headers
                                                if let Some(ref sh) = scenario_headers {
                                                    for (k, v) in sh {
                                                        reqwest_builder = reqwest_builder
                                                            .header(k.as_str(), v.as_str());
                                                    }
                                                }
                                                if let Some(ref rh) = d.headers {
                                                    for (k, v) in rh {
                                                        reqwest_builder = reqwest_builder
                                                            .header(k.as_str(), v.as_str());
                                                    }
                                                }

                                                // Apply timeout
                                                if let Some(ref timeout_str) = d.timeout
                                                    && let Some(ms) = parse_timeout_ms(timeout_str)
                                                {
                                                    reqwest_builder = reqwest_builder
                                                        .timeout(Duration::from_millis(ms));
                                                }

                                                let goose_request = if final_body.is_empty() {
                                                    GooseRequest::builder()
                                                        .method(method_enum)
                                                        .path(request_name.as_str())
                                                        .build()
                                                } else {
                                                    GooseRequest::builder()
                                                        .method(method_enum)
                                                        .path(request_name.as_str())
                                                        .set_request_builder(
                                                            reqwest_builder.body(final_body),
                                                        )
                                                        .build()
                                                };
                                                let goose_response =
                                                    user.request(goose_request).await?;

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
                                                            let err_msg = e.to_string();
                                                            let _ = user.set_failure(
                                                                "Assertion Failed",
                                                                &mut req,
                                                                None,
                                                                Some(&err_msg),
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
            return variables.get(var).is_some_and(|v| v == val);
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
            return variables.get(var).is_none_or(|v| v != val);
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

/// Parses a timeout string (e.g. "30s", "5000ms", "5m") into milliseconds.
/// Returns `None` if the string cannot be parsed.
#[must_use]
pub fn parse_timeout_ms(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.ends_with("ms") {
        s.trim_end_matches("ms").parse::<u64>().ok()
    } else if s.ends_with('s') {
        s.trim_end_matches('s')
            .parse::<u64>()
            .ok()
            .map(|v| v * 1000)
    } else if s.ends_with('m') {
        s.trim_end_matches('m')
            .parse::<u64>()
            .ok()
            .map(|v| v * 60_000)
    } else {
        s.parse::<u64>().ok()
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
                headers: None,
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
                pacing: None,
            }],
            scenarios,
            reporting: vec![],
        };
        let result = StateTranslator::translate(&config, None);
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
                        label: None,
                        body_file: None,
                        timeout: None,
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
                        label: None,
                        body_file: None,
                        timeout: None,
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
                headers: None,
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
                pacing: None,
            }],
            scenarios,
            reporting: vec![],
        };
        let result = StateTranslator::translate(&config, None);
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

    #[test]
    fn test_parse_timeout_ms_seconds() {
        assert_eq!(parse_timeout_ms("30s"), Some(30_000));
        assert_eq!(parse_timeout_ms("5s"), Some(5_000));
    }

    #[test]
    fn test_parse_timeout_ms_milliseconds() {
        assert_eq!(parse_timeout_ms("500ms"), Some(500));
        assert_eq!(parse_timeout_ms("10000ms"), Some(10_000));
    }

    #[test]
    fn test_parse_timeout_ms_minutes() {
        assert_eq!(parse_timeout_ms("1m"), Some(60_000));
        assert_eq!(parse_timeout_ms("5m"), Some(300_000));
    }

    #[test]
    fn test_parse_timeout_ms_invalid() {
        assert_eq!(parse_timeout_ms(""), None);
        assert_eq!(parse_timeout_ms("abc"), None);
    }
}
