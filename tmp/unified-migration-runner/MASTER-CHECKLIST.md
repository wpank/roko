# Unified Migration — Master Checklist (v2 Parallel)

> 123+ batches across 6+ phases, driven by `tmp/unified-migration/` phase files.
> **v2**: 4-agent parallel execution with dynamic mega-batching.
> Updated by the runner as batches execute.

## Mega-Batch Schedule (Dynamic)

Mega-batches are computed dynamically at startup based on task count and phase grouping.
Run `bash tmp/unified-migration-runner/run.sh --list` for the current schedule.

### 4-Agent Crate Partitioning

| Agent | Crates | Role |
|---|---|---|
| A (kernel) | roko-core, roko-primitives, roko-fs, roko-std | Core types, exclusively owns roko-core writes |
| B (protocols) | roko-gate, roko-compose, roko-learn, roko-neuro | Protocol implementations |
| C (runtime) | roko-orchestrator, roko-runtime, roko-conductor, roko-dreams, roko-daimon | Execution engine |
| D (surface) | roko-cli, roko-serve, roko-agent, roko-agent-server, roko-mcp-*, roko-chain | User-facing surfaces |

### Estimated Timeline (4 agents)

```
Hour | Agent A          | Agent B          | Agent C          | Agent D
-----|------------------|------------------|------------------|------------------
0:00 | MB1-A (baseline) | --               | MB1-C (wiring)   | --
0:30 | SYNC-1           |                  |                  |
0:35 | MB2 (aliases)    | --               | --               | --
1:35 | SYNC-2           |                  |                  |
1:40 | MB3-A (demurr)   | MB3-B (heur+EFE) | MB3-C (Bus)      | --
2:40 | SYNC-3 + test    |                  |                  |
2:45 | MB7-A (brain)    | MB5-B (workspace)| MB4-C (Graph)    | MB4-D (agent rt)
3:45 | SYNC-4           |                  |                  |
3:50 | --               | MB6-B (CaMeL)    | MB6-C (corrigib) | MB5-D (surfaces)
4:50 | SYNC-5 + test    |                  |                  |
4:55 | MB8 fix-up       | MB8 fix-up       | MB8 fix-up       | MB8 fix-up
5:25 | FINAL + review   |                  |                  |
```

**Conflict avoidance**: Agent A exclusively owns roko-core writes.
Other agents only READ roko-core. After each SYNC, all agents rebase onto merged source.

---

## Status Legend

| Symbol | Meaning |
|---|---|
| `[ ]` | Pending |
| `[~]` | In progress |
| `[x]` | Done — verified and committed |
| `[!]` | Failed — needs attention |
| `[B]` | Blocked — waiting on depth docs |

## Phase 0: Prep (10 batches)

- [ ] **M001** — Baseline verification snapshot. §0.4
- [ ] **M002** — Create module stubs (signal.rs, cell.rs). §0.3
- [ ] **M003** — Wire ExtensionChain into orchestrate.rs. §0.1
- [ ] **M004** — Wire KnowledgeAdmissionController. §0.1
- [ ] **M015** — Wire ContextualBanditPolicy into CascadeRouter. §0.1
- [ ] **M016** — Audit ConnectorRegistry + FeedRegistry. §0.1
- [ ] **M017** — Fix token accounting in gateway.rs. §0.2
- [ ] **M018** — Parallelize batch requests in gateway.rs. §0.2
- [ ] **M019** — Fix routing context in gateway.rs. §0.2
- [ ] **M020** — Bus module stub + TopicFilter alignment. §0.3

## Phase 1: Kernel (27 batches)

### §1.1 Core Type Renames
- [ ] **M005** — Type alias: Engram → Signal
- [ ] **M006** — Type alias: Substrate → Store
- [ ] **M007** — Type alias: Scorer → ScoreProtocol
- [ ] **M008** — Type alias: Gate → VerifyProtocol
- [ ] **M009** — Type alias: Router → RouteProtocol
- [ ] **M010** — Type alias: Composer → ComposeProtocol
- [ ] **M011** — Type alias: Policy → ReactProtocol
- [ ] **M013** — Verdict.reward field + verify_pre method

### §1.2 Pulse/Bus Kernel
- [ ] **M021** — Pulse struct: align fields with spec
- [ ] **M022** — Bus trait + BroadcastBus: verify spec alignment
- [ ] **M023** — Topic taxonomy constants module
- [ ] **M024** — Wire Bus lifecycle Pulses into execution

### §1.3 React Breaking Change
- [ ] **M025** — React protocol: Pulse-based breaking change

### §1.4 Cell Trait + TypeSchema
- [ ] **M012** — Define Cell trait skeleton
- [ ] **M014** — Define TypeSchema enum

### §1.5 Predict-Publish-Correct
- [ ] **M026** — CalibrationReact Cell implementation
- [ ] **M027** — Wire prediction Pulses for Score/Route/Compose

### §1.6 Demurrage
- [ ] **M028** — Add balance fields to Signal
- [ ] **M029** — Reinforcement kinds with novelty weighting
- [ ] **M030** — Wire demurrage into Store + tier multipliers

### §1.7 Heuristic Kind
- [ ] **M031** — Define Kind::Heuristic + payload
- [ ] **M032** — Wire calibration from Verify verdicts

### §1.8–1.12 New Protocols + Features
- [ ] **M033** — EFE routing: implement + replace LinUCB
- [ ] **M034** — Dream cycle: wire automatic trigger
- [ ] **M035** — Observe protocol + 10 builtin Lenses
- [ ] **M036** — Trigger protocol + Cron/Bus/FileWatch impls
- [ ] **M037** — Connect protocol + refactor MCP connector

## Phase 2: Engine (27 batches)

### §2.1–2.2 Graph Engine
- [ ] **M038** — Graph TOML schema
- [ ] **M039** — Graph loader + TypeSchema validation
- [ ] **M040** — Graph executor + Flow lifecycle
- [ ] **M041** — Failure strategies (Retry/Skip/Fallback)
- [ ] **M042** — Flow snapshot/resume
- [ ] **M043** — Hot Graph resident execution

### §2.4 Migration Tool
- [ ] **M044** — `roko plan migrate` CLI
- [ ] **M045** — Wire Graph executor into `roko plan run`

### §2.5 Agent Runtime
- [ ] **M046** — Type-state Agent (Provisioning/Active/Dreaming/Terminal)
- [ ] **M047** — Vitality model + behavioral phases
- [ ] **M048** — Multi-slot concurrent execution

### §2.6 CognitiveWorkspace
- [ ] **M049** — CognitiveWorkspace with VCG auction
- [ ] **M050** — Section effect tracking (Beta posteriors)

### §2.7 StateHub
- [ ] **M051** — StateHub projection types
- [ ] **M052** — Wire StateHub into TUI/HTTP/WS

### §2.8 Surfaces
- [ ] **M053** — Five named surface protocol contracts
- [ ] **M054** — Workbench tab in TUI
- [ ] **M055** — Agent Inbox in TUI
- [ ] **M056** — Autonomy Slider in TUI

### §2.9–2.11 Rack, SPI, Marketplace
- [ ] **M057** — Rack: Graph + Macros + Slots
- [ ] **M058** — SPI Tier 1: prompt loader
- [ ] **M059** — SPI Tier 2: config deep merge
- [ ] **M060** — SPI Tier 3: declarative tool loader
- [ ] **M061** — SPI Tier 4: WASM Cell runtime
- [ ] **M062** — Cell manifest + local registry
- [ ] **M063** — `roko marketplace` CLI
- [ ] **M064** — Marketplace HTTP routes

## Phase 3: Economy (31 batches)

### §3.1 Extension + CaMeL IFC
- [ ] **M065** — Extension: formalize 8 layers
- [ ] **M066** — CaMeL: define CamelTag types
- [ ] **M067** — CaMeL: tag propagation rules
- [ ] **M068** — CaMeL Monitor Verify Cell

### §3.2 Corrigibility
- [ ] **M069** — 5-head corrigibility Verify chain
- [ ] **M070** — RecursiveSafetyMonitor
- [ ] **M071** — Wire corrigibility into Graph executor

### §3.3 L4 Self-Evolution
- [ ] **M072** — Structural change proposals
- [ ] **M073** — Approval workflow via Inbox
- [ ] **M074** — Wire L4 into dream cycle
- [ ] **M075** — Variance Inequality enforcement

### §3.4 On-Chain Registries
- [B] **M076** — Finalize Solidity contracts
- [B] **M077** — Deploy to Nunchi testnet
- [B] **M078** — Rust clients for all registries
- [B] **M079** — Wire passport into Agent startup
- [B] **M080** — Wire knowledge publication
- [B] **M081** — Event indexer

### §3.5 Arena System
- [B] **M082** — Arena types
- [B] **M083** — 7-step flywheel
- [B] **M084** — Eval protocol
- [B] **M085** — Bounty system
- [B] **M086** — Arena + Bounty HTTP routes
- [B] **M087** — Cross-arena transfer detection

### §3.6 Brain Export/Import
- [ ] **M088** — Brain export format
- [ ] **M089** — Brain export with filters
- [ ] **M090** — Brain import with decay
- [ ] **M091** — Merkle-CRDT sync

### §3.7–3.8 Knowledge Sharing + Deployment
- [ ] **M092** — Knowledge broadcast via relay
- [B] **M093** — On-chain knowledge discovery
- [B] **M094** — WASM compilation target
- [B] **M095** — Agent execution tiers

---

## Phase 4: Memory and Knowledge (17 batches)

> Depth docs: `tmp/unified-depth/11-memory/01-11`. These batches restructure the
> knowledge store (roko-neuro), HDC operations (roko-primitives), dream cycle
> (roko-dreams), and stigmergic coordination to align with unified Signal/Cell/Graph
> primitives. Dependencies flow from Phase 1 kernel (Signal alias, Store alias,
> Cell trait, Pulse/Bus) and Phase 1 features (dream trigger, heuristic Kind).

### Knowledge as Signal (01-knowledge-as-signal.md)
- [ ] **M096** — Knowledge Kind mapping to Signal Kind system. Maps KnowledgeKind variants to unified Kind enum.
- [ ] **M097** — Neuro Store as Signal Store adapter. Wraps KnowledgeStore with Store protocol interface.

### HDC Algebra and Retrieval (02-hdc-algebra-and-retrieval.md)
- [ ] **M098** — HDC operations as Cell implementations. Bind/Bundle/Permute/Similarity as composable Cells.
- [ ] **M099** — Three-tier HDC search pipeline. HDC pre-filter, keyword re-rank, optional dense re-score.

### Knowledge Lifecycle Loop (03-knowledge-lifecycle-loop.md)
- [ ] **M100** — Distillation as Pipeline Graph (D1/D2/D3). Refactors sequential batch functions into composable stages.
- [ ] **M112** — Calibration receipts and predict-publish-correct. Wires feedback into distillation stages.

### AntiKnowledge and Immunity (04-antiknowledge-and-immunity.md)
- [ ] **M101** — Immune Verify Pipeline and memetic fitness Score. Three-stage verification with fitness scoring.

### Cross-Domain Transfer (05-cross-domain-transfer.md)
- [ ] **M102** — Federation Spaces and confidence Functor. Trust boundaries with confidence attenuation.

### Dream Cycle as Loop (06-dream-cycle-as-loop.md)
- [ ] **M103** — Dream Loop Graph structure (NREM/REM/Integration). Decomposes monolithic DreamCycle into graph.
- [ ] **M104** — Dream Trigger Cell scheduling. Idle timer, episode threshold, manual triggers.

### Replay and Counterfactual Cells (07-replay-and-counterfactual-cells.md)
- [ ] **M105** — NREM replay Cells (selection/sequencing/extraction/ranking). Four-stage replay pipeline.
- [ ] **M106** — REM counterfactual Cells (generation/testing/gap-finding). Three-stage imagination pipeline.

### Hypnagogia and Creativity (08-hypnagogia-and-creativity.md)
- [ ] **M107** — Hypnagogia Pipeline (anti-correlation/novelty/compose/verify). Four-cell creative fragment pipeline.

### Consolidation and Staging (09-consolidation-and-staging.md)
- [ ] **M108** — Staging Store partition and SHY renormalization. Tag-based partitions + homeostatic scaling.

### Threat Simulation and Nightmares (10-threat-simulation-and-nightmares.md)
- [ ] **M109** — Threat simulation Verify Cells (FMEA/FTA/nightmare). Threat scoring + fault tree + nightmare detection.

### Stigmergy as Bus (11-stigmergy-as-bus.md)
- [ ] **M110** — Pheromone Pulse types on Bus. Success/difficulty/claim/breadcrumb/attraction/repulsion kinds.
- [ ] **M111** — Stigmergic Route Cell. Pheromone-influenced routing score adjustments.

---

## Phase 5: Conductor and Affect (14 batches)

> Depth docs: `tmp/unified-depth/07-agent-runtime/14-21`. These batches refactor the
> conductor supervision pipeline into Verify Cells and Pipeline Graphs, add diagnosis
> routing and stuck detection Lens Cells, implement adaptive threshold learning, and
> wire the daimon affect engine (PAD metadata, appraisal pipeline, behavioral states,
> somatic landscape, and emotional contagion) into the dispatch path.

### Conductor Pipeline (14-conductor-as-verify-pipeline.md)
- [ ] **M116** -- Refactor conductor watchers as Verify Cells. Add Verdict return with metric+remediation to all 10 watchers.
- [ ] **M117** -- Conductor Pipeline Graph and Route Cell. Compose 10 watcher Cells into a pipeline with DecisionRouter.

### Circuit Breaker (15-circuit-breaker-and-interventions.md)
- [ ] **M118** -- Circuit breaker state-machine Cell with AIMD. Three-state (Closed/Open/HalfOpen) + Holt-Winters predictive tripping + AIMD concurrency.

### Diagnosis and Stuck Detection (16-diagnosis-and-stuck-detection.md)
- [ ] **M119** -- Diagnosis Route Cell and error categories. 20 ErrorKind variants, 34 substring patterns, 9 InterventionKind actions.
- [ ] **M120** -- Stuck detection Lens Cells and aggregation. 6 Lens Cells (OutputLoop, NoProgress, GateLoop, CompileLoop, EmptyOutput, ExcessiveRetries) + StuckAggregate.

### Adaptive Supervision (17-adaptive-supervision-loop.md)
- [ ] **M121** -- Self-model accuracy Lens and Yerkes-Dodson pressure. 5 accuracy metrics, Brier score tracker, pressure-to-sensitivity modifier, flow detection.
- [ ] **M122** -- Threshold adaptation with predict-publish-correct. Beta-posterior per-watcher thresholds, Yerkes-Dodson pressure scaling, persistence.

### Affect Engine (18-affect-as-functor.md, 19-behavioral-states-and-routing.md)
- [ ] **M123** -- PAD as Signal metadata and PadContext type. Confidence field, AffectOctant enum, octant() classifier, stamp/read helpers.
- [ ] **M124** -- Appraisal Pipeline and ALMA temporal model. 8-step pipeline, 3-layer EMA (emotion/mood/personality), prospect theory 2x asymmetry.
- [ ] **M125** -- Behavioral state Score Cell with hysteresis. 6 states, asymmetric thresholds, 10-tick dwell, RoutingModulation.
- [ ] **M126** -- Affect Functor for Compose enrichment. AffectEnrichment + VcgAffectModulation, pre/post enrichment, wired into orchestrate.rs dispatch.

### Somatic Landscape (20-somatic-landscape.md)
- [ ] **M127** -- Somatic Store with dual k-d tree and contrarian Functor. Immutable+mutable trees, 15% contrarian retrieval, resource pressure compression.

### Collective Contagion (21-collective-contagion.md)
- [ ] **M128** -- Contagion accumulator and attenuation Functor. PAD attenuation (Px0.3, Ax0.3, Dx0.0), 6h decay, susceptibility modifier, SomaticField stub.

### Integration
- [ ] **M129** -- Wire affect-modulated routing into orchestrate.rs. Behavioral state -> CascadeRouter bias, somatic query at dispatch, appraisal feedback loop.

---

## Phase 6: Chain and Registries (11 batches)

> Depth docs: `tmp/unified-depth/18-registries/01-06`. All batches are **[B] Blocked** because
> chain deployment is Tier 6 / Phase 3+. The batch definitions exist so they are ready when
> unblocked. Dependencies flow from Phase 1 kernel (Cell trait, protocol aliases) and
> Phase 2 engine (Graph executor) into the chain domain.

### Chain as Domain Plugin (01-chain-as-domain-plugin.md)
- [B] **M131** — ChainConnector Cell (Connect protocol wrapper). Wraps ChainClient+ChainWallet behind Connect protocol.
- [B] **M132** — Registry Store Cells (Identity, Reputation, Validation). Three registries as Store Cells with optional on-chain backing.

### HDC On-Chain and Verification (02-hdc-on-chain-and-verification.md)
- [B] **M133** — HDC Precompile Cell (on-chain HDC operations). Three-tier search Pipeline: Bloom -> Approximate -> Exact.
- [B] **M134** — Verifiable HDC Verify Cells (ZK/Optimistic/TEE/Binius). Four interchangeable Verify Cells + Route Cell.

### Job Market and Hiring (03-job-market-and-hiring.md)
- [B] **M135** — Job marketplace Graph types (posting, matching, hiring). JobPostCell + CapabilityMatchCell + three HiringRoute Cells.
- [B] **M136** — Escrow, Settlement, and Dispute Resolution Cells. EscrowStoreCell + SettlementCell + JuryVerifyCell.

### Reputation and Peer Scoring (04-reputation-and-peer-scoring.md)
- [B] **M137** — Reputation Score Cell with EMA + TraceRank Pipeline. ReputationScoreCell wrapping ReputationRegistry + 3-stage Pipeline.

### Chain Witness and Triage (05-chain-witness-and-triage.md)
- [B] **M138** — ChainWitnessFeed Cell (Connect+Trigger+Store). Binary Fuse T0 probe, gap detection, block ingestion.
- [B] **M139** — Triage Pipeline (4-stage Score/Observe/Compose). RuleClassifier + MIDAS-R + Enricher + CuriosityScorer.

### Payments and Settlement (06-payments-and-settlement.md)
- [B] **M140** — Payment Connect Cells (x402 + State Channels). Two payment Connect Cells + PaymentRouteCell.
- [B] **M141** — ISFR Score Cell + ClearingHouse Pipeline. Netting, trust-weighted batching, multi-level dispute.

---

## Summary

| Phase | Total | Done | Failed | Blocked | Pending |
|---|---|---|---|---|---|
| Phase 0 | 10 | 0 | 0 | 0 | 10 |
| Phase 1 | 27 | 0 | 0 | 0 | 27 |
| Phase 2 | 27 | 0 | 0 | 0 | 27 |
| Phase 3 | 31 | 0 | 0 | 17 | 14 |
| Phase 4: Memory | 17 | 0 | 0 | 0 | 17 |
| Phase 5: Conductor | 14 | 0 | 0 | 0 | 14 |
| Phase 6: Chain | 11 | 0 | 0 | 11 | 0 |
| **Total** | **137** | **0** | **0** | **28** | **109** |

Last updated: 2026-04-26 (added Phase 5: Conductor and Affect)
