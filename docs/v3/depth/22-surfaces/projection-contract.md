# Depth: Projection Contract Types

> Parent: [22-SURFACES](../../22-SURFACES.md) SS1, SS3

This file documents the typed projection contract layer that backs all five named
surfaces: the concrete Rust types, how they are constructed from live StateHub and
durable feedback state, and the `ProjectionEnvelope<T>` metadata wrapper.

---

## 1. Envelope and Versioning

Every surface projection response is wrapped in `ProjectionEnvelope<T>`, defined in
`crates/roko-serve/src/projection_contract.rs`.

```rust
pub struct ProjectionEnvelope<T> {
    pub name: String,        // projection name, e.g. "workbench"
    pub version: u32,        // schema version, bumped on breaking changes
    pub cursor: u64,         // monotonic StateHub sequence number
    pub computed_at: String,  // RFC 3339 timestamp of snapshot computation
    pub recovered: bool,     // true = loaded from disk recovery, not live
    pub data: T,             // the typed projection body
}
```

The `version` field lets consumers reject or adapt to schema changes. The `cursor` field
provides a monotonic ordering for SSE delta streams. The `recovered` flag distinguishes
live event-driven state from disk-recovered state after a restart.

For non-surface projections, a broader `state_frame()` method produces an untyped JSON
wrapper with additional fields:

| Field | Description |
|---|---|
| `canonical_name` | Normalized projection name after alias resolution |
| `channel` | SSE channel identifier, e.g. `"projection:workbench"` |
| `freshness.state` | One of `"live"`, `"recovered"`, `"invalid"`, `"missing"` |
| `evidence` | Data quality summary from the RuntimeFeedbackProjection |

---

## 2. Workbench Projection

Constructed by `RuntimeProjectionSet::workbench_surface()`. Joins live dashboard
snapshot state with typed Lens projections.

### Construction

```
DashboardSnapshot.plans → flow_summaries() → Vec<FlowSummary>
DashboardSnapshot.agents + agent_vitality Lens → SlotState per agent
Typed Lens "gate_pipeline" or fallback from snapshot → GatePipelineProjection
Typed Lens "cost_meter" or fallback from snapshot → CostMeterProjection
```

### FlowSummary derivation

Flows are built from `DashboardSnapshot.plans` entries. For each active plan:
- `run_id` and `graph_name` come from the plan's `plan_id`
- `progress_pct` is derived from `tasks_done / tasks_total`
- `cost_usd` is the plan-level accumulated cost
- `elapsed` is computed from `started_at_ms` if present
- `status` maps plan state to `FlowStatus` enum
- `active_nodes` lists task IDs currently in-flight
- `pending_human` is populated when a task awaits human input

### SlotState derivation

Agents are sorted by `agent_id` and assigned sequential slot indices. Vitality is
looked up from the `AgentVitalityProjection` Lens data; agents not present in the
Lens default to 1.0 (active) or 0.0 (inactive).

---

## 3. Inbox Projection

Constructed by `RuntimeProjectionSet::inbox_surface()`. Reads materialized inbox
items from `DashboardSnapshot.inbox_items`.

### Item ordering

Items are sorted by urgency rank (Review > Question > Notify), then by timestamp
(newest first within the same urgency). The `inbox_urgency_rank()` helper assigns
numeric ranks for sorting.

### Timestamp handling

Inbox items store `received_at_ms` as a millisecond epoch. The projection converts
this to `DateTime<Utc>` via `timestamp_from_millis()`. Because the inbox event does
not carry a persisted timestamp field, unresolved items recompute their timestamps
from the raw millisecond value during JSONL replay -- a known product residual.

---

## 4. Canvas Projection

Constructed by `RuntimeProjectionSet::canvas_surface()`. Currently derives graph
identity from dashboard plan IDs rather than a dedicated GraphRegistry.

```rust
let graph_names = snapshot.plans.values()
    .map(|plan| plan.plan_id.clone())
    .collect();
```

The `graph_source` field is explicitly set to `"dashboard_plan_ids"` to document
this interim identity source. When a `GraphRegistry` is added to `AppState`, the
source will change and the field value will reflect the new source.

---

## 5. Minimap Projection

Constructed by `RuntimeProjectionSet::minimap_surface()`. Agent positions use a
deterministic grid layout:

```rust
// crates/roko-serve/src/projection_contract.rs
x: (index % 8) as f64,
y: (index / 8) as f64,
```

The `position_source` field is set to `"deterministic_layout"` to make this explicit.
The target design uses 2D projection of HDC space embeddings, but the infrastructure
for runtime HDC coordinate computation is not yet wired.

c-factor components are read from the typed `CFactorProjection` Lens or derived from
a fallback when Lens data is unavailable.

---

## 6. Autonomy Projection

Constructed by `RuntimeProjectionSet::autonomy_surface()`. Reads autonomy configs
from `RuntimeFeedbackProjection.autonomy_configs`, which in turn loads from
`.roko/state/autonomy.json` if the file exists.

When no autonomy configs are available:
- `configs` is an empty `Vec`
- `config_source` is `"unavailable"`

When configs exist:
- `config_source` is `"disk:.roko/state/autonomy.json"`

This explicit source field prevents consumers from mistaking an empty config list
for a default state.

---

## 7. RuntimeProjectionSet

The `RuntimeProjectionSet` struct is the canonical data source for all projection
handlers. It is constructed once per request via `RuntimeProjectionSet::load()`:

1. Bootstrap the StateHub from the working directory if not yet bootstrapped
2. Refresh recovered state from disk if no live events have been applied
3. Capture the cursor snapshot (atomic snapshot + sequence number)
4. Load durable feedback from `.roko/learn/` stores
5. Read provider health from the circuit-breaker registry
6. Read materialized Lens projections from StateHub

### Typed Lens access

The `typed_lens<T>()` helper deserializes a named Lens projection from the
`statehub_projections` BTreeMap. If the Lens data is unavailable or does not
deserialize, surface builders fall back to constructing the projection from
the raw `DashboardSnapshot` state.

---

## 8. Data Quality

The `DataQuality` struct is attached to projection responses that read from on-disk
telemetry. Consumers should check `has_real_data` before rendering charts or metrics.

```rust
pub struct DataQuality {
    pub has_real_data: bool,      // source file existed with >= 1 entry
    pub entry_count: usize,       // total entries read
    pub null_cost_count: usize,   // entries with zero or absent cost_usd
}
```
