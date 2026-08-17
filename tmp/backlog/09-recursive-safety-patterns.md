# Backlog: Recursive Safety Continuous Monitoring

**Status**: Backlog
**Priority**: P3 (Phase 2+)
**Size**: L (4-5 days)
**Origin**: `tmp/architecture-archive/13-meta.md`

---

## Problem Statement

The existing `RecursiveSafetyMonitor` in `crates/roko-agent/src/safety/recursive.rs` is a bounded, per-activation validator. It runs the canonical five-head corrigibility graph at each `validate_activation` and `validate_action` call, enforcing non-widening delegation and depth/fanout/cost bounds at the moment of agent creation or role morph. What it does **not** do is run continuously to detect emergent patterns across the lifetime of a recursive lineage.

Specifically, the present implementation has no runtime visibility into:

- **Quality drift across generations**: whether child agents are producing progressively worse outputs over time (quality degradation trend).
- **Rate spikes**: whether a meta-agent's child-creation rate is accelerating toward its configured `MAX_META_AGENT_FANOUT` in a burst pattern that would exceed a sensible hourly window before any individual activation is denied.
- **Circular lineage**: whether two agents in a lineage tree have inadvertently formed a cycle through forking or role-morph operations.
- **Global backstop**: whether the system-wide aggregate meta-agent creation rate has exceeded a configurable threshold that no single agent's per-activation check would catch.

The architecture document specifies a `RecursiveSafetyMonitor` struct with a `scan()` method that operates over a live `Vec<RecursiveProcess>` snapshot. That fuller design — the continuous monitoring loop, `SafetyAnomaly` classification, and recommended `SafetyAction` (Log / Pause / Quarantine / Terminate) — is not yet implemented.

Note: the static per-activation enforcement (R04) is complete and passing. This item is the live, continuous monitoring overlay on top of that foundation.

---

## Proposed Solution

Add a background monitoring task to the runner event loop and agent serve runtime that periodically scans active recursive lineages for anomalies and emits typed events when any are detected.

### Key components

**`RecursiveProcess` snapshot record** — a lightweight serializable struct capturing, per active meta-agent lineage, the current creation count, rolling hourly rate, per-generation quality scores, and lineage DAG edges. The runner already tracks spawned child processes in `ProcessSupervisor`; the snapshot record maps directly onto that existing registry.

**`RecursiveSafetyMonitor::scan()`** — a pure function over `&[RecursiveProcess]` that returns `Vec<SafetyAnomaly>`. The anomaly variants are:

```rust
pub enum SafetyAnomaly {
    RateLimitViolation   { meta_agent_id: String, rate: u32, limit: u32 },
    QualityDegradation   { meta_agent_id: String, generation: u32, quality_trend: Vec<f64>, slope: f64 },
    CircularDependency   { agents: Vec<String> },
    GlobalRateExceeded   { current_rate: u32, limit: u32 },
    CaveatEscalation     { meta_agent_id: String, attempted_caveat: String },
}
```

**`SafetyAction` recommendation** — `scan()` returns anomalies; a separate `recommend_action()` maps each anomaly to Log / Pause / Quarantine / Terminate. Default policy is `Log` for warning-level anomalies and `Pause` for critical ones. `auto_pause_on_anomaly` config flag enables automatic execution of the recommended action.

**Background monitor task** — a tokio task spawned in `roko-serve`'s runtime and optionally in the plan runner, polling at a configurable interval (default: 30s). Anomalies are published as `DashboardEvent::RecursiveSafetyAnomaly` (new variant) so they surface in the TUI and HTTP event stream.

**Configuration** in `roko.toml`:

```toml
[meta.safety]
quality_trend_window    = 10
min_quality_slope       = -0.05   # flag if quality drops >5% per generation
circular_detection      = true
auto_pause_on_anomaly   = false
global_max_rate_per_hour = 50
monitor_interval_secs   = 30
```

### What is explicitly out of scope

- On-chain lineage recording (separate Phase 2+ connector work tracked in `.roko/GAPS.md`).
- `LineageService` and the full `LineageGraph` visualization API — those belong in a dedicated lineage-browser feature.
- The generator sub-system (`GeneratorConfig`, `GeneratorOutput`, schema validation).

---

## Implementation Location

| Component | Path |
|---|---|
| Anomaly types + `scan()` | `crates/roko-agent/src/safety/recursive.rs` (extend existing module) |
| Background monitor task | `crates/roko-serve/src/runtime.rs` |
| Runner integration | `crates/roko-cli/src/runner/event_loop.rs` |
| `DashboardEvent` variant | `crates/roko-core/src/dashboard_snapshot.rs` |
| Config schema | `crates/roko-core/src/config/schema.rs` (`MetaSafetyConfig`) |
| Tests | `crates/roko-agent/src/safety/recursive.rs` (#[cfg(test)]) |

The existing `RecursiveSafetyMonitor` struct at line 183 of `recursive.rs` is the correct attachment point. The `scan()` and `recommend_action()` methods are pure additions; no existing signatures change.

---

## Acceptance Criteria

1. `RecursiveSafetyMonitor::scan()` correctly identifies `RateLimitViolation` when a `RecursiveProcess` snapshot shows `creations_this_hour > max_creations_per_hour`, and returns `SafetyAnomaly::GlobalRateExceeded` when the sum across all processes exceeds `global_max_rate_per_hour`.

2. `scan()` correctly identifies `QualityDegradation` when the per-generation quality score series has a negative linear slope steeper than `min_quality_slope` over the configured `quality_trend_window`.

3. `scan()` detects `CircularDependency` when the lineage DAG extracted from the process snapshots contains a cycle (verified via DFS, no false positives on valid DAGs).

4. The background monitor task in `roko-serve` emits a `DashboardEvent::RecursiveSafetyAnomaly` for each detected anomaly, which the TUI event log displays, and the `/api/events` SSE stream includes.

5. With `auto_pause_on_anomaly = true`, a `Pause`-level recommendation causes the offending meta-agent to be sent a stop signal via `ProcessSupervisor`, and the incident is appended to `.roko/learn/recursive-safety.jsonl`.

6. All five existing `RecursiveSafetyMonitor` test cases (delegation rejection, canonical veto) continue to pass without modification. New unit tests cover each `SafetyAnomaly` variant.

---

## References

- Source spec: `/Users/will/dev/nunchi/roko/roko/tmp/architecture-archive/13-meta.md` (Recursive safety section)
- Existing implementation: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/recursive.rs`
- Lifecycle constants: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/lifecycle.rs` (`MAX_META_AGENT_DEPTH`, `MAX_META_AGENT_FANOUT`, `MAX_META_AGENT_LINEAGE_COST_USD`)
- Corrigibility graph: `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/cells/cognitive.rs`
- Dashboard events: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs`
- GAPS entry: `/Users/will/dev/nunchi/roko/roko/\.roko/GAPS.md` ("bounded meta-agent lineage/recursive safety" residual)
