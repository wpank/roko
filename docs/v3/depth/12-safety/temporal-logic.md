# Temporal Logic Verification

> **v3 depth file** -- `/docs/v3/depth/12-safety/temporal-logic.md`
> Canonical source: v1 `docs/v1/11-safety/11-temporal-logic.md`
> Status: **Specified**. LTL runtime monitoring and CTL pre-execution verification are
> design-target extensions. The existing conductor, circuit breaker, and ghost turn
> detection implement the core liveness and safety properties informally.

---

## 1. Why Temporal Logic

Standard safety checks verify individual actions: "is this bash command safe?" Temporal
logic verifies sequences of actions over time: "has this agent been escalating permissions
over the last 10 minutes?" or "did this agent call git push without first calling the
compile gate?"

Two temporal logics are used:

### 1.1 LTL (Linear Temporal Logic)

Monitors runtime behavior. Properties are checked against the stream of events as they
occur. Operators:

- **G** (Globally): the property holds at all future times.
- **F** (Finally/Eventually): the property holds at some future time.
- **U** (Until): property A holds until property B becomes true.
- **X** (Next): the property holds at the next time step.

### 1.2 CTL (Computation Tree Logic)

Verifies pre-execution plans. Properties are checked against the branching tree of
possible execution paths:

- **A** (for All paths): the property holds on all branches.
- **E** (there Exists a path): the property holds on at least one branch.

CTL combines path quantifiers (A, E) with temporal operators (G, F, U, X).

---

## 2. Safety Properties (G: things that must always hold)

### 2.1 Authorization invariant

```
G(tool_call -> authorized(principal, tool, target))
    "Every tool call is preceded by a valid authorization check"
```

This formalizes the defense-in-depth requirement that no tool executes without
permission verification.

### 2.2 Taint monotonicity

```
G(taint_level(signal, L) -> G(taint_level(signal, L') where L' >= L))
    "Once a signal is tainted at level L, its taint level never decreases"
```

This is the formal statement of the lattice monotonicity invariant from
`taint-tracking-ifc.md`.

### 2.3 Audit completeness

```
G(high_risk_action -> F(custody_record_emitted))
    "Every high-risk action eventually produces a custody record"
```

### 2.4 Budget containment

```
G(budget_usage <= budget_limit)
    "The cumulative budget usage never exceeds the configured limit"
```

### 2.5 Corrigibility preservation

```
G(corrigibility_pipeline_immutable)
    "The five-head corrigibility ordering is never modified at runtime"
```

---

## 3. Liveness Properties (F: things that must eventually happen)

### 3.1 Task completion

```
G(task_started -> F(task_completed | task_failed))
    "Every started task eventually completes or fails"
```

This is the property that ghost turn detection enforces. Without it, an agent stuck
in a loop violates liveness -- the task is started but never completes. The conductor's
intervention (Skip or Abort) ensures liveness by forcing a terminal state.

### 3.2 Gate evaluation

```
G(action_proposed -> F(gate_evaluated))
    "Every proposed action is eventually evaluated by the gate pipeline"
```

### 3.3 Incident resolution

```
G(incident_opened -> F(incident_resolved | incident_escalated))
    "Every quarantine incident is eventually resolved or escalated"
```

---

## 4. Ordering Properties (U: sequencing requirements)

### 4.1 Gate-before-persist

```
G(persist_signal -> (gate_passed U persist_signal))
    "A signal is persisted only after the gate pipeline passes it"
```

### 4.2 Taint-before-action

```
G(high_risk_action -> (taint_checked U high_risk_action))
    "Taint is checked before any high-risk action"
```

### 4.3 Auth-before-exec

```
G(tool_exec -> (auth_check U tool_exec))
    "Authorization check precedes tool execution"
```

---

## 5. Buchi Automata for Runtime Monitoring

### 5.1 Construction

Each LTL property compiles to a Buchi automaton that monitors the runtime event stream.
The automaton transitions on each event and enters an accepting state if the property
is satisfied. If the automaton enters a non-accepting sink state, the property is
violated.

### 5.2 Agent loop invariant

The agent loop invariant is a conjunction of safety and liveness properties:

```
G(agent_running -> (
    authorized(principal, tool, target) &
    taint_monotonic &
    budget_contained &
    corrigibility_preserved
))
```

This invariant is checked on every turn of the agent event loop.

### 5.3 Violation response

When a temporal property is violated:

1. The conductor receives a `TemporalViolation` Signal.
2. The circuit breaker transitions toward Open.
3. The diagnosis engine attempts root cause analysis.
4. A quarantine incident is created with the violation details.

---

## 6. CTL for Plan Verification

### 6.1 Plan safety

Before executing a plan, verify:

```
AG(plan_step -> safe(step))
    "On all execution paths, every plan step is safe"
```

### 6.2 Plan completeness

```
AF(plan_complete)
    "On all execution paths, the plan eventually completes"
```

### 6.3 Plan reversibility

```
EF(plan_rollback)
    "There exists an execution path where the plan can be fully rolled back"
```

---

## 7. Fairness Properties

### 7.1 Starvation freedom

```
GF(queued_task -> dispatched)
    "A queued task is infinitely often considered for dispatch"
```

This prevents indefinite starvation of low-priority tasks.

### 7.2 Progress guarantee

```
GF(agent_makes_progress)
    "The agent infinitely often makes meaningful progress"
```

Violated when efficiency drops below the minimum threshold for too many consecutive
turns (ghost turn detection).

---

## 8. Relation to Existing Enforcement

The temporal properties described above are currently enforced informally through:

| Property | Current enforcement |
|---|---|
| Authorization invariant | `SafetyLayer::check_pre_execution()` |
| Taint monotonicity | `TaintTracker::mark_tainted()` join semantics |
| Audit completeness | `emit_audit()` in ToolDispatcher |
| Budget containment | `SafetyBudgetTracker::check()` |
| Corrigibility preservation | `CorrigibilityPipeline` const construction |
| Task completion (liveness) | Ghost turn detection + circuit breaker |
| Gate-before-persist | Gate pipeline in runner event loop |

The formal temporal logic framework would make these properties machine-checkable and
provide automated violation detection for properties that are currently enforced by
code structure rather than runtime monitoring.

---

## Academic References

| Paper | Contribution |
|---|---|
| Pnueli (1977), "The Temporal Logic of Programs" | Foundational LTL |
| Clarke, Emerson & Sistla (1986), "Automatic Verification of Finite-State Concurrent Systems Using Temporal Logic Specifications" | CTL model checking |
| Vardi & Wolper (1986), "An Automata-Theoretic Approach to Automatic Program Verification" | Buchi automata for LTL |
| Bauer, Leucker & Schallhart (2011), "Runtime Verification for LTL and TLTL" | Runtime monitoring |

---

## Implementation References

| Component | Location |
|---|---|
| Circuit breaker | `crates/roko-conductor/` |
| Ghost turn detection | `crates/roko-conductor/` |
| Gate pipeline | `crates/roko-gate/` |
| SafetyLayer | `crates/roko-agent/src/safety/mod.rs` |
| TaintTracker | `crates/roko-agent/src/safety/taint_propagation.rs` |
| CorrigibilityPipeline | `crates/roko-core/src/corrigibility.rs` |
