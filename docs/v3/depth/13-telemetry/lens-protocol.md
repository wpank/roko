# Depth 13-01: Lens Protocol

> TelemetryObserve trait, Lens trait, LensScope hierarchy, and the
> separation between event-oriented and metric-oriented observation.

**Parent**: [13-TELEMETRY](../../13-TELEMETRY.md) -- Sections 1, 2

---

## 1. Two Observation Protocols

The telemetry system exposes two complementary observation protocols.
Both are read-only -- removing every Lens from a running system changes
nothing about its behavior, only visibility.

### 1.1 TelemetryObserve (Event-Oriented)

Defined in `crates/roko-core/src/telemetry_observe.rs`. Lenses that
implement this protocol observe discrete lifecycle events
(`ObservableEvent`) and emit zero or more Signals per event:

```rust
#[async_trait]
pub trait TelemetryObserve: Send + Sync {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>>;
    fn observes(&self) -> &[ObservableEventKind];
    fn scope(&self) -> LensScope;
}
```

The three methods serve distinct roles:

| Method | Purpose |
|--------|---------|
| `observe` | Process one immutable event, emit derived Signals |
| `observes` | Declare event-family filters for routing |
| `scope` | Declare subject scope for scope-matching |

### 1.2 TelemetryEventSink

The execution engine depends only on the passive sink contract, not the
full Lens protocol. This decouples the engine from routing, chaining,
projection, and overhead enforcement:

```rust
#[async_trait]
pub trait TelemetryEventSink: Send + Sync {
    async fn emit(
        &self,
        event: &ObservableEvent,
        ancestry: &[LensScope],
    ) -> Result<Vec<Signal>>;
}
```

The `ancestry` parameter carries runtime-resolved scope information
(Cell, Graph, Agent, Space) so the sink can route without guessing
topology.

### 1.3 Lens (Metric-Oriented)

Defined in `crates/roko-core/src/obs/lens.rs`. Lenses that implement
this protocol project live metric state from a `MetricRegistry` into a
uniform `LensSnapshot`:

```rust
pub trait Lens: Send + Sync {
    fn name(&self) -> &str;
    fn scope(&self) -> &LensScope;
    fn snapshot(&self) -> LensSnapshot;
}
```

The `snapshot()` method is deliberately synchronous and infallible.
The source (`MetricRegistry`) handles its own locking. Callers schedule
how often to poll -- the Lens itself never initiates I/O.

---

## 2. LensScope Hierarchy

Two `LensScope` enums exist in the codebase. They share the same
semantic hierarchy but live in different modules:

### 2.1 Event-Oriented LensScope

Location: `crates/roko-core/src/telemetry_observe.rs`

```rust
pub enum LensScope {
    Cell(String),
    Graph(String),
    Agent(String),
    Space(String),
    Lens(String),    // Chain: observes another Lens
    Global,
}
```

### 2.2 Metric-Oriented LensScope

Location: `crates/roko-core/src/obs/lens.rs`

```rust
pub enum LensScope {
    Component(String),
    Graph(String),
    Agent(String),
    Space(String),
    Lens(String),
    Global,
}
```

The only difference is `Cell` vs `Component` at the leaf level. Both
use `String` identifiers to avoid coupling the core contract to graph,
agent, or workspace types.

### 2.3 Scope Matching Rules

For the event-oriented `LensScope`:

| Scope | Matches |
|-------|---------|
| `Global` | Every source |
| `Cell("")` | Any Cell source (family wildcard) |
| `Cell("compile")` | Only the named Cell |
| `Lens("cost")` | Output of the named upstream Lens |

The `matches_source` method implements these rules:

```rust
pub fn matches_source(&self, source: &Self) -> bool {
    match (self, source) {
        (Self::Global, _) => true,
        (Self::Cell(expected), Self::Cell(actual))
        | (Self::Graph(expected), Self::Graph(actual))
        | (Self::Agent(expected), Self::Agent(actual))
        | (Self::Space(expected), Self::Space(actual))
        | (Self::Lens(expected), Self::Lens(actual)) => {
            expected.is_empty() || expected == actual
        }
        _ => false,
    }
}
```

Cross-level containment (Cell belongs to Graph belongs to Agent) must
be resolved by the runtime because the portable event contract carries
no topology graph. The `route_with_ancestry` method in `LensRegistry`
handles this by accepting a runtime-provided ancestry list.

---

## 3. LensConfig and TOML Representation

Each Lens is configured via a `[[lenses]]` entry in TOML:

```toml
[[lenses]]
name = "cost-monitor"
block = "roko:cost-lens@^1.0"
scope = "graph:checkout"

[lenses.params]
interval = "60s"
budget_warn_pct = 0.8
```

The parsed `LensConfig`:

```rust
pub struct LensConfig {
    pub name: String,    // Unique within the registry
    pub block: String,   // Lens executor reference
    pub scope: String,   // Parsed into LensScope
    pub params: BTreeMap<String, toml::Value>,
}
```

### 3.1 Scope Parsing

The `parse_scope` function in `lens_registry.rs` converts string
scope declarations into `LensScope` values:

| Input | Result |
|-------|--------|
| `"global"` | `LensScope::Global` |
| `"cell"` | `LensScope::Cell("")` (family wildcard) |
| `"cell:worker"` | `LensScope::Cell("worker")` |
| `"lens:cost-monitor"` | `LensScope::Lens("cost-monitor")` |
| `""` | Error: scope cannot be empty |
| `"lens"` | Error: chained lens requires a name |
| `"cell:"` | Error: empty target |
| `"global:x"` | Error: global cannot have a target |

Parsing is case-insensitive and whitespace-tolerant.

### 3.2 Validation Invariants

`validate_config` enforces:
- Name must be non-empty and have no surrounding whitespace
- Block must be non-empty
- Duplicate names within a registry are rejected

---

## 4. LensSnapshot

The uniform output shape of every metric-oriented `Lens`:

```rust
pub struct LensSnapshot {
    pub lens_name: Cow<'static, str>,
    pub scope: LensScope,
    pub version: u64,
    pub metrics: Vec<MetricSnapshot>,
}
```

The `version` field is a monotonically increasing counter bumped on
every `snapshot()` call. Downstream consumers (StateHub, TUI, SSE) can
detect stale snapshots by comparing versions.

---

## 5. Source References

| File | What it contains |
|------|------------------|
| `crates/roko-core/src/telemetry_observe.rs` | `TelemetryObserve`, `TelemetryEventSink`, event `LensScope`, `ObservableEvent`, `ObservableEventKind`, typed payloads, pure calculation helpers |
| `crates/roko-core/src/obs/lens.rs` | `Lens` trait, metric `LensScope`, `LensSnapshot`, `CollectorLens`, `TokenUsageLens`, `LatencyLens`, `CostLens`, `LensRegistry`, `default_registry` |
| `crates/roko-core/src/obs/telemetry_observe.rs` | `TelemetryObserve` trait, `TelemetryObservation`, `PeriodicObserver` |
| `crates/roko-core/src/lens_registry.rs` | `LensRegistry` (event routing + chain validation), `LensConfig`, `LensRegistration`, `parse_scope`, `observes_for_block`, topological sort |

---

## Verification

```bash
# Lens trait Send + Sync
cargo test -p roko-core collector_lens_is_send_sync
cargo test -p roko-core token_usage_lens_is_send_sync
cargo test -p roko-core latency_lens_is_send_sync
cargo test -p roko-core cost_lens_is_send_sync
cargo test -p roko-core periodic_observer_is_send_sync

# LensScope parsing
cargo test -p roko-core parse_scope_accepts_wildcards_and_named_scopes

# LensConfig TOML round-trip
cargo test -p roko-core lens_config_round_trips_toml_params

# CollectorLens projects MetricRegistry state
cargo test -p roko-core collector_lens_projects_registry_into_snapshot
cargo test -p roko-core collector_lens_version_increments
cargo test -p roko-core collector_lens_reflects_live_mutations
```
