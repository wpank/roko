# Active Inference — Theoretical Foundations

> The free-energy principle and active inference: the theoretical basis for Roko's
> predict-and-correct processing model. This page covers the science; architectural
> application lives in [`reference/06-loop/11-active-inference.md`](../../reference/06-loop/11-active-inference.md).

**Kind**: Foundation
**Source**: `docs/00-architecture/11-dual-process-and-active-inference.md` (theoretical-foundations section)
**Related architecture**: [`reference/06-loop/11-active-inference.md`](../../reference/06-loop/11-active-inference.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Active inference (Friston et al., 2010) proposes that biological agents act to minimize
variational free energy — a bound on the surprise (negative log-evidence) of sensory
observations. Rather than reacting to stimuli, an agent continuously generates predictions
about its sensory inputs and updates those predictions when they are wrong. The distinction
that matters for Roko: perception is the process of minimizing prediction error by updating
beliefs; action is the process of minimizing prediction error by changing the world to match
predictions. Both are free-energy minimization under different constraints.

---

## The Free-Energy Principle

### Variational Free Energy

Karl Friston's free-energy principle (Friston, 2010; Friston & Stephan, 2007) states that
living systems resist the tendency toward disorder by minimizing a quantity called variational
free energy \( F \). Formally:

\[
F = E_q[\ln q(s)] - E_q[\ln p(o, s)]
\]

where:
- \( q(s) \) is the agent's approximate posterior distribution over hidden states \( s \)
- \( p(o, s) \) is the agent's generative model — the joint probability of observations
  \( o \) and hidden states
- The expectation \( E_q \) is taken under the approximate posterior

This quantity upper-bounds **surprise** (also called surprisal or self-information):

\[
F \geq -\ln p(o)
\]

An agent that minimizes \( F \) therefore minimizes surprise about its sensory observations
— it keeps the world in states it "expects," where expectation is encoded by the generative
model.

### Decomposition: Accuracy and Complexity

Free energy decomposes into two intuitive terms:

\[
F = \underbrace{-E_q[\ln p(o | s)]}_{\text{inaccuracy}} + \underbrace{D_{KL}[q(s) \| p(s)]}_{\text{complexity}}
\]

- **Inaccuracy** measures how poorly the agent's current beliefs predict its observations.
  High inaccuracy means the generative model is failing.
- **Complexity** (KL divergence from prior to posterior) measures how far the agent's updated
  beliefs have moved from its prior. High complexity means the agent has had to revise its
  model extensively.

Minimizing free energy balances these: the agent should revise its beliefs enough to explain
its observations (reduce inaccuracy) but not more than necessary (keep complexity low). This
is Bayesian inference carried out approximately and efficiently.

### Expected Free Energy and Planning

For action selection, agents minimize **expected free energy** \( G(\pi) \) over a policy
\( \pi \) (a sequence of actions):

\[
G(\pi) = \underbrace{-E_{\tilde{q}}[\ln p(\tilde{o})]}_{\text{risk}} + \underbrace{H[q(\tilde{o} | \tilde{s}, \pi)]}_{\text{ambiguity}}
\]

where \( \tilde{o} \) denotes future observations and \( \tilde{s} \) denotes future states.

- **Risk** is the KL divergence between predicted future observations and preferred outcomes.
  An agent that cares about reaching goal states encodes those states as preferred observations.
- **Ambiguity** is the expected conditional entropy of future observations — the agent's
  uncertainty about what it will observe even given future states. Low ambiguity means the
  agent's generative model is well-calibrated about the consequences of its actions.

Policies that minimize \( G(\pi) \) are both goal-directed (risk minimization) and
epistemically curious (ambiguity minimization through information-seeking). This unifies
goal-directed behavior and exploration in a single framework.

---

## Markov Blankets

### The Concept

A **Markov blanket** is the set of variables that separates an agent from its environment
in a statistical sense: knowing the Markov blanket of a system makes its internal states
conditionally independent of external states (Pearl, 1988).

For a Bayesian network, the Markov blanket of a node consists of its parents, children, and
co-parents (other parents of its children). For a dynamical system, the Markov blanket
consists of **sensory states** (external → internal influence) and **active states**
(internal → external influence).

### Relevance to Active Inference

The Markov blanket structure is what makes active inference viable. An agent can only
interact with the world through its blanket: it receives observations through sensory states
and acts on the world through active states. The blanket is the interface.

Key insight (Friston, 2013): any system that maintains a Markov blanket over time — that is,
any system that persists as a distinct entity — can be described as performing active
inference. The blanket structure *implies* the existence of an internal generative model, even
if that model is not explicitly represented. This is why active inference has been applied not
just to brains but to cells, immune systems, and multi-agent systems.

### Nested Blankets

Blankets can be nested: cells within tissues within organs within organisms within social
groups. At each level, the aggregate system has its own Markov blanket and can be described
as minimizing free energy at that scale. This nesting is important for understanding
hierarchical prediction.

---

## Predictive Processing

### The Hierarchy

Predictive processing (Clark, 2013; Rao & Ballard, 1999) is the computational implementation
of the free-energy principle in hierarchical systems. Higher levels of a hierarchy generate
predictions about the activity of lower levels. Prediction errors propagate upward when
predictions fail. Beliefs flow downward to suppress prediction errors.

The key quantities at each level:
- **Predictions** (\( \hat{x} \)): top-down beliefs about lower-level activity
- **Prediction errors** (\( \epsilon \)): bottom-up discrepancy between prediction and
  actual activity
- **Precision** (\( \Pi \)): the confidence attached to predictions, which modulates how
  strongly prediction errors are weighted

### Precision Weighting and Attention

**Attention**, in the predictive processing framework, is the optimization of precision
weights (Feldman & Friston, 2010). Attending to a stimulus increases the precision of
predictions about it, amplifying the influence of prediction errors from that source. This
is not merely a metaphor: attentional phenomena (saccades, attentional cueing, sustained
attention) fall out naturally from this framework.

This links directly to the [attention-as-currency perspective](../perspectives/attention-as-currency/README.md):
if attention is precision weighting, then allocating attention is allocating the confidence
multiplier on prediction errors, which determines how strongly evidence updates beliefs.

### Temporal Depth

Predictions can extend over multiple time steps. A deep temporal hierarchy predicts not just
the next observation but sequences of observations — planning, in essence. The agent
minimizes expected free energy over trajectories, not just moment-to-moment observations.
This temporal extension is what distinguishes active inference from simple reactive systems.

---

## Dual Process and Active Inference

### System 1 and System 2 as Precision Regimes

Kahneman's (2011) dual-process theory distinguishes fast, automatic, low-effort cognition
(System 1) from slow, deliberate, high-effort cognition (System 2). Within active inference,
this distinction maps naturally onto precision:

- **System 1** (T0/T1 in Roko): high-precision priors, low prediction error threshold,
  fast stereotyped responses. The generative model is highly confident; little updating
  occurs. Responses are cached policies.
- **System 2** (T2 in Roko): low-precision priors (high prior uncertainty), high prediction
  error threshold triggers model updating. The agent invests in resolving ambiguity through
  deliberate inference.

The switching between regimes is itself a free-energy computation: when accumulated prediction
error exceeds the cost of System 2 processing, the agent switches registers.

### Allostasis vs. Homeostasis

Classic homeostasis minimizes the *present* deviation from a setpoint. Active inference
generalizes this to **allostasis**: predicting future needs and pre-emptively acting to meet
them before deviation occurs. An allostatic agent models its own future states and acts to
keep those predictions in preferred regions. This is the theoretical basis for anticipatory
behavior in Roko's policy layer.

---

## Key Papers

- **Friston, K. (2010).** "The free-energy principle: a unified brain theory?" *Nature
  Reviews Neuroscience*, 11(2), 127–138. The foundational statement of the principle.

- **Friston, K. J., & Stephan, K. E. (2007).** "Free-energy and the brain." *Synthese*,
  159(3), 417–458. Formal derivation and connection to Helmholtz machines.

- **Friston, K. et al. (2017).** "Active inference: a process theory." *Neural Computation*,
  29(1), 1–49. The full process-level theory connecting FEP to neural circuits.

- **Rao, R. P. N., & Ballard, D. H. (1999).** "Predictive coding in the visual cortex: a
  functional interpretation of some extra-classical receptive-field effects." *Nature
  Neuroscience*, 2(1), 79–87. The neural implementation of predictive processing.

- **Clark, A. (2013).** "Whatever next? Predictive brains, situated agents, and the future
  of cognitive science." *Behavioral and Brain Sciences*, 36(3), 181–204. Synthesis of
  predictive processing across cognition.

- **Feldman, H., & Friston, K. J. (2010).** "Attention, uncertainty, and free-energy."
  *Frontiers in Human Neuroscience*, 4, 215. Attention as precision optimization.

- **Kahneman, D. (2011).** *Thinking, Fast and Slow*. Farrar, Straus and Giroux. Dual
  process theory — the psychological grounding.

- **Pearl, J. (1988).** *Probabilistic Reasoning in Intelligent Systems*. Morgan Kaufmann.
  Markov blankets.

---

## Open Questions

- To what extent does Roko's T0/T1/T2 tier structure map cleanly onto precision regimes, vs.
  being a coarser engineering approximation?
- Can Markov blanket structure be made explicit in the Engram graph, enabling formal
  verification of agent boundaries?
- Does allostatic prediction require a separate temporal prediction layer, or does it emerge
  from deep temporal hierarchies in the existing loop?

---

## See Also

- [`reference/06-loop/11-active-inference.md`](../../reference/06-loop/11-active-inference.md) — architectural application
- [`research/perspectives/attention-as-currency/README.md`](../perspectives/attention-as-currency/README.md) — attention as precision
- [`research/perspectives/emergent-goals/README.md`](../perspectives/emergent-goals/README.md) — goal-directed behavior
- [`research/perspectives/temporal-topology/README.md`](../perspectives/temporal-topology/README.md) — temporal structure
