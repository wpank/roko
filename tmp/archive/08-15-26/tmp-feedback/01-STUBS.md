# STUB Tasks -- Complete Implementation Designs

These tasks are marked `status = "implemented"` in STATUS.toml but have no meaningful
code changes attributable to them. This document provides complete implementation
designs for each one, detailed enough for an agent to implement without questions.

## Dependency Order

```
037 (Signal rename core)     -- no dependencies, do FIRST
 |
 v
039 (Observe trait)          -- blocked_by: [] (but use Signal if 037 done)
040 (Connect trait)          -- blocked_by: []
041 (Trigger trait)           -- blocked_by: []
048 (CI pipeline)            -- blocked_by: [] (independent infra task)
 |
 v (039, 040, 041 must land before 097)
097 (Feed trait)             -- blocked_by: [35, 39, 40, 41]
 |
 v (031, 099 must land before 100)
100 (Predict-Publish-Correct) -- blocked_by: [31, 99]
```

Recommended execution order: `037 -> 048 -> 039 -> 040 -> 041 -> 097 -> 100`

Task 048 is independent infrastructure and can run in parallel with 037.
Tasks 039, 040, 041 can run in parallel after 037.
Task 097 requires 039+040+041.
Task 100 has external blockers (031+099) beyond this batch.

---

## Task 037: Signal Rename Core (Engram -> Signal)

**Spec file**: `tmp/taskrunner/tasks/037-signal-rename-core.md`
**Estimated time**: 180 minutes

### Current State of Stub Code

| File | Line(s) | Current State |
|---|---|---|
| `crates/roko-core/src/engram.rs:63` | `pub struct Engram` | Still the canonical struct name |
| `crates/roko-core/src/engram.rs:100-105` | `impl Engram` | Methods still on `Engram` |
| `crates/roko-core/src/signal.rs:6` | `pub use crate::engram::{Engram as Signal, EngramBuilder as SignalBuilder, HdcFingerprint}` | Alias goes wrong direction (Engram->Signal instead of Signal->Engram) |
| `crates/roko-core/src/datum.rs:37` | `Datum::Engram(&'a Engram)` | Variant still named `Engram` |
| `crates/roko-core/src/lib.rs:206` | `pub use engram::{Engram, EngramBuilder, HdcFingerprint}` | Exports old names |
| `crates/roko-core/src/traits.rs` | 28 occurrences of `Engram` | All trait sigs use old name |

**Total `Engram`/`EngramBuilder` occurrences in roko-core/src**: 232 across 22 files.

### What Was Required vs What Exists

| Requirement | Delivered |
|---|---|
| Rename `pub struct Engram` -> `pub struct Signal` | Not done |
| Rename `pub struct EngramBuilder` -> `pub struct SignalBuilder` | Not done |
| Add `#[deprecated] pub type Engram = Signal` alias | Not done |
| Flip `signal.rs` to re-export canonical `Signal` | Not done |
| Update `Datum::Engram` -> `Datum::Signal` | Not done |
| Update all 222 internal references | Not done |

### Implementation Design

#### Data Structures

No new data structures. This is a mechanical rename of existing types.

#### Step-by-Step

**Step 1**: Rename the struct in `crates/roko-core/src/engram.rs`

```rust
// Line 63: pub struct Engram -> pub struct Signal
pub struct Signal { ... }

// Line 100: impl Engram -> impl Signal
impl Signal { ... }

// Line ~170: pub fn builder(kind: Kind) -> EngramBuilder
// becomes:   pub fn builder(kind: Kind) -> SignalBuilder

// EngramBuilder struct (~line 230+): rename to SignalBuilder
pub struct SignalBuilder { ... }
impl SignalBuilder { ... }
```

All methods (`content_hash`, `weight_at`, `age_ms`, `touch`, `tag`, `is`,
`derive`, `derive_verdict`, `from_pulse_synthetic`, `from_pulses`, `bind`,
`bundle`, `builder`) keep their names -- they are methods on the struct, not
named after it.

**Step 2**: Add deprecated aliases at the bottom of `engram.rs`

```rust
/// Deprecated alias for [`Signal`]. Use `Signal` in new code.
#[deprecated(since = "0.2.0", note = "Use Signal instead of Engram")]
pub type Engram = Signal;

/// Deprecated alias for [`SignalBuilder`]. Use `SignalBuilder` in new code.
#[deprecated(since = "0.2.0", note = "Use SignalBuilder instead of EngramBuilder")]
pub type EngramBuilder = SignalBuilder;
```

**Step 3**: Replace `crates/roko-core/src/signal.rs` contents

```rust
//! Signal -- the universal datum of the Roko system.
//!
//! Re-exports the canonical type from `engram.rs`. The old `Engram`
//! spelling remains available through deprecated aliases for downstream
//! compatibility.

pub use crate::engram::{HdcFingerprint, Signal, SignalBuilder};
```

**Step 4**: Update `crates/roko-core/src/lib.rs` exports (line 206)

```rust
pub use engram::{Signal, SignalBuilder, HdcFingerprint};
#[allow(deprecated)]
pub use engram::{Engram, EngramBuilder};
```

**Step 5**: Update `crates/roko-core/src/datum.rs`

- Line 37: `Datum::Engram(&'a Engram)` -> `Datum::Signal(&'a Signal)`
- Line 8: `use crate::{Body, Engram, Kind, Pulse}` -> `use crate::{Body, Signal, Kind, Pulse}`
- All `Datum::Engram(...)` match arms -> `Datum::Signal(...)`
- `is_engram()` method -> add `is_signal()` and deprecated forwarding `is_engram()`
- `From<&Engram>` -> `From<&Signal>`

**Step 6**: Mechanically update all remaining roko-core files

Use this command to find every reference:
```bash
rg -n '\bEngram\b|\bEngramBuilder\b|Datum::Engram' \
  crates/roko-core/src crates/roko-core/tests crates/roko-core/benches \
  --glob '*.rs'
```

Files with most references (in update order):
1. `traits.rs` (~28): `Store::put(Engram)`, `Score::score(&Engram)`, etc.
2. `datum.rs` (~21): variant + match arms
3. `loop_tick.rs` (~18): loop step types
4. `prediction.rs` (~17): prediction types
5. `attestation.rs` (~16): attestation constructors
6. `pulse.rs` (~14): promotion/graduation
7. `catalyst.rs` (~6), `cfactor.rs` (~7), `forensic.rs` (~6)
8. `body.rs` (~2), `kind.rs` (~1), `immune.rs` (~5), `affect.rs` (~1)
9. `chat_types.rs` (~4), `cell.rs` (~3), `error/mod.rs` (~1)
10. `tool/handler.rs` (~5), `config/subscriptions.rs` (~1), `config/serve.rs` (~1)
11. `tests/property_tests.rs`, `benches/engram_bench.rs`

**Step 7**: Suppress deprecation warnings only on the alias definitions

Add `#[allow(deprecated)]` on the alias definitions themselves and on the
lib.rs re-export line. Do NOT add crate-level `#[allow(deprecated)]`.

#### Integration Points

- **Callers**: Every crate in the workspace imports `Signal` via the alias.
  After this task, downstream crates continue to compile because the
  deprecated `Engram` alias remains. Task 038 propagates the rename.
- **Serialization**: Struct field names are unchanged (`id`, `kind`, `body`,
  etc.). The struct name does not appear in serde output. Existing
  `.roko/signals.jsonl` files continue to parse.

#### Error Handling

No new error types. This is a rename. Any compilation error is a missed
reference -- find with `rg '\bEngram\b' crates/roko-core/src --glob '*.rs'`.

#### Test Strategy

```bash
# Unit tests
cargo test -p roko-core

# Serde roundtrip (add if not present)
let json = serde_json::to_string(&Signal::builder(Kind::Task).build()).unwrap();
let parsed: Signal = serde_json::from_str(&json).unwrap();

# Bench compilation check
cargo bench -p roko-core --bench engram_bench --no-run

# Full workspace (downstream uses deprecated alias)
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

#### Cross-References

- Follow the pattern from `crates/roko-core/src/connector.rs` which has a
  migration note saying "prefer Connect trait once available" -- similar
  transitional pattern.
- `signal.rs` currently follows the alias pattern at line 6 -- flip direction.

---

## Task 039: Observe Trait Wiring

**Spec file**: `tmp/taskrunner/tasks/039-observe-trait-wiring.md`
**Estimated time**: 150 minutes
**Depends on**: None (but uses Signal if 037 is done first)

### Current State of Stub Code

| File | Line(s) | Current State |
|---|---|---|
| `crates/roko-core/src/traits.rs:399-403` | `pub trait Observe` | Sync stub: `fn observe(&self) -> Vec<Engram>`, no ctx, no `#[async_trait]` |
| `crates/roko-core/src/store_observer.rs` | Does not exist | File never created |
| `crates/roko-cli/src/commands/util.rs:474` | `signal_count: Some(all.len())` | Count sourced from raw query, not through Observe |

Zero implementations of Observe exist anywhere in the workspace. Zero callers.

### What Was Required vs What Exists

| Requirement | Delivered |
|---|---|
| Redesign Observe to async with `ctx: &Context` parameter | Not done |
| Add `observer_name()` and `observation_topic()` defaults | Not done |
| Create `StoreObserver` in `store_observer.rs` | Not done |
| Wire into `cmd_status` in `commands/util.rs` | Not done |
| Export from `lib.rs` | Not done |
| Unit tests | Not done |

### Implementation Design

#### Data Structures

No new data structures beyond the trait methods. `StoreObserver` is a
simple wrapper around `Arc<dyn Store>`.

```rust
// crates/roko-core/src/store_observer.rs
pub struct StoreObserver {
    store: Arc<dyn Store>,
}
```

#### Function Signatures

**Observe trait** (replace in `crates/roko-core/src/traits.rs`, lines 397-403):

```rust
// ---- Observe ----

/// Observation protocol -- passive data collection from external sources.
///
/// Unlike Connect (which manages bidirectional I/O), Observe is read-only:
/// it reads state and emits observation Signals without side effects.
#[async_trait]
pub trait Observe: crate::cell::Cell {
    /// Collect observations from the environment.
    /// Returns zero or more observation signals.
    async fn observe(&self, ctx: &Context) -> Result<Vec<Engram>>;

    /// Human-readable name for this observer.
    fn observer_name(&self) -> &str {
        self.cell_name()
    }

    /// Topic filter for the observations this observer produces.
    fn observation_topic(&self) -> Option<&str> {
        None
    }
}
```

Note: Use `Engram` (or `Signal` if task 037 has landed). The trait already
has `async_trait` imported at line 15 of traits.rs.

**StoreObserver** (new file `crates/roko-core/src/store_observer.rs`):

```rust
use crate::cell::Cell;
use crate::context::Context;
use crate::engram::Engram;    // or Signal if 037 landed
use crate::traits::{Observe, Store};
use crate::{Body, Kind, Provenance};
use async_trait::async_trait;
use std::sync::Arc;

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
    fn protocols(&self) -> &[&str] { &["Observe"] }
}

#[async_trait]
impl Observe for StoreObserver {
    async fn observe(&self, _ctx: &Context) -> crate::error::Result<Vec<Engram>> {
        let count = self.store.len().await?;
        let store_name = self.store.name();

        let body = serde_json::json!({
            "observer": "store-observer",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Store;
    use crate::{ContentHash, Query};
    // Define a minimal test store inline (roko-core must not depend on roko-std)

    struct TestStore { items: Vec<Engram> }
    #[async_trait]
    impl Store for TestStore {
        async fn put(&self, _e: Engram) -> crate::error::Result<ContentHash> { todo!() }
        async fn get(&self, _id: &ContentHash) -> crate::error::Result<Option<Engram>> { Ok(None) }
        async fn query(&self, _q: &Query, _c: &Context) -> crate::error::Result<Vec<Engram>> { Ok(vec![]) }
        async fn len(&self) -> crate::error::Result<usize> { Ok(self.items.len()) }
        fn name(&self) -> &'static str { "test-store" }
    }

    #[tokio::test]
    async fn store_observer_returns_count() {
        let items = (0..5).map(|_| Engram::builder(Kind::Task).build()).collect();
        let store = Arc::new(TestStore { items }) as Arc<dyn Store>;
        let observer = StoreObserver::new(store);
        let ctx = Context::now();
        let obs = observer.observe(&ctx).await.unwrap();
        assert_eq!(obs.len(), 1);
        let body_str = serde_json::to_string(&obs[0].body).unwrap();
        assert!(body_str.contains("\"signal_count\":5"));
    }
}
```

#### Integration Points

**Wire into `cmd_status`** in `crates/roko-cli/src/commands/util.rs`:

The function `cmd_status` (line 357) currently does:
```rust
let substrate = FileSubstrate::open(workdir.join(".roko")).await...;
let all = substrate.query(&Query::all(), &ctx).await...;
// Line 474: signal_count: Some(all.len()),
```

Change to wrap the substrate in an Arc and use StoreObserver:
```rust
use roko_core::store_observer::StoreObserver;
use roko_core::traits::Observe;

// After opening the substrate (line 378):
let substrate = FileSubstrate::open(workdir.join(".roko")).await...;
let store: Arc<dyn roko_core::traits::Store> = Arc::new(substrate);

// Replace the direct query with observer call:
let observer = StoreObserver::new(Arc::clone(&store));
let observations = observer.observe(&ctx).await
    .map_err(|e| anyhow!("observe: {e}"))?;
let signal_count = observations.first()
    .and_then(|obs| {
        if let Body::Json(ref v) = obs.body {
            v.get("signal_count").and_then(|n| n.as_u64()).map(|n| n as usize)
        } else {
            None
        }
    });

// Then for the query (still needed for kind breakdown):
let all = store.query(&Query::all(), &ctx).await...;
```

Line 474 becomes: `signal_count: signal_count.or(Some(all.len())),`

This preserves the existing output format and adds the Observe indirection.

**Export from lib.rs** -- add to `crates/roko-core/src/lib.rs`:
```rust
pub mod store_observer;
pub use store_observer::StoreObserver;
```

#### Error Handling

- `observe()` returns `crate::error::Result<Vec<Engram>>` -- uses existing
  `RokoError` variants.
- `Store::len()` already returns `Result<usize>` with `Ok(0)` default --
  propagate via `?`.
- In `cmd_status`, fall back to `all.len()` if observer fails.

#### Test Strategy

```bash
cargo test -p roko-core -- store_observer    # Unit test
cargo test --workspace                        # No regressions
cargo run -p roko-cli -- status              # Wire verification
```

#### Cross-References

- `Cell` trait at `crates/roko-core/src/cell.rs:1-60` -- `StoreObserver`
  implements `Cell` with `cell_id`, `cell_name`, `protocols`.
- `Store::len()` at `crates/roko-core/src/traits.rs:67` -- default returns
  `Ok(0)`, real impls return actual count.
- `Engram::builder(Kind::Metric)` at `crates/roko-core/src/engram.rs:103` --
  confirmed builder API.
- `Provenance::agent("...")` at `crates/roko-core/src/provenance.rs:309` --
  confirmed constructor.

---

## Task 040: Connect Trait Wiring

**Spec file**: `tmp/taskrunner/tasks/040-connect-trait-wiring.md`
**Estimated time**: 180 minutes
**Depends on**: None (parallel with 039, 041)

### Current State of Stub Code

| File | Line(s) | Current State |
|---|---|---|
| `crates/roko-core/src/traits.rs:405-415` | `pub trait Connect` | Sync stub: `fn connect()`, `fn health() -> bool`, `fn disconnect()` |
| `crates/roko-core/src/connector.rs:1-60` | `ConnectorKind`, `ConnectorStatus`, `ConnectorConfig` | Existing registry types -- must NOT duplicate |
| `crates/roko-cli/src/provider_connection.rs` | Does not exist | File never created |
| `crates/roko-cli/src/commands/config_cmd.rs:127` | `cmd_provider_health(&wd)?` | Still synchronous call |
| `crates/roko-cli/src/commands/config_cmd.rs:404` | `pub(crate) fn cmd_provider_health` | Sync function, reads from files only |

Zero implementations of Connect exist anywhere. Zero callers.

### What Was Required vs What Exists

| Requirement | Delivered |
|---|---|
| Create `ConnectionHealth` struct | Not done |
| Redesign `Connect` to async with `open/close/health/request` | Not done |
| Create `ProviderConnection` in roko-cli | Not done |
| Wire into `cmd_provider_health` (make async) | Not done |
| Export `ConnectionHealth` from lib.rs | Not done |

### Implementation Design

#### Data Structures

**ConnectionHealth** (add to `crates/roko-core/src/traits.rs`, before the Connect trait):

```rust
/// Health status of a connection managed by the Connect trait.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectionHealth {
    /// Whether the connection is currently established.
    pub connected: bool,
    /// Round-trip latency in milliseconds (if measured).
    pub latency_ms: Option<f64>,
    /// Human-readable error message (if unhealthy).
    pub error: Option<String>,
    /// Name of the connection target.
    pub target: String,
}

impl ConnectionHealth {
    pub fn healthy(target: impl Into<String>, latency_ms: f64) -> Self {
        Self {
            connected: true,
            latency_ms: Some(latency_ms),
            error: None,
            target: target.into(),
        }
    }

    pub fn unhealthy(target: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            connected: false,
            latency_ms: None,
            error: Some(error.into()),
            target: target.into(),
        }
    }
}
```

Note: This is distinct from `ConnectorStatus` in `connector.rs` (which is an
enum `Connected/Disconnected/Degraded` for the registry). `ConnectionHealth`
carries latency and error details for live probes.

#### Function Signatures

**Connect trait** (replace in `crates/roko-core/src/traits.rs`, lines 405-415):

```rust
// ---- Connect ----

/// Connectivity protocol -- manage connections to external systems.
///
/// Provides open/close/health lifecycle for durable connections
/// (LLM APIs, databases, WebSocket streams, MCP servers).
#[async_trait]
pub trait Connect: crate::cell::Cell {
    /// Open/establish the connection.
    async fn open(&self, ctx: &Context) -> Result<()>;

    /// Close/tear down the connection.
    async fn close(&self, ctx: &Context) -> Result<()>;

    /// Check connection health. Must be cheap (no heavy I/O).
    async fn health(&self, ctx: &Context) -> Result<ConnectionHealth>;

    /// Send a request through the connection and receive a response.
    async fn request(&self, input: Engram, ctx: &Context) -> Result<Engram>;
}
```

**ProviderConnection** (new file `crates/roko-cli/src/provider_connection.rs`):

```rust
use roko_core::cell::Cell;
use roko_core::context::Context;
use roko_core::traits::{Connect, ConnectionHealth};
use roko_core::{Engram, RokoError};
use async_trait::async_trait;

pub struct ProviderConnection {
    provider_name: String,
    endpoint: String,
    api_key_env: Option<String>,
    credential_available: bool,
}

impl ProviderConnection {
    pub fn new(
        provider_name: String,
        endpoint: String,
        api_key_env: Option<String>,
        credential_available: bool,
    ) -> Self { ... }

    /// Create connections from the config's provider list.
    pub fn from_config(config: &roko_core::config::RokoConfig) -> Vec<Self> {
        let providers = config.effective_providers();
        providers.iter().map(|(id, p)| {
            let endpoint = p.base_url.clone().unwrap_or_else(|| default_endpoint(id));
            let available = config.is_provider_available(p);
            ProviderConnection::new(
                id.clone(),
                endpoint,
                p.api_key_env.clone(),
                available,
            )
        }).collect()
    }
}

impl Cell for ProviderConnection {
    fn cell_id(&self) -> &str { &self.provider_name }
    fn cell_name(&self) -> &str { &self.provider_name }
    fn protocols(&self) -> &[&str] { &["Connect"] }
}

#[async_trait]
impl Connect for ProviderConnection {
    async fn open(&self, _ctx: &Context) -> roko_core::error::Result<()> { Ok(()) }
    async fn close(&self, _ctx: &Context) -> roko_core::error::Result<()> { Ok(()) }

    async fn health(&self, _ctx: &Context) -> roko_core::error::Result<ConnectionHealth> {
        if !self.credential_available {
            return Ok(ConnectionHealth::unhealthy(
                &self.provider_name,
                format!("API key env {} not set",
                    self.api_key_env.as_deref().unwrap_or("?")),
            ));
        }
        let start = std::time::Instant::now();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| RokoError::Transport(e.to_string()))?;

        match client.head(&self.endpoint).send().await {
            Ok(resp) => {
                let latency = start.elapsed().as_secs_f64() * 1000.0;
                let status = resp.status().as_u16();
                if resp.status().is_success() || status == 401 || status == 403 {
                    Ok(ConnectionHealth::healthy(&self.provider_name, latency))
                } else {
                    Ok(ConnectionHealth::unhealthy(
                        &self.provider_name,
                        format!("HTTP {}", resp.status()),
                    ))
                }
            }
            Err(e) => Ok(ConnectionHealth::unhealthy(
                &self.provider_name, e.to_string()
            )),
        }
    }

    async fn request(&self, _input: Engram, _ctx: &Context)
        -> roko_core::error::Result<Engram>
    {
        Err(RokoError::Invalid(
            "ProviderConnection::request() not implemented -- use agent dispatch".into(),
        ))
    }
}

fn default_endpoint(provider_id: &str) -> String {
    match provider_id {
        "anthropic" => "https://api.anthropic.com".into(),
        "openai" | "openai-compatible" => "https://api.openai.com/v1".into(),
        "gemini" => "https://generativelanguage.googleapis.com".into(),
        "perplexity" => "https://api.perplexity.ai".into(),
        "cerebras" => "https://api.cerebras.ai/v1".into(),
        _ => format!("https://{provider_id}.example.com"),
    }
}
```

#### Integration Points

**Wire into `cmd_provider_health`** in `crates/roko-cli/src/commands/config_cmd.rs`:

1. Line 404: change `pub(crate) fn cmd_provider_health` to `pub(crate) async fn cmd_provider_health`
2. Line 127: change `cmd_provider_health(&wd)?` to `cmd_provider_health(&wd).await?`
3. After the existing credential coverage output (line 421), add live health:

```rust
use crate::provider_connection::ProviderConnection;
use roko_core::traits::Connect;

// After "credential coverage" section:
println!("live connection health:");
let connections = ProviderConnection::from_config(&config);
let ctx = roko_core::Context::now();
for conn in &connections {
    let health = conn.health(&ctx).await
        .unwrap_or_else(|e| ConnectionHealth::unhealthy(conn.cell_name(), e.to_string()));
    if health.connected {
        println!("  {:<18} connected ({:.0}ms)",
            health.target, health.latency_ms.unwrap_or(0.0));
    } else {
        println!("  {:<18} UNREACHABLE: {}",
            health.target, health.error.as_deref().unwrap_or("unknown"));
    }
}
println!();
```

**Export `ConnectionHealth`** from `crates/roko-core/src/lib.rs`:
```rust
pub use traits::ConnectionHealth;
```

#### Error Handling

- `health()` never fails fatally -- network errors return
  `ConnectionHealth::unhealthy(...)` wrapped in `Ok(...)`.
- `request()` returns `RokoError::Invalid` (not `RokoError::Other`, which
  does not exist in the error enum).
- HTTP client build failure uses `RokoError::Transport(e.to_string())`.

#### Test Strategy

```bash
# No-network test (credential failure):
#[tokio::test]
async fn provider_connection_no_creds_returns_unhealthy() {
    let conn = ProviderConnection::new(
        "test-provider".into(),
        "https://example.com".into(),
        Some("NONEXISTENT_KEY".into()),
        false, // no credential
    );
    let ctx = Context::now();
    let health = conn.health(&ctx).await.unwrap();
    assert!(!health.connected);
    assert!(health.error.as_ref().unwrap().contains("not set"));
}

# Full workspace
cargo test --workspace
cargo run -p roko-cli -- config providers health
```

#### Cross-References

- `ConnectorKind`/`ConnectorStatus`/`ConnectorConfig` in
  `crates/roko-core/src/connector.rs:1-60` -- do NOT duplicate. The
  registry tracks static config; `ConnectionHealth` is for live probes.
- `RokoConfig::is_provider_available` considers both process env and
  `[agent.env]` -- use this, not `ProviderConfig::resolve_api_key()` alone.
- Existing `inspect_provider`/`probe_base_url` helpers in
  `crates/roko-cli/src/commands/config_cmd.rs` -- reuse timeout/probe patterns.

---

## Task 041: Trigger Trait Wiring

**Spec file**: `tmp/taskrunner/tasks/041-trigger-trait-wiring.md`
**Estimated time**: 180 minutes
**Depends on**: None (parallel with 039, 040)

### Current State of Stub Code

| File | Line(s) | Current State |
|---|---|---|
| `crates/roko-core/src/traits.rs:417-425` | `pub trait Trigger` | Sync stub: `fn arm() -> Result<()>`, `fn disarm() -> Result<()>`, no `check()`, no `TriggerBinding` return |
| `crates/roko-core/src/bus_trigger.rs` | Does not exist | File never created |
| `crates/roko-core/src/config/subscriptions.rs` | `SubscriptionConfig` | Has `trigger: String` field but no `topic_filter_from_trigger()` helper |
| `crates/roko-cli/src/subscriptions.rs:23` | `pub fn cmd_list` | Sync, does not use Trigger trait |

Zero implementations of Trigger exist anywhere. Zero callers.

### What Was Required vs What Exists

| Requirement | Delivered |
|---|---|
| Create `TriggerBinding` struct | Not done |
| Redesign `Trigger` to async with `arm/check/disarm` | Not done |
| Create `BusTrigger` in `bus_trigger.rs` | Not done |
| Add `topic_filter_from_trigger()` helper | Not done |
| Wire into `roko config subscriptions list` | Not done |
| Integration tests | Not done |

### Implementation Design

#### Data Structures

**TriggerBinding** (add to `crates/roko-core/src/traits.rs`, before Trigger trait):

```rust
/// Describes what a Trigger is watching after being armed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TriggerBinding {
    /// The topic filter this trigger matches against.
    pub filter: TopicFilter,
    /// Human-readable description of what this trigger watches.
    pub description: String,
}

impl TriggerBinding {
    pub fn new(filter: TopicFilter, description: impl Into<String>) -> Self {
        Self { filter, description: description.into() }
    }
}
```

**BusTrigger** (new file `crates/roko-core/src/bus_trigger.rs`):

```rust
use crate::cell::Cell;
use crate::context::Context;
use crate::error::Result;
use crate::pulse::{Pulse, Topic, TopicFilter};
use crate::traits::{Trigger, TriggerBinding};
use crate::{Body, Engram, Kind, Provenance};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct BusTrigger {
    name: String,
    filter: TopicFilter,
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
```

#### Function Signatures

**Trigger trait** (replace in `crates/roko-core/src/traits.rs`, lines 417-425):

```rust
// ---- Trigger ----

/// Trigger protocol -- armed conditions that fire when criteria are met.
///
/// Triggers subscribe to Bus topics or poll external state, then emit
/// activation Signals when their condition is satisfied.
#[async_trait]
pub trait Trigger: crate::cell::Cell {
    /// Arm the trigger. Returns a TriggerBinding describing what it watches.
    async fn arm(&self, ctx: &Context) -> Result<TriggerBinding>;

    /// Check if the trigger should fire given a batch of pulses.
    /// Returns Some(signals) if the trigger fires, None if not.
    async fn check(&self, pulses: &[Pulse], ctx: &Context) -> Result<Option<Vec<Engram>>>;

    /// Disarm the trigger, stopping all watches.
    async fn disarm(&self, ctx: &Context) -> Result<()>;
}
```

**BusTrigger::check()** -- the core logic:

```rust
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
            "matched_count": matching.len(),
            "topics": matching.iter().map(|p| p.topic.to_string()).collect::<Vec<_>>(),
        });
        let signal = Engram::builder(Kind::Metric)
            .body(Body::Json(body))
            .provenance(Provenance::agent(&self.name))
            .build();
        Ok(Some(vec![signal]))
    }

    async fn disarm(&self, _ctx: &Context) -> Result<()> {
        self.armed.store(false, Ordering::SeqCst);
        Ok(())
    }
}
```

Note: Uses `Kind::Metric` because `Kind::Event` and
`Kind::Custom("trigger.fired".into())` -- check which `Kind` variants
exist before using `Custom`. If `Custom(String)` exists, prefer
`Kind::Custom("trigger.fired".into())`. If not, use `Kind::Metric`.

**topic_filter_from_trigger** (add to `crates/roko-core/src/config/subscriptions.rs`):

```rust
use crate::pulse::{Topic, TopicFilter};

/// Convert a trigger string from SubscriptionConfig into a TopicFilter.
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
```

#### Integration Points

**Wire into `roko config subscriptions list`** in `crates/roko-cli/src/subscriptions.rs`:

1. Change `pub fn cmd_list` (line 23) to `pub async fn cmd_list`
2. Update dispatch arm in `crates/roko-cli/src/commands/config_cmd.rs` to
   `cmd_list(...).await?`
3. In the subscription iteration, create a BusTrigger and show binding info:

```rust
use roko_core::bus_trigger::BusTrigger;
use roko_core::config::subscriptions::topic_filter_from_trigger;
use roko_core::traits::Trigger;

// For each subscription in the registry:
let filter = topic_filter_from_trigger(&sub.trigger);
let trigger = BusTrigger::new(&sub_id, filter);
let ctx = roko_core::Context::now();
if let Ok(binding) = trigger.arm(&ctx).await {
    // Add binding.description to the row output
}
```

**Export from lib.rs** -- add to `crates/roko-core/src/lib.rs`:
```rust
pub mod bus_trigger;
pub use bus_trigger::BusTrigger;
pub use traits::TriggerBinding;
```

#### Error Handling

- `arm()`/`check()`/`disarm()` return `crate::error::Result` -- existing
  error types suffice.
- `check()` returns `Ok(None)` when disarmed or no matches -- not an error.

#### Test Strategy

```rust
#[tokio::test]
async fn bus_trigger_fires_on_matching_pulse() {
    let trigger = BusTrigger::new("test", TopicFilter::Exact(Topic::new("test.event")));
    let ctx = Context::now();

    let binding = trigger.arm(&ctx).await.unwrap();
    assert!(binding.description.contains("test"));

    // Non-matching
    let wrong = Pulse::new(1, Topic::new("other"), Kind::Metric, Body::empty());
    assert!(trigger.check(&[wrong], &ctx).await.unwrap().is_none());

    // Matching
    let right = Pulse::new(2, Topic::new("test.event"), Kind::Metric, Body::empty());
    let result = trigger.check(&[right], &ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);

    // After disarm
    trigger.disarm(&ctx).await.unwrap();
    let right2 = Pulse::new(3, Topic::new("test.event"), Kind::Metric, Body::empty());
    assert!(trigger.check(&[right2], &ctx).await.unwrap().is_none());
}
```

Confirmed: `Pulse::new(seq, topic, kind, body)` signature is at
`crates/roko-core/src/pulse.rs:93`. `Body::empty()` exists at
`crates/roko-core/src/body.rs`.

#### Cross-References

- `TopicFilter` at `crates/roko-core/src/pulse.rs:196-209` -- variants:
  `Exact(Topic)`, `Prefix(String)`, `All`, `And(Vec)`, `Or(Vec)`, `Not(Box)`.
- `TopicFilter::matches(&Topic)` at `crates/roko-core/src/pulse.rs:213` --
  confirmed method exists.
- `SubscriptionConfig` at `crates/roko-core/src/config/subscriptions.rs:59` --
  has `trigger: String` field (line 63).
- `SubscriptionRegistry` at `crates/roko-serve/src/dispatch.rs` -- loaded by
  `roko_cli::subscriptions::cmd_list`.

---

## Task 048: Harden CI Pipeline and Test Infrastructure

**Spec file**: `tmp/taskrunner/tasks/048-ci-pipeline-hardening.md`
**Estimated time**: 90 minutes
**Depends on**: None (independent infrastructure task)

### Current State of Stub Code

| File | Line(s) | Current State |
|---|---|---|
| `rust-toolchain.toml:2` | `channel = "stable"` | Unpinned, drifts with each stable release |
| `.github/workflows/ci.yml:19` | `dtolnay/rust-toolchain@stable` | Unpinned stable |
| `.github/workflows/ci.yml:13-24` | Single `test` job | Test + Clippy combined, no integration split |
| `crates/roko-serve/src/lib.rs:193` | `== 6677` | Hardcoded port literal |
| `crates/roko-serve/src/lib.rs:354` | `.unwrap_or(6677)` | Hardcoded port literal |

### What Was Required vs What Exists

| Requirement | Delivered |
|---|---|
| Pin `rust-toolchain.toml` to `1.95.0` | Not done -- still `channel = "stable"` |
| Pin CI to `dtolnay/rust-toolchain@1.95.0` | Not done -- still `@stable` |
| Split test into unit + integration jobs | Not done -- single combined job |
| Separate clippy job | Not done -- in same job as tests |
| Replace hardcoded `6677` with `DEFAULT_SERVE_PORT` | Not done |
| Fix port-race in smoke.rs | Not done |

### Implementation Design

#### Step 1: Pin `rust-toolchain.toml`

Replace contents of `/Users/will/dev/nunchi/roko/roko/rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.95.0"
components = ["rustfmt", "clippy"]
```

#### Step 2: Rewrite `.github/workflows/ci.yml`

Replace entire file:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  fmt:
    name: Formatting
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: rustfmt
      - run: cargo fmt --all --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.95.0
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --no-deps -- -D warnings

  test:
    name: Unit Tests
    runs-on: ubuntu-latest
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.95.0
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace --lib --bins

  integration-test:
    name: Integration Tests
    runs-on: ubuntu-latest
    timeout-minutes: 60
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.95.0
      - uses: Swatinem/rust-cache@v2
      - run: cargo test -p roko-serve --tests -- --test-threads=1
      - run: cargo test -p roko-cli --tests -- --test-threads=1

  layer-check:
    name: Layer Check
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.95.0
      - uses: Swatinem/rust-cache@v2
      - name: Build roko-cli
        run: cargo build -p roko-cli
      - name: Run layer check
        run: cargo run -p roko-cli -- layer-check
```

Key changes from current:
- `test` job renamed "Unit Tests", runs `--lib --bins` only
- New `integration-test` job runs `--tests` with `--test-threads=1`
- New `clippy` job runs in parallel with test jobs
- All non-fmt jobs use `@1.95.0` instead of `@stable`
- `layer-check` uses pinned toolchain

#### Step 3: Replace hardcoded `6677` in `crates/roko-serve/src/lib.rs`

Line 193 -- change:
```rust
// Before:
if self.roko_config.server.port == 6677 {
// After:
if self.roko_config.server.port == roko_core::defaults::DEFAULT_SERVE_PORT {
```

Line 354 -- change:
```rust
// Before:
let serve_port = self.config.port.unwrap_or(6677);
// After:
let serve_port = self.config.port.unwrap_or(roko_core::defaults::DEFAULT_SERVE_PORT);
```

Confirmed: `DEFAULT_SERVE_PORT` is defined at
`crates/roko-core/src/defaults.rs:199` as `pub const DEFAULT_SERVE_PORT: u16 = 6677`.

Do NOT change the test assertions at lines 2094-2148 -- those are testing
that the default value IS 6677, which is correct behavior documentation.

#### Step 4: Audit port usage in tests

Run:
```bash
rg ':6677|:6678|:6679|:6680' crates/roko-serve/tests/ crates/roko-cli/tests/ --glob '*.rs'
```

If any test files use hardcoded ports, change to port 0 binding. The current
test infrastructure at `crates/roko-cli/tests/common/mod.rs` uses
`pick_unused_port()` which binds `127.0.0.1:0` -- this is acceptable but
document the race in the status log if retained.

#### Integration Points

- `roko_core::defaults::DEFAULT_SERVE_PORT` at
  `crates/roko-core/src/defaults.rs:199` -- already exists, just needs to
  be referenced.
- No runtime behavior changes -- only CI structure and literal centralization.

#### Error Handling

No new error handling. This is infrastructure.

#### Test Strategy

```bash
# Verify YAML is valid
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"

# Verify tests pass with port 0
cargo test -p roko-serve --tests -- --test-threads=1
cargo test -p roko-cli --tests -- --test-threads=1

# Verify no hardcoded ports remain in production code
rg 'unwrap_or\(6677\)|== 6677' crates/roko-serve/src/lib.rs
# Should return 0 matches (test code excluded)

# Verify no unpinned stable remains
rg 'dtolnay/rust-toolchain@stable|channel = "stable"' \
  .github/workflows/ci.yml rust-toolchain.toml
# Should return 0 matches
```

---

## Task 097: Feed Trait + FileWatchFeed + ProviderHealthFeed

**Spec file**: `tmp/taskrunner/tasks/097-feed-trait-and-impls.md`
**Estimated time**: 300 minutes
**Depends on**: [35 (CellContext), 39, 40, 41]

### Current State of Stub Code

| File | Line(s) | Current State |
|---|---|---|
| `crates/roko-core/src/feed.rs` | Lines 1-271 | Only `FeedRegistry`, `FeedInfo`, `FeedKind`, `FeedAccess` -- no `Feed` trait, no `FeedRuntimeStatus` |
| `crates/roko-core/src/lib.rs:208` | `pub use feed::{FeedAccess, FeedInfo, FeedKind, FeedRegistry}` | No `Feed` or runtime types exported |
| `crates/roko-core/src/cell.rs:22-37` | `pub struct CellContext` | CellContext exists (task 035 DID land) with `bus`, `store`, `cancel`, `trace_id`, `run_id`, `budget_remaining` |

### What Was Required vs What Exists

| Requirement | Delivered |
|---|---|
| `Feed` trait with `start/stop/poll/status` | Not done |
| `FeedRuntimeStatus` struct | Not done |
| `FileWatchFeed` implementation | Not done |
| `ProviderHealthFeed` implementation | Not done |
| Export from lib.rs | Not done |
| Integration tests | Not done |

### Implementation Design

#### Data Structures

All added to `crates/roko-core/src/feed.rs` AFTER the existing `FeedRegistry`
tests (line 271). The existing registry types are untouched.

**FeedRuntimeStatus**:

```rust
/// Runtime status of a running Feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedRuntimeStatus {
    /// Whether the feed is currently connected to its source.
    pub connected: bool,
    /// Approximate publication rate (events per second over last minute).
    pub rate_hz: f64,
    /// Unix milliseconds of the last published Pulse.
    pub last_update_ms: Option<i64>,
    /// Last error message, if any.
    pub error: Option<String>,
    /// Total Pulses published since start.
    pub pulses_produced: u64,
}
```

**FileWatchFeed**:

```rust
use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use parking_lot::Mutex;

pub struct FileWatchFeed {
    id: String,
    path: PathBuf,
    topic: crate::pulse::Topic,
    running: Arc<AtomicBool>,
    pulses_produced: Arc<AtomicU64>,
    last_update_ms: Arc<Mutex<Option<i64>>>,
    latest_pulse: Arc<Mutex<Option<crate::Pulse>>>,
    last_error: Arc<Mutex<Option<String>>>,
}
```

**ProviderHealthFeed**:

```rust
pub struct ProviderHealthFeed {
    id: String,
    topic: crate::pulse::Topic,
    poll_interval: std::time::Duration,
    running: Arc<AtomicBool>,
    pulses_produced: Arc<AtomicU64>,
    last_update_ms: Arc<Mutex<Option<i64>>>,
    latest_pulse: Arc<Mutex<Option<crate::Pulse>>>,
    last_error: Arc<Mutex<Option<String>>>,
    /// Injected snapshot source -- core must not import roko-learn.
    health_source: Arc<dyn Fn() -> serde_json::Value + Send + Sync>,
}
```

#### Function Signatures

**Feed trait** (append to `crates/roko-core/src/feed.rs` after line 155):

```rust
use crate::cell::{Cell, CellContext};
use crate::error::Result;
use async_trait::async_trait;

/// A continuous data stream that publishes Pulses to the Bus.
///
/// Feeds compose the Cell, Connect, and Trigger protocols into a
/// unified external-data ingestion interface. They are activated by
/// the Engine and live for the duration of the runtime.
#[async_trait]
pub trait Feed: Cell + Send + Sync {
    /// The Bus topic this feed publishes to.
    fn topic(&self) -> &crate::pulse::Topic;

    /// Classification of this feed's data lineage.
    fn feed_kind(&self) -> FeedKind;

    /// Start producing data. Implementations should spawn a background
    /// task and return immediately.
    async fn start(&self, ctx: &CellContext) -> Result<()>;

    /// Stop producing data. Must be idempotent.
    async fn stop(&self) -> Result<()>;

    /// Poll for the latest Pulse without subscribing.
    /// Returns None if no data has been produced yet.
    async fn poll(&self) -> Result<Option<crate::Pulse>>;

    /// Return the current runtime status of this feed.
    async fn status(&self) -> Result<FeedRuntimeStatus>;
}
```

Note: `CellContext` exists at `crates/roko-core/src/cell.rs:25` with
`bus: Arc<dyn BusErased>`, `store: Arc<dyn Substrate>`,
`cancel: CancellationToken`. No local placeholder needed.

**FileWatchFeed::start** -- uses std polling (no `notify` dependency in roko-core):

```rust
impl FileWatchFeed {
    pub fn new(id: impl Into<String>, path: PathBuf, topic: crate::pulse::Topic) -> Self {
        Self {
            id: id.into(), path, topic,
            running: Arc::new(AtomicBool::new(false)),
            pulses_produced: Arc::new(AtomicU64::new(0)),
            last_update_ms: Arc::new(Mutex::new(None)),
            latest_pulse: Arc::new(Mutex::new(None)),
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn for_roko_dir(workdir: PathBuf) -> Self {
        Self::new("file-watch-roko-dir", workdir.join(".roko"),
            crate::pulse::Topic::new("fs.changed"))
    }
}

#[async_trait]
impl Feed for FileWatchFeed {
    fn topic(&self) -> &crate::pulse::Topic { &self.topic }
    fn feed_kind(&self) -> FeedKind { FeedKind::Raw }

    async fn start(&self, _ctx: &CellContext) -> Result<()> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(()); // already running
        }
        let path = self.path.clone();
        let topic = self.topic.clone();
        let running = Arc::clone(&self.running);
        let pulses_produced = Arc::clone(&self.pulses_produced);
        let last_update_ms = Arc::clone(&self.last_update_ms);
        let latest_pulse = Arc::clone(&self.latest_pulse);
        let last_error = Arc::clone(&self.last_error);

        // Std-based polling: scan directory recursively, compare
        // fingerprint (path + len + mtime) every 1s with 200ms debounce.
        std::thread::spawn(move || {
            let mut prev_fingerprint = fingerprint_dir(&path);
            let debounce = std::time::Duration::from_millis(200);
            let poll_interval = std::time::Duration::from_secs(1);

            while running.load(Ordering::Relaxed) {
                std::thread::sleep(poll_interval);
                let current = fingerprint_dir(&path);
                if current != prev_fingerprint {
                    std::thread::sleep(debounce); // debounce
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    let seq = pulses_produced.fetch_add(1, Ordering::Relaxed) + 1;
                    let body = crate::Body::Json(serde_json::json!({
                        "path": path.display().to_string(),
                        "changed_at_ms": now_ms,
                    }));
                    let pulse = crate::Pulse::new(seq, topic.clone(),
                        crate::Kind::Metric, body);
                    *last_update_ms.lock() = Some(now_ms);
                    *latest_pulse.lock() = Some(pulse);
                    *last_error.lock() = None;
                    prev_fingerprint = current;
                }
            }
        });
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn poll(&self) -> Result<Option<crate::Pulse>> {
        Ok(self.latest_pulse.lock().clone())
    }

    async fn status(&self) -> Result<FeedRuntimeStatus> {
        Ok(FeedRuntimeStatus {
            connected: self.running.load(Ordering::Relaxed),
            rate_hz: 0.0,
            last_update_ms: *self.last_update_ms.lock(),
            error: self.last_error.lock().clone(),
            pulses_produced: self.pulses_produced.load(Ordering::Relaxed),
        })
    }
}

/// Compute a fingerprint of a directory tree: sorted (path, len, mtime_ms).
fn fingerprint_dir(path: &std::path::Path) -> Vec<(String, u64, i64)> {
    let mut entries = Vec::new();
    if let Ok(walker) = std::fs::read_dir(path) {
        collect_entries(walker, &mut entries);
    }
    entries.sort();
    entries
}

fn collect_entries(
    dir: std::fs::ReadDir,
    out: &mut Vec<(String, u64, i64)>,
) {
    for entry in dir.flatten() {
        let path = entry.path();
        if let Ok(meta) = entry.metadata() {
            let mtime = meta.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            out.push((path.display().to_string(), meta.len(), mtime));
            if meta.is_dir() {
                if let Ok(sub) = std::fs::read_dir(&path) {
                    collect_entries(sub, out);
                }
            }
        }
    }
}
```

**ProviderHealthFeed**:

```rust
impl ProviderHealthFeed {
    pub fn new(
        id: impl Into<String>,
        poll_interval: std::time::Duration,
        health_source: Arc<dyn Fn() -> serde_json::Value + Send + Sync>,
    ) -> Self {
        Self {
            id: id.into(),
            topic: crate::pulse::Topic::new("provider.health"),
            poll_interval,
            running: Arc::new(AtomicBool::new(false)),
            pulses_produced: Arc::new(AtomicU64::new(0)),
            last_update_ms: Arc::new(Mutex::new(None)),
            latest_pulse: Arc::new(Mutex::new(None)),
            last_error: Arc::new(Mutex::new(None)),
            health_source,
        }
    }

    pub fn default_interval() -> Self {
        Self::new(
            "provider-health-feed",
            std::time::Duration::from_secs(30),
            Arc::new(|| serde_json::json!({})),
        )
    }
}

#[async_trait]
impl Feed for ProviderHealthFeed {
    fn topic(&self) -> &crate::pulse::Topic { &self.topic }
    fn feed_kind(&self) -> FeedKind { FeedKind::Meta }

    async fn start(&self, _ctx: &CellContext) -> Result<()> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let running = Arc::clone(&self.running);
        let pulses_produced = Arc::clone(&self.pulses_produced);
        let last_update_ms = Arc::clone(&self.last_update_ms);
        let latest_pulse = Arc::clone(&self.latest_pulse);
        let last_error = Arc::clone(&self.last_error);
        let poll_interval = self.poll_interval;
        let topic = self.topic.clone();
        let source = Arc::clone(&self.health_source);

        tokio::spawn(async move {
            while running.load(Ordering::Relaxed) {
                tokio::time::sleep(poll_interval).await;
                let snapshot = source();
                let now_ms = chrono::Utc::now().timestamp_millis();
                let seq = pulses_produced.fetch_add(1, Ordering::Relaxed) + 1;
                let body = crate::Body::Json(snapshot);
                let pulse = crate::Pulse::new(seq, topic.clone(),
                    crate::Kind::Metric, body);
                *last_update_ms.lock() = Some(now_ms);
                *latest_pulse.lock() = Some(pulse);
                *last_error.lock() = None;
            }
        });
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn poll(&self) -> Result<Option<crate::Pulse>> {
        Ok(self.latest_pulse.lock().clone())
    }

    async fn status(&self) -> Result<FeedRuntimeStatus> {
        Ok(FeedRuntimeStatus {
            connected: self.running.load(Ordering::Relaxed),
            rate_hz: 1.0 / self.poll_interval.as_secs_f64(),
            last_update_ms: *self.last_update_ms.lock(),
            error: self.last_error.lock().clone(),
            pulses_produced: self.pulses_produced.load(Ordering::Relaxed),
        })
    }
}
```

#### Integration Points

**Export from lib.rs** -- change line 208 of `crates/roko-core/src/lib.rs`:

```rust
pub use feed::{
    Feed, FeedAccess, FeedInfo, FeedKind, FeedRegistry, FeedRuntimeStatus,
    FileWatchFeed, ProviderHealthFeed,
};
```

**TUI watcher**: The existing `crates/roko-cli/src/tui/fs_watch.rs` is NOT
modified in this task. Task 098 wires `FileWatchFeed` into the TUI,
replacing the direct `notify` usage.

#### Error Handling

- `start()` returns `Ok(())` if already running (idempotent).
- Background thread/task panics are caught by the runtime; feed status
  shows `connected: false` when the running flag goes false.
- `poll()` returns `Ok(None)` until a pulse has been produced.

#### Test Strategy

All tests inline in `crates/roko-core/src/feed.rs`, after the existing
`FeedRegistry` tests:

```rust
#[cfg(test)]
mod feed_trait_tests {
    use super::*;
    use crate::cell::CellContext;
    use crate::bus_backends::MemoryBus;
    use tokio_util::sync::CancellationToken;
    use std::sync::Arc;

    fn make_ctx() -> CellContext {
        let bus = Arc::new(MemoryBus::new());
        let store = /* minimal in-memory store */;
        CellContext::new(bus, store, CancellationToken::new())
    }

    #[tokio::test]
    async fn file_watch_feed_starts_and_stops() {
        let tempdir = tempfile::tempdir().unwrap();
        let feed = FileWatchFeed::for_roko_dir(tempdir.path().to_path_buf());
        let ctx = make_ctx();
        feed.start(&ctx).await.unwrap();
        let status = feed.status().await.unwrap();
        assert!(status.connected);
        feed.stop().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let status = feed.status().await.unwrap();
        assert!(!status.connected);
    }

    #[tokio::test]
    async fn provider_health_feed_topic_is_correct() {
        let feed = ProviderHealthFeed::default_interval();
        assert_eq!(feed.topic().0, "provider.health");
        assert_eq!(feed.feed_kind(), FeedKind::Meta);
    }

    #[test]
    fn feed_kind_is_raw_for_file_watch() {
        let feed = FileWatchFeed::for_roko_dir(std::path::PathBuf::from("/tmp"));
        assert_eq!(feed.feed_kind(), FeedKind::Raw);
    }

    // Existing FeedRegistry tests remain above and must still pass.
}
```

Note: The `make_ctx()` helper needs a minimal `Store` implementation.
Either use the same inline `TestStore` pattern from task 039, or create a
`NullStore` that returns empty results for everything. Check if
`crates/roko-core/src/` has a `MemorySubstrate` or test fixture.

#### Cross-References

- `CellContext` at `crates/roko-core/src/cell.rs:25-37` -- confirmed it
  exists with `bus: Arc<dyn BusErased>`, `store: Arc<dyn Substrate>`.
- `FeedKind` at `crates/roko-core/src/feed.rs:24` -- reuse existing enum.
- `BusErased` at `crates/roko-core/src/bus_backends.rs` -- publish-only
  bus handle for feed publishing.
- `MemoryBus` at `crates/roko-core/src/bus_backends.rs` -- can be used
  in tests.
- TUI watcher at `crates/roko-cli/src/tui/fs_watch.rs` -- reference for
  fingerprint/debounce logic, but do NOT import from it.

---

## Task 100: Predict-Publish-Correct for CascadeRouter

**Spec file**: `tmp/taskrunner/tasks/100-predict-publish-correct.md`
**Estimated time**: 240 minutes
**Depends on**: [31, 99] (external blockers beyond this batch)

### Current State of Stub Code

| File | Line(s) | Current State |
|---|---|---|
| `crates/roko-learn/src/calibration_policy.rs:1` | `//! STATUS: NOT WIRED` header | Explicitly marked unwired |
| `crates/roko-learn/src/calibration_policy.rs:39-50` | `CalibrationPolicy` struct | Has `tracker`, `bias_threshold`, `min_samples`, `pending`, `corrections` -- but NO task-scoped methods |
| `crates/roko-learn/src/calibration_policy.rs:89` | `process_event()` | Processes `AgentEvent` variants but drains ALL pending on `TurnCompleted` -- unsafe for parallel tasks |
| `crates/roko-cli/src/runner/types.rs:1236-1310` | `RunConfig` struct | No `calibration_policy` field |
| `crates/roko-cli/src/runner/event_loop.rs` | Lines 336-4068 | Zero references to `CalibrationPolicy` or `calibration_policy` |

### What Was Required vs What Exists

| Requirement | Delivered |
|---|---|
| `register_prediction(task_id, model, category, score)` on CalibrationPolicy | Not done |
| `resolve_prediction(task_id, actual_success) -> Option<Correction>` | Not done |
| `calibration_policy: Option<Arc<Mutex<CalibrationPolicy>>>` on RunConfig | Not done |
| Record prediction at dispatch time in event_loop | Not done |
| Resolve prediction at gate completion | Not done |
| Flush at run-end | Not done |
| Integration test | Not done |

### Implementation Design

#### Data Structures

**New methods on `CalibrationPolicy`** (add to `crates/roko-learn/src/calibration_policy.rs`):

```rust
impl CalibrationPolicy {
    // ... existing methods ...

    /// Register a task-scoped prediction. The key uniquely identifies
    /// a (plan, task, attempt) tuple.
    pub fn register_prediction(
        &mut self,
        key: impl Into<String>,
        model: impl Into<String>,
        category: impl Into<String>,
        predicted_success_prob: f64,
    ) {
        self.pending.insert(key.into(), PendingPrediction {
            model: model.into(),
            category: category.into(),
            predicted_success_prob: predicted_success_prob.clamp(0.0, 1.0),
        });
    }

    /// Resolve a task-scoped prediction by key and return a correction
    /// if bias exceeds the threshold.
    ///
    /// Unlike `process_event()`, this removes only the matching entry
    /// (safe for concurrent task execution).
    pub fn resolve_prediction(
        &mut self,
        key: impl AsRef<str>,
        actual_success: bool,
    ) -> Option<CalibrationCorrection> {
        let pending = self.pending.remove(key.as_ref())?;
        let actual = if actual_success { 1.0 } else { 0.0 };
        let residual = pending.predicted_success_prob - actual;
        let category = if pending.category.is_empty() { "" } else { &pending.category };

        self.tracker.record_residual(&pending.model, category, residual);

        let samples = self.tracker.sample_count(&pending.model, category);
        if samples >= self.min_samples {
            let bias = self.tracker.mean_bias(&pending.model, category);
            if bias.abs() > self.bias_threshold {
                return Some(CalibrationCorrection {
                    model: pending.model,
                    category: category.to_string(),
                    mean_bias: bias,
                    correction: -bias,
                    sample_count: samples,
                });
            }
        }
        None
    }
}
```

Key difference from `process_event()`: `resolve_prediction()` removes
only the matching key (not all pending entries), making it safe for
parallel task execution.

**New field on `RunConfig`** (add to `crates/roko-cli/src/runner/types.rs` line ~1295):

```rust
/// Calibration policy for closing the predict-publish-correct loop.
pub calibration_policy: Option<Arc<parking_lot::Mutex<roko_learn::calibration_policy::CalibrationPolicy>>>,
```

#### Function Signatures

**Attempt key helper** (add to `crates/roko-cli/src/runner/event_loop.rs`):

```rust
/// Build a unique key for a calibration prediction: "plan_id:task_id:attempt"
fn calibration_attempt_key(plan_id: &str, task_id: &str, attempt: u32) -> String {
    format!("{plan_id}:{task_id}:{attempt}")
}
```

#### Integration Points (4 wiring sites in event_loop.rs)

**Site 1: Record prediction at dispatch time** (~line 3062, after `routing_context` is set)

Find the code block where `requested_model` is final and the TUI logs
the model selection. Insert:

```rust
// Record calibration prediction if router-driven
if let (Some(cal), Some(router), Some(routing_ctx)) = (
    &config.calibration_policy,
    &config.cascade_router,
    &state.routing_context,
) {
    let came_from_router = !dispatch_plan.forced
        && task_def.model_hint.is_none()
        && config.cli_model_override.is_none();
    if came_from_router {
        let explanation = router.explain_route(routing_ctx, None);
        if explanation.selected_slug == requested_model {
            let score = explanation.candidates.iter()
                .find(|c| c.selected || c.slug == requested_model)
                .map(|c| c.score.clamp(0.0, 1.0))
                .unwrap_or(0.5);
            cal.lock().register_prediction(
                calibration_attempt_key(&plan_id, &task_id, attempt_num),
                requested_model.clone(),
                routing_ctx.task_category.label().to_string(),
                score,
            );
        }
    }
}
```

**Site 2: Resolve prediction at terminal gate pass** (near the
`RunnerEvent::task_attempt_completed` emit for the PASS path)

```rust
// After: output_sink.event(&RunnerEvent::task_attempt_completed(...Passed...));
if let (Some(cal), Some(router)) = (&config.calibration_policy, &config.cascade_router) {
    let key = calibration_attempt_key(&completion.plan_id, &completion.task_id,
        completion_attempt.attempt);
    if let Some(correction) = cal.lock().resolve_prediction(&key, true) {
        let synthetic_success = correction.correction > 0.0;
        if router.record_confidence_outcome(&correction.model, synthetic_success) {
            tracing::info!(
                model = %correction.model,
                mean_bias = correction.mean_bias,
                correction_value = correction.correction,
                synthetic_success,
                "CalibrationPolicy correction applied to CascadeRouter"
            );
            if let Err(err) = router.save(&config.layout.cascade_router_path()) {
                tracing::warn!(error = %err, "failed to persist router after calibration");
            }
            tui.cascade_router_updated(&router.snapshot_json());
        }
    }
}
```

**Site 3: Resolve prediction at terminal gate fail/exhausted**

Same pattern as Site 2 but with `actual_success = false`:

```rust
// After: output_sink.event(&RunnerEvent::task_attempt_completed(...Failed/Exhausted...));
if let (Some(cal), Some(router)) = (&config.calibration_policy, &config.cascade_router) {
    let key = calibration_attempt_key(&completion.plan_id, &completion.task_id,
        completion_attempt.attempt);
    if let Some(correction) = cal.lock().resolve_prediction(&key, false) {
        // ... same correction application as Site 2 ...
    }
}
```

**Site 4: Flush at run-end** (~line 2534, near `cascade_router_json` snapshot)

```rust
if let (Some(cal), Some(router)) = (&config.calibration_policy, &config.cascade_router) {
    let corrections = cal.lock().drain_corrections();
    for correction in &corrections {
        let synthetic_success = correction.correction > 0.0;
        tracing::debug!(
            model = %correction.model,
            samples = correction.sample_count,
            bias = correction.mean_bias,
            synthetic_success,
            "Calibration correction at run end"
        );
        router.record_confidence_outcome(&correction.model, synthetic_success);
    }
    if !corrections.is_empty() {
        if let Err(e) = router.save(&config.layout.cascade_router_path()) {
            tracing::warn!(error = %e, "failed to persist router after calibration");
        }
    }
}
```

**Site 5: Startup log** (~line 537, near `has_cascade_router` log)

```rust
tracing::info!(
    has_cascade_router = config.cascade_router.is_some(),
    has_calibration_policy = config.calibration_policy.is_some(),
    "predict-publish-correct calibration loop active"
);
```

**RunConfig construction sites** -- find with:
```bash
rg -n "RunConfig \{" crates/roko-cli/src/
```

At each construction site, add:
```rust
calibration_policy: Some(Arc::new(parking_lot::Mutex::new(
    roko_learn::calibration_policy::CalibrationPolicy::new()
))),
```

For test/smoke construction sites, use `calibration_policy: None`.

#### Error Handling

- CalibrationPolicy errors are never fatal. Gate failures are expected.
- `resolve_prediction()` returns `None` for unknown keys -- silently skip.
- `router.save()` failure is logged at `warn` level but does not abort.
- `router.record_confidence_outcome()` returns `false` for unknown models --
  log and skip.

#### Test Strategy

Add to `crates/roko-learn/src/calibration_policy.rs`:

```rust
#[cfg(test)]
mod task_scoped_tests {
    use super::*;

    #[test]
    fn register_and_resolve_prediction() {
        let mut policy = CalibrationPolicy::new()
            .with_min_samples(3)
            .with_bias_threshold(0.1);

        // Register 5 overconfident predictions
        for i in 0..5 {
            policy.register_prediction(
                format!("task-{i}"),
                "model-a",
                "implementation",
                0.9, // predicted high
            );
            policy.resolve_prediction(format!("task-{i}"), false); // always fail
        }

        // After enough samples, should see correction
        let tracker = policy.tracker();
        assert!(tracker.sample_count("model-a", "implementation") >= 5);
        assert!(tracker.mean_bias("model-a", "implementation") > 0.1);
    }

    #[test]
    fn concurrent_predictions_resolve_independently() {
        let mut policy = CalibrationPolicy::new()
            .with_min_samples(1)
            .with_bias_threshold(0.01);

        policy.register_prediction("task-a", "model-a", "impl", 0.9);
        policy.register_prediction("task-b", "model-b", "impl", 0.2);

        // Resolve in different order
        let _corr_b = policy.resolve_prediction("task-b", true);
        let _corr_a = policy.resolve_prediction("task-a", false);

        // Both should have been removed
        assert!(policy.resolve_prediction("task-a", true).is_none()); // already resolved
        assert!(policy.resolve_prediction("task-b", false).is_none());
    }

    #[test]
    fn overconfident_router_triggers_task_scoped_correction() {
        let mut policy = CalibrationPolicy::new()
            .with_min_samples(5)
            .with_bias_threshold(0.15);

        let mut saw_correction = false;
        for i in 0..10 {
            let key = format!("t-{i}");
            policy.register_prediction(key.clone(), "model-a", "implementation", 0.9);
            if let Some(c) = policy.resolve_prediction(key, false) {
                assert!(c.mean_bias > 0.1);
                assert_eq!(c.model, "model-a");
                saw_correction = true;
            }
        }
        assert!(saw_correction);
    }
}
```

Verify the existing 3 tests in the `tests` module still pass -- they use
`process_event()` which is unchanged.

```bash
cargo test -p roko-learn -- calibration_policy
```

#### Cross-References

- `CascadeRouter::record_confidence_outcome()` at
  `crates/roko-learn/src/cascade_router.rs:1086` -- takes `(&str, bool)`,
  returns `bool`.
- `CascadeRouter::explain_route()` at line 1327 -- returns
  `CascadeRouteExplanation` with `selected_slug` and `candidates` (vec of
  `CascadeCandidateScore { slug, score, selected, ... }`).
- `CascadeRouter::save()` -- persists to path.
- `CascadeRouter::snapshot_json()` -- returns `String`.
- `CalibrationTracker` at `crates/roko-learn/src/prediction.rs` -- has
  `record_residual()`, `sample_count()`, `mean_bias()`.
- `RunConfig.cascade_router` at `crates/roko-cli/src/runner/types.rs:1284` --
  `Option<Arc<CascadeRouter>>`.
- Normal feedback path: `RoutingObservationSink` already records task
  outcomes -- calibration must NOT double-count. Only apply synthetic
  corrections from CalibrationPolicy, not raw outcomes.

---

## Common Anti-Patterns to Avoid Across All Stubs

1. **`RokoError::Other` does not exist** -- use `RokoError::Invalid(String)` or
   `RokoError::Transport(String)` instead.

2. **`Kind::Event` does not exist** -- use `Kind::Metric` or check if
   `Kind::Custom(String)` is available.

3. **`Engram::builder()` requires a `Kind`** -- always pass
   `Engram::builder(Kind::Metric)` or similar, never `Engram::builder()`.

4. **`.author(...)` does not exist on builder** -- use
   `.provenance(Provenance::agent("name"))`.

5. **Do not add blocking wrappers** (`block_on`) around async calls -- the
   dispatch paths are already async.

6. **Do not import cross-crate where forbidden** -- `roko-core` must not
   depend on `roko-cli`, `roko-learn`, `roko-serve`, or `roko-std`.

7. **Verify the Substrate trait** -- `crates/roko-core/src/traits.rs:427-429`
   defines `trait Substrate: Store {}` with a blanket impl. CellContext uses
   `Arc<dyn Substrate>` for the store handle.
