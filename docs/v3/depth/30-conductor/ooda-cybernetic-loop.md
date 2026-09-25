# OODA Cybernetic Loop -- 4 Phases

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 8.
> Source: `crates/roko-conductor/src/conductor.rs`,
>         `crates/roko-conductor/src/stuck_detection.rs`

---

## 1. The OODA Framework

Boyd's OODA loop (Observe-Orient-Decide-Act) provides the conceptual framework for
the Conductor's evaluation cycle. Each conductor tick maps directly to one OODA
iteration.

> "The key to victory is operating at a faster tempo than the adversary."
> -- Boyd (1987), *A Discourse on Winning and Losing*

For the Conductor, the adversary is drift -- the gap between what agents are doing
and what they should be doing. The goal is to update the world model faster than
agent behavior can diverge from the plan.

---

## 2. The Four Phases

### 2.1 Observe

The signal stream is the observation input. Every agent turn, gate result, phase
transition, cost event, and timing measurement produces a Signal.

Signals consumed:
- `TokenUsage` -- token counts per turn
- `GateVerdict` -- gate pass/fail with structured results
- `AgentOutput` -- agent turn content
- `PlanPhase` -- phase transition events
- `Metric` -- numeric measurements (cost, drift, coverage)
- `Custom("conductor.agent_output")` -- timing data

The observation phase is pure reading. No state is modified. No decisions are made.

### 2.2 Orient

Orientation is where raw observations become assessments. Each watcher transforms
raw signals into structured evaluations:

- Ghost turn watcher: "Agent 7 produced zero output for 3 consecutive turns"
- Cost overrun watcher: "Plan 12 has spent $8.40 of its $10.00 budget"
- Spec drift watcher: "Plan 3 has 32% file changes outside declared scope"

Orientation also includes the stuck detector's meta-cognition assessment and the
diagnosis engine's error classification. Raw data becomes typed assessments.

The orient phase corresponds to the `check_all()` method -- running all watchers
against the signal stream.

### 2.3 Decide

The intervention policy resolves multiple watcher assessments into a single
decision. `WorstSeverityPolicy` selects the maximum severity:

```
Input:  [Warning(compile-fail), Warning(spec-drift)]
Output: ConductorDecision::Restart { reason: "..." }
```

The circuit breaker also participates: a tripped plan produces `Fail` regardless of
watcher outputs.

### 2.4 Act

The Conductor does not act directly. It returns a `ConductorDecision` to the
orchestrator:

| Decision | Orchestrator Action |
|----------|-------------------|
| Continue | Do nothing -- proceed |
| Restart | Kill agent, prepare error context, spawn fresh agent |
| Fail | Cancel in-flight tasks, mark plan Failed, move on |

This separation of decision from action is deliberate. The Conductor operates
purely on the signal stream and produces pure decisions. The orchestrator translates
decisions into effects.

---

## 3. Cybernetic Structure

The Conductor implements a cybernetic feedback loop in the classical sense (Wiener,
1948):

```
+---------------+     Signals      +---------------+
|               | ----------------> |               |
|  Execution    |                   |  Conductor    |
|  (Agents,     |                   |  (Watchers,   |
|   Gates,      |  <-------------- |   Policy,     |
|   Merges)     |   Decision        |   Breaker)    |
|               |                   |               |
+---------------+                   +---------------+
```

**Sensor**: Signal stream (observes execution state)
**Comparator**: Watchers (compare observed state to thresholds)
**Controller**: Intervention policy (decides corrective action)
**Actuator**: Orchestrator (executes the decision)
**Environment**: Agents + codebase + gates (the system being regulated)

### 3.1 Negative Feedback

The Conductor implements negative feedback -- it acts to reduce deviation from the
desired state. When spec drift exceeds 25%, the intervention pushes toward in-scope
work. When cost exceeds budget, it pushes toward termination.

### 3.2 Positive Feedback (Absent by Design)

The Conductor does not implement positive feedback -- it does not amplify trends.
Positive feedback lives in the learning system: successful patterns promote to
playbook rules. The Conductor's job is stability, not optimization.

---

## 4. Feedback Loop Frequency

Three frequencies provide layered coverage:

| Frequency | What Runs | Catches |
|-----------|----------|---------|
| Per-event | All 12 watchers | Task-level anomalies |
| Every 10s | Health monitor | Infrastructure failures |
| Theta | MetaCognitionHook | Stuck agents between events |

**Per-event evaluation**: The orchestrator calls `evaluate()` after significant
events -- agent turn completion, gate result, phase transition.

**Periodic health check**: Fixed interval independent of events. Catches
infrastructure problems that do not produce events.

**Theta-frequency meta-cognition**: Medium granularity self-assessment.

---

## 5. Closed-Loop Properties

### 5.1 Stability

The feedback loop is stable because:

1. **Bounded responses**: Every decision is one of three options
2. **Cooldown periods**: Watchers cannot fire again for 120s per plan
3. **Circuit breaker**: After two failures, permanent fail
4. **Monotonic progress**: Failed plans do not re-enter automatically

### 5.2 Observability

Every decision produces a signal that enters the stream. The Conductor's own
behavior is observable -- dashboard, signal replay, and learning system can all
inspect its decisions.

### 5.3 Latency

Total per-evaluation latency is < 2 ms. Negligible compared to agent turn times
(seconds to minutes).

---

## 6. Nested OODA Loops -- Multi-Timescale Control

A single OODA loop is insufficient for a system operating across multiple
timescales. Agent turns happen in seconds, tasks take minutes, plans run for hours.

### 6.1 Three-Level Nesting

```
+-----------------------------------------------------+
|  Delta Loop (Strategic)                               |
|  Period: per-batch (hours)                            |
|  Orient: cross-plan patterns, model effectiveness     |
|  Decide: cascade router updates, threshold tuning     |
|                                                       |
|  +----------------------------------------------+   |
|  |  Theta Loop (Operational)                     |   |
|  |  Period: per-task (minutes)                    |   |
|  |  Orient: MetaCognitionHook assessment          |   |
|  |  Decide: strategy adjustment, escalation       |   |
|  |                                                |   |
|  |  +----------------------------------------+  |   |
|  |  |  Gamma Loop (Tactical)                  |  |   |
|  |  |  Period: per-turn (seconds)              |  |   |
|  |  |  Orient: all 12 watchers                 |  |   |
|  |  |  Decide: intervention policy             |  |   |
|  |  +----------------------------------------+  |   |
|  +----------------------------------------------+   |
+-----------------------------------------------------+
```

### 6.2 Parameter Cascade

Slower loops set the parameters for faster loops. This is the foundational
principle from hierarchical control theory (Mesarovic et al., 1970):

- **Delta sets Theta**: adaptive gate thresholds, default model tier, cost budgets
- **Theta sets Gamma**: adjusted watcher thresholds, intervention cooldown periods

The cascade flows one direction: slow to fast. If the Gamma loop detects something
requiring a strategic response, it emits a signal. The Delta loop picks it up at
its own pace.

### 6.3 Singular Perturbation

When timescales are well-separated (~15x between adjacent levels), each loop can be
analyzed independently. The Gamma loop treats Theta parameters as constants. This
makes the hierarchical architecture tractable.

---

## 7. Algedonic Signals -- Priority Interrupts

Algedonic signals (Beer, 1972) bypass the normal hierarchy. Four conditions trigger
algedonic escalation:

1. **Runaway cost**: total session cost exceeds 2x budget before 50% of wall time
2. **Safety violation**: modification outside declared workspace scope
3. **Total infrastructure failure**: all agents down simultaneously
4. **Operator interrupt**: Ctrl+C or shutdown command

Each layer gets a bounded window to respond before escalation:

```
Agent detects anomaly -> Conductor has 5s to respond
    | (no response)
Orchestrator has 30s to respond
    | (no response)
Policy layer triggers emergency shutdown
```

---

## 8. Implicit Guidance and Control (IG&C)

Boyd described a shortcut: when the Orient phase recognizes a well-known pattern, it
can bypass full Decide and jump straight to Act. Pre-compiled rules for common
failure patterns:

```rust
pub struct ImplicitRule {
    pub name: &'static str,
    pub matcher: Box<dyn Fn(&[Signal]) -> bool + Send + Sync>,
    pub action: ConductorDecision,
    pub min_confidence: f64,
}
```

IG&C rules should be extracted from the ConductorBandit's converged actions. When a
bandit arm converges to >95% selection rate, that pattern graduates to an IG&C rule.

---

## 9. References

- Boyd, J. (1987). *A Discourse on Winning and Losing*. OODA loop, tempo, IG&C.
- Beer, S. (1972). *Brain of the Firm*. Viable System Model, algedonic signals.
- Wiener, N. (1948). *Cybernetics*. Feedback loops, homeostatic regulation.
- Mesarovic, M., Macko, D., & Takahara, Y. (1970). *Theory of Hierarchical,
  Multilevel Systems*. Parameter cascade.
- Klein, G. (1998). *Sources of Power*. Recognition-Primed Decision model.

---

## 10. File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/conductor.rs` | The OODA loop implementation (evaluate()) |
| `crates/roko-conductor/src/stuck_detection.rs` | MetaCognitionHook (Theta assessment) |
| `crates/roko-conductor/src/health.rs` | Health monitor (periodic check) |
| `crates/roko-conductor/src/interventions.rs` | Decision resolution (Orient -> Decide) |
