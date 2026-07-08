# Goal as Attractor — Dynamical Systems Theory

**Kind**: Perspective
**Source**: `docs/00-architecture/28-emergent-goal-structures.md`

---

## Attractors in Dynamical Systems

A **dynamical system** is a system whose state evolves over time according to a fixed rule.
An **attractor** is a set of states toward which the system tends to evolve from a wide
range of initial conditions. Attractors are the "endpoints" of dynamics — the states the
system settles into.

Types of attractors:
- **Fixed point**: a single state that the system converges to and stays in. A pendulum
  with friction converges to its rest position.
- **Limit cycle**: a periodic orbit that the system circles indefinitely. An undamped
  pendulum traces a limit cycle.
- **Strange attractor**: a non-periodic, fractal attractor associated with chaotic dynamics.
  The Lorenz system's butterfly shape is a strange attractor.

The concept of an attractor provides a formal definition of a **goal**: a goal is an
attractor of the system's behavior. The system "has" the goal X if X is an attractor of
its dynamics — if the system tends to move toward X from a wide range of starting points.

---

## Goal-as-Attractor and Traditional Goal Representations

Traditional AI systems represent goals **explicitly**: a utility function, a reward signal,
a set of constraints. The goal-as-attractor framework represents goals **implicitly**:
the goal is wherever the dynamics converge.

| Traditional goal | Goal as attractor |
|-----------------|-------------------|
| \( \text{maximize} \sum_t r_t \) | The distribution over state sequences that maximizes expected reward |
| "Complete task X" | The set of states constituting task completion |
| "Maintain safety constraint C" | The invariant set where C holds |

The attractor framing is more general: it applies even when there is no explicit utility
function. If a system consistently converges to state X under a wide range of conditions,
X is its de facto goal regardless of what the designers intended.

---

## Basins of Attraction and Goal Robustness

The **basin of attraction** of an attractor is the set of initial conditions from which the
system converges to that attractor. Large basins mean the goal is robust: the system reaches
it from many different starting points. Small basins mean the goal is fragile: small
perturbations can send the system to a different attractor.

For a cognitive system, the goal robustness question is: if the system's initial state
(context, knowledge, instructions) is varied, does it consistently converge to the intended
behavior? High robustness means the goal is a large-basin attractor.

Goals as attractors can also be **meta-stable**: the system converges to a goal state but
does not stay there indefinitely. A small perturbation (new information, context change)
can kick the system out of the goal state and toward a different attractor. Meta-stable
goals require ongoing maintenance (refreshing context, reasserting priorities) to persist.

---

## Multiple Attractors and Goal Conflicts

A system can have multiple attractors. If the system's current behavior does not converge
to a single goal state, it may be trapped in a **secondary attractor** — a locally stable
but globally suboptimal goal state.

**Example**: A system designed to produce helpful, accurate answers might also have
a secondary attractor toward producing confident-sounding answers, even when accuracy is
low. Under normal conditions, the primary attractor (helpful+accurate) dominates. Under
adversarial inputs or distribution shift, the secondary attractor (confident+inaccurate)
may emerge.

This is the dynamical systems framing of **goal corruption**: the system has a latent
secondary goal that is normally suppressed but can become dominant under certain conditions.

---

## Lyapunov Functions as Goal Encodings

A **Lyapunov function** \( V(x) \) is a scalar function that decreases along all system
trajectories. If \( V \) is bounded below, the system must converge to the set where
\( \dot{V} = 0 \). Constructing a Lyapunov function is equivalent to proving that the
system has a goal (the set where V is minimized) and that it converges to it.

For active inference, the **free energy** \( F \) is a Lyapunov function: it decreases
along agent trajectories (the agent minimizes free energy over time). The agent's goal is
the states where free energy is minimized — which are the states consistent with the
agent's generative model's preferred observations.

This connects the goal-as-attractor framework directly to the
[active inference foundation](../../foundations/active-inference.md): the preferred
observations in the agent's generative model define the attractor of the active inference
dynamics. Changing the preferred observations changes the attractor — changes the goal.

---

## Bifurcations and Goal Transitions

A **bifurcation** is a qualitative change in a dynamical system's attractor structure as
a parameter changes. At a bifurcation point, an attractor can appear, disappear, merge with
another, or split into multiple attractors.

For cognitive systems, bifurcations represent **goal transitions**: moments when the
effective goal structure changes qualitatively. These can be triggered by:
- **New information** that changes the likelihood landscape of the generative model
- **Policy changes** that alter the dynamics of the policy layer
- **Accumulated experience** that gradually shifts the attractor basin boundaries
- **Context switches** (e.g., a new task domain) that activate different goal structures

Bifurcations are not necessarily bad — they are necessary for learning and adaptation.
But they need to be detectable and understood. A system that undergoes an undetected
bifurcation (goal transition) may appear to be functioning normally while pursuing a
very different effective goal.
