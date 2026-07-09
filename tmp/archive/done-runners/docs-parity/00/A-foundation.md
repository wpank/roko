# A — Foundation (Docs 00-05)

Post-audit parity notes for `docs/00-architecture/00-vision-and-thesis.md` through
`05-provenance-and-attestation.md`.

The foundation arc stays mostly intact. The audit correction is scope: keep the live durable
kernel story centered on `Engram`, keep thesis language grounded, and stop treating later medium
and transport proposals as if they already landed.

---

## Baseline Facts For This Arc

- `Engram` is the live durable kernel noun.
- The legacy durable-medium naming drift is effectively resolved; parity wording should stay
  Engram-centered.
- `Pulse`, `Datum`, `Demurrage`, `Worldview`, and `Custody` have 0 production LOC.
- The live runtime bus is still a generic `EventBus<E>` with exactly two live `RokoEvent`
  variants: `PlanRevision` and `PrdPublished`.

## Audit Verdicts To Carry Forward

| Ref | Verdict | Parity update |
|-----|---------|---------------|
| `REF01` | `keep` + `narrow` | keep the cleaner foundation framing, but describe it as an Engram-centered architecture with naming cleanup still in progress |
| `REF02` | `defer` | `Pulse` is a target-state concept only; do not describe it as a live sibling medium |
| `REF03` | `narrow` | a generic `Bus<E>` trait is a plausible future cleanup, but the shipped runtime transport is still `EventBus<E>` |

The diagnosis behind REF01-03 is still useful: naming drift exists, runtime event shapes drifted,
and the transport abstraction is thinner than the docs implied. The prescription was overscoped.
Batch `00` should describe the current architecture more honestly, not stage a kernel rewrite.

## What Can Stay In Present Tense

| Doc | Status | Current truth |
|-----|--------|---------------|
| `00-vision-and-thesis.md` | `partial` | the thesis is directionally grounded by real runtime surfaces, but not yet proven as a closed self-improvement loop |
| `01-naming-and-glossary.md` | `keep` | `Engram` is canonical and should anchor new parity wording |
| `02-engram-data-type.md` | `keep` | `Engram` is the real durable runtime type |
| `03-score-7-axis-appraisal.md` | `keep` | the score model is live and can be described directly |
| `04-decay-variants.md` | `keep` + `narrow` | ordinary decay behavior is live; the economic demurrage story is not |
| `05-provenance-and-attestation.md` | `keep` + `narrow` | provenance and attestation are real; custody-chain extensions remain planned |

## What Must Be Marked Planned Or Deferred

- `Pulse` stays `planned` or `deferred`, never present tense.
- `Bus` stays `planned generic bus trait`, not `current transport fabric`.
- `Demurrage` stays `target-state durable-memory model`, not current runtime language.
- `Custody` stays `planned`, not part of the shipped provenance surface.

## File-Level Rewrite Guidance

- `00-vision-and-thesis.md`: keep the thesis, but phrase it as directionally supported by a large
  real workspace rather than as a proven autonomous loop.
- `01-naming-and-glossary.md`: keep parity notes Engram-centered and treat legacy naming as
  cleanup residue, not as a new architecture noun.
- `02-engram-data-type.md`: keep the durable-medium story about `Engram` only.
- `03-score-7-axis-appraisal.md`: avoid tying the score model to later attention-economy claims.
- `04-decay-variants.md`: split live decay mechanics from deferred demurrage language.
- `05-provenance-and-attestation.md`: keep present-tense attestation and taint, defer custody and
  larger immune-system overlays.

## Batch-00 Boundary

For docs `00-05`, the right outcome is:

1. tighten wording around what already ships,
2. move zero-code concepts back into explicit future-work language,
3. replace leftover legacy naming with Engram-centered wording.
