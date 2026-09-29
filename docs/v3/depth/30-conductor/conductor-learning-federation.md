# Conductor Learning, Federation, and Self-Healing

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 16.
> Source: `crates/roko-learn/src/conductor.rs`, `crates/roko-conductor/src/`

---

## 1. The Learning Gap

The conductor uses static thresholds. `MAX_GHOST_TURNS=3`. `WorstSeverityPolicy`.
These constants were calibrated from production batch runs in March-April 2026 and
they work for that workload. But workloads change. Model versions change. Codebase
complexity changes. A threshold that was correct last month may be too strict or
too lenient today.

The learning infrastructure exists. `ConductorBandit` in
`roko-learn/src/conductor.rs` implements a contextual bandit for intervention
selection. The efficiency event pipeline records every agent turn with 20+ fields
of outcome data. The cascade router already uses Thompson Sampling to learn
model-task mappings. The conductor's decision path does not use any of this.

The gap: the conductor collects data but does not learn from it. Interventions are
rule-driven, not data-driven. Closing this gap means wiring the bandit into the
conductor's `evaluate()` path, replacing `WorstSeverityPolicy` with a learned
policy that falls back to static rules when confidence is low.

---

## 2. Contextual Bandit for Intervention Selection

The bandit models intervention selection as a contextual multi-armed bandit problem.

### 2.1 State

19-dimensional feature vector extracted from watcher outputs and execution context:

- Iteration number (how many gate-fail cycles so far)
- Failure count (total failures in this plan attempt)
- Elapsed milliseconds (wall-clock time since task start)
- Accumulated cost in USD
- Model tier (0=haiku, 1=sonnet, 2=opus)
- Task complexity (from TOML frontmatter: 0=trivial, 1=simple, 2=standard,
  3=complex)
- Error pattern hash (which error categories have appeared)
- Interaction terms: iteration x failure_count, cost x complexity,
  elapsed x model_tier

Interaction terms matter because the right intervention depends on combinations. A
high iteration count alone might mean "keep trying." A high iteration count
combined with rising cost means "abort."

### 2.2 Actions

Continue, InjectHint, SwitchModel, Restart, Abort.

### 2.3 Algorithm

Thompson Sampling blended with a linear context model. 65% Thompson (exploration),
35% linear (exploitation from context features). The blend prevents the bandit
from over-exploiting early patterns while still using context to make informed
decisions.

```rust
/// Learned conductor policy using contextual bandits.
/// Replaces static WorstSeverityPolicy with data-driven decisions.
pub struct LearnedConductorPolicy {
    /// The underlying bandit that selects actions.
    bandit: ConductorBandit,
    /// Minimum confidence before overriding static policy.
    /// Below this, fall back to WorstSeverityPolicy.
    min_confidence: f64,  // default: 0.6
    /// Number of observations before learning activates.
    warmup_observations: usize,  // default: 50
}

impl InterventionPolicy for LearnedConductorPolicy {
    fn evaluate(
        &self,
        outputs: &[WatcherOutput],
        ctx: &Context,
    ) -> ConductorDecision {
        let features = self.extract_features(outputs, ctx);

        if self.bandit.total_observations() < self.warmup_observations {
            return WorstSeverityPolicy.evaluate(outputs, ctx);
        }

        let (action, confidence) =
            self.bandit.select_with_confidence(&features);

        if confidence < self.min_confidence {
            return WorstSeverityPolicy.evaluate(outputs, ctx);
        }

        action.to_conductor_decision(outputs)
    }
}
```

The warmup period (50 observations) prevents the bandit from making decisions
before it has enough data. After warmup, the bandit selects actions but defers to
the static policy whenever its confidence falls below 0.6. This two-tier fallback
means the learned policy can only override static rules when it has both sufficient
data and sufficient confidence.

---

## 3. Reward Shaping

Defining good rewards for conductor actions is the hard part. The naive approach --
reward 1.0 for success, 0.0 for failure -- does not capture the nuance. A
well-timed Abort on a futile plan is a good outcome: it saves tokens and frees the
executor to work on plans that can succeed.

| Action | Outcome | Reward |
|--------|---------|--------|
| Continue | Next gate passes | 0.9 |
| Continue | Next gate fails | 0.1 |
| Restart | Restarted agent succeeds | 0.8 |
| Restart | Restarted agent also fails | 0.2 |
| Fail | Plan was later retried and failed again | 0.7 (correct fail-fast) |
| Fail | Plan was later retried and succeeded | 0.1 (premature failure) |

Three design decisions:

**Continue-pass gets 0.9, not 1.0.** Reserving 1.0 prevents reward saturation. The
bandit can always find room to improve.

**Fail-correct gets 0.7.** A correct Abort is valuable but not as valuable as a
successful Continue or Restart. The system should prefer actions that lead to
success over actions that correctly predict failure. But correct failure prediction
still earns substantial reward because it saves tokens and wall-clock time.

**Fail-premature gets 0.1, not 0.0.** The plan was recoverable but the conductor
gave up too early. The low reward (but not zero) prevents the bandit from
completely avoiding Fail actions.

The Restart-fail reward (0.2) is higher than Continue-fail (0.1) because a restart
at least attempted a different strategy. Rewarding attempted recovery over passive
continuation encourages the bandit to try restarts when it detects problems, even
if restarts do not always succeed.

---

## 4. Online Learning Loop

The learning loop closes within the conductor's evaluation cycle:

```
Agent turn completes
    -> Conductor evaluates (bandit selects action)
    -> Action executed (Continue/Restart/Fail)
    -> Outcome observed (next gate result)
    -> Bandit updated with (state, action, reward)
    -> Policy improves
```

The delay between action and reward varies by action type. Continue rewards arrive
on the next turn (fast feedback). Restart rewards arrive after the restarted agent
completes (slower). Fail rewards arrive only when the plan is retried (possibly
never, if the circuit breaker trips). This variable delay means the bandit must
handle sparse and delayed rewards for Fail actions. The implementation queues
pending rewards and resolves them when outcomes become available.

---

## 5. Conductor Federation -- Multi-Level Control

### 5.1 Four-Level Federation Architecture

The current conductor operates at a single level: per-task. Federation puts a
conductor at each level:

```
+----------------------------------------------------+
|  L4: Fleet Conductor (cross-plan, per-batch)       |
|  Scope: All plans in a session                      |
|  Signals: Plan outcomes, fleet-level metrics        |
|  Actions: Router policy updates, global budgets     |
|                                                      |
|  +----------------------------------------------+  |
|  |  L3: Plan Conductor (per-plan)               |  |
|  |  Scope: All tasks in one plan                 |  |
|  |  Signals: Task outcomes, plan-level cost       |  |
|  |  Actions: Resource reallocation, priority      |  |
|  |                                                |  |
|  |  +----------------------------------------+  |  |
|  |  |  L2: Task Conductor (per-task)         |  |  |
|  |  |  Current roko-conductor                |  |  |
|  |  |  12 watchers + circuit breaker         |  |  |
|  |  |  Continue / Restart / Fail              |  |  |
|  |  |                                        |  |  |
|  |  |  +--------------------------------+   |  |  |
|  |  |  |  L1: Turn Conductor            |   |  |  |
|  |  |  |  AnomalyDetector               |   |  |  |
|  |  |  |  Prompt loop, cost spike       |   |  |  |
|  |  |  +--------------------------------+   |  |  |
|  |  +----------------------------------------+  |  |
|  +----------------------------------------------+  |
+----------------------------------------------------+
```

**L1 (Turn)** operates at the granularity of a single agent turn. The
`AnomalyDetector` already does this -- it checks prompt hashes, cost spikes, and
quality degradation before each turn.

**L2 (Task)** is the current conductor. Twelve watchers, one circuit breaker, one
intervention policy. This is what `roko-conductor` implements today.

**L3 (Plan)** observes all tasks within one plan. It sees patterns that L2 cannot:
task A failed with a compile error, task B depends on the code that A was supposed
to write, so B will also fail. L3 can reallocate resources or reprioritize.

**L4 (Fleet)** observes all plans in a session. It sees cross-plan patterns: three
authentication-related plans failed this batch, suggesting a systemic issue. L4 can
update router policies globally or halt entire categories of work.

### 5.2 Conductor Trait at Each Level

All four levels implement the same trait:

```rust
/// All conductor levels implement the same trait.
/// Federation is achieved through composition, not hierarchy.
pub trait ConductorLevel: Send + Sync {
    /// The scope of signals this conductor observes.
    fn scope(&self) -> ConductorScope;

    /// Evaluate the signal stream and produce decisions.
    fn evaluate(
        &self,
        stream: &[Signal],
        ctx: &Context,
    ) -> Vec<ConductorDecision>;

    /// Accept parameter updates from the level above.
    fn accept_parameters(&mut self, params: &ParameterUpdate);

    /// Emit observations for the level above.
    fn emit_observations(&self) -> Vec<Signal>;
}

pub enum ConductorScope {
    Turn,    // L1: per-agent-turn signals
    Task,    // L2: per-task signals (current conductor)
    Plan,    // L3: per-plan signals
    Fleet,   // L4: cross-plan signals
}
```

### 5.3 Communication via Signal Stream

All conductors communicate through the same signal stream. Signal tags encode the
level and type:

- L1 emits `conductor.anomaly.prompt_loop`, `conductor.anomaly.cost_spike`
- L2 reads L1 signals and emits `conductor.intervention.restart`,
  `conductor.intervention.fail`
- L3 reads L2 signals and emits `conductor.plan.budget_realloc`,
  `conductor.plan.reprioritize`
- L4 reads L3 signals and emits `conductor.fleet.policy_update`,
  `conductor.fleet.budget_adjust`

Each level filters the stream by tag prefix. No special hierarchy protocol is
needed. The signal stream is the communication channel.

### 5.4 VSM Mapping

Each federation level maps to a system in Beer's Viable System Model:

| Level | VSM System | Function |
|-------|-----------|----------|
| L1 (Turn) | System 2 | Coordination -- prevent oscillations within a turn |
| L2 (Task) | System 3 | Control -- internal oversight of task execution |
| L3 (Plan) | System 3* | Audit -- independent check of plan progress |
| L4 (Fleet) | System 4 | Intelligence -- scanning cross-plan patterns for adaptation |

System 5 (policy) is not a conductor level -- it is the human operator who sets
the constraints within which all four levels operate: `roko.toml` configuration,
plan definitions, and acceptance criteria.

---

## 6. Self-Healing Conductor

### 6.1 Conductor Failure Modes

The conductor can fail. Its thresholds can drift. Its model can go stale. Its
watchers can develop blind spots. Four failure modes:

| Failure | Symptom | Detection | Recovery |
|---------|---------|-----------|----------|
| Threshold drift | Good plans get killed (false positives) | Intervention effectiveness drops below 50% | Bayesian threshold adaptation |
| Model staleness | Conductor interventions have no effect | Restart success rate unchanged from continue | Re-calibrate from recent efficiency events |
| Watcher blindness | New failure mode not caught by any watcher | Plans fail without intervention | Unclassified error clustering in efficiency logs |
| Circuit breaker stuck | Plans permanently tripped that should retry | Tripped plans with changed environment | Auto-probe after sleep window (half-open state) |

### 6.2 Recovery-Oriented Computing Principles

The self-healing conductor borrows four principles from Patterson et al.'s
Recovery-Oriented Computing:

**Make restart cheap.** A conductor threshold reset is cheap: update a constant, no
process restart needed. The conductor can recalibrate its thresholds without
interrupting ongoing execution.

**Test recovery paths.** The self-model accuracy metrics validate that recovery
mechanisms work. If intervention effectiveness drops, the system knows its recovery
path (restart) is not effective.

**Micro-reboots.** Reset individual watcher thresholds without resetting the entire
conductor. If the ghost-turn watcher is too aggressive, recalibrate that one
threshold.

**Survivor functions.** Conductor state -- circuit breaker records, watcher history,
bandit weights -- persists through process restarts via `.roko/state/`. The
conductor does not start from zero on every restart.

```rust
/// Self-healing conductor that detects and repairs its own model drift.
pub struct SelfHealingConductor {
    /// The underlying conductor with all watchers and policies.
    inner: Conductor,
    /// Self-model accuracy tracker.
    accuracy: SelfModelAccuracy,
    /// Threshold learner for adaptive calibration.
    threshold_learner: ThresholdLearner,
    /// Minimum accuracy before triggering self-repair.
    min_accuracy: f64,  // default: 0.5
    /// Interval between self-assessments.
    self_check_interval: Duration,  // default: 300s (5 min)
}

impl SelfHealingConductor {
    pub fn self_assess(&mut self) -> Option<SelfRepairAction> {
        if self.accuracy.intervention_effectiveness < self.min_accuracy {
            return Some(SelfRepairAction::RecalibrateThresholds);
        }
        if self.accuracy.stuck_detection_precision < 0.3 {
            return Some(SelfRepairAction::ExpandStuckHeuristics);
        }
        if self.accuracy.undetected_failure_rate() > 0.2 {
            return Some(SelfRepairAction::AddNewWatcher);
        }
        None
    }
}

pub enum SelfRepairAction {
    RecalibrateThresholds,
    ExpandStuckHeuristics,
    AddNewWatcher,
    ResetCircuitBreakers,
    RetrainBandit,
}
```

### 6.3 Priority Order

1. **Intervention effectiveness below 50%.** The most common failure mode and the
   cheapest to fix. Recovery: `RecalibrateThresholds` triggers the
   `ThresholdLearner` to adjust thresholds based on recent outcome data.

2. **Stuck detection precision below 30%.** More than 70% of "stuck" detections are
   false positives. Recovery: `ExpandStuckHeuristics` tightens the stuck detection
   thresholds so they trigger less often.

3. **Undetected failure rate above 20%.** More than one in five plan failures occurs
   without any conductor intervention. The watchers have a blind spot. Recovery:
   `AddNewWatcher` clusters unclassified errors from the efficiency logs and
   proposes a new watcher pattern.

---

## 7. Triple-Loop Learning

Self-healing operates at three levels of abstraction, following Argyris and Schon's
organizational learning framework.

```
Loop 1 (Single-loop): Correct errors
    Agent fails -> Conductor restarts -> Agent succeeds
    The system fixes the immediate problem.

Loop 2 (Double-loop): Change the rules
    Conductor thresholds produce too many false positives
    -> ThresholdLearner adjusts thresholds
    -> Future interventions are more accurate
    The system improves its own detection.

Loop 3 (Triple-loop): Change the meta-rules
    The threshold learning rate is too slow (or too fast)
    -> Self-model accuracy metrics detect the meta-problem
    -> Learning parameters are adjusted
    The system improves its own improvement process.
```

**Single-loop** is what the conductor does today. An agent fails. The conductor
detects the failure pattern. It restarts the agent or fails the plan. No learning
occurs -- the same threshold, the same watcher, the same intervention.

**Double-loop** changes the thresholds. The `ThresholdLearner` tracks intervention
effectiveness per watcher. If the ghost-turn watcher's interventions succeed 90% of
the time, its threshold might be too lenient (only catching obvious cases). If
interventions succeed 30% of the time, the threshold is too strict (too many false
positives). The learner adjusts toward the sweet spot.

**Triple-loop** changes the learning process itself. The self-model accuracy metrics
track whether the double-loop is converging. If threshold adjustments are
oscillating (too strict, then too lenient, then too strict again), the learning rate
is too high. If thresholds barely move despite clear evidence of drift, the learning
rate is too low. The triple-loop adjusts the learning rate, the discount factor, and
the minimum sample size for the `ThresholdLearner`.

The practical test for whether triple-loop learning is needed: does intervention
effectiveness stabilize after double-loop adjustments? If it does, double-loop is
sufficient. If it oscillates or fails to converge, the learning parameters
themselves need adjustment -- and that is the triple-loop.

---

## 8. File Reference

| File | What |
|------|------|
| `crates/roko-learn/src/conductor.rs` | ConductorBandit (built, not wired into live evaluate()) |
| `crates/roko-conductor/src/conductor.rs` | Current static conductor |
| `crates/roko-conductor/src/interventions.rs` | InterventionPolicy trait |
| `crates/roko-learn/src/anomaly.rs` | AnomalyDetector (L1 conductor) |
| `crates/roko-learn/src/efficiency.rs` | AgentEfficiencyEvent (reward data) |
| `crates/roko-learn/src/cascade_router.rs` | CascadeRouter (L4 conductor analogue) |
