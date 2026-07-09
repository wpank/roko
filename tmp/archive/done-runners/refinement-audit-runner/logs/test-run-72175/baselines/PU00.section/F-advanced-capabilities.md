# F — Advanced Capabilities (Docs 23-29)

Audit-aligned parity notes for `docs/00-architecture/23-architectural-analysis-improvements.md`
through `29-cognitive-energy-model.md`.

This arc should read mostly as planned or deferred architecture. The audit rejected the earlier
posture because it treated speculative chapters as if they were current implementation gaps waiting
for a large enough batch.

In parity terms, docs `23-29` are planning artifacts first and implementation debt only where code
surfaces already exist.

Treat this arc as a planning artifact with explicit deferrals, not as a dependency ordering sheet
for near-term implementation.

---

## Zero-Code Concepts

These concepts have 0 lines of production code and must stay out of present-tense capability
claims:

| Concept | Where it appears | Audit posture |
|---------|------------------|---------------|
| `Pulse` | docs `24`, `26`, `28` | deferred |
| `Datum` | docs `24`, `28` | deferred |
| `Worldview` | learning/goal spillover around doc `28` | deferred |

Related concepts that also remain non-live for this arc:

- `Demurrage`
- `Custody`

If any of these appear in docs `23-29`, label them `planned`, `target-state`, or `deferred`.

## What Can Stay In Present Tense

| Doc | Status | Current truth |
|-----|--------|---------------|
| 23 — Architectural Analysis / Improvements | KEEP + NARROW | the analysis exists, and the `roko-conductor -> roko-learn` violation is a real finding |
| 26 — Cognitive Immune System | NARROW | the repo has a minimal safety foundation today |

The safety foundation that can be stated in present tense is narrow but real:

- attestation
- taint as a minimal provenance flag
- practical policy and guardrail systems

That foundation should not be described as an empty space, but it also should not be inflated into
a fully realized cognitive immune system.

## Planned / Deferred

| Doc | Status | Current truth |
|-----|--------|---------------|
| 24 — Cross-Section Integration Map | REWRITE | useful as a backlog map, not as a live wiring map |
| 25 — Attention as Currency | DEFER | future-state economic model only |
| 26 — Cognitive Immune System | NARROW | everything beyond the minimal safety spine remains planned |
| 27 — Temporal Knowledge Topology | DEFER | documented target-state only |
| 28 — Emergent Goal Structures | DEFER | documented target-state only |
| 29 — Cognitive Energy Model | DEFER | minimal foundation at most; not a live runtime system |

Required language for this arc:

- `documented target-state`
- `minimal or no corresponding production code today`
- `future architecture note`

Avoid phrasing like `0% implemented` or `batch 00 should build this`. That wording turns proposal
material into fake implementation backlog.

## Post-Audit Gap Picture

What this batch should keep:

- doc 23 as a useful architecture analysis
- doc 24 as a dependency and integration planning aid
- doc 26 as a narrow extension of a real safety spine

What this batch should explicitly defer:

- attention-token economy
- temporal knowledge topology
- emergent goal structures
- energy budgeting
- any Pulse / Datum / Worldview present-tense story

That framing keeps the useful analysis while preventing the pack from reading like a hidden
architecture build sheet.

## Batch-00 Boundary

For docs `23-29`, batch `00` should only do three things:

1. preserve the useful analysis,
2. label speculative systems honestly,
3. move future work back into planned/deferred posture instead of implying parity requires those
   systems now.
