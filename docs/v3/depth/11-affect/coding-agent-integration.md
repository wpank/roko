# Coding Agent Integration

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- v1 source: `docs/v1/09-daimon/11-coding-agent-integration.md`

---

## Overview

The Daimon is domain-agnostic -- it tracks PAD vectors and behavioral states
regardless of what the agent is doing. But its *signals* are domain-specific. For
coding agents, the Daimon integrates with the gate pipeline, task system, and
codebase index to provide three capabilities that generic affect tracking cannot:
**per-crate confidence**, **error pattern sensitivity**, and **fatigue detection**.

These are not separate features built on top of the Daimon -- they are projections
of the standard PAD-and-appraisal pipeline onto the coding domain. A gate failure
on `roko-core` produces the same appraisal as a gate failure on `roko-daimon`, but
the per-crate confidence tracker records which crate the failure occurred in,
enabling the agent to distinguish "I'm struggling with everything" from "I'm
confident with most crates but struggling with `roko-daimon`."

---

## Per-Crate Confidence

### The Problem

A global confidence score does not distinguish between domain-specific competence
levels. An agent that has successfully modified `roko-core` 50 times but has never
touched `roko-daimon` should approach these two crates with different caution
levels. The global confidence averages across all experiences, losing this
distinction.

### The Solution

`DaimonState` maintains a `HashMap<String, CrateConfidence>` keyed by crate name.
When a gate result arrives, the appraisal engine extracts the crate name from task
metadata:

```rust
let crate_key = format!("{}:{}", task_id, affected_crate);
let state = self.crate_trackers.entry(crate_key)
    .or_insert_with(CrateConfidence::default);
```

### Per-Crate Confidence Query

```rust
impl DaimonState {
    pub fn crate_confidence(&self, crate_name: &str) -> f64 {
        let crate_states: Vec<_> = self.crate_trackers.iter()
            .filter(|(key, _)| key.ends_with(&format!(":{}", crate_name)))
            .map(|(_, state)| state)
            .collect();

        if crate_states.is_empty() { return 0.50; }

        let total: f64 = crate_states.iter().map(|s| s.confidence).sum();
        total / crate_states.len() as f64
    }
}
```

### Behavioral Impact

Per-crate confidence modulates behavior at two points:

1. **Tier routing**: Low crate confidence (< 0.40) promotes model tier. High crate
   confidence (> 0.80) demotes model tier. Applied on top of global behavioral
   state bias.

2. **Strategy space coordinates**: The Confidence dimension of the 8D strategy
   space uses per-crate confidence when available, falling back to global
   confidence. The somatic landscape distinguishes "confident in this crate" from
   "confident overall."

### Example

```
Agent modified roko-core 30 times: 28 pass, 2 fail -> confidence: 0.85
Agent modified roko-daimon 3 times: 1 pass, 2 fail -> confidence: 0.35

Task: "Add method to roko-core"
  -> Crate confidence: 0.85
  -> Strategy: Focused/Exploratory (cheap model, more exploration)

Task: "Wire somatic landscape into roko-daimon"
  -> Crate confidence: 0.35
  -> Strategy: Struggling/Escalating (promote model, more retries)
```

---

## Error Pattern Sensitivity

### The Problem

Not all errors are equal. A borrow checker error the agent has seen and resolved
10 times before is different from one in a novel context. The standard appraisal
pipeline treats all gate failures identically.

### The Solution

The `ErrorPatternTracker` tracks which error categories have been encountered and
resolved:

```rust
pub struct ErrorPatternTracker {
    patterns: HashMap<String, (u32, u32)>,  // category -> (seen, resolved)
}

impl ErrorPatternTracker {
    pub fn familiarity(&self, error_category: &str) -> f64 {
        let (seen, resolved) = self.patterns
            .get(error_category)
            .copied()
            .unwrap_or((0, 0));
        if seen == 0 { return 0.0; }
        let resolution_rate = resolved as f64 / seen as f64;
        let experience = (seen as f64 / 10.0).min(1.0);
        resolution_rate * experience
    }

    pub fn scale_gate_failure(
        &self, error_category: &str, base_delta: (f64, f64, f64, f64),
    ) -> (f64, f64, f64, f64) {
        let familiarity = self.familiarity(error_category);
        let scale = 1.5 - familiarity;  // unfamiliar: 1.5x; familiar: 0.5x
        (
            base_delta.0 * scale,
            base_delta.1 * scale,
            base_delta.2 * scale,
            base_delta.3 * scale,
        )
    }
}
```

### Error Category Extraction

| Gate | Category Extraction |
|---|---|
| Compile gate | Parse `rustc` error codes: E0382 -> "borrow_check" |
| Test gate | Failed test name -> "test_failure:{test_name}" |
| Clippy gate | Lint IDs -> "clippy_lint:unwrap_used" |
| Diff review gate | "large_change", "api_break", "missing_test" |
| LLM judge gate | "logic_error", "incomplete_impl" |

### Behavioral Impact

- **Familiar error (familiarity > 0.7)**: reduced PAD delta (0.5x). Agent stays in
  Engaged or Focused -- "I know what to do."
- **Unfamiliar error (familiarity < 0.3)**: amplified PAD delta (1.5x). Agent more
  likely to enter Struggling, triggering model escalation.

---

## Fatigue Detection

### The Problem

Repeated failures on the same task produce a characteristic pattern: high arousal,
low pleasure, decreasing dominance. Continuing to attempt the same approach wastes
compute.

### Detection

```rust
pub struct FatigueDetector {
    task_failures: HashMap<String, FatigueState>,
}

impl FatigueDetector {
    pub fn is_fatigued(&self, task_id: &str) -> bool {
        let state = match self.task_failures.get(task_id) {
            Some(s) => s,
            None => return false,
        };
        let many_failures = state.consecutive_failures >= 3;
        let pleasure_drop = state.pleasure_at_start - state.current_pleasure > 0.15;
        let rapid = (state.last_failure_at - state.first_failure_at)
            .num_minutes() as f64 / 60.0 < 2.0;
        many_failures && pleasure_drop && rapid
    }
}
```

Three indicators: 3+ consecutive failures, significant pleasure drop, and failures
within a short time window (< 2 hours).

### Response to Fatigue

| Response | When | Mechanism |
|---|---|---|
| **Re-plan** | Approach is wrong | Trigger plan regeneration |
| **Model escalation** | Model cannot handle difficulty | Promote to T2, extended turns |
| **Dream cycle** | Pattern recognition might help | Trigger consolidation |
| **Deprioritize** | Other tasks available | Move task down queue |
| **Help request** | No automated solution | Signal to operator or mesh |

The response is selected by current behavioral state:

```rust
fn fatigue_response(state: &BehavioralState) -> FatigueAction {
    match state {
        Struggling => FatigueAction::Escalate,
        Exploring  => FatigueAction::Replan,
        Resting    => FatigueAction::DreamCycle,
        _          => FatigueAction::Deprioritize,
    }
}
```

---

## SystemPromptBuilder Integration

The Daimon state is injected into the agent's system prompt:

```
<daimon>
  behavioral_state: Struggling
  confidence: 0.35
  crate_confidence:
    roko-core: 0.85
    roko-daimon: 0.35
  recent_emotions:
    - gate_fail (rung 2): P:-0.13, A:+0.05, D:-0.10
    - task_fail: P:-0.20, A:0.00, D:-0.15
  fatigue: detected (3 consecutive failures on task-abc)
  recommendation: escalate to stronger model, consider re-planning
</daimon>
```

The Daimon section bids for inclusion in the context window via the VCG auction.
Under high arousal, the bid increases, making emotional context more likely to be
included.

---

## Academic Foundations

- Mehrabian, A. (1996). *Current Psychology*, 14(4), 261-292.
- Seligman, M.E.P. (1967). "Failure to escape traumatic shock." *Journal of
  Experimental Psychology*, 74(1), 1-9.
- Shinn, N. et al. (2023). "Reflexion." *NeurIPS*.

---

## Cross-References

- `six-behavioral-states.md` -- behavioral states
- `8-dimensional-strategy-space.md` -- coding domain dimensions
- `integration-points.md` -- system-wide integration
- `daimon-state-and-affect-engine.md` -- DaimonState struct and persistence
