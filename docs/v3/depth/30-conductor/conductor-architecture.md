# Conductor Architecture -- L3 Harness and Evaluation Flow

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 1.
> Source: `crates/roko-conductor/src/conductor.rs`, `crates/roko-conductor/src/lib.rs`

---

## 1. Position in the Five-Layer Architecture

Roko's runtime stacks into five layers. Each layer has a distinct responsibility
boundary:

| Layer | Name | What It Owns | Key Traits |
|-------|------|-------------|------------|
| L0 | Runtime | Processes, I/O, OS-level lifecycle | `Substrate` |
| L1 | Framework | Tool definitions, agent capabilities | (tools API) |
| L2 | Scaffold | Prompt construction, context engineering | `Composer` |
| **L3** | **Harness** | **Output evaluation, meta-cognition** | **`Gate`, `Policy`** |
| L4 | Orchestration | Multi-agent scheduling, DAG execution | `Router`, `Scheduler` |

The Conductor sits at **Layer 3 -- Harness**. It shares this layer with the gate
pipeline (compile, test, clippy, diff, coverage, spec, etc.) but serves a
fundamentally different function:

- **Gates** answer: did the output meet the acceptance criteria?
- **Conductor** answers: is the process itself healthy?

Gates evaluate artifacts. The Conductor evaluates trajectories.

This distinction matters because a plan can pass every individual gate and still be
pathological -- looping through identical implement-gate cycles, burning tokens on
ghost turns, or drifting outside its declared file scope without any single gate
catching it.

---

## 2. Kernel Placement -- Composite Policy

Roko's kernel defines one noun (`Signal`) and twelve verb traits. The Conductor is a
**composite `Policy`** -- a reactive stream evaluator. Every watcher implements the
`React` trait. The Conductor delegates to its inner watchers and aggregates their
outputs through an intervention policy.

```rust
// From crates/roko-conductor/src/conductor.rs
pub struct Conductor {
    watchers: Vec<Box<dyn React>>,
    policy: Box<dyn InterventionPolicy>,
    circuit_breaker: CircuitBreaker,
}
```

The composability means the Conductor can be used anywhere a reactive evaluator is
expected: inside the orchestrator's main loop, as a standalone evaluation pass, or
nested inside a larger policy composition.

---

## 3. What the Conductor Is Not

Understanding the Conductor requires understanding what it deliberately does not do.

**It is not a scheduler.** The Conductor does not decide which task runs next, which
agent gets spawned, or how resources are allocated. That is L4 (Orchestration). The
Conductor evaluates whether the current execution trajectory is healthy and emits
signals when it is not.

**It is not a gate.** Gates produce binary verdicts (pass/fail) on artifacts. The
Conductor produces graduated interventions (continue / restart / fail) on processes.
A gate looks at the code; the Conductor looks at the agent producing the code.

**It is not a timeout manager.** Timeouts are one of twelve watcher categories. The
Conductor's scope includes loop detection, cost monitoring, context pressure
tracking, spec drift measurement, test regression detection, and review cycle
analysis. Reducing it to "timeouts" misses 90% of its function.

**It does not nudge.** A nudge is "please fix yourself" -- which does not work on
confused agents. The Conductor has exactly three actions: Continue (everything is
fine), Restart (kill and restart with different context), or Fail (mark the plan as
failed). There is no "try harder." This is a deliberate design decision derived from
production experience: agents that are stuck remain stuck after nudges (Hard
Guarantee 6 from the failure prevention catalog).

---

## 4. Core Components

The Conductor comprises seven subsystems, each in its own module:

### 4.1 Watcher Ensemble (`watchers/`)

Twelve watchers, each implementing `React`. Each watcher monitors a specific failure
mode by examining the signal stream. See `watcher-ensemble-12.md` for full catalog.

### 4.2 Circuit Breaker (`circuit_breaker.rs`)

Per-plan failure budget tracking. Uses `DashMap` for thread-safe concurrent access.
A plan that accumulates `MAX_PLAN_FAILURES` (default 2) failures is permanently
tripped -- no further retries. See `circuit-breaker.md`.

### 4.3 Intervention Policy (`interventions.rs`)

Maps watcher outputs to conductor decisions through a severity system:

```
Info     -> ConductorDecision::Continue
Warning  -> ConductorDecision::Restart
Critical -> ConductorDecision::Fail
```

The default policy is `WorstSeverityPolicy`: the highest severity among all watcher
outputs determines the decision. See `graduated-interventions.md`.

### 4.4 Diagnosis Engine (`diagnosis.rs`)

Thirty-four built-in error patterns covering twenty error categories. Given raw error
output, the diagnosis engine classifies the error, assigns a confidence score, and
suggests an intervention. See `diagnosis-engine.md`.

### 4.5 Stuck Detection (`stuck_detection.rs`)

Six heuristics for detecting stuck agents: OutputLoop, NoProgress, GateLoop,
CompileLoop, EmptyOutput, ExcessiveRetries. The `MetaCognitionHook` wraps them for
periodic self-assessment at Theta frequency. See `stuck-detection.md`.

### 4.6 Health Monitor (`health.rs`)

Four system-level health checks producing a `HealthStatus` (Healthy / Degraded /
Critical): terminal liveness, agent status, spec drift, coverage trend. See
`health-monitors.md`.

### 4.7 State Machine (`state_machine.rs`)

Phase timeout configuration by plan complexity. `PhaseTransition` records capture
the plan ID, source phase, target phase, timestamp, and reason -- providing a
complete audit trail of every plan's progression through the pipeline. See
`adaptive-timeouts-state-machine.md`.

---

## 5. Evaluation Flow

When the orchestrator calls `conductor.evaluate()`, the following sequence executes:

```
1. Circuit breaker check
   +-- If plan is tripped -> return Fail immediately

2. Run all 12 watchers against the signal stream
   +-- Each watcher returns Vec<Signal> (empty = healthy)
   +-- Collect all non-empty results as WatcherOutputs

3. Apply intervention policy
   +-- WorstSeverityPolicy: max(all severities) -> decision
   +-- Info -> Continue, Warning -> Restart, Critical -> Fail

4. If decision is Restart or Fail:
   +-- Record failure in circuit breaker
   +-- Emit intervention signal to stream

5. Return ConductorDecision
```

The entire evaluation is stateless from the Conductor's perspective -- it reads the
signal stream and produces a decision. State tracking (failure counts, circuit
breaker trips) lives in the `CircuitBreaker`, which uses thread-safe `DashMap` for
concurrent access.

### 5.1 Evaluation Latency

| Component | Typical Latency |
|-----------|----------------|
| Circuit breaker check | < 1 us (DashMap lookup) |
| All 12 watchers | < 1 ms (stream scan, no I/O) |
| Intervention policy | < 1 us (max comparison) |
| Signal emission | < 10 us (signal construction) |
| **Total** | **< 2 ms** |

This latency is negligible compared to agent turn times (seconds to minutes) and
gate execution times (seconds to minutes). The conductor evaluation is never the
bottleneck.

---

## 6. Signal Flow

The Conductor communicates exclusively through signals. It reads `Signal` instances
from the stream and writes `Signal` instances back.

**Input signals consumed:**

| Kind | What the Conductor Reads |
|------|------------------------|
| `TokenUsage` | Token counts for context pressure |
| `GateVerdict` | Test results for failure budget |
| `AgentOutput` | Output content for ghost turn / stuck detection |
| `PlanPhase` | Phase events for review loop tracking |
| `Metric` (name=spec_drift) | Drift ratios for spec drift |
| `Custom("conductor.agent_output")` | Timing data for time overrun |

**Output signals emitted:**

| Kind | When |
|------|------|
| `Custom("conductor.intervention")` | Any watcher fires |

Intervention signals carry tags: `watcher` (which watcher fired), `severity`
(info/warning/critical), and watcher-specific metadata (ratio, count, plan_id,
task_id, etc.).

---

## 7. Design Decisions

### 7.1 Why Watchers Are Policies, Not Gates

Gates produce binary verdicts. Policies produce signals with graduated severity. The
Conductor needs graduation because not every anomaly warrants the same response:

- Context window at 82% -- warning (restart with compacted context)
- Spec drift at 30% -- warning (the agent is exploring nearby files)
- Three identical compile errors -- critical (the agent is stuck)

A gate would reduce all of these to "fail," losing the information needed for
appropriate response.

### 7.2 Why Twelve Watchers Instead of One Smart Monitor

Each watcher is a focused detector for one failure mode. This decomposition
provides:

1. **Testability** -- each watcher has isolated unit tests
2. **Configurability** -- thresholds are per-watcher
3. **Composability** -- add or remove watchers without touching others
4. **Diagnosability** -- the intervention signal says which watcher fired

A monolithic monitor would conflate detection with diagnosis. By keeping watchers
separate, the system can tell you not just "something is wrong" but "the agent has
produced three identical compile errors" -- a much more actionable signal.

### 7.3 Why the Circuit Breaker is Per-Plan

Plans are the unit of retry. A failing plan should not poison other plans. The
circuit breaker tracks failures per plan ID, so plan A hitting its failure budget
does not affect plan B. The `DashMap` provides thread-safe concurrent access because
the orchestrator may evaluate multiple plans in parallel.

---

## 8. References

- Conant & Ashby (1970) -- "Every good regulator of a system must be a model of
  that system." The Conductor models the pipeline's health.
- Beer (1972) -- Viable System Model, System 3 (internal oversight) + System 3*
  (audit). The Conductor fills both roles.
- Boyd (1987) -- OODA loop (Observe-Orient-Decide-Act). Each conductor evaluation
  cycle is one OODA iteration.

---

## 9. File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/lib.rs` | Module structure, re-exports |
| `crates/roko-conductor/src/conductor.rs` | Conductor struct, evaluate(), React impl |
| `crates/roko-conductor/src/circuit_breaker.rs` | Per-plan failure tracking |
| `crates/roko-conductor/src/interventions.rs` | Severity, WatcherOutput, InterventionPolicy |
| `crates/roko-conductor/src/diagnosis.rs` | 34 error patterns, 20 categories |
| `crates/roko-conductor/src/health.rs` | SystemSnapshot, 4 health checks |
| `crates/roko-conductor/src/state_machine.rs` | Phase timeouts, PhaseTransition records |
| `crates/roko-conductor/src/stuck_detection.rs` | 6 stuck heuristics, MetaCognitionHook |
| `crates/roko-conductor/src/watchers/` | 12 watcher modules |
