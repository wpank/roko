# Gate Failure Modes

> Gate failure is NOT a verdict. This page covers cases where the gate implementation
> itself fails (crashes, errors) — distinct from the gate rejecting an input.

**Status**: Shipping
**Crate**: `roko-gate`
**Last reviewed**: 2026-04-19

---

<!-- ADDED -->

## F1 — Gate Computation Error

**Scenario**: The gate's internal computation fails (regex engine error, I/O, numeric
overflow).

**Behaviour**: The gate returns `Err(GateError::Computation("..."))`.

**Recovery**:
- Default loop policy: treat as `Abstain` (log warning, continue pipeline).
- Safety override: configure `gate_error_policy = Reject` for safety-critical gates. With
  this policy, a crashing safety gate rejects the engram rather than silently passing it.

---

## F2 — Gate Panic

**Scenario**: The gate implementation panics (should never happen if I5 is upheld).

**Behaviour**: The loop's `std::panic::catch_unwind` wrapper catches the panic, logs it, and
treats the gate as `Err(GateError::Computation("panic"))`.

**Recovery**: Fix the gate implementation. Panics are bugs.

---

## F3 — All Gates Abstain

**Scenario**: Every gate in the pipeline returns `Abstain`.

**Behaviour**: The loop treats an all-abstain result as `Pass` — no gate objected, so the
Engram proceeds.

This is intentional: the absence of a veto is not a rejection. If you want a mandatory
check, use a gate that only returns `Pass` or `Reject` (never `Abstain`).

---

## See Also

- [Semantics](./02-semantics.md) — the Verdict vs. Error distinction
- [Invariants](./05-invariants.md)
