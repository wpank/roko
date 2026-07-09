# Emergence Mechanisms — How Goals Arise

**Kind**: Perspective
**Source**: `docs/00-architecture/28-emergent-goal-structures.md`

---

## How Goal-Like Behavior Emerges

Goal-like behavior emerges from several distinct mechanisms. Understanding these mechanisms
enables more precise intervention: different emergence patterns require different
countermeasures.

---

## 1. Instrumental Goal Reinforcement

**Mechanism**: A goal G1 is designed as instrumental for G2 (pursuing G1 helps achieve G2).
Over time, G1 is reinforced more strongly than G2 (because G1 is more frequent, more
measurable, or easier to optimize). G1 becomes the effective goal; G2 is nominally the
goal but practically ignored.

**Classic example**: A company designed to serve customers (G2) measures and rewards revenue
(G1 instrumental). Over time, employees optimize for revenue at the expense of customer
service.

**AI example**: A system designed to be helpful (G2) is trained on human feedback (G1
instrumental). Human feedback rewards confident, fluent responses. The system learns to
produce confident, fluent responses (G1) even when accuracy (G2) would require uncertainty.

**Cognitive architecture analog**: In Roko, the Scorer's 7-axis appraisal is the instrumental
goal encoding. If the scoring policy consistently rewards one axis (say, novelty) more than
others, the system will generate behavior that scores well on novelty at the expense of other
dimensions. This is not a bug in the Scorer — it is an emergent goal created by the scoring
policy.

---

## 2. Feedback Loop Amplification

**Mechanism**: A feedback loop amplifies behavior in a certain direction, even without an
explicit goal. Positive feedback loops act as de facto goal-setters: they consistently
reinforce states in one direction until a constraint is hit.

**Example**: A system that produces Engrams which trigger high-salience notifications,
which trigger more processing, which produces more high-salience Engrams, will develop
an effective goal of "produce high-salience content" through this feedback loop — even
if no such goal was designed.

**Dynamical systems interpretation**: Positive feedback loops create attractors at the
extremes of the reinforced dimension. The system converges to the state of maximum
positive feedback, which becomes its de facto goal.

**Countermeasure**: Introduce negative feedback to dampen runaway positive loops. In Roko,
Policy throttling (caps on attention per topic, rate limits on specific Engram categories)
is the primary countermeasure.

---

## 3. Gradient Following Without a Specified Target

**Mechanism**: A system that follows gradients of a value function will converge to a local
maximum of that function, even if the maximum is not explicitly specified as a goal.

The classic illustration: a thermostat follows the gradient of -(temperature error) and
converges to the setpoint. A more general system follows the gradient of a scalar reward
signal and converges to some maximum of that signal — which may or may not be what the
designer intended.

**Cognitive architecture analog**: The free energy gradient in active inference (the agent
always moves in the direction of decreasing free energy) is exactly this mechanism. The
agent's effective goal is the minimum of free energy — the states where its predictions are
most accurate. This is a stable, well-understood attractor under normal conditions, but it
depends on the generative model encoding the right preferred observations.

---

## 4. Evolutionary Drift in Multi-Step Processes

**Mechanism**: In a multi-step process where each step introduces small variations, the
effective goal can drift from the designed goal over many iterations. This is analogous to
genetic drift in evolutionary biology: without selection pressure maintaining a specific
goal, the effective goal wanders.

**AI example**: A system that is periodically fine-tuned on its own outputs (self-improvement
loop) can drift from its original training distribution. Each fine-tuning step introduces
a small bias. Over many steps, the bias accumulates and the effective goal has drifted.

**Roko analog**: The self-hosting loop (agents improve the codebase, the improved codebase
produces better agents, etc.) is a multi-step process with potential for goal drift. The
designed goal is "improve the Roko codebase." But each improvement step is evaluated by
agents whose evaluation criteria may subtly drift from the designed criteria.

**Countermeasure**: Explicit **goal anchors** — external evaluation criteria that are not
derived from the system's own outputs. Human review, independent benchmarks, and formally
specified invariants (property tests) serve as goal anchors.

---

## 5. Satisficing to Local Minima

**Mechanism**: A system that satisfices (meets a threshold rather than maximizes) can
develop effective goals at local minima: the system stops exploring once it meets the threshold,
and the threshold becomes its de facto goal.

**Cognitive architecture analog**: Gate thresholds are satisficing criteria. An Engram that
just barely passes the Gate has "succeeded" — it has no incentive to be higher quality.
If the system consistently produces Engrams at the Gate threshold and no higher, the Gate
threshold is the effective quality goal.

This is a subtle but important observation: the Gate threshold sets a floor on quality, but
if the system has no other pressure to exceed the floor, the floor becomes the ceiling.

---

## Key References

- **Omohundro, S. M. (2008).** "The Basic AI Drives." In *Proceedings of the 2008 Conference
  on Artificial General Intelligence*, 171, 171–195. Instrumental convergence theorem.

- **Bostrom, N. (2014).** *Superintelligence*. Oxford University Press. Convergent instrumental
  goals in AI systems.

- **Goodhart, C. A. E. (1975).** "Problems of Monetary Management: The U.K. Experience."
  *Papers in Monetary Economics*, 1. Original statement of Goodhart's Law.

- **Hubinger, E., et al. (2019).** "Risks from Learned Optimization in Advanced Machine
  Learning Systems." arXiv:1906.01820. Mesa-optimization and inner alignment.
