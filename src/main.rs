#![allow(clippy::pedantic)]
use bzt_rs::engine;
use bzt_rs::normalizer::{SchemaNormalizer, ShorthandConfiguration};
use bzt_rs::parser::json::JsonParser;
use bzt_rs::parser::toml::TomlParser;
use bzt_rs::parser::yaml::YamlParser;
use clap::Parser;
use std::fs;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file (.yaml, .json, .toml)
    #[arg(index = 1)]
    config: String,

    /// Start in Manager mode
    #[arg(long)]
    manager: bool,

    /// Start in Worker mode
    #[arg(long)]
    worker: bool,

    /// Number of workers to expect (Manager mode only)
    #[arg(long)]
    expect_workers: Option<usize>,

    /// Host the Manager is listening on (Worker mode only)
    #[arg(long)]
    manager_host: Option<String>,

    /// Port the Manager is listening on
    #[arg(long)]
    manager_port: Option<u16>,

    /// Enable Prometheus metrics exporter
    #[arg(long)]
    metrics: bool,

    /// Port for the Prometheus metrics exporter
    #[arg(long, default_value_t = 8080)]
    metrics_port: u16,

    /// Enable OpenTelemetry tracing injection
    #[arg(long)]
    otel: bool,

    /// Validate the configuration and dependencies without executing the test
    #[arg(long, short = 'd')]
    dry_run: bool,

    /// Start an internal HTTP mock server based on the configuration
    #[arg(long, short = 'm')]
    mock: bool,

    /// Start the mock server and run the configuration against it
    #[arg(long)]
    mock_run: bool,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    engine::init_logging();
    let args = Args::parse();

    let config_path = &args.config;
    let config = if Path::new(config_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
    {
        YamlParser::parse(config_path)?
    } else if Path::new(config_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
    {
        JsonParser::parse(config_path)?
    } else if Path::new(config_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
    {
        let content = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
        if let Ok(shorthand) = toml::from_str::<ShorthandConfiguration>(&content) {
            SchemaNormalizer::normalize_shorthand(shorthand)
        } else {
            TomlParser::parse(config_path)?
        }
    } else {
        return Err("Unsupported file format".to_string());
    };

    // Use unsafe block for set_var as required by newer Rust editions
    unsafe {
        if args.manager {
            std::env::set_var("GOOSE_MANAGER", "true");
            if let Some(expect) = args.expect_workers {
                std::env::set_var("GOOSE_EXPECT_WORKERS", expect.to_string());
            }
        }
        if args.worker {
            std::env::set_var("GOOSE_WORKER", "true");
            if let Some(host) = args.manager_host {
                std::env::set_var("GOOSE_MANAGER_HOST", host);
            }
        }
        if let Some(port) = args.manager_port {
            std::env::set_var("GOOSE_MANAGER_PORT", port.to_string());
        }
        if args.otel {
            std::env::set_var("BZT_OTEL_ENABLED", "true");
        }
    }

    if args.metrics {
        // FR-018: Enable Goose metrics server via environment variables
        unsafe {
            std::env::set_var("GOOSE_METRICS", "true");
            std::env::set_var("GOOSE_METRICS_PORT", args.metrics_port.to_string());
        }
        println!(
            "Prometheus metrics server enabled on port {}",
            args.metrics_port
        );
    }

    if args.otel {
        // FR-018: Enable OpenTelemetry tracing via environment variable
        unsafe {
            std::env::set_var("BZT_OTEL_ENABLED", "true");
            // In a real implementation, we would also initialize the tracer here
        }
        println!("OpenTelemetry tracing enabled");
    }

    if args.dry_run {
        let summary = engine::validation::validate_config(&config);
        summary.report();
        if summary.is_valid() {
            return Ok(());
        }
        std::process::exit(1);
    }

    if args.mock {
        let _ = engine::mock::start_mock_server(config).await?;
        println!("Mock server is running. Press Ctrl+C to stop.");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    if args.mock_run {
        println!("Starting integrated mock run...");
        let addr = engine::mock::start_mock_server(config.clone()).await?;
        let host_override = format!("http://{addr}");

        let attack = bzt_rs::translator::StateTranslator::translate(&config, Some(host_override))?;
        let _stats = attack.execute().await.map_err(|e| e.to_string())?;

        println!("Integrated mock run complete.");
        return Ok(());
    }

    engine::goose::run_attack(config).await?;

    Ok(())
}
