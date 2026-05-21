# bzt-rs

**bzt-rs** is a high-performance, protocol-agnostic load generator that bridges the simplicity of Taurus YAML configurations with the extreme scale of the Rust-native attack engine.

Built for mission-critical SRE workflows, `bzt-rs` allows you to define complex scenarios once and execute them across REST, WebSockets, and gRPC with unified observability.

## 🚀 Key Features

- **Multi-Protocol Power**: Native support for **HTTP/S**, **WebSocket**, and dynamic **gRPC** (Unary & Streaming).
- **Dynamic gRPC Reflection**: Test any gRPC service at runtime without pre-compiled `.proto` files.
- **Real-Time Observability**: Periodic metrics shipping to **InfluxDB** with unique worker identification for distributed "Gaggle" runs.
- **Advanced Flow Control**: Complex branching with numeric comparisons (`>`, `<=`) and logical operators (`&&`, `||`).
- **Stateful Extraction**: Pull variables using **JSONPath**, **Regex**, or full **XPath 2.0** from responses.
- **Smart Validation**: verify configurations, file dependencies, and protocol schemas using `--dry-run`.
- **Integrated Mocking**: Protocol-aware mock server (Axum/Tonic) for functional verification of scenarios.
- **Security First**: Path traversal prevention, sensitive variable masking (`[REDACTED]`), and restricted local-only mock binding.

## 📦 Protocol Matrix

| Feature | HTTP/S | WebSocket | gRPC (Dynamic) |
| :--- | :---: | :---: | :---: |
| **Methods** | GET-PATCH | Text frames | Unary & Server-Stream |
| **Assertions** | Body/Status | Message Content | Response Message |
| **Extraction** | JSON/Regex/XML | Regex/XML | Regex/JSON |
| **Mocking** | Integrated (Axum) | Integrated (Axum) | Integrated (Tonic) |
| **Real-time Metrics** | ✅ | ✅ | ✅ |

## 🛠️ Installation

**Prerequisites**: Rust 2024+ toolchain.

```bash
git clone https://github.com/py-centric/bzt-rs.git
cd bzt-rs
cargo build --release
```

Binary: `./target/release/bzt-rs`.

## 📖 Common Patterns

### Dynamic gRPC (Unary & Streaming)
`bzt-rs` uses gRPC reflection to discover service schemas at runtime. No code generation needed.

```yaml
scenarios:
  streaming_test:
    requests:
      - url: grpc://localhost:50051
        protocol: grpc
        method-name: my.service.v1.RealtimeService/GetStatus
        grpc-mode: server-streaming  # processes every message in the stream
        body: '{"id": "user-123"}'
        assert:
          - contains: ["ACTIVE"]
            subject: body
```

### WebSocket Testing
Bidirectional text frame testing with integrated response assertions.

```yaml
scenarios:
  chat_test:
    requests:
      - url: ws://localhost:8080/ws
        protocol: websocket
        message: "Hello Server"
        assert:
          - contains: ["Welcome"]
```

### Advanced XPath Extraction
Extract variables from complex XML structures (e.g., SOAP).

```yaml
requests:
  - url: https://api.example.com/soap
    extract-xpath:
      token: "//auth:Session/token/text()"
    assert:
      - contains: ["Success"]
```

### Real-time InfluxDB Reporting
Monitor your load test as it happens with periodic metric pushes.

```yaml
reporting:
  - module: influxdb
    url: http://influxdb:8086
    bucket: load_tests
    token: ${env.INFLUX_TOKEN}
    interval: 10s  # push cumulative metrics every 10 seconds
```

## 🏗️ Architecture

`bzt-rs` follows a modular pipeline optimized for async concurrency:

1.  **Parser & Normalizer**: Deserializes multi-format configs and resolves hierarchical scenario inheritance.
2.  **Async Translator**: Performs runtime gRPC discovery and maps the configuration to non-blocking Goose transactions.
3.  **Specialized Engines**:
    *   **Logic**: `ControlFlowEngine` for branching; `AssertionEngine` for verification.
    *   **State**: `ExtractionEngine` for JSON/Regex/XPath variable capture.
    *   **Observability**: `SlaEngine` for thresholds; `InfluxDbReporter` for live telemetry.
4.  **Integrated Mocking**: Spin up an internal Axum/Tonic server to verify your logic without a live backend.

## ⚖️ License

Apache License 2.0.
