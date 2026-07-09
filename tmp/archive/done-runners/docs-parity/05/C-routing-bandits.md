# C — Routing + Bandits

Audit-corrected parity view of the routing and bandit docs in `docs/05-learning/`.

---

## What Is Already Shipped

- `CascadeRouter` is live.
- UCB1, Track-and-Stop, LinUCB, and the current Pareto frontier are live.
- `active_inference.rs` is live, and the router already has an active-inference tier-selection hook.
- prompt experiments already feed back into routing behavior.

## What The Old Parity Material Overstated

- the routing core is **not** missing; it is already broad and wired,
- the missing pieces are mostly research extensions that should not be described in present tense,
- contextual Thompson, NeuralUCB, ensembles, lookahead routing, cost-spectrum routing, and richer router calibration remain design work,
- c-factor should stay an input or measurement surface, not the canonical theory of routing.

## Corrected Status

### Shipping

- `CascadeRouter`
- UCB1 and `BanditBank`
- Track-and-Stop
- LinUCB
- 2D Pareto frontier
- active-inference tier selection

### Ship Soon

- better operator-visible summaries around routing decisions,
- one explicit calibration path tied to routing logs,
- tighter docs around the actual stage transitions and cost-pressure behavior.

### Deferred

- contextual Thompson
- NeuralUCB
- bandit ensembles
- lookahead / planner-aware routing
- cost-spectrum routing
- 4D Pareto and full router-calibration research

## Practical Rewrite Guidance

When touching routing docs:

1. keep the current cascade/UCB1/active-inference path in present tense,
2. mark the research routers as `planned` or `design-only`,
3. do not let doc cleanup turn into a new routing-algorithm roadmap.

## Batch-Ready Follow-Ups

- `L4`: canonicalize predictive calibration around the current routing-log path
- `L5`: make budget pressure and experiment winners easier to inspect without changing the router family

## Source Anchors

- `crates/roko-learn/src/cascade_router.rs:994` — `CascadeRouter`
- `crates/roko-learn/src/cascade_router.rs:1213` — active-inference tier hook
- `crates/roko-learn/src/bandits.rs:408` — UCB1 selection
- `crates/roko-learn/src/bandits.rs:511` — `BanditBank`
- `crates/roko-learn/src/model_router.rs:60` — routing constants
- `crates/roko-learn/src/active_inference.rs:17` — `BeliefState`
- `crates/roko-learn/src/active_inference.rs:83` — `select_tier`

## Bottom Line

The routing docs should now read like documentation for an existing system with a few well-bounded gaps, not like a pitch deck for router research that still has to be built.
