# Depth 13-02: StateHub Projections

> All 7 core projections, push protocol, and the typed aggregation
> pipeline that connects Lens output to surfaces.

**Parent**: [13-TELEMETRY](../../13-TELEMETRY.md) -- Sections 4, 5

---

## 1. Projection Architecture

StateHub projections are typed, pre-aggregated views of Lens output.
Each projection summarizes one domain into a struct consumed by TUI tabs,
SSE streams, REST endpoints, and WebSocket subscribers. Projections are
push-updated: the telemetry runtime pushes new values when Lens output
changes rather than requiring consumers to poll raw Lens snapshots.

The projection structs live in `crates/roko-core/src/telemetry_projections.rs`.
The HTTP routes are in `crates/roko-serve/src/routes/projections.rs`.

---

## 2. The Seven Core Projections

### 2.1 CohortHealthProjection

Fleet-level health summary across all active agents:

```rust
pub struct CohortHealthProjection {
    pub agent_count: usize,
    pub active_count: usize,
    pub avg_vitality: f64,
    pub avg_pass_rate: f64,
    pub total_spend_usd: f64,
    pub error_rate: f64,
    pub t0_hit_rate: f64,
    pub regime_distribution: BTreeMap<String, usize>,
}
```

**Source Lenses**: Efficiency, Error, Budget
**Consumers**: Dashboard overview, fleet status API

### 2.2 ActiveTasksProjection

Active and recently completed task summary:

```rust
pub struct ActiveTasksProjection {
    pub tasks: Vec<TaskSnapshot>,
    pub queued: usize,
    pub completed_last_hour: usize,
    pub avg_task_duration_ms: u64,
}

pub struct TaskSnapshot {
    pub id: String,
    pub title: String,
    pub status: String,
    pub agent: Option<String>,
    pub started_at: Option<String>,
    pub duration_ms: Option<u64>,
}
```

**Source Lenses**: Cell lifecycle events
**Consumers**: TUI task tab, plan status API

### 2.3 GatePipelineProjection

Verification pipeline health across all rungs:

```rust
pub struct GatePipelineProjection {
    pub rungs: Vec<RungSnapshot>,
    pub overall_pass_rate: f64,
    pub avg_reward: f64,
    pub hard_criteria_fail_rate: f64,
}

pub struct RungSnapshot {
    pub name: String,
    pub pass_count: u64,
    pub fail_count: u64,
    pub pass_rate: f64,
}
```

**Source Lenses**: Quality
**Consumers**: Gate health dashboard, adaptive threshold tuning

### 2.4 CostMeterProjection

Cost, remaining budget, and burn rate trend:

```rust
pub struct CostMeterProjection {
    pub total_usd: f64,
    pub budget_remaining: f64,
    pub burn_rate_usd_per_hour: f64,
    pub model_breakdown: BTreeMap<String, f64>,
    pub cost_trend: String,
}
```

**Source Lenses**: Cost, Budget
**Consumers**: TUI cost tab, budget alert API

### 2.5 KnowledgeHealthProjection

Knowledge store quality and lifecycle summary:

```rust
pub struct KnowledgeHealthProjection {
    pub total_entries: u64,
    pub tier_distribution: BTreeMap<String, u64>,
    pub avg_balance: f64,
    pub cold_entries: u64,
    pub heuristic_count: u64,
    pub heuristic_avg_calibration: f64,
    pub anti_knowledge_count: u64,
}
```

**Source Lenses**: Drift
**Consumers**: Knowledge dashboard, GC scheduling

### 2.6 CFactorProjection

Collective intelligence measurement:

```rust
pub struct CFactorProjection {
    pub c_factor: f64,
    pub components: BTreeMap<String, f64>,
    pub trend: String,
    pub agent_diversity: f64,
}
```

**Source Lenses**: Collective Intelligence
**Consumers**: TUI coordination tab, c-factor API

### 2.7 AgentVitalityProjection

Per-agent vitality state for surfaces:

```rust
pub struct AgentVitalityProjection {
    pub agents: Vec<AgentVitalitySnapshot>,
}

pub struct AgentVitalitySnapshot {
    pub name: String,
    pub vitality: f64,
    pub phase: String,
    pub regime: String,
    pub slots_active: usize,
    pub slots_total: usize,
    pub tasks_completed: u64,
    pub current_task: Option<String>,
}
```

**Source Lenses**: Agent lifecycle events
**Consumers**: TUI agent tab, agent status API

---

## 3. Push Protocol

### 3.1 HTTP REST Read

Each projection has a dedicated GET endpoint:

```
GET /projections/workbench     -> WorkbenchProjection
GET /projections/inbox         -> InboxProjection
GET /projections/canvas        -> CanvasProjection
GET /projections/minimap       -> MinimapProjection
GET /projections/autonomy      -> AutonomyProjection
GET /projections/telemetry     -> Telemetry snapshot
GET /projections/{name}        -> Named projection by key
```

### 3.2 SSE Streaming

Each projection can be consumed as a server-sent event stream:

```
GET /projections/telemetry/stream  -> SSE of telemetry updates
GET /projections/{name}/stream     -> SSE of named projection deltas
```

The stream uses `ProjectionEnvelope` as the envelope type and supports
a `since` query parameter to skip events older than a given timestamp.
Delta frames are computed via `projection_delta_frame()` which filters
events through `projection_accepts_event()`.

### 3.3 Lens Runtime Controls

The serve routes expose per-runtime inspection:

```
GET /statehub/lens-runtimes             -> List all Lens runtimes
GET /statehub/lens-runtimes/{runtime_id} -> Detail for one runtime
```

### 3.4 Projection Catalog

```
GET /projections/catalog
```

Returns metadata about all available projections including their names,
schemas, and update frequencies.

---

## 4. Projection Query Protocol

Named projection reads accept `ProjectionQuery` parameters:

```rust
pub struct ProjectionQuery {
    pub resolution: Option<String>,    // Temporal resolution
    pub since: Option<DateTime<Utc>>,  // History window start
    pub limit: Option<usize>,          // Max records
}
```

The resolution parameter supports coalescing: multiple data points
within a resolution window are merged into one record. This allows
dashboards to request hourly summaries without transferring per-second
granularity.

---

## 5. Projection Defaults

All seven core projection structs derive `Default`. The default state
represents an empty system with zero agents, zero cost, and neutral
metrics. This ensures surfaces render correctly before the first Lens
observation cycle runs.

---

## 6. Source References

| File | What it contains |
|------|------------------|
| `crates/roko-core/src/telemetry_projections.rs` | All 7 projection structs |
| `crates/roko-serve/src/routes/projections.rs` | REST/SSE projection routes, `RuntimeProjectionSet`, delta frames |
| `crates/roko-serve/src/projection_contract.rs` | `ProjectionEnvelope`, `ProjectionQuery`, named surface projections |
| `crates/roko-serve/src/state.rs` | `AppState` fields that hold projection state |

---

## Verification

```bash
# Projection structs derive Default and Serialize
cargo test -p roko-core -- telemetry_projections

# Projection routes compile and register
grep -c 'route.*projections' crates/roko-serve/src/routes/projections.rs

# SSE stream endpoint exists
grep -n 'stream_projection\|stream_telemetry' \
  crates/roko-serve/src/routes/projections.rs
```
