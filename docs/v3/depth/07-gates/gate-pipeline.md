# Gate Pipeline Execution Flow

> Depth file for [07-GATES.md](../../07-GATES.md) section 5 (dispatch) and v1 03.
> Source: `crates/roko-gate/src/gate_pipeline.rs`, `crates/roko-gate/src/rung_dispatch.rs`

---

## 1. Overview

The `GatePipeline` composes multiple gates into a single verification step.
It accepts a `Vec<Box<dyn Gate>>`, runs them sequentially, and produces an
aggregated `Verdict`. It implements the `Gate` trait itself, so a pipeline
can be nested inside another pipeline or used anywhere a single gate is
expected.

---

## 2. Structure

```rust
pub struct GatePipeline {
    gates: Vec<Box<dyn Gate>>,
    short_circuit: bool,
    name: String,
}
```

| Field | Purpose |
|-------|---------|
| `gates` | Ordered list of gates to execute |
| `short_circuit` | If true, stop on first failure |
| `name` | Display name for the pipeline's own verdict |

### Construction

```rust
GatePipeline::new(vec![
    Box::new(CompileGate::cargo()),
    Box::new(ClippyGate::cargo()),
    Box::new(TestGate::cargo()),
])
.with_short_circuit(true)
.with_name("rung-pipeline")
```

---

## 3. Short-Circuit vs Full Execution

### 3.1 Short-Circuit Mode (default)

The pipeline stops at the first gate that fails and returns a failure
verdict immediately. This is the correct behavior for the rung pipeline:
if compile fails, there is no point running lint or tests.

```
CompileGate -> FAIL -> stop -> return Verdict::fail(...)
              (ClippyGate and TestGate never run)
```

**Why this matters:** In a 7-rung pipeline where integration tests take
30 minutes, a compile failure caught in 3 seconds saves 30+ minutes.
Short-circuit mode makes the verification-first architecture efficient.

### 3.2 Full Execution Mode

All gates run regardless of individual outcomes. The final verdict is a
failure if *any* gate failed. Useful for comprehensive diagnostic reports.

```
CompileGate -> FAIL -> continue
ClippyGate  -> PASS -> continue
TestGate    -> FAIL -> continue
-> return aggregated Verdict::fail(...)
   (detail includes output from all three gates)
```

---

## 4. Verdict Aggregation

### 4.1 Pass Condition

The pipeline passes if and only if **every** gate passes. A single failure
anywhere makes the whole pipeline fail.

### 4.2 Detail Aggregation

Individual gate outputs are concatenated with headers:

```
--- [compile:cargo] ---
Compiling foo v0.1.0
Finished dev in 2.3s

--- [clippy:cargo] ---
warning: unused variable

--- [test:cargo] ---
test result: ok. 12 passed; 0 failed; 0 ignored
```

### 4.3 Reason Construction

On failure, the pipeline's `reason` field lists which gate(s) failed:

```
gate pipeline failed: compile:cargo (error: bad thing; error[E0425]: ...)
```

In short-circuit mode: exactly one failed gate. In full-execution mode:
potentially multiple.

### 4.4 Duration

Pipeline `duration_ms` = sum of all individual gate durations. Tracks
total wall-clock time spent on verification.

### 4.5 Test Count Merging

If any gate produces `TestCount`, the pipeline merges them by summing
passed, failed, and ignored counts across all gates. Relevant when a
pipeline contains multiple test gates.

---

## 5. The Pipeline as a Gate

`GatePipeline` implements `Gate`:

```rust
#[async_trait]
impl Gate for GatePipeline {
    async fn verify(&self, signal: &Signal, ctx: &Context) -> Verdict {
        // ... iterate over self.gates, aggregate verdicts
    }

    fn name(&self) -> &str {
        &self.name
    }
}
```

This composability is intentional:

- A pipeline can contain other pipelines (nesting)
- Any code that accepts `&dyn Gate` can accept a pipeline
- The adaptive threshold system, ratchet, and feedback systems work with
  pipelines without special-casing

---

## 6. Execution Flow

```
Pipeline::verify(signal, ctx)
|
+-- gate[0].verify(signal, ctx) -> verdict_0
|   +-- if failed && short_circuit -> return fail verdict
|   +-- collect detail, test counts
|
+-- gate[1].verify(signal, ctx) -> verdict_1
|   +-- if failed && short_circuit -> return fail verdict
|   +-- collect detail, test counts
|
+-- ... (for each gate)
|
+-- aggregate:
    +-- passed = all verdicts passed
    +-- reason = join failure reasons
    +-- detail = join all details with headers
    +-- duration = sum of durations
    +-- test_count = sum of test counts
    +-- return aggregated Verdict
```

---

## 7. ComposedGatePipeline and GateComposition

Beyond `GatePipeline`, the gate system provides `ComposedGatePipeline`
which supports richer composition strategies through `GateComposition`:

```rust
pub enum GateComposition {
    Sequential,   // Default: one after another, short-circuit on fail
    Parallel,     // All concurrently, merge verdicts
    Voting(usize), // Quorum: pass if >= N inner gates pass
    Fallback,     // Try in order, use first non-error verdict
}
```

This allows constructing verification topologies like:

```rust
// Standard 4-rung pipeline
Sequential(vec![CompileGate, ClippyGate, TestGate, SymbolGate])

// Parallel lint: clippy + format check run concurrently
Sequential(vec![
    CompileGate,
    Parallel(vec![ClippyGate, FormatGate]),
    TestGate,
])

// LLM judge panel: 3 judges, pass if 2+ agree
Voting { gates: vec![Judge1, Judge2, Judge3], quorum: 2 }

// Degraded environment fallback
Fallback(ClippyGate, GrepLintGate)
```

---

## 8. How the Orchestrator Uses the Pipeline

The orchestrator constructs a pipeline per task:

```
1. Determine plan complexity (Trivial/Simple/Standard/Complex)
2. Detect environment capabilities (which build tools exist)
3. select_rungs(complexity, caps, prior_failures)
4. Map each Rung to a concrete Box<dyn Gate>
5. GatePipeline::new(gates).with_short_circuit(true)
6. pipeline.verify(signal, ctx)
7. Feed verdict to ratchet, thresholds, feedback
```

The pipeline is constructed fresh for each task execution:

- Different tasks can have different pipelines (based on complexity)
- Escalation adds gates to the pipeline on retry
- The pipeline is lightweight -- no persistent state

---

## 9. Error Handling Within the Pipeline

Because the Gate trait returns `Verdict` (not `Result<Verdict>`), the
pipeline never has to handle gate errors. Every gate handles its own
infrastructure failures internally. The pipeline simply collects verdicts
and aggregates them.

This is the practical benefit of the `-> Verdict` design: composition is
trivial. No error propagation paths, no `?` operators, no `Result::map`
chains. Just: run the gate, get a verdict, check if it passed.

---

## 10. Pipeline Lifecycle in the Universal Loop

```
Universal loop: query -> score -> route -> compose -> act -> VERIFY -> write -> react

Signal produced by agent (act step)
    |
GatePipeline.verify(signal, ctx)
    |
Verdict flows to:
    +-- write: Verdict persisted as signal in Substrate
    +-- react: GateRatchet.record_pass(plan_id, rung)
    +-- react: AdaptiveThresholds.update(rung, passed)
    +-- react: GateFeedback for agent context on retry
    +-- react: EfficiencyEvent for learning
    +-- react: CascadeRouter.update_arm(model, reward)
    +-- react: VerdictPublisher emits observable event
```

The pipeline is the single point where all downstream systems get their
input. One place to add new feedback consumers, instrumentation, or logging.

---

## 11. Pipeline Instrumentation

### Per-Gate Metrics

```rust
pub struct GateMetrics {
    pub gate_name: String,
    pub rung: u8,
    pub passed: bool,
    pub duration_ms: u64,
    pub skipped: bool,
}
```

### Pipeline-Level Summary

```rust
pub struct PipelineMetrics {
    pub name: String,
    pub total_duration_ms: u64,
    pub gates_run: usize,
    pub gates_skipped: usize,
    pub gates_passed: usize,
    pub gates_failed: usize,
    pub short_circuited: bool,
    pub gate_metrics: Vec<GateMetrics>,
}
```

---

## 12. Design: Sequential, Not Parallel (by default)

The default pipeline executes gates sequentially. This is deliberate:

1. **Dependency ordering.** Rung N often depends on Rung N-1's success.
   Running tests on code that does not compile wastes time and produces
   confusing errors.
2. **Short-circuit value.** Sequential execution enables short-circuit,
   which is the pipeline's primary optimization.
3. **Simplicity.** Sequential execution has no synchronization concerns.

The `ParallelGate` wrapper enables concurrent execution where appropriate
(independent gates within the same logical step).

---

## 13. Testing

| Test | What It Verifies |
|------|------------------|
| `pipeline_empty_passes` | Empty pipeline returns pass verdict |
| `pipeline_single_pass` | Single passing gate -> pass |
| `pipeline_single_fail` | Single failing gate -> fail |
| `pipeline_short_circuits` | Stops at first failure |
| `pipeline_full_execution` | Runs all gates when short_circuit=false |
| `pipeline_aggregates_test_counts` | Merges test counts across gates |
| `pipeline_detail_headers` | Detail output has per-gate headers |
| `pipeline_duration_sums` | Total duration = sum of gate durations |

---

## Verification

```bash
cargo test -p roko-gate -- pipeline
```
