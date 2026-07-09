# Design Implications from Collective Intelligence

**Kind**: Perspective
**Source**: `docs/00-architecture/14-c-factor-collective-intelligence.md`

---

## Overview

The collective-intelligence lens is not merely descriptive — it generates concrete design
constraints and decision criteria. The following implications are derived from the *c*-factor
research, transactive memory theory, the Hong-Page diversity theorem, and the empirical record of
high-performing human teams.

Each implication is stated as a design constraint, accompanied by the theoretical grounding and
the specific Roko components it targets.

---

## Implication 1: Router Quality Is the Highest-Leverage Intervention

**Constraint**: Router capability-model accuracy should be treated as a tier-1 quality metric,
not an implementation detail.

**Grounding**: Woolley et al. (2010) found that transactive memory system strength was a better
predictor of group *c* than average individual IQ. The Router is Roko's transactive memory
directory. Errors in the Router's capability model — misrouting a problem to an under-qualified
operator — cascade through the entire loop.

**Components**: [Router](../../../reference/05-operators/router.md),
[Scorer](../../../reference/05-operators/scorer.md)

**Measurement**: Routing accuracy should be tracked as an observable: what fraction of problems
were assigned to the most capable available operator? This can be measured retrospectively by
correlating routing decisions with downstream output quality.

---

## Implication 2: Scorer Diversity Must Be Structural, Not Superficial

**Constraint**: Scorer functions must be designed to have genuinely uncorrelated failure modes.
Diversity in implementation details that converges on the same underlying signal is not
sufficient.

**Grounding**: Hong and Page (2004) showed that error cancellation requires *diverse heuristics*,
not merely diverse implementors. Two scoring functions that both rely on the same underlying
embedding model are correlated at the level that matters: they will fail together on the same
inputs.

**Components**: [Scorer](../../../reference/05-operators/scorer.md)

**Concrete recommendation**: Audit the Scorer ensemble for correlations on adversarial inputs
specifically — inputs that are unusual or that the system has not encountered before. These are
precisely the cases where error cancellation matters most, and where correlated failure is most
dangerous.

---

## Implication 3: Composer Must Preserve Minority Positions, Not Just Consensus

**Constraint**: The Composer synthesis step should not silently resolve conflicts between
operators. Minority positions that are well-evidenced or high-stakes should be surfaced in the
final output, flagged rather than discarded.

**Grounding**: Groupthink (Janis 1972) and shared information bias (Stasser & Titus 1985) both
arise from premature convergence — suppressing the minority view before its information value
has been fully evaluated. In human groups, the fix requires active protocol intervention.
For Roko, the equivalent is an architectural constraint on Composer: it must not discard minority
operator outputs that exceed a confidence or novelty threshold.

**Components**: [Composer](../../../reference/05-operators/composer.md),
[Policy](../../../reference/05-operators/policy.md)

**Concrete recommendation**: Composer produces a primary synthesis *and* a structured list of
dissenting operator outputs when operator scores deviate significantly from the majority. This
creates an explicit minority-view channel that downstream consumers can attend to.

---

## Implication 4: Policy Must Encode Coordination Protocols, Not Just Constraints

**Constraint**: Policy should encode *positive* coordination protocols — explicit rules for how
operators interact, resolve conflicts, and sequence contributions — not only negative constraints
(prohibitions and limits).

**Grounding**: Tetlock's superforecaster teams outperformed unstructured groups precisely because
they had explicit protocols for eliciting diverse views, updating on evidence, and revising
estimates. The protocol did the work — individuals following it performed better even holding
individual ability constant. Policy is Roko's mechanism for encoding these protocols.

**Components**: [Policy](../../../reference/05-operators/policy.md)

**Concrete examples of positive protocols**:
- *Mandatory minority-view elicitation*: before Composer synthesis, query the lowest-scoring
  operators for their assessment of the input.
- *Explicit update step*: after new information is received, route through Scorer again before
  routing to action operators.
- *Dissent preservation*: if two operators produce contradictory assessments above a confidence
  threshold, both are preserved in the response payload.

---

## Implication 5: Group Size Bounding

**Constraint**: The number of operators active in a single loop pass should be bounded. The
bound should be derived empirically from coordination-overhead measurements, not set arbitrarily.

**Grounding**: *C*-factor degrades with group size above moderate thresholds. The mechanism
involves coordination overhead, attention fragmentation, and authority-gradient effects. For Roko,
the analogues are:
- Coordinator overhead: Router and Composer cost increases super-linearly with operator count.
- Attention fragmentation: each operator attends to a smaller fraction of the input when the
  pool is larger.
- Authority gradients: high-weight operators suppress the signal of low-weight operators in
  aggregation.

**Components**: [Router](../../../reference/05-operators/router.md),
[Composer](../../../reference/05-operators/composer.md)

**Measurement**: Run controlled experiments varying operator pool size and measuring output
quality on a fixed benchmark. Identify the inflection point where adding operators stops helping
and begins to hurt. Set that as the default bound.

---

## Implication 6: Organisational Learning Must Close the Loop

**Constraint**: The learning loop (Neuro / Dreams) must update the Router's capability model, not
just individual operator weights. Otherwise, the transactive memory directory drifts out of sync
with actual operator capabilities as those capabilities evolve.

**Grounding**: In human organisations, a persistent failure mode is *directory decay*: the
shared model of who-knows-what becomes stale as members gain and lose expertise. Groups with
stale directories route problems incorrectly and over-rely on members whose expertise has moved
on. Roko faces the same risk: as operators are updated or replaced, the Router's capability model
must be updated to reflect the new reality.

**Components**: [Neuro cross-cut](../../../reference/09-cross-cuts/README.md),
[Dreams](../../../reference/09-cross-cuts/README.md),
[Router](../../../reference/05-operators/router.md)

**Concrete recommendation**: After each Dreams consolidation cycle, emit a Router-capability audit
event that checks whether routing decisions made in the previous period were optimal given the
outcomes observed. Flag any systematic routing errors for Router model update.

---

## Summary Table

| Implication | Constraint | Primary target | Measurement |
|---|---|---|---|
| 1. Router quality | Tier-1 quality metric | Router, Scorer | Routing accuracy vs. outcome |
| 2. Scorer diversity | Structural uncorrelated failure modes | Scorer | Correlation on adversarial inputs |
| 3. Minority preservation | Surfaced, not discarded | Composer, Policy | Minority-view surfacing rate |
| 4. Positive protocols | Encode coordination, not just prohibition | Policy | Protocol compliance rate |
| 5. Group size bounding | Empirically derived active-operator bound | Router, Composer | Quality vs. operator count benchmark |
| 6. Directory maintenance | Router model updated by learning loop | Neuro, Dreams, Router | Routing error rate over time |

---

## Open Questions

- **What is the right measurement for routing accuracy?** The gold standard is retrospective
  comparison: given a problem and its eventual quality outcome, was the problem optimally routed?
  This requires a quality oracle that may not exist in all cases.
- **How should "minority position" be defined operationally?** A fixed threshold on operator
  score deviation is simple but may not capture the cases where minority views matter most.
  Is there a better operational definition grounded in information-theoretic terms?
- **Can Composer's integration loss be measured directly?** If operators produce contributions
  and Composer discards some, can the contribution of discarded inputs to ground-truth quality be
  estimated after the fact?
- **Does group-size bounding interact with the three-speed architecture?** The right bound may
  differ by speed tier: T0 may require fewer operators (speed) while T2 may benefit from more
  (depth). How should the bound be parameterised per tier?
