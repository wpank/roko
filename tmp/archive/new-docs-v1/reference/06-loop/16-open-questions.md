# Loop Open Questions

> Unresolved design decisions and known gaps in the cognitive loop specification.

**Last reviewed**: 2026-04-19

---

## OQ-1: Sub-agent result integration

When ACT dispatches a sub-agent asynchronously, the result arrives as a Pulse in a
later tick. Currently, the parent agent re-runs the full QUERY → SCORE → ROUTE →
COMPOSE cycle when the sub-agent's result Pulse arrives. This is correct but
potentially wasteful — the parent may already have all the context needed to compose
the sub-agent's result with the original question.

**Candidate solutions**:
- "Context continuation": store the partially-assembled context from the original tick
  as a special Engram, and resume COMPOSE from there when the result arrives.
- Accept the current approach as-is; the redundant QUERY/SCORE pass is cheap.

---

## OQ-2: Budget accounting for multi-tick tasks

A task that spans multiple ticks (e.g., a research agent making 10 sub-agent calls)
may exhaust its per-tick budget on each individual tick, but the *total* cost of the
task is not tracked anywhere. The orchestrator has no visibility into whether a long-
running task is on budget.

**Candidate solutions**:
- Task-level budget tracking: introduce a `TaskId` that groups related ticks, with an
  aggregate budget.
- Rely on the existing per-tick budgets and flag tasks that exceed N ticks.

---

## OQ-3: VERIFY and hallucination detection at scale

The current `hallucination_check` gate compares model output to the composed context.
This catches claims that contradict the context but does not catch:
- Claims that are true but not in the context (the model used its training knowledge)
- Claims that are false but consistent with the context (the context itself was wrong)

A more principled hallucination check would require grounding in an external fact
source, which is expensive and introduces a dependency on network availability.

---

## OQ-4: Parallelism within a tick

QUERY and active inference prediction could in principle run in parallel (prediction
does not depend on QUERY output). SCORE and COMPOSE stages are sequential by current
design but could theoretically overlap with background pre-fetching. Introducing
parallelism would reduce tick latency at the cost of code complexity and reasoning
difficulty.

---

## OQ-5: Predictive QUERY (anticipatory retrieval)

The current QUERY is reactive: it retrieves candidates in response to the current
stimulus. An anticipatory QUERY would retrieve candidates for *expected future stimuli*
during idle time, pre-warming the substrate cache. This is the retrieval-system analog
of active inference's prediction step.

---

## OQ-6: Cross-agent loop composition

When two Roko agents operate in the same environment, their loops are independent. A
stimulus that arrives at agent A may cause agent A to dispatch to agent B, whose
result arrives back at agent A in a future tick. This works but creates latency.
A tighter integration ("shared tick context") would allow the two loops to synchronize
within a single tick, at the cost of coupling the two agents' execution cycles.

---

## See also

- [Invariants](12-invariants.md) — what is settled
- [Failure Modes](13-failure-modes.md) — known failure patterns
- [Active Inference](11-active-inference.md) — OQ-3 and OQ-5 are related to the active
  inference model
