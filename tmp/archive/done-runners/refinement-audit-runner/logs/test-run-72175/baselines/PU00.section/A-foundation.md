# A — Foundation (Docs 00-05)

Audit-aligned parity read of `docs/00-architecture/00-vision-and-thesis.md` through
`05-provenance-and-attestation.md`.

Use these facts consistently in this arc:

- 36 workspace members
- 322,088 Rust LOC
- Engram is the live durable kernel noun; only small legacy naming residue remains
- docs `23-35` are proposal posture unless matching code exists

The core correction for docs `00-05` is scope control: keep shipped primitives in present tense,
keep thesis claims qualified, and keep later rewrite ideas out of the "missing backlog" column.

---

## Shipped Today

| Item | Status | Current truth |
|------|--------|---------------|
| 01 — Naming / Glossary | MOSTLY DONE | `Engram` is canonical; old naming is legacy residue, not a live design center |
| 02 — Engram Data Type | SHIPPED | `Engram` is the real durable runtime type |
| 03 — Score: 7-Axis Appraisal | SHIPPED | the score model is implemented and richer than the original doc baseline |
| 04 — Decay Variants | SHIPPED | decay variants are real and testable |
| 05 — Provenance / Attestation | SHIPPED | provenance and attestation exist in code today |

These sections can stay in present tense, but only for the features that actually ship. They
should not borrow authority from later active-inference, custody, or memory-economy proposals.

## Partial / Narrowly True

### 00 — Vision / Thesis

- **Status**: PARTIAL
- **Shipped basis**: the workspace has real scaffold pieces: traits, gates, routing, learning,
  `roko-serve`, and a large TUI surface.
- **Not shipped**: there is no closed, proven loop where repeated failure is automatically
  converted into reliable replanning and compound self-improvement.
- **Required wording**: describe the thesis as directionally grounded in runtime, not as a proven
  self-improving system.

## REF01-03 Audit Verdicts

| Ref | Audit verdict | What parity should say |
|-----|---------------|------------------------|
| `REF01` | `narrow` | "one noun, six verbs" is a diagnostic simplification; today the noun is still `Engram` |
| `REF02` | `defer` | `Pulse` is a target-state idea with 0 production LOC |
| `REF03` | `narrow` | a generic `Bus<E>` trait is a plausible future cleanup, but the live runtime transport is still a narrow `EventBus<E>` with exactly two live RokoEvent variants |

The diagnosis behind REF01-03 is directionally right: naming drift exists, event shapes are messy,
and the runtime bus is not a kernel trait. The prescription was overscoped. Batch `00` should
retell that as a docs-truth problem, not as a demand to ship Pulse or a two-fabric rewrite.

That also means old naming residue is a cleanup task, not an excuse to introduce a second live
kernel noun in parity materials.

## Planned / Deferred Corrections

These items belong to proposal posture and should not be backfilled into docs `00-05` as though
batch `00` merely needs to "finish" them:

| Ref | Proposed change | Audit posture |
|-----|-----------------|---------------|
| `REF01` | make the foundation story cleaner and more explicit | keep as wording cleanup, not as a new runtime model |
| `REF02` | add `Pulse` as an ephemeral sibling to `Engram` | planned / deferred, 0 LOC in current workspace |
| `REF03` | promote `Bus` as a second kernel fabric | planned / narrow; current runtime bus is a small utility, not a full fabric split |
| `REF04` | generalize operators around `Datum` | deferred, 0 LOC in current workspace |
| `REF05` | restate the loop as seven co-equal steps | documentation target only; runtime has not migrated |

## File-Level Corrections

- `00-vision-and-thesis.md`: describe the thesis as directionally grounded. The runtime scaffold
  exists, but the closed self-improvement loop is not yet proven at production scale.
- `01-naming-and-glossary.md`: say `Engram` is canonical. Legacy wording may appear as residue,
  but it must not anchor new parity notes.
- `02-engram-data-type.md`: keep `Engram` as the live durable kernel noun; do not retrofit
  `Pulse` or `Datum` into the same present-tense story.
- `03-score-7-axis-appraisal.md`: keep the score model in present tense; do not connect it to a
  broader attention-token economy or VCG system.
- `04-decay-variants.md`: keep ordinary decay variants in present tense; keep `Demurrage`
  explicitly deferred because it has zero production LOC.
- `05-provenance-and-attestation.md`: keep provenance and attestation in present tense; keep
  custody-chain and immune-system extensions explicitly deferred.

The foundation pass should feel like a narrowing edit, not the first phase of a target-state
kernel migration.

## Editing Bias For This Arc

Prefer:

- `Engram remains the live durable kernel type`
- `directionally grounded thesis`
- `planned generic bus trait`
- `deferred Pulse concept`

Avoid:

- treating Pulse as existing architecture
- treating a two-fabric kernel as current runtime truth
- turning REF01-03 into an implementation backlog for this docs batch
