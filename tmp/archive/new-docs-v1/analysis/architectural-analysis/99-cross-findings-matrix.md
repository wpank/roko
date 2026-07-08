---
title: "Cross-Findings Matrix"
section: analysis
subsection: architectural-analysis
id: aa-99
source: 23-architectural-analysis-improvements.md (all sections)
tags: [matrix, cross-reference, index, findings, improvements]
---

# Cross-Findings Matrix

> **Purpose**: Makes the seven findings, three novel proposals, nine inconsistencies, and eleven improvements searchable and cross-linked. Every row is a finding; every column is a category of consequence.

## Master Findings Index

| ID | File | Title | Source Section | Priority |
|---|---|---|---|---|
| AA-00 | [00-overview.md](./00-overview.md) | Overview & Methodology | §1 Executive Summary | — |
| AA-01 | [01-findings-summary.md](./01-findings-summary.md) | All Findings Summary | §1-10 | — |
| AA-02 | [02-finding-trait-sufficiency.md](./02-finding-trait-sufficiency.md) | Six Traits Are Sufficient | §2 | Low (stable) |
| AA-03 | [03-finding-layer-taxonomy.md](./03-finding-layer-taxonomy.md) | Layer Taxonomy: One Violation, Six Unclassified | §3 | High (fix I1, I2, I3) |
| AA-04 | [04-finding-cognitive-speeds.md](./04-finding-cognitive-speeds.md) | Three Cognitive Speeds: Clean Mapping | §4 | Low (stable) |
| AA-05 | [05-finding-engram-universality.md](./05-finding-engram-universality.md) | Engram Is Genuinely Universal | §5 | Low (stable) |
| AA-06 | [06-finding-crosscut-isolation.md](./06-finding-crosscut-isolation.md) | Cross-Cut Isolation Gaps | §6 | Medium (fix I6, I8) |
| AA-07 | [07-finding-category-theory.md](./07-finding-category-theory.md) | Categorical Composability | §7 | Low (stable, long-term I11) |
| AA-08 | [08-novel-proposals.md](./08-novel-proposals.md) | Three Novel Proposals | §8 | Mixed (see I5, I7, I9) |
| AA-09 | [09-finding-inconsistencies.md](./09-finding-inconsistencies.md) | Documentation Inconsistencies | §9 | High (quick fixes) |
| AA-10 | [10-prioritized-improvements.md](./10-prioritized-improvements.md) | 11 Prioritized Improvements | §10 | All tiers |

---

## Finding × Improvement Matrix

Which findings motivate which improvements:

| Finding | I1 | I2 | I3 | I4 | I5 | I6 | I7 | I8 | I9 | I10 | I11 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| AA-02 Trait Sufficiency | — | — | — | — | — | — | — | — | ✓ | — | — |
| AA-03 Layer Taxonomy | ✓ | ✓ | ✓ | — | — | — | — | — | — | — | — |
| AA-04 Cognitive Speeds | — | — | — | — | — | — | ✓ | — | ✓ | — | — |
| AA-05 Engram Universality | — | — | — | — | — | — | — | — | — | ✓ | — |
| AA-06 Cross-Cut Isolation | ✓ | — | — | — | ✓ | ✓ | — | ✓ | — | — | — |
| AA-07 Category Theory | ✓ | — | — | — | — | — | ✓ | ✓ | — | — | ✓ |
| AA-08 Novel Proposals | — | — | — | — | ✓ | — | ✓ | — | ✓ | — | — |
| AA-09 Inconsistencies | — | ✓ | ✓ | ✓ | — | — | — | — | — | — | — |

---

## Finding × Integration-Map Pairs

Which findings are most relevant to which integration-map pairs:

| Finding | Key Pairs |
|---|---|
| AA-02 Trait Sufficiency | All pairs (trait boundary defines all integrations) |
| AA-03 Layer Taxonomy | [conductor-x-orchestration](../integration-map/conductor-x-orchestration.md), [learning-x-verification](../integration-map/learning-x-verification.md) |
| AA-04 Cognitive Speeds | [orchestration-x-learning](../integration-map/orchestration-x-learning.md), [dreams-x-neuro](../integration-map/dreams-x-neuro.md) |
| AA-05 Engram Universality | [neuro-x-composition](../integration-map/neuro-x-composition.md), [code-intel-x-composition](../integration-map/code-intel-x-composition.md) |
| AA-06 Cross-Cut Isolation | [daimon-x-orchestration](../integration-map/daimon-x-orchestration.md), [daimon-x-composition](../integration-map/daimon-x-composition.md), [dreams-x-neuro](../integration-map/dreams-x-neuro.md), [dreams-x-daimon](../integration-map/dreams-x-daimon.md) |
| AA-07 Category Theory | [learning-x-verification](../integration-map/learning-x-verification.md), [neuro-x-learning](../integration-map/neuro-x-learning.md) |
| AA-08 Novel Proposals | [learning-x-verification](../integration-map/learning-x-verification.md), [neuro-x-composition](../integration-map/neuro-x-composition.md), [learning-x-composition](../integration-map/learning-x-composition.md) |
| AA-09 Inconsistencies | (documentation fixes, minimal integration impact) |

---

## Finding × Readiness-Audit Gaps

Which architectural findings manifest in the readiness audit:

| Finding | Readiness-Audit Manifestation | Audit Gap(s) |
|---|---|---|
| AA-03 Layer Taxonomy | `roko-conductor` listed under unclear layer | G13 (safety wiring links) |
| AA-06 Cross-Cut Isolation | Daimon not injected via trait object | G5, G9, G12 |
| AA-06 Cross-Cut Isolation | Dreams→Neuro not wired | G15, G7 |
| AA-08 Proposal 2 (gradient feedback) | Gate feedback not continuous | G7 (feedback loops) |
| AA-09 Signal→Engram rename | Code-doc terminology divergence | G8 |
| AA-09 Score axis mismatch | Documentation overpromises | (doc fix only) |

---

## Improvement Execution Sequence

Recommended order respecting dependencies:

```
Week 1 (Quick wins):
  I3 (roko-fs label)        — trivial, 30 min
  I4 (Score doc alignment)  — small, 1-2 hr
  I2 (classify 6 crates)    — small, 1 day
  I1 (conductor violation)  — small, 2 days

Week 2-3 (Medium enhancements):
  I6 (AffectModel trait)    — small, 2-3 days; prerequisite for I8
  I5 (gradient gate)        — medium, 1 week; highest ROI
  I7 (Pipeline struct)      — medium, 1 week; depends on AA-07 analysis

Week 4+ (Longer-horizon):
  I8 (arbitration protocol) — medium, 1-2 weeks; depends on I6
  I9 (CompetitiveRouter)    — large; depends on AA-02 understanding
  I10 (HDC on Signal)       — large; research-grade work
  I11 (CT verification)     — large; formal methods work
```

---

## Key Stability Zones

Three areas where the architecture is **stable and needs no changes**:

1. **Trait model** (AA-02): All six traits are correctly scoped. Do not add a seventh.
2. **Engram universality** (AA-05): The universal type handles all current cases. No new types needed.
3. **Category theory structure** (AA-07): The pipeline's compositional properties are sound. Preserve them in all new implementations.

---

## Key Fragility Zones

Three areas where the architecture has confirmed weaknesses:

1. **Layer taxonomy** (AA-03): One violation + six unclassified crates. Fix before scaling.
2. **Cross-cut isolation** (AA-06): Daimon lacks trait-object injection; Dreams directly imports Neuro. Fix before building on these cross-cuts.
3. **Documentation consistency** (AA-09): Score axes, layer labels, and crate counts are inconsistent. Fix before onboarding new contributors.
