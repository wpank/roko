# G — Innovation & Meta (Docs 30-35)

Audit-aligned parity notes for `docs/00-architecture/30-cross-pollination-innovations.md` through
`35-consolidated-roadmap.md`.

This file is the honesty layer for topic `00`: keep what is useful as planning material, but stop
letting research, synergy, and roadmap docs read like proof that the current architecture already
delivers every named composition.

---

## Current-State Facts Worth Keeping

| Item | Status | Current truth |
|------|--------|---------------|
| Real test estate | SHIPPED | the audit baseline is 3,761 tests |
| 31 — Implementation Readiness Audit | KEEP + NARROW | useful as a spec-quality audit, not as a runtime status dashboard |

The honest current-state summary is:

- testing is substantial
- coverage quality is uneven
- some infrastructure gaps still matter
- `lack of testing` is not the honest problem statement

## Planning / Research Artifacts

| Doc | Status | Current truth |
|-----|--------|---------------|
| 30 — Cross-Pollination Innovations | DEFER | research backlog, not a current advantage |
| 32 — Comprehensive Test Strategy | REWRITE | test-hardening roadmap, not proof that testing is absent |
| 33 — Refactor Plan Phases | REWRITE | historical and phased planning material, not active truth of the current repo |
| 34 — Synergy & Integration Map | REWRITE | aspirational fiction unless explicitly labeled otherwise |
| 35 — Consolidated Roadmap | REWRITE + DEFER | useful dependency ordering only; staffing and quarterly execution plan are overscoped |

Docs `31-35` still have value when read as planning material, research synthesis, backlog
structuring, and dependency-order references. That framing should remain explicit.

The practical batch-00 rewrite is to turn these chapters into planning artifacts with dependency
ordering, not to defend them as current moat proof.

## Required Honesty Corrections

- Cross-pollination and moat claims should be labeled aspirational compositions, not current
  advantages.
- The synergy matrix should be described plainly as aspirational fiction in its current form:
  five named primitives are absent in production code (`Pulse`, `Datum`, `Demurrage`,
  `Worldview`, `Custody`), and the generic bus trait plus operator generalization remain
  proposal-level abstractions.
- The long annual roadmap should be labeled overscoped for the current single-developer-plus-agents
  setup and best read as dependency ordering plus future optionality.
- Testing language should acknowledge the real 3,761-test estate before switching into planning
  artifact or hardening-roadmap posture.

This arc should not imply that batch `00` owes a multi-quarter execution program. That is the
backlog inflation pattern the audit rejected.

## Wording Rules

Prefer:

- `planning artifact`
- `research backlog`
- `useful dependency ordering`
- `selective testing gaps`
- `aspirational composition`
- `single-developer-plus-agents calibration`

Avoid:

- `already-proven moat`
- `synergy is already load-bearing`
- `testing barely exists`
- `live 5-7 engineer execution plan`
