# Router Rationale

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## Why a Cascade?

Static rules handle known patterns efficiently. Confidence routing handles cases where rules don't apply but the score signal is strong. UCB handles novel situations by exploring. The cascade degrades gracefully: rules first, exploration last.

## Why `Ok(None)` vs a Default Action?

Returning `None` instead of a default action makes the cascade composition explicit. The caller (CascadeRouter or the loop) decides the fallback, not the individual strategy. This keeps strategies composable.

## Why UCB and Not ε-Greedy?

UCB has logarithmic regret bounds (proven optimal for i.i.d. rewards), while ε-greedy is heuristic. UCB also has no hyperparameter to tune beyond `C`.

## Open Questions

- Should `LinUCB` ship before `UCBRouter` is widely used, to avoid regressing on context-dependent routing?
