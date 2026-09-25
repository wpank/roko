# Graduated Interventions -- ConductorDecision and Severity

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 4.
> Source: `crates/roko-conductor/src/interventions.rs`,
>         `crates/roko-core/src/agent.rs`

---

## 1. The ConductorDecision Enum

Every evaluation cycle produces exactly one decision:

```rust
pub enum ConductorDecision {
    Continue,
    Restart { reason: String },
    Fail { reason: String },
}
```

**Continue**: All watchers report healthy. The plan proceeds without intervention.

**Restart**: At least one watcher reported Warning severity. The current agent is
killed and restarted with different context. The key difference from a retry: the
restarted agent gets a FRESH start with ADDITIONAL information about what went wrong.
It is not the same agent continuing from a confused state -- it is a new agent with
the benefit of hindsight.

**Fail**: At least one watcher reported Critical severity, or the circuit breaker is
tripped. The plan is marked as failed. The orchestrator removes it from the merge
queue, cancels in-flight tasks, and dispatches work for other plans.

---

## 2. Why No Nudge

Production experience (Issue #9, agent ghost turns; Issue #6, conductor nudges
without effect) demonstrated that nudging does not work:

```
Agent is stuck -> Conductor sends nudge message ->
Agent reads nudge -> Agent attempts same approach ->
Agent is still stuck -> Conductor sends another nudge -> ...
```

The problem: a confused agent remains confused after receiving a nudge. The nudge
says "you seem stuck, try a different approach" -- but the agent's confusion is
WITHIN its context. Adding a nudge message to an already-confused context does not
reduce confusion. It may even increase it by adding more text the agent needs to
process.

The structural fix: the Conductor does not nudge. It either restarts (kills the
agent, gives a new agent the error analysis) or fails (marks the plan for human
attention). Both actions create a clean break from the confused state.

This is Hard Guarantee 6 from the failure prevention catalog: "The Conductor
DECIDES, Never Nudges."

---

## 3. The Severity System

### 3.1 Three Levels

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info = 0,
    Warning = 1,
    Critical = 2,
}
```

The `PartialOrd` derivation enables severity comparison: `Critical > Warning >
Info`. The intervention policy uses this ordering to select the maximum severity.

### 3.2 Mapping to Decisions

| Severity | Decision | Orchestrator Action |
|----------|----------|-------------------|
| Info | Continue | No action. Log the observation. |
| Warning | Restart | Kill current agent. Spawn fresh agent with error context. |
| Critical | Fail | Mark plan as failed. Cancel in-flight work. Move to next plan. |

### 3.3 Watcher Severity Defaults

| Watcher | Default Severity | Rationale |
|---------|-----------------|-----------|
| ghost-turn | Warning | Agent may recover; fresh start often helps |
| compile-fail-repeat | Warning | Different context may resolve the error |
| cost-overrun | Warning | May be worth one more attempt with budget awareness |
| iteration-loop | **Critical** | Three gate failures = fundamental mismatch |
| review-loop | Warning | Skip reviews and proceed to merge |
| spec-drift | Warning | Refocus the agent on declared scope |
| stuck-pattern | Warning | Fresh agent with different strategy |
| test-failure-budget | Warning | Agent is introducing regressions; needs restart |
| time-overrun | Warning | Early warning; may still finish in time |
| context-window-pressure | Warning | Compact context and retry |

Only `iteration-loop` defaults to Critical. Every other watcher produces Warning,
giving the plan one chance to recover through restart before being failed.

---

## 4. WatcherOutput

The intermediate representation between watcher signals and conductor decisions:

```rust
pub struct WatcherOutput {
    pub watcher: String,       // which watcher fired
    pub severity: Severity,    // info / warning / critical
    pub description: String,   // human-readable explanation
    pub metric: Option<f64>,   // optional numeric value (ratio, count, etc.)
}
```

The Conductor collects `WatcherOutput`s from all watchers that fired, then passes
the collection to the `InterventionPolicy` for resolution.

---

## 5. The InterventionPolicy Trait

```rust
pub trait InterventionPolicy: Send + Sync {
    fn evaluate(
        &self,
        outputs: &[WatcherOutput],
        ctx: &Context,
    ) -> ConductorDecision;
}
```

### 5.1 WorstSeverityPolicy (Default)

```rust
pub struct WorstSeverityPolicy;

impl InterventionPolicy for WorstSeverityPolicy {
    fn evaluate(&self, outputs: &[WatcherOutput], _ctx: &Context)
        -> ConductorDecision
    {
        if outputs.is_empty() {
            return ConductorDecision::Continue;
        }
        let worst = outputs.iter().map(|o| o.severity).max()
            .unwrap_or(Severity::Info);
        match worst {
            Severity::Info => ConductorDecision::Continue,
            Severity::Warning => ConductorDecision::Restart {
                reason: format_watcher_reasons(outputs),
            },
            Severity::Critical => ConductorDecision::Fail {
                reason: format_watcher_reasons(outputs),
            },
        }
    }
}
```

Conservative: if ANY watcher reports a problem, the Conductor acts on it. One
watcher saying "critical" overrides nine watchers saying "continue."

### 5.2 BanditPolicy (Learned)

`BanditPolicy` wraps `ConductorBandit` from `roko-learn` and falls back to
`WorstSeverityPolicy` when confidence is low. The bandit is built but not yet wired
into the live `evaluate()` path. See `conductor-learning-federation.md`.

---

## 6. Decision Flow

The complete decision flow from signal stream to orchestrator action:

```
Signal Stream
    |
    +-- Watcher 1: ghost-turn      -> [no fire]
    +-- Watcher 2: compile-fail    -> Warning: "3 identical E0308 errors"
    +-- Watcher 3: cost-overrun    -> [no fire]
    +-- Watcher 4: iteration-loop  -> [no fire]
    +-- Watcher 5: review-loop     -> [no fire]
    +-- Watcher 6: spec-drift      -> Warning: "drift 32% exceeds 25%"
    +-- ...remaining watchers      -> [no fire]
    |
    v
WatcherOutputs: [
    { watcher: "compile-fail-repeat", severity: Warning, ... },
    { watcher: "spec-drift", severity: Warning, ... },
]
    |
    v
WorstSeverityPolicy: max(Warning, Warning) = Warning
    |
    v
ConductorDecision::Restart {
    reason: "compile-fail-repeat: 3 identical E0308 errors;
             spec-drift: drift 32% exceeds 25%"
}
    |
    v
Orchestrator:
    1. Kill current agent
    2. Record failure in circuit breaker
    3. Spawn new agent with:
       - Error analysis from Diagnosis Engine
       - Updated context with compile error details
       - Refocused scope from spec drift data
```

---

## 7. Escalation Semantics

### 7.1 What Happens on Restart

1. The current agent is terminated. Not paused, not given a final chance --
   terminated.
2. The error context is preserved. Gate results, compiler errors, watcher
   observations, and the reason for restart are collected into an error brief.
3. A new agent is spawned. Fresh context. No memory of the confused state. But it
   receives the error brief.
4. The iteration counter increments. This restart counts toward the plan's iteration
   limit.

### 7.2 What Happens on Fail

1. All in-flight tasks for the plan are cancelled.
2. The plan phase transitions to Failed(reason).
3. The circuit breaker records the failure.
4. The orchestrator moves on to other plans.
5. The failure is surfaced in the dashboard with the full reason.

---

## 8. Cooldown Periods

Each watcher intervention has a built-in cooldown (default 120s per plan per
watcher) to prevent the conductor from firing the same intervention on consecutive
evaluation cycles. This prevents double-fires where the conductor detects a stuck
agent, emits a restart signal, and then on the next tick (before the restart has
taken effect) detects the same signal again.

---

## 9. Yerkes-Dodson Connection

Every Conductor threshold is a position on the Yerkes-Dodson inverted-U curve. Too
aggressive and agents collapse. Too lenient and agents waste. The learning system
provides data for tuning these thresholds over time -- moving along the
Yerkes-Dodson curve toward the peak. See `yerkes-dodson-pressure.md`.

---

## 10. File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/interventions.rs` | Severity, WatcherOutput, InterventionPolicy, WorstSeverityPolicy |
| `crates/roko-conductor/src/conductor.rs` | evaluate() -- decision flow |
| `crates/roko-core/src/agent.rs` | ConductorDecision enum |
