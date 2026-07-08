# Router Invariants

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

**I1**: `Ok(None)` is not an error — it means "no applicable strategy."
**I2**: `Err(RouterError)` is a computation failure only.
**I3**: The same `(Engram, Score)` input to a deterministic router (Static, Confidence) must produce the same action.
**I4**: `UCBRouter` is non-deterministic only in the exploration phase; it must be seeded deterministically for reproducible tests.
**I5**: No side effects on Substrate or Bus.
