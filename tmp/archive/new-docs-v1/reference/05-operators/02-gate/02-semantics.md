# Gate Semantics — Pass, Reject, Abstain

> The three-verdict model: what each verdict means, when to use `Abstain` vs `Pass`, and
> the critical distinction between a verdict (filtering decision) and a gate error
> (implementation failure).

**Status**: Shipping
**Crate**: `roko-gate`
**Depends on**: [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## The Three Verdicts

### `Pass`

The gate evaluated the `Engram` and approves it. The gate makes an active positive
statement: "I have checked this input and it is acceptable."

`Pass` is appropriate when:
- The gate evaluated the input and found it satisfactory.
- The gate is not applicable to this input but you want to explicitly affirm it (rare —
  prefer `Abstain` for inapplicability).

### `Reject(reason: String)`

The gate evaluated the `Engram` and rejects it. The `reason` string is logged and included
in the `LoopOutcome::Rejected` result for observability.

`Reject` is appropriate when:
- `score.confidence < threshold`
- The body contains unsafe or prohibited content.
- The `Engram`'s coherence score is below the contradiction threshold.
- Any other specific criterion the gate is designed to enforce.

The loop halts for this `Engram` after the first `Reject`. The reason is preserved.

### `Abstain`

The gate has no opinion. It neither approves nor rejects. The loop skips this gate and
continues to the next.

`Abstain` is appropriate when:
- The gate is domain-specific (e.g., only checks `Kind::Fact`) and the input is a different
  `Kind`.
- The gate lacks sufficient information to render a verdict.
- The gate is a safety gate that is only triggered by specific triggers, and those triggers
  are absent.

**Key design rule**: A gate that cannot evaluate an input MUST return `Abstain`, not `Pass`.
Returning `Pass` without evaluating is a false affirmation. This distinction matters in
safety-critical pipelines.

---

## Verdict vs. Error — The Critical Distinction

A `Verdict` is a **filtering decision** — it is what the gate decided about the input.
A `GateError` is an **implementation failure** — the gate crashed (memory fault, I/O error,
numeric panic).

| Situation | Return |
|---|---|
| Low confidence score | `Ok(Verdict::Reject("confidence below 0.5"))` |
| Prohibited content detected | `Ok(Verdict::Reject("safety: prohibited content"))` |
| Gate cannot evaluate this kind | `Ok(Verdict::Abstain)` |
| Gate code panicked | `Err(GateError::Computation("..."))` |
| Gate I/O failed | `Err(GateError::Computation("..."))` |

The loop treats `Err(GateError)` as `Abstain` by default (configurable to `Reject` for
safety-critical gates).

---

## Pipeline Semantics

In a gate pipeline with `[G1, G2, G3]`:

| G1 | G2 | G3 | Outcome |
|---|---|---|---|
| Pass | Pass | Pass | All pass → Engram proceeds |
| Pass | Reject | — | Rejected at G2; G3 not called |
| Abstain | Pass | Pass | G1 skipped; G2, G3 pass → Engram proceeds |
| Abstain | Abstain | Abstain | All abstain → Engram proceeds (no objection) |
| Pass | Err | Pass | G2 crash → treated as Abstain; G3 called |

---

## See Also

- [Gate Composition](./09-gate-composition.md)
- [Failure Modes](./06-failure-modes.md)
