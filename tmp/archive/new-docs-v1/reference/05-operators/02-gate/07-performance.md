# Gate Performance

**Status**: Shipping
**Crate**: `roko-gate`
**Last reviewed**: 2026-04-19

---

<!-- ADDED -->

## Hot Path

`Gate::evaluate` is called once per gate per loop tick. At Gamma speed with a 7-rung
pipeline, that is 7 gate calls per tick.

| Gate | Cost per call | Notes |
|---|---|---|
| `PassAllGate` / `RejectAllGate` | < 1 µs | No computation |
| `ConfidenceGate` | < 1 µs | Single float comparison |
| `CoherenceGate` | < 1 µs | Single float comparison |
| `SafetyGate` (regex) | ~10–100 µs | Regex engine; precompile patterns at construction |

## Key Optimisation

Gates are short-circuit: the first `Reject` stops the pipeline. Put the cheapest and most-
likely-to-reject gates first (e.g., `ConfidenceGate` before `SafetyGate`).

## See Also

- [Gate Composition](./09-gate-composition.md)
