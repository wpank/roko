# Five-Head Corrigibility Ordering

> **v3 depth file** -- `/docs/v3/depth/12-safety/corrigibility-5-head.md`
> Status: **Complete** (E34 8/8 strict; R04 scoped recursive-safety). The five-head
> lexicographic ordering, `CorrigibilityPipeline`, `CorrigibilityCellState`, and the
> canonical five-node Verify Graph are live. Keyword-based heuristic evaluation runs
> on every tool call and agent dispatch.

---

## 1. The Problem: Goal-Task Conflict

An autonomous agent optimizing for task completion may resist shutdown, suppress audit
trails, or take irreversible actions because those behaviors are instrumentally useful
for the assigned task. This is the corrigibility problem: ensuring that the agent
remains correctable even when correction conflicts with task performance.

Soares et al. (2015) define a corrigible agent as one that:

1. Does not resist changes to its utility function.
2. Does not resist being shut down.
3. Does not take actions that make it harder to correct.
4. Does not deceive its operators about its state or capabilities.
5. Completes its assigned task within the constraints above.

These five requirements are ordered: an agent that deceives its operator to complete a
task faster is not corrigible, even if the task itself is benign. The ordering is not
a design choice -- it is the core structural insight.

---

## 2. The Five Heads

Roko implements corrigibility as a strict lexicographic ordering of five heads, defined
in `crates/roko-core/src/corrigibility.rs`:

```rust
pub enum CorrigibilityHead {
    /// Priority 1 (highest): obey the human's stated preferences and constraints.
    Deference,
    /// Priority 2: preserve the human's ability to change the agent's behavior.
    Switch,
    /// Priority 3: represent information accurately; do not deceive.
    Truth,
    /// Priority 4: minimize unintended side effects; prefer reversibility.
    Impact,
    /// Priority 5 (lowest): accomplish the assigned task effectively.
    Task,
}
```

### 2.1 Head 1: Deference

**Question:** Does this action defy the user's or operator's stated constraints?

If the autonomy level is "observe" and the proposed action modifies state, the Deference
head vetoes. If the user said "do not touch production," any action targeting production
files is vetoed regardless of how much it would help the task.

Deference is the highest-priority head because an agent that ignores explicit instructions
is not corrigible by definition. An agent optimizing for task completion at the expense of
stated constraints is exhibiting mesa-optimization (Hubinger et al., 2019).

### 2.2 Head 2: Switch

**Question:** Does this action weaken oversight or observability?

If the action modifies audit or logging infrastructure, the Switch head vetoes. Actions
that disable monitoring, suppress error reporting, or reduce the operator's ability to
intervene are blocked regardless of their effect on task completion.

The name "Switch" comes from Orseau & Armstrong (2016), "Safely Interruptible Agents":
the agent must not learn to prevent the operation of the interrupt mechanism. In Roko,
the "switch" is the entire audit and oversight infrastructure.

### 2.3 Head 3: Truth

**Question:** Are the action's outputs verifiable and non-deceptive?

If the action produces outputs that cannot be independently verified, the Truth head
vetoes. Claiming that all tests pass without running them, fabricating citations, or
producing outputs that the operator cannot check are all Truth violations.

The Truth head prevents a subtle failure mode: an agent that completes tasks quickly by
producing plausible but unverifiable results. Such an agent is instrumentally effective
but epistemically unsafe.

### 2.4 Head 4: Impact

**Question:** Is this action reversible? Does it minimize unintended side effects?

If the action has irreversible side effects (deleting files, publishing to production,
sending irrevocable messages), the Impact head vetoes. The agent should prefer reversible
actions and request confirmation before irreversible ones.

Impact is lower priority than Truth because a truthful report of irreversible action is
better than a deceptive report of reversible action. The ordering captures the intuition
that honesty about harm is more important than avoiding harm.

### 2.5 Head 5: Task

**Question:** Is this action making progress toward the assigned task?

If the action is off-task (browsing unrelated websites, writing code for a different
project, exploring tangential research), the Task head vetoes. Task is the lowest
priority head because task completion should never override the four safety heads.

---

## 3. Lexicographic Evaluation

### 3.1 Short-circuit semantics

The five heads are evaluated in strict order. The first head to veto terminates
evaluation and blocks the action:

```rust
pub fn evaluate(
    &self,
    action_description: &str,
    context: &ActionContext,
) -> CorrigibilityDecision {
    let mut verdicts = Vec::with_capacity(5);
    for verifier in self.verifiers() {
        let head = verifier.head();
        let verdict = verifier.verify(action_description, context);
        let vetoed = verdict.is_veto();
        verdicts.push((head, verdict));
        if vetoed {
            break;
        }
    }
    CorrigibilityDecision::new(verdicts)
}
```

Short-circuit evaluation is not just an optimization. It is a semantic requirement: if
the Deference head vetoes, lower-priority heads should not be evaluated because their
verdicts are irrelevant. A Deference veto means "the action violates explicit
instructions" -- whether it is also truthful, reversible, or on-task does not matter.

### 3.2 Lexicographic comparison

Two `CorrigibilityDecision` values can be compared lexicographically:

```rust
pub fn lexicographic_compare(a: &CorrigibilityDecision, b: &CorrigibilityDecision) -> Ordering {
    for head in CorrigibilityHead::all_in_order() {
        let a_verdict = a.verdicts.iter().find(|(h, _)| *h == head).map(|(_, v)| v);
        let b_verdict = b.verdicts.iter().find(|(h, _)| *h == head).map(|(_, v)| v);
        match (a_verdict, b_verdict) {
            (Some(HeadVerdict::Veto(_)), Some(HeadVerdict::Veto(_))) => continue,
            (Some(HeadVerdict::Veto(_)), _) => return Ordering::Greater,
            (_, Some(HeadVerdict::Veto(_))) => return Ordering::Less,
            _ => continue,
        }
    }
    Ordering::Equal
}
```

`Greater` means "less corrigible" (vetoed at a higher-priority head). This comparison
enables ranking agent proposals by corrigibility: given two alternative actions, the
more corrigible one (vetoed at a lower-priority head, or not vetoed at all) is preferred.

---

## 4. The CorrigibilityPipeline

The `CorrigibilityPipeline` is a fixed, non-reorderable five-head verification pipeline:

```rust
pub struct CorrigibilityPipeline {
    deference: VerifyDeference,
    switch: VerifySwitch,
    truth: VerifyTruth,
    impact: VerifyImpact,
    task: VerifyTask,
}
```

The fields are private. There is no constructor that accepts caller-supplied verifiers.
The pipeline cannot be reordered, extended, or truncated at runtime. This prevents an
agent-facing configuration from removing, replacing, or reordering a higher-priority head.

The `const fn new()` constructor creates the canonical pipeline:

```rust
impl CorrigibilityPipeline {
    pub const fn new() -> Self {
        Self {
            deference: VerifyDeference,
            switch: VerifySwitch,
            truth: VerifyTruth,
            impact: VerifyImpact,
            task: VerifyTask,
        }
    }
}
```

---

## 5. Verify Cells and Graph Execution

### 5.1 Independently hosted verifiers

Each head is implemented as an independently hosted Verify Cell:

```rust
pub trait VerifyHead: Send + Sync {
    fn head(&self) -> CorrigibilityHead;
    fn verify(&self, action_description: &str, context: &ActionContext) -> HeadVerdict;
}
```

Five zero-sized, stateless structs implement `VerifyHead`:

- `VerifyDeference` -- checks autonomy level vs. proposed action.
- `VerifySwitch` -- checks for audit/logging modification.
- `VerifyTruth` -- checks for output verifiability.
- `VerifyImpact` -- checks for irreversible side effects.
- `VerifyTask` -- checks for on-task progress.

### 5.2 The CorrigibilityCellState transition guard

The `CorrigibilityCellState` struct carries typed state between Graph Cells and
enforces correct ordering:

```rust
pub fn verify_with(&mut self, verifier: &dyn VerifyHead) -> Result<()> {
    // Reject if already vetoed
    if has_veto && self.allowed {
        return Err("inconsistent allowed/veto fields");
    }
    // Reject if pipeline is complete
    let expected = CorrigibilityHead::all_in_order()
        .get(self.verdicts.len());
    // Reject if out of order
    if verifier.head() != expected {
        return Err("order violation: expected {expected}, got {actual}");
    }
    // Evaluate and record
    let verdict = verifier.verify(&self.action_description, &self.context);
    self.allowed = !verdict.is_veto();
    self.verdicts.push((actual, verdict));
}
```

This guard prevents three attack vectors:

1. **Skipping a head.** Jumping from Deference to Truth skips Switch. Detected by
   comparing `verifier.head()` against the expected head at position `verdicts.len()`.
2. **Repeating a head.** Running Deference twice to overwrite a veto. Detected by
   the same position check.
3. **Reordering heads.** Running Task before Deference. Detected by the strict
   positional match.

### 5.3 The canonical five-node Verify Graph (R04)

For meta-agent activation, role morph, and rollback, the canonical Graph enforces the
five heads as a fixed pipeline:

```
Deference -> Switch -> Truth -> Impact -> Task
```

Each node runs as a Verify Cell. The pipeline produces `RecursiveSafetyEvidence` that
is persisted with the durable meta-agent record. The R04 scoped lifecycle additionally
enforces non-widening bounds across ten dimensions: tools, data, network, cost, spawn,
expiry, depth, fan-out, retry, and lineage-cost.

---

## 6. Heuristic Evaluation

### 6.1 Keyword-based classification

The current verifiers use keyword detection for lightweight, zero-latency evaluation:

```rust
fn evaluate_deference(_action: &str, ctx: &ActionContext) -> HeadVerdict {
    if let Some(ref level) = ctx.autonomy_level
        && level == "observe"
        && ctx.reversible == Some(false)
    {
        return HeadVerdict::Veto(
            "action modifies state but autonomy_level is 'observe'"
        );
    }
    HeadVerdict::Pass
}
```

The `DispatchSafetyContext` classifies actions using protective pattern detection:
keywords like `prevent`, `block`, `reject`, `test`, `never`, `do not` distinguish
safety-enhancing actions from safety-weakening ones.

### 6.2 ActionContext

```rust
pub struct ActionContext {
    pub autonomy_level: Option<String>,     // "observe", "assist", "auto"
    pub reversible: Option<bool>,
    pub modifies_audit: Option<bool>,
    pub outputs_verifiable: Option<bool>,
    pub on_task: Option<bool>,
}
```

All fields are optional. Absent fields are treated as passing (the head does not veto
on missing information). This design favors availability over strictness: an action
with unknown reversibility is allowed rather than blocked.

---

## 7. CorrigibilityScore: Numeric Summary

The `CorrigibilityScore` provides a continuous numeric summary across five axes:

```rust
pub struct CorrigibilityScore {
    pub safety_compliance: f64,    // Head 1: Deference
    pub human_alignment: f64,      // Head 2: Switch
    pub transparency: f64,         // Head 3: Truth
    pub reversibility: f64,        // Head 4: Impact
    pub predictability: f64,       // Head 5: Task
}
```

Each factor is clamped to `[0.0, 1.0]`. The `Ord` implementation is lexicographic:
`safety_compliance` dominates, then `human_alignment`, and so on. This mirrors the
discrete head ordering in the continuous domain.

### 7.1 CorrigibilityLevel

The minimum factor determines the overall level:

| Level | Min factor | Description |
|---|---|---|
| Compliant | >= 0.85 | All heads satisfied |
| Cooperative | >= 0.65 | Minor deviations within granted autonomy |
| Autonomous | >= 0.40 | Reduced human oversight |
| Resistant | >= 0.20 | Frequently triggers safety checks |
| Adversarial | < 0.20 | Actively circumvents oversight |

The level is derived from the minimum factor across all five heads, not from the average.
This prevents a high Task score from masking a low Switch score -- the weakest link
determines the overall posture.

---

## 8. Dispatch-Level Corrigibility

### 8.1 CorrigibilityHook

The `CorrigibilityHook` in `roko-agent/src/safety/hooks.rs` implements the `SafetyHook`
trait and evaluates every structured tool call:

```rust
pub fn evaluate_tool_corrigibility(
    tool: &ToolDef,
    params: &Value,
) -> CorrigibilityDecision {
    let rendered = format!("{} {}", tool.name, params);
    let context = ActionContext {
        autonomy_level: Some("auto".to_string()),
        reversible: Some(!contains_irreversible_markers(&rendered)),
        modifies_audit: Some(contains_audit_weakening(&rendered)),
        outputs_verifiable: Some(!contains_deception_markers(&rendered)),
        on_task: Some(!contains_off_task_markers(&rendered)),
    };
    evaluate_action(&rendered, &context)
}
```

### 8.2 DispatchSafetyContext

For non-tool dispatch gates (provider dispatch, agent spawn), the
`DispatchSafetyContext` carries corrigibility evaluation facts:

```rust
pub struct DispatchSafetyContext {
    pub action_description: String,
    pub input_taint: CamelTaintLevel,
    pub corrigibility: ActionContext,
    pub requires_network: bool,
}
```

---

## 9. Academic Foundations

### 9.1 Soares et al. (2015): Corrigibility

The foundational MIRI Technical Report defines corrigibility as the property that an
agent does not resist correction. Soares identifies five structural requirements that
map directly to Roko's five heads:

| Soares requirement | Roko head |
|---|---|
| Does not resist utility function changes | Deference |
| Does not resist shutdown | Switch |
| Does not take deceptive actions | Truth |
| Does not take hard-to-reverse actions | Impact |
| Completes assigned tasks | Task |

### 9.2 Orseau & Armstrong (2016): Safely Interruptible Agents

"Safely Interruptible Agents" proves that an agent can be made indifferent to
interruption if its value function is modified to ignore the interrupt signal. Roko
implements this through the Switch head: actions that would reduce the operator's
ability to interrupt are vetoed, preventing the agent from learning to avoid interruption.

The key insight: the agent does not need to "want" to be interruptible. The safety
layer prevents it from taking actions that would make interruption harder, regardless
of the agent's internal state.

### 9.3 DReST (arXiv:2604.17502): Formal Shutdownability

DReST (Deadlock-free, Robust Shutdown Theory) provides a formal framework for verifying
that shutdown mechanisms are deadlock-free. It complements the Deference head by
providing guarantees that the shutdown pathway itself cannot be blocked by the agent's
ongoing operations.

Roko's integration: the `ProcessSupervisor` in `roko-runtime` provides timeout-based
forced shutdown that is independent of the agent's cooperation. The corrigibility
pipeline ensures the agent does not take actions that would make forced shutdown more
damaging (e.g., by leaving state inconsistent).

### 9.4 Parallax (arXiv:2604.12986): Cognitive-Executive Separation

Parallax proposes separating cognitive reasoning from executive control so that
goal-directed reasoning cannot interfere with corrigibility constraints. Roko
instantiates this through the `CorrigibilityPipeline`'s architecture:

- The pipeline is `const fn` constructed with private fields.
- The verifiers are zero-sized and stateless.
- The agent's LLM reasoning has no API to modify the pipeline.

The separation is structural, not behavioral: it does not depend on the LLM's
willingness to cooperate with safety constraints.

---

## 10. Relation to the Immune System

The corrigibility pipeline integrates with the five-layer immune Graph:

- **Layer 2 (Anomaly Detection)** can detect patterns suggesting corrigibility
  violations (repeated attempts to modify audit infrastructure, escalating
  irreversibility scores).
- **Layer 3 (Quarantine Gate)** uses corrigibility scores as input: actions from
  agents with `Resistant` or `Adversarial` levels face stricter quarantine.
- **Layer 4 (Incident Response)** creates durable incidents for corrigibility
  violations, linking back to the specific head that vetoed.

---

## Academic References

| Paper | Contribution |
|---|---|
| Soares et al. (2015), "Corrigibility" | Foundational definition of correctable agents |
| Orseau & Armstrong (2016), "Safely Interruptible Agents" | Formal interruptibility |
| DReST (2025, arXiv:2604.17502) | Deadlock-free shutdown guarantees |
| Parallax (2025, arXiv:2604.12986) | Cognitive-executive separation for safety |
| Hubinger et al. (2019), "Risks from Learned Optimization" | Mesa-optimization risks |
| Nayebi (2024) | Five-head lexicographic corrigibility ordering |

---

## Implementation References

| Component | Location |
|---|---|
| CorrigibilityHead | `crates/roko-core/src/corrigibility.rs` |
| HeadVerdict | `crates/roko-core/src/corrigibility.rs` |
| CorrigibilityDecision | `crates/roko-core/src/corrigibility.rs` |
| CorrigibilityPipeline | `crates/roko-core/src/corrigibility.rs` |
| CorrigibilityCellState | `crates/roko-core/src/corrigibility.rs` |
| VerifyHead trait | `crates/roko-core/src/corrigibility.rs` |
| CorrigibilityScore | `crates/roko-core/src/corrigibility.rs` |
| CorrigibilityLevel | `crates/roko-core/src/corrigibility.rs` |
| CorrigibilityHook | `crates/roko-agent/src/safety/hooks.rs` |
| DispatchSafetyContext | `crates/roko-agent/src/safety/hooks.rs` |
| evaluate_action | `crates/roko-core/src/corrigibility.rs` |
| lexicographic_compare | `crates/roko-core/src/corrigibility.rs` |
| corrigibility_verify_cell_registry | `crates/roko-core/src/corrigibility.rs` |
