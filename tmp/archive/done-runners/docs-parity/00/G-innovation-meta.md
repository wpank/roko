# G — Innovation & Meta (Docs 30-35)

Post-audit parity notes for `docs/00-architecture/30-cross-pollination-innovations.md` through
`35-consolidated-roadmap.md`.

This section is where the overscope problem was most obvious. The useful material is still worth
keeping, but it has to read as planning, research synthesis, and dependency ordering rather than
as proof that the current architecture already delivers every named composition.

---

## Grounded Facts Worth Keeping

- the audit baseline includes 3,761 test functions
- `31-implementation-readiness-audit.md` is useful as a spec-quality audit
- the repo already has meaningful safety, learning, serve, and TUI surfaces

Those facts matter because the honest problem is selective gap and posture drift, not architectural
emptiness.

## Required Rewrite Posture

| Doc | Status | Current truth |
|-----|--------|---------------|
| `30-cross-pollination-innovations.md` | `defer` | research backlog, not a current advantage |
| `31-implementation-readiness-audit.md` | `keep` + `narrow` | useful audit input, not a live runtime status dashboard |
| `32-comprehensive-test-strategy.md` | `rewrite` | test-hardening roadmap, not evidence that testing barely exists |
| `33-refactor-plan-phases.md` | `rewrite` | phased planning material, not a statement of present architecture truth |
| `34-synergy-integration-map.md` | `rewrite` | **aspirational fiction** unless explicitly labeled as design-only |
| `35-consolidated-roadmap.md` | `rewrite` + `defer` | useful dependency ordering only; quarterly staffing posture is overscoped |

## Honesty Corrections That Must Stay Explicit

- the synergy matrix is `aspirational fiction` in its current form because most of its claimed
  load-bearing primitives are not implemented
- moat-style language should be rewritten as aspirational composition, not current advantage
- roadmap language should be calibrated for a `single-developer-plus-agents` setup, not preserved
  as a live 5-7 engineer plan
- testing language should acknowledge the substantial current test estate before describing future
  hardening work

## Rewrite Bias For Docs 30-35

Prefer:

- `planning artifact`
- `research backlog`
- `dependency ordering`
- `aspirational composition`
- `single-developer-plus-agents calibration`

Avoid:

- `already-proven moat`
- `synergy is already load-bearing`
- `testing barely exists`
- `active 5-7 engineer execution plan`

## Batch-00 Boundary

For docs `30-35`, parity work is:

1. keep the useful planning and audit material,
2. mark the synergy matrix as aspirational fiction,
3. reduce the roadmap to dependency ordering and future optionality.
