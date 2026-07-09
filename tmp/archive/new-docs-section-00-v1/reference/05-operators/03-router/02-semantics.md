# Router Semantics — Static → Confidence → UCB Cascade

> The three routing strategies and how `CascadeRouter` combines them.

**Status**: Shipping
**Crate**: `roko-agent`
**Last reviewed**: 2026-04-19

---

## `Ok(None)` Is Not a Failure

`route` returns `Ok(None)` when the router has no applicable strategy. This is a valid
result — it tells the cascade to try the next strategy. Only `Err(RouterError)` indicates
a failure.

---

## Strategy 1: Static Router

Rule-based routing. A list of `(pattern, action)` pairs is evaluated in order; the first
match wins.

```rust
// source: crates/roko-agent/src/router.rs
pub struct StaticRouter {
    pub rules: Vec<RoutingRule>,
}

pub struct RoutingRule {
    pub kind_match: Option<Kind>,
    pub min_confidence: Option<f32>,
    pub max_confidence: Option<f32>,
    pub action: Action,
}
```
<!-- source: crates/roko-agent/src/router.rs -->

Returns `Ok(None)` if no rule matches.

---

## Strategy 2: Confidence Router

Ranks available actions by `score.utility * score.confidence`. Selects the top-ranked
action above a minimum threshold. Deterministic — no randomness.

Returns `Ok(None)` if no action exceeds the minimum threshold.

---

## Strategy 3: UCB Router

Multi-armed bandit exploration. Each `ActionKind` is an arm. The UCB formula:

```
ucb(arm) = mean_reward(arm) + C * sqrt(ln(total_pulls) / pulls(arm))
```

where `C` is the exploration constant (default: 1.41 ≈ sqrt(2)). The arm with the highest
UCB value is selected.

After each loop tick, the outcome is fed back via `update_reward(action, reward)`. This
updates the arm's mean reward and pull count.

Returns `Ok(None)` only if no arms have been initialised.

---

## CascadeRouter

```rust
// source: crates/roko-agent/src/router.rs
pub struct CascadeRouter {
    pub static_router: StaticRouter,
    pub confidence_router: ConfidenceRouter,
    pub ucb_router: UCBRouter,
}

impl Router for CascadeRouter {
    fn route(&self, engram: &Engram, score: &Score) -> Result<Option<Action>, RouterError> {
        if let Some(action) = self.static_router.route(engram, score)? {
            return Ok(Some(action));
        }
        if let Some(action) = self.confidence_router.route(engram, score)? {
            return Ok(Some(action));
        }
        self.ucb_router.route(engram, score)
    }
}
```
<!-- source: crates/roko-agent/src/router.rs -->

---

## See Also

- [Bandit Integration](./09-bandit-integration.md) — UCB, LinUCB, Track-and-Stop in depth
- [Invariants](./05-invariants.md)
