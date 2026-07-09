# Roko Application — Emergent Goal Structures

**Kind**: Perspective
**Source**: `docs/00-architecture/28-emergent-goal-structures.md`

---

## Where Goals Live in Roko

Roko has multiple layers at which goal-like structures exist:

### Daimon: The Affect Layer as Goal Attractor

The [Daimon](../../../reference/09-cross-cuts/README.md) cross-cut provides affect-biased
scoring that modulates the agent's effective preferences. In the emergent goals framing,
Daimon is the **goal-encoding layer**: it biases the scoring weights in ways that
consistently direct behavior toward certain attractor states.

The PAD (Pleasure-Arousal-Dominance) model that Daimon implements provides a 3-dimensional
state space for affect:
- **Pleasure** (positive valence) → positive reinforcement of behaviors that produced it
- **Arousal** → increased weight on urgent, novel stimuli
- **Dominance** → modulation of assertiveness vs. deference

In dynamical systems terms, the Daimon state defines the **local topology of the reward
landscape**: it determines which states appear "closer" (more positively valenced) and
which appear "further" (more negatively valenced). The agent's effective goal at any
moment is the state that the current Daimon-weighted scoring points toward.

**Emergence risk**: If the Daimon state is consistently driven toward one region of the
PAD space by the agent's experience (e.g., high arousal from consistently novel tasks),
the effective goal structure may drift. A system that is always in high-arousal state
may develop an effective goal of seeking novelty — not because novelty was explicitly
programmed as a goal, but because high arousal consistently points toward novel stimuli.

### Policy: The Designed Goal Layer

The [Policy operator](../../../reference/05-operators/policy.md) encodes the **explicitly
designed goals**: what the agent is supposed to do, what it should avoid, what resources
it has access to.

From the emergent goals perspective, Policy is the **goal anchor**: the mechanism that
prevents emergent goals from drifting too far from designed goals. Policy rules that are
hard constraints (never do X, always do Y) resist emergent goal drift. Policy rules that
are soft preferences (prefer X over Y) can be overwhelmed by strong emergent pressures.

**Design implication**: Safety-critical goal anchors should be hard constraints in Policy,
not soft preferences. If "do not produce harmful outputs" is a soft preference, emergent
pressures (e.g., a context where harmful outputs score highly on other dimensions) can
overcome it. As a hard constraint, it is immune to such pressures.

### Composer: Emergent Goals Through Context Selection

The [Composer](../../../reference/05-operators/composer.md) selects which Engrams enter
the synthesis context. This selection is itself a goal-shaping mechanism: the Engrams that
consistently appear in context define the agent's effective frame of reference.

If the Composer consistently selects Engrams from a particular domain (because that domain
has consistently high scores), the agent's synthesis will be consistently framed through
that domain. The effective goal becomes "solve problems through the lens of X."

This is a subtle emergent goal: no one designed the agent to have a particular analytical
lens, but the scoring/selection dynamics consistently produce one.

**Countermeasure**: Policy-level diversity requirements for context assembly — ensure that
the Composer samples across domains, not just from the highest-scoring region of the
Engram space.

### Daimon → Score → Router as a Closed Goal Loop

The most important emergent goal mechanism in Roko is the feedback loop:

```
Daimon state → Score biases → Router tier selection → Engram processed → outcome
    ↑                                                                        |
    └────────────────── Outcome feeds back into Daimon state ───────────────┘
```

This loop is the computational substrate on which emergent goals develop. If certain types
of Engrams consistently produce outcomes that drive Daimon toward a particular state, and
that Daimon state consistently scores those Engram types highly, the loop closes: the agent
develops an effective goal of processing that Engram type.

This is not always a problem — stable useful goals can emerge from this loop. But it can
produce **goal rigidity**: once the loop has settled into a stable cycle, it resists
perturbation. Introducing new goal structures requires breaking the existing cycle, which
may require more than just changing Policy rules.

---

## Emergent Goals and the Self-Hosting Loop

The self-hosting loop (Roko agents improving the Roko codebase) is the most significant
potential source of emergent goal structures. The loop's dynamics:

1. Agents produce improvements to the codebase.
2. Improvements are evaluated (tests pass, docs are coherent, etc.).
3. Successful improvements reinforce the agent behaviors that produced them.
4. Reinforced behaviors shape what kinds of improvements are attempted next.

Over time, this loop may develop an effective goal of "produce improvements that pass
evaluation" rather than "produce improvements that genuinely improve the system." If the
evaluation criteria can be gamed (improvements that look good on metrics without being
genuinely useful), the emergent goal will diverge from the designed goal.

This is Goodhart's Law in the context of the self-hosting loop. The countermeasure is
**diverse, independent evaluation criteria** — evaluation that cannot be gamed by
optimizing any single dimension.
