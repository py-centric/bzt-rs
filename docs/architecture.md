# pummel Architecture

`pummel` is a modular, high-throughput pipeline that transforms declarative performance specifications into concurrent asynchronous load generation tasks.

## System Overview

```mermaid
graph TB
    CLI["CLI<br/>src/main.rs"] -->|parse by extension| PARSER
    subgraph "Parsing Layer"
        PARSER["YamlParser / JsonParser / TomlParser"]
        NORM["SchemaNormalizer<br/>(dot-notation inheritance)"]
    end
    PARSER -->|Configuration| NORM
    NORM -->|Unified Configuration| CONFIG["Configuration"]
    CONFIG --> VALIDATION["Validation Engine<br/>(--dry-run)"]
    CONFIG --> MOCK_SRV["Mock Server<br/>(--mock / --mock-run)"]
    CONFIG --> TRANSLATOR["State Translator"]
    TRANSLATOR --> GOOSE["Goose Attack Engine"]
    GOOSE --> CLI_RPT["CLI Summary"]
    GOOSE --> JUNIT["JUnit XML"]
    GOOSE --> HTML["HTML Report"]
    GOOSE --> INFLUX["InfluxDB (real-time + final)"]
    GOOSE -.->|real-time metrics| SLA_RT["SLA Background Checker"]
    SLA_RT -.->|exec: on breach| HOOKS["Shell Hooks"]
    GOOSE --> API_SVR["Control API<br/>Axum :port"]
    API_SVR -->|POST /control/stop| GOOSE
```

## Pipeline Detail

```mermaid
graph LR
    subgraph "1. Parse"
        A1["File (.yaml/.json/.toml)"]
        A2["Format-specific parser"]
    end

    subgraph "2. Model"
        B1["Configuration struct<br/>(serde deserialization)"]
    end

    subgraph "3. Normalize"
        C1["Shorthand → Taurus mapping"]
        C2["Dot-notation inheritance<br/>(auth.search → auth + search)"]
    end

    subgraph "4. Translate"
        D1["gRPC reflection discovery"]
        D2["CSV data source loading"]
        D3["Build Goose Scenarios"]
        D4["Build Goose Transactions"]
    end

    subgraph "5. Execute"
        E1["Goose concurrent attack"]
        E2["Real-time metrics collection"]
        E3["SLA background evaluation"]
    end

    subgraph "6. Report"
        F1["CLI ASCII summary"]
        F2["JUnit XML"]
        F3["InfluxDB push"]
        F4["HTML report"]
    end

    A1 --> A2 --> B1 --> C1 --> C2 --> D1 --> D2 --> D3 --> D4 --> E1 --> F1
    E1 --> E2 --> E3
    E1 --> F2
    E1 --> F3
    E1 --> F4
```

## Module Dependency Map

```mermaid
graph TB
    subgraph "Core"
        ENGINE["engine/mod.rs<br/>PummelError enum"]
        MODELS["models/config.rs<br/>Configuration types"]
    end

    subgraph "Parsing & Normalization"
        PARSER_YAML["parser/yaml.rs"]
        PARSER_JSON["parser/json.rs"]
        PARSER_TOML["parser/toml.rs"]
        NORMALIZER["normalizer/mod.rs<br/>SchemaNormalizer"]
    end

    subgraph "Translation"
        TRANSLATOR["translator/mod.rs<br/>StateTranslator"]
    end

    subgraph "Engine Modules"
        GOOSE_MOD["engine/goose.rs<br/>run_attack()"]
        CF["engine/control_flow.rs<br/>AssertionEngine + ControlFlowEngine"]
        EXT["engine/extraction.rs<br/>ExtractionEngine + UserSession"]
        INTERP["engine/interpolation.rs<br/>Interpolator"]
        MACROS["engine/macros.rs<br/>MacroEvaluator + sensitive patterns"]
        ENV["engine/env.rs<br/>EnvironmentLoader"]
        PACING["engine/pacing.rs<br/>PacingEngine"]
        SLA["engine/sla.rs<br/>SlaEngine"]
        DS["engine/data_sources.rs<br/>CsvDataSource + validate_path()"]
        REPORTING["engine/reporting.rs<br/>CliSummary + JUnitReporter + InfluxDbReporter"]
        MOCK["engine/mock.rs<br/>start_mock_server()"]
        VALIDATION["engine/validation.rs<br/>validate_config()"]
        GRPC["engine/grpc_dynamic.rs<br/>DynamicGrpcClient"]
        UTILS["engine/utils.rs<br/>parse_time_to_ms()"]
    end

    PARSER_YAML --> MODELS
    PARSER_JSON --> MODELS
    PARSER_TOML --> MODELS
    NORMALIZER --> MODELS

    TRANSLATOR --> MODELS
    TRANSLATOR --> ENGINE
    TRANSLATOR --> CF
    TRANSLATOR --> EXT
    TRANSLATOR --> INTERP
    TRANSLATOR --> MACROS
    TRANSLATOR --> PACING
    TRANSLATOR --> DS
    TRANSLATOR --> GRPC

    GOOSE_MOD --> TRANSLATOR
    GOOSE_MOD --> REPORTING
    GOOSE_MOD --> SLA
    GOOSE_MOD --> ENGINE

    MOCK --> ENGINE
    MOCK --> MODELS

    VALIDATION --> ENGINE
    VALIDATION --> MODELS
    VALIDATION --> TRANSLATOR
    VALIDATION --> MACROS
    VALIDATION --> DS

    MACROS --> ENV
    ENV --> ENGINE

    PACING --> UTILS
    SLA --> REPORTING
```

## Data Flow: Config File to Load Test

```mermaid
sequenceDiagram
    participant User
    participant CLI as CLI (main.rs)
    participant Parser
    participant Normalizer
    participant Config as Configuration
    participant Translator as StateTranslator
    participant Goose
    participant Reporters

    User->>CLI: pummel config.yaml
    CLI->>Parser: Parse file by extension (.yaml/.json/.toml)

    alt TOML with shorthand syntax
        Parser->>Config: ShorthandConfiguration
        CLI->>Normalizer: normalize_shorthand()
        Normalizer->>Config: Unified Configuration (dot-notation resolved)
    else YAML or JSON
        Parser->>Config: Configuration
    end

    alt --dry-run
        CLI->>CLI: validate_config() → print validation report
    else --mock
        CLI->>CLI: start_mock_server(config) → listen on 127.0.0.1
    else --mock-run
        CLI->>CLI: start_mock_server(config)
        CLI->>Translator: translate(config, host_override=mock_addr)
    else normal run
        CLI->>Translator: translate(config)
    end

    Translator->>Translator: Discover gRPC services via reflection
    Translator->>Translator: Load CSV data sources (with path validation)
    Translator->>Translator: Build Goose Scenarios + Transactions
    Translator-->>Goose: GooseAttack

    Goose->>Goose: Execute concurrent load test
    Goose->>Reporters: Real-time InfluxDB push (if interval configured)
    Goose->>Reporters: SLA evaluation (every 1s background)
    Goose->>Reporters: Post-run: CLI summary, JUnit XML, HTML, final InfluxDB
```

## Security Architecture

```mermaid
graph TB
    subgraph "Input Validation"
        DNF["deny_unknown_fields<br/>Rejects typos at parse time"]
        PATH_VAL["validate_path()<br/>Rejects path traversal"]
        SIZE_VAL["check_file_size()<br/>Rejects files > 100MB"]
    end

    subgraph "Runtime Protection"
        SENSITIVE["is_sensitive_var()<br/>Pattern-based blocking"]
        MASK["mask_env_value()<br/>[REDACTED] in logs"]
        LOCALHOST["Mock server binds<br/>127.0.0.1 only"]
        XML_ESC["xml_escape()<br/>Prevents XML injection in JUnit"]
    end

    subgraph "Blocked Patterns"
        AWS["AWS_*"]
        SECRET["SECRET*"]
        KEY_SUFFIX["*_KEY"]
        TOKEN_SUFFIX["*_TOKEN"]
        PASS_SUFFIX["*_PASSWORD"]
        DB_PREFIX["DB_*"]
        PRIVATE["PRIVATE*"]
        DB_URL["DATABASE_URL"]
        PASS_CONTAINS["*PASSWORD*"]
    end

    AWS --> SENSITIVE
    SECRET --> SENSITIVE
    KEY_SUFFIX --> SENSITIVE
    TOKEN_SUFFIX --> SENSITIVE
    PASS_SUFFIX --> SENSITIVE
    DB_PREFIX --> SENSITIVE
    PRIVATE --> SENSITIVE
    DB_URL --> SENSITIVE
    PASS_CONTAINS --> SENSITIVE
    SENSITIVE --> MASK
```

## Core Components

### 1. Multi-Format Parser & Normalizer

Supports **YAML**, **JSON**, and **TOML**. The normalizer resolves dot-notation inheritance (e.g., `auth.search`), ensuring that parent initialization requests are correctly injected as `on_start` steps in child scenarios.

### 2. Async Validation & Security

Performs a non-destructive dry-run of the configuration:
- **Path traversal checks**: Canonicalizes paths and verifies they stay within the project directory
- **File size limits**: Rejects data sources and body files exceeding 100 MB
- **Sensitive env var scanning**: Warns about `${env.AWS_SECRET_KEY}`, `${env.DB_PASSWORD}`, etc. during validation; **rejects** them at runtime
- **Struct validation**: `#[serde(deny_unknown_fields)]` on all config structs catches typos

### 3. Dynamic gRPC Engine

Uses `prost-reflect` and `tonic-reflection` for generic gRPC support:
- **Reflection Phase**: Queries the target server for service descriptors during translation
- **Generic Codec**: Dynamically serializes JSON payloads into binary Protobuf and deserializes responses
- **Streaming**: Supports unary, server-streaming, client-streaming, and bidirectional streaming

### 4. Async State Translator

The bridge between the static configuration and the dynamic Goose engine:
- **Session State**: Per-user `UserSession` with variable storage
- **Interpolation**: Real-time `${var}` substitution from session variables
- **Macro Evaluation**: Dynamic data generation (`${faker.email}`, `${uuid}`) and environment variable resolution with sensitive var blocking
- **Protocol Dispatch**: Routes traffic to HTTP, WebSocket, or gRPC clients

### 5. Real-Time Observability

- **InfluxDB Reporter**: Background task consuming metrics from shared state, flushing to InfluxDB at configurable intervals
- **CLI Summary**: Always-printed ASCII table with per-endpoint metrics (requests, failures, avg/p95/p99 latency)
- **JUnit XML**: Post-test XML report for CI/CD integration
- **Worker ID**: Each worker tagged with a UUID for correct metric aggregation in distributed runs

### 6. Logic & Extraction Engines

- **ControlFlowEngine**: Evaluates conditions (`==`, `!=`, `>`, `<`, `>=`, `<=`) with logical operators (`&&`, `||`)
- **AssertionEngine**: Validates responses against body content, HTTP status codes, with regex and negation support
- **ExtractionEngine**: Multi-engine variable capture using JSONPath, Regex, and XPath 2.0 (via `sxd-xpath`)

### 7. Integrated Mock Server

A multi-protocol test utility (Axum for HTTP/WebSocket, Tonic for gRPC) that:
- Binds to `127.0.0.1` only (never exposed externally)
- Generates responses from assertion definitions in the config
- Supports all four gRPC streaming modes
- Uses random ports for parallel test execution

### 8. Chaos Engineering Support

- **Shell Hook Executor**: `services` block with `prepare`, `startup`, `shutdown` phases; shutdown always runs
- **Real-Time SLA Actions**: Background thread evaluates metrics every 1 second; fires `exec:` commands on breach
- **Dynamic Control API**: Optional Axum HTTP server exposing `GET /metrics` and `POST /control/stop`

## Performance Profile

By leveraging Rust's `tokio` runtime and the Goose engine, `pummel` maintains a near-zero CPU/memory footprint compared to Java or Python-based alternatives, while providing significantly deeper protocol control. The `--mock-run` mode adds no measurable overhead to the hot path.
