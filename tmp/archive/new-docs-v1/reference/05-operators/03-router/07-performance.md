# Router Performance

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

<!-- ADDED -->

| Router | Cost per call |
|---|---|
| `StaticRouter` | O(rules) — typically < 1 µs |
| `ConfidenceRouter` | O(actions) — < 1 µs |
| `UCBRouter` | O(arms) — < 1 µs |
| `CascadeRouter` | Sum of tried strategies |

All router calls are sub-microsecond at typical scale (< 20 rules, < 10 arms).
