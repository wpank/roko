# Depth 18-01: The Connect Contract

> Five-method async lifecycle protocol for external system I/O. Transport-independent.
> Implementations: one concrete HTTP JSON adapter (R01).

**Source:** `crates/roko-core/src/connector.rs`

---

## 1. The Five Methods

The `Connect` trait defines the complete external-system lifecycle:

```rust
#[async_trait]
pub trait Connect: Send + Sync {
    async fn connect(&mut self, config: &ConnectConfig) -> Result<()>;
    async fn query(&self, req: QueryRequest) -> Result<QueryResponse>;
    async fn execute(&self, req: ExecuteRequest) -> Result<ExecuteResponse>;
    async fn health(&self) -> Result<ConnectHealthStatus>;
    async fn disconnect(&mut self) -> Result<()>;
}
```

### 1.1 connect

Establishes the external connection. Takes `&mut self` because it transitions internal
state. Failure leaves the connector unavailable -- a subsequent `health()` call should
return `Disconnected`. The config is validated before the call: empty endpoints and
zero timeouts are rejected.

### 1.2 query

Idempotent read operation. Takes `&self` because it does not modify connector state.
Returns structured `QueryResponse { data: Value, latency_ms: u64 }`.

### 1.3 execute

Potentially mutating operation. Same signature pattern as `query` but semantically distinct:
callers know that this may change external state. Returns
`ExecuteResponse { result: Value, latency_ms: u64 }`.

### 1.4 health

Returns the current transport health:

```rust
pub struct ConnectHealthStatus {
    pub status: ConnectorStatus,  // Connected | Disconnected | Degraded
    pub latency_ms: u64,
    pub last_check: DateTime<Utc>,
    pub error: Option<String>,
}
```

Health checks are passive -- they report state, they do not attempt reconnection.

### 1.5 disconnect

Gracefully releases connection resources. Idempotent: calling `disconnect` on an already
disconnected connector is a no-op. Takes `&mut self` because it transitions internal state.

---

## 2. ConnectConfig

```rust
pub struct ConnectConfig {
    pub endpoint: String,
    pub auth: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub timeout_ms: u64,
}
```

### 2.1 Credential Safety

The `auth` and `headers` fields are both annotated with `#[serde(default, skip_serializing)]`.
This means:

- They are never included in serialized output (logs, JSON dumps, state files)
- They default to `None`/empty when deserializing from sources that omit them
- The `Debug` implementation for `ConnectConfig` redacts `auth` as `"[REDACTED]"` and
  shows only the header count

### 2.2 Validation Invariants

`ConnectConfig::validate()` enforces:
- `endpoint` must not be empty or whitespace-only
- `timeout_ms` must be greater than zero

The default timeout is 5000ms, applied by serde when the field is absent.

---

## 3. Request and Response Types

### QueryRequest / ExecuteRequest

Both carry `operation: String` and `params: Value`. Both validate that `operation` is
non-empty. The `params` field defaults to `Value::Null` when omitted from JSON input.

### QueryResponse / ExecuteResponse

Both carry a result/data `Value` and `latency_ms: u64`. The latency is measured by the
`Connect` implementation, not by the caller.

---

## 4. ConnectorManifest

Static identity that a connector publishes at registration time:

```rust
pub struct ConnectorManifest {
    pub name: String,
    pub kind: ConnectorKind,
    pub version: String,
    pub description: String,
    pub config_schema: Option<Value>,
    pub capabilities: Vec<String>,
    pub health_interval_secs: u64,
    pub reconnect_strategy: ReconnectStrategy,
}
```

Validation: name and version must be non-empty, `health_interval_secs` must be > 0, and
the reconnect strategy must pass its own validation.

---

## 5. ReconnectStrategy

```rust
pub enum ReconnectStrategy {
    ExponentialBackoff { base_ms: u64, max_ms: u64, jitter: bool },
    FixedInterval { interval_ms: u64 },
    Manual,
}
```

Validation rejects non-progressing configurations:
- `ExponentialBackoff`: requires `base_ms > 0` and `max_ms >= base_ms`
- `FixedInterval`: requires `interval_ms > 0`
- `Manual`: always valid

---

## 6. ConnectorKind

The eleven connector kinds with backward-compatible serde aliases:

| Kind | Aliases | Purpose |
|------|---------|---------|
| `Mcp` | -- | Model Context Protocol server |
| `Api` | -- | Generic REST/gRPC API |
| `Database` | -- | Relational or document database |
| `Blockchain` | -- | On-chain RPC endpoint |
| `Feed` | -- | Streaming data feed |
| `Custom` | -- | User-defined |
| `ChainRpc` | `chain-rpc`, `blockchain_rpc` | Chain RPC (v2 spelling) |
| `Exchange` | -- | Centralized exchange API |
| `McpServer` | `mcp-server` | MCP tool server (v2 spelling) |
| `A2aClient` | `a2a`, `a2a-client` | Agent-to-Agent peer |
| `Webhook` | -- | Outbound HTTP webhook |

---

## 7. ConnectorRegistry

In-memory descriptor registry for the HTTP discovery API:

```rust
pub struct ConnectorRegistry {
    connectors: Vec<ConnectorInfo>,
}
```

Operations:
- `register(info)` -- upsert by name (replace if exists)
- `unregister(name)` -- remove by name, returns `bool`
- `get(name)` -- O(n) lookup
- `list()` -- slice of all entries
- `healthy_count()` -- count of `Connected` entries

The registry is `Clone + Serialize + Deserialize` for state persistence and HTTP responses.

Note: new transport implementations should implement `Connect` directly. The registry is
retained for the existing HTTP discovery API surface.

---

## Verification

```bash
# Five-method lifecycle (MockConnect)
cargo test -p roko-core async_connect_contract_exercises_all_five_methods

# ConnectConfig backward-compatible timeout and validation
cargo test -p roko-core connect_config_has_backward_compatible_timeout_and_rejects_zero

# ConnectorKind serde aliases
cargo test -p roko-core connector_kind_accepts_documented_compatibility_spellings

# ConnectorManifest validation and serde round-trip
cargo test -p roko-core manifest_validation_rejects_non_progressing_reconnect_policies

# ConnectorRegistry operations
cargo test -p roko-core register_and_list
cargo test -p roko-core register_replaces_existing
cargo test -p roko-core unregister_returns_true_when_present
cargo test -p roko-core healthy_count_filters_connected
cargo test -p roko-core serde_roundtrip
```
