# Composer Invariants

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

**I1**: `system_prompt` must be valid UTF-8.
**I2**: `estimated_tokens` must be ≥ actual token count.
**I3**: `included_engrams` must be a subset of `ctx.recalled`.
**I4**: `Err(ContextWindowExceeded)` on overflow — never silently truncate.
**I5**: No side effects on Substrate.
