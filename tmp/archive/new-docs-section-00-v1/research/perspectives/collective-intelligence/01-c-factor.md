# The C-Factor Across Cohorts

**Kind**: Perspective
**Source**: `docs/00-architecture/14-c-factor-collective-intelligence.md`

---

## Measuring C

The *c*-factor is measured by factor analysis of group performance across a battery of
diverse cognitive tasks. For the measurement to be valid:
1. The tasks must be genuinely diverse (different cognitive demands).
2. Performance must be quantifiable for each task.
3. A factor analysis must be applied to the group-by-task performance matrix.
4. If a dominant first factor emerges, it is *c*.

This procedure produces a scalar *c* for each group at a moment in time. *C* is:
- **Ordinal**: groups can be ranked by *c*, but the absolute differences are not directly
  interpretable without calibration.
- **Dynamic**: *c* changes over time as the group's composition and interaction patterns
  change.
- **Contextual**: *c* measured on one battery of tasks may not transfer to very different
  task domains.

---

## C Across Different Group Types

### Face-to-Face Groups (Woolley et al., 2010)

192 groups of 2–5 members. Strong *c* factor emerged explaining ~43% of variance in group
task performance. Predictors: social sensitivity, conversational balance.

### Online Groups (Engel et al., 2014)

Groups collaborating via text only. *C* factor replicated with similar structure. Social
sensitivity (RME scores) remained predictive, suggesting it measures something deeper
than face reading.

### Large Groups

The original studies used small groups (2–5 members). Subsequent work suggests that *c*
may degrade in large groups without explicit coordination structures: as group size
increases, maintaining equal conversational balance becomes harder (a few voices dominate
by default), and social sensitivity fails to scale (modeling many minds simultaneously
is harder than modeling a few).

For large multi-agent systems, this suggests that *c* must be architected rather than
left to emerge: explicit mechanisms for balanced contribution and mutual modeling become
necessary.

---

## Temporal Stability of C

Woolley et al. found that *c* is moderately stable over time: groups re-tested weeks later
showed similar *c* values. This stability is important because it means *c* is a property
of the group's interaction structure, not just its current task.

For AI agent systems, temporal stability of *c* would mean that a fleet's collective
intelligence is a persistent property that can be improved through architectural changes —
not just a transient artifact of the current task.

---

## C in Human-AI Hybrids

Emerging research (Westby & Riedl, 2023) suggests that human-AI collaborative groups
also exhibit a measurable *c* factor, and that the human-AI interaction dynamics predict *c*
similarly to human-human interactions. Groups where the AI demonstrates higher "social
sensitivity" (better models of what the human needs, better calibrated uncertainty) show
higher *c*.

**Implication for Roko**: In human-Roko collaborative settings, Roko's *c* contribution
depends on how accurately it models the human user's knowledge state, uncertainty, and
goals. A Roko that provides confident, detailed outputs without modeling the user's
current understanding may dominate the collaborative interaction — reducing *c* just as
a dominant human speaker would.

---

## Reference

- **Westby, S., & Riedl, M. (2023).** "Collective Intelligence of Teams of Humans and
  Language Models." *Collective Intelligence*, 2(2).
