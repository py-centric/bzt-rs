use bzt_rs::engine;
use bzt_rs::engine::BztError;
use bzt_rs::models::config::Configuration;
use bzt_rs::normalizer::{SchemaNormalizer, ShorthandConfiguration};
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

    /// Override option values (e.g. -o modules.console.disable=true)
    #[arg(short = 'o', value_name = "key=value")]
    option: Vec<String>,
}

fn yaml_to_json(yaml_val: serde_yaml::Value) -> serde_json::Value {
    match yaml_val {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(b) => serde_json::Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(serde_json::Number::from(i))
            } else if let Some(u) = n.as_u64() {
                serde_json::Value::Number(serde_json::Number::from(u))
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        serde_yaml::Value::String(s) => serde_json::Value::String(s),
        serde_yaml::Value::Sequence(seq) => {
            serde_json::Value::Array(seq.into_iter().map(yaml_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut json_map = serde_json::Map::new();
            for (k, v) in map {
                if let Some(k_str) = k.as_str() {
                    json_map.insert(k_str.to_string(), yaml_to_json(v));
                }
            }
            serde_json::Value::Object(json_map)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_json(tagged.value),
    }
}

fn toml_to_json(toml_val: toml::Value) -> serde_json::Value {
    match toml_val {
        toml::Value::String(s) => serde_json::Value::String(s),
        toml::Value::Integer(i) => serde_json::Value::Number(serde_json::Number::from(i)),
        toml::Value::Float(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::Boolean(b) => serde_json::Value::Bool(b),
        toml::Value::Datetime(d) => serde_json::Value::String(d.to_string()),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(toml_to_json).collect())
        }
        toml::Value::Table(table) => {
            let mut map = serde_json::Map::new();
            for (k, v) in table {
                map.insert(k, toml_to_json(v));
            }
            serde_json::Value::Object(map)
        }
    }
}

fn apply_override(root: &mut serde_json::Value, path_str: &str, val_str: &str) -> Result<(), String> {
    let parts: Vec<&str> = path_str.split('.').collect();
    let mut current = root;

    for (i, &part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            let val = if let Ok(b) = val_str.parse::<bool>() {
                serde_json::Value::Bool(b)
            } else if let Ok(num) = val_str.parse::<i64>() {
                serde_json::Value::Number(serde_json::Number::from(num))
            } else if let Ok(num) = val_str.parse::<f64>() {
                serde_json::Number::from_f64(num)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::String(val_str.to_string()))
            } else if let Ok(v) = serde_json::from_str::<serde_json::Value>(val_str) {
                v
            } else {
                serde_json::Value::String(val_str.to_string())
            };

            match current {
                serde_json::Value::Object(map) => {
                    map.insert(part.to_string(), val);
                }
                serde_json::Value::Array(arr) => {
                    if let Ok(idx) = part.parse::<usize>() {
                        if idx >= arr.len() {
                            arr.resize(idx + 1, serde_json::Value::Null);
                        }
                        arr[idx] = val;
                    } else {
                        return Err(format!("Expected array index but found '{}' at path {}", part, path_str));
                    }
                }
                _ => {
                    return Err(format!("Cannot set property of non-object/non-array for path {}", path_str));
                }
            }
            return Ok(());
        }

        match current {
            serde_json::Value::Object(map) => {
                let next_is_array_idx = parts.get(i + 1).and_then(|p| p.parse::<usize>().ok()).is_some();
                current = map.entry(part.to_string()).or_insert_with(|| {
                    if next_is_array_idx {
                        serde_json::Value::Array(Vec::new())
                    } else {
                        serde_json::Value::Object(serde_json::Map::new())
                    }
                });
            }
            serde_json::Value::Array(arr) => {
                if let Ok(idx) = part.parse::<usize>() {
                    if idx >= arr.len() {
                        let next_is_array_idx = parts.get(i + 1).and_then(|p| p.parse::<usize>().ok()).is_some();
                        arr.resize_with(idx + 1, || {
                            if next_is_array_idx {
                                serde_json::Value::Array(Vec::new())
                            } else {
                                serde_json::Value::Object(serde_json::Map::new())
                            }
                        });
                    }
                    current = &mut arr[idx];
                } else {
                    return Err(format!("Expected array index but found '{}' at path {}", part, path_str));
                }
            }
            _ => {
                return Err(format!("Expected object or array at part '{}' for path {}", part, path_str));
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), BztError> {
    let args = Args::parse();

    // 1. Process overrides to check if console logging should be disabled
    let mut disable_console = false;
    for opt in &args.option {
        if let Some((path, val)) = opt.split_once('=') {
            let path_trimmed = path.trim();
            let val_trimmed = val.trim();
            if path_trimmed == "modules.console.disable" && val_trimmed == "true" {
                disable_console = true;
            }
        }
    }

    // 2. Initialize logging subscriber
    engine::init_logging(disable_console);

    tracing::debug!(
        "CLI flags: dry_run={}, mock={}, mock_run={}, option={:?}",
        args.dry_run,
        args.mock,
        args.mock_run,
        args.option
    );

    let config_path = &args.config;
    let content = fs::read_to_string(config_path)?;
    let mut json_val: serde_json::Value = if Path::new(config_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
    {
        tracing::info!("Parsing YAML config: {}", config_path);
        let yaml_val: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| BztError::Serde {
            message: e.to_string(),
            file_path: Some(config_path.clone()),
            source: None,
        })?;
        yaml_to_json(yaml_val)
    } else if Path::new(config_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
    {
        tracing::info!("Parsing JSON config: {}", config_path);
        serde_json::from_str(&content).map_err(|e| BztError::Serde {
            message: e.to_string(),
            file_path: Some(config_path.clone()),
            source: Some(Box::new(e)),
        })?
    } else if Path::new(config_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
    {
        tracing::info!("Parsing TOML config: {}", config_path);
        if let Ok(shorthand) = toml::from_str::<ShorthandConfiguration>(&content) {
            let normalized = SchemaNormalizer::normalize_shorthand(shorthand);
            serde_json::to_value(&normalized).map_err(|e| BztError::Serde {
                message: e.to_string(),
                file_path: Some(config_path.clone()),
                source: Some(Box::new(e)),
            })?
        } else {
            let toml_val: toml::Value = toml::from_str(&content).map_err(|e| BztError::Serde {
                message: e.to_string(),
                file_path: Some(config_path.clone()),
                source: Some(Box::new(e)),
            })?;
            toml_to_json(toml_val)
        }
    } else {
        tracing::error!("Unsupported file format: {}", config_path);
        return Err(BztError::Validation {
            field: "file_extension".to_string(),
            reason: "Unsupported file format. Supported: .yaml, .yml, .json, .toml".to_string(),
        });
    };

    // Apply configuration-level overrides
    for opt in &args.option {
        if let Some((path, val)) = opt.split_once('=') {
            let path_trimmed = path.trim();
            let val_trimmed = val.trim();
            if path_trimmed.starts_with("execution")
                || path_trimmed.starts_with("scenarios")
                || path_trimmed.starts_with("reporting")
                || path_trimmed.starts_with("services")
                || path_trimmed.starts_with("api")
            {
                tracing::info!("Applying config override: {} = {}", path_trimmed, val_trimmed);
                if let Err(e) = apply_override(&mut json_val, path_trimmed, val_trimmed) {
                    tracing::warn!("Failed to apply override '{}': {}", opt, e);
                }
            }
        }
    }

    // Deserialize into target strongly typed Configuration
    let config: Configuration = serde_json::from_value(json_val).map_err(|e| BztError::Serde {
        message: e.to_string(),
        file_path: Some(config_path.clone()),
        source: Some(Box::new(e)),
    })?;

    if args.dry_run {
        let summary = engine::validation::validate_config(&config).await;
        if !disable_console {
            summary.report();
        }
        if summary.is_valid() {
            return Ok(());
        }
        std::process::exit(1);
    }

    if args.mock {
        #[cfg(feature = "grpc")]
        {
            let _ = engine::mock::start_mock_server(config).await?;
            println!("Mock server is running. Press Ctrl+C to stop.");
            loop {
                tokio::time::sleep(std::time::Duration::from_hours(1)).await;
            }
        }
        #[cfg(not(feature = "grpc"))]
        {
            let _ = config;
            return Err(BztError::Internal(
                "Mock server requires the 'grpc' feature".to_string(),
            ));
        }
    }

    if args.mock_run {
        #[cfg(feature = "grpc")]
        {
            println!("Starting integrated mock run...");
            let addrs = engine::mock::start_mock_server(config.clone()).await?;
            let host_override = format!("http://{}", addrs.http_addr);

            let attack =
                bzt_rs::translator::StateTranslator::translate(&config, Some(host_override), None)
                    .await?;
            let _stats = attack
                .execute()
                .await
                .map_err(|e| BztError::Goose(Box::new(e)))?;

            println!("Integrated mock run complete.");
            return Ok(());
        }
        #[cfg(not(feature = "grpc"))]
        {
            let _ = config;
            return Err(BztError::Internal(
                "Mock run requires the 'grpc' feature".to_string(),
            ));
        }
    }

    engine::goose::run_attack(config).await?;

    Ok(())
}
