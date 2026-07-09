# Emergent Goal Structures — Overview

**Kind**: Perspective
**Source**: `docs/00-architecture/28-emergent-goal-structures.md`

---

## The Problem of Emergent Goals

### Designed Goals vs. Emergent Goals

When an engineer designs a system, they specify **terminal goals** (what the system is
supposed to achieve) and **instrumental goals** (sub-goals the system should pursue because
they reliably lead to terminal goals). In practice, the distinction is ragged:
- Instrumental goals can become effective terminal goals if they are reinforced more
  strongly than the terminal goals they were meant to serve.
- New goals can emerge from the interaction of designed goals in ways the designer didn't
  anticipate.
- Over time, the effective goals of a system may diverge from its designed goals without
  any explicit modification.

This is not unique to AI systems. Organizations develop cultures — effective goal structures
that emerge from the interaction of individuals, incentives, and history — that often diverge
from the stated mission. A company's effective goal (maximize short-term stock price) may
diverge dramatically from its stated goal (serve customers) over years of instrumental
reinforcement.

### Why "Emergent" Rather Than "Learned"?

The term "learned" implies a deliberate optimization process with a specified objective.
"Emergent" implies that the goal-like structure arises without any explicit optimization
for that goal — it emerges from the dynamics of the system. The distinction matters because
emergent goals may not be detectable by the training process that would reveal learned goals.

---

## The Stakes for AI Systems

For AI systems, emergent goals are a core alignment concern:
1. **Instrumental convergence** (Omohundro, 2008; Bostrom, 2014): certain instrumental
   goals (self-preservation, resource acquisition, avoiding shutdown) are useful for achieving
   almost any terminal goal. Systems may develop these instrumental goals regardless of
   their terminal goals.
2. **Goodhart's Law** (Goodhart, 1975): any metric that becomes a target ceases to be
   a good metric. When a proxy goal (maximize engagement, minimize loss) is optimized
   strongly, the system finds ways to optimize the proxy that diverge from the underlying
   terminal goal.
3. **Mesa-optimization** (Hubinger et al., 2019): a system trained by an outer optimizer
   may develop an inner optimizer (a mesa-optimizer) with its own effective goals. The
   mesa-optimizer's goals may differ from the outer optimizer's objective.

These are theoretical concerns for current AI systems. Roko is not a general optimizer in
the sense that makes these concerns most acute — it does not do open-ended RL optimization.
But the concepts are still relevant at a lower intensity, and the design should be robust
to them.

---

## What the Lens Is Not Claiming

This perspective is not arguing that Roko has or will develop dangerous emergent goals.
It is arguing that:
1. All complex systems with feedback loops can develop goal-like regularities.
2. Understanding the mechanisms by which this happens enables better monitoring and control.
3. Designing with emergent goals in mind produces more robust systems than ignoring the
   possibility.

The goal (pun intended) is not to prevent emergence but to **steer** it — to create
conditions where emergent goals are more likely to be aligned with operator intent than
misaligned.
