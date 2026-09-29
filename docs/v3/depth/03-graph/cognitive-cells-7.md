# Seven Cognitive Cells

> The cognitive loop is a Hot Graph with 7 typed Cells executing sequentially
> each tick, modeling the agent's inner cognitive cycle. This file documents
> each cell, the T0 short-circuit optimization, calibration tracking, and
> the graph builder.

---

## Source File

`crates/roko-graph/src/cells/cognitive.rs`

---

## Cognitive Loop Topology

```
  [Sense] ---(t0_short_circuit == true)---> [React]     (T0 fast path: ~80% of ticks)
     |
     +---(t0_short_circuit == false)---> [Assess] --> [Compose] --> [Act] --> [Verify] --> [Persist] --> [React]
```

The graph is configured as Hot with `max_concurrent_nodes = 1` (strictly
sequential). React always runs -- it is the terminal node on both paths.

---

## Cell Inventory

| # | Cell | Struct | Protocol | ExecutionClass | Type Schema In | Type Schema Out |
|---|---|---|---|---|---|---|
| 1 | Sense | `SenseCell` | `Observe` | Workflow | -- | `AgentMessage` |
| 2 | Assess | `AssessCell` | `Score` | Workflow | `AgentMessage` | `AgentMessage` |
| 3 | Compose | `CognitiveComposeCell` | `Compose` | Workflow | `AgentMessage` | `Prompt` |
| 4 | Act | `ActCell` | `Connect` | Activity | `Prompt` | `Episode` |
| 5 | Verify | `VerifyCell` | `Verify` | Workflow | `Episode` | `GateVerdict` |
| 6 | Persist | `PersistCell` | `Store` | Workflow | `GateVerdict` | -- |
| 7 | React | `ReactCell` | `React` + `Trigger` | Workflow | -- | -- |

Only ActCell is classified as `Activity` (non-deterministic LLM dispatch).
All others are `Workflow` (deterministic, re-derived on replay).

---

## 1. SenseCell

**Protocol:** `Observe` (passive signal observation).

**Purpose:** Reads Signals from Store and Pulses from Bus. Detects whether
a full cognitive tick is needed or whether the T0 short-circuit applies.

**T0 Short-Circuit:** When all three conditions hold, SenseCell emits a
single Signal tagged `t0_short_circuit=true`:

1. **No actionable input** -- the input signal vector is empty.
2. **Not deadline-proximate** -- `budget_remaining` is above
   `DEADLINE_BUDGET_THRESHOLD_USD` ($0.05).
3. **No forced full tick** -- no upstream signal carries
   `force_full_tick=true`.

The `OutputEquals` conditional edge from Sense to React fires on
`t0_short_circuit=true`, skipping the expensive middle cells. This is the
common case (~80% of ticks in steady state).

**Short-circuit inhibition:** The T0 path is suppressed when:
- Budget drops below $0.05 -- forces a full pass to flush pending work.
- Previous ReactCell output requested `force_full_tick=true`.

**T0 counter:** `t0_count: Arc<AtomicU64>` tracks how many ticks were
short-circuited for dashboard reporting.

**Estimated cost:** $0.00 (no external calls).
**Estimated duration:** 5ms.

---

## 2. AssessCell

**Protocol:** `Score` (relevance scoring).

**Purpose:** Scores sensed Signals for relevance and priority. In the
current implementation, this is a passthrough -- real scoring would
assign relevance weights to incoming Signals.

**Calibration:** AssessCell implements the `predict()` / `correct()` cycle:

```rust
fn predict(&self, input: &[Signal]) -> Option<PredictionRecord> {
    // Predicts output_count will equal input.len()
}

fn calibration_error(&self, prediction: &PredictionRecord, actual: &[Signal]) -> Option<f64> {
    // Normalized error: |expected - observed| / max(expected, observed, 1.0)
}

fn correct(&self, prediction: &PredictionRecord, actual: &[Signal]) {
    // Records cumulative calibration error in atomic counters
}
```

The calibration counters use `AtomicU64` (error stored as millionths)
for lock-free concurrent observation from dashboard and Bus reporting.

**Estimated cost:** $0.001.
**Estimated duration:** 10ms.

---

## 3. CognitiveComposeCell

**Protocol:** `Compose` (prompt assembly).

**Purpose:** Assembles the system prompt from scored signals and context.
Transforms `AgentMessage` inputs into a `Prompt` output.

Named `CognitiveComposeCell` to avoid collision with `cells::compose::ComposeCell`
(which is the general-purpose compose cell).

**Estimated cost:** $0.005.
**Estimated duration:** 50ms.

---

## 4. ActCell

**Protocol:** `Connect` (external agent dispatch).

**Purpose:** Dispatches the composed prompt to an LLM agent and collects
the response. This is the only `Activity`-class cell in the cognitive loop
-- its outputs are recorded for replay.

Transforms `Prompt` input into `Episode` output.

**Estimated cost:** $0.10 (real LLM dispatch).
**Estimated duration:** 30 seconds.

---

## 5. VerifyCell

**Protocol:** `Verify` (gate execution).

**Purpose:** Runs the gate pipeline (compile, test, clippy, diff) against
the agent's output. Transforms `Episode` input into `GateVerdict` output.

**Estimated cost:** $0.00 (local commands).
**Estimated duration:** 60 seconds.

---

## 6. PersistCell

**Protocol:** `Store` (signal persistence).

**Purpose:** Writes verified outputs to the durable Signal store.
Accepts `GateVerdict` input; output schema is untyped (no downstream
type constraint).

**Estimated cost:** $0.00 (disk I/O only).
**Estimated duration:** 10ms.

---

## 7. ReactCell

**Protocols:** `React` (reactive policy) + `Trigger` (event-driven triggers).

**Purpose:** Emits lifecycle events, performs housekeeping, and optionally
requests a forced full tick for the next iteration. ReactCell always runs,
even on T0 short-circuit ticks.

ReactCell is the only cell that can set `force_full_tick=true` on its output
Signals to force the next tick to execute the full cognitive loop regardless
of sensed material.

**Tick counter:** `tick_counter: AtomicU64` tracks total ticks processed
(both T0 and full ticks) for observability.

**Estimated cost:** $0.00.
**Estimated duration:** 5ms.

---

## CalibrationTracker

`CalibrationTracker` provides online calibration tracking for any predictive
Cell:

```rust
pub struct CalibrationTracker {
    observations:            AtomicU64,   // total observations
    cumulative_error_micros: AtomicU64,   // sum of errors (1_000_000 = 1.0)
}
```

| Method | Purpose |
|---|---|
| `record(error)` | Record one observation; error clamped to [0.0, 1.0] |
| `observation_count()` | Total observations |
| `mean_error()` | Running mean calibration error, or `None` if no observations |

Errors are stored as millionths for atomic accumulation without
floating-point. The `mean_error()` computation:

```
mean = cumulative_error_micros / (observations * 1_000_000.0)
```

---

## Graph Builder: `build_cognitive_loop_graph`

```rust
pub fn build_cognitive_loop_graph(
    name: &str,
    tick_interval_ms: u64,
    max_ticks: Option<u64>,
) -> Graph
```

Builds a complete Hot Graph with:
- 7 nodes (sense, assess, compose, act, verify, persist, react).
- 7 edges:
  - Sense -> Assess (conditional: `t0_short_circuit == false`).
  - Assess -> Compose -> Act -> Verify -> Persist -> React (unconditional chain).
  - Sense -> React (conditional: `t0_short_circuit == true`).
- Policy: `GraphMode::Hot`, `FailureStrategy::FailFast`, `max_concurrent_nodes = 1`.
- Hot policy with `persist_tick_state = true`.

### Hot Policy Configuration

```rust
let hot_policy = HotPolicy {
    tick_interval_ms,           // e.g. 1000ms for Theta level
    max_ticks,                  // Some(100) or None for indefinite
    persist_tick_state: true,   // save per-node outputs between ticks
    loop_level: None,           // optional named level (Gamma/Theta/Delta)
};
```

---

## CoALA Connection

The 7 cognitive cells are informed by the CoALA (Cognitive Architectures
for Language Agents) 9-step cognitive cycle from Sumers et al. (2023,
arXiv:2309.02427). The Roko implementation collapses the 9 CoALA steps
into 7 cells, combining some phases:

| CoALA Step | Roko Cell |
|---|---|
| Observe | Sense |
| Retrieve | (folded into Sense and Assess) |
| Reason | Assess |
| Ground | Compose |
| Act | Act |
| Learn | (folded into Persist and React) |
| Plan | (handled by the outer plan topology, not the cognitive loop) |
| Reflect | Verify |
| Communicate | React |

---

## Verification

```bash
cargo test -p roko-graph --lib cells::cognitive::tests
```

Test coverage:
- T0 short-circuit fires on empty input.
- T0 counter increments on each short-circuit tick.
- T0 suppressed when input is non-empty.
- T0 suppressed when budget is near deadline threshold.
- T0 fires when budget is above threshold (sufficient).
- All 7 cells pass through input in stub form.
- Protocol declarations match expected values.
- AssessCell completes predict-correct calibration cycle.
- CalibrationTracker records, averages, and clamps.
- Graph builder produces correct topology (7 nodes, 7 edges).
- Graph builder sets sequential concurrency (`max_concurrent_nodes = 1`).
