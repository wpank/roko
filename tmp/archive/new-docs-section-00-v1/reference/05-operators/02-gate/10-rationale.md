# Gate Rationale

**Status**: Shipping
**Crate**: `roko-gate`
**Last reviewed**: 2026-04-19

---

## Why Separate Gate from Scorer?

Scoring assigns numeric values; gating makes a binary decision based on those values.
Keeping them separate means:
- Scoring thresholds can change without touching scoring code.
- The same score can drive different gate decisions in different deployment contexts.
- Multiple gates can apply different criteria to the same score.

## Why Three Verdicts, Not Two?

`Pass` and `Reject` are obvious. `Abstain` is less so — why not just `Pass` for "no opinion"?

Because `Pass` is an affirmative statement: "I have checked and approve." A gate that has not
checked is not in a position to affirm. `Abstain` is semantically honest: "I have no opinion."
This matters in safety-critical pipelines where a gate crash should not silently approve
dangerous inputs.

## Why Is `GateError` Not a Verdict Variant?

An error is not a decision. Conflating "I crashed" with "I reject" or "I pass" would make
error handling invisible — calling code could not distinguish between a deliberate rejection
and a gate failure. Keeping errors in `Err(GateError)` forces callers to handle them
explicitly.

## Open Questions

- Should `Verdict::Reject` carry structured metadata (a `RejectReason` enum) instead of a
  free-form string, to enable programmatic rejection analysis?
