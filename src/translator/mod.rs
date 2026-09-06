mod body;
mod context;
mod metrics_recorder;

use body::resolve_body;
use context::{GrpcClients, TransactionContext};
use metrics_recorder::record_request;

use crate::engine::PummelError;
use crate::engine::control_flow::{AssertionEngine, ControlFlowEngine};
use crate::engine::data_sources::CsvDataSource;
use crate::engine::extraction::{ExtractionEngine, UserSession};
use crate::engine::interpolation::Interpolator;
use crate::engine::macros::MacroEvaluator;
use crate::engine::pacing::PacingEngine;
use crate::engine::reporting::RealTimeMetrics;
use crate::engine::utils::parse_time_to_ms;
use crate::models::config::{Configuration, HttpMethod, HTTPRequestDefinition, Protocol, ScenarioDefinition};
#[cfg(feature = "grpc")]
use crate::models::config::GrpcMode;
use futures_util::{SinkExt, StreamExt};
use goose::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

/// Logs a macro evaluation error and returns `Ok(())` to skip the transaction.
/// Sensitive env vars are rejected (never injected) but the load test continues.
fn log_macro_error(e: &PummelError) {
    tracing::error!("[SECURITY] Macro evaluation blocked: {e}");
}

/// Evaluates macros, rejecting sensitive env vars. On rejection, logs the security
/// error and performs an early `return Ok(())` from the enclosing transaction closure.
macro_rules! eval_macros {
    ($expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                log_macro_error(&e);
                return Ok(());
            }
        }
    };
}

pub struct StateTranslator;

impl StateTranslator {
    #[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
    /// Translates a BZT configuration into a Goose attack.
    ///
    /// # Errors
    ///
    /// Returns `PummelError` if translation from configuration to Goose fails.
    #[allow(clippy::unused_async)]
    pub async fn translate(
        config: &Configuration,
        host_override: Option<String>,
        real_time_metrics: Option<Arc<RwLock<RealTimeMetrics>>>,
    ) -> Result<GooseAttack, PummelError> {
        let mut configuration = goose::config::GooseConfiguration::default();

        // FR-004: Enable HTML report generation via Goose configuration
        for report_def in &config.reporting {
            if report_def.module == "html" {
                configuration.report_file = vec![
                    report_def
                        .filename
                        .clone()
                        .unwrap_or_else(|| "report.html".to_string()),
                ];
            }
        }

        let mut attack = GooseAttack::initialize_with_config(configuration)
            .map_err(|e| PummelError::Goose(Box::new(e)))?;

        // 1. Discovery phase for gRPC reflection (only when grpc feature is enabled)
        #[cfg(feature = "grpc")]
        let grpc_clients: GrpcClients = {
            let mut clients = HashMap::new();
            for scenario in config.scenarios.values() {
                for req in &scenario.requests {
                    if let HTTPRequestDefinition::Detailed(d) = req {
                        let is_grpc = d.protocol == Some(Protocol::Grpc);
                        if is_grpc {
                            let host = host_override.clone().unwrap_or_else(|| d.url.clone());
                            if let std::collections::hash_map::Entry::Vacant(e) =
                                clients.entry(host.clone())
                            {
                                tracing::info!("[DISCOVERY] Performing gRPC reflection for {}", host);
                                if let Ok(client) = DynamicGrpcClient::discover(&host).await {
                                    e.insert(Arc::new(client));
                                } else {
                                    tracing::warn!(
                                        "[DISCOVERY] gRPC reflection failed for {}. Fallback to static or mock client.",
                                        host
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Arc::new(clients)
        };
        #[cfg(not(feature = "grpc"))]
        let grpc_clients: GrpcClients = ();

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
                .map_err(|e| PummelError::Goose(Box::new(e)))?;
            attack = *attack
                .set_default(
                    GooseDefault::RunTime,
                    parse_time_to_ms(&exec.hold_for) as usize / 1000,
                )
                .map_err(|e| PummelError::Goose(Box::new(e)))?;
            attack = *attack
                .set_default(GooseDefault::HatchRate, hatch_rate.as_str())
                .map_err(|e| PummelError::Goose(Box::new(e)))?;

            let mut host = host_override.clone();
            if host.is_none() {
                if let Some(scenario_def) = config.scenarios.get(&exec.scenario) {
                    if let Some(extracted) = extract_default_host(scenario_def) {
                        host = Some(extracted);
                    }
                }
            }
            let host = host.unwrap_or_else(|| "http://localhost".to_string());
            attack = *attack
                .set_default(GooseDefault::Host, host.as_str())
                .map_err(|e| PummelError::Goose(Box::new(e)))?;

            if let Some(throughput) = exec.throughput {
                attack = *attack
                    .set_default(GooseDefault::ThrottleRequests, throughput)
                    .map_err(|e| PummelError::Goose(Box::new(e)))?;
            }

            if let Some(scenario_def) = config.scenarios.get(&exec.scenario) {
                let scenario_name = exec.scenario.clone();
                let mut scenario = Scenario::new(&scenario_name);

                if scenario_def.weight > 1 {
                    scenario = scenario
                        .set_weight(scenario_def.weight)
                        .map_err(|e| PummelError::Goose(Box::new(e)))?;
                }

                // Handle scenario-level think-time
                if let Some(tt) = &scenario_def.think_time {
                    let (min, max) = parse_think_time(tt);
                    scenario = scenario
                        .set_wait_time(
                            Duration::from_millis(min as u64),
                            Duration::from_millis(max as u64),
                        )
                        .map_err(|e| PummelError::Goose(Box::new(e)))?;
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

                let ctx = TransactionContext {
                    scenario_name: Arc::new(scenario_name.clone()),
                    scenario_headers: Arc::new(scenario_def.headers.clone()),
                    data_sources: Arc::clone(&data_sources),
                    real_time_metrics: real_time_metrics.clone(),
                    #[allow(clippy::clone_on_copy)]
                    grpc_clients: grpc_clients.clone(),
                };

                for req in &scenario_def.requests {
                    let transaction =
                        Self::build_transaction(req, &ctx, exec.pacing.clone());

                    scenario = scenario.register_transaction(transaction);
                }
                attack = attack.register_scenario(scenario);
            }
        }

        Ok(attack)
    }

    #[allow(clippy::too_many_lines)]
    #[allow(clippy::items_after_statements)]
    fn build_transaction(
        req: &HTTPRequestDefinition,
        ctx: &TransactionContext,
        pacing: Option<crate::models::config::PacingConfig>,
    ) -> Transaction {
        let req_clone = req.clone();
        let ctx = ctx.clone();
        // Wrap in Arc to avoid cloning PacingConfig per-request.
        let pacing = pacing.map(Arc::new);

        let mut transaction = Transaction::new(Arc::new(move |user| {
            let req = req_clone.clone();
            let ctx = ctx.clone();
            let pacing = pacing.clone();
            // Destructure context into the variable names the rest of the
            // closure body already uses so downstream code needs no changes.
            let scenario_name = ctx.scenario_name;
            let scenario_headers = ctx.scenario_headers;
            let ds_map = ctx.data_sources;
            let rt_metrics = ctx.real_time_metrics;

            Box::pin(async move {
                // Apply pacing delay before each request if configured
                if let Some(ref pc) = pacing {
                    let delay = PacingEngine::calculate_delay(pc);
                    tokio::time::sleep(delay).await;
                }

                // Get record from data source if available
                let mut record = None;
                if let Some(ds_list) = ds_map.get(scenario_name.as_ref())
                    && !ds_list.is_empty()
                {
                    record = ds_list[0].get_record(user.weighted_users_index);
                }

                match req {
                    HTTPRequestDefinition::Simple(url) => {
                        // Borrow session variables directly — no clone needed because
                        // Simple requests never call get_session_data_mut.
                        let empty_vars = HashMap::new();
                        let variables = user
                            .get_session_data::<UserSession>()
                            .map_or(&empty_vars, |s| &s.variables);
                        let url = Interpolator::interpolate(&url, variables);
                        let final_url = eval_macros!(MacroEvaluator::evaluate(&url, record));
                        let _ = user.get(&final_url).await?;
                    }
                    HTTPRequestDefinition::Detailed(d) => {
                        // Clone is required here because extraction code later calls
                        // get_session_data_mut, which invalidates any live references
                        // to session variables.
                        let mut variables = user
                            .get_session_data::<UserSession>()
                            .map(|s| s.variables.clone())
                            .unwrap_or_default();

                        // REQ-5.4: Control Flow (if)
                        if let Some(cond) = &d.execute_if
                            && !ControlFlowEngine::evaluate_condition(cond, &variables)
                        {
                            return Ok(());
                        }

                        let url = Interpolator::interpolate(&d.url, &variables);
                        let final_url = eval_macros!(MacroEvaluator::evaluate(&url, record));
                        let full_url = user.build_url(&final_url)?;
                        let request_name = match &d.label {
                            Some(l) => eval_macros!(MacroEvaluator::evaluate(l, record)),
                            None => final_url.clone(),
                        };

                        // REQ-5.4: Control Flow (loop)
                        let mut loop_count = 0;
                        loop {
                            if let Some(cond) = &d.loop_while
                                && (!ControlFlowEngine::evaluate_condition(cond, &variables)
                                    || loop_count > 100)
                            {
                                break;
                            }

                            match d.protocol {
                                Some(Protocol::Websocket | Protocol::Ws) => {
                                    tracing::debug!("[WS] Connecting to {}", full_url);
                                    let start = std::time::Instant::now();
                                    match connect_async(&full_url).await {
                                        Ok((mut ws_stream, _)) => {
                                            let mut success = true;
                                            if let Some(msg) = &d.message {
                                                let final_msg =
                                                    Interpolator::interpolate(msg, &variables);
                                                let final_msg = eval_macros!(
                                                    MacroEvaluator::evaluate(&final_msg, record)
                                                );
                                                if let Err(e) = ws_stream
                                                    .send(Message::Text(final_msg.into()))
                                                    .await
                                                {
                                                    tracing::error!("[WS] Send failed: {}", e);
                                                    success = false;
                                                }
                                            }
                                            // Wait for at least one response or timeout
                                            let response = ws_stream.next().await;
                                            let duration = start.elapsed().as_millis() as usize;

                                            // Update real-time metrics
                        record_request(
                            rt_metrics.as_ref(),
                                                &request_name,
                                                duration,
                                                success && response.is_some(),
                                            );

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
                                                            tracing::error!(
                                                                "[WS] Assertion Failed: {}",
                                                                e
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
                                                        #[cfg(feature = "xpath")]
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
                                Some(Protocol::Grpc) => {
                                    #[cfg(feature = "grpc")]
                                    {
                                    let method_name = d
                                        .method_name
                                        .as_deref()
                                        .unwrap_or("UnknownService/UnknownMethod");

                                    let final_body = eval_macros!(
                                        resolve_body(
                                            d.body.as_deref(),
                                            d.body_file.as_deref(),
                                            &variables,
                                            record,
                                        )
                                        .await
                                    );

                                    tracing::debug!(
                                        "[GRPC] Calling {} at {}",
                                        method_name,
                                        full_url
                                    );
                                    let start = std::time::Instant::now();

                                    // Dynamic gRPC call via reflection
                                    let host_key = if full_url.starts_with("http") {
                                        full_url.clone()
                                    } else {
                                        format!("http://{full_url}")
                                    };

                                    if let Some(client) = _grpc_cls.get(&host_key) {
                                        match client.find_method(method_name) {
                                            Ok(method_desc) => {
                                                // Create a channel for this call
                                                if let Ok(channel) =
                                                    tonic::transport::Channel::from_shared(host_key)
                                                        .unwrap()
                                                        .connect()
                                                        .await
                                                {
                                                    match d.grpc_mode {
                                                        Some(GrpcMode::ServerStreaming) => {
                                                            match client
                                                                .call_server_streaming(
                                                                    channel,
                                                                    method_desc,
                                                                    &final_body,
                                                                )
                                                                .await
                                                            {
                                                                Ok(mut stream) => {
                                                                    while let Some(
                                                                        res_json_result,
                                                                    ) = stream.next().await
                                                                    {
                                                                        if let Ok(res_json) =
                                                                            res_json_result
                                                                        {
                                                                             let duration = start
                                                                                .elapsed()
                                                                                .as_millis()
                                                                                as usize;
                                                                            record_request(rt_metrics.as_ref(), &request_name, duration, true);
                                                                            tracing::info!(
                                                                                "[GRPC-STREAM] {} message: {}",
                                                                                request_name,
                                                                                res_json
                                                                            );

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
                                                                    tracing::error!(
                                                                        "[GRPC] Stream failed: {}",
                                                                        e
                                                                    );
                                                                }
                                                            }
                                                        }
                                                        Some(GrpcMode::ClientStreaming) => {
                                                            let mut payloads = Vec::new();
                                                            if d.messages.is_empty() {
                                                                payloads.push(final_body);
                                                            } else {
                                                                for m in &d.messages {
                                                                    let final_m =
                                                                        Interpolator::interpolate(
                                                                            m, &variables,
                                                                        );
                                                                    payloads.push(eval_macros!(
                                                                        MacroEvaluator::evaluate(
                                                                            &final_m, record
                                                                        )
                                                                    ));
                                                                }
                                                            }
                                                            match client
                                                                .call_client_streaming(
                                                                    channel,
                                                                    method_desc,
                                                                    payloads,
                                                                )
                                                                .await
                                                            {
                                                                 Ok(res_json) => {
                                                                    let duration =
                                                                        start.elapsed().as_millis()
                                                                            as usize;
                                                                    record_request(rt_metrics.as_ref(), &request_name, duration, true);
                                                                    tracing::info!(
                                                                        "[GRPC-CLIENT] {} success: {}",
                                                                        request_name,
                                                                        res_json
                                                                    );

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
                                                                    tracing::error!(
                                                                        "[GRPC] Client stream failed: {}",
                                                                        e
                                                                    );
                                                                }
                                                            }
                                                        }
                                                        Some(GrpcMode::BidiStreaming) => {
                                                            let mut payloads = Vec::new();
                                                            if d.messages.is_empty() {
                                                                payloads.push(final_body);
                                                            } else {
                                                                for m in &d.messages {
                                                                    let final_m =
                                                                        Interpolator::interpolate(
                                                                            m, &variables,
                                                                        );
                                                                    payloads.push(eval_macros!(
                                                                        MacroEvaluator::evaluate(
                                                                            &final_m, record
                                                                        )
                                                                    ));
                                                                }
                                                            }
                                                            match client
                                                                .call_bidi_streaming(
                                                                    channel,
                                                                    method_desc,
                                                                    payloads,
                                                                )
                                                                .await
                                                            {
                                                                Ok(mut stream) => {
                                                                    while let Some(
                                                                        res_json_result,
                                                                    ) = stream.next().await
                                                                    {
                                                                        if let Ok(res_json) =
                                                                            res_json_result
                                                                        {
                                                                             let duration = start
                                                                                .elapsed()
                                                                                .as_millis()
                                                                                as usize;
                                                                            record_request(rt_metrics.as_ref(), &request_name, duration, true);
                                                                            tracing::info!(
                                                                                "[GRPC-BIDI] {} message: {}",
                                                                                request_name,
                                                                                res_json
                                                                            );

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
                                                                    tracing::error!(
                                                                        "[GRPC] Bidi stream failed: {}",
                                                                        e
                                                                    );
                                                                }
                                                            }
                                                        }
                                                        _ => {
                                                            // Default: unary
                                                            match client
                                                                .call_unary(
                                                                    channel,
                                                                    method_desc,
                                                                    &final_body,
                                                                )
                                                                .await
                                                            {
                                                                 Ok(res_json) => {
                                                                    let duration =
                                                                        start.elapsed().as_millis()
                                                                            as usize;
                                                                    record_request(rt_metrics.as_ref(), &request_name, duration, true);
                                                                    tracing::info!(
                                                                        "[GRPC] {} success: {}",
                                                                        request_name,
                                                                        res_json
                                                                    );

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
                                                                     let duration =
                                                                        start.elapsed().as_millis()
                                                                            as usize;
                                                                    record_request(rt_metrics.as_ref(), &request_name, duration, false);
                                                                    tracing::error!(
                                                                        "[GRPC] Call failed: {}",
                                                                        e
                                                                    );
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!(
                                                    "[GRPC] Method discovery failed: {}",
                                                    e
                                                );
                                            }
                                        }
                                    } else {
                                        tracing::warn!(
                                            "[GRPC] No reflection client found for {}. Fallback to static call.",
                                            host_key
                                        );
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
                                                    record_request(rt_metrics.as_ref(), &request_name, duration, true);
                                                    let res = response.into_inner();
                                                    tracing::info!("[GRPC-STATIC] {} success: {}", request_name, res.message);
                                                }
                                                Err(e) => {
                                                    let duration = start.elapsed().as_millis() as usize;
                                                    record_request(rt_metrics.as_ref(), &request_name, duration, false);
                                                    tracing::error!("[GRPC-STATIC] {} failed: {}", request_name, e);
                                                }
                                            }
                                        }
                                    }
                                    }
                                    #[cfg(not(feature = "grpc"))]
                                    {
                                        tracing::warn!("[GRPC] gRPC support not compiled in. Enable the 'grpc' feature.");
                                    }
                                }
                                _ => {
                                    let method =
                                        d.method.clone().unwrap_or(HttpMethod::Get);

                                    // Body: prefer body_file if set, fall back to inline body
                                    let final_body = eval_macros!(
                                        resolve_body(
                                            d.body.as_deref(),
                                            d.body_file.as_deref(),
                                            &variables,
                                            record,
                                        )
                                        .await
                                    );

                                    let method_enum = match method {
                                        HttpMethod::Post => GooseMethod::Post,
                                        HttpMethod::Put => GooseMethod::Put,
                                        HttpMethod::Delete => GooseMethod::Delete,
                                        HttpMethod::Patch => GooseMethod::Patch,
                                        HttpMethod::Head => GooseMethod::Head,
                                        HttpMethod::Get => GooseMethod::Get,
                                    };

                                    tracing::debug!(
                                        "[CONFIG] request_name={}, timeout={:?}, body_file={}, headers={}",
                                        request_name,
                                        d.timeout,
                                        d.body_file.as_deref().unwrap_or("none"),
                                        d.headers.as_ref().map_or(0, HashMap::len)
                                    );

                                    let mut attempts = 0;
                                    let mut goose_response;
                                    let start = std::time::Instant::now();
                                    loop {
                                        let mut reqwest_builder = match method_enum.clone() {
                                            GooseMethod::Post => user.client.post(&full_url),
                                            GooseMethod::Put => user.client.put(&full_url),
                                            GooseMethod::Delete => user.client.delete(&full_url),
                                            GooseMethod::Patch => user.client.patch(&full_url),
                                            GooseMethod::Head => user.client.head(&full_url),
                                            GooseMethod::Get => user.client.get(&full_url),
                                        };

                                        // Apply scenario-level + per-request headers
                                        if let Some(ref sh) = *scenario_headers {
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
                                                reqwest_builder =
                                                    reqwest_builder.timeout(Duration::from_millis(ms));
                                            }
                                        }

                                        let goose_request = if final_body.is_empty() {
                                            GooseRequest::builder()
                                                .method(method_enum.clone())
                                                .path(request_name.as_str())
                                                .set_request_builder(reqwest_builder)
                                                .build()
                                        } else {
                                            GooseRequest::builder()
                                                .method(method_enum.clone())
                                                .path(request_name.as_str())
                                                .set_request_builder(reqwest_builder.body(final_body.clone()))
                                                .build()
                                        };

                                        goose_response = user.request(goose_request).await?;
                                        attempts += 1;

                                        if let Err(ref e) = goose_response.response {
                                            let err_str = e.to_string().to_lowercase();
                                            if (err_str.contains("connect")
                                                || err_str.contains("reset")
                                                || err_str.contains("pool")
                                                || err_str.contains("broken pipe"))
                                                && attempts < 3
                                            {
                                                tracing::warn!(
                                                    "[THROTTLE] Connection error detected: {}. Throttling and retrying (attempt {}/3)...",
                                                     err_str,
                                                     attempts
                                                );
                                                tokio::time::sleep(Duration::from_millis(50)).await;
                                                continue;
                                            }
                                        }
                                        break;
                                    }
                                    let duration = start.elapsed().as_millis() as usize;

                                    // Update real-time metrics
                                    record_request(
                                        rt_metrics.as_ref(),
                                        &request_name,
                                        duration,
                                        goose_response.response.is_ok(),
                                    );

                                    {
                                        let (status, text) = match goose_response.response {
                                            Ok(resp) => {
                                                let s = resp.status().as_u16();
                                                let t = resp.text().await.unwrap_or_default();
                                                (s, t)
                                            }
                                            Err(_) => (0, String::new()),
                                        };

                                        for a in &d.assert {
                                            if let Err(e) =
                                                AssertionEngine::check_assertion(&text, status, a)
                                            {
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
                                            #[cfg(feature = "xpath")]
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
                                    tracing::trace!(
                                        "[SESSION] {} variables updated for loop",
                                        variables.len()
                                    );
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

fn extract_default_host(scenario_def: &ScenarioDefinition) -> Option<String> {
    for req in &scenario_def.requests {
        let url_str = match req {
            HTTPRequestDefinition::Simple(url) => url.as_str(),
            HTTPRequestDefinition::Detailed(d) => d.url.as_str(),
        };
        if url_str.starts_with("http://")
            || url_str.starts_with("https://")
            || url_str.starts_with("ws://")
            || url_str.starts_with("wss://")
            || url_str.starts_with("grpc://")
        {
            if let Some((scheme, rest)) = url_str.split_once("://") {
                let host_part = rest.split('/')
                    .next()?
                    .split('?')
                    .next()?
                    .split('#')
                    .next()?;
                if !host_part.is_empty() {
                    return Some(format!("{}://{}", scheme, host_part));
                }
            }
        }
    }
    None
}

#[allow(clippy::cast_precision_loss)]
#[allow(clippy::cast_possible_truncation)]
fn parse_hatch_rate(ramp_up: &str, concurrency: usize) -> String {
    let ms = parse_time_to_ms(ramp_up);
    let seconds = ms as f32 / 1000.0;
    if seconds <= 0.0 {
        concurrency.to_string()
    } else {
        (concurrency as f32 / seconds).to_string()
    }
}

#[allow(clippy::cast_possible_truncation)]
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
    use crate::models::config::{DetailedRequest, ExecutionPlan, Protocol, ScenarioDefinition};

    #[tokio::test]
    async fn test_translate_basic() {
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
    async fn test_translate_protocols() {
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
                        protocol: Some(Protocol::Websocket),
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
                        protocol: Some(Protocol::Grpc),
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

    #[test]
    fn test_extract_default_host_resolution() {
        let mut scenarios = HashMap::new();
        scenarios.insert(
            "test".to_string(),
            ScenarioDefinition {
                requests: vec![
                    HTTPRequestDefinition::Simple("/relative-path".to_string()),
                    HTTPRequestDefinition::Simple("https://my-domain.org:9000/api/v1".to_string()),
                ],
                weight: 1,
                think_time: None,
                data_sources: None,
                headers: None,
            },
        );
        let scenario_def = scenarios.get("test").unwrap();
        let extracted = extract_default_host(scenario_def);
        assert_eq!(extracted, Some("https://my-domain.org:9000".to_string()));
    }
}
