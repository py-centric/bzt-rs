use bzt_rs::engine;
use bzt_rs::normalizer::{SchemaNormalizer, ShorthandConfiguration};
use bzt_rs::parser::json::JsonParser;
use bzt_rs::parser::toml::TomlParser;
use bzt_rs::parser::yaml::YamlParser;
use clap::Parser;
use std::fs;

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
}

#[tokio::main]
async fn main() -> Result<(), String> {
    engine::init_logging();
    let args = Args::parse();

    let config_path = &args.config;
    let config = if config_path.ends_with(".yaml") || config_path.ends_with(".yml") {
        YamlParser::parse(config_path)?
    } else if config_path.ends_with(".json") {
        JsonParser::parse(config_path)?
    } else if config_path.ends_with(".toml") {
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
        // REQ-8.1: Start metrics server
        println!("Starting metrics server on port {}", args.metrics_port);
        // Skeleton: In a real implementation, we'd start an Axum server here
    }

    engine::goose::run_attack(config).await?;

    Ok(())
}
