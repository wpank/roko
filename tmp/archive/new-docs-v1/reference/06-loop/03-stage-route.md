# ROUTE — Stage 3 of the Cognitive Loop

> Select the winning sub-agent, tool, or model for the current task.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [Router operator](../05-operators/router.md),
[ScoredEngram](02-stage-score.md), [Dual-Process](10-dual-process.md)
**Used by**: [COMPOSE](04-stage-compose.md), [loop\_tick()](09-loop-tick-code.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

ROUTE takes the top-ranked scored candidates and the incoming stimulus, decides which
execution capability (sub-agent, model endpoint, or tool) should handle the task, and
returns a `RouteDecision`. A route decision carries both the selected target and a
confidence score. Low-confidence decisions escalate to slower, more deliberate
processing (the T1/T2 path in the dual-process model).

---

## The Idea

Routing is the point where the loop makes its highest-stakes decision: *who* handles
this task. A correct routing decision multiplies everything downstream. A wrong one
wastes the tick budget and may produce harmful output.

Roko's Router trait deliberately separates two concerns:

1. **Which capability?** — model A vs model B vs tool C vs sub-agent D
2. **With how much confidence?** — should this go through the fast path or be escalated
   for deeper deliberation?

The second concern connects directly to the dual-process model. If the Router returns
`confidence < threshold`, the loop switches to the T1 path (Theta speed), assembles a
richer context, and calls a more capable (and more expensive) model. If
`confidence < second_threshold`, the loop escalates to T2 (Delta), involving offline
consolidation or human-in-the-loop signaling.

---

## Specification

```rust
// source: crates/roko-agent/src/loop/route.rs
pub struct RouteDecision {
    pub target:     RouteTarget,
    pub confidence: f32,        // 0.0–1.0
    pub rationale:  Option<String>,
}

pub enum RouteTarget {
    Model(ModelId),
    Tool(ToolId),
    SubAgent(AgentId),
    Defer(DeferReason),  // escalate to slower tier or human
}

pub trait Router: Send + Sync {
    fn route(
        &self,
        candidates: &[ScoredEngram],
        stimulus:   &Pulse,
        context:    &RouterContext,
    ) -> Result<RouteDecision, RouterError>;
}
```

`RouterContext` carries the set of available targets, their capability descriptors,
current load and cost estimates, and the speed tier of this tick.

---

## The CascadeRouter

The default `Router` implementation in `roko-agent` is the `CascadeRouter`. It
applies three selection strategies in sequence:

1. **Static routing** — if the incoming Pulse has a `route_hint` tag, use it directly.
   No scoring needed; confidence = 1.0. This is the "muscle memory" path.

2. **Wilson CI routing** — compute a Wilson confidence interval over historical
   success rates for each candidate target given the stimulus type. If the best target
   has a tight interval above threshold, route there. This is the "habit" path.

3. **LinUCB routing** — if neither static nor Wilson routing yields a confident
   decision, apply a contextual bandit (LinUCB) to trade off exploration and
   exploitation. This is the "deliberate" path.

If even LinUCB yields `confidence < low_threshold`, the router emits `RouteTarget::Defer`.

---

## Dual-Process Integration

The `confidence` field of `RouteDecision` is the primary signal that drives tier
selection:

| Confidence | Tier selected | What happens |
|---|---|---|
| ≥ 0.85 | T0 (Gamma) | Proceed on fast path; COMPOSE uses minimal context |
| 0.60–0.85 | T1 (Theta) | Escalate; COMPOSE assembles richer context; slower model |
| < 0.60 | T2 (Delta) | Defer; publish `route.uncertain` Pulse; may pause for consolidation |

See [Dual-Process](10-dual-process.md) for the full model and thresholds.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| `RouterError::NoTargets` | No capable targets registered | Publish `route.failed` Pulse; abort tick |
| `RouterError::AllBusy` | All targets over load limit | Retry after `backoff_ms`; max 3 retries then defer |
| `RouterError::Timeout` | Router took > stage budget | Return best partial result; log warning |
| Confidence below both thresholds | Genuinely ambiguous task | `RouteTarget::Defer`; human signal or Delta consolidation |

---

## Performance

| Metric | Target | P99 budget |
|---|---|---|
| Static path (route\_hint) | < 0.1 ms | < 0.5 ms |
| Wilson CI path | < 1 ms | < 3 ms |
| LinUCB path | < 5 ms | < 12 ms |
| Total stage budget | < 8 ms | < 20 ms |

The static and Wilson paths are dominant in steady-state operation (> 80% of ticks).
LinUCB is the fallback for novel stimuli.

---

## Examples

### 1. High-confidence static route

An agent configured to always handle `kind=UserQuestion` with `model=gpt-4o` receives
a user question. The Pulse has `route_hint=gpt-4o`. Confidence = 1.0; the loop
proceeds on the Gamma fast path.

### 2. Escalation via LinUCB

A novel prompt arrives. Static routing finds no hint. Wilson CI has no history for
this stimulus type. LinUCB selects `model=claude-opus` with confidence 0.70. The loop
switches to T1 (Theta), queues a richer context assembly.

### 3. Deferral

A high-stakes financial decision arrives with conflicting prior evidence. LinUCB
confidence = 0.45. The loop emits `RouteTarget::Defer(DeferReason::LowConfidence)`.
A `route.uncertain` Pulse is published. The orchestrator flags this for human review.

---

## See also

- [Router operator](../05-operators/router.md) — how to implement a custom router
- [Dual-Process](10-dual-process.md) — how confidence drives tier selection
- [SCORE](02-stage-score.md) — produces the ranked candidates consumed here
- [COMPOSE](04-stage-compose.md) — assembles context for the selected target
- [Three Cognitive Speeds](../07-speeds/README.md) — the speed tiers escalated into
