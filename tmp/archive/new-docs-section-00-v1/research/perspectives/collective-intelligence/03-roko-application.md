# Roko Through the Collective-Intelligence Lens

**Kind**: Perspective
**Source**: `docs/00-architecture/14-c-factor-collective-intelligence.md`

---

## The Central Claim

Roko is not a single AI agent. It is a *cognitive collective* — a structured ensemble of
specialised operators, memory substrates, and coordination mechanisms that jointly produce
intelligent behaviour that no single component could produce alone. The collective-intelligence
lens makes this structure legible: every architectural choice has an analogue in what we know
about how groups achieve (or fail to achieve) cognitive synergy.

The question this lens asks of every design decision: *Does this increase the system's effective
c-factor?*

---

## The Universal Cognitive Loop as Group Process

The [Universal Cognitive Loop](../../../reference/06-loop/README.md) — perceive, score, route,
act, remember — is not a serial pipeline through a single agent. It is a coordination protocol
for a cognitive collective. Each pass through the loop is a round of collaborative sensemaking:

1. **Perception** (Gate + Scorer): Multiple parallel scoring functions evaluate an input
   simultaneously. This is the analogue of a group's initial information gathering — different
   members attend to different signal dimensions.
2. **Routing** (Router): The transactive memory directory in action. The system selects which
   specialised agent holds the expertise relevant to this problem. Routing quality is limited by the
   accuracy of the capability model — exactly as a group's transactive memory system is limited by
   directory accuracy.
3. **Action** (Composer): Synthesis of contributions from multiple operators into a coherent
   response. The analogue of group deliberation and output production.
4. **Memory** (Neuro / Substrate): Updating the shared epistemic state. The analogue of shared
   external representations — whiteboards, documents — that persist beyond the conversation.

---

## Mapping Roko Operators to Group Roles

### Gate as Gatekeeper

The [Gate](../../../reference/05-operators/gate.md) controls information entry. In group terms,
it is the *boundary spanner* — the member who regulates what external information enters the
group's working memory. Gatekeeper quality determines whether the group works on the right
problem. A Gate that is too permissive floods the system with noise; one that is too restrictive
deprives specialists of the signal they need.

The group-cognition analogy has a direct design implication: Gate should be calibrated to the
downstream capability of the collective, not just to an absolute signal threshold.

### Scorer as Collective Evaluator

The [Scorer](../../../reference/05-operators/scorer.md) implements multi-dimensional evaluation.
The *c*-factor research establishes that groups with diverse evaluation criteria perform better
than groups that converge on a single criterion too quickly. Scorer embodies this: by maintaining
a battery of scoring dimensions (relevance, novelty, coherence, risk), it resists premature
convergence.

This maps to the structured debate protocols that enhanced Tetlock's superforecaster teams:
*before converging, explicitly evaluate from multiple angles*.

The failure mode — scoring axes that are too correlated — is the AI analogue of shared
information bias (Stasser & Titus 1985): redundant evaluators produce a false sense of consensus.

### Router as Transactive Memory System

The [Router](../../../reference/05-operators/router.md) is the operational heart of Roko's
transactive memory. Its quality is determined by:

1. **Directory accuracy**: Does it correctly model which agents are capable of which tasks?
2. **Routing latency**: Can it make dispatch decisions fast enough to be useful?
3. **Coverage**: Are there gaps — problem types for which no capable agent exists?
4. **Load balancing**: Does it avoid concentrating work on a single agent, creating a bottleneck
   and a single point of failure?

Woolley et al. (2010) found that transactive memory system strength predicted *c* more reliably
than any single member's IQ. This translates directly: Router quality is a stronger predictor of
system-level performance than any individual operator's capability.

### Composer as Group Output Synthesiser

The [Composer](../../../reference/05-operators/composer.md) synthesises contributions from
multiple operators into a coherent final response. In group terms, this is the most demanding
coordination task — where individual contributions must be integrated without losing their
distinct signal, while producing an output that is more than a union of parts.

The primary risk is *integration loss*: the Composer discards minority perspectives that contain
the most novel information, privileging consensus over accuracy. This is the AI analogue of
groupthink (Janis 1972): premature closure on a dominant narrative.

Mitigation: Composer should maintain and propagate *uncertainty* and *attribution* — knowing
which operators contributed which claims, and flagging when contributions conflict rather than
silently resolving the conflict.

### Policy as Group Norms

The [Policy](../../../reference/05-operators/policy.md) operator encodes the rules that constrain
operator behaviour. In group-intelligence terms, Policy is the system of *norms, protocols, and
shared mental models* that coordinate behaviour without requiring explicit negotiation on every
decision.

High-*c* groups develop strong shared norms for conflict resolution, information sharing, and role
definition. These norms reduce coordination overhead dramatically — the group doesn't deliberate
on process when it should be deliberating on content. Policy plays this role for Roko.

### Neuro Cross-Cut as Organisational Learning

The [Neuro cross-cut](../../../reference/09-cross-cuts/README.md) and the [Dreams
subsystem](../../../reference/09-cross-cuts/README.md) implement the learning loop: updating
weights, forming new associations, consolidating memories during low-activity periods.

In collective intelligence terms, this is *organisational learning* — the mechanism by which
group experience is encoded into the group's operating procedures, shared models, and member
expertise. Argyris and Schön (1978) distinguished *single-loop learning* (adjusting behaviour
within a fixed framework) from *double-loop learning* (adjusting the framework itself). Dreams is
architecturally capable of double-loop learning: it can revise the associations that structure
future routing, scoring, and composition decisions.

---

## Diversity and Error Cancellation in Scoring

The Hong-Page theorem (2004) requires that diversity is in *heuristics*, not merely in superficial
features. For Roko's Scorer ensemble to benefit from the theorem, scoring functions must:

1. **Use genuinely different features**: a relevance scorer based on keyword matching and a
   relevance scorer based on semantic embedding are not diverse in the Hong-Page sense if they are
   correlated on the inputs that matter.
2. **Be individually competent**: the theorem requires that each member exceeds chance. A diverse
   ensemble of incompetent scorers does not cancel errors — it aggregates them.
3. **Operate independently before aggregation**: if scorers share intermediate representations
   (e.g., a single embedding model whose outputs feed all scorers), they are not independent, and
   the error-cancellation guarantee weakens.

These conditions should be treated as architectural invariants, not aspirational properties.

---

## The Shared Information Bias Risk

Stasser and Titus (1985) showed that groups reliably over-discuss information that all members
already know, and under-discuss uniquely-held information — even when the unique information is
decision-critical. In Roko terms:

- **Over-represented information**: Patterns that appear in many training contexts, that many
  operators have processed before, that the Scorer assigns high familiarity — these will be
  over-weighted.
- **Under-represented information**: Novel inputs, rare problem types, edge cases — these will be
  under-weighted precisely because they are not salient to most operators.

The architectural fix is *active elicitation of minority views*: before Composer synthesis,
explicitly query operators that have *not* yet contributed, or that scored the input differently
from the majority. Policy can enforce this as a mandatory minority-view step.

---

## Three-Speed Architecture as Cognitive Tempo

The [Three cognitive speeds](../../../reference/07-speeds/README.md) map onto the three tempos
of collective intelligence:

| Speed tier | Group analogue | Function |
|---|---|---|
| **T0 — Fast** | Immediate reaction | Shared situational awareness; fast signals |
| **T1 — Working** | Active deliberation | Group problem-solving, role coordination |
| **T2 — Slow** | Reflection and learning | Organisational learning, norm revision |

Collective intelligence theory predicts that groups which *cannot* shift tempo — that deliberate
when they should react, or react when they should deliberate — fail systematically. The
three-speed architecture is Roko's solution to this problem.

---

## Scaling Considerations

The *c*-factor literature consistently finds that *c* degrades with group size above moderate
thresholds. The mechanism is not fully understood, but leading hypotheses include:

- **Coordination overhead** scales super-linearly with agent count
- **Authority gradients** intensify: a dominant operator can suppress diverse minority signals
- **Attention fragmentation**: each agent attends to a smaller fraction of the total input stream

For Roko, this has architectural implications: the number of active operators in any single loop
pass should be bounded. Routing should concentrate work, not distribute it maximally. The goal is
a cognitive *team*, not a cognitive *crowd*.

---

## Open Questions

- **What is Roko's effective group size?** The *c*-factor literature gives guidance for groups of
  2–20 humans. How do these results scale to systems with hundreds of operators?
- **Can Roko's c-factor be measured directly?** Designing a battery of diverse cognitive tasks and
  running factor analysis on operator performance would produce an empirical *c* estimate. Is this
  feasible operationally?
- **Who detects and fixes shared information bias?** The Stasser-Titus effect requires active
  protocol intervention. Which Roko component is responsible for ensuring under-represented
  information receives attention?
- **How should Composer handle genuine conflicts between operators?** Current synthesis likely
  produces a weighted average. But sometimes the minority operator is right. What is the mechanism
  for surfacing and preserving minority positions?
- **Is there an authority gradient risk in Roko?** If some operators are systematically weighted
  more heavily by the Scorer, their outputs will dominate Composer input — suppressing signal from
  lower-weighted operators exactly when those operators hold unique information.
