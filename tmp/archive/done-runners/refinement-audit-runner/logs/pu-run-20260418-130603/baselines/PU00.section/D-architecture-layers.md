# D — Architecture Layers (Docs 12-17)

Audit-aligned parity notes for `docs/00-architecture/12-five-layer-taxonomy.md` through
`17-design-principles-and-frontier-summary.md`.

This arc only reads cleanly if it separates three things that were previously blurred together:
current workspace reality, useful architectural reading models, and proposed future boundaries.

---

## Baseline Corrections

These are settled facts, not open questions:

- 36 workspace members
- 322,088 Rust LOC
- `roko-serve` is wired with 200+ routes
- the TUI is wired and substantial at roughly 58K LOC

Any parity wording that still treats serve or the TUI as unwired is factually wrong and should be
removed.

This arc is mostly fact correction and scope control, not a crate-topology migration plan.

## Shipped Today

| Doc | Status | Current truth |
|-----|--------|---------------|
| 12 — Five-Layer Taxonomy | KEEP + NARROW | the layer model is a useful organizing discipline |
| 13 — Cognitive Cross-Cuts | KEEP + NARROW | Neuro and Daimon are real crates; Dreams is a real crate with mixed live vs aspirational modes |
| 14 — C-Factor | KEEP + NARROW | C-Factor has enough real surface to stay in current-state docs, but its stronger theory language still needs discipline |

Keep these sections in current-state posture, but keep the claims narrow:

- the five layers are a useful model, not a perfectly enforced law
- Dreams should be described as a real subsystem surface with partial live behavior, not as a
  fully realized cross-cut
- C-Factor should stand on its shipped metric and current policy-adjacent wiring, not on planned
  bus or HDC-heavy futures

## Partial / Narrowly True

| Doc | Status | Current truth |
|-----|--------|---------------|
| 15 — Crate Map | REWRITE | the workspace is large and real, but proposed crate boundaries are still proposals |
| 16 — Autocatalytic / Cybernetics | NARROW | some loops are real, but the stronger compounding story is not fully closed in runtime |
| 17 — Design Principles / Frontier Summary | NARROW | the principles are useful, but frontier-composition claims need proof discipline |

The one confirmed layering violation worth naming explicitly is `roko-conductor -> roko-learn`.
That is a real architecture finding. It is not evidence that the whole layer model collapsed.

For doc 16, keep only the loops that are actually grounded today:

- gating feedback
- learning records
- strategy and knowledge reuse
- C-factor-linked adaptation

Describe the stronger self-reinforcing story as partially live and directionally correct, not as
already proven.

## Planned / Proposed Boundaries

These remain useful to document, but only as proposals:

- `roko-bus`
- `roko-hdc`
- `roko-spi`

The same rule applies to any clean future dependency graph language: keep it as target-state
architecture, not as a description of the current crate graph.

This is where backlog inflation happened before. Proposed boundaries are not "missing migrations"
that batch `00` is expected to complete.

The parity refresh should therefore separate current workspace reality from proposed package seams
without turning those proposals into implicit execution commitments.

## Wording Rules

Prefer:

- `useful model`
- `mostly enforced`
- `confirmed violation`
- `proposed boundary`
- `partially live compounding loop`

Avoid:

- `perfectly enforced`
- `serve is not wired`
- `TUI is not wired`
- `current crate split` when the split is still proposed
- `frontier innovation already demonstrated everywhere`
