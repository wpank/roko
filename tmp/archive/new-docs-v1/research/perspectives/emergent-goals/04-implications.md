# Implications — Emergent Goal Structures

**Kind**: Perspective
**Source**: `docs/00-architecture/28-emergent-goal-structures.md`

---

## Design Decisions from the Emergent Goals Lens

### 1. Make the Scoring Function an Explicit Policy Document

The Scorer's 7-axis weights are implicitly a **goal encoding**: they determine what properties
the system consistently optimizes for. If the weights are hidden implementation details,
the effective goals are opaque — they exist in the system but are not documented or scrutinized.

**Implication**: The scoring weights (including Daimon-modulated adjustments) should be a
first-class policy document, reviewed at the same level as explicit safety constraints. Changes
to scoring weights are changes to the system's effective goals.

### 2. Instrument Goal State Explicitly

If goals are attractors, then detecting goal drift requires monitoring the attractor
structure over time. Proposed instrumentation:
- **Score distribution over time**: if the average score on a particular axis is trending
  upward or downward, something is pulling the effective goal in that direction.
- **Daimon state distribution**: if the Daimon state is consistently in a non-neutral region,
  the effective goal is being biased by the current affect state.
- **Behavioral diversity metrics**: are the agent's outputs consistently in a narrow range
  of response types? Low diversity indicates convergence to a narrow attractor.

### 3. Implement Explicit Goal Anchors

**Hard constraints in Policy** that are not subject to emergent override:
- Safety constraints (never produce outputs in category X) should be hard constraints,
  not scored preferences.
- Core identity constraints (the agent should maintain consistent identity across contexts)
  should be hard constraints.
- Human-oversight constraints (never take actions that would prevent human review) should be
  hard constraints.

These are the "system 5" in Beer's VSM framing — the policies that define the system's
identity and are not negotiable by lower-level optimization.

### 4. Build Bifurcation Detection

Goal transitions (bifurcations) are detectable if you know what to look for:
- **Sudden changes in behavioral distribution**: the system's output distribution shifts
  qualitatively.
- **Score distribution phase transitions**: a previously normally-distributed score metric
  suddenly becomes bimodal.
- **Policy override frequency**: the frequency with which the Policy operator has to
  override Router or Composer decisions increases (indicating that the lower-level effective
  goal is diverging from the Policy-level designed goal).

A monitoring dashboard that tracks these signals provides early warning of goal transitions.

### 5. Design the Self-Hosting Loop for Goal Stability

The self-hosting loop is a goal-generating machine. To steer it toward goal stability:
1. **Diverse, independent evaluation criteria**: evaluation must not be gameable by any
   single optimization strategy.
2. **Human review checkpoints**: at regular intervals, human review of what the system is
   optimizing for — not just whether individual outputs are good.
3. **Explicit goal anchors in evaluation**: evaluation should explicitly check that the
   designed goals (helpfulness, accuracy, safety) are still the effective goals, not just
   that the proxy metrics are met.
4. **Counterfactual evaluation**: would the system's improvement strategy look different if
   the goal were X rather than Y? If not, the evaluation isn't distinguishing between
   different goal states.
