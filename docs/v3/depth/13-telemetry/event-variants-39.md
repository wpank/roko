# Depth 13-04: Event Variants (39)

> All 39 ObservableEvent variants organized by lifecycle family,
> with their fields, source scope resolution, and duration semantics.

**Parent**: [13-TELEMETRY](../../13-TELEMETRY.md) -- Section 2

---

## 1. Event Families

The `ObservableEvent` enum in `crates/roko-core/src/telemetry_observe.rs`
defines 39 variants across 8 lifecycle families:

| Family | Variants | Filter constant |
|--------|----------|-----------------|
| Signal | 8 | `ObservableEventKind::SignalLifecycle` |
| Cell | 7 | `ObservableEventKind::CellLifecycle` |
| Graph | 6 | `ObservableEventKind::GraphLifecycle` |
| Agent | 7 | `ObservableEventKind::AgentLifecycle` |
| Memory | 4 | `ObservableEventKind::MemoryLifecycle` |
| Verify | 2 | `ObservableEventKind::VerifyLifecycle` |
| Trigger | 3 | `ObservableEventKind::TriggerLifecycle` |
| Extension | 2 | `ObservableEventKind::ExtensionLifecycle` |

The `All` filter constant accepts every event from every family.

---

## 2. Signal Lifecycle (8)

| Variant | Fields | Source scope |
|---------|--------|--------------|
| `SignalCreated` | `Signal` | `Global` |
| `SignalScored` | signal_id, scorer | `Global` |
| `SignalRouted` | signal_id, route | `Global` |
| `SignalVerified` | signal_id, `Verdict` | `Global` |
| `SignalComposed` | `Vec<String>` inputs, output `Signal` | `Global` |
| `SignalDemurrageApplied` | signal_id, amount | `Global` |
| `SignalPromoted` | signal_id, old_tier, new_tier | `Global` |
| `SignalPruned` | signal_id | `Global` |

All Signal events resolve to `LensScope::Global` because Signals are
not bound to a specific Cell, Graph, or Agent in the event payload.

---

## 3. Cell Lifecycle (7)

| Variant | Fields | Source scope | Has duration |
|---------|--------|--------------|:------------:|
| `CellStarted` | block, run, input_hash | `Cell(block)` | No |
| `CellCompleted` | block, run, duration_ms, cost_usd | `Cell(block)` | Yes |
| `CellFailed` | block, run, error | `Cell(block)` | No |
| `CellRetried` | block, run, attempt, reason | `Cell(block)` | No |
| `CellCancelled` | block, run | `Cell(block)` | No |
| `CellPredictionPublished` | block, prediction | `Cell(block)` | No |
| `CellCalibrationReceived` | block, error | `Cell(block)` | No |

All Cell events resolve to `LensScope::Cell(block.clone())`.

`CellCompleted` is the primary duration-bearing Cell event. The
`observed_duration_ms()` method returns `Some(duration_ms)` for this
variant and `None` for the others. This duration is the denominator
used by the overhead circuit breaker.

---

## 4. Graph Lifecycle (6)

| Variant | Fields | Source scope | Has duration |
|---------|--------|--------------|:------------:|
| `GraphStarted` | graph, run, input_hash | `Graph(graph)` | No |
| `GraphNodeCompleted` | graph, run, node, duration_ms | `Graph(graph)` | Yes |
| `GraphCompleted` | graph, run, duration_ms, cost_usd | `Graph(graph)` | Yes |
| `GraphFailed` | graph, run, error | `Graph(graph)` | No |
| `GraphPaused` | graph, run, reason | `Graph(graph)` | No |
| `GraphResumed` | graph, run | `Graph(graph)` | No |

`GraphNodeCompleted` and `GraphCompleted` both carry `duration_ms`.
The node-level event provides per-step granularity; the graph-level
event provides the total run duration.

---

## 5. Agent Lifecycle (7)

| Variant | Fields | Source scope |
|---------|--------|--------------|
| `AgentTick` | agent, regime, prediction_error, vitality | `Agent(agent)` |
| `AgentRegimeChange` | agent, old, new_regime | `Agent(agent)` |
| `AgentBudgetUpdate` | agent, spent_usd, remaining_usd, vitality | `Agent(agent)` |
| `AgentModeChange` | agent, old, new_mode | `Agent(agent)` |
| `AgentPhaseChange` | agent, old, new_phase | `Agent(agent)` |
| `AgentStateTransition` | agent, old, new_state | `Agent(agent)` |
| `AgentSlotUpdate` | agent, slot, state | `Agent(agent)` |

No Agent event carries a `duration_ms` field. Agent vitality and
prediction error are carried as scalar values in the `AgentTick`
event, emitted on each cognitive tick.

---

## 6. Memory Lifecycle (4)

| Variant | Fields | Source scope | Has duration |
|---------|--------|--------------|:------------:|
| `MemoryRetrieved` | query, results, duration_ms | `Global` | Yes |
| `MemoryStored` | signal, tier | `Global` | No |
| `MemoryConsolidated` | promoted, demoted, pruned | `Global` | No |
| `DemurrageApplied` | count, total_balance_lost | `Global` | No |

All Memory events resolve to `LensScope::Global`. The
`MemoryRetrieved` event carries `duration_ms` for retrieval latency
tracking.

---

## 7. Verify Lifecycle (2)

| Variant | Fields | Source scope |
|---------|--------|--------------|
| `VerifyPreResult` | block, verdict, evidence | `Cell(block)` |
| `VerifyPostResult` | block, verdict, reward, evidence | `Cell(block)` |

Both resolve to `LensScope::Cell(block.clone())`. `VerifyPostResult`
additionally carries the `reward` scalar used by learning.

---

## 8. Trigger Lifecycle (3)

| Variant | Fields | Source scope |
|---------|--------|--------------|
| `TriggerFired` | trigger, graph | `Graph(graph)` |
| `TriggerArmed` | trigger | `Global` |
| `TriggerDisarmed` | trigger | `Global` |

`TriggerFired` resolves to `LensScope::Graph(graph.clone())` because
it names the Graph that was triggered. The other two resolve to
`Global`.

---

## 9. Extension Lifecycle (2)

| Variant | Fields | Source scope | Has duration |
|---------|--------|--------------|:------------:|
| `ExtensionHookCalled` | extension, hook, layer, duration_ms | `Global` | Yes |
| `ExtensionHookFailed` | extension, hook, error | `Global` | No |

Both resolve to `Global`. `ExtensionHookCalled` carries `duration_ms`
for hook latency tracking.

---

## 10. Duration-Bearing Events

Five variants return `Some(duration_ms)` from `observed_duration_ms()`:

1. `CellCompleted`
2. `GraphNodeCompleted`
3. `GraphCompleted`
4. `MemoryRetrieved`
5. `ExtensionHookCalled`

This value is the denominator for the overhead circuit breaker:
`budget_ms = cell_duration_ms * budget_pct`. Events without a
duration return `None` and do not participate in overhead checking.

---

## 11. Event Filtering

### 11.1 Kind Matching

The `ObservableEventKind::matches` method provides fast pre-filtering:

```rust
pub fn matches(self, event: &ObservableEvent) -> bool {
    self == Self::All || self == event.kind()
}
```

### 11.2 Multi-Filter Matching

`ObservableEvent::matches_any` accepts a list of filters. An empty
filter list observes nothing:

```rust
pub fn matches_any(&self, filters: &[ObservableEventKind]) -> bool {
    filters.iter().any(|filter| filter.matches(self))
}
```

---

## 12. Serde and Transport

All `ObservableEvent` variants derive `Serialize` and `Deserialize`.
The enum uses default serde tagging (externally tagged) so each
variant serializes with its variant name as the JSON key. String fields
use portable `String` rather than runtime-specific reference types to
support cross-process transport.

---

## Source References

| File | What it contains |
|------|------------------|
| `crates/roko-core/src/telemetry_observe.rs` | Complete `ObservableEvent` enum (39 variants), `ObservableEventKind` (8 families + `All`), `source_scope()`, `observed_duration_ms()`, `matches_any()` |

---

## Verification

```bash
# Count enum variants
grep -c 'SignalCreated\|SignalScored\|SignalRouted\|SignalVerified\|SignalComposed\|SignalDemurrageApplied\|SignalPromoted\|SignalPruned\|CellStarted\|CellCompleted\|CellFailed\|CellRetried\|CellCancelled\|CellPredictionPublished\|CellCalibrationReceived\|GraphStarted\|GraphNodeCompleted\|GraphCompleted\|GraphFailed\|GraphPaused\|GraphResumed\|AgentTick\|AgentRegimeChange\|AgentBudgetUpdate\|AgentModeChange\|AgentPhaseChange\|AgentStateTransition\|AgentSlotUpdate\|MemoryRetrieved\|MemoryStored\|MemoryConsolidated\|DemurrageApplied\|VerifyPreResult\|VerifyPostResult\|TriggerFired\|TriggerArmed\|TriggerDisarmed\|ExtensionHookCalled\|ExtensionHookFailed' \
  crates/roko-core/src/telemetry_observe.rs

# Verify all 8 kind() match arms
cargo test -p roko-core -- telemetry_observe
```
