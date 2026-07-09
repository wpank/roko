# Collective Intelligence — Overview

**Kind**: Perspective
**Source**: `docs/00-architecture/14-c-factor-collective-intelligence.md`

---

## The Central Claim

The intelligence of a group is not a simple function of the intelligence of its members.
Woolley et al. (2010) demonstrated this empirically: a single factor (*c*, collective
intelligence) predicts group performance across diverse tasks, and *c* is largely
independent of the mean or maximum individual intelligence of group members.

What predicts *c*:
1. **Social sensitivity**: how accurately group members model each other's mental states.
2. **Conversational balance**: how evenly speaking turns are distributed.

Not: average IQ, maximum IQ, group cohesion, satisfaction.

The implication for multi-agent AI systems is direct and non-obvious: **optimizing individual
agents may be less important than optimizing the interaction structure**. A group of
moderately capable agents with high mutual understanding and balanced contribution may
outperform a group of highly capable agents that communicate poorly.

---

## Why This Is Non-Obvious

The engineering intuition is to decompose a problem, assign parts to agents, and sum the
results. This intuition works when problems are decomposable and outputs are combinable.
It breaks down when:
- Problems require **integrating diverse perspectives** (not just combining separate results).
- Solutions require **recognizing connections** between agents' independently held knowledge.
- Quality depends on **identifying which outputs are reliable** (which requires modeling
  other agents' uncertainty).

For these problem types — which are precisely the "hard" problems that matter most —
collective intelligence is determined by interaction quality, not individual quality.

---

## The Group as a Cognitive Unit

The c-factor literature treats the **group as a unit of analysis** distinct from its members.
The group has an intelligence level (*c*) that is not reducible to individual levels. This
is the same theoretical stance taken in complex systems analysis: emergent properties of
systems are not predictable from components alone.

Treating the group as a cognitive unit means:
- Monitoring group-level metrics (not just individual agent metrics)
- Diagnosing group-level failure modes (not just individual agent failures)
- Designing interaction structures (not just agent capabilities)
- Measuring c (not just measuring individual task performance)

This is a design perspective shift: from "design better agents" to "design better agent
interaction."

---

## Scope

This perspective covers:
- What *c* is and how to measure it in AI agent systems.
- What structural properties of multi-agent Roko deployments predict *c*.
- How Roko's architecture can be instrumented to monitor and optimize *c*.
- The limits of the analogy (where human group dynamics don't transfer to AI agents).
