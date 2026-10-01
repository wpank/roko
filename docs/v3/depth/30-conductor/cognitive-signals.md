# Cognitive Signals -- 8 Typed Interrupts

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 10.
> Source: `crates/roko-core/src/agent.rs`,
>         `crates/roko-conductor/src/conductor.rs`

---

## 1. Definition

Cognitive Signals extend Roko's signal system with typed interrupt semantics. Where
standard signals carry data (token counts, gate verdicts, cost metrics), cognitive
signals carry intent -- they tell the pipeline what to DO, not just what IS.

> "Only variety can absorb variety."
> -- Ashby (1956), *An Introduction to Cybernetics*

The current `ConductorDecision` has three variants (Continue, Restart, Fail).
Cognitive signals expand the vocabulary to eight, directly addressing Ashby's Law:
the regulator's variety must match the system's variety.

```rust
pub enum CognitiveSignal {
    Pause,
    Resume,
    Reprioritize(TaskId),
    InjectContext(Signal),
    Escalate,
    Cooldown,
    Explore,
    Shutdown,
}
```

---

## 2. Signal Semantics

### 2.1 Pause

**Intent**: Temporarily halt execution of the current task or plan.

**When emitted**: Spec drift detected (need to verify spec is current), cost
approaching budget (need operator approval), infrastructure degraded (wait for
recovery).

**Orchestrator response**: Suspend the affected agent process. Preserve state. Do
not kill -- the work may be resumable.

**Difference from Restart**: Restart kills the agent and starts fresh. Pause
preserves state and conversation history. Pause is appropriate when the agent's
work is valid but the environment needs to change.

### 2.2 Resume

**Intent**: Continue execution after a Pause.

**When emitted**: The condition that caused the Pause has been resolved.

**Orchestrator response**: Resume the suspended process or restart dispatching.

### 2.3 Reprioritize(TaskId)

**Intent**: Change the priority of a specific task in the scheduling queue.

**When emitted**: A dependency just completed (task unblocked, move up), a file set
conflicts with a higher-priority in-flight task (deprioritize), or a task has been
waiting too long (elevate to prevent starvation).

**Orchestrator response**: Adjust queue position. Does not affect in-flight tasks.

### 2.4 InjectContext(Signal)

**Intent**: Add specific context to the current agent's prompt.

**When emitted**: The diagnosis engine has a specific fix suggestion, a playbook
rule was matched, or another agent's work produced relevant context.

**Orchestrator response**: Append content to the agent's next prompt via system
prompt, `context/in/`, or MCP tool response.

### 2.5 Escalate

**Intent**: Move the task to a more capable processing tier.

**When emitted**: A Haiku-tier agent has failed twice on a complex task, the
diagnosis engine identified an error category requiring deeper reasoning, or the
quality judge scored output below threshold.

**Orchestrator response**: Kill current agent. Respawn with a more capable model
(Haiku -> Sonnet -> Opus), more context, or different tools.

**Connection to cascade router**: Escalation feeds a negative reward. Over time,
the router learns to route complex tasks directly, reducing the need for
escalation.

### 2.6 Cooldown

**Intent**: Reduce pressure on the current task or plan.

**When emitted**: The Conductor detects the agent is approaching the Yerkes-Dodson
collapse zone -- rapid context growth, decreasing output quality per turn,
increasing token cost with decreasing progress.

**Orchestrator response**: Extend timeouts, reduce iteration pressure, or add a
deliberate pause. The goal is to move the agent back toward the productive zone of
the inverted-U curve.

### 2.7 Explore

**Intent**: Grant the agent more freedom to explore alternative approaches.

**When emitted**: Two different approaches have both failed at the gate, the
diagnosis engine suggests an architectural change, or historical episodes show
this task type benefits from exploration.

**Orchestrator response**: Expand tool access, increase iteration limit, or provide
broader context.

**Tension with Cooldown**: Explore and Cooldown pull in different directions.
Explore grants more freedom (potentially more pressure). Cooldown restricts freedom.
The choice depends on the failure mode: repeated identical errors -> Cooldown;
diverse but unsuccessful attempts -> Explore.

### 2.8 Shutdown

**Intent**: Gracefully terminate execution.

**When emitted**: Budget exhausted, critical infrastructure failure, or operator
interrupt (Ctrl+C).

**Orchestrator response**: Execute graceful shutdown sequence:
1. Stop accepting new tasks
2. Drain in-flight tasks (30-second grace period)
3. Kill remaining agents if drain times out
4. Save checkpoint
5. Flush logs
6. Exit

---

## 3. Signal vs. Signal

| Aspect | Standard Signal | Cognitive Signal |
|--------|----------------|-----------------|
| **Purpose** | Data transport | Intent transport |
| **Content** | Measurement (tokens, cost, time) | Command (pause, escalate, inject) |
| **Producer** | Any component | Conductor, learning system |
| **Consumer** | Any component | Orchestrator |
| **Action** | Read and react | Execute the intent |

### 3.1 Encoding as Standard Signals

Cognitive signals can be encoded as standard Signals using `Kind::Custom`:

```rust
fn cognitive_to_signal(cs: &CognitiveSignal) -> Signal {
    match cs {
        CognitiveSignal::Pause => {
            Signal::builder(Kind::Custom("conductor.cognitive.pause".into()))
                .body(Body::text("pause execution"))
                .tag("cognitive_signal", "pause")
                .build()
        }
        CognitiveSignal::Escalate => {
            Signal::builder(Kind::Custom("conductor.cognitive.escalate".into()))
                .body(Body::text("escalate to higher tier"))
                .tag("cognitive_signal", "escalate")
                .build()
        }
        // ...
    }
}
```

This preserves backward compatibility -- components that do not understand cognitive
signals can safely ignore them.

---

## 4. Implementation Status

The Conductor currently expresses decisions through `ConductorDecision`
(Continue/Restart/Fail), which covers a subset of cognitive signal semantics:

| ConductorDecision | Equivalent Cognitive Signal |
|-------------------|---------------------------|
| Continue | (no signal -- healthy) |
| Restart | Escalate or InjectContext + Resume |
| Fail | Shutdown (for the specific plan) |

The missing signals (Pause, Resume, Reprioritize, InjectContext, Cooldown, Explore)
represent planned extensions that would give the Conductor more nuanced control.

---

## 5. Ashby's Law Application

**Without cognitive signals**: 3 responses for N distinct failure modes. Many
failure modes receive the same generic treatment.

**With cognitive signals**: 8+ responses. Each failure mode can receive a tailored
intervention.

The expansion from 3 to 8 directly increases regulatory variety. More distinct
responses means better matching between detected anomalies and corrective actions.

---

## 6. References

- Ashby, W.R. (1956). *An Introduction to Cybernetics*. Chapman & Hall. -- Law of
  Requisite Variety: "Only variety can absorb variety."
- Yerkes, R.M. & Dodson, J.D. (1908). "The relation of strength of stimulus to rapidity of habit-formation."
- Beer, S. (1972). *Brain of the Firm*.

---

## 7. File Reference

| File | What |
|------|------|
| `crates/roko-core/src/agent.rs` | ConductorDecision (current 3-state decision) |
| `crates/roko-conductor/src/conductor.rs` | evaluate() (where decisions are made) |
| `crates/roko-conductor/src/interventions.rs` | Intervention policy (decision resolution) |
