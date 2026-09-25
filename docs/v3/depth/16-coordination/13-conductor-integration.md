# Depth: Conductor Integration

> Parent: [16-COORDINATION](../../16-COORDINATION.md) -- Section 13
>
> Source: `crates/roko-conductor/src/lib.rs`

---

## Overview

The Conductor is the meta-cognitive controller that monitors the orchestration
pipeline and detects anomalies. While the Graph engine drives plans through
cells and the PlanRunner dispatches actions, the Conductor watches the
system's behavior over time and intervenes when things go wrong.

The Conductor operates at a different timescale than the executor. The
executor is reactive (process this event, emit this action). The Conductor is
reflective (over the last 30 seconds, are these signals healthy?).

---

## Architecture

```
                 +----------------+
                 |   Conductor    |
                 |                |
  Vec<Signal> -->| 12 Watchers    |--> ConductorDecision
                 | DiagnosisEngine|
                 | Circuit Breaker|
                 +----------------+
```

The Conductor contains:

- **12 watchers**: Pattern detectors scanning recent signals for anomalies
- **DiagnosisEngine**: Correlates watcher findings into diagnoses
- **Circuit breaker**: Prevents cascading failures by halting problematic plans

---

## Watchers

Each watcher checks for a specific anomaly pattern:

| # | Watcher | Detects | Signal |
|---|---------|---------|--------|
| 1 | Silence detector | Agent hasn't produced output for > threshold | `conductor:alert:silence` |
| 2 | Ghost turn detector | Agent looping without progress | `conductor:alert:ghost_turn` |
| 3 | Compile failure escalation | Repeated compilation failures on same files | `conductor:alert:compile_loop` |
| 4 | Review loop detector | Implementation-review-rejection cycle repeated | `conductor:alert:review_loop` |
| 5 | Cost overrun detector | Cumulative cost exceeds budget threshold | `conductor:alert:cost_overrun` |
| 6 | Context window pressure | Token usage approaching context limit | `conductor:alert:context_pressure` |
| 7 | Gate failure rate | Gate failures exceed threshold per plan | `conductor:alert:gate_failure_rate` |
| 8 | Deadlock detector | Multiple plans waiting on each other | `conductor:alert:deadlock` |
| 9 | Resource pressure | Too many concurrent processes / disk usage | `conductor:alert:resource_pressure` |
| 10 | Progress stall | No phase transitions for extended period | `conductor:alert:progress_stall` |
| 11 | Provider health | Provider error rates exceed threshold | `conductor:alert:provider_health` |
| 12 | Worktree pressure | Too many active worktrees consuming disk | `conductor:alert:worktree_pressure` |

### Signal-Based Operation

Watchers consume Signal values from the signal log (`.roko/engrams.jsonl`).
The WatcherRunner periodically reads the most recent signals and passes them
to the conductor:

```rust
let findings = self.conductor.check_all(&signals);
```

Alert signals are written back to the signal log, where they become visible
to the orchestrator on the next evaluation cycle.

---

## Background Watcher Runner

The WatcherRunner runs as a background Tokio task:

```rust
struct WatcherRunner {
    conductor: Arc<Conductor>,
    signals_path: PathBuf,
    efficiency_path: PathBuf,
    budget_usd: Option<f64>,
    cancel: TokioCancellationToken,
}
```

### Operation Cycle

Every `WATCHER_INTERVAL_SECS` (30 seconds):

1. Read the most recent `WATCHER_SIGNAL_TAIL` (200) signals from
   `.roko/engrams.jsonl`
2. Load efficiency events and build cost metric signals
3. Build context window pressure signals from efficiency data
4. Run `conductor.check_all(&signals)`
5. Filter for alert-type signals
6. Persist alert signals back to the signal log

The watcher respects the cancellation token for graceful shutdown.

---

## Cost Monitoring

Cost monitoring is tightly integrated with the conductor:

### Budget Tracking

Sums cost from recent efficiency events and emits metric signals:

```
{ "kind": "Metric", "name": "plan_cost", "value": "1.234567" }
{ "kind": "Metric", "name": "plan_budget", "value": "5.000000" }
```

The cost overrun watcher compares plan cost against budget and fires
`conductor:alert:cost_overrun` when the threshold is breached.

### Context Window Pressure

Reads the latest efficiency event to extract token usage. High token usage
indicates the agent is approaching the model's context window limit, which
degrades performance.

```
{ "kind": "TokenUsage", "plan_id": "01-workspace",
  "model": "claude-sonnet-4-20250514", "tokens_used": "180000" }
```

The conductor can respond by:

1. Escalating to a model with a larger context window
2. Splitting the task into smaller subtasks
3. Pruning context to reduce token usage

---

## Yerkes-Dodson Dynamics

The Conductor implements Yerkes-Dodson pressure dynamics [Yerkes, R.M. &
Dodson, J.D. "The Relation of Strength of Stimulus to Rapidity of
Habit-Formation." *J. Comparative Neurology and Psychology*, 18(5):459-482,
1908]:

> Moderate pressure maximizes multi-agent cooperation; extreme pressure
> collapses it.

The Daimon's arousal dimension maps to this pressure model:

| Arousal Level | Behavior | Conductor Action |
|---------------|----------|------------------|
| Low (< 0.3) | Under-stimulated, over-exploring | Increase urgency signals |
| Moderate (0.3-0.7) | Optimal zone | No intervention |
| High (> 0.7) | Over-stressed, error-prone | Reduce load, pause low-priority plans |

The conductor adjusts pressure by:

1. **Pausing plans**: If too many plans are active and agents are struggling,
   pause lower-priority plans to reduce load
2. **Model escalation**: If an agent repeatedly fails, escalate to a more
   capable model
3. **Task decomposition**: If a task is too complex, suggest splitting
4. **Budget reallocation**: If one plan consumes too much budget, constrain
   it and redistribute

---

## Diagnosis Engine

The DiagnosisEngine correlates multiple watcher findings into actionable
diagnoses:

```rust
let findings = conductor.check_all(&signals);
let diagnosis = diagnosis_engine.diagnose(&findings);
```

Example diagnosis:

```
Watcher: compile_loop (3 consecutive failures on roko-core/src/lib.rs)
Watcher: cost_overrun (plan cost $2.34 exceeds 80% of $3.00 budget)
Diagnosis: Plan 01-workspace is stuck in a compile-fix loop and burning
           budget.
Action: Escalate model from claude-sonnet to claude-opus for the fix task.
```

The DiagnosisEngine applies correlation rules:

| Pattern | Diagnosis | Action |
|---------|-----------|--------|
| compile_loop + cost_overrun | Stuck in expensive retry loop | Escalate model or decompose task |
| silence + progress_stall | Agent is hung or deadlocked | Restart agent or fail the task |
| ghost_turn + context_pressure | Agent looping at context limit | Prune context and retry |
| gate_failure_rate + review_loop | Quality loop not converging | Replan with different approach |
| resource_pressure + deadlock | System overloaded | Pause all but highest-priority plan |

---

## Conductor Decisions

The conductor produces `ConductorDecision` values:

| Decision | Effect |
|----------|--------|
| `Continue` | No intervention needed |
| `PausePlan(plan_id)` | Pause a plan to reduce load |
| `EscalateModel(plan_id, model)` | Use a more capable model |
| `ReplanTask(plan_id, task_id)` | Regenerate the task plan |
| `FailPlan(plan_id, reason)` | Mark a plan as failed |
| `Alert(message)` | Emit an alert for operator attention |

The PlanRunner processes these decisions:

```rust
match decision {
    ConductorDecision::PausePlan(plan_id) => {
        executor.pause_plan(&plan_id)?;
    }
    ConductorDecision::EscalateModel(plan_id, model) => {
        task_tracker.set_task_model_hint(task_id, Some(model))?;
    }
    ConductorDecision::FailPlan(plan_id, reason) => {
        executor.fail_plan(&plan_id, &reason)?;
    }
    ConductorDecision::Alert(msg) => {
        tracing::warn!("Conductor alert: {msg}");
    }
    _ => {}
}
```

---

## Conductor and Coordination

The Conductor connects to the broader coordination system at several points:

### Pheromone Interaction

Conductor alerts function as high-priority pheromone-like signals:

| Alert | Equivalent Pheromone | Effect |
|-------|---------------------|--------|
| cost_overrun | Threat (intensity 0.8) | Agents constrain spending |
| compile_loop | Threat (intensity 0.9) | Agents avoid the broken module |
| gate_failure_rate | Anomaly (intensity 0.7) | Agents investigate quality issues |
| progress_stall | Anomaly (intensity 0.5) | Agents reassess approach |
| context_pressure | Alpha (intensity 0.6) | Agents reduce context size |

### c-factor Integration

The Conductor's watcher outputs feed into the c-factor computation:

- **delivery_rate**: Conductor monitors Bus health directly
- **turn_taking_entropy**: Silence and ghost-turn detectors identify
  monopolized or stalled conversations
- **peer_prediction_accuracy**: Review loop detection signals calibration
  failures

When c-factor drops alongside worsening Conductor alerts, the system has
both a process signal (c-factor) and a mechanism signal (Conductor) pointing
to the same problem -- enabling more targeted intervention.

### Morphogenetic Feedback

Conductor decisions influence morphogenetic specialization:

- If the Conductor repeatedly escalates models for a particular task type,
  this creates selection pressure for agents specializing in that domain
- If budget reallocation consistently favors certain plan types, agents
  specializing in those areas receive more resources (reinforcement)
- Cost overruns in a domain raise the effective "niche cost," discouraging
  new agents from competing in that saturated space

---

## Circuit Breaker

The circuit breaker prevents cascading failures using the Nygard pattern
[Nygard, M.T. *Release It!* Pragmatic Bookshelf, 2007]:

```
States: Closed --> Open --> Half-Open --> Closed
                     ^                      |
                     |______________________|
                       (failure threshold)
```

| State | Behavior |
|-------|----------|
| Closed | Normal operation; failures counted |
| Open | Plan execution blocked; waiting for cooldown |
| Half-Open | Single probe attempt allowed; success resets, failure reopens |

Configuration:

```rust
pub struct CircuitBreakerConfig {
    /// Failures before tripping. Default: 5.
    pub failure_threshold: u32,
    /// Cooldown before half-open probe. Default: 60s.
    pub cooldown: Duration,
    /// Consecutive successes to close. Default: 2.
    pub success_threshold: u32,
}
```

---

## Viable System Model Connection

The Conductor maps to Beer's Viable System Model [Beer, S. *Brain of the
Firm*. Allen Lane, 1972]:

| VSM System | Roko Component |
|-----------|----------------|
| System 1 (Operations) | Graph engine, agent dispatch |
| System 2 (Coordination) | Pheromone field, morphogenetic specialization |
| System 3 (Control) | **Conductor** -- operational monitoring and intervention |
| System 3* (Audit) | Conductor watchers -- sporadic inspection |
| System 4 (Intelligence) | Learning runtime, research agent |
| System 5 (Policy) | Operator configuration, safety layer |

The Conductor is System 3/3*: it monitors operations (System 1) and
coordination (System 2), intervenes when homeostasis is threatened, and
conducts sporadic inspections through the watcher pattern.

The Good Regulator theorem [Conant, R.C. & Ashby, W.R. "Every Good Regulator
of a System Must Be a Model of That System." *Int. J. Systems Science*,
1(2):89-97, 1970] requires that the Conductor maintains a model of the
orchestration system -- signal patterns, cost trends, progress rates -- to
regulate it effectively. The 12 watchers collectively form this model.

---

## Implementation Status

| Component | Status | Source |
|-----------|--------|--------|
| Conductor struct with 12 watchers | Wired | `roko-conductor/src/lib.rs` |
| WatcherRunner background task | Wired | `roko-cli/src/runner/` |
| DiagnosisEngine | Wired | `roko-conductor/src/` |
| Circuit breaker | Wired | `roko-conductor/src/` |
| ConductorDecision processing | Wired | PlanRunner event loop |
| Cost monitoring | Wired | Efficiency events integration |
| Context pressure detection | Wired | Token usage signals |
| c-factor integration | Partial | CFactorSummary computed but not fed back to Conductor |

---

## Cross-References

- `12-collective-intelligence-metrics.md` -- c-factor axes that Conductor
  data feeds
- `11-exponential-flywheel.md` -- Conductor interventions prevent flywheel
  degradation
- [07-GATES](../../07-GATES.md) -- Gate pipeline that Conductor monitors
- [08-LEARNING](../../08-LEARNING.md) -- Learning runtime that Conductor
  diagnosis feeds
- [11-AUTONOMY](../../11-AUTONOMY.md) -- Daimon arousal maps to
  Yerkes-Dodson dynamics

---

## References

- [Beer 1972] *Brain of the Firm: The Managerial Cybernetics of
  Organization*, Allen Lane
- [Conant & Ashby 1970] Every Good Regulator of a System Must Be a Model
  of That System, *Int. J. Systems Science*, 1(2):89-97
- [Nygard 2007] *Release It! Design and Deploy Production-Ready Software*,
  Pragmatic Bookshelf
- [Yerkes & Dodson 1908] The Relation of Strength of Stimulus to Rapidity
  of Habit-Formation, *J. Comparative Neurology and Psychology*, 18(5):459-482
