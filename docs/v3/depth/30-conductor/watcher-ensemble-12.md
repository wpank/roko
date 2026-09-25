# Watcher Ensemble -- All 12 Watchers

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 2.
> Source: `crates/roko-conductor/src/watchers/`

---

## 1. The React Trait

Every watcher implements the same trait:

```rust
pub trait React: Send + Sync {
    fn decide(&self, stream: &[Signal], ctx: &Context) -> Vec<Signal>;
    fn name(&self) -> &str;
}
```

`decide()` receives the full signal stream and returns intervention signals. An
empty return means "healthy -- nothing to report." A non-empty return means the
watcher has detected an anomaly and is emitting one or more intervention signals
with severity tags.

---

## 2. Watcher Catalog

### 2.1 Ghost Turn Detector

**Module**: `watchers/ghost_turn.rs`
**Constant**: `MAX_GHOST_TURNS = 3`
**Watcher name**: `ghost-turn`

Detects agent turns that produce zero meaningful output. A ghost turn is a turn
where the model returned immediately with no tool calls, no file changes, and no
substantive content. This is a known failure mode with API-based agents.

Scans the signal stream for `AgentOutput` signals. If the output body matches the
ghost pattern (below minimum meaningful length), increments a counter. After three
consecutive ghost turns from the same agent, fires a Warning.

**Severity**: Warning (triggers restart with fresh context)

**Why three, not one**: A single empty response can be a transient API issue. Two
might be a flaky connection. Three consecutive ghost turns indicate the agent has
entered a degenerate state and will not recover without intervention.

### 2.2 Compile Fail Repeat Detector

**Module**: `watchers/compile_fail_repeat.rs`
**Constant**: `MAX_COMPILE_FAIL_REPEAT = 3`
**Watcher name**: `compile-fail-repeat`

Detects the same compile error appearing across consecutive gate verdicts without
progress. Examines `GateVerdict` signals for compile-related gate results, extracts
error fingerprints. Three identical compile errors in sequence trigger Warning.

**Severity**: Warning (triggers restart with error analysis context)

**Why this matters**: An agent stuck on a compile error is the most common form of
agent loop. Without intervention, this cycle continues until the iteration limit.

### 2.3 Cost Overrun Detector

**Module**: `watchers/cost_overrun.rs`
**Constant**: `DEFAULT_BUDGET_USD = 10.0`
**Watcher name**: `cost-overrun`

Detects plan-level cost exceeding the allocated budget. Scans for `Metric` signals
tagged with `name=plan_cost`. Compares accumulated cost against the plan's budget.

**Severity**: Warning at threshold, escalates based on overage

**Budget allocation by complexity**:

| Complexity | Typical Cost | Suggested Budget |
|-----------|-------------|-----------------|
| Trivial | $0.10-0.50 | $2.00 |
| Simple | $0.50-2.00 | $5.00 |
| Standard | $1.00-5.00 | $10.00 |
| Complex | $3.00-15.00 | $25.00 |

### 2.4 Iteration Loop Detector

**Module**: `watchers/iteration_loop.rs`
**Constant**: `MAX_ITERATION_LOOP = 3`
**Watcher name**: `iteration-loop`

Detects plans cycling through the gate-fail-retry loop without making progress.
Tracks `GateVerdict` signals per plan. Three consecutive gate failures without an
intervening pass trigger the intervention.

**Severity**: **Critical** (triggers plan failure)

This is the only watcher that defaults to Critical. Three consecutive gate failures
indicate a fundamental mismatch between the agent's approach and the requirements.
More iterations of the same approach will not converge. This implements Hard
Guarantee 3: "Hard Iteration Cap (Not Soft, Not Heuristic)."

### 2.5 Review Loop Detector

**Module**: `watchers/review_loop.rs`
**Constant**: `MAX_REVIEW_CYCLES = 3`
**Watcher name**: `review-loop`

Detects plans receiving repeated review rejects without advancing. Scans `PlanPhase`
signals for `ReviewRejected` events. Resets the counter on `ReviewApproved`,
`DocRevisionDone`, or `MergeSucceeded`. Fires when the count reaches three.

**Severity**: Warning (triggers review skip or strategy change)

Catches the bikeshedding problem: reviewer agents entering cycles where code passes
compilation and tests but the reviewer repeatedly requests stylistic changes.

### 2.6 Spec Drift Detector

**Module**: `watchers/spec_drift.rs`
**Constant**: `MAX_SPEC_DRIFT_RATIO = 0.25`
**Watcher name**: `spec-drift`

Detects agent file edits drifting outside the declared scope of the task. Examines
`Metric` signals tagged `name=spec_drift`. When drift ratio exceeds 25%, fires.

**Severity**: Warning

**Why 25%**: Some drift is normal. An agent implementing a new function may need to
update a `mod.rs`. At 25%, the agent is making substantial changes outside scope --
potentially stepping on another concurrent agent's territory.

### 2.7 Stuck Pattern Detector

**Module**: `watchers/stuck_pattern.rs`
**Constant**: `MAX_STUCK_REPEATS = 4`
**Watcher name**: `stuck-pattern`

Detects agent producing identical actions across consecutive turns. Tracks recent
agent actions (tool calls, file edits) and computes similarity between consecutive
turns. Four consecutive identical or near-identical actions trigger the
intervention.

**Severity**: Warning

### 2.8 Test Failure Budget Detector

**Module**: `watchers/test_failure_budget.rs`
**Constant**: `MIN_FAILURE_INCREASE = 1`
**Watcher name**: `test-failure-budget`

Detects test failure count increasing beyond the baseline. Scans `GateVerdict`
signals for structured test counts. Records the first observed failure count as
baseline. When the latest count exceeds baseline by `MIN_FAILURE_INCREASE`, fires.

**Severity**: Warning

Detects a specific problem: the agent is making things worse, not better. If a plan
starts with 1 failing test and ends with 3, the agent introduced 2 new regressions.

### 2.9 Time Overrun Detector

**Module**: `watchers/time_overrun.rs`
**Constant**: `ALERT_THRESHOLD = 0.80`
**Watcher name**: `time-overrun`

Detects tasks approaching their timeout threshold. Examines
`Custom("conductor.agent_output")` signals for timing payloads. When the duration
exceeds 80% of the timeout, fires an early warning.

```rust
// Integer arithmetic to avoid floating-point edge cases
fn exceeds_threshold(duration_ms: u64, timeout_secs: u64) -> bool {
    if timeout_secs == 0 { return false; }
    let timeout_ms = timeout_secs.saturating_mul(1000);
    duration_ms.saturating_mul(5) > timeout_ms.saturating_mul(4)
    // Equivalent to: duration_ms / timeout_ms > 0.80
}
```

**Severity**: Warning

### 2.10 Context Window Pressure Detector

**Module**: `watchers/context_window_pressure.rs`
**Constant**: `MAX_CONTEXT_USAGE_RATIO = 0.80`
**Watcher name**: `context-window-pressure`

Detects agent context window filling beyond safe limits. Supports two signal
formats: structured `AgentEfficiencyEvent` body or tag-based format. Fires when
`used / total > 0.80`.

**Model context windows**:

| Model Pattern | Context Window |
|--------------|---------------|
| `*opus*` | 1,000,000 tokens |
| `*haiku*`, `*sonnet*` | 200,000 tokens |
| Unknown | No fire (cannot compute ratio) |

**Severity**: Warning (triggers context compaction)

Reads from the same `AgentEfficiencyEvent` signals that feed efficiency tracking --
every turn that records efficiency data also gets context pressure monitoring.

### 2.11 Pattern Detector

**Module**: `watchers/pattern_detector.rs`
**Watcher name**: `pattern-detector`

Detects complex multi-signal failure patterns using NFA-style matching across
watcher outputs. Identifies correlated failures that no single watcher catches --
such as rising cost concurrent with stuck patterns concurrent with context pressure.

**Severity**: Configurable per pattern

### 2.12 Threshold Learner Watcher

**Module**: `watchers/threshold_learner.rs`
**Watcher name**: `threshold-learner`

Monitors whether the Conductor's own thresholds are well-calibrated by tracking
intervention effectiveness over time. Fires when the model detects its own accuracy
has degraded, indicating threshold drift.

**Severity**: Info (advisory signal for the self-healing policy)

---

## 3. Watcher Independence

Each watcher is independent:

- **No shared state**: Watchers do not read each other's output or maintain shared
  counters.
- **No ordering dependency**: Watchers can execute in any order. The Conductor
  iterates them sequentially for simplicity, but parallel execution would produce
  identical results.
- **No cross-watcher interaction**: If both ghost-turn and stuck-pattern fire
  simultaneously, the intervention policy resolves the conflict (worst severity
  wins).

This independence is what makes the ensemble testable. Each watcher has its own
`#[cfg(test)] mod tests` with focused test cases.

---

## 4. Watcher Families

Watchers group into three families based on what they measure:

**Resource family**: cost-overrun, context-window-pressure, time-overrun. These
track consumption of finite budgets (money, tokens, wall-clock time). When two
resource watchers fire together, the root cause is usually a single runaway process.

**Behavioral family**: ghost-turn, stuck-pattern, compile-fail-repeat,
iteration-loop. These detect degenerate agent behavior. Two behavioral watchers
firing together confirms the agent is stuck from multiple angles.

**Coordination family**: review-loop, spec-drift, test-failure-budget. These detect
multi-agent coordination breakdowns.

**Within-family correlation**: 2+ watchers in the same family firing simultaneously
indicates a single correlated event. De-duplicate before escalating.

**Cross-family correlation**: Watchers from different families firing simultaneously
is more severe. A behavioral failure combined with a resource failure means the
agent is both stuck AND burning budget. Cross-family correlation escalates severity
by one level.

---

## 5. Adding a New Watcher

To add a thirteenth watcher:

1. Create a new file in `watchers/` implementing `React`
2. Add it to `watchers/mod.rs`
3. Add it to `Conductor::new()` in `conductor.rs`
4. Update the `watcher_count()` test (currently asserts 12)
5. Write focused tests for the new watcher's detection logic

The Conductor's `evaluate()` method automatically picks up any watcher in the
`watchers` vector. No other code needs to change.

---

## 6. File Reference

| File | Lines | What |
|------|-------|------|
| `watchers/mod.rs` | ~25 | Module declarations, re-exports |
| `watchers/ghost_turn.rs` | ~150 | Ghost turn detection |
| `watchers/compile_fail_repeat.rs` | ~180 | Compile error repetition |
| `watchers/cost_overrun.rs` | ~180 | Cost budget monitoring |
| `watchers/iteration_loop.rs` | ~170 | Gate-fail cycle detection |
| `watchers/review_loop.rs` | ~230 | Review reject cycles |
| `watchers/spec_drift.rs` | ~264 | File scope drift |
| `watchers/stuck_pattern.rs` | ~170 | Repeated action detection |
| `watchers/test_failure_budget.rs` | ~202 | Test regression detection |
| `watchers/time_overrun.rs` | ~182 | Timeout approach warning |
| `watchers/context_window_pressure.rs` | ~233 | Token usage monitoring |
| `watchers/pattern_detector.rs` | ~200 | Multi-signal pattern matching |
| `watchers/threshold_learner.rs` | ~180 | Self-calibration monitoring |
