# Stuck Detection -- 6 StuckKind Heuristics

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 6.
> Source: `crates/roko-conductor/src/stuck_detection.rs`

---

## 1. The Stuck Problem

An agent can be stuck in ways that no single watcher catches. All variations share
a theme: the agent consumes resources (tokens, wall-clock time, API quota) without
making progress toward task completion.

---

## 2. StuckKind Enum

```rust
pub enum StuckKind {
    OutputLoop,
    NoProgress,
    GateLoop,
    CompileLoop,
    EmptyOutput,
    ExcessiveRetries,
}
```

Each variant represents a distinct detection heuristic. Multiple variants can be
detected simultaneously.

---

## 3. StuckDetector

```rust
pub struct StuckDetector {
    thresholds: StuckThresholds,
}

pub struct StuckThresholds {
    pub output_loop: usize,         // default: 4
    pub no_progress_ms: u64,        // default: 300_000 (5 minutes)
    pub gate_loop: usize,           // default: 3
    pub compile_loop: usize,        // default: 3
    pub empty_output: usize,        // default: 3
    pub excessive_retry: usize,     // default: 6
}
```

### 3.1 OutputLoop (threshold: 4)

Computes a content hash of each agent turn's output. Four consecutive turns
producing the same hash indicates a loop.

**Why hash-based**: Exact string comparison would miss near-identical outputs.
Hashing normalizes the comparison. In practice, true output loops produce
byte-identical output.

**Why 4**: One repeated output is common (agent checks something twice). Two may
indicate deliberate verification. Three is suspicious. Four consecutive identical
outputs is definitively a loop.

### 3.2 NoProgress (threshold: 300,000 ms / 5 minutes)

Checks elapsed time since the last file modification or test state change. Unlike
event-counting heuristics, this is time-based because a truly stuck agent might be
in a reasoning loop with no tool calls at all.

**Why 5 minutes**: Normal implementation tasks show file changes every 30-120
seconds. A 5-minute gap is 5-10x the normal interval.

### 3.3 GateLoop (threshold: 3)

Tracks gate failure patterns per plan. Detects oscillation: the agent "fixes" one
error only to reintroduce a previous one. The failure count stays the same but the
failures cycle.

**Differs from iteration-loop watcher**: The iteration-loop watcher counts
consecutive gate failures. The gate loop detector looks for oscillation patterns.

### 3.4 CompileLoop (threshold: 3)

A specialized gate loop detector for compile errors. Tracks compile error
fingerprints. Error A appears, agent fixes A but introduces B, agent fixes B but
reintroduces A. The watcher sees A, then B, then A (not repeated) -- but the loop
detector recognizes the cycle.

### 3.5 EmptyOutput (threshold: 3)

Counts consecutive turns with no tool calls and no file changes. Three such turns
indicate the agent is producing text without taking action.

**Differs from ghost turns**: Ghost turns are turns with zero output. Empty output
turns have content (potentially verbose content) but no actions.

### 3.6 ExcessiveRetries (threshold: 6)

Counts retry attempts for the same operation. Six retries of `cargo check` without
changing approach indicates a retry loop.

---

## 4. Threshold Tuning

| Threshold | Default | Too Low -> | Too High -> |
|-----------|---------|-----------|-----------|
| output_loop | 4 | False positives on verification | Late detection |
| no_progress_ms | 300,000 | Kills slow-but-progressing agents | 10+ min stalls |
| gate_loop | 3 | Normal retry cycles flagged | 5+ cycle oscillation |
| compile_loop | 3 | Normal fix attempts flagged | 5+ cycle toggling |
| empty_output | 3 | Kills agents that are thinking | Agent describes without acting |
| excessive_retry | 6 | Normal retries flagged | 10+ retries |

The `StuckThresholds` struct accepts custom values, enabling per-deployment tuning.

---

## 5. MetaCognitionHook

The `MetaCognitionHook` wraps the `StuckDetector` into a periodic self-assessment
mechanism operating at Theta frequency:

```rust
pub struct MetaCognitionHook {
    detector: StuckDetector,
    frequency: OperatingFrequency,  // Theta
}
```

### 5.1 Operating Frequency

| Frequency | Rate | Purpose |
|-----------|------|---------|
| Gamma | High (every turn) | Real-time tool dispatch, safety checks |
| Theta | Medium (periodic) | Self-assessment, meta-cognition |
| Delta | Low (between sessions) | Consolidation, pattern extraction |

Theta frequency means the meta-cognition check runs periodically -- not on every
turn (too expensive) but often enough to catch stuck agents before they burn
significant budget.

### 5.2 Assessment Output

```rust
pub enum MetaCognitionAction {
    Continue,
    AdjustStrategy,
    Escalate,
}

pub struct MetaCognitionAssessment {
    pub frequency: OperatingFrequency,
    pub action: MetaCognitionAction,
    pub reason: String,
    pub stuck_kinds: Vec<StuckKind>,
}
```

| Stuck Kind | MetaCognition Action | Rationale |
|-----------|---------------------|-----------|
| OutputLoop | AdjustStrategy | Agent needs a different approach |
| NoProgress | AdjustStrategy | Agent is stalled; refocus |
| GateLoop | Escalate | Cycling indicates fundamental problem |
| CompileLoop | Escalate | Architectural mismatch |
| EmptyOutput | AdjustStrategy | Needs more directive prompting |
| ExcessiveRetries | AdjustStrategy | Different operation needed |

`AdjustStrategy` maps to Conductor Restart. `Escalate` maps to Conductor Fail.

### 5.3 Signal Serialization

Assessments are serializable as Signals with `Kind::Custom("conductor.meta_cognition")`.
These feed into the Conductor's signal stream for incorporation into the overall
decision.

---

## 6. The Self-Model Requirement

The meta-cognition hook implements a principle from the Good Regulator Theorem
(Conant & Ashby, 1970):

> "Every good regulator of a system must be a model of that system."

The stuck detector is Roko's self-model -- its representation of what "healthy
execution" looks like. By defining six specific stuck kinds, the system models six
ways execution can deviate from health.

This self-model is necessarily incomplete. Ashby's Law of Requisite Variety
constrains growth: the detector must have at least as many distinguishable states
as the execution system has pathological states. With six heuristics, the detector
can distinguish six stuck modes. If execution can be stuck in seven distinct ways,
the detector has insufficient variety. The modular architecture supports expansion.

---

## 7. Overlap with Watcher Ensemble

| Detection | Stuck Detector | Watcher Ensemble |
|-----------|---------------|-----------------|
| Identical compile errors | CompileLoop | compile-fail-repeat watcher |
| Zero output | EmptyOutput | ghost-turn watcher |
| No file changes | NoProgress | (not directly covered) |
| Identical actions | OutputLoop | stuck-pattern watcher |
| Gate failure cycling | GateLoop | iteration-loop watcher |
| Cost overrun | (not covered) | cost-overrun watcher |
| Context pressure | (not covered) | context-window-pressure watcher |

The overlaps are intentional -- the stuck detector provides a complementary
detection mechanism with different thresholds and detection logic. The watcher
ensemble operates on the signal stream (structured data). The stuck detector can
operate on raw agent output (unstructured data).

---

## 8. CooldownFilter

The `CooldownFilter` wraps stuck detections to prevent rapid re-firing. After a
stuck detection fires for a given plan, it will not fire again for the same plan
until the cooldown period (default 120s) has elapsed. This prevents the conductor
from issuing multiple restarts before the first restart has taken effect.

---

## 9. File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/stuck_detection.rs` | StuckDetector, StuckKind, StuckThresholds, MetaCognitionHook, CooldownFilter |
