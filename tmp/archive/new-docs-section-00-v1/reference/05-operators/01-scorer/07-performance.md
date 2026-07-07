# Scorer Performance

**Status**: Shipping
**Crate**: `roko-core`
**Last reviewed**: 2026-04-19

---

<!-- ADDED -->

## Hot Path

`Scorer::score` is called on every loop tick. At Gamma speed (1–10 ticks/second), this is
~10 calls/second.

| Implementation | Cost per call | Notes |
|---|---|---|
| `ConstantScorer` | < 1 µs | Pure return, no computation |
| `RecencyScorer` | < 1 µs | Timestamp delta only |
| `DefaultScorer` (no substrate) | ~5 µs | Arithmetic on scalar fields |
| `DefaultScorer` (with fingerprint) | ~1 ms | HDC distance to n substrate records |

The fingerprint-dependent scorers are bounded by the substrate scan cost — see
[Substrate Performance](../../03-substrate/13-performance.md).

## Allocation

`Scorer::score` should allocate minimally. `Score` is a plain struct (7 × f32 = 28 bytes);
copying it is cheap. No heap allocation required for simple scorers.

## See Also

- [Substrate Performance](../../03-substrate/13-performance.md)
