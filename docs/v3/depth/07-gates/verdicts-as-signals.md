# Verdicts as Signals

> Depth file for [07-GATES.md](../../07-GATES.md) section 15.
> Source: `crates/roko-gate/src/verdict_publisher.rs`,
> `crates/roko-core/src/kind.rs` (`Kind::GateVerdict`),
> `crates/roko-learn/src/episode_logger.rs`

---

## 1. The Core Claim: Verification Is Cognition

In a standard CI pipeline, a gate verdict is an end state: pass or fail,
logged and forgotten. In Roko, a gate verdict is a beginning. When a compile
gate fails, that failure is a Signal with a Kind, Score, Decay, and lineage.
It enters the Substrate. Other components query it.

The verdict is not metadata about the pipeline. It is a data point in the
agent's cognitive process.

---

## 2. Verdict-to-Signal Transformation

### 2.1 The VerdictPublisher

The `verdict_publisher.rs` module converts gate verdicts into `Pulse` events
with `Kind::GateVerdict`:

```rust
pub struct VerdictSummary {
    pub gate: String,       // Gate that produced the verdict
    pub passed: bool,
    pub score: f32,
    pub reason: String,     // Empty on pass
    pub rung: Option<u32>,
    pub duration_ms: u64,
}

pub struct VerdictPublisher {
    callback: Arc<VerdictPublishFn>,
    sequence: AtomicU64,
}
```

The publisher is optional. When configured, it broadcasts each verdict as a
`Pulse` with topic `gate.verdict.emitted`. When not configured, verdicts are
produced but not broadcast -- the pipeline still works, just without real-time
downstream notification.

### 2.2 Transformation Properties

When a gate completes, the orchestrator transforms its verdict into a Signal:

| Property | Value | Rationale |
|---|---|---|
| Kind | `Kind::GateVerdict` | Defined in `roko-core/src/kind.rs` |
| Decay | HalfLife 24 hours | Code changes invalidate verdicts; yesterday's compile pass is stale |
| Lineage | Points to the task Signal | Preserves causal chain for auditing |
| Tags | gate name, passed, plan_id, task_id | Enable filtering by gate type and outcome |

### 2.3 The Two GateVerdict Structs

Two `GateVerdict` structs exist in the codebase, serving different purposes:

**Episode logger version** (learning-relevant fields):

```rust
// crates/roko-learn/src/episode_logger.rs
pub struct GateVerdict {
    pub gate: String,
    pub passed: bool,
    pub signature: Option<String>,  // Hashed diagnostic, never raw output
}
```

**Dashboard version** (plan/task context):

```rust
// crates/roko-core/src/dashboard_snapshot.rs
pub struct GateVerdict {
    pub plan_id: String,
    pub task_id: String,
    pub gate: String,
    pub passed: bool,
    pub ts_millis: u64,
}
```

Both serialize to JSON and are consumed by their respective subsystems.

---

## 3. Signal Pipeline Flow

Once emitted, the verdict Signal enters the standard cognitive loop:

```
Gate verdict emitted
    |
    v
Substrate.write(verdict_signal)     -- persisted to .roko/engrams.jsonl
    |
    v
Scorer.score(verdict_signal)        -- appraise relevance and urgency
    |
    v
Router.select(candidates)           -- verdict history influences routing
    |
    v
Composer.compose(context)           -- recent verdicts injected into prompts
    |
    v
Dreams.replay(episodes)             -- verdict patterns extracted during
                                       consolidation
```

---

## 4. Downstream Consumers

### 4.1 Scorer: Verdict Appraisal

The Scorer assigns a Score to the verdict Signal:

| Dimension | Value |
|---|---|
| Relevance | 1.0 (active task), 0.5 (same plan), 0.1 (other plan) |
| Confidence | 1.0 (gate verdicts are deterministic) |
| Urgency | 0.9 (failure needs attention), 0.3 (pass is informational) |
| Novelty | 1.0 (first verdict for this gate+task), 0.2 (repeated) |
| Surprise | 1.0 (contradicts prediction), 0.0 (matches prediction) |
| Coherence | 1.0 (verdicts are self-consistent by construction) |

### 4.2 Router: Verdict-Informed Model Selection

The cascade router queries verdict history when selecting a model:

```
For task T, query Substrate for GateVerdict signals where task_id == T:
  - 0 prior failures: standard routing
  - 1 prior failure:  escalate model tier by 1 (e.g., Haiku -> Sonnet)
  - 2+ prior failures: escalate to maximum tier (Opus)
  - 3+ same-signature failures: flag for replanning
```

### 4.3 Composer: Verdict Injection

The SystemPromptBuilder includes recent verdicts as a dedicated prompt
section:

```
## Previous Attempts on This Task

Attempt 1: FAIL (compile)
  Error: E0599 - no method named `foo` found for struct `Bar`
  Signature: a3f8c2

Attempt 2: FAIL (test)
  Error: assertion failed in test_routing_basic
  Signature: 7d1e4b
```

This gives the agent direct visibility into its own failure history,
preventing it from repeating the same mistake.

### 4.4 Dreams: Verdict Pattern Extraction

During NREM replay, Dreams extracts patterns from verdict sequences:

```
Input: all GateVerdict signals from the last consolidation window
Process:
  1. Group by (gate, signature) -- same error type
  2. For each group with >= 3 occurrences:
     a. Extract common context (file paths, error codes, task types)
     b. Generate a Heuristic knowledge entry:
        "When working on [context], [gate] tends to fail with [signature]"
     c. Insert at Transient tier for validation
  3. For groups where failure was followed by success:
     a. Extract the delta between failing and succeeding attempts
     b. Generate a StrategyFragment:
        "To fix [signature], the successful approach was [delta]"
```

### 4.5 Adaptive Thresholds

Each verdict updates the per-rung EMA, adjusting retry budgets and skip
advisories for future executions.

### 4.6 Ratchet

The ratchet records the highest rung passed, preventing regression. This
tracking depends on verdict Signals flowing back through the system.

---

## 5. Verdict Decay and Lifecycle

| Stage | Timing | Action |
|---|---|---|
| Emission | Gate completes | Signal written to Substrate |
| Active use | 0-4 hours | Composer injects into prompts; Router adjusts |
| Fading relevance | 4-24 hours | Weight decays below 0.5; lower priority |
| Consolidation | During Dreams Delta | Patterns extracted; individual verdicts consumed |
| Pruning | Weight < threshold | Substrate.prune() removes the verdict Signal |

The 24-hour HalfLife means a verdict retains 50% weight after one day. Code
changes within a day can invalidate any verdict. After Dreams consolidation,
patterns survive in knowledge entries even after raw verdicts are pruned.

---

## 6. Lineage and Auditing

Every verdict Signal records its lineage -- the task Signal it derived from:

```
Plan Signal
  |
  +-- Task Signal (T1)
  |     |
  |     +-- GateVerdict (compile: pass)
  |     +-- GateVerdict (test: fail)
  |     +-- GateVerdict (test: pass, attempt 2)
  |
  +-- Task Signal (T2)
        |
        +-- GateVerdict (compile: fail)
```

The `roko replay` command walks this DAG to reconstruct the full verification
history for any plan or task.

---

## 7. Verdict Aggregation: Trend Detection

Individual verdicts are snapshots. Trends across verdicts reveal systemic
changes. The `VerdictTimeSeries` tracks outcomes per gate over a sliding
window:

```rust
pub struct VerdictTimeSeries {
    pub gate: String,
    pub observations: VecDeque<VerdictObservation>,
    pub max_observations: usize,     // default: 500
    pub ema_pass_rate: f64,
    pub ema_score: f64,
    pub trend: VerdictTrend,
}

pub enum VerdictTrend {
    Stable,       // No significant change
    Improving,    // Consistent improvement
    Degrading,    // Consistent degradation
    Volatile,     // High variance, no direction
    RegimeShift,  // Fundamental change (BOCPD detected)
}
```

Trend classification uses three signals: EMA slope (direction), CUSUM (sustained
shift), and BOCPD (regime change). A `Degrading` trend on the test gate
triggers sympathetic tightening on upstream gates (compile, lint).

---

## 8. Co-Failure Pattern Detection

The `CoFailureDetector` identifies gates that tend to fail together:

```rust
pub struct CoFailureDetector {
    pub co_failures: HashMap<(String, String), u64>,
    pub gate_counts: HashMap<String, u64>,
    pub correlation_threshold: f64,   // default: 0.3 (phi coefficient)
}
```

When compile and lint failures correlate above threshold, they likely share a
root cause (e.g., syntax errors cause both). Identifying these correlations
enables root-cause targeting rather than per-gate retrying.

---

## 9. Failure Signature Clustering

Group failures by their error signature to identify recurring issues:

```rust
pub struct SignatureCluster {
    pub signature: String,
    pub gate: String,
    pub count: u64,
    pub affected_tasks: Vec<String>,
    pub affected_models: Vec<String>,
    pub trend: VerdictTrend,
}
```

A signature cluster with count >= 3 triggers the replanning engine. Clusters
growing over time indicate a systemic issue that needs structural attention,
not just retries.

---

## 10. Verdict-Driven Replanning

When verdict patterns indicate a task is fundamentally broken, the
`ReplanEngine` produces an action:

| Trigger | Action |
|---|---|
| Same signature fails 3 times | `ModifyTask` -- adjust constraints |
| Negative progress for 3 turns | `DecomposeTask` -- split into sub-tasks |
| Promise below 0.2 for 2 turns | `ReplaceTask` -- regenerate from failure context |
| Gate degradation trend | `Escalate` -- stronger model or more gates |

These actions feed back into the orchestrator, which adjusts the plan's
execution strategy without human intervention.

---

## 11. Configuration Parameters

| Parameter | Default | Range | Description |
|---|---|---|---|
| `verdict_decay_half_life_ms` | 86,400,000 (24h) | 1h-7d | Verdict relevance decay |
| `verdict_max_prompt_tokens` | 500 | 50-2,000 | Max tokens for verdict prompt section |
| `verdict_escalation_threshold` | 2 | 1-5 | Failures before model tier escalation |
| `verdict_replan_threshold` | 3 | 2-10 | Same-signature failures before replanning |
| `verdict_dreams_min_group_size` | 3 | 2-10 | Min occurrences for pattern extraction |

---

## 12. Test criteria

| Test | Property |
|---|---|
| `verdict_produces_gate_verdict_kind` | Signal has `Kind::GateVerdict` |
| `verdict_signal_roundtrips_serde` | JSON serialization preserves all fields |
| `substrate_query_by_gate_tag` | Filter by `tag("gate", "compile")` returns only compile verdicts |
| `router_escalates_after_2_failures` | Same-task failures -> model tier increase |
| `composer_respects_token_budget` | Verdict section within `verdict_max_prompt_tokens` |
| `dreams_extracts_heuristic_from_3_failures` | 3+ same-signature -> knowledge entry |
| `verdict_weight_halves_at_24_hours` | Decay follows HalfLife formula |
| `duplicate_verdicts_deduplicated` | Same content hash -> no duplicate storage |
| `trend_stable_on_constant_rate` | 100 obs at 85% -> VerdictTrend::Stable |
| `co_failure_detects_correlation` | Compile+lint fail together -> phi > threshold |
