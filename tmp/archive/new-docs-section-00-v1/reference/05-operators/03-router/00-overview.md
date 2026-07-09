# Router Overview

> `Router` maps an (`Engram`, `Score`) pair to an `Action` — the thing the agent will do
> next. It is the decision-making operator of the cognitive loop.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [Score](../../10-types/score.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`Router::route(engram, score) -> Action` returns the action to take. Three routing
strategies are provided: `StaticRouter` (rule-based), `ConfidenceRouter` (score-driven),
and `UCBRouter` (bandit-driven exploration). `CascadeRouter` tries them in sequence,
falling back to the next strategy on `None`.

---

## What Routing Does

After an `Engram` has been scored and gated, the agent must decide what to do with it.
Options might include: call a specific tool, ask a clarifying question, retrieve more
context, respond to the user, or defer to a different agent.

The Router makes this decision. It is the "frontal lobe" of the loop — after the sensory
and evaluative steps, routing is where the agent commits to a course of action.

---

## Three Strategies

### 1. Static (Rule-Based)

Matches the `Engram` against a set of deterministic rules (kind, score range, topic):

```
If kind == Task AND confidence > 0.8 → Action::ExecuteTask
If kind == Question → Action::RetrieveContext
else → None (fall through to next strategy)
```

### 2. Confidence (Score-Driven)

Ranks available actions by expected score correlation and picks the highest-ranked action.
Deterministic given the same score; no exploration.

### 3. UCB (Bandit-Driven Exploration)

Uses a multi-armed bandit algorithm (Upper Confidence Bound) to balance exploitation
(actions with high historical success) and exploration (actions with uncertain outcomes).

---

## Cascade

`CascadeRouter` tries the three strategies in order: Static → Confidence → UCB. The first
strategy that returns a non-`None` action wins. This provides a sensible fallback: use
rules when they apply, use confidence scoring when rules don't match, and fall back to
bandit exploration for novel situations.

---

## Where Router Fits

```
GATE → ROUTE ← router.route(engram, score)
          ↓
         Action
          ↓
     COMPOSE ← composer.compose(ctx, action)
```

---

## See Also

- [Semantics](./02-semantics.md)
- [Bandit Integration](./09-bandit-integration.md)
- [Composer Overview](../04-composer/00-overview.md)
