# Gate Invariants

**Status**: Shipping
**Crate**: `roko-gate`
**Last reviewed**: 2026-04-19

---

**I1 — No Side Effects**: `evaluate` must not modify the `Engram`, the `Score`, or the
substrate. Gates are pure read operations.

**I2 — Abstain for Inapplicable Input**: A gate that cannot evaluate the input MUST return
`Abstain`, not `Pass`. Never silently approve something you haven't evaluated.

**I3 — Errors Are Not Verdicts**: `Reject` is not an error. `GateError` is for crashes only.
See [Semantics](./02-semantics.md).

**I4 — Determinism**: Given the same `Engram` and `Score`, a gate must return the same
`Verdict` (no hidden mutable state).

**I5 — No Panic**: Gates must return `Err(GateError)` rather than panicking.

---

## See Also

- [Semantics](./02-semantics.md)
- [Failure Modes](./06-failure-modes.md)
