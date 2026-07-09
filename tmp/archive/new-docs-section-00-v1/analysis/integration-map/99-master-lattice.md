---
title: "Master Lattice Index"
section: analysis
subsection: integration-map
id: im-99
source: 24-cross-section-integration-map.md (§2, §6, §9)
tags: [index, matrix, lattice, all-pairs, missing-integrations]
---

# Master Lattice Index

> **Purpose**: Searchable index of all integration pairs and the complete dependency matrix. Use this to find any connection between two sections, see its status, and navigate to the detailed pair file.

## Complete Pair File Index

| Pair File | Sections | M# | Tier | Status |
|---|---|---|---|---|
| [agents-x-composition](./agents-x-composition.md) | 02↔03 | — | — | Wired |
| [agents-x-verification](./agents-x-verification.md) | 02→04 | — | — | Wired + Critical Gap G1 |
| [anti-knowledge-x-composition](./anti-knowledge-x-composition.md) | 06→03 | M15 | 2 | Missing |
| [code-intel-x-composition](./code-intel-x-composition.md) | 15→03 | M8 | 2 | Missing |
| [code-intel-x-verification](./code-intel-x-verification.md) | 15→04 | M16 | 3 | Missing |
| [conductor-x-orchestration](./conductor-x-orchestration.md) | 07↔01 | — | — | Wired |
| [conductor-x-routing](./conductor-x-routing.md) | 07→05 | M9 | 2 | Missing |
| [coordination-x-dreams](./coordination-x-dreams.md) | 13→10 | M19 | 4 | Missing |
| [coordination-x-orchestration](./coordination-x-orchestration.md) | 13→01 | M12 | 4 | Missing |
| [daimon-x-composition](./daimon-x-composition.md) | 09→03 | M2 | 1 | Missing |
| [daimon-x-learning](./daimon-x-learning.md) | 09→05 | — | — | Wired |
| [daimon-x-orchestration](./daimon-x-orchestration.md) | 09→01 | M1 | 1 | Missing |
| [dreams-x-daimon](./dreams-x-daimon.md) | 10→09 | M18 | 3 | Missing |
| [dreams-x-neuro](./dreams-x-neuro.md) | 10→06 | M7 | 2 | Missing |
| [learning-x-composition](./learning-x-composition.md) | 05→03 | M4 | 1 | Missing |
| [learning-x-config](./learning-x-config.md) | 05→00 | M10 | 2 | Missing |
| [learning-x-routing](./learning-x-routing.md) | 05→05 | M6 | 1 | Missing |
| [learning-x-verification](./learning-x-verification.md) | 04→05 | — | — | Wired |
| [lifecycle-x-neuro](./lifecycle-x-neuro.md) | 17→06 | M20 | 3 | Missing |
| [neuro-x-composition](./neuro-x-composition.md) | 06→03 | M5 | 2 | Missing |
| [neuro-x-learning](./neuro-x-learning.md) | 05↔06 | — | — | Partial |
| [neuro-x-verification](./neuro-x-verification.md) | 06→04 | M14 | 3 | Missing |
| [orchestration-x-daimon](./orchestration-x-daimon.md) | 01→09 | M11 | 2 | Missing |
| [orchestration-x-learning](./orchestration-x-learning.md) | 01↔05 | — | — | Wired |
| [safety-x-agents](./safety-x-agents.md) | 11↔02 | — | — | Built/Not-Wired |
| [safety-x-composition](./safety-x-composition.md) | 11→03 | M13 | 3 | Missing |
| [tech-analysis-x-heartbeat](./tech-analysis-x-heartbeat.md) | 20→16 | M17 | 4 | Missing |
| [verification-x-orchestration](./verification-x-orchestration.md) | 04→01 | M3 | 1 | Missing |

---

## 20 Missing Integrations — Quick Reference

| M# | Name | Pair File | LOC | Tier | Blocking |
|---|---|---|---|---|---|
| M1 | Daimon → Orchestration | [daimon-x-orchestration](./daimon-x-orchestration.md) | ~60 | 1 | Needs AffectModel trait (I6) |
| M2 | Daimon → Composition | [daimon-x-composition](./daimon-x-composition.md) | ~45 | 1 | Needs AffectModel trait (I6) |
| M3 | Failure → Replanning | [verification-x-orchestration](./verification-x-orchestration.md) | ~80 | 1 | None |
| M4 | Skills → Prompts | [learning-x-composition](./learning-x-composition.md) | ~55 | 1 | None |
| M5 | Neuro → Composition (full) | [neuro-x-composition](./neuro-x-composition.md) | ~90 | 2 | Knowledge types exist |
| M6 | Cost → Routing | [learning-x-routing](./learning-x-routing.md) | ~70 | 1 | None |
| M7 | Dreams → Neuro | [dreams-x-neuro](./dreams-x-neuro.md) | ~200 | 2 | Dream projection design (§7.3) |
| M8 | Code Intel → Composition | [code-intel-x-composition](./code-intel-x-composition.md) | ~120 | 2 | roko-index G2, G3 |
| M9 | Conductor → Routing | [conductor-x-routing](./conductor-x-routing.md) | ~45 | 2 | None |
| M10 | Experiments → Static | [learning-x-config](./learning-x-config.md) | ~90 | 2 | None |
| M11 | Orchestration → Daimon | [orchestration-x-daimon](./orchestration-x-daimon.md) | ~40 | 2 | Needs M1 first |
| M12 | Pheromones → Orchestration | [coordination-x-orchestration](./coordination-x-orchestration.md) | ~150 | 4 | Agent Mesh (G26) |
| M13 | Safety → Composition | [safety-x-composition](./safety-x-composition.md) | ~80 | 3 | None (code exists) |
| M14 | Neuro → Gate Thresholds | [neuro-x-verification](./neuro-x-verification.md) | ~60 | 3 | Knowledge types exist |
| M15 | AntiKnowledge → Composition | [anti-knowledge-x-composition](./anti-knowledge-x-composition.md) | ~35 | 2 | Part of M5 |
| M16 | Code Intel → Verification | [code-intel-x-verification](./code-intel-x-verification.md) | ~100 | 3 | Needs M8 first |
| M17 | Tech Analysis → Heartbeat | [tech-analysis-x-heartbeat](./tech-analysis-x-heartbeat.md) | ~150 | 4 | Oracle trait (G21) |
| M18 | Dreams → Daimon | [dreams-x-daimon](./dreams-x-daimon.md) | ~80 | 3 | Needs M7 first |
| M19 | Coordination → Dreams | [coordination-x-dreams](./coordination-x-dreams.md) | ~120 | 4 | Needs M7, M12 |
| M20 | Lifecycle → Neuro (restore) | [lifecycle-x-neuro](./lifecycle-x-neuro.md) | ~100 | 3 | None |

**Grand total**: ~2,070 LOC

---

## Dependency Graph for M-Items

```
Tier 1 (no dependencies):
  M3, M4, M6  — standalone; implement immediately

  M1, M2      — depend on AffectModel trait (I6, ~2-3 days)

Tier 2 (depends on Tier 1):
  M5, M15     — knowledge types exist; M15 is subset of M5
  M8          — requires G2/G3 (roko-index wiring)
  M9          — standalone
  M10         — standalone
  M11         — implement after M1

  Event Bus (~200 LOC) — enables all remaining M-items at ~20-40 LOC each

Tier 3 (depends on Tier 2):
  M7          — requires Dream projection design
  M13, M14    — standalone code exists
  M16         — implement after M8
  M18         — implement after M7
  M20         — standalone

Tier 4 (Phase 2+):
  M12         — requires Agent Mesh (G26)
  M17         — requires Oracle trait (G21) and Heartbeat (G24)
  M19         — requires M7 and M12
```

---

## Full Dependency Matrix (Extract — M-cells only)

From the source file 24 full matrix, the M (missing) cells:

```
FROM \ TO    00   01   03   04   05   06   09   10   16
01 Orch                            M         M11  M
02 Agents    (G1 safety gap)
03 Comp           M
04 Verif     M3
05 Learn          M4   M6   M10
06 Neuro     M    M5   M14
07 Cond      M9
09 Daimon    M1   M2
10 Dreams    M7             M18
11 Safety    M13
13 Coord     M12            M19
15 Code      M8   M16
17 Life                          M20
20 TechA          M    M   M17
```

---

## Sections with No Expected Dependencies

Not all 462 section pairs should connect. Confirmed independent pairs:

| A | B | Reason |
|---|---|---|
| 08-Chain | 15-Code Intel | Domain plugin vs. scaffold service; no shared data |
| 10-Dreams | 18-Tools | Dreams operates on episodes, not tools |
| 19-Deployment | 09-Daimon | Infrastructure vs. cognitive state |
| 19-Deployment | 10-Dreams | Deployment doesn't affect consolidation |
| 21-References | All | Documentation only |
| 14-Economy | 03-Composition | Economic primitives don't affect prompt assembly |
| 17-Lifecycle | 04-Verification | Lifecycle ops don't use gates |

~180 expected zero-dependency pairs (39% of 462-pair space).

---

## Cross-References

- Detailed overview: [00-overview.md](./00-overview.md)
- Architectural findings: [../architectural-analysis/99-cross-findings-matrix.md](../architectural-analysis/99-cross-findings-matrix.md)
- Readiness gaps: [../readiness-audit/99-next-actions.md](../readiness-audit/99-next-actions.md)
