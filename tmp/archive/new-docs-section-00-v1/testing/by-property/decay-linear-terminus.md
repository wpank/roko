# Linear Decay Reaches Zero at Lifetime

> An Engram with linear decay reaches exactly value 0 at its configured lifetime.

**Crate**: `roko-core`
**Test type**: Unit test
**Enforcement**: `DecayLinear::value_at`
**Last reviewed**: 2026-04-19

---

## Statement

For all initial values v₀ > 0 and lifetimes L > 0:
`DecayLinear(L).value_at(v₀, L) == 0.0`

And for t > L: `value_at(v₀, t) == 0.0` (clamped, not negative).

---

## See also

- [decay-monotonicity.md](decay-monotonicity.md)
