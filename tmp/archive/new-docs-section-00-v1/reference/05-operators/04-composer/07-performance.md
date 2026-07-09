# Composer Performance

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

<!-- ADDED -->

## Cost

`compose` is called once per loop tick. Primary cost is string allocation and concatenation.

| Operation | Cost |
|---|---|
| Layer rendering | O(tokens) — typically < 1 ms |
| Token estimation | O(chars / 4) — < 0.1 ms |

Total: < 2 ms per compose call at typical prompt sizes (< 32,000 tokens).
