# F — Advanced Capabilities (Docs 23-29)

Post-audit parity notes for `docs/00-architecture/23-architectural-analysis-improvements.md`
through `29-cognitive-energy-model.md`.

This arc is mostly future architecture. The audit rejected the earlier posture because it treated
research-heavy chapters as if parity merely required a larger implementation batch. Topic `00`
should preserve the useful analysis while making the deferrals explicit.

---

## Explicitly Deferred Zero-Code Concepts

These concepts must stay out of present-tense claims:

| Concept | Status | Why |
|---------|--------|-----|
| `Pulse` | `deferred` | 0 production LOC |
| `Datum` | `deferred` | 0 production LOC |
| `Worldview` | `deferred` | 0 production LOC |

Related concepts that also remain deferred in this arc:

- `Demurrage`
- `Custody`

If any of these appear in docs `23-29`, label them `planned`, `target-state`, or `deferred`.

## What Can Stay Useful

| Doc | Status | Current truth |
|-----|--------|---------------|
| `23-architectural-analysis-improvements.md` | `keep` + `narrow` | useful architecture analysis, including the real `roko-conductor -> roko-learn` violation |
| `24-cross-section-integration-map.md` | `rewrite` | useful dependency and backlog map, not a live wiring map |
| `26-cognitive-immune-system.md` | `narrow` | a minimal safety foundation exists today, but the larger immune-system stack is still planned |

## What Must Stay Planned Or Deferred

| Doc | Status | Current truth |
|-----|--------|---------------|
| `25-attention-as-currency.md` | `defer` | future economic model only |
| `27-temporal-knowledge-topology.md` | `defer` | architecture design material only |
| `28-emergent-goal-structures.md` | `defer` | architecture design material only |
| `29-cognitive-energy-model.md` | `defer` | design reference, not a live runtime system |

## Rewrite Bias For Docs 23-29

Prefer:

- `documented target-state`
- `minimal or no corresponding production code today`
- `future architecture note`
- `planning artifact`

Avoid:

- `batch 00 should build this`
- `0% implemented therefore immediate parity debt`
- any present-tense `Pulse`, `Datum`, or `Worldview` story

## Batch-00 Boundary

For docs `23-29`, parity work is limited to:

1. preserving useful analysis,
2. labeling speculative systems honestly,
3. moving future work back into explicit deferred posture.
