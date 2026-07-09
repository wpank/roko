# C — Cognitive Loop (Docs 09-11)

Audit-aligned parity read of `docs/00-architecture/09-universal-cognitive-loop.md` through
`11-dual-process-and-active-inference.md`.

The correction here is not that the loop story is wrong. It is that the docs overstate how
completely the current runtime has converged on that story.

---

## Shipped Today

| Item | Status | Current truth |
|------|--------|---------------|
| 10 — Three Cognitive Speeds | MOSTLY SHIPPED | operating-frequency scheduling exists and is used |

This can stay in present tense, but only at the scheduler level. It should not imply that the
intended T0/T1/T2 traffic mix is already proven in production.

## Partial / Constrained Reality

| Item | Status | Current truth |
|------|--------|---------------|
| 09 — Universal Cognitive Loop | PARTIAL | `loop_tick()` exists, but major runtime ownership still lives in open-coded orchestrator paths |
| 11 — Dual Process / Active Inference | PARTIAL / overstated | tier types and active-inference math exist, but live routing still does not match the doc's clean EFE-driven story |

Key factual corrections to keep explicit:

- `roko-serve` is wired with 200+ routes.
- The TUI is wired and substantial at roughly 58K LOC.
- Because those interfaces are live, the loop cannot be described as the sole owner of production
  control flow when major orchestration still sits outside the shared loop path.

The right wording is `real abstraction, incomplete ownership`, not `missing loop implementation`.

## REF05 Audit Verdict

`REF05` should be treated as **narrowed documentation posture**.

- The seven-step loop retelling is a **target narrative**.
- Co-equal `PERSIST` and `BROADCAST` wording is not current runtime truth.
- Any future migration would have to preserve the existing serve, CLI, and TUI control flows.
- Batch `00` only needs to label the stronger loop story as proposed migration language.
- The live bus still exposes exactly two live `RokoEvent` variants, which is another sign that the
  fuller REF05 transport story remains target-state rather than current runtime fact.

## Labeling Rule

When parity notes reference the REF05 retelling, the sentence should carry one of these qualifiers:

- `target narrative`
- `documentation target`
- `proposed migration`

It should **not** say "the universal loop" without qualification, because:

- `roko-serve` owns large control-flow paths today,
- the TUI owns large control-flow paths today,
- and those paths would need to survive any real loop migration.

## Doc File Coverage

| Doc file | Post-audit read |
|----------|-----------------|
| `09-universal-cognitive-loop.md` | real abstraction, incomplete runtime ownership |
| `10-three-cognitive-speeds.md` | Gamma / Theta / Delta are live; traffic-mix claims need narrower wording |
| `11-dual-process-and-active-inference.md` | tier types and EFE math exist; live routing is partially diverged from the clean doc story |

The right parity outcome here is modest: confirm the shared abstractions, then keep the stronger
runtime-convergence story labeled as a proposed migration.

## Editing Bias For This Arc

Prefer:

- `shared loop helper`
- `partial runtime ownership`
- `target narrative`
- `proposed migration`

Avoid:

- `universal owner of production control flow`
- `complete active-inference runtime`
- `REF05 already landed`
