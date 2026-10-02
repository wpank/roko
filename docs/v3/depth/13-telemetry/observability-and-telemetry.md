# Depth 13-06: Observability and Telemetry

> PeriodicObserver, JSONL rotation, metric schema, and the server-side
> telemetry persistence pipeline.

**Parent**: [13-TELEMETRY](../../13-TELEMETRY.md) -- Sections 7, 8

---

## 1. PeriodicObserver

The `PeriodicObserver` is the concrete `TelemetryObserve` implementation
that snapshots all registered Lenses on each observation cycle. It is
the bridge between the pull-based Lens model and the push-based
persistence pipeline.

**Source**: `crates/roko-core/src/obs/telemetry_observe.rs`

### 1.1 Observe Protocol

```rust
impl TelemetryObserve for PeriodicObserver {
    fn observe(&self, registry: &LensRegistry) -> Vec<TelemetryObservation> {
        let now = Utc::now();
        registry
            .snapshot_all()
            .into_iter()
            .map(|(name, snap)| TelemetryObservation {
                lens_name: name,
                timestamp: now,
                data: serde_json::to_value(&snap).unwrap_or(Value::Null),
            })
            .collect()
    }
}
```

Key properties:
- All Lenses in a single cycle share the same timestamp
- Failed serialization produces `Value::Null` rather than aborting
- The observer is stateless (`#[derive(Default, Clone)]`)

### 1.2 TelemetryObservation Record

```rust
pub struct TelemetryObservation {
    pub lens_name: String,
    pub timestamp: DateTime<Utc>,
    pub data: Value,
}
```

This is the persistence unit: one record per Lens per observation
cycle. The `data` field is an opaque JSON value so consumers do not
need to know the concrete Lens type.

---

## 2. Server-Side Observation Loop

The `start_periodic_telemetry_observer` function in
`crates/roko-serve/src/telemetry_observer.rs` creates the production
observation task. It runs for the lifetime of the server and is
terminated by the shared cancellation token during graceful shutdown.

### 2.1 Task Structure

```rust
pub(crate) fn start_periodic_telemetry_observer(
    state: &AppState,
) -> JoinHandle<()>
```

The function captures only cloneable lifecycle components. It does
not retain `AppState` to prevent forming an ownership cycle with the
server.

Captured components:
- `Arc<LensRegistry>` (the default three-lens registry)
- `Arc<DebugLogTelemetryObservationSink>` (logs each sample at debug level)
- `CancelToken` (server-lifetime cancellation)
- `Option<PathBuf>` (cost log path for spike detection)

### 2.2 Observation Interval

The default interval is 30 seconds (`DEFAULT_OBSERVATION_INTERVAL`).
The internal ticker uses `MissedTickBehavior::Skip` to avoid backlog
accumulation when observation takes longer than expected.

A zero interval is clamped to 1 millisecond to prevent panic from
`tokio::time::interval(Duration::ZERO)`.

### 2.3 Observation Cycle

Each cycle:

1. Wait for ticker or cancellation (biased toward cancellation)
2. Call `PeriodicObserver::observe(&registry)` to snapshot all Lenses
3. Hand the batch to the sink via `spawn_blocking`
4. Check cost spike threshold against 15-minute rolling rate

### 2.4 Error Handling

| Failure | Behavior |
|---------|----------|
| Sink emit fails | `tracing::warn` + continue |
| `spawn_blocking` panics | `tracing::warn` + continue |
| Cost spike check fails (file not found) | Silent (expected on first run) |
| Cost spike check fails (other I/O) | `tracing::debug` + continue |

Failures are best-effort and never affect request handling. The
observer is derived telemetry -- it is expendable.

---

## 3. Where Samples Go

Each sample goes to the debug log (`telemetry lens sample`, with the lens name and its data); serve keeps none on
disk. Serve used to append them to `.roko/metrics/telemetry-observations.jsonl`, which nothing read (backlog 2124).
An old file of that name is left in place.

---

## 4. Cost Spike Detection

The observation loop includes an integrated cost spike check:

```rust
const DEFAULT_COST_SPIKE_THRESHOLD: f64 = 1.0; // $1/min = $60/hr
```

On each cycle, the observer reads the `costs.jsonl` log and computes
the 15-minute rolling cost rate. If the rate exceeds the threshold,
a warning is logged:

```
WARN cost spike detected: 15-minute rolling cost rate exceeds threshold
  threshold_usd_per_min=1.0
```

This is a passive alert, not an enforcement mechanism. Budget
enforcement is handled separately by the Budget Lens and the ACP
budget system.

---

## 5. Canonical Metric Schema

The `CanonicalMetricSchema` in `crates/roko-core/src/obs/schema.rs`
defines 16 metric families shared across the core registry and
sidecars:

| Metric family | Kind | Labels |
|--------------|------|--------|
| `roko_plans_total` | Counter | `status` |
| `roko_tasks_total` | Counter | `status`, `role` |
| `roko_tool_calls_total` | Counter | `tool`, `outcome` |
| `roko_gate_verdicts_total` | Counter | `gate`, `verdict` |
| `roko_agent_duration_seconds` | Histogram | `backend`, `role` |
| `roko_llm_tokens_total` | Counter | `provider`, `model`, `direction` |
| `roko_llm_cost_usd_total` | Counter | `provider`, `model` |
| `roko_agent_server_requests_total` | Counter | -- |
| `roko_agent_server_message_requests_total` | Counter | -- |
| `roko_llm_calls_total` | Counter | `provider`, `model`, `status` |
| `roko_llm_errors_total` | Counter | `provider`, `model`, `error_type` |
| `roko_llm_ttft_seconds` | Histogram | `provider`, `model` |
| `roko_llm_request_duration_seconds` | Histogram | `provider`, `model` |
| `roko_context_utilization` | Gauge | `provider`, `model` |
| `roko_token_throughput_per_second` | Gauge | `provider`, `model` |
| `roko_target_dir_size_bytes` | Gauge | -- |

### 5.1 MetricDescriptor

Each entry is a static `MetricDescriptor`:

```rust
pub struct MetricDescriptor {
    pub name: &'static str,
    pub help: &'static str,
    pub kind: MetricKind,
    pub labels: &'static [&'static str],
}
```

The `MetricSchema` trait provides versioned access:

```rust
pub trait MetricSchema {
    fn schema_version() -> u32;
    fn metrics() -> &'static [MetricDescriptor];
}
```

Current schema version: 1.

### 5.2 Standard Labels

11 label constants are defined for consistent naming:

`status`, `role`, `tool`, `outcome`, `gate`, `verdict`, `backend`,
`provider`, `model`, `direction`, `error_type`

---

## 6. Default Lens Registry

The `default_registry` function creates a `LensRegistry` with three
pre-registered metric-oriented Lenses:

| Name | Lens type | Filtered metrics |
|------|-----------|-----------------|
| `"token-usage"` | `TokenUsageLens` | `roko_llm_tokens_total` |
| `"latency"` | `LatencyLens` | `roko_llm_ttft_seconds`, `roko_llm_request_duration_seconds` |
| `"cost"` | `CostLens` | `roko_llm_cost_usd_total` |

All three are scoped to `LensScope::Global`.

---

## 7. Graceful Shutdown

The observation task respects the server's `CancelToken`:

```rust
tokio::select! {
    biased;
    _ = cancel.cancelled() => break,
    _ = ticker.tick() => {}
}
```

The `biased` mode ensures cancellation takes priority over pending
ticks. After cancellation, the task drops its `Arc` references,
releasing the `LensRegistry` and sink. Tests verify that these
references are fully released after shutdown.

---

## 8. Source References

| File | What it contains |
|------|------------------|
| `crates/roko-core/src/obs/telemetry_observe.rs` | `TelemetryObserve` trait, `TelemetryObservation`, `PeriodicObserver` |
| `crates/roko-core/src/obs/lens.rs` | `LensRegistry`, `default_registry`, concrete Lens impls |
| `crates/roko-core/src/obs/schema.rs` | `CanonicalMetricSchema`, 16 `MetricDescriptor` entries, label constants |
| `crates/roko-serve/src/telemetry_observer.rs` | `start_periodic_telemetry_observer`, debug-log sink, cost spike detection |

---

## Verification

```bash
# PeriodicObserver snapshots all default Lenses
cargo test -p roko-core periodic_observer_snapshots_all_default_lenses

# Observations have timestamps and valid JSON
cargo test -p roko-core periodic_observer_observations_have_timestamps
cargo test -p roko-core periodic_observer_data_is_valid_json

# JSONL round-trip
cargo test -p roko-core telemetry_observation_roundtrips_via_json

# Server observer emits initial snapshot and shuts down cleanly
cargo test -p roko-serve periodic_observer_emits_all_lens_snapshots
cargo test -p roko-serve cancellation_stops_waiting_task_and_releases_captured_state
cargo test -p roko-serve run_server_with_state_persists_no_telemetry_and_shuts_down

# Canonical schema size
cargo test -p roko-core canonical_metrics_count_is_15
cargo test -p roko-core descriptor_labels_are_valid
```
