# E09-T09: Telemetry-as-Lens Pipeline Design

> **Design document only.** No runtime Lens code is implemented in E09-T09.
> E13 consumes this document: E13-T01 defines the `Lens` trait and E13-T02 wraps
> `MetricRegistry` as the first `CollectorLens`. This document reconciles the live
> `MetricRegistry` and `StateHub` with the v2 Lens vocabulary and narrows the
> contract for those follow-up tasks.

---

## Current State

### What works today (post E09-T01..T08)

| Component | Reality after E09 | Location |
|---|---|---|
| `MetricRegistry` | Write-side owner of all metric values. Atomic counters, gauges, and histograms. Thread-safe via `parking_lot::RwLock` over `Vec<Family>`. | `crates/roko-core/src/obs/metrics.rs` |
| `render_prometheus()` | Serializes registry to Prometheus text format. Written to `.roko/metrics/prometheus.txt` at end of each runner-v2 plan run (E09-T02). | `MetricRegistry::render_prometheus` |
| `MetricRegistry::snapshot()` | Serializes registry to `Vec<MetricSnapshot>` — the JSON-friendly projection already consumed by `CollectorLens`. | `MetricRegistry::snapshot` |
| `StateHub` | Broadcast hub for `DashboardEvent`. Has optional `EventLogWriter` for `.roko/events.jsonl` persistence. Provides replay and live SSE/WS subscriptions. | `crates/roko-runtime/src/state_hub.rs` |
| `CollectorLens` | Read-side adapter wrapping `MetricRegistry`. Produces `LensSnapshot { metrics: Vec<MetricSnapshot>, .. }`. Implemented in E13-T02. | `crates/roko-core/src/obs/lens.rs` |
| Canonical metric names | `ROKO_GATE_VERDICTS_TOTAL`, `ROKO_LLM_CALLS_TOTAL`, `ROKO_TASKS_TOTAL`, etc. | `crates/roko-core/src/obs/schema.rs` |

### What does not exist yet

- No `TransformLens` type or trait implementation.
- No `ExportLens` type or trait implementation.
- No pipeline that carries `LensSnapshot` output into `StateHub` projections (the
  `DashboardSnapshot` fields are populated by direct `StateHub::publish(DashboardEvent)`
  calls from the runner, not by Lens output).
- No scheduled polling mechanism that drives `CollectorLens::snapshot()` and forwards
  results to any consumer.
- No v2 `Observe` protocol wired to the `Lens` trait (the spec describes `async fn observe`
  on the full `Observe: Cell` supertrait; the `Lens` trait in `obs/lens.rs` is the
  synchronous, `roko-core`-resident subset of that).

---

## Lens Pipeline

The v2 telemetry pipeline has three stages. Each stage is a named Lens kind that
implements the `Lens` trait defined in `crates/roko-core/src/obs/lens.rs`.

```
MetricRegistry (write side)
        │
        ▼
  CollectorLens          ← Stage 1: Collect
        │  LensSnapshot
        ▼
  TransformLens          ← Stage 2: Derive / aggregate
        │  LensSnapshot
        ▼
  ExportLens             ← Stage 3: Serialize / sink
        │
        ▼
StateHub / Prometheus / OTLP / disk
```

### Stage 1 — CollectorLens (already exists)

**Responsibility**: Gather raw metrics from a source without mutating it.

**Contract**:
- Wraps `Arc<MetricRegistry>`.
- `snapshot()` calls `MetricRegistry::snapshot()` and wraps the result in a `LensSnapshot`.
- Version is monotonically increasing per call; the registry's internal lock is held only
  during the `snapshot()` call.
- `scope()` is set at construction time to `LensScope::Global` for workspace-wide metrics
  or `LensScope::Agent(name)` for per-agent sidecars.

**No changes needed from E09-T09.** `CollectorLens` is already implemented. E13-T02
wires it to `StateHub` (the missing link described below under *Migration Plan*).

### Stage 2 — TransformLens (future, E13+)

**Responsibility**: Derive a secondary metric from one or more upstream `LensSnapshot`
values. Examples: trend direction (rising/falling cost over N samples), anomaly detection
(gate pass rate drops below threshold), rate computation (tokens per second from a
cumulative counter).

**Contract**:
- Holds `Vec<Arc<dyn Lens>>` as upstream sources.
- `snapshot()` calls `source.snapshot()` for each upstream, computes the derived value,
  and returns a new `LensSnapshot` containing only the derived metrics.
- Must not mutate upstream sources or the `MetricRegistry`.
- The `scope()` is the union of upstream scopes (or explicitly narrowed at construction).

**Deferred**: No `TransformLens` code is implemented in E09-T09.

### Stage 3 — ExportLens (future, E13+)

**Responsibility**: Serialize `LensSnapshot` output for an external sink. Examples:
`PrometheusExportLens` renders to Prometheus text format; `OtlpExportLens` pushes to an
OTLP endpoint; `StateHubExportLens` converts `MetricSnapshot` values to
`DashboardEvent::MetricUpdate` and calls `StateHub::publish`.

**Contract**:
- Wraps an upstream `Arc<dyn Lens>` (typically a `CollectorLens` or `TransformLens`).
- `snapshot()` drives the upstream snapshot then serializes/sinks it.
- Side effects (HTTP call, disk write, `StateHub::publish`) happen inside `snapshot()`.
- Failures are logged but never propagate — observability must never fail a run.
- The Lens trait's `snapshot()` is synchronous; async sinks must use `try_send` / channel
  offloading so the caller is never blocked.

**Deferred**: No `ExportLens` code is implemented in E09-T09. The Prometheus text dump
written by E09-T02 (`metrics.render_prometheus()` → `.roko/metrics/prometheus.txt`) is the
temporary equivalent until `PrometheusExportLens` is built.

---

## MetricRegistry Adapter

`MetricRegistry` is the canonical write-side store and must not be replaced. The v2 Lens
pipeline adapts it on the read side:

| Lens role | `MetricRegistry` interaction |
|---|---|
| `CollectorLens` | Calls `MetricRegistry::snapshot()` (read-lock, no mutation). |
| `TransformLens` | Reads only `CollectorLens` output (`LensSnapshot`); never touches `MetricRegistry` directly. |
| `ExportLens` | Reads only `CollectorLens` or `TransformLens` output; calls `MetricRegistry::render_prometheus()` for the Prometheus text sink only. |

### `MetricSnapshot` as the pipeline unit

`MetricSnapshot` (`crates/roko-core/src/obs/metrics.rs`) is the stable value type that
flows through the pipeline:

```rust
pub struct MetricSnapshot {
    pub name: String,
    pub help: String,
    pub kind: MetricKind,
    pub labels: Vec<(String, String)>,
    pub value: MetricValue,   // Counter(u64) | Gauge(i64) | Histogram(HistogramSnapshot)
}
```

`LensSnapshot.metrics: Vec<MetricSnapshot>` is already this type. No new wire format is
needed for Stage 1 or 2. Stage 3 (`ExportLens`) converts `MetricSnapshot` to the target
format (Prometheus text, OTLP proto, `DashboardEvent`) at the boundary.

### What must not change

- `MetricRegistry::register_counter/gauge/histogram` — write-side API is stable.
- `MetricRegistry::render_prometheus()` — used directly by E09-T02 for the post-run dump.
- `MetricRegistry::snapshot()` — the read-side method that `CollectorLens` calls.
- `Counter / Gauge / Histogram` types — atomic handles shared by multiple recording sites.

---

## StateHub Projection Contract

### Current wiring (post-E09)

`StateHub` receives `DashboardEvent` values from two sources:

1. The runner (`runner/event_loop.rs`): emits `DashboardEvent::TaskStarted`,
   `DashboardEvent::GateVerdict`, etc. directly via `StateHub::publish`.
2. The serve layer (`roko-serve/src/lib.rs`): maps `ServerEvent::FeedTick` and
   `ServerEvent::ChainBlock` into `DashboardEvent` before publish (firehose filtered by
   E09-T04).

`MetricRegistry` counters are **not** reflected in `StateHub` projections today. The
`DashboardSnapshot` does not contain metric values; it contains typed task/gate/run state.

### Target wiring (E13 deliverable)

`CollectorLens` snapshots should feed a new `DashboardEvent::MetricsSnapshot` variant (or
the nearest appropriate existing variant) so that the dashboard surfaces can display live
counter values without polling `/metrics` directly.

The canonical wiring point is `StateHubExportLens` (Stage 3, to be built in E13):

```
CollectorLens.snapshot()
    → Vec<MetricSnapshot>
    → DashboardEvent::MetricsSnapshot(Vec<MetricSnapshot>)
    → StateHub::publish(...)
    → TUI / SSE / WS consumers
```

### Polling cadence

The `Lens` trait is synchronous and pull-based. A background task (tokio `interval`) must
drive the poll loop. The recommended cadences mirror the spec (§15.7):

| Consumer | Poll interval |
|---|---|
| TUI sparklines | 1s (fast enough for human perception; registry reads are cheap) |
| SSE/WS dashboard | 1s |
| Low-bandwidth monitoring | 10s |
| Historical / audit | 60s |

The poll task is the responsibility of the embedding binary, not the `Lens` trait itself.
`roko-serve` spawns it alongside the StateHub; `roko-cli` does not need a persistent poll
task because it dumps the registry at end-of-run (E09-T02).

### Persistence contract

`DashboardEvent::MetricsSnapshot` events should **not** be persisted to `.roko/events.jsonl`.
Metric snapshots are point-in-time readings; replaying them is not useful for resume
diagnostics and they would reintroduce the volume problem fixed by E09-T04. The
`StateHub::EventLogWriter` persistence filter (added in E09-T04) must include
`DashboardEvent::MetricsSnapshot` in its skip set.

---

## Migration Plan

The path from today's state to the full v2 pipeline is incremental. No breaking changes
are required at any step.

### Step 0 (complete — E09-T01..T08)

- `MetricRegistry` threaded into `RunConfig.metrics` (E09-T01).
- Post-run Prometheus dump to `.roko/metrics/prometheus.txt` (E09-T02).
- Serve `AppState` registry threaded into serve-launched runner (E09-T03).
- FeedTick / ChainBlock filtered from `events.jsonl` (E09-T04).
- Log rotation added (E09-T05, E09-T06, E09-T07).
- `FsObservabilitySinks` attached to runner-v2 tool loop (E09-T08).

### Step 1 (E13-T01 — defines trait)

Define `trait Lens`, `LensScope`, and `LensSnapshot` in `crates/roko-core/src/obs/lens.rs`.
No concrete implementations. No consumers. Compiles cleanly.

**Already done** as part of the E13-T02 work that implemented `CollectorLens`. The trait
exists at `crates/roko-core/src/obs/lens.rs`. E13-T01's acceptance criteria are satisfied.

### Step 2 (E13-T02 — first concrete lens, already done)

`CollectorLens` wraps `MetricRegistry` and implements `Lens`. Tested. The read-side
adapter exists and is verified.

**Already done.** See `crates/roko-core/src/obs/lens.rs`.

### Step 3 (E13 follow-up — wire CollectorLens into StateHub)

The missing link: build `StateHubExportLens` (or inline the wiring in the serve layer)
so that a background task calls `CollectorLens::snapshot()` on a 1s interval and converts
the result to `DashboardEvent` variants that `StateHub::publish` accepts.

This task is **explicitly not in E09-T09** and belongs in E13 or a follow-up task after E13-T02.

### Step 4 (future — TransformLens)

Once `CollectorLens` feeds `StateHub`, build `TransformLens` variants for cost trend,
gate pass-rate anomaly, and token throughput derivation. These are pure functions over
`LensSnapshot` input and carry no external dependencies.

### Step 5 (future — OTLP ExportLens)

The `init_otlp_tracing` stub in `roko-serve/src/lib.rs` currently returns without
connecting. An `OtlpExportLens` would replace this stub and push `MetricSnapshot` values
to a configured OTLP endpoint on each poll cycle.

---

## Open Questions

1. **`DashboardEvent::MetricsSnapshot` variant**: Does `DashboardSnapshot` need a new
   field for metric values, or can the existing `run_metrics` / `telemetry` fields absorb
   them? Answer needed before E13 Step 3 begins. Prefer the smallest `DashboardSnapshot`
   change that allows the TUI telemetry tab (F5) to display live counters.

2. **Poll task ownership**: Should the `CollectorLens` poll loop live in `roko-serve`
   (alongside the existing `StateHub`), in `roko-runtime` (as a `ProcessSupervisor`
   subtask), or in `roko-cli` runner (as a tick-branch handler)? Recommendation: serve
   owns the poll loop for long-running `roko serve` sessions; the CLI runner uses the
   end-of-run dump only (E09-T02 output) and does not need a background poller.

3. **`TransformLens` source topology**: Should chained Lenses be assembled as a static
   `Vec<Arc<dyn Lens>>` at construction time, or should E13 introduce a `LensPipeline`
   builder that wires them declaratively from TOML config? TOML config is the v2 ideal
   but adds complexity; start with static construction and migrate later.

4. **Histogram projection in `DashboardEvent`**: `MetricSnapshot::Histogram` contains a
   `HistogramSnapshot { buckets, sum, count }`. Dashboard surfaces need summary statistics
   (p50, p99), not raw buckets. Should quantile computation happen in `TransformLens` or
   in the `StateHubExportLens` before emitting the `DashboardEvent`? Recommendation:
   `TransformLens` emits a `MetricSnapshot::Summary { p50, p95, p99 }` variant; the
   export lens forwards it as-is.

5. **Lens identity and deduplication**: If two callers construct independent
   `CollectorLens` instances wrapping the same `Arc<MetricRegistry>`, the registry is
   read twice per poll cycle. Is this acceptable, or should `CollectorLens` be a
   singleton per registry? Recommendation: acceptable for now; add a `LensRegistry`
   singleton wrapper only if profiling shows read-lock contention.
