# 102 — Spec-Debt Ledger (Concept-Level)

> Status-quo pack · **DEEPER SECOND PASS** · verified against code **2026-07-08 at git HEAD `5852c93c05`** (branch `main`).
> Scope: one exhaustive ledger of every named architectural concept across `docs/v2/` (28 sections) and `docs/v2-depth/` (22 sections), synthesizing 15/85/86/87 (v2 coverage), 18 (v2-depth coverage), and 02-SPEC-EVOLUTION into a single evidence-backed spec-debt register.
> Method: every status is backed by an actual `rg` run over `crates/` + `apps/` at this HEAD (hit counts + a `file:line` or "0 hits"). Greps re-run this pass; deltas vs prior docs flagged inline.

Status tags: **Built** = wired end-to-end on a shipping path · **Partial** = built-not-wired / reachable-but-cold / behavior-yes-shape-no (🔌🟡) · **Renamed-only** = docs renamed the concept, zero code motion (🕰️) · **Zero-code** = grep → 0 relevant hits (❌).

---

## The one-paragraph verdict

Across the two spec corpora there are **~129 distinct named concepts**. By code status: **~24 Built**, **~52 Partial**, **~5 Renamed-only**, **~48 Zero-code** (see the [tally](#status-tally) — a concept is "Zero-code" only when its key type/symbol greps to 0). The Built third is real but v1-shaped and mostly *not* the concepts the v2 docs lead with; the Zero-code set is dominated by **whole aspirational bands** (10-GROUPS, 23-ARENAS, 24-DEFI trading stack, the Lens/telemetry pipeline, the 4-role Verdict, the composite Agent) that the newest docs (v2-depth) describe as if they ship. **The single most load-bearing zero-code concept is the `Lens` protocol** — it is the organizing abstraction for five specs (09-TELEMETRY, 15-TELEMETRY, 03-GRAPH conductor reframe, 10-LEARNING c-factor, 16/20/21 surfaces & marketplace) and greps to exactly 0 (`rg 'trait Lens|struct .*Lens' crates/ apps/` → **0**); until it exists, every "X-as-Lens" doc reads as vaporware.

---

## How to read the evidence column

Every row cites a live grep at HEAD `5852c93c05`. Counts are `rg -c` totals over `crates/ apps/`. A "0 hits" means the concept's canonical type/symbol name is absent (README/comment-only mentions are called out as such, since they are not code). Where a concept exists under a *different* name, the nearest real symbol is cited and the row is tagged Renamed-only or Partial, not Zero-code.

---

## KERNEL band — docs/v2 01-07, 27 · docs/v2-depth 01-11

### Built (wired on a shipping path)

| Concept | Source doc | Status | Grep evidence |
|---|---|---|---|
| `Store/Score/Verify/Route/Compose/React` verb traits | 02-CELL, 00-index | Built | `traits.rs:37-339` (11 traits :37-425); `Substrate: Store` compat `:428` |
| Kind compound-join lattice | 01-SIGNAL | Built | `roko-core/src/kind.rs:163,181` (`compound()`+`matches()`) |
| Graduation `Pulse::graduate()` | 01-SIGNAL / 02-block store-bus-duality | Built | `roko-core/src/pulse.rs:138-151` |
| 7-rung gate pipeline + 11 gates | 02-CELL / 02-block | Built | `roko-gate/src/rung_selector.rs:60-120` (`CANONICAL_ORDER:[Rung;7]`); `gate_pipeline.rs:209` |
| Runner v2 execution engine | 27-ORCHESTRATOR | Built (🕰️ shape) | `roko-cli/src/runner/event_loop.rs` (6,681 LOC); `#[default] RunnerV2` `main.rs:1301` |
| Episodes / experiments / efficiency | 07-LEARNING | Built | `roko-learn/src/{episode_logger,prompt_experiment,efficiency}.rs`; wired `runner/event_loop.rs:764` |
| C5 playbook distillation | 07-LEARNING | Built | `roko-learn/src/playbook.rs`; `runner/event_loop.rs:5562` |
| Neuro tiers + AntiKnowledge + falsifiers | 06-MEMORY / 11-memory | Built | `roko-neuro/src/tier_progression.rs:94-540`; `knowledge_store.rs:536-559` |
| Dream post-plan trigger | 06-MEMORY / 11-memory | Built (hindsight ❌) | `runner/event_loop.rs:1314-1322,1474-1480`; `roko-dreams/src/cycle.rs:537` |
| Section-effect tracking | 05-AGENT | Built (counts, not Beta) | `roko-learn/src/section_effect.rs:30-187` |
| CascadeRouter (3-stage LinUCB + persistence) | 02-CELL/05/07 Route | Built (🕰️ LinUCB≠EFE) | `roko-learn/src/cascade_router.rs:37,85,436` (285 refs) |
| Gap 4 (post-gate reflection), Gap 8 (neuro→router bias) | 27-ORCHESTRATOR | Built | `post_gate_reflection.rs`; `cascade_router.rs:404-422,623-679` |

### Partial (built-not-wired / cold / behavior-not-shape)

| Concept | Source doc | Status | Grep evidence |
|---|---|---|---|
| `Pulse` struct | 01-SIGNAL | Partial (no `source/lineage_hint/trace_id`) | `roko-core/src/pulse.rs:75` (6 `pub struct Pulse`-scope hits) |
| `Bus` trait / `PulseBus` | 01-SIGNAL | Partial (no `replay_since`; consumers = runtime + gate) | `traits.rs:385`; `pulse_bus.rs:29` (43 refs); consumers `roko-runtime/src/pulse_bus.rs`, `roko-gate/src/verdict_publisher.rs` — **not ISFR-only** (corrects briefing) |
| Two `Cell` traits (incompatible) | 02-CELL / 02-block | Partial | `roko-core/src/cell.rs:91` **and** `roko-graph/src/cell.rs:74` (both `pub trait Cell`) |
| 9 protocols as `: Cell` supertraits | 02-CELL / 02-block | Partial (only Observe/Connect/Trigger; impls test-only) | `traits.rs:400,408,420`; impls `roko-core/tests/phase1_integration.rs:188,237,294` |
| VCG attention auction | 02-CELL / 02-block vcg-attention-auction | Partial (reachable-but-cold) | `roko-compose/src/auction.rs:380` (`fn vcg_allocate`, 4 refs); call `prompt.rs:1213`; greedy dominates |
| AttentionBidder vocabulary | 02-block | Built (compose-path only) | `system_prompt_builder.rs:38` (108 refs) |
| `TaskExecutorCell` live dispatch | 04-EXECUTION / 03-graph | Partial ❌P0 (`dry_run:true`) | `roko-graph/src/cells/task_executor.rs:20,30-32` (16 refs) |
| `GraduationCell` | 02-CELL | Partial (0 runtime call sites) | `roko-graph/src/cells/graduation.rs:37-160` (22 refs, all lib/test) |
| Conditional edges + `Condition`/`EdgeCondition` | 03-GRAPH | Partial (evaluator built, engine ignores) | `roko-graph/src/condition.rs:31-92`; `engine.rs:277-300` |
| Graph `BudgetTracker` | 04-EXECUTION | Partial (built+tested, never called) | `roko-graph/src/budget.rs:17-180` |
| Hot Graph tick loop | 03-GRAPH / 05-execution-engine | Partial (`persist_tick_state` dead; fresh state/tick) | `roko-graph/src/hot.rs:40,117-223`; `.roko/GAPS.md:17` |
| gamma/theta/delta heartbeat + `CorticalState` | 04-EXECUTION / 07-agent-runtime | Partial (legacy-orchestrate only) | `roko-runtime/src/heartbeat.rs:23,26,29,228-556`; driven from `orchestrate.rs:7775` (feature-gated off) |
| `DeltaConsumer` (dream cadence) | 07-agent-runtime | Partial (self-labelled NOT WIRED) | `roko-runtime/src/delta_consumer.rs:168` (8 refs); never instantiated |
| `ThetaConsumer` | 07-agent-runtime | Partial | `roko-runtime/src/theta_consumer.rs:140` (8 refs) |
| Demurrage economy (tax, no income) | 01-SIGNAL / 06-MEMORY | Partial (🔌 balances stuck 0.0) | `RuntimeKnowledgeLifecycle` `roko-neuro/src/lifecycle.rs:194` (3 refs, **0 external callers**) |
| HDC fingerprint/repulsion/resonance | 01/06 / 11-memory | Partial (compiled out) | `HdcFingerprint` 28 refs but `hdc` feature enabled by no consumer; `knowledge.jsonl` 89/89 `hdc_vector:null` |
| Temporal knowledge (Allen algebra) | 06-MEMORY / 11-memory | Partial (built, 0 callers) | `AllenRelation` `roko-neuro/src/temporal.rs:12` (48 refs), 25 tests, no runtime caller |
| `WisdomGate` / anti-groupthink consensus | 07-LEARNING C4 | Partial (legacy-orchestrate only) | `roko-orchestrator/src/coordination.rs:743` (5 refs) — **exists; corrects 85-doc "grep→0"** |
| CognitiveWorkspace (name collision) | 05-AGENT §16 | Partial (audit object ≠ VCG market) | core `cognitive_workspace.rs:14` (31 refs) is an invocation audit record; VCG market lives in `roko-compose/auction.rs` |
| MAP-Elites QD archive | 10-learning EvoSkills | Built (mis-located) | actually `roko-dreams/src/phase2/evolution.rs`, **not** `skill_library.rs`; `EvoSkills` at `skill_library.rs:1807` is success-rate, not QD |
| Adaptive gate thresholds | 07-LEARNING L1 | Partial (not consulted by runner rung) | `roko-gate/src/adaptive_threshold.rs`; `rung_dispatch.rs` has 0 adaptive refs |
| CFactor metrics | 07-LEARNING C4 / 15-TELEMETRY | Partial (data yes, no Lens/learner/gate) | `roko-core/src/cfactor.rs:103` `CFactorSummary` (34 refs) |
| 27-Gap 1/2/3/9/10/11 | 27-ORCHESTRATOR | Partial (types/partial wiring) | `review_verdict.rs`, `compile_errors.rs`, `error_patterns.rs`, `pattern_discovery.rs`, `playbook_rules.rs` |
| 27-Gap 6 WarmPool | 27-ORCHESTRATOR | Partial (container, `WarmPool::new(0)`) | `dispatch/warm_pool.rs:28`; `dispatch/factory.rs:89` |
| ColdStore / ArchiveColdSubstrate | 01-SIGNAL | Partial→Built at edge | trait `traits.rs:102`; `roko-fs/src/cold_substrate.rs`; timer now wired serve-side (`lib.rs:344`) |
| Predict-publish-correct | 02-CELL / 07 | Partial (one CalibrationPolicy, not per-Cell) | `roko-learn/src/calibration_policy.rs:3,39` |

### Renamed-only (docs renamed; code unchanged)

| Concept | Source doc | Status | Grep evidence |
|---|---|---|---|
| `Signal` as universal noun | 01-SIGNAL | Renamed-only 🕰️ | `Engram` is the struct `engram.rs:63`; `Signal` alias `signal.rs:6` (`Engram as Signal`, 3 hits) |
| **block** (Cell rename) | v2-depth 02-block | Renamed-only (docs) | **no `Block` type in code**: `rg '(struct\|enum\|trait) Block' → 0`; `BlockObserver/BlockTag/BlockSpaceAgent` are chain concepts, unrelated |
| Verdicts-as-Signals | v2-depth 02-block | Renamed-only 🕰️ | `Verdict` is v1 pass/score `verdict.rs:51` (8 refs); no demurrage/Kind/lineage on it |
| Knowledge-as-Signal | v2-depth 11-memory | Renamed-only 🕰️ | separate `KnowledgeEntry` JSONL store `roko-neuro/src/lib.rs:337`; not a Signal/Cell |
| Verdict redesign (4-role) | 02-CELL | Renamed-only 🕰️ | v1 shape; scalar `reward` exists on a *separate* `Outcome` struct `verdict.rs:200` (3 refs), wrong type |

### Zero-code (grep → 0)

| Concept | Source doc | Status | Grep evidence |
|---|---|---|---|
| `SignalId` (ULID) / `SignalRef` | 01-SIGNAL | Zero-code | `rg 'SignalId\|SignalRef'` → **0** |
| `Signal::to_pulse()` projection | 01-SIGNAL §12 | Zero-code | `rg 'to_pulse'` → **0** |
| Calibration stack (temperature/ECE/Beta-Binomial/isotonic) | 01-SIGNAL §5 | Zero-code | `rg 'temperature_scale\|AxisCalibrator\|Isotonic\|compute_ece'` → **0** |
| Taint lattice ops (`join`/`flows_to`) | 01-SIGNAL §10 / 16 | Zero-code | `Taint` enum exists `provenance.rs:24` but no lattice ops → **0** |
| `EvidenceKind` (19 typed) | 02-CELL | Zero-code | `rg 'EvidenceKind'` → **0** |
| `CriterionResult` (hard/soft criteria) | 02-CELL | Zero-code | `rg 'CriterionResult'` → **0** |
| Bradley-Terry / `PairwiseJudgment` | 02-CELL / 02-block | Zero-code | `rg 'BradleyTerry\|PairwiseJudgment'` → **0** |
| Free-monad `CellProgram` / `PROTOCOL_ADJACENCY` | 02-CELL / 02-block | Zero-code | `rg 'CellProgram\|PROTOCOL_ADJACENCY'` → **0** |
| Cost as `u64` microcents | 02-CELL | Zero-code | f64 USD everywhere; no integer-cost type |
| `NodeKind` variants (Branch/FanOut/FanIn/Loop/Slot/Wait/HumanInput) | 03-GRAPH | Zero-code | Node = id+cell_type string `types.rs:56`; no NodeKind enum |
| Graph-as-Cell (fractal SubGraph) | 03-GRAPH | Zero-code | Graph does not `impl Cell` `types.rs:89` |
| Workflow/Activity split + replay | 03/04 | Zero-code | `rg 'ExecutionClass\|ActivityRecord'` → **0** |
| `GraphPolicy`/`FailureStrategy` (8 variants) | 03-GRAPH | Zero-code | fail-fast skip-downstream only `engine.rs:132` |
| `GraphEstimate` (cost/critical-path) | 04-EXECUTION | Zero-code | `rg 'GraphEstimate'` → **0** |
| `Engine` API (start/resume/pause/register_hot/estimate) | 04-EXECUTION | Zero-code | only one-shot `GraphEngine::execute` `engine.rs:111` |
| `DegradationLevel` ladder | 04-EXECUTION | Zero-code | `rg 'DegradationLevel'` → **0** |
| composite `Agent<S>` type-state lifecycle | 05-AGENT / 07-agent-runtime | Zero-code | `rg 'Agent<.*Active'` → **0**; no composite Agent type |
| `SlotManager` (CAS budget) | 05-AGENT | Zero-code | agent pools exist (different shape); no SlotManager |
| 16 `T0Probe` | 05-AGENT | Zero-code | `rg 'T0Probe'` → **0** |
| `ReflexStore`/`ReflexRule` | 05-AGENT | Zero-code | `rg 'ReflexStore\|ReflexRule'` → **0** |
| `CognitiveEnergy` (fatigue/zones) | 05-AGENT | Zero-code | `rg 'CognitiveEnergy'` → **0** |
| `EmergentGoal` / `GoalEmergence` (ZPD/IM) | 05-AGENT | Zero-code | `rg 'EmergentGoal\|GoalEmergence'` → **0** |
| 15% contrarian retrieval | 05-AGENT | Zero-code | `rg 'contrarian'` → **0** |
| Hindsight relabeling | 06-MEMORY Ph2 / 07 L3 | Zero-code | `rg 'hindsight'` → **0** |
| Resonator Networks | 06-MEMORY §7 | Zero-code | `rg 'Resonator'` → **0** |
| `RecursiveSafetyMonitor` | 07-LEARNING L4 | Zero-code | `rg 'RecursiveSafetyMonitor'` → **0** |
| `StructuralChange` (L4 adaptation) | 07-LEARNING L4 | Zero-code | `rg 'StructuralChange'` → **0** |
| `AutonomyLevel` (0-5 per Space) | 07-LEARNING / 20 | Zero-code | `rg 'AutonomyLevel'` → **0** |
| `KnowledgeConfig` / context scoping (Gap 5) | 27-ORCHESTRATOR | Zero-code | `rg 'KnowledgeConfig\|context_scoping'` → **0** |
| `apply_rustc_fixes` (Gap 2 autofix) | 27-ORCHESTRATOR | Zero-code | `rg 'apply_rustc_fixes'` → **0** |
| `roko plan enrich` command (§13) | 27-ORCHESTRATOR | Zero-code | `rg 'PlanCmd::Enrich'` → **0** |

---

## PLATFORM band — docs/v2 08-19 · docs/v2-depth 12-18

### Built / Partial

| Concept | Source doc | Status | Grep evidence |
|---|---|---|---|
| Extension trait, 8 layers, ExtensionChain | 12-EXTENSIONS / 08-ext | Built (16/22 hooks) | `roko-core/src/extension.rs:168-596` (760 LOC); consumed runner/serve/do |
| ConnectorRegistry/Kind/Health + routes | 11-CONNECTIVITY | Built | `roko-core/src/connector.rs`; `roko-serve/src/routes/connectors.rs:24` |
| StateHub + projections + `/metrics` | 09/15-TELEMETRY | Built | `roko-runtime/src/state_hub.rs`; `roko-serve/src/routes/metrics.rs:1-35` |
| 4-layer config merge + hot reload | 19-CONFIG / 14-config | Built | `roko-core/src/config/loader.rs:33-128` |
| JWT/JWKS + API-key scopes + team roles | 17-AUTH | Built | `jwks.rs:1-249`; `routes/middleware.rs:166`; `routes/team.rs:171,274` (durable store) |
| gateway ThinkingCap/Convergence cells | 08-GATEWAY | Built (in ModelCallService) | `roko-agent/src/model_call_service.rs:104,106` |
| FeedRegistry + Feed HTTP CRUD | 09-FEEDS / 11 | Built (metadata only) | `roko-core/src/feed.rs:81`; `routes/feeds.rs:29` |
| `Observe`/`Connect`/`Trigger` protocols | 09/11/13 | Partial (test-only impls) | `traits.rs:400,408,420`; `phase1_integration.rs:188,237,294` |
| Cron/FileWatch/Webhook EventSources | 13-TRIGGERS | Partial (🕰️ v1-shape, not TriggerBinding) | `roko-plugin/src/lib.rs:19-33,148,335`; Webhook has manifest kind but **no impl struct** |
| `X402Manager` (x402 payments) | 18-PAYMENTS | Partial (🔌 mock, no 402 middleware) | `roko-chain/src/x402.rs` (958 LOC); no serve route returns 402 |
| ReputationTier pricing / Dispute types | 18-PAYMENTS | Partial (🔌 types, no path) | `identity_economy_markets.rs`; `phase2.rs` |
| `Taint` enum IFC | 16-SECURITY | Partial (no lattice-join propagation) | `roko-core/src/provenance.rs:24`; `engram.rs:81,121` |
| CaMeL dual-LLM | 16-SECURITY / 17-sec | Partial (🔌 built; no CamelTag IFC) | `roko-agent/src/safety/data_llm.rs` (439 LOC) |
| Immune-system types | 16-SECURITY | Partial (🔌 no pipeline graph) | `roko-core/src/immune.rs` (573 LOC); consumed by 4 crates |

### Zero-code (grep → 0)

| Concept | Source doc | Status | Grep evidence |
|---|---|---|---|
| **All of 10-GROUPS**: `Group`/`GroupIdentity`/`GroupContextBidder`/`CoordinationMode`/`RelayRoom` | 10-GROUPS | Zero-code | `rg 'struct Group\b\|GroupIdentity\|GroupContextBidder\|CoordinationMode\|RelayRoom'` → **0** |
| **Lens system**: `Lens`/`LensScope`/`CollectorLens`/`TransformLens`/`ExportLens` + 11 named lenses | 15-TELEMETRY / 09 / 21 / 03 | Zero-code | `rg 'trait Lens\|struct .*Lens\|LensScope\|CollectorLens\|TransformLens\|ExportLens'` → **0** (nearest reality: hand-rolled `MetricRegistry` `roko-core/src/obs/metrics.rs:263`) |
| `CamelTag` IFC (propagation/no-laundering) | 12-EXTENSIONS / 16 | Zero-code | `rg -ril 'CamelTag'` → **0** |
| **MPP** streaming sessions (`MppSession`/`Micropayment`) | 18-PAYMENTS | Zero-code | `rg 'MppSession\|Micropayment'` → **0**; only a doc-comment "MPP session" field `identity_economy_identity.rs:1149` (not a type) |
| Gateway shaping cells: `CacheLookupCell` L1/L2, `CacheStoreCell`, `ToolPruneCell`, `OutputBudgetCell` | 08-GATEWAY | Zero-code | `rg 'CacheLookup\|semantic_cache\|ToolPrune\|OutputBudget'` → **0** |
| `RecipeCell` / `FeedPublisherExt` (feed-as-Pulse) | 09-FEEDS | Zero-code | no feed data flows as a Pulse; registry metadata-only |
| `ConnectorManifest` / `ReconnectStrategy` / finality oracle | 11-CONNECTIVITY | Zero-code | `rg 'ConnectorManifest\|ReconnectStrategy'` → **0** |
| `TriggerBinding` runtime + `SignalPattern` + `.roko/triggers/` + `roko trigger` CLI | 13-TRIGGERS | Zero-code | manifest `TriggerDef` parses but nothing drives it; no engine/persistence/policy |
| Builtin-Cell catalog registry (kebab names) | 14-TOOLS | Zero-code | no name→Cell registry; tools are `ToolDef` not Cells |
| `Capability<T>` 3-layer stack (decl∩allow∩grant) | 16-SECURITY / 02 | Zero-code | flat tiers only `safety/capabilities.rs`; no 3-layer intersection |
| 5-head lexicographic corrigibility pipeline | 16-SECURITY | Zero-code | no ordered Verify-head pipeline |
| Config-as-Signal (versioned/demurrage/L4) | 19-CONFIG / 14 | Zero-code | no `Kind::Config` Signal wrapping; 0 registered migrations |
| Device flow / OS keychain | 17-AUTH | Zero-code | no `/auth/device/*`; creds in `~/.roko/credentials.json` |

---

## ECOSYSTEM band — docs/v2 20-26, 28 · docs/v2-depth 19-22 + guides

### Built / Partial

| Concept | Source doc | Status | Grep evidence |
|---|---|---|---|
| CLI + ~80-module TUI + 288 serve routes | 20-SURFACES / 16 | Built (10 tabs ≠ spec 7) | `roko-cli/src/tui/tabs.rs:10-31`; 288 raw `.route(` |
| deploy railway/fly/docker + daemon + worker | 25-DEPLOYMENT | Built | `roko-cli` deploy/daemon cmds; daemon dream loop `daemon.rs:339` |
| Cold-substrate archival timer | 25 / 26 | Built (newly wired) | `roko-serve/src/lib.rs:344,2096-2134`; commit `8f3497063` |
| bench harness (23 `/bench` routes) | 23-ARENAS (nearest) | Built (unnamed) | `roko-cli/src/bench.rs`; `roko-learn/src/pareto.rs` |
| roko-chain: 3 registries + witness + TraceRank + ISFR + korai + futures | 22/24 / 18-registries | Partial (🔌 mock-backed, 31 modules) | `roko-chain/src/{agent_registry,reputation_registry,validation_registry,witness,trace_rank,isfr_keeper}.rs` |
| `OnChainAgentRegistry` serve→contract bridge | 22-REGISTRIES | Partial (🟡 behind `[chain]` config) | `roko-serve/src/routes/agents.rs:512` (2 refs) |
| ISFR keeper + 4 real sources | 22/24-DEFI | Partial | `roko-cli/src/commands/isfr.rs`; `roko-chain/src/isfr_sources/` |
| 13 Foundry contracts | 22-REGISTRIES | Partial (🔌 undeployed) | `contracts/src/`; `contracts/script/Deploy.s.sol` |
| `nelson_siegel` rate curves | 24-DEFI | Partial (🔌 no route) | `roko-chain/src/nelson_siegel.rs` |
| knowledge backup/restore/sync (mesh) | 25 §5 | Partial (no merkle/CRDT/.roko-brain) | `main.rs:918-964`; `rg 'merkle\|crdt'` neuro → **0** |
| cross-cut behaviors in runner | 26-CROSS-CUTS | Built (behavior) / ❌ (functor shape) | `runner/event_loop.rs` direct calls |

### Zero-code (grep → 0)

| Concept | Source doc | Status | Grep evidence |
|---|---|---|---|
| **All of 23-ARENAS**: `Arena`/`Eval`/`Bounty`/`ArenaRegistry` + 8 arenas + 37 routes | 23-ARENAS / 19-arenas | Zero-code | `rg '\bArena\b'` → **1**, and it's a *comment* in `bench.rs:82`; no arena/eval/bounty type or route |
| `ClearingHouse` + yield perps + `VenueAdapter`/`DeFiRiskEngine`/`TradingReflect` | 24-DEFI | Zero-code | `rg 'ClearingHouse'` → **1**, a *comment* in `apps/mirage-rs/src/rpc.rs:844`; all five types → 0 |
| `CrossCutFunctor` + endofunctor/nat-transformation formalism | 26-CROSS-CUTS | Zero-code | `rg 'CrossCutFunctor\|pre_enrich'` → **0** |
| 5 surface contracts: `InboxCategory`/`UrgencyLevel`/`AutonomyConfig`/`FlowSummary` | 20-SURFACES §3 | Zero-code | `rg` across crates → **0** |
| Package marketplace / `roko market` CLI (14 subcmds) + publish/install Graphs | 21-MARKETPLACE | Zero-code | `rg 'Market' main.rs` → **0**; no registry/publish/install/lockfile |
| `roko inbox` / `roko autonomy` CLI (levels 0-4) | 20-SURFACES §4.1 | Zero-code | `rg 'Inbox\|Autonomy'` → **0** |
| Event indexer + `roko indexer start` + `:6678` API | 22-REGISTRIES §11.2 | Zero-code | no `Indexer` struct/mod; no `indexer` clap cmd |
| ZK-HDC proofs / gossip networking | 22-REGISTRIES | Zero-code | `rg 'zk'` chain → 0; `phase2.rs` gossip is stubs |
| WASM packaging (fuel/ABI/wit-bindgen; SPI T4) | 25-DEPLOYMENT / 21 | Zero-code | `rg 'wasmtime\|wasm32'` all Cargo.toml → **0** |
| Merkle-CRDT brain export / `.roko-brain` bundle | 25-DEPLOYMENT §5 | Zero-code | `rg 'merkle\|crdt'` neuro → **0** |
| Self-healing supervisor Graph / `ROKO_SUPERVISOR_AUTOFIX` | 25-DEPLOYMENT §3.6 | Zero-code | `rg 'SUPERVISOR_AUTOFIX\|CrashReport'` → **0** |
| `[defi]` config section / `/api/defi/*` (17 routes) | 24-DEFI §8/§12 | Zero-code | no `defi` config; actual is `/api/isfr/*` (4) + `/api/chain/*` (7) |

---

## Terms not yet in code — glossary (spec term → nearest code reality)

The newest doc layer (v2-depth) leads with vocabulary the code never adopted. A reader who greps for these symbols finds nothing; here is the nearest real thing so no depth doc reads as if its subject ships.

| Spec term (as written in docs) | Nearest code reality | Note |
|---|---|---|
| **Signal** (the noun) | `Engram` struct (`engram.rs:63`), `Signal` is an alias (`signal.rs:6`) | Noun inversion never happened; ~50/50 import split across crates |
| **Block** (v2-depth Cell rename) | **no type**; concept is `Cell` — and there are *two* `Cell` traits (`roko-core/cell.rs:91`, `roko-graph/cell.rs:74`) | Three-way drift (code=Cell, v2=Cell, v2-depth=block), zero code motion |
| **Cell** (single canonical trait) | two incompatible traits + a third de-facto `NodeOutput` family | Pick-canonical still open (85 Q2) |
| **Lens** / Lens pipeline | hand-rolled `MetricRegistry` (`obs/metrics.rs:263`, 56 refs) + `StateHub` + `/metrics`; ad-hoc, not composable Lens cells | The single widest zero-code gap; blocks 5 specs |
| **Pulse** (canonical event kernel) | `Pulse`/`PulseBus` exist (`pulse.rs:75`, `pulse_bus.rs:29`) but consumed only by roko-runtime + `roko-gate/verdict_publisher.rs` | Multiple buses (runtime EventBus, server bus, learn bus); not one kernel. **Not ISFR-only** (corrects briefing) |
| **Verdict** (4-role: reward/criteria/evidence) | v1 `Verdict{passed,score,reason}` (`verdict.rs:51`); scalar `reward` sits on a separate `Outcome` (`verdict.rs:200`) | The reward primitive exists on the wrong type |
| **Group** / Arena / MPP / CamelTag / ClearingHouse / CrossCutFunctor | none | Pure vocabulary — see zero-code rows |
| **MAP-Elites** (docs place in learn) | `roko-dreams/src/phase2/evolution.rs`; `skill_library.rs:1807` is EvoSkills (success-rate), not QD | Mis-attribution corrected |
| **WisdomGate** (docs/85 said absent) | **exists** `roko-orchestrator/src/coordination.rs:743` (5 refs), legacy-orchestrate only | Correction: prior kernel audit's "grep→0" was stale |
| **demurrage** (Gesell per-Kind law) | flat 0.005/h decay in neuro; `balance` field on Engram; `RuntimeKnowledgeLifecycle` has 0 external callers | Taxes without income |
| **EFE routing** | LinUCB `CascadeRouter` + a small `active_inference.rs` tier-selector | Spec says EFE "replaces LinUCB"; didn't happen |

---

## Status tally

| Band | Built | Partial | Renamed-only | Zero-code |
|---|---|---|---|---|
| Kernel (01-07, 27) | 12 | 26 | 5 | 30 |
| Platform (08-19) | 7 | 6 | 0 | 13 |
| Ecosystem (20-26, 28) | 5 | 6 | 0 | 12 |
| **Total (~129 concepts)** | **~24** | **~52** | **~5** | **~55** |

> Counting note: whole "bands" (10-GROUPS, 23-ARENAS) are counted as one clustered concept each in some rows and expanded in others; the ±2 wobble is in the Zero-code column. The load-bearing conclusion is invariant to it.

---

## Spec-debt burn-down — prioritized ordering

Ordering key: **LB** = load-bearing for the v2 execution contract (many downstream concepts blocked); **ASP** = aspirational / ecosystem (self-contained, no v2 concept depends on it). Within LB, ordered by fan-out.

### Tier 0 — the default path is a no-op (Partial, not zero-code, but strictly gates everything)

- [ ] **`TaskExecutorCell` dry-run default** (`task_executor.rs:20`) + clap `--engine graph` default (`main.rs:1361`) contradicting `#[default] RunnerV2` (`main.rs:1301`) + `roko resume` hardcoding Graph (`main.rs:2699`). Not zero-code, but it is the P0 that makes the advertised self-hosting loop print SUCCESS while doing nothing. **Fix before any zero-code work is worth doing.**

### Tier 1 — load-bearing zero-code (build these; they unblock whole doc families)

1. [ ] **`Lens` protocol** (15/09/03/10/16/20/21) — **top load-bearing zero-code item.** 0 hits; blocks telemetry-as-Lens, conductor-as-Lens, c-factor-as-Lens, surface composition, marketplace lenses. Minimum: `trait Lens` + `LensScope` + wrap `CFactorSummary`/`efficiency`/`MetricRegistry` as the first lenses feeding StateHub.
2. [ ] **4-role Verdict** — `reward: f64` + `CriterionResult` (hard/soft) + `EvidenceKind` (02-CELL). 0 hits for the latter two. Blocks routing-reward, hindsight relabel, pre-action veto, reputation. Reward primitive already on `Outcome` — migrate it onto `Verdict`.
3. [ ] **`SignalId`/`SignalRef` + `Signal::to_pulse` + Pulse `source/lineage_hint/trace_id` + Bus `replay_since`** (01-SIGNAL). 0 hits. Prerequisite for spec-shaped predict-publish-correct (join-by-lineage) and every Lens that reads Pulses.
4. [ ] **`Engine` API + `NodeKind` + conditional-edge eval** (03/04). 0 hits for Engine/NodeKind. Prerequisite for the Graph path to ever be a real runtime rather than a sequential interpreter.
5. [ ] **Production `Trigger`/`Observe`/`Connect` impls** (11/13/15). Traits exist; impls test-only. Adapt roko-plugin EventSources to the protocol.
6. [ ] **Taint lattice ops** (`join`/`flows_to`) + `Capability<T>` 3-layer stack (16). Enum exists; propagation/intersection are 0 hits. Load-bearing for the whole security composition story.

### Tier 2 — load-bearing but narrower (finish the Partial engines)

7. [ ] Demurrage income side — call `RuntimeKnowledgeLifecycle` reinforce from the runner; enable `hdc` feature so fingerprints stop being null.
8. [ ] Wire `DeltaConsumer` → `roko-dreams::replay` so the dream cycle runs on the delta cadence; adopt heartbeat/`CorticalState` in a non-legacy home.
9. [ ] Consult adaptive gate thresholds from the runner rung path; wire `AllenRelation`/`TemporalIndex` to episode boundaries.
10. [ ] Hindsight relabeling (0 hits) inside roko-dreams NREM input — the last missing L3 organ.

### Tier 3 — aspirational / ecosystem (decide: implement, feature-gate, or mark deferred)

11. [ ] **10-GROUPS** (0 code, 933-line spec) — largest spec/code delta in the platform band. Implement minimal Group = relay-topic + membership, or tag the spec deferred like 19-arenas.
12. [ ] **23-ARENAS / 24-DEFI trading stack** — pure vapor (`Arena`/`ClearingHouse` are comments). Tag "research/futures" like 21-roadmap so they stop inflating coverage.
13. [ ] **MPP, CamelTag, CrossCutFunctor, package marketplace, WASM/edge, Merkle-CRDT brain, device flow, indexer** — self-contained; no v2 concept depends on them. Move to explicit phase gates in 28-ROADMAP with no code claim.
14. [ ] **L4 structural adaptation** (`RecursiveSafetyMonitor`/`StructuralChange`/`AutonomyLevel`, all 0) — deliberately last; needs Tier-1 Verdict + Lens + approval CLI first.

---

## Checklist (doc-hygiene, for the pack)

- [ ] Publish the "terms not yet in code" glossary above at the pack navigation root so every v2-depth reader hits it before any depth doc.
- [ ] Correct 85-V2-COVERAGE-KERNEL: `WisdomGate` is **not** 0 hits — it exists at `coordination.rs:743` (built, legacy-only).
- [ ] Correct the briefing/15-doc "PulseBus only ISFR uses it": consumers are `roko-runtime` + `roko-gate/verdict_publisher.rs`.
- [ ] Retag every "X-as-Lens" doc (09/03/10/16/21) as design-only until `trait Lens` exists.
- [ ] Tag 10-GROUPS, 23-ARENAS, 24-DEFI-trading, 26-formalism, MPP as "unfunded / research" in their section headers.
- [ ] Reconcile the Signal↔Engram and Cell↔block naming direction before any depth doc claims either ships.

## Verification commands (re-run to refresh this ledger)

```bash
cd /Users/will/dev/nunchi/roko/roko
# Zero-code sentinels (expect 0 or comment-only):
rg -c 'trait Lens|struct .*Lens|LensScope' crates/ apps/
rg -c 'SignalId|SignalRef|to_pulse|EvidenceKind|CriterionResult' crates/ apps/
rg -c 'struct Group\b|GroupContextBidder|RelayRoom|CamelTag|MppSession' crates/ apps/
rg -c 'RecursiveSafetyMonitor|StructuralChange|AutonomyLevel|T0Probe|hindsight' crates/ apps/
rg -c 'CrossCutFunctor|ArenaRegistry|ClearingHouse' crates/ apps/
rg -n '\bArena\b|ClearingHouse' crates/ apps/    # confirm the 1-hit each is a comment
# Corrections (expect non-zero):
rg -n 'WisdomGate' crates/roko-orchestrator/src/coordination.rs
rg -l 'PulseBus' crates/ apps/ | grep -v roko-core
# Nearest-reality anchors:
rg -n 'struct MetricRegistry' crates/roko-core/src/obs/metrics.rs
rg -n 'pub trait Cell' crates/roko-core/src/cell.rs crates/roko-graph/src/cell.rs
```
