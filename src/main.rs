use bzt_rs::engine;
use bzt_rs::engine::BztError;
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
async fn main() -> Result<(), BztError> {
    engine::init_logging();
    let args = Args::parse();
    tracing::debug!(
        "CLI flags: dry_run={}, mock={}, mock_run={}",
        args.dry_run,
        args.mock,
        args.mock_run
    );

    let config_path = &args.config;
    let config = if Path::new(config_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
    {
        tracing::info!("Parsing YAML config: {}", config_path);
        YamlParser::parse(config_path)?
    } else if Path::new(config_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
    {
        tracing::info!("Parsing JSON config: {}", config_path);
        JsonParser::parse(config_path)?
    } else if Path::new(config_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
    {
        tracing::info!("Parsing TOML config: {}", config_path);
        let content = fs::read_to_string(config_path)?;
        if let Ok(shorthand) = toml::from_str::<ShorthandConfiguration>(&content) {
            SchemaNormalizer::normalize_shorthand(shorthand)
        } else {
            TomlParser::parse(config_path)?
        }
    } else {
        tracing::error!("Unsupported file format: {}", config_path);
        return Err(BztError::Validation {
            field: "file_extension".to_string(),
            reason: "Unsupported file format. Supported: .yaml, .yml, .json, .toml".to_string(),
        });
    };

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
        let _stats = attack
            .execute()
            .await
            .map_err(|e| BztError::Goose(Box::new(e)))?;

        println!("Integrated mock run complete.");
        return Ok(());
    }

    engine::goose::run_attack(config).await?;

    Ok(())
}
