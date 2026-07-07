# Migration Log — Cluster F: Analysis Tree

**Cluster**: F  
**Migration date**: 2026-04-19  
**Author**: Automated refactor (session)  
**Status**: Complete

---

## Summary

Cluster F refactored four large source markdown files — totalling approximately 135 KB of dense
analysis material — into **82 target files** across four subtrees under `analysis/`, plus this
migration log. The governing principle throughout was **NOTHING IS LOST**: every finding, every
integration edge, every audit gap, and every synergy from the source files has a named home in
the target tree.

The `analysis/` tree is meta-documentation **about** the architecture, not part of it. These
files do not define types, specify traits, or describe subsystem behavior. For specifications
see `reference/` and `subsystems/`.

---

## Source Files

| Source file | Branch | Approx size | Primary content |
|---|---|---|---|
| `docs/tmp/refinements/23-architectural-analysis-improvements.md` | `main` | ~37 KB | Coherence analysis: trait sufficiency, layer taxonomy, cognitive speeds, Engram universality, crosscut isolation, category theory grounding, novel proposals, inconsistencies, prioritized improvements |
| `docs/tmp/refinements/24-cross-section-integration-map.md` | `main` | ~44 KB | Per-pair subsystem integration map: missing connections, wired connections, data-flow descriptions, failure modes, open questions |
| `docs/tmp/refinements/31-implementation-readiness-audit.md` | `main` | ~34 KB | Per-subsystem implementation readiness scores, gap lists, next-action recommendations |
| `docs/tmp/refinements/34-synergy-integration-map.md` | `agent-refinements` | ~20 KB | Ten load-bearing primitives, 10×10 synergy matrix, ten named synergies, non-synergies, emergent properties |

Total source: ~135 KB, ~1,600 lines of dense markdown.

---

## Target Files

### `analysis/architectural-analysis/` — 13 files

Source: `23-architectural-analysis-improvements.md`

| File | What it covers |
|---|---|
| `README.md` | Folder index, contents table, reading order |
| `00-overview.md` | What the architectural analysis is and how to use it |
| `01-findings-summary.md` | All seven findings condensed for quick orientation |
| `02-finding-trait-sufficiency.md` | F1: Trait surface is sufficient; Scoring and Gating are well-defined |
| `03-finding-layer-taxonomy.md` | F2: Layer taxonomy is present but boundary violations need attention |
| `04-finding-cognitive-speeds.md` | F3: Cognitive speeds are correctly differentiated |
| `05-finding-engram-universality.md` | F4: Engram universality is under-exploited |
| `06-finding-crosscut-isolation.md` | F5: Cross-cut isolation is inconsistent |
| `07-finding-category-theory.md` | F6: Category-theory grounding is present and valuable |
| `08-novel-proposals.md` | F7 / eight novel proposals derived from the analysis |
| `09-finding-inconsistencies.md` | All named inconsistencies and gaps |
| `10-prioritized-improvements.md` | Ordered improvement list with rationale |
| `99-cross-findings-matrix.md` | Which findings affect which subsystems; cross-findings table |

### `analysis/integration-map/` — 31 files

Source: `24-cross-section-integration-map.md`

| File | What it covers |
|---|---|
| `README.md` | Folder index, tier explanation, reading order |
| `00-overview.md` | Overview of the full integration surface |
| **Tier 1 pairs** (missing, high priority) | |
| `daimon-x-orchestration.md` | M1: Daimon → Orchestration (session memory persistence gap) |
| `daimon-x-composition.md` | M2: Daimon → Composition (persona guidance routing gap) |
| `verification-x-orchestration.md` | M3: Verification → Orchestration (gate result feedback gap) |
| `learning-x-composition.md` | M4: Learning → Composition (heuristic injection gap) |
| `learning-x-routing.md` | M6: Learning → Routing (c-factor policy gap) |
| **Tier 2 pairs** (missing, medium priority) | |
| `neuro-x-composition.md` | M5: Neuro → Composition (HDC relevance scoring gap) |
| `anti-knowledge-x-composition.md` | M15: Anti-knowledge → Composition (exclusion list gap) |
| `code-intel-x-composition.md` | M8: Code-intel → Composition (codebase context gap) |
| `conductor-x-routing.md` | M9: Conductor → Routing (multi-agent session routing gap) |
| `learning-x-config.md` | M10: Learning → Config (heuristic persistence gap) |
| `orchestration-x-daimon.md` | M11: Orchestration → Daimon (episode ingestion gap) |
| **Tier 3 pairs** (missing, lower priority) | |
| `dreams-x-neuro.md` | M7: Dreams → Neuro (reinterpretation fingerprint gap) |
| `dreams-x-daimon.md` | M18: Dreams → Daimon (long-term memory replay gap) |
| `neuro-x-verification.md` | M14: Neuro → Verification (semantic gate input gap) |
| `safety-x-composition.md` | M13: Safety → Composition (constraint injection gap) |
| `code-intel-x-verification.md` | M16: Code-intel → Verification (code-aware gate gap) |
| `lifecycle-x-neuro.md` | M20: Lifecycle → Neuro (model lifecycle embedding gap) |
| **Tier 4 pairs** (missing, low priority) | |
| `coordination-x-orchestration.md` | M12: Coordination → Orchestration (multi-session state gap) |
| `coordination-x-dreams.md` | M19: Coordination → Dreams (collaborative memory gap) |
| `tech-analysis-x-heartbeat.md` | M17: Tech-analysis → Heartbeat (health metric gap) |
| **Wired pairs** (already connected) | |
| `agents-x-composition.md` | Agents → Composition (domain profile injection — wired) |
| `agents-x-verification.md` | Agents → Verification (output gate checking — wired) |
| `conductor-x-orchestration.md` | Conductor → Orchestration (task delegation — wired) |
| `learning-x-verification.md` | Learning → Verification (heuristic gate config — wired) |
| `neuro-x-learning.md` | Neuro → Learning (clustering signals — wired) |
| `orchestration-x-learning.md` | Orchestration → Learning (outcome feedback — wired) |
| `daimon-x-learning.md` | Daimon → Learning (long-term memory to heuristics — wired) |
| `safety-x-agents.md` | Safety → Agents (constraint propagation — wired) |
| `99-master-lattice.md` | Full integration lattice index with tier table and missing-edge count |

### `analysis/readiness-audit/` — 25 files

Source: `31-implementation-readiness-audit.md`

| File | What it covers |
|---|---|
| `README.md` | Folder index, readiness scoring key, reading order |
| `00-overview.md` | Readiness score distribution, headline gaps |
| `01-audit-summary.md` | All 21 subsystems condensed with readiness scores and critical gaps |
| `subsystem-architecture.md` | Architecture subsystem readiness |
| `subsystem-agents.md` | Agents subsystem readiness |
| `subsystem-composition.md` | Composition subsystem readiness |
| `subsystem-orchestration.md` | Orchestration subsystem readiness |
| `subsystem-verification.md` | Verification subsystem readiness |
| `subsystem-learning.md` | Learning subsystem readiness |
| `subsystem-neuro.md` | Neuro subsystem readiness |
| `subsystem-daimon.md` | Daimon subsystem readiness |
| `subsystem-dreams.md` | Dreams subsystem readiness |
| `subsystem-safety.md` | Safety subsystem readiness |
| `subsystem-conductor.md` | Conductor subsystem readiness |
| `subsystem-coordination.md` | Coordination subsystem readiness |
| `subsystem-lifecycle.md` | Lifecycle subsystem readiness |
| `subsystem-heartbeat.md` | Heartbeat subsystem readiness |
| `subsystem-tools.md` | Tools subsystem readiness |
| `subsystem-chain.md` | Chain subsystem readiness |
| `subsystem-code-intelligence.md` | Code-intelligence subsystem readiness |
| `subsystem-deployment.md` | Deployment subsystem readiness |
| `subsystem-interfaces.md` | Interfaces subsystem readiness |
| `subsystem-identity-economy.md` | Identity/economy subsystem readiness |
| `subsystem-technical-analysis.md` | Technical-analysis subsystem readiness |
| `99-next-actions.md` | Consolidated action list ordered by priority across all subsystems |

### `analysis/synergy-map/` — 13 files

Source: `34-synergy-integration-map.md`

| File | What it covers |
|---|---|
| `README.md` | Folder index, primitive roster, reading order |
| `00-overview.md` | Full primitive table, 10×10 synergy matrix, synergy summaries, seven-step loop, moat argument, emergent properties, design guidance |
| `synergy-01-demurrage-x-hdc.md` | S1: Demurrage × HDC → self-trimming semantic memory |
| `synergy-02-heuristics-pulse-bus.md` | S2: Heuristics × Pulse × Bus → continuous calibration |
| `synergy-03-cfactor-bus-hdc.md` | S3: c-factor × Bus × HDC → diversity-aware routing |
| `synergy-04-replication-living-research.md` | S4: Replication ledger × Heuristics × paper Engram → living research |
| `synergy-05-plugin-spi-ecosystem.md` | S5: Plugin SPI × Substrate × Bus → ecosystem growth path |
| `synergy-06-cfactor-heuristics-peer-model.md` | S6: c-factor × Heuristics → peer-model learning |
| `synergy-07-dreams-retroactive.md` | S7: Dreams × Substrate × Pulse → retroactive insight |
| `synergy-08-demurrage-heuristic-relearning.md` | S8: Demurrage × Heuristics × calibration → graceful relearning |
| `synergy-09-hdc-consensus-agreement.md` | S9: HDC × Consensus × Bus → substantive agreement detection |
| `synergy-10-typed-context-domain-safety.md` | S10: TypedContext × domain profiles × Gate → auditable domain safety |
| `99-master-synergy-table.md` | Searchable index: synergies by primitive, by status, by layer; non-synergies; emergent properties |

---

## File Count Summary

| Subtree | Files | Source |
|---|---|---|
| `analysis/architectural-analysis/` | 13 | `23-architectural-analysis-improvements.md` |
| `analysis/integration-map/` | 31 | `24-cross-section-integration-map.md` |
| `analysis/readiness-audit/` | 25 | `31-implementation-readiness-audit.md` |
| `analysis/synergy-map/` | 13 | `34-synergy-integration-map.md` |
| `analysis/README.md` | 1 | (cluster-level index) |
| `_migration/cluster-F-analysis.md` | 1 | (this file) |
| **Total** | **84** | — |

---

## Transformation Rules Applied

1. **One concept per file** — every finding, every pair, every synergy gets its own file.
2. **CONVENTIONS.md § 6.3 Integration Page Template** used for all `integration-map/` pair files.
3. **Adapted synergy template** used for all `synergy-map/` synergy files: Primitives Involved,
   What the Synergy Unlocks, What Flows, Invariants, Failure Modes, Today vs. Planned, Cross-
   References, Open Questions.
4. **Target-state / today separation** — every file has a `## Today vs. Planned` section
   rather than `[target-state]` disclaimers scattered through prose.
5. **Cross-linking** — synergy files link to related integration-map pairs and readiness-audit
   subsystems; architectural-analysis findings link to relevant synergies; master index files
   (`99-*`) make the lattice searchable.
6. **NOTHING IS LOST** — the source files contained approximately 35 named findings, 30 named
   integration pairs, 21 subsystem audits, 10 named synergies, 3 non-synergies, 3 emergent
   properties, a full 10×10 primitive matrix, and a seven-step loop description. All are
   present and individually addressable in the target tree.

---

## What Was Not Changed

- Source files are not deleted. They remain at their original paths in `docs/tmp/refinements/`.
- `analysis/README.md` (the cluster-level index) was written as part of this cluster and
  already references all four subtrees.
- No reference/ or subsystems/ files were modified. This cluster is purely analysis.

---

## Notes

- Files 33, 34, and 35 were only committed to the `agent-refinements` branch, not `main`, at
  the time of this refactor. Source file 34 was read from that branch via GitHub API.
- The `analysis/integration-map/` tree contains 20 "missing integration" pairs and 8 "wired"
  pairs (+ README, overview, master lattice = 31 files total).
- The `analysis/synergy-map/` tree preserves the full 10×10 primitive matrix verbatim in
  `00-overview.md`, so the original source's tabular content is not lost even though the
  individual synergies live in separate files.
