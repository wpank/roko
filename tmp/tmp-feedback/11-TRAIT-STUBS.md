# Dead Protocol Traits: Observe, Connect, Trigger — Full Implementation Design

## Architecture Context

Roko's architecture is "1 noun (Signal/Engram) + 6 verb traits." The universal loop
composes the six verbs into a single function (`loop_tick`) that drives every operation:

```
query → score → route → compose → act → verify → write → react
  │        │       │        │        │       │        │       │
Store    Score   Route   Compose   (agent)  Verify  Store   React
```

The six verb traits declared in `crates/roko-core/src/traits.rs`:

| # | Trait | Role in loop | Async | Implementations | Status |
|---|-------|-------------|-------|-----------------|--------|
| 1 | `Store` | query + write | Yes | 3 (FileSubstrate, MemorySubstrate, HdcSubstrate) | **Wired** |
| 2 | `Score` | score | No | 5+ (PredictiveScorer, CatalystScorer, SumScorer, etc.) | **Wired** |
| 3 | `Route` | route | No | 5 (FirstRouter, HighestScoreRouter, RoundRobinRouter, etc.) | **Wired** |
| 4 | `Compose` | compose | No | 3 (PromptComposer, SystemPromptBuilder, NoOpComposer) | **Wired** |
| 5 | `Verify` | verify | Yes | 20+ gates (CompileGate, TestGate, ClippyGate, DiffGate, ...) | **Wired** |
| 6 | `React` | react | No | 12+ (Conductor, 10 watchers, EpisodePolicy, CFactorPolicy) | **Wired** |

In addition to the six loop verbs, three protocol traits extend the architecture with
lifecycle/eventing capabilities that sit *outside* the per-tick loop:

| # | Trait | Architectural role | Async | Implementations | Status |
|---|-------|--------------------|-------|-----------------|--------|
| 7 | `Observe` | Pull-based sensing (read-only) | **No (stub)** | 0 | **Dead stub** |
| 8 | `Connect` | Bidirectional I/O lifecycle | **No (stub)** | 0 | **Dead stub** |
| 9 | `Trigger` | Event-driven activation | **No (stub)** | 0 | **Dead stub** |

Plus the supporting transport traits that ARE wired:

| Trait | Role | Status |
|-------|------|--------|
| `Bus` | Pub/sub for ephemeral `Pulse`s | Wired (PulseBus, BroadcastBus, MemoryBus, MultiBus) |
| `ColdStore` | Archival for aged-out engrams | Built (ArchiveColdSubstrate) |
| `Substrate` | Legacy alias for Store | Wired (blanket impl) |

### Where the three dead traits fit

```
                          ┌─────────────────────────────────────────────┐
                          │           Universal Loop (per-tick)         │
                          │  Store → Score → Route → Compose → Verify  │
                          │                    │                  │     │
                          │                    └──── React ◄──────┘     │
                          └──────────────────────┬──────────────────────┘
                                                 │
                    ┌────────────────────────────┬┴────────────────────────────┐
                    │                            │                            │
              ┌─────┴─────┐              ┌───────┴───────┐            ┌───────┴───────┐
              │  Observe   │              │   Connect     │            │   Trigger     │
              │            │              │               │            │               │
              │ Pull data  │              │ Manage I/O    │            │ React to      │
              │ from env   │              │ connections   │            │ events        │
              │ (read-only)│              │ (lifecycle)   │            │ (condition    │
              │            │              │               │            │  → fire)      │
              └─────┬──────┘              └───────┬───────┘            └───────┬───────┘
                    │                             │                            │
                    │   ┌─────────────────────────┤                            │
                    │   │                         │                            │
                    ▼   ▼                         ▼                            ▼
              ┌──────────────┐            ┌──────────────┐            ┌──────────────┐
              │    Store     │            │     Bus      │            │     Bus      │
              │  (persist    │            │  (publish    │            │  (subscribe  │
              │  observations│            │   pulses)    │            │   to topics) │
              │  as Engrams) │            │              │            │              │
              └──────────────┘            └──────────────┘            └──────────────┘
```

The three dead traits form the **sensing/connectivity/eventing layer** that wraps around
the per-tick loop. They answer:

- **Observe**: "What is the current state of the world?" (pull)
- **Connect**: "How do I talk to external systems?" (lifecycle)
- **Trigger**: "When should I do something?" (push/event)

The per-tick loop answers "What do I do *right now* with *this data*?" These three traits
feed data into and react to events from that loop.

---

## Current Definitions (Dead Stubs)

All three are in `crates/roko-core/src/traits.rs`, exported via:
```rust
// crates/roko-core/src/lib.rs:303
pub use traits::{Bus, ColdStore, Compose, Connect, Observe, React, Route, Store, Substrate, Trigger, Verify};
```

### Observe (`traits.rs:400-403`)

```rust
/// Observation protocol — passive data collection from external sources.
pub trait Observe: crate::cell::Cell {
    /// Collect observations from the environment.
    fn observe(&self) -> Vec<Engram>;
}
```

Problems:
1. Synchronous -- observation typically requires I/O (store queries, API calls, health checks)
2. Takes no `Context` parameter -- no way to pass runtime state, time, budget
3. Returns `Vec<Engram>` not `Result<Vec<Engram>>` -- cannot report failures
4. Zero implementations, zero callers

### Connect (`traits.rs:408-415`)

```rust
/// Connectivity protocol — manage connections to external systems.
pub trait Connect: crate::cell::Cell {
    /// Establish the connection.
    fn connect(&self) -> Result<()>;
    /// Check if the connection is healthy.
    fn health(&self) -> bool;
    /// Tear down the connection.
    fn disconnect(&self) -> Result<()>;
}
```

Problems:
1. Synchronous -- connections to APIs, databases require async I/O
2. `health()` returns bare `bool` -- no latency, error details, diagnostics
3. No `request()` method -- no way to send data through the connection
4. `connector.rs` (line 7-12) says "will be superseded by the Connect protocol trait" but Connect is a stub
5. Zero implementations, zero callers

### Trigger (`traits.rs:420-425`)

```rust
/// Trigger protocol — armed conditions that fire when criteria are met.
pub trait Trigger: crate::cell::Cell {
    /// Arm the trigger to begin watching.
    fn arm(&self) -> Result<()>;
    /// Disarm the trigger, stopping all watches.
    fn disarm(&self) -> Result<()>;
}
```

Problems:
1. Synchronous -- triggers watching async event streams need async I/O
2. `arm()` returns nothing -- no way to know what the trigger is watching
3. No `check()` method -- no way to evaluate if the trigger should fire
4. Zero implementations, zero callers

---

## Trait 1: Observe — Complete Implementation Design

### Design Intent

Observe is the **pull-based sensing** protocol. Observers passively read state from
internal or external sources and return observation Engrams. They are read-only: they
never modify the state they observe. They sit outside the per-tick loop and are called
on-demand by CLI commands (`roko status`), the TUI dashboard, the HTTP control plane,
and scheduled health checks.

### Trait Definition

File: `crates/roko-core/src/traits.rs` (replace lines 397-403)

```rust
// ─── Observe ─────────────────────────────────────────────────────────────

/// Passive data collection from internal or external state.
///
/// Unlike [`Connect`] (which manages bidirectional I/O), Observe is read-only:
/// it reads state and emits observation [`Engram`]s without side effects.
///
/// Observers are called on-demand by CLI commands (`roko status`), the TUI
/// dashboard, the HTTP control plane, and scheduled health checks. They
/// do NOT run inside the per-tick universal loop.
///
/// # Implementations
///
/// - `StoreObserver` — storage statistics (signal count, store name)
/// - `AgentObserver` — agent process health (running/stopped, PID, uptime)
/// - `SystemObserver` — system resource usage (memory, disk, CPU)
#[async_trait]
pub trait Observe: Cell {
    /// Collect observations from the environment. Returns zero or more
    /// observation engrams. Each engram should have `Kind::Metric` or
    /// `Kind::Custom("observe.*")` and include a JSON body with structured
    /// observation data.
    async fn observe(&self, ctx: &Context) -> Result<Vec<Engram>>;

    /// Human-readable name for this observer (defaults to Cell::cell_name).
    fn observer_name(&self) -> &str {
        self.cell_name()
    }

    /// Topic string for the observations this observer produces.
    /// Used for routing observation pulses on the Bus.
    /// Returns None if this observer does not publish to the Bus.
    fn observation_topic(&self) -> Option<&str> {
        None
    }
}
```

### Concrete Implementation 1: StoreObserver

File: `crates/roko-core/src/store_observer.rs` (new file)

```rust
//! Observes a Store and returns statistics as observation Engrams.

use crate::{
    cell::Cell, Body, CellVersion, Context, Engram, Kind, Provenance,
    traits::{Observe, Store},
    error::Result,
};
use async_trait::async_trait;
use std::sync::Arc;

/// Observes a [`Store`] and returns statistics as observation Engrams.
///
/// This is the simplest useful observer: it queries `Store::len()` and
/// `Store::name()` and returns a single metric engram with the results.
///
/// Wired into `roko status` via `cmd_status` in `commands/util.rs`.
pub struct StoreObserver {
    store: Arc<dyn Store>,
}

impl StoreObserver {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }
}

impl Cell for StoreObserver {
    fn cell_id(&self) -> &str { "store-observer" }
    fn cell_name(&self) -> &str { "Store Observer" }
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &["Observe"] }
}

#[async_trait]
impl Observe for StoreObserver {
    async fn observe(&self, _ctx: &Context) -> Result<Vec<Engram>> {
        let count = self.store.len().await?;
        let store_name = self.store.name();

        let body = serde_json::json!({
            "observer": "store-observer",
            "topic": "observe.store.stats",
            "store": store_name,
            "signal_count": count,
        });

        let engram = Engram::builder(Kind::Metric)
            .body(Body::Json(body))
            .provenance(Provenance::agent("store-observer"))
            .build();

        Ok(vec![engram])
    }

    fn observation_topic(&self) -> Option<&str> {
        Some("observe.store.stats")
    }
}
```

**Data flow**: `Store::len()` + `Store::name()` --> JSON body --> `Engram(Kind::Metric)`
**Storage**: Does not persist; caller decides whether to store the observation
**Config**: No config needed; constructed programmatically from the active Store

### Concrete Implementation 2: AgentObserver

File: `crates/roko-core/src/agent_observer.rs` (new file)

```rust
//! Observes agent processes and returns health status as observation Engrams.

use crate::{
    cell::Cell, Body, CellVersion, Context, Engram, Kind, Provenance,
    traits::Observe, error::Result,
};
use async_trait::async_trait;

/// Snapshot of an agent's process health.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentSnapshot {
    pub name: String,
    pub running: bool,
    pub pid: Option<u32>,
    pub uptime_secs: Option<u64>,
    pub last_error: Option<String>,
}

/// Observes agent processes via a provider function.
///
/// The provider function is injected to decouple from ProcessSupervisor
/// (which lives in roko-runtime, a downstream crate).
pub struct AgentObserver {
    /// Function that returns current agent snapshots.
    provider: Box<dyn Fn() -> Vec<AgentSnapshot> + Send + Sync>,
}

impl AgentObserver {
    pub fn new(provider: impl Fn() -> Vec<AgentSnapshot> + Send + Sync + 'static) -> Self {
        Self { provider: Box::new(provider) }
    }
}

impl Cell for AgentObserver {
    fn cell_id(&self) -> &str { "agent-observer" }
    fn cell_name(&self) -> &str { "Agent Observer" }
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &["Observe"] }
}

#[async_trait]
impl Observe for AgentObserver {
    async fn observe(&self, _ctx: &Context) -> Result<Vec<Engram>> {
        let snapshots = (self.provider)();
        let running = snapshots.iter().filter(|s| s.running).count();
        let total = snapshots.len();

        let body = serde_json::json!({
            "observer": "agent-observer",
            "topic": "observe.agent.health",
            "agents_total": total,
            "agents_running": running,
            "agents": snapshots,
        });

        let engram = Engram::builder(Kind::Metric)
            .body(Body::Json(body))
            .provenance(Provenance::agent("agent-observer"))
            .build();

        Ok(vec![engram])
    }

    fn observation_topic(&self) -> Option<&str> {
        Some("observe.agent.health")
    }
}
```

**Data flow**: `ProcessSupervisor` state (via closure) --> `AgentSnapshot` vec --> JSON body --> Engram
**Storage**: Does not persist; TUI/HTTP callers display live
**Config**: No config; wired at construction from ProcessSupervisor in orchestrate.rs

### Concrete Implementation 3: SystemObserver

File: `crates/roko-core/src/system_observer.rs` (new file)

```rust
//! Observes system resource usage (disk, memory).

use crate::{
    cell::Cell, Body, CellVersion, Context, Engram, Kind, Provenance,
    traits::Observe, error::Result,
};
use async_trait::async_trait;
use std::path::PathBuf;

/// Observes system resources for the .roko/ data directory.
pub struct SystemObserver {
    /// Path to the .roko/ data directory.
    data_dir: PathBuf,
}

impl SystemObserver {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn dir_size_bytes(path: &std::path::Path) -> u64 {
        std::fs::read_dir(path)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| {
                let meta = e.metadata().ok();
                if e.path().is_dir() {
                    Self::dir_size_bytes(&e.path())
                } else {
                    meta.map(|m| m.len()).unwrap_or(0)
                }
            })
            .sum()
    }
}

impl Cell for SystemObserver {
    fn cell_id(&self) -> &str { "system-observer" }
    fn cell_name(&self) -> &str { "System Observer" }
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &["Observe"] }
}

#[async_trait]
impl Observe for SystemObserver {
    async fn observe(&self, _ctx: &Context) -> Result<Vec<Engram>> {
        let data_dir = self.data_dir.clone();
        let size_bytes = tokio::task::spawn_blocking(move || {
            Self::dir_size_bytes(&data_dir)
        }).await.unwrap_or(0);

        let body = serde_json::json!({
            "observer": "system-observer",
            "topic": "observe.system.resources",
            "data_dir": self.data_dir.display().to_string(),
            "data_dir_bytes": size_bytes,
            "data_dir_mb": size_bytes as f64 / (1024.0 * 1024.0),
        });

        let engram = Engram::builder(Kind::Metric)
            .body(Body::Json(body))
            .provenance(Provenance::agent("system-observer"))
            .build();

        Ok(vec![engram])
    }

    fn observation_topic(&self) -> Option<&str> {
        Some("observe.system.resources")
    }
}
```

**Data flow**: `.roko/` directory scan --> size calculation --> JSON body --> Engram
**Storage**: Does not persist
**Config**: Takes `data_dir` path, typically `workdir.join(".roko")`

### Integration with Universal Loop

Observe does NOT participate in the per-tick loop directly. Instead:

1. **CLI on-demand**: `roko status` calls `StoreObserver::observe()` to get signal count
2. **TUI dashboard**: Dashboard tab polls observers every refresh interval
3. **HTTP control plane**: `GET /api/v1/observers` endpoint calls all registered observers
4. **Scheduled**: Future cron-based health checks call observers and persist results

```
roko status ──► StoreObserver::observe() ──► Engram { signal_count: N }
                                                 │
                                                 ▼
                                           Display / JSON output
```

### Wire Target

```bash
# StoreObserver replaces the direct store.query() in cmd_status
cargo run -p roko-cli -- status
# Output includes "signals: N" sourced through Observe trait
```

### Test Strategy

1. **Unit test** (`store_observer.rs`): Create in-memory store with known count, call
   `observe()`, assert body contains correct `signal_count`
2. **Unit test** (`agent_observer.rs`): Provide closure returning mock snapshots,
   assert body contains correct counts
3. **Integration test**: Run `roko status` and verify output includes signal count
   sourced through the observer path

---

## Trait 2: Connect — Complete Implementation Design

### Design Intent

Connect is the **bidirectional I/O lifecycle** protocol. It manages connections to
external systems (LLM APIs, databases, MCP servers, WebSocket streams) with
open/close/health lifecycle semantics. It replaces the ad-hoc `ConnectorRegistry`
in `connector.rs` with a trait-based approach.

The migration note in `connector.rs:7-12` explicitly states:
> This module will be superseded by the `Connect` protocol trait...
> Do not add new callers — prefer the Connect trait once available.

### Supporting Types

File: `crates/roko-core/src/traits.rs` (add before the Connect trait)

```rust
/// Health status of a connection managed by the [`Connect`] trait.
///
/// Richer than the legacy `ConnectorHealth` (which only has status/latency/timestamp).
/// `ConnectionHealth` includes error messages, the target name, and consecutive
/// failure tracking.
///
/// Note: This complements (does not replace) `ConnectorHealth` in `connector.rs`
/// and `ProviderHealth` in `roko-learn`. Those track historical circuit-breaker
/// state; `ConnectionHealth` reports live point-in-time reachability.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectionHealth {
    /// Whether the connection is currently established and responding.
    pub connected: bool,
    /// Round-trip latency in milliseconds (if measured).
    pub latency_ms: Option<f64>,
    /// Human-readable error message (if unhealthy).
    pub error: Option<String>,
    /// Name of the connection target (e.g., provider name, endpoint URL).
    pub target: String,
    /// Number of consecutive health-check failures (0 when healthy).
    pub consecutive_failures: u32,
}

impl ConnectionHealth {
    /// Construct a healthy status with measured latency.
    pub fn healthy(target: impl Into<String>, latency_ms: f64) -> Self {
        Self {
            connected: true,
            latency_ms: Some(latency_ms),
            error: None,
            target: target.into(),
            consecutive_failures: 0,
        }
    }

    /// Construct an unhealthy status with an error message.
    pub fn unhealthy(target: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            connected: false,
            latency_ms: None,
            error: Some(error.into()),
            target: target.into(),
            consecutive_failures: 1,
        }
    }
}
```

### Trait Definition

File: `crates/roko-core/src/traits.rs` (replace lines 405-415)

```rust
// ─── Connect ─────────────────────────────────────────────────────────────

/// Manages a connection to an external system with lifecycle semantics.
///
/// Connect provides open/close/health lifecycle for durable connections
/// (LLM APIs, databases, WebSocket streams, MCP servers). It is the
/// trait-based successor to `ConnectorRegistry` in `connector.rs`.
///
/// # Lifecycle
///
/// ```text
/// open(ctx) -> [healthy] -> request(engram, ctx) -> ... -> close(ctx)
///                        -> health(ctx) -> ConnectionHealth
/// ```
///
/// # Implementations
///
/// - `ProviderConnection` — LLM provider health (HTTP HEAD with timeout)
/// - `McpConnection` — MCP server lifecycle (spawn/health/shutdown)
/// - `WebSocketConnection` — WebSocket stream management
#[async_trait]
pub trait Connect: Cell {
    /// Open/establish the connection. Idempotent: calling open on an
    /// already-open connection is a no-op.
    async fn open(&self, ctx: &Context) -> Result<()>;

    /// Close/tear down the connection gracefully.
    async fn close(&self, ctx: &Context) -> Result<()>;

    /// Check connection health. Must be cheap (bounded timeout, no heavy I/O).
    /// Returns structured health information including latency and error details.
    async fn health(&self, ctx: &Context) -> Result<ConnectionHealth>;

    /// Send a request through the connection and receive a response.
    /// Not all connections support request/response (some are stream-only).
    /// Default implementation returns an error.
    async fn request(&self, input: Engram, ctx: &Context) -> Result<Engram> {
        let _ = (input, ctx);
        Err(crate::error::RokoError::Invalid(format!(
            "{}: request() not supported on this connection",
            self.cell_name()
        )))
    }
}
```

### Concrete Implementation 1: ProviderConnection

File: `crates/roko-cli/src/provider_connection.rs` (new file)

```rust
//! Connect implementation that checks LLM provider health.

use roko_core::{
    cell::Cell, CellVersion, Context, Engram, ConnectionHealth, traits::Connect,
    error::{Result, RokoError},
};
use async_trait::async_trait;

/// Connect implementation that checks LLM provider reachability.
///
/// For HTTP-based providers (Anthropic, OpenAI, Gemini, Perplexity),
/// performs a bounded HEAD request to the base URL. For CLI providers
/// (Claude CLI), checks command availability via `which`.
pub struct ProviderConnection {
    provider_name: String,
    endpoint: String,
    api_key_env: Option<String>,
    credential_available: bool,
    timeout_ms: u64,
}

impl ProviderConnection {
    pub fn new(
        provider_name: String,
        endpoint: String,
        api_key_env: Option<String>,
        credential_available: bool,
        timeout_ms: u64,
    ) -> Self {
        Self { provider_name, endpoint, api_key_env, credential_available, timeout_ms }
    }

    /// Create ProviderConnections from all providers in config.
    /// Uses `RokoConfig::is_provider_available` for credential detection.
    pub fn from_config(config: &roko_core::config::schema::RokoConfig) -> Vec<Self> {
        config.providers.iter().map(|(name, prov)| {
            let endpoint = prov.base_url.clone().unwrap_or_default();
            let available = config.is_provider_available(name);
            let timeout = prov.timeout_ms.unwrap_or(5000);
            Self::new(
                name.clone(),
                endpoint,
                prov.api_key_env.clone(),
                available,
                timeout,
            )
        }).collect()
    }
}

impl Cell for ProviderConnection {
    fn cell_id(&self) -> &str { &self.provider_name }
    fn cell_name(&self) -> &str { &self.provider_name }
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &["Connect"] }
}

#[async_trait]
impl Connect for ProviderConnection {
    async fn open(&self, _ctx: &Context) -> Result<()> {
        // HTTP is stateless; open is a no-op
        Ok(())
    }

    async fn close(&self, _ctx: &Context) -> Result<()> {
        Ok(())
    }

    async fn health(&self, _ctx: &Context) -> Result<ConnectionHealth> {
        if !self.credential_available {
            return Ok(ConnectionHealth::unhealthy(
                &self.provider_name,
                format!(
                    "API key env {} not set",
                    self.api_key_env.as_deref().unwrap_or("(none)")
                ),
            ));
        }

        if self.endpoint.is_empty() {
            return Ok(ConnectionHealth::unhealthy(
                &self.provider_name,
                "No base_url configured",
            ));
        }

        let start = std::time::Instant::now();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .build()
            .map_err(|e| RokoError::Transport(e.to_string()))?;

        match client.head(&self.endpoint).send().await {
            Ok(resp) => {
                let latency = start.elapsed().as_secs_f64() * 1000.0;
                let status = resp.status().as_u16();
                // 401/403 = endpoint reachable (auth is a separate concern)
                if resp.status().is_success() || status == 401 || status == 403 {
                    Ok(ConnectionHealth::healthy(&self.provider_name, latency))
                } else {
                    Ok(ConnectionHealth::unhealthy(
                        &self.provider_name,
                        format!("HTTP {}", resp.status()),
                    ))
                }
            }
            Err(e) => Ok(ConnectionHealth::unhealthy(&self.provider_name, e.to_string())),
        }
    }

    async fn request(&self, _input: Engram, _ctx: &Context) -> Result<Engram> {
        Err(RokoError::Invalid(
            "ProviderConnection::request() not implemented — use agent dispatch".into(),
        ))
    }
}
```

**Data flow**: Config providers --> `ProviderConnection` per provider --> `Connect::health()` --> `ConnectionHealth`
**Storage**: Does not persist; `roko-learn`'s circuit breaker handles historical tracking
**Config**: Reads `[providers.*]` from `roko.toml`; uses `base_url`, `api_key_env`, `timeout_ms`

### Concrete Implementation 2: McpConnection

File: `crates/roko-agent/src/mcp_connection.rs` (new file)

```rust
//! Connect implementation for MCP (Model Context Protocol) servers.

use roko_core::{
    cell::Cell, CellVersion, Context, Engram, ConnectionHealth, traits::Connect,
    error::Result,
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};

/// Manages an MCP server connection lifecycle.
///
/// MCP servers are spawned as child processes. Connect::open() starts the
/// process, health() checks if it's still running, close() sends SIGTERM.
pub struct McpConnection {
    name: String,
    command: String,
    args: Vec<String>,
    connected: AtomicBool,
}

impl McpConnection {
    pub fn new(name: String, command: String, args: Vec<String>) -> Self {
        Self {
            name,
            command,
            args,
            connected: AtomicBool::new(false),
        }
    }
}

impl Cell for McpConnection {
    fn cell_id(&self) -> &str { &self.name }
    fn cell_name(&self) -> &str { &self.name }
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &["Connect"] }
}

#[async_trait]
impl Connect for McpConnection {
    async fn open(&self, _ctx: &Context) -> Result<()> {
        // Check if the MCP command exists
        let output = tokio::process::Command::new("which")
            .arg(&self.command)
            .output()
            .await
            .map_err(|e| roko_core::error::RokoError::Transport(e.to_string()))?;

        if output.status.success() {
            self.connected.store(true, Ordering::SeqCst);
            Ok(())
        } else {
            Err(roko_core::error::RokoError::Transport(
                format!("MCP command '{}' not found in PATH", self.command),
            ))
        }
    }

    async fn close(&self, _ctx: &Context) -> Result<()> {
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn health(&self, _ctx: &Context) -> Result<ConnectionHealth> {
        if !self.connected.load(Ordering::SeqCst) {
            return Ok(ConnectionHealth::unhealthy(
                &self.name,
                "MCP server not started",
            ));
        }
        Ok(ConnectionHealth::healthy(&self.name, 0.0))
    }
}
```

**Data flow**: MCP config --> `McpConnection` --> process lifecycle management
**Storage**: In-memory `AtomicBool` for connection state
**Config**: Reads `[agent.mcp_config]` from `roko.toml`

### Concrete Implementation 3: ConnectorBridge

File: `crates/roko-core/src/connector_bridge.rs` (new file)

```rust
//! Bridge from legacy ConnectorRegistry entries to the Connect trait.
//!
//! Allows existing ConnectorInfo entries to be used through the Connect
//! trait interface, enabling incremental migration from ConnectorRegistry.

use crate::{
    cell::Cell, CellVersion, Context, Engram, ConnectionHealth,
    connector::{ConnectorInfo, ConnectorStatus},
    traits::Connect, error::Result,
};
use async_trait::async_trait;

/// Wraps a legacy `ConnectorInfo` to expose it via the `Connect` trait.
///
/// This bridge enables the `connector.rs` migration: existing registry
/// entries work through the new trait interface without rewriting callers
/// all at once.
pub struct ConnectorBridge {
    info: ConnectorInfo,
}

impl ConnectorBridge {
    pub fn new(info: ConnectorInfo) -> Self {
        Self { info }
    }
}

impl Cell for ConnectorBridge {
    fn cell_id(&self) -> &str { &self.info.name }
    fn cell_name(&self) -> &str { &self.info.name }
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &["Connect"] }
}

#[async_trait]
impl Connect for ConnectorBridge {
    async fn open(&self, _ctx: &Context) -> Result<()> {
        Ok(()) // Legacy connectors are always "open"
    }

    async fn close(&self, _ctx: &Context) -> Result<()> {
        Ok(())
    }

    async fn health(&self, _ctx: &Context) -> Result<ConnectionHealth> {
        match self.info.health.status {
            ConnectorStatus::Connected => {
                Ok(ConnectionHealth::healthy(
                    &self.info.name,
                    self.info.health.latency_ms as f64,
                ))
            }
            ConnectorStatus::Degraded => {
                let mut h = ConnectionHealth::healthy(
                    &self.info.name,
                    self.info.health.latency_ms as f64,
                );
                h.error = Some("degraded".into());
                Ok(h)
            }
            ConnectorStatus::Disconnected => {
                Ok(ConnectionHealth::unhealthy(
                    &self.info.name,
                    "disconnected",
                ))
            }
        }
    }
}
```

**Data flow**: `ConnectorRegistry` entries --> `ConnectorBridge` --> `Connect` trait
**Purpose**: Migration bridge so callers can switch to trait-based interface incrementally

### Integration with Universal Loop

Connect does NOT participate in the per-tick loop. Instead it manages the infrastructure
that the loop depends on:

```
                 ┌──────────────┐
                 │ ProviderConn │ ── health() ──► ConnectionHealth
                 └──────┬───────┘                      │
                        │                              ▼
                        │ open()              ┌────────────────┐
                        ▼                     │ roko config    │
                 ┌──────────────┐             │ providers      │
                 │ LLM Provider │             │ health         │
                 │ (Anthropic,  │             └────────────────┘
                 │  OpenAI etc) │
                 └──────┬───────┘
                        │
                        ▼
              ┌──────────────────┐
              │ Agent Dispatch   │ ◄── universal loop calls agents
              │ (orchestrate.rs) │     through dispatched backends
              └──────────────────┘
```

### Wire Target

```bash
cargo run -p roko-cli -- config providers health
# Shows live connection status via Connect::health() alongside persisted circuit data
# Example:
#   anthropic: Live: connected (45ms) | Circuit: CLOSED | Calls: 150 | Err: 2%
```

### Test Strategy

1. **Unit test** (`provider_connection.rs`): Test with no credentials (unhealthy), no endpoint (unhealthy)
2. **Unit test** (`connector_bridge.rs`): Bridge Connected/Degraded/Disconnected status correctly
3. **Integration test**: `roko config providers health` shows live Connect data alongside circuit state

---

## Trait 3: Trigger — Complete Implementation Design

### Design Intent

Trigger is the **event-driven activation** protocol. Triggers subscribe to Bus topics
or poll external state, then emit activation Engrams when their condition is satisfied.
They are the reactive complement to Observe (which is pull-based). Triggers bridge
the gap between incoming events (Pulses on the Bus) and actions (plan execution,
agent dispatch, notifications).

### Supporting Types

File: `crates/roko-core/src/traits.rs` (add before the Trigger trait)

```rust
/// Describes what a [`Trigger`] is watching after being armed.
///
/// Returned by `Trigger::arm()` so the caller knows what topics/conditions
/// the trigger will fire on. Used by the subscription system and TUI to
/// display active trigger bindings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TriggerBinding {
    /// The topic filter this trigger matches against.
    pub filter: TopicFilter,
    /// Human-readable description of what this trigger watches.
    pub description: String,
    /// Whether the trigger is currently armed.
    pub armed: bool,
}

impl TriggerBinding {
    /// Create a new armed trigger binding.
    pub fn new(filter: TopicFilter, description: impl Into<String>) -> Self {
        Self {
            filter,
            description: description.into(),
            armed: true,
        }
    }
}
```

### Trait Definition

File: `crates/roko-core/src/traits.rs` (replace lines 417-425)

```rust
// ─── Trigger ─────────────────────────────────────────────────────────────

/// Event-driven activation — watches for conditions and fires when met.
///
/// Triggers subscribe to [`Bus`] topics or poll external state, then emit
/// activation [`Engram`]s when their condition is satisfied. They are the
/// reactive complement to [`Observe`] (which is pull-based).
///
/// # Lifecycle
///
/// ```text
/// arm(ctx) -> TriggerBinding -> [waiting]
///   -> check(pulses, ctx) -> Some(engrams) -> [fired!]
///   -> check(pulses, ctx) -> None          -> [continue waiting]
/// disarm(ctx) -> [inactive]
/// ```
///
/// # Implementations
///
/// - `BusTrigger` — fires when matching Pulses arrive on the Bus
/// - `CronTrigger` — fires on a cron schedule
/// - `FileWatchTrigger` — fires when watched files change
#[async_trait]
pub trait Trigger: Cell {
    /// Arm the trigger. Returns a [`TriggerBinding`] describing what
    /// topics/conditions it will watch for.
    async fn arm(&self, ctx: &Context) -> Result<TriggerBinding>;

    /// Check if the trigger should fire given a batch of pulses.
    /// Returns `Some(engrams)` if the trigger fires (with activation
    /// engrams describing what happened), `None` if it doesn't.
    async fn check(&self, pulses: &[Pulse], ctx: &Context) -> Result<Option<Vec<Engram>>>;

    /// Disarm the trigger, stopping all watches. Idempotent.
    async fn disarm(&self, ctx: &Context) -> Result<()>;

    /// Whether the trigger is currently armed.
    fn is_armed(&self) -> bool;
}
```

### Concrete Implementation 1: BusTrigger

File: `crates/roko-core/src/bus_trigger.rs` (new file)

```rust
//! A Trigger that watches Bus topics and fires when matching Pulses arrive.

use crate::{
    cell::Cell, Body, CellVersion, Context, Engram, Kind, Provenance,
    Pulse, TopicFilter, TriggerBinding,
    traits::Trigger, error::Result,
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};

/// The simplest useful Trigger: watches for Pulses matching a TopicFilter
/// and emits activation Engrams when they arrive.
///
/// Used by the subscription system to map configured `trigger` strings
/// to topic-based activation.
pub struct BusTrigger {
    /// Unique name for this trigger instance.
    name: String,
    /// The topic filter to match against incoming Pulses.
    filter: TopicFilter,
    /// Whether this trigger is currently armed.
    armed: AtomicBool,
}

impl BusTrigger {
    pub fn new(name: impl Into<String>, filter: TopicFilter) -> Self {
        Self {
            name: name.into(),
            filter,
            armed: AtomicBool::new(false),
        }
    }
}

impl Cell for BusTrigger {
    fn cell_id(&self) -> &str { &self.name }
    fn cell_name(&self) -> &str { &self.name }
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &["Trigger"] }
}

#[async_trait]
impl Trigger for BusTrigger {
    async fn arm(&self, _ctx: &Context) -> Result<TriggerBinding> {
        self.armed.store(true, Ordering::SeqCst);
        Ok(TriggerBinding::new(
            self.filter.clone(),
            format!("BusTrigger '{}' watching for matching pulses", self.name),
        ))
    }

    async fn check(&self, pulses: &[Pulse], _ctx: &Context) -> Result<Option<Vec<Engram>>> {
        if !self.armed.load(Ordering::SeqCst) {
            return Ok(None);
        }

        let matching: Vec<&Pulse> = pulses.iter()
            .filter(|p| self.filter.matches(&p.topic))
            .collect();

        if matching.is_empty() {
            return Ok(None);
        }

        let body = serde_json::json!({
            "trigger": self.name,
            "topic": "trigger.fired",
            "matched_count": matching.len(),
            "topics": matching.iter().map(|p| p.topic.to_string()).collect::<Vec<_>>(),
        });

        let engram = Engram::builder(Kind::Custom("trigger.fired".into()))
            .body(Body::Json(body))
            .provenance(Provenance::agent(&self.name))
            .build();

        Ok(Some(vec![engram]))
    }

    async fn disarm(&self, _ctx: &Context) -> Result<()> {
        self.armed.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_armed(&self) -> bool {
        self.armed.load(Ordering::SeqCst)
    }
}
```

**Data flow**: `Pulse` batch --> `TopicFilter::matches()` --> activation `Engram(Kind::Custom("trigger.fired"))`
**Storage**: In-memory `AtomicBool` for armed state
**Config**: Constructed from `SubscriptionConfig.trigger` string via `topic_filter_from_trigger()`

### Concrete Implementation 2: CronTrigger

File: `crates/roko-core/src/cron_trigger.rs` (new file)

```rust
//! A Trigger that fires on a cron schedule.
//!
//! Checks whether the current time falls within a cron window relative
//! to the last check. Does not spawn background threads -- it is polled
//! via `check()` by the subscription dispatch loop.

use crate::{
    cell::Cell, Body, CellVersion, Context, Engram, Kind, Provenance,
    Pulse, TopicFilter, TriggerBinding,
    traits::Trigger, error::Result,
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// A Trigger that fires when the current time matches a cron expression.
///
/// CronTrigger is poll-based: each call to `check()` evaluates the cron
/// expression against `ctx.now_ms` and fires if enough time has elapsed
/// since the last fire. This avoids background threads and integrates
/// with the existing subscription dispatch loop.
pub struct CronTrigger {
    name: String,
    /// Cron expression string (5 or 6 fields).
    schedule: String,
    armed: AtomicBool,
    /// Timestamp of last fire (millis since epoch).
    last_fired_ms: Mutex<Option<i64>>,
    /// Minimum interval between fires (derived from cron).
    min_interval_ms: i64,
}

impl CronTrigger {
    pub fn new(
        name: impl Into<String>,
        schedule: impl Into<String>,
        min_interval_ms: i64,
    ) -> Self {
        Self {
            name: name.into(),
            schedule: schedule.into(),
            armed: AtomicBool::new(false),
            last_fired_ms: Mutex::new(None),
            min_interval_ms,
        }
    }
}

impl Cell for CronTrigger {
    fn cell_id(&self) -> &str { &self.name }
    fn cell_name(&self) -> &str { &self.name }
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &["Trigger"] }
}

#[async_trait]
impl Trigger for CronTrigger {
    async fn arm(&self, _ctx: &Context) -> Result<TriggerBinding> {
        self.armed.store(true, Ordering::SeqCst);
        Ok(TriggerBinding::new(
            TopicFilter::All, // Cron triggers are not topic-based
            format!("CronTrigger '{}' on schedule '{}'", self.name, self.schedule),
        ))
    }

    async fn check(&self, _pulses: &[Pulse], ctx: &Context) -> Result<Option<Vec<Engram>>> {
        if !self.armed.load(Ordering::SeqCst) {
            return Ok(None);
        }

        let now = ctx.now_ms;
        let should_fire = {
            let last = self.last_fired_ms.lock().unwrap();
            match *last {
                None => true, // Never fired; fire on first check
                Some(last_ms) => (now - last_ms) >= self.min_interval_ms,
            }
        };

        if !should_fire {
            return Ok(None);
        }

        // Update last-fired timestamp
        *self.last_fired_ms.lock().unwrap() = Some(now);

        let body = serde_json::json!({
            "trigger": self.name,
            "topic": "trigger.cron.fired",
            "schedule": self.schedule,
            "fired_at_ms": now,
        });

        let engram = Engram::builder(Kind::Custom("trigger.cron.fired".into()))
            .body(Body::Json(body))
            .provenance(Provenance::agent(&self.name))
            .build();

        Ok(Some(vec![engram]))
    }

    async fn disarm(&self, _ctx: &Context) -> Result<()> {
        self.armed.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_armed(&self) -> bool {
        self.armed.load(Ordering::SeqCst)
    }
}
```

**Data flow**: `ctx.now_ms` --> cron schedule evaluation --> activation Engram
**Storage**: In-memory `last_fired_ms` for debouncing
**Config**: `SubscriptionTrigger::Cron { schedule }` from `roko.toml`

### Concrete Implementation 3: FileWatchTrigger

File: `crates/roko-core/src/file_watch_trigger.rs` (new file)

```rust
//! A Trigger that fires when watched files change.
//!
//! Wraps the existing `notify::RecommendedWatcher` pattern used in
//! `tui/fs_watch.rs`. Emits activation Engrams with the changed paths.

use crate::{
    cell::Cell, Body, CellVersion, Context, Engram, Kind, Provenance,
    Pulse, TopicFilter, TriggerBinding,
    traits::Trigger, error::Result,
};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// A Trigger that fires when files matching configured patterns change.
///
/// Integrates with the existing file-watcher infrastructure. Does not
/// spawn its own watcher thread -- instead it receives change notifications
/// via an injected channel and reports them on `check()`.
pub struct FileWatchTrigger {
    name: String,
    paths: Vec<PathBuf>,
    extensions: Vec<String>,
    armed: AtomicBool,
    /// Pending changed paths accumulated since last check.
    pending_changes: Mutex<Vec<PathBuf>>,
}

impl FileWatchTrigger {
    pub fn new(
        name: impl Into<String>,
        paths: Vec<PathBuf>,
        extensions: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            paths,
            extensions,
            armed: AtomicBool::new(false),
            pending_changes: Mutex::new(Vec::new()),
        }
    }

    /// Record a file change (called by the watcher thread).
    pub fn notify_change(&self, path: PathBuf) {
        if !self.armed.load(Ordering::SeqCst) {
            return;
        }

        // Filter by extension if configured
        if !self.extensions.is_empty() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !self.extensions.iter().any(|e| e == ext) {
                return;
            }
        }

        self.pending_changes.lock().unwrap().push(path);
    }
}

impl Cell for FileWatchTrigger {
    fn cell_id(&self) -> &str { &self.name }
    fn cell_name(&self) -> &str { &self.name }
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &["Trigger"] }
}

#[async_trait]
impl Trigger for FileWatchTrigger {
    async fn arm(&self, _ctx: &Context) -> Result<TriggerBinding> {
        self.armed.store(true, Ordering::SeqCst);
        let desc = format!(
            "FileWatchTrigger '{}' watching {} path(s)",
            self.name, self.paths.len()
        );
        Ok(TriggerBinding::new(
            TopicFilter::Prefix("fs.change.".into()),
            desc,
        ))
    }

    async fn check(&self, _pulses: &[Pulse], _ctx: &Context) -> Result<Option<Vec<Engram>>> {
        if !self.armed.load(Ordering::SeqCst) {
            return Ok(None);
        }

        let changes: Vec<PathBuf> = {
            let mut pending = self.pending_changes.lock().unwrap();
            std::mem::take(&mut *pending)
        };

        if changes.is_empty() {
            return Ok(None);
        }

        let body = serde_json::json!({
            "trigger": self.name,
            "topic": "trigger.fs.changed",
            "changed_count": changes.len(),
            "paths": changes.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        });

        let engram = Engram::builder(Kind::Custom("trigger.fs.changed".into()))
            .body(Body::Json(body))
            .provenance(Provenance::agent(&self.name))
            .build();

        Ok(Some(vec![engram]))
    }

    async fn disarm(&self, _ctx: &Context) -> Result<()> {
        self.armed.store(false, Ordering::SeqCst);
        self.pending_changes.lock().unwrap().clear();
        Ok(())
    }

    fn is_armed(&self) -> bool {
        self.armed.load(Ordering::SeqCst)
    }
}
```

**Data flow**: `notify` events --> `pending_changes` buffer --> `check()` --> activation Engram
**Storage**: In-memory pending buffer (drained on each check)
**Config**: `SubscriptionTrigger::FileWatch { paths, extensions }` from `roko.toml`

### Config-to-Trigger Mapping

File: `crates/roko-core/src/config/subscriptions.rs` (add helper functions)

```rust
use crate::{TopicFilter, Topic};

/// Convert a trigger string from SubscriptionConfig into a TopicFilter.
///
/// Rules:
/// - Empty or "*" → TopicFilter::All
/// - Ends with "*" → TopicFilter::Prefix(prefix)
/// - Otherwise → TopicFilter::Exact(topic)
pub fn topic_filter_from_trigger(trigger: &str) -> TopicFilter {
    let trigger = trigger.trim();
    if trigger.is_empty() || trigger == "*" {
        TopicFilter::All
    } else if let Some(prefix) = trigger.strip_suffix('*') {
        TopicFilter::Prefix(prefix.to_string())
    } else {
        TopicFilter::Exact(Topic::new(trigger))
    }
}

impl SubscriptionConfig {
    /// Derive a TopicFilter from this subscription's trigger string.
    pub fn trigger_topic_filter(&self) -> TopicFilter {
        topic_filter_from_trigger(&self.trigger)
    }
}
```

### Integration with Universal Loop

Triggers do NOT participate in the per-tick loop. They sit in the event dispatch layer
between the Bus and the orchestrator:

```
Bus ──publish()──► Pulse stream
                      │
                      ▼
              ┌──────────────┐
              │ BusTrigger   │ ── check(pulses) ──► Some(activation engrams)
              │ CronTrigger  │                              │
              │ FileWatch    │                              ▼
              └──────────────┘                     ┌──────────────────┐
                                                   │ Subscription     │
                                                   │ Dispatch Loop    │
                                                   │ (roko-serve)     │
                                                   └────────┬─────────┘
                                                            │
                                                            ▼
                                                   ┌──────────────────┐
                                                   │ Agent Dispatch   │
                                                   │ (orchestrate.rs) │
                                                   └──────────────────┘
```

### Wire Target

```bash
cargo run -p roko-cli -- config subscriptions list
# Shows subscription entries with trigger binding information
#   prd-auto-plan:
#     filter: prd.published
#     trigger: BusTrigger 'prd-auto-plan' watching for matching pulses

cargo test -p roko-core -- bus_trigger
# BusTrigger lifecycle test: arm → check(non-matching) → check(matching) → disarm
```

### Test Strategy

1. **Unit test** (`bus_trigger.rs`): Arm, check with non-matching pulse (None), check with
   matching pulse (Some), disarm, check again (None)
2. **Unit test** (`cron_trigger.rs`): Arm, check before interval (None), check after
   interval (Some), verify last-fired updates
3. **Unit test** (`file_watch_trigger.rs`): Arm, notify changes, check (Some with paths),
   check again (None, buffer drained), disarm
4. **Unit test** (`subscriptions.rs`): `topic_filter_from_trigger` maps "*" to All,
   "gate.*" to Prefix, "prd.published" to Exact

---

## Registration and Discovery

The three traits do NOT need a formal registry in Phase 1. The task specs explicitly
say "Do NOT create an ObserverRegistry" and "Do NOT build a TriggerRegistry or
TriggerScheduler." The Graph engine (Phase 2) will manage cell discovery.

For Phase 1, discovery is manual and direct:

```rust
// In cmd_status (roko-cli/src/commands/util.rs):
let observer = StoreObserver::new(Arc::clone(&store));
let observations = observer.observe(&ctx).await?;

// In cmd_provider_health (roko-cli/src/commands/config_cmd.rs):
let connections = ProviderConnection::from_config(&config);
for conn in &connections {
    let health = conn.health(&ctx).await?;
}

// In cmd_subscriptions_list (roko-cli/src/subscriptions.rs):
let filter = topic_filter_from_trigger(&sub.trigger);
let trigger = BusTrigger::new(&sub.id, filter);
let binding = trigger.arm(&ctx).await?;
```

Phase 2 (Graph engine) will add:

```rust
// Graph discovers cells by protocol tag:
let observers: Vec<Arc<dyn Observe>> = graph.cells_by_protocol("Observe");
let triggers: Vec<Arc<dyn Trigger>> = graph.cells_by_protocol("Trigger");
```

This works because every implementation already returns `protocols()` from its `Cell`
impl (e.g., `&["Observe"]`, `&["Connect"]`, `&["Trigger"]`).

---

## Complete Trait Relationship Diagram

```
                              ┌──────────────┐
                              │    Cell       │ ← Supertrait for ALL
                              │  (identity,   │   protocol traits
                              │   cost,       │
                              │   protocols)  │
                              └──────┬───────┘
                                     │
        ┌────────────────────────────┼────────────────────────────┐
        │                            │                            │
   ┌────┴─────┐                ┌─────┴─────┐              ┌──────┴──────┐
   │ LOOP     │                │ LIFECYCLE │              │ TRANSPORT   │
   │ VERBS    │                │ PROTOCOLS │              │             │
   └────┬─────┘                └─────┬─────┘              └──────┬──────┘
        │                            │                           │
  ┌─────┼─────┐              ┌───────┼───────┐                   │
  │     │     │              │       │       │                   │
  ▼     ▼     ▼              ▼       ▼       ▼                   ▼
┌────┐┌────┐┌─────┐    ┌───────┐┌───────┐┌───────┐         ┌───────┐
│Stor││Scor││Route│    │Observe││Connect││Trigger│         │  Bus  │
│e   ││e   ││     │    │       ││       ││       │         │       │
└─┬──┘└─┬──┘└──┬──┘    └───┬───┘└───┬───┘└───┬───┘         └───┬───┘
  │      │      │           │        │        │                 │
  ▼      ▼      ▼           │        │        │                 │
┌─────┐┌─────┐┌─────┐      │        │        │                 │
│Compo││Verif││React│      │        │        │                 │
│se   ││y    ││     │      │        │        │                 │
└──┬──┘└──┬──┘└──┬──┘      │        │        │                 │
   │      │      │          │        │        │                 │
   └──────┼──────┘          │        │        │                 │
          │                 │        │        │                 │
          ▼                 ▼        ▼        ▼                 ▼
   ┌──────────────────────────────────────────────────────────────────┐
   │                         Engram                                   │
   │  The universal datum: content-hashed, decaying, scored, traced   │
   │  - Engrams flow through loop_tick (Store→Score→Route→Compose     │
   │    →Verify→React)                                                │
   │  - Observations produce Engrams (Observe)                        │
   │  - Trigger activations produce Engrams (Trigger)                 │
   │  - Connection requests exchange Engrams (Connect)                │
   └──────────────────────────────────────────────────────────────────┘
          ▲                                                    ▲
          │                                                    │
   ┌──────┴──────┐                                      ┌──────┴──────┐
   │   Pulse     │ ─── ephemeral transport ────────────► │  ColdStore  │
   │ (Bus event) │                                       │  (archive)  │
   └─────────────┘                                       └─────────────┘
```

Data flow through the complete architecture:

```
External World
     │
     ▼
 ┌───────┐     ┌───────┐     ┌───────┐
 │Observe│     │Connect│     │Trigger│      ← Sensing/Lifecycle/Eventing
 │(pull) │     │(bidir)│     │(push) │
 └───┬───┘     └───┬───┘     └───┬───┘
     │             │             │
     ▼             ▼             ▼
 ┌─────────────────────────────────────┐
 │              Engrams                │    ← Universal data format
 └──────────────────┬──────────────────┘
                    │
                    ▼
 ┌─────────────────────────────────────┐
 │          loop_tick()                │    ← Universal loop
 │  Store → Score → Route → Compose   │
 │            → Verify → React        │
 └──────────────────┬──────────────────┘
                    │
                    ▼
 ┌─────────────────────────────────────┐
 │       Bus (Pulses) + Store          │    ← Persistence + Transport
 └─────────────────────────────────────┘
```

---

## Implementation Order

The three traits have dependencies that determine implementation order:

```
Task 039: Observe         Task 040: Connect         Task 041: Trigger
  │ (no deps)               │ (no deps)               │ (no deps)
  ▼                         ▼                         ▼
  StoreObserver             ProviderConnection        BusTrigger
  │                         │                         │
  ▼                         ▼                         ▼
  Wire: roko status         Wire: providers health    Wire: subscriptions list
  │                         │                         │
  └─────────────┬───────────┘                         │
                │                                     │
                ▼                                     │
          Task 042: Phase 1 Integration Test ◄────────┘
                │
                ▼
          Task 097: Feed trait (depends on Observe async)
```

**Recommended order: 039 → 040 → 041 → 042**

- Tasks 039-041 are independent of each other (can be parallelized)
- Task 042 (integration test) depends on all three
- Task 097 (Feed trait) depends on 039 (Observe) being async
- The wave-3 graph engine and wave-4 feed system are partially blocked

All three tasks share the same pattern:
1. Replace sync stub with async trait definition in `traits.rs`
2. Add supporting types (ConnectionHealth, TriggerBinding)
3. Create first concrete implementation in new file
4. Wire into an existing CLI command
5. Export from `lib.rs`
6. Add unit tests

---

## How These Traits Enable Self-Hosting

Roko's self-hosting loop is:

```
read PRDs → generate plans → execute tasks → validate with gates → learn → iterate
```

The three dead traits fill critical gaps in this loop:

### Observe enables state awareness

Without Observe, `roko status` queries the store directly. With Observe:
- The system can monitor its own health during plan execution
- AgentObserver reports which agents are running/stuck/crashed
- SystemObserver reports disk usage approaching limits
- StoreObserver tracks signal accumulation rate

This is essential for the "develop itself" goal: roko needs to observe its own
state to decide what to work on next.

### Connect enables reliable external I/O

Without Connect, provider health is checked via ad-hoc code in `connector.rs`.
With Connect:
- `roko config providers health` reports live reachability
- Agent dispatch can check connection health before attempting LLM calls
- MCP server lifecycle is managed through a standard interface
- The circuit breaker (roko-learn) can feed `ConnectionHealth` into its state machine

This is essential for reliability: roko needs to know which providers are available
before routing agent tasks to them.

### Trigger enables reactive execution

Without Trigger, event-driven behavior uses ad-hoc patterns (e.g.,
`prd_publish_subscriber` in roko-serve). With Trigger:
- `SubscriptionConfig.trigger` strings map to `BusTrigger` instances
- Cron schedules fire `CronTrigger` for periodic tasks (dream consolidation, GC)
- File changes fire `FileWatchTrigger` for reactive re-execution
- The subscription dispatch loop uses `Trigger::check()` uniformly

This is essential for autonomy: roko needs to react to events (PRD published,
tests failed, file changed) without human intervention.

### Together: the complete autonomous loop

```
 Observe                     Connect                      Trigger
 "What is my state?"         "Can I reach my tools?"      "Something happened!"
       │                          │                             │
       ▼                          ▼                             ▼
 ┌──────────────────────────────────────────────────────────────────────┐
 │                        Orchestrate.rs                                │
 │  1. Observe: check store stats, agent health, disk usage             │
 │  2. Connect: verify LLM providers reachable before dispatch          │
 │  3. Plan: read PRDs → generate tasks → build DAG                     │
 │  4. Execute: dispatch agents via universal loop                      │
 │  5. Verify: run gates (compile, test, clippy)                        │
 │  6. React: log episodes, update playbooks, tune router               │
 │  7. Trigger: fire on gate failure → replan; fire on PRD publish →    │
 │     auto-plan; fire on cron → dream consolidation                    │
 └──────────────────────────────────────────────────────────────────────┘
```

With all nine traits wired, roko can:
1. **Sense** its environment (Observe) to know what state it's in
2. **Connect** to external tools (Connect) to verify they're available
3. **React** to events (Trigger) to start work without human prompting
4. **Execute** plans through the universal loop (Store/Score/Route/Compose/Verify/React)
5. **Learn** from outcomes and iterate

This is the difference between "a tool that runs when you type a command" and
"a system that develops itself."

---

## Dependency Impact

| Blocked task | What it needs | From which trait |
|--------------|---------------|------------------|
| 097 (Feed trait) | `Observe` async redesign | Task 039 |
| 042 (Phase 1 integration test) | All three traits wired | Tasks 039-041 |
| Wave-3 graph engine | Cell protocol discovery for Observe/Connect/Trigger | Tasks 039-041 |
| Wave-4 feed system | Observe for data streams | Task 039 |
| Cold substrate archival | CronTrigger for periodic GC | Task 041 |
| Dream consolidation runtime trigger | CronTrigger | Task 041 |

---

## Recommendation

**Implement all three traits** as specified. They are not optional cleanup -- they are
load-bearing parts of the architecture that enable self-hosting. The "either implement
or delete" framing from the original document is correct, but deletion is not viable:
downstream tasks (042, 097, graph engine, feed system) depend on these traits existing
and being async.

The implementation cost is moderate (estimated 510 minutes / ~8.5 hours for all three
tasks) and the patterns are well-established: every concrete implementation follows the
same Cell + async_trait + Engram builder pattern used by the 30+ existing Verify and
React implementations.

Keeping them as synchronous stubs with zero callers is the worst option. It signals
architectural intent without delivering capability, blocks five downstream tasks, and
misleads anyone reading the exports of `roko-core`.
