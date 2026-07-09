use crate::engine::BztError;
use crate::engine::control_flow::{AssertionEngine, ControlFlowEngine};
use crate::engine::data_sources::CsvDataSource;
use crate::engine::extraction::{ExtractionEngine, UserSession};
use crate::engine::interpolation::Interpolator;
use crate::engine::macros::MacroEvaluator;
use crate::engine::pacing::PacingEngine;
use crate::engine::reporting::RealTimeMetrics;
use crate::engine::utils::parse_time_to_ms;
use crate::models::config::{Configuration, HTTPRequestDefinition};
use futures_util::{SinkExt, StreamExt};
use goose::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::engine::grpc_dynamic::DynamicGrpcClient;

pub struct StateTranslator;

impl StateTranslator {
    pub async fn translate(
        config: &Configuration,
        host_override: Option<String>,
        real_time_metrics: Option<Arc<Mutex<RealTimeMetrics>>>,
    ) -> Result<GooseAttack, BztError> {
        let mut configuration = goose::config::GooseConfiguration::default();

        // FR-004: Enable HTML report generation via Goose configuration
        for report_def in &config.reporting {
            if report_def.module == "html" {
                configuration.report_file = vec![
                    report_def.filename.clone().unwrap_or_else(|| "report.html".to_string())
                ];
            }
        }

        let mut attack = GooseAttack::initialize_with_config(configuration)
            .map_err(|e| BztError::Goose(Box::new(e)))?;

        // 1. Discovery phase for gRPC reflection
        let mut grpc_clients = HashMap::new();
        for scenario in config.scenarios.values() {
            for req in &scenario.requests {
                if let HTTPRequestDefinition::Detailed(d) = req {
                    let is_grpc = d.protocol.as_deref() == Some("grpc");
                    if is_grpc {
                        let host = host_override.clone().unwrap_or_else(|| d.url.clone());
                        if let std::collections::hash_map::Entry::Vacant(e) = grpc_clients.entry(host.clone()) {
                            tracing::info!("[DISCOVERY] Performing gRPC reflection for {}", host);
                            if let Ok(client) = DynamicGrpcClient::discover(&host).await {
                                e.insert(Arc::new(client));
                            } else {
                                tracing::warn!("[DISCOVERY] gRPC reflection failed for {}. Fallback to static or mock client.", host);
                            }
                        }
                    }
                }
            }
        }
        let grpc_clients = Arc::new(grpc_clients);

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
                .set_default(GooseDefault::RunTime, parse_time_to_ms(&exec.hold_for) as usize / 1000)
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
                    let transaction = Self::build_transaction(
                        req,
                        &scenario_name_clone,
                        &scenario_headers,
                        &ds_map,
                        exec.pacing.clone(),
                        real_time_metrics.clone(),
                        grpc_clients.clone(),
                    );

                    scenario = scenario.register_transaction(transaction);
                }
                attack = attack.register_scenario(scenario);
            }
        }

        Ok(attack)
    }

    fn build_transaction(
        req: &HTTPRequestDefinition,
        scenario_name: &str,
        scenario_headers: &Option<HashMap<String, String>>,
        ds_map: &Arc<HashMap<String, Vec<Arc<CsvDataSource>>>>,
        pacing: Option<crate::models::config::PacingConfig>,
        real_time_metrics: Option<Arc<Mutex<RealTimeMetrics>>>,
        grpc_clients: Arc<HashMap<String, Arc<DynamicGrpcClient>>>,
    ) -> Transaction {
        let req_clone = req.clone();
        let ds_map = Arc::clone(ds_map);
        let scenario_name = scenario_name.to_string();
        let scenario_headers = scenario_headers.clone();

        let mut transaction = Transaction::new(Arc::new(move |user| {
            let req = req_clone.clone();
            let ds_map = Arc::clone(&ds_map);
            let scenario_name = scenario_name.clone();
            let scenario_headers = scenario_headers.clone();
            let pacing = pacing.clone();
            let rt_metrics = real_time_metrics.clone();
            let grpc_cls = grpc_clients.clone();

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
                        if let Some(cond) = &d.execute_if && !ControlFlowEngine::evaluate_condition(cond, &variables) {
                            return Ok(());
                        }

                        let url = Interpolator::interpolate(&d.url, &variables);
                        let final_url = MacroEvaluator::evaluate(&url, record);
                        let full_url = user.build_url(&final_url)?;
                        let request_name = d
                            .label
                            .as_deref()
                            .map(|l| MacroEvaluator::evaluate(l, record))
                            .unwrap_or_else(|| final_url.clone());

                        // REQ-5.4: Control Flow (loop)
                        let mut loop_count = 0;
                        loop {
                            if let Some(cond) = &d.loop_while
                                && (!ControlFlowEngine::evaluate_condition(cond, &variables) || loop_count > 100)
                            {
                                break;
                            }

                            match d.protocol.as_deref() {
                                Some("websocket" | "ws") => {
                                    tracing::debug!("[WS] Connecting to {}", full_url);
                                    let start = std::time::Instant::now();
                                    match connect_async(&full_url).await {
                                        Ok((mut ws_stream, _)) => {
                                            let mut success = true;
                                            if let Some(msg) = &d.message {
                                                let final_msg =
                                                    Interpolator::interpolate(msg, &variables);
                                                let final_msg =
                                                    MacroEvaluator::evaluate(&final_msg, record);
                                                if let Err(e) =
                                                    ws_stream.send(Message::Text(final_msg.into())).await
                                                {
                                                    tracing::error!("[WS] Send failed: {}", e);
                                                    success = false;
                                                }
                                            }
                                            // Wait for at least one response or timeout
                                            let response = ws_stream.next().await;
                                            let duration = start.elapsed().as_millis() as usize;

                                            // Update real-time metrics
                                            if let Some(ref rtm) = rt_metrics {
                                                let mut lock = rtm.lock().unwrap();
                                                let stats = lock.endpoints.entry(request_name.clone()).or_default();
                                                stats.count += 1;
                                                stats.total_time_ms += duration;
                                                stats.times.push(duration);
                                                if !success || response.is_none() {
                                                    stats.failures += 1;
                                                }
                                            }

                                            if success && response.is_some() {
                                                if let Some(Ok(Message::Text(text))) = response {
                                                    // Handle assertions and extraction for WS response
                                                    let status = 200; // WS upgrade success
                                                    for a in &d.assert {
                                                        if let Err(e) =
                                                            AssertionEngine::check_assertion(
                                                                &text, status, a,
                                                            )
                                                        {
                                                            tracing::error!("[WS] Assertion Failed: {}", e);
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
                                                        if let Some(rules) = &d.extract_xpath {
                                                            ExtractionEngine::extract_xpath(
                                                                &text, rules, session,
                                                            );
                                                        }
                                                    }
                                                }
                                            } else {
                                                tracing::error!("[WS] Response Failed");
                                            }
                                            let _ = ws_stream.close(None).await;
                                        }
                                        Err(e) => {
                                            tracing::error!("[WS] Connection failed: {}", e);
                                        }
                                    }
                                }
                                Some("grpc") => {
                                    let method_name = d
                                        .method_name
                                        .as_deref()
                                        .unwrap_or("UnknownService/UnknownMethod");

                                    let body = if let Some(ref bf) = d.body_file {
                                        let body_path = MacroEvaluator::evaluate(bf, record);
                                        tokio::fs::read_to_string(&body_path)
                                            .await
                                            .unwrap_or_default()
                                    } else {
                                        d.body
                                            .as_ref()
                                            .map(|b| Interpolator::interpolate(b, &variables))
                                            .unwrap_or_default()
                                    };
                                    let final_body = MacroEvaluator::evaluate(&body, record);

                                    tracing::debug!("[GRPC] Calling {} at {}", method_name, full_url);
                                    let start = std::time::Instant::now();

                                    // Dynamic gRPC call via reflection
                                    let host_key = if full_url.starts_with("http") {
                                        full_url.clone()
                                    } else {
                                        format!("http://{}", full_url)
                                    };

                                    if let Some(client) = grpc_cls.get(&host_key) {
                                        match client.find_method(method_name) {
                                            Ok(method_desc) => {
                                                // Create a channel for this call
                                                if let Ok(channel) = tonic::transport::Channel::from_shared(host_key).unwrap().connect().await {
                                                    match d.grpc_mode.as_deref() {
                                                        Some("server-streaming" | "server") => {
                                                            match client.call_server_streaming(channel, method_desc, &final_body).await {
                                                                Ok(mut stream) => {
                                                                    while let Some(res_json_result) = stream.next().await {
                                                                        if let Ok(res_json) = res_json_result {
                                                                            let duration = start.elapsed().as_millis() as usize;
                                                                            if let Some(ref rtm) = rt_metrics {
                                                                                let mut lock = rtm.lock().unwrap();
                                                                                let stats = lock.endpoints.entry(request_name.clone()).or_default();
                                                                                stats.count += 1;
                                                                                stats.total_time_ms += duration;
                                                                                stats.times.push(duration);
                                                                            }
                                                                            tracing::info!("[GRPC-STREAM] {} message: {}", request_name, res_json);
                                                                            
                                                                            if let Some(session) = user.get_session_data_mut::<UserSession>() {
                                                                                let status = 200;
                                                                                for a in &d.assert {
                                                                                    if let Err(e) = AssertionEngine::check_assertion(&res_json, status, a) {
                                                                                        tracing::error!("[GRPC-STREAM] Assertion Failed: {}", e);
                                                                                    }
                                                                                }
                                                                                if let Some(rules) = &d.extract_regexp {
                                                                                    ExtractionEngine::extract_regex(&res_json, rules, session);
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    tracing::error!("[GRPC] Stream failed: {}", e);
                                                                }
                                                            }
                                                        }
                                                        Some("client-streaming" | "client") => {
                                                            let mut payloads = Vec::new();
                                                            if d.messages.is_empty() {
                                                                payloads.push(final_body);
                                                            } else {
                                                                for m in &d.messages {
                                                                    let final_m = Interpolator::interpolate(m, &variables);
                                                                    payloads.push(MacroEvaluator::evaluate(&final_m, record));
                                                                }
                                                            }
                                                            match client.call_client_streaming(channel, method_desc, payloads).await {
                                                                Ok(res_json) => {
                                                                    let duration = start.elapsed().as_millis() as usize;
                                                                    if let Some(ref rtm) = rt_metrics {
                                                                        let mut lock = rtm.lock().unwrap();
                                                                        let stats = lock.endpoints.entry(request_name.clone()).or_default();
                                                                        stats.count += 1;
                                                                        stats.total_time_ms += duration;
                                                                        stats.times.push(duration);
                                                                    }
                                                                    tracing::info!("[GRPC-CLIENT] {} success: {}", request_name, res_json);
                                                                    
                                                                    if let Some(session) = user.get_session_data_mut::<UserSession>() {
                                                                        let status = 200;
                                                                        for a in &d.assert {
                                                                            if let Err(e) = AssertionEngine::check_assertion(&res_json, status, a) {
                                                                                tracing::error!("[GRPC-CLIENT] Assertion Failed: {}", e);
                                                                            }
                                                                        }
                                                                        if let Some(rules) = &d.extract_regexp {
                                                                            ExtractionEngine::extract_regex(&res_json, rules, session);
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    tracing::error!("[GRPC] Client stream failed: {}", e);
                                                                }
                                                            }
                                                        }
                                                        Some("bidi-streaming" | "bidi") => {
                                                            let mut payloads = Vec::new();
                                                            if d.messages.is_empty() {
                                                                payloads.push(final_body);
                                                            } else {
                                                                for m in &d.messages {
                                                                    let final_m = Interpolator::interpolate(m, &variables);
                                                                    payloads.push(MacroEvaluator::evaluate(&final_m, record));
                                                                }
                                                            }
                                                            match client.call_bidi_streaming(channel, method_desc, payloads).await {
                                                                Ok(mut stream) => {
                                                                    while let Some(res_json_result) = stream.next().await {
                                                                        if let Ok(res_json) = res_json_result {
                                                                            let duration = start.elapsed().as_millis() as usize;
                                                                            if let Some(ref rtm) = rt_metrics {
                                                                                let mut lock = rtm.lock().unwrap();
                                                                                let stats = lock.endpoints.entry(request_name.clone()).or_default();
                                                                                stats.count += 1;
                                                                                stats.total_time_ms += duration;
                                                                                stats.times.push(duration);
                                                                            }
                                                                            tracing::info!("[GRPC-BIDI] {} message: {}", request_name, res_json);
                                                                            
                                                                            if let Some(session) = user.get_session_data_mut::<UserSession>() {
                                                                                let status = 200;
                                                                                for a in &d.assert {
                                                                                    if let Err(e) = AssertionEngine::check_assertion(&res_json, status, a) {
                                                                                        tracing::error!("[GRPC-BIDI] Assertion Failed: {}", e);
                                                                                    }
                                                                                }
                                                                                if let Some(rules) = &d.extract_regexp {
                                                                                    ExtractionEngine::extract_regex(&res_json, rules, session);
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    tracing::error!("[GRPC] Bidi stream failed: {}", e);
                                                                }
                                                            }
                                                        }
                                                        _ => {
                                                            // Default: unary
                                                            match client.call_unary(channel, method_desc, &final_body).await {
                                                                Ok(res_json) => {
                                                                    let duration = start.elapsed().as_millis() as usize;
                                                                    if let Some(ref rtm) = rt_metrics {
                                                                        let mut lock = rtm.lock().unwrap();
                                                                        let stats = lock.endpoints.entry(request_name.clone()).or_default();
                                                                        stats.count += 1;
                                                                        stats.total_time_ms += duration;
                                                                        stats.times.push(duration);
                                                                    }
                                                                    tracing::info!("[GRPC] {} success: {}", request_name, res_json);
                                                                    
                                                                    if let Some(session) = user.get_session_data_mut::<UserSession>() {
                                                                        let status = 200;
                                                                        for a in &d.assert {
                                                                            if let Err(e) = AssertionEngine::check_assertion(&res_json, status, a) {
                                                                                tracing::error!("[GRPC] Assertion Failed: {}", e);
                                                                            }
                                                                        }
                                                                        if let Some(rules) = &d.extract_regexp {
                                                                            ExtractionEngine::extract_regex(&res_json, rules, session);
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    let duration = start.elapsed().as_millis() as usize;
                                                                    if let Some(ref rtm) = rt_metrics {
                                                                        let mut lock = rtm.lock().unwrap();
                                                                        let stats = lock.endpoints.entry(request_name.clone()).or_default();
                                                                        stats.count += 1;
                                                                        stats.failures += 1;
                                                                        stats.total_time_ms += duration;
                                                                        stats.times.push(duration);
                                                                    }
                                                                    tracing::error!("[GRPC] Call failed: {}", e);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!("[GRPC] Method discovery failed: {}", e);
                                            }
                                        }
                                    } else {
                                        tracing::warn!("[GRPC] No reflection client found for {}. Fallback to static call.", host_key);
                                        // Fallback to the previous "static" implementation for the internal mock
                                        use crate::engine::proto::bzt_mock;
                                        if let Ok(mut client) = bzt_mock::mock_service_client::MockServiceClient::connect(full_url.clone()).await {
                                            let req = tonic::Request::new(bzt_mock::MockRequest {
                                                method: method_name.to_string(),
                                                payload: final_body,
                                            });

                                            match client.call(req).await {
                                                Ok(response) => {
                                                    let duration = start.elapsed().as_millis() as usize;
                                                    if let Some(ref rtm) = rt_metrics {
                                                        let mut lock = rtm.lock().unwrap();
                                                        let stats = lock.endpoints.entry(request_name.clone()).or_default();
                                                        stats.count += 1;
                                                        stats.total_time_ms += duration;
                                                        stats.times.push(duration);
                                                    }
                                                    let res = response.into_inner();
                                                    tracing::info!("[GRPC-STATIC] {} success: {}", request_name, res.message);
                                                }
                                                Err(e) => {
                                                    let duration = start.elapsed().as_millis() as usize;
                                                    if let Some(ref rtm) = rt_metrics {
                                                        let mut lock = rtm.lock().unwrap();
                                                        let stats = lock.endpoints.entry(request_name.clone()).or_default();
                                                        stats.count += 1;
                                                        stats.failures += 1;
                                                        stats.total_time_ms += duration;
                                                        stats.times.push(duration);
                                                    }
                                                    tracing::error!("[GRPC-STATIC] {} failed: {}", request_name, e);
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    let method =
                                        d.method.clone().unwrap_or_else(|| "GET".to_string());

                                    // Body: prefer body_file if set, fall back to inline body
                                    let body = if let Some(ref bf) = d.body_file {
                                        let body_path = MacroEvaluator::evaluate(bf, record);
                                        match tokio::fs::read_to_string(&body_path).await {
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
                                            .map(|b| Interpolator::interpolate(b, &variables))
                                            .unwrap_or_default()
                                    };
                                    let final_body = MacroEvaluator::evaluate(&body, record);

                                    let method_enum = match method.as_str() {
                                        "POST" => GooseMethod::Post,
                                        "PUT" => GooseMethod::Put,
                                        "DELETE" => GooseMethod::Delete,
                                        "PATCH" => GooseMethod::Patch,
                                        "HEAD" => GooseMethod::Head,
                                        _ => GooseMethod::Get,
                                    };

                                    tracing::debug!(
                                        "[CONFIG] request_name={}, timeout={:?}, body_file={}, headers={}",
                                        request_name,
                                        d.timeout,
                                        d.body_file.as_deref().unwrap_or("none"),
                                        d.headers.as_ref().map_or(0, |h| h.len())
                                    );

                                    let mut reqwest_builder = match method_enum {
                                        GooseMethod::Post => user.client.post(&full_url),
                                        GooseMethod::Put => user.client.put(&full_url),
                                        GooseMethod::Delete => user.client.delete(&full_url),
                                        GooseMethod::Patch => user.client.patch(&full_url),
                                        GooseMethod::Head => user.client.head(&full_url),
                                        _ => user.client.get(&full_url),
                                    };

                                    // Apply scenario-level + per-request headers
                                    if let Some(ref sh) = scenario_headers {
                                        for (k, v) in sh {
                                            reqwest_builder =
                                                reqwest_builder.header(k.as_str(), v.as_str());
                                        }
                                    }
                                    if let Some(ref rh) = d.headers {
                                        for (k, v) in rh {
                                            reqwest_builder =
                                                reqwest_builder.header(k.as_str(), v.as_str());
                                        }
                                    }

                                    // Apply timeout
                                    if let Some(ref timeout_str) = d.timeout {
                                        let ms = parse_time_to_ms(timeout_str);
                                        if ms > 0 {
                                            reqwest_builder = reqwest_builder
                                                .timeout(Duration::from_millis(ms));
                                        }
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
                                            .set_request_builder(reqwest_builder.body(final_body))
                                            .build()
                                    };
                                    let start = std::time::Instant::now();
                                    let goose_response = user.request(goose_request).await?;
                                    let duration = start.elapsed().as_millis() as usize;

                                    // Update real-time metrics
                                    if let Some(ref rtm) = rt_metrics {
                                        let mut lock = rtm.lock().unwrap();
                                        let stats = lock.endpoints.entry(request_name.clone()).or_default();
                                        stats.count += 1;
                                        stats.total_time_ms += duration;
                                        stats.times.push(duration);
                                        if goose_response.response.is_err() {
                                            stats.failures += 1;
                                        }
                                    }

                                    if let Ok(response) = &goose_response.response {

                                        let status = response.status().as_u16();
                                        let text = goose_response
                                            .response
                                            .unwrap()
                                            .text()
                                            .await
                                            .unwrap_or_default();

                                        for a in &d.assert {
                                            if let Err(e) = AssertionEngine::check_assertion(
                                                &text, status, a,
                                            ) {
                                                let mut req = goose_response.request.clone();
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
                                            if let Some(rules) = &d.extract_xpath {
                                                ExtractionEngine::extract_xpath(
                                                    &text, rules, session,
                                                );
                                            }
                                        }
                                    }
                                }
                            }

                            // Unified variable synchronization for loop condition
                                if d.loop_while.is_some() {
                                    let session_opt = user.get_session_data::<UserSession>();
                                    if let Some(session) = session_opt {
                                        variables = session.variables.clone();
                                        tracing::trace!("[SESSION] {} variables updated for loop", variables.len());
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

        transaction
    }
}

fn parse_hatch_rate(ramp_up: &str, concurrency: usize) -> String {
    let ms = parse_time_to_ms(ramp_up);
    let seconds = ms as f32 / 1000.0;
    if seconds <= 0.0 {
        concurrency.to_string()
    } else {
        (concurrency as f32 / seconds).to_string()
    }
}

fn parse_think_time(s: &str) -> (usize, usize) {
    if s.contains('-') {
        let parts: Vec<&str> = s.split('-').collect();
        let min = parse_time_to_ms(parts[0]) as usize;
        let max = parse_time_to_ms(parts[1]) as usize;
        (min, max)
    } else {
        let ms = parse_time_to_ms(s) as usize;
        (ms, ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::{DetailedRequest, ExecutionPlan, ScenarioDefinition};

    #[tokio::test]
    async fn
 test_translate_basic() {
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
            services: vec![],
            api: None,
        };
        let result = StateTranslator::translate(&config, None, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn
 test_translate_protocols() {
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
                        ..Default::default()
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
                        ..Default::default()
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
            services: vec![],
            api: None,
        };
        let result = StateTranslator::translate(&config, None, None).await;
        assert!(result.is_ok());
    }
}
