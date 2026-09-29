# Implementation Readiness Audit

> **v3 depth file** -- `/docs/v3/depth/00-architecture/implementation-readiness-audit.md`
> Canonical source: v1 `docs/v1/00-architecture/31-implementation-readiness-audit.md`
> Status: **Updated for current state** -- 39 workspace members, ~1M LOC, 10,300+ tests.
> 48/48 epics accepted as programme manifests (the executable-task count was withdrawn
> on 2026-09-29 as stale). Graph is the sole execution
> engine. The v1 audit covered 36 members at ~322K LOC with 3,761 tests; this revision
> reflects the 2026-09-15 reality.

---

## 1. Methodology

Every documentation section was scored against 6 implementation-readiness criteria:

| Criterion | What it measures |
|---|---|
| **rust_structs** | Quality/completeness of struct, trait, enum definitions with field-level types |
| **pseudocode** | Algorithm pseudocode, Rust code blocks, step-by-step decision logic |
| **config_params** | Configuration parameters with defaults, ranges, rationale |
| **error_handling** | Error types, recovery paths, failure mode specification |
| **integration_wiring** | How components connect to other crates and the CLI entry point |
| **test_criteria** | Observable test conditions, acceptance thresholds, named test cases |

Scale: 0 = absent, 1 = mentioned, 2 = partial, 3 = adequate, 4 = strong, 5 = exemplary.

File classifications:
- **Wired** -- Code exists AND is called from the CLI/orchestration loop
- **Built** -- Code exists in crates and tests pass
- **Specified** -- Has concrete Rust types, real constants, wiring details
- **Scaffold** -- Has design intent but gaps remain
- **Concept only** -- Primarily theoretical

---

## 2. Current State Summary

### 2.1 Workspace Reality (2026-09-15)

| Metric | v1 Audit (2026-04-13) | Current |
|---|---|---|
| Workspace members | 36 | 39 |
| Total LOC | ~322K | ~1M |
| Total tests | 3,761 | 10,300+ |
| Crates wired to CLI | 12 | 30+ |
| Epics accepted | 0 | 48/48 |
| Executable tasks complete | 0 | Withdrawn 2026-09-29 (stale count) |
| Execution engine | WorkflowEngine | Graph (sole engine) |
| HTTP routes | 0 | ~376 canonical (~421 incl. aliases) |
| ACP tests | 0 | 180 |

### 2.2 Section Scorecard (Updated)

| # | Section | Structs | Pseudo | Config | Errors | Wiring | Tests | Total | Crate Status |
|---|---|---|---|---|---|---|---|---|---|
| 00 | Architecture | 5 | 5 | 5 | 4 | 5 | 4 | **28/30** | roko-core: Stable |
| 01 | Orchestration | 5 | 5 | 5 | 5 | 5 | 5 | **30/30** | Graph engine: Wired |
| 02 | Agents | 5 | 5 | 5 | 4 | 5 | 4 | **28/30** | roko-agent: Wired (12 providers) |
| 03 | Composition | 5 | 5 | 5 | 4 | 5 | 4 | **28/30** | roko-compose: Wired |
| 04 | Verification | 5 | 5 | 5 | 5 | 5 | 5 | **30/30** | roko-gate: Wired (19 gates) |
| 05 | Learning | 5 | 5 | 5 | 5 | 5 | 5 | **30/30** | roko-learn: Wired |
| 06 | Neuro | 5 | 5 | 5 | 4 | 5 | 4 | **28/30** | roko-neuro: Wired |
| 07 | Conductor | 5 | 5 | 5 | 5 | 5 | 5 | **30/30** | roko-conductor: Wired |
| 08 | Chain | 4 | 4 | 4 | 3 | 4 | 3 | **22/30** | roko-chain: Local tranche |
| 09 | Daimon | 5 | 5 | 5 | 4 | 5 | 5 | **29/30** | roko-daimon: Wired (E23 10/10) |
| 10 | Dreams | 5 | 5 | 5 | 4 | 5 | 4 | **28/30** | roko-dreams: Wired |
| 11 | Safety | 5 | 5 | 5 | 5 | 5 | 5 | **30/30** | E34 8/8 strict |
| 12 | Interfaces | 5 | 5 | 5 | 4 | 5 | 4 | **28/30** | roko-cli TUI: Wired |
| 13 | Coordination | 5 | 5 | 5 | 4 | 5 | 4 | **28/30** | E28 8/8 groups |
| 14 | Identity/Economy | 5 | 5 | 4 | 3 | 4 | 4 | **25/30** | Local tranche complete |
| 15 | Code Intelligence | 5 | 5 | 4 | 3 | 5 | 5 | **27/30** | roko-mcp-code: Wired |
| 16 | Heartbeat | 5 | 5 | 5 | 4 | 5 | 5 | **29/30** | E33 9/9 telemetry |
| 17 | Lifecycle | 5 | 5 | 5 | 5 | 5 | 5 | **30/30** | R04 lifecycle complete |
| 18 | Tools | 5 | 5 | 5 | 5 | 5 | 5 | **30/30** | roko-plugin: E32 8/8 |
| 19 | Deployment | 5 | 5 | 5 | 4 | 4 | 4 | **27/30** | roko-serve: Wired |
| 20 | Technical Analysis | 5 | 5 | 4 | 3 | 3 | 4 | **24/30** | Partial (arena R03) |

### 2.3 Averages by Criterion (Updated)

| Criterion | v1 Mean | Current Mean | Improvement |
|---|---|---|---|
| rust_structs | 4.6 | 5.0 | +0.4 |
| pseudocode | 4.5 | 5.0 | +0.5 |
| config_params | 4.6 | 4.9 | +0.3 |
| **error_handling** | **3.4** | **4.1** | **+0.7** |
| integration_wiring | 3.9 | 4.8 | +0.9 |
| test_criteria | 4.1 | 4.5 | +0.4 |

The v1 universal weakness (error handling, mean 3.4) has been significantly addressed.
Integration wiring saw the largest improvement (+0.9) reflecting the 48 accepted epics.

---

## 3. Crate Implementation Status

| Crate | Tests | CLI Wired? | Maturity | Change from v1 |
|---|---|---|---|---|
| roko-core | 1,200+ | Yes (kernel) | **Stable** | +600 tests, pub(crate) encapsulation |
| roko-agent | 1,100+ | Yes | **Wired** (12 providers) | +550 tests, 7 new providers |
| roko-agent-server | 200+ | Yes (sidecar) | **Wired** | New crate |
| roko-serve | 800+ | Yes | **Wired** (~376 routes) | +774 tests, from 0 routes to 376 |
| roko-gate | 500+ | Yes | **Wired** (19 gates) | +300 tests, 8 new gates |
| roko-compose | 300+ | Yes | **Wired** | +277 tests |
| roko-conductor | 350+ | Yes | **Wired** (12 watchers) | +220 tests, +2 watchers |
| roko-learn | 600+ | Yes | **Wired** | +252 tests, experiments live |
| roko-cli | 500+ | Entry point | **Wired** | +462 tests, TUI parity |
| roko-fs | 150+ | Yes | **Stable** | +90 tests |
| roko-std | 250+ | Yes | **Stable** (35 defs) | +130 tests, +16 defs |
| roko-execution | 100+ | Yes | **Wired** | New crate (RuntimeServices) |
| roko-runtime | 80+ | Yes | **Stable** | +74 tests |
| roko-primitives | 60+ | Yes | **Stable** | +42 tests |
| roko-graph | 600+ | Yes | **Wired** (sole engine) | New crate (from #260/#276) |
| roko-neuro | 200+ | Yes | **Wired** | +197 tests |
| roko-dreams | 150+ | Yes | **Wired** | +143 tests |
| roko-daimon | 300+ | Yes | **Wired** (E23 10/10) | +300 tests, from 0 |
| roko-acp | 180+ | Yes | **Wired** (E17 8/8) | New crate, 180 ACP tests |
| roko-plugin | 200+ | Yes | **Wired** (E32 8/8) | New crate |
| roko-chain | 300+ | Partial | **Local tranche** | +290 tests, 4 state machines |
| roko-index | 100+ | Yes | **Wired** | +68 tests, MCP server |
| roko-mcp-code | 50+ | Yes | **Wired** | New crate |
| roko-demo | 30+ | Yes | Built | New crate |
| roko-lang-* (3) | 100+ | Yes | **Wired** | +100 tests |
| roko-mcp-* (5) | 80+ | Partial | Partial | Partial |
| **Total** | **10,300+** | | | **+6,539 tests** |

---

## 4. Gap Analysis: What Remains

### 4.1 Resolved v1 Gaps

The following v1 critical gaps (G1-G15) are now resolved:

| v1 Gap | Resolution |
|---|---|
| G1: Wire ToolDispatcher into orchestrate.rs | Done -- safety pipeline is live for all 12 providers |
| G2: Wire roko-index into ContextProvider | Done -- roko-mcp-code provides code intelligence |
| G3: Register lang providers | Done -- all three wired |
| G4: Expand role prompts | Done -- 9-layer SystemPromptBuilder with 11 templates |
| G5: Somatic landscape + VCG affect bidding | Done (E23) -- DaimonState, GoalTree, somatic k-d tree |
| G6: Active inference EFE scorer | Done (E23) -- EFE routing wired |
| G7: 8 missing feedback loops | Done -- all 8 have at least one real code path |
| G8: Normalize kernel vocabulary | Done -- Signal/Engram rename complete |
| G9: PAD persistence | Done -- DaimonState persists across sessions |
| G10: Add tier field to KnowledgeEntry | Done -- four tiers wired |
| G11: Fix half-life constants | Done |
| G12: Consolidate daimon implementations | Done -- single roko-daimon crate |
| G13: Safety critical integration | Done (E34 8/8) |
| G14: Create Dockerfiles | Done -- deployment targets exist |
| G15: Mattar-Daw utility scoring | Done -- Dream consolidation wired |

### 4.2 Remaining Product Work

| # | Area | What Remains | Complexity |
|---|---|---|---|
| R1 | Chain runtime | Production transport, persistence, authorization, indexing adapters | Very High |
| R2 | Fresh dogfood proof | Clean live full self-hosting rerun | Medium |
| R3 | Provider-owned internals | Provider trace Signals, internal call/result screening | Medium |
| R4 | Adaptive immune memory | Semantic/adaptive immune memory for CIS | Medium |
| R5 | External ledger authenticity | Externally anchored whole-ledger authentication | High |
| R6 | WIT/Component hostcalls | WASM Component-model Store/Bus hostcalls | Medium |
| R7 | Native Agent observation | Direct native Agent publication into E33 observation ingress | Medium |
| R8 | ACP/serve experiment parity | Canonical-section/receipt parity for experiment injection | Medium |
| R9 | Named-surface TUI rendering | Full named-surface TUI rendering (Workbench/Inbox/Canvas/Minimap/Autonomy) | Medium |

### 4.3 v1 Rewrite-Track Assessment (Retrospective)

The v1 audit identified five rewrite-track candidates. Here is their resolution:

| v1 Candidate | Resolution | Outcome |
|---|---|---|
| R1: roko-core kernel | Engram + Pulse model adopted, Bus as kernel trait | Done |
| R2: roko-learn reorganization | Restructured with experiments, playbooks, bandits | Done |
| R3: Substrate trait rewrite | Expanded with query/scan semantics, cold tier | Done |
| R4: Gate pipeline | Expanded to 19 gates, 7-rung pipeline with adaptive thresholds | Done (incremental) |
| R5: roko-compose engine | 9-layer SystemPromptBuilder with enrichment pipeline | Done |

All five were addressed, though R4 took the incremental rather than rewrite path.

---

## 5. Documentation Quality

### 5.1 Systemic Strengths (Maintained)

1. **Self-aware status docs.** Honest gap inventories continue across all sections.
2. **Academic grounding.** Every section cites primary research correctly applied.
3. **Config completeness.** 19 of 20 sections score 4+ on config_params.
4. **Mathematical precision.** Formulas at paper-quality precision with worked examples.

### 5.2 Improvements from v1

1. **Error handling** improved from mean 3.4 to 4.1. The eight accepted safety epic
   (E34) drove systematic error type coverage across the workspace.
2. **Integration wiring** improved from mean 3.9 to 4.8. The 48 epics closed most
   "built but not wired" gaps.
3. **Test criteria** improved from mean 4.1 to 4.5. 10,300+ tests provide real
   validation for spec claims.

### 5.3 Remaining Weakness

Integration wiring for the chain/economic runtime (Section 08, Section 14) remains the
weakest area at 4/5 and 4/5 respectively. This reflects the intentional deferral of
production chain adapters to Phase 2+.

---

## 6. Recommended Next Steps

1. **Fresh dogfood proof**: Rerun the complete self-hosting workflow against the
   regression fixes. This is the highest-priority validation gap.
2. **Chain adapter scoping**: Define the minimum viable production chain adapter set
   for Phase 2 (daeji node, transport, indexer).
3. **Named-surface TUI completion**: Wire the five named surfaces (Workbench, Inbox,
   Canvas, Minimap, Autonomy) into TUI rendering.
4. **ACP/serve experiment parity**: Bring experiment injection in ACP and serve paths
   to canonical-section/receipt parity with the runner path.

---

## Cross-References

- [Cross-section integration map](./cross-section-integration-map.md) -- Section dependency matrix
- [Comprehensive test strategy](./comprehensive-test-strategy.md) -- Per-crate test targets
- [Synergy integration map](./synergy-integration-map.md) -- Primitive interaction matrix
- [Cross-pollination innovations](./cross-pollination-innovations.md) -- Composition innovations
