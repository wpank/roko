# Exponential Decay Approaches Zero

> An Engram with exponential decay has value approaching 0 as time approaches infinity.

**Crate**: `roko-core`
**Test type**: Unit test
**Enforcement**: `DecayExponential::value_at`
**Last reviewed**: 2026-04-19

---

## Statement

For all initial values v₀ > 0 and all decay rates λ > 0:
`lim(t→∞) v₀ × e^(-λt) = 0`

Practically tested: at t = 10 × half-life, value < 0.001 × v₀.

---

## See also

- [decay-monotonicity.md](decay-monotonicity.md)
- [../by-subsystem/subsystem-core.md](../by-subsystem/subsystem-core.md)
