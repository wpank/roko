# Router Implementations

**Status**: Shipping
**Crate**: `roko-agent`
**Last reviewed**: 2026-04-19

---

## Shipping Implementations

- `StaticRouter` — rule-based; list of `(pattern, action)` pairs.
- `ConfidenceRouter` — scores actions by `score.utility * score.confidence`.
- `UCBRouter` — UCB1 multi-armed bandit.
- `CascadeRouter` — tries Static → Confidence → UCB in order.
- `NoOpRouter` — always returns `Ok(None)`; useful as a stub.

See [Semantics](./02-semantics.md) for full details of each strategy.
