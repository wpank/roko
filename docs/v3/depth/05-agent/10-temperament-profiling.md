# 05-10 -- Temperament Profiling

> **Implementation status (2026-09):** The temperament enum and config
> field exist in `AgentConfig`. Temperament influences CascadeRouter
> confidence thresholds and exploration parameters. Full propagation
> to gate strictness, tool selection, and review depth is partially
> wired.

---

## The Temperament Concept

Roko's temperament system provides a **single configuration dial** that
adjusts multiple agent behaviors simultaneously. Rather than tuning 15
individual parameters (temperature, max_tokens, tool_selection_bias,
gate_threshold, review_passes, etc.), the operator selects one of four
temperaments, and all downstream behaviors adjust accordingly.

| Temperament | Use case | Key behaviors |
|---|---|---|
| **Conservative** | Production, safety-critical | Low temperature, strict gates, full review, minimal tool use |
| **Balanced** | Default development | Medium temperature, standard gates, standard review |
| **Aggressive** | Rapid prototyping | Higher temperature, relaxed gates, faster review, more tools |
| **Exploratory** | Research, experimentation | High temperature, permissive gates, broad tool access |

This design implements Meta-Harness Principle 5 (Graduate Autonomy Based
on Confidence) at the configuration level: Conservative = low autonomy
with high validation; Exploratory = high autonomy with low validation.

---

## What Temperament Controls

### 1. Model Parameters

| Parameter | Conservative | Balanced | Aggressive | Exploratory |
|---|---|---|---|---|
| `temperature` | 0.1 | 0.3 | 0.7 | 1.0 |
| `top_p` | 0.9 | 0.95 | 0.98 | 1.0 |
| `max_tokens` | profile default | profile default | profile x 1.5 | profile x 2.0 |

Conservative keeps the model focused on the most likely tokens, reducing
creativity but increasing reliability. Exploratory allows the full
token distribution, encouraging novel approaches.

### 2. Tool Selection

| Behavior | Conservative | Balanced | Aggressive | Exploratory |
|---|---|---|---|---|
| Tool count | Minimal | Standard | Expanded | All available |
| Dangerous tools | Blocked | Blocked | Allowed with confirm | Allowed |
| Network access | Denied | Per-request | Allowed | Allowed |
| File writes | Confirmed | Allowed | Allowed | Allowed |

Conservative restricts the agent to read-only tools by default,
requiring explicit approval for any write or exec operation.
Exploratory gives the agent access to all registered tools including
network fetch and bash execution.

### 3. Gate Strictness

| Gate behavior | Conservative | Balanced | Aggressive | Exploratory |
|---|---|---|---|---|
| Compile gate | Required | Required | Required | Warning |
| Test gate | Required | Required | Warning | Skipped |
| Clippy gate | Required | Warning | Skipped | Skipped |
| Diff size gate | Strict (< 500 lines) | Standard (< 2000) | Relaxed (< 5000) | Disabled |
| Review gate | Required | Optional | Skipped | Skipped |

Conservative requires all gates to pass before accepting agent output.
Aggressive relaxes test and lint gates to speed iteration. Exploratory
disables most gates entirely.

### 4. Review Depth

| Review behavior | Conservative | Balanced | Aggressive | Exploratory |
|---|---|---|---|---|
| Review passes | 2 (double review) | 1 | 0 (self-review) | 0 |
| Review model | Premium tier | Standard tier | Same as implementer | None |
| Feedback loop | Required | Optional | Disabled | Disabled |

### 5. Model Routing

| Routing behavior | Conservative | Balanced | Aggressive | Exploratory |
|---|---|---|---|---|
| Starting tier | Standard | Standard | Fast | Fast |
| Escalation threshold | High (0.9 confidence) | Medium (0.7) | Low (0.5) | Low (0.3) |
| Budget multiplier | 0.8x | 1.0x | 1.5x | 2.0x |
| Fallback on error | Always | Usually | Sometimes | Rarely |

---

## Configuration

Temperament is set in `roko.toml`:

```toml
[agent]
temperament = "balanced"  # conservative | balanced | aggressive | exploratory
```

Per-role overrides are supported:

```toml
[agent.roles.implementer]
temperament = "balanced"

[agent.roles.researcher]
temperament = "exploratory"

[agent.roles.auditor]
temperament = "conservative"
```

The `Temperament` enum is defined in `roko-core`:

```rust
pub enum Temperament {
    Conservative,
    Balanced,
    Aggressive,
    Exploratory,
}
```

---

## Temperament in the CascadeRouter

The CascadeRouter uses temperament to set its initial parameters:

- **Confidence threshold** -- how confident the fast model must be
  before the task is accepted without escalation. Conservative: 0.9,
  Balanced: 0.7.
- **UCB exploration parameter** -- controls how much the LinUCB bandit
  explores vs. exploits. Exploratory temperament sets a high exploration
  parameter, causing the bandit to try more model combinations.
- **Cost weight in Pareto frontier** -- how much cost factors into model
  selection. Aggressive temperament uses a lower cost weight (willing to
  spend more for speed).

This means temperament affects not just the current task, but the
learning trajectory: an Exploratory temperament causes the router to try
more model combinations, building a richer reward signal for future
decisions.

---

## Temperament and Active Inference

The temperament system has a theoretical connection to the Free Energy
Principle (Friston, 2006). In active inference terms:

- **Conservative** = high precision on expected outcomes. The agent
  strongly expects correct code and requires strong evidence (gate
  passes) before accepting. This corresponds to a low free-energy
  tolerance.
- **Exploratory** = low precision on expected outcomes. The agent
  accepts more variance, allowing exploration of the state space. This
  corresponds to a high free-energy tolerance (more surprise is
  acceptable).

The precision parameter in active inference maps directly to the
confidence threshold in the CascadeRouter: higher precision means the
agent demands more confidence before committing to a model tier.

---

## Temperament Interaction with Budget

Temperament interacts with the per-role budget system:

| Temperament | Budget effect |
|---|---|
| Conservative | 0.8x multiplier (lower ceiling) |
| Balanced | 1.0x (no adjustment) |
| Aggressive | 1.5x (higher ceiling) |
| Exploratory | 2.0x (highest ceiling) |

This creates a natural cost-safety tradeoff: Conservative is cheapest
and safest, Exploratory is most expensive but discovers optimal model
routing faster.

---

## Temperament in the Harness Framework

The temperament system implements Meta-Harness Principle 5 (Graduate
Autonomy Based on Confidence) at the configuration level. The operator
selects the trust level appropriate for the context:

- Production deployments use **Conservative**.
- Development sprints use **Balanced** or **Aggressive**.
- Research spikes use **Exploratory**.

This is a deliberate design choice: rather than having the system
automatically escalate autonomy (which could be unsafe), the operator
explicitly sets the autonomy envelope. Automatic escalation *within*
a temperament level is handled by the CascadeRouter's model tier
selection, but the overall autonomy envelope is set by the human.

---

## Wiring Path

The implementation path for full temperament propagation:

1. **Read temperament from config** -- `AgentConfig::temperament`
   is already parsed into the enum.
2. **Pass to CascadeRouter** -- set initial confidence threshold,
   exploration parameter, and cost weight from the temperament table.
   This is wired.
3. **Pass to gate pipeline** -- set `required` / `warning` / `skipped`
   per gate based on the temperament table. Partially wired through
   adaptive gate thresholds.
4. **Pass to ToolDispatcher** -- adjust tool allowlists.
   Conservative restricts to read-only by default; Exploratory allows
   all tools. Not yet wired.
5. **Pass to SystemPromptBuilder** -- include temperament-appropriate
   behavioral instructions in the role prompt layer. Not yet wired.

Each step is independent and can be wired incrementally.

---

## Implementation Sources

| File | Purpose |
|------|---------|
| `crates/roko-core/src/config/schema.rs` | AgentConfig temperament field |
| `crates/roko-core/src/lib.rs` | Temperament enum definition |
| `crates/roko-learn/src/` | CascadeRouter, adaptive gate thresholds |
| `crates/roko-compose/src/system_prompt_builder.rs` | SystemPromptBuilder (future temperament layer) |

---

## Citations

1. Lee, Y. et al. (2026). "Meta-Harness: End-to-End Optimization of Model
   Harnesses." arXiv:2603.28052. -- Principle 5: Graduate Autonomy.
2. Friston, K. (2006). "A free energy principle for the brain." Journal
   of Physiology - Paris. -- Precision parameter and active inference.
3. `crates/roko-core/src/config/schema.rs` -- AgentConfig temperament
   field.
4. `crates/roko-learn/` -- CascadeRouter, adaptive gate thresholds.
