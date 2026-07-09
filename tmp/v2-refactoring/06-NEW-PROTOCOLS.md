# Phase 1C: New Protocol Traits — Observe, Connect, Trigger

## What Changes

V2 defines 9 protocols. V1 has 6 (Store, Score, Verify, Route, Compose, React).
Three are missing: Observe, Connect, Trigger.

## The Three New Protocols

### Observe — Passive data collection

```rust
/// Read-only observation of external or internal state.
///
/// Unlike Connect (which manages bidirectional I/O), Observe is passive:
/// it reads state and emits observation Signals without side effects.
#[async_trait]
pub trait Observe: Cell {
    /// Observe the current state and return observation signals.
    async fn observe(&self, ctx: &Context) -> Result<Vec<Signal>>;
}
```

**What it replaces**: Ad-hoc status/health check functions scattered across crates.

**Existing code that could implement it**:
- Agent health checks → `ObserveCell` for agent status
- Store statistics → `ObserveCell` for storage metrics
- System metrics → `ObserveCell` for resource usage

**Wire target**: `roko status` currently calls ad-hoc functions. Could call
`Observe::observe()` on registered observers instead.

### Connect — External system lifecycle

```rust
/// Manages a connection to an external system with lifecycle semantics.
///
/// Connect provides open/close/health lifecycle for durable connections
/// (databases, APIs, WebSocket streams, blockchain nodes).
#[async_trait]
pub trait Connect: Cell {
    /// Open/establish the connection.
    async fn open(&self, ctx: &Context) -> Result<()>;
    /// Close/tear down the connection.
    async fn close(&self, ctx: &Context) -> Result<()>;
    /// Check connection health.
    async fn health(&self, ctx: &Context) -> Result<ConnectionHealth>;
    /// Send a request and receive a response through the connection.
    async fn request(&self, input: Signal, ctx: &Context) -> Result<Signal>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHealth {
    pub connected: bool,
    pub latency_ms: Option<f64>,
    pub error: Option<String>,
}
```

**What it replaces**: Provider connection management in roko-agent (currently each
LLM backend manages its own HTTP client lifecycle informally).

**Existing code that could implement it**:
- LLM provider backends → `ConnectCell` per provider
- MCP server connections → `ConnectCell` per MCP server
- WebSocket connections in roko-serve → `ConnectCell`

**Wire target**: Provider health checks in `roko config providers health` could use
`Connect::health()` instead of ad-hoc HTTP pings.

### Trigger — Event-driven activation

```rust
/// Watches for conditions and fires when criteria are met.
///
/// Triggers subscribe to Bus topics or poll external state, then emit
/// activation Signals when their condition is satisfied.
#[async_trait]
pub trait Trigger: Cell {
    /// Arm the trigger. Returns the filter it will watch.
    async fn arm(&self, ctx: &Context) -> Result<TriggerBinding>;
    /// Check if the trigger should fire given a set of pulses.
    async fn check(&self, pulses: &[Pulse], ctx: &Context) -> Result<Option<Vec<Signal>>>;
    /// Disarm the trigger.
    async fn disarm(&self, ctx: &Context) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerBinding {
    pub filter: TopicFilter,
    pub description: String,
}
```

**What it replaces**: Webhook handlers, cron triggers, file watchers — currently
ad-hoc implementations in roko-serve and roko-conductor.

**Existing code that could implement it**:
- File watcher in TUI → `TriggerCell` for file changes
- PRD auto-plan trigger → `TriggerCell` that fires on prd.published
- Webhook endpoints → `TriggerCell` per webhook

**Wire target**: `roko config subscriptions` already defines event filters. Triggers
formalize this.

## Implementation Plan

### Step 1: Add traits to roko-core/src/traits.rs (1-2 hours)

Add the three traits after the existing six. Each requires `Cell` as supertrait.

### Step 2: Add one implementation per trait (2-3 hours)

**Observe**: Implement `StoreObserver` that returns store statistics:
```rust
struct StoreObserver { store: Arc<dyn Store> }
impl Observe for StoreObserver {
    async fn observe(&self, ctx: &Context) -> Result<Vec<Signal>> {
        let count = self.store.len().await?;
        Ok(vec![Signal::observation("store.stats", json!({ "count": count }))])
    }
}
```
Wire into: `roko status` command.

**Connect**: Implement `ProviderConnection` for LLM provider health:
```rust
struct ProviderConnection { endpoint: String, client: reqwest::Client }
impl Connect for ProviderConnection {
    async fn health(&self, _ctx: &Context) -> Result<ConnectionHealth> {
        // HTTP ping to provider endpoint
    }
}
```
Wire into: `roko config providers health` command.

**Trigger**: Implement `BusTrigger` that watches for specific Bus topics:
```rust
struct BusTrigger { filter: TopicFilter, condition: TriggerCondition }
impl Trigger for BusTrigger {
    async fn check(&self, pulses: &[Pulse], _ctx: &Context) -> Result<Option<Vec<Signal>>> {
        if pulses.iter().any(|p| self.filter.matches(&p.topic)) {
            Ok(Some(vec![Signal::trigger_fired(...)]))
        } else {
            Ok(None)
        }
    }
}
```
Wire into: Event subscription system (roko-serve already has subscriptions).

### Step 3: Wire at least one impl per trait (1-2 hours each)

This is the critical step. Without wiring, these become more floating code.

## Files to Change

| File | Change |
|------|--------|
| `crates/roko-core/src/traits.rs` | Add Observe, Connect, Trigger traits |
| `crates/roko-core/src/lib.rs` | Export new traits + ConnectionHealth, TriggerBinding |
| `crates/roko-cli/src/status.rs` | Wire StoreObserver into `roko status` |
| `crates/roko-cli/src/config.rs` | Wire ProviderConnection into `roko config providers health` |

## What NOT to Do

- Don't implement Connect for every LLM provider immediately. Start with one.
- Don't build a TriggerRegistry. The Engine (Phase 2) will manage triggers.
- Don't add Observe to every gate/scorer. Start with StoreObserver for `roko status`.
