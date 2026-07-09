# D — Architecture Layers (Docs 12-17)

Post-audit parity notes for `docs/00-architecture/12-five-layer-taxonomy.md` through
`17-design-principles-and-frontier-summary.md`.

This arc needs factual correction more than conceptual invention. The previous pass blurred current
workspace reality with proposed crate boundaries and repeated stale status claims about serve and
the TUI.

---

## Baseline Corrections

These are settled facts for topic `00`:

- 36 workspace members
- 322,088 Rust LOC
- `roko-serve` is wired with 200+ routes
- the TUI is wired and substantial at roughly 58K LOC

Any parity wording that still describes serve or the TUI as unwired is wrong.

## What Can Stay Grounded

| Doc | Status | Current truth |
|-----|--------|---------------|
| `12-five-layer-taxonomy.md` | `keep` + `narrow` | the layer model is a useful organizing discipline, even if not perfectly enforced |
| `13-cognitive-cross-cuts.md` | `keep` + `narrow` | Neuro and Daimon are real subsystem surfaces; Dreams is mixed live plus aspirational |
| `14-c-factor-collective-intelligence.md` | `keep` + `narrow` | C-Factor has enough real surface to stay in present tense, but not enough to support every theory-heavy claim |
| `15-crate-map.md` | `rewrite` | the live workspace is large and real; proposed crate boundaries remain target-state |
| `16-autocatalytic-and-cybernetics.md` | `narrow` | some compounding loops are grounded, but the stronger self-reinforcing story is only partially closed |
| `17-design-principles-and-frontier-summary.md` | `narrow` | the principles are useful; frontier and moat-style claims still need proof discipline |

## Confirmed Architecture Debt

The parity pack should keep the real findings small and specific:

- `roko-conductor -> roko-learn` is a confirmed layer violation.
- the layer model is useful, but not a perfectly enforced law.
- proposed crate splits such as `roko-bus`, `roko-hdc`, and `roko-spi` remain proposals.

## Rewrite Bias For Docs 12-17

Prefer:

- `useful model`
- `confirmed violation`
- `proposed boundary`
- `partially live compounding loop`

Avoid:

- treating serve or the TUI as scaffolds
- `current crate split` when the split is still proposed
- `frontier innovation already demonstrated everywhere`

## Batch-00 Boundary

For docs `12-17`, the parity refresh should:

1. correct the serve/TUI status immediately,
2. separate current workspace reality from planned crate seams,
3. keep theory-heavy sections grounded in the parts that actually ship.
