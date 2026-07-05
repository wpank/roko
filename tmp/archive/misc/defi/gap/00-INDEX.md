# DeFi Gap Analysis — Master Index

> **Generated**: 2026-04-23
> **Scope**: Roko DeFi capabilities vs bardo PRD specifications + offchainservices-agent patterns
> **Codebase**: `/Users/will/dev/nunchi/roko/roko/` (18 crates, ~177K LOC)
> **PRD Source**: `/Users/will/dev/nunchi/roko/bardo-backup/prd/` (52+ documents)
> **Reference Agent**: `/Users/will/dev/nunchi/offchainservices-agent/` (production Hyperliquid bot)

---

## Legend

| Marker | Meaning |
|--------|---------|
| `[x]` | Built and wired — works at runtime |
| `[~]` | Built but unwired — code exists, not called from runtime path |
| `[p]` | Partial — some functionality exists, significant gaps remain |
| `[ ]` | Not built — no implementation in roko |
| `[n/a]` | Not applicable — PRD concept doesn't map to roko architecture |
| **S** | Small — <1 day, single file change |
| **M** | Medium — 1-3 days, 2-5 files |
| **L** | Large — 3-7 days, new module or significant refactor |
| **XL** | Extra Large — 1-2 weeks, new crate or cross-cutting change |

## Document Map

Docs 01-10 are **agent-executable work batches** — self-contained work orders with code skeletons, acceptance criteria, and verification commands. Docs 00, 11-13 are support documents.

| # | File | Domain | Batches | Key Question |
|---|------|--------|---------|-------------|
| 0 | [`00-INDEX.md`](00-INDEX.md) | Meta | — | How is this analysis structured? |
| 1 | [`01-GAP-CHAIN-RUNTIME.md`](01-GAP-CHAIN-RUNTIME.md) | Chain | 6 (1.1–1.6) | Can roko connect to chains and process events? |
| 2 | [`02-GAP-TOOLS.md`](02-GAP-TOOLS.md) | Tools | 5 (2.1–2.5) | Can agents execute DeFi operations? |
| 3 | [`03-GAP-TA-INDICATORS.md`](03-GAP-TA-INDICATORS.md) | Analysis | 6 (3.1–3.6) | Can roko analyze market data? |
| 4 | [`04-GAP-SAFETY.md`](04-GAP-SAFETY.md) | Safety | 4 (4.1–4.4) | Are DeFi operations safe and bounded? |
| 5 | [`05-GAP-AGENT-ARCHETYPES.md`](05-GAP-AGENT-ARCHETYPES.md) | Agents | 3 (5.1–5.3) | Can specialized trading agents be defined? |
| 6 | [`06-GAP-HEARTBEAT.md`](06-GAP-HEARTBEAT.md) | Runtime | 4 (6.1–6.4) | Does the tick loop support real-time trading? |
| 7 | [`07-GAP-LEARNING-LOOPS.md`](07-GAP-LEARNING-LOOPS.md) | Learning | 5 (7.1–7.5) | Does the system learn from trading outcomes? |
| 8 | [`08-GAP-DAIMON-INTEGRATION.md`](08-GAP-DAIMON-INTEGRATION.md) | Affect | 3 (8.1–8.3) | Does affect modulate trading behavior? |
| 9 | [`09-GAP-DREAMS-INTEGRATION.md`](09-GAP-DREAMS-INTEGRATION.md) | Dreams | 2 (9.1–9.2) | Can the system replay and discover strategies offline? |
| 10 | [`10-GAP-NEURO-HDC.md`](10-GAP-NEURO-HDC.md) | Knowledge | 2 (10.1–10.2) | Does durable knowledge inform market decisions? |
| 11 | [`11-CHECKLIST-IMPLEMENTATION.md`](11-CHECKLIST-IMPLEMENTATION.md) | Plan | — | Batch execution order (topological sort, parallel groups) |
| 12 | [`12-OFFCHAIN-AGENT-MAPPING.md`](12-OFFCHAIN-AGENT-MAPPING.md) | Reference | — | How does the production agent map to roko? |
| 13 | [`13-BENCHMARKS.md`](13-BENCHMARKS.md) | Targets | — | What performance targets must be met? |

**Total**: 41 batches (0.1 + 1.1–10.2) across docs 01-10, ~185 work items.

> **Batch 0.1** (Mirage-rs integration toolkit) lives in doc 01 as a foundation section. It has no dependencies and unblocks mirage-based testing in all other batches.

## Cross-Reference Matrix

Each gap document (01-10) touches multiple roko crates. This matrix shows which crates are primary (`●`) and secondary (`○`) references for each document.

| Crate | 01 | 02 | 03 | 04 | 05 | 06 | 07 | 08 | 09 | 10 |
|-------|----|----|----|----|----|----|----|----|----|----|
| roko-chain | ● | ● | ○ | ○ | | ○ | | | | |
| roko-agent | | | | ○ | ● | | | | | |
| roko-learn | | | ● | | | | ● | | | ○ |
| roko-gate | | | | ● | | | ○ | | | |
| roko-compose | | | | | ○ | | | | | |
| roko-conductor | | | | ○ | | ● | | | | |
| roko-daimon | | | | | | | | ● | ○ | |
| roko-dreams | | | | | | | | ○ | ● | |
| roko-neuro | | | | | | | | | ○ | ● |
| roko-primitives | | | ○ | | | | | | | ● |
| roko-runtime | ○ | | | | | ○ | | | | |
| roko-std | | ○ | | | | | | | | |
| roko-core | ○ | ○ | ○ | ○ | ○ | ○ | ○ | ○ | ○ | ○ |
| roko-cli | | | | | | | | | | |
| roko-serve | | | | | | | | | | |

## PRD Cross-Reference

| PRD Section | Maps To |
|-------------|---------|
| `14-chain/` (9 files) | Doc 01 (chain runtime), Doc 02 (chain tools) |
| `07-tools/` (14 files) | Doc 02 (tools), Doc 05 (agent skills) |
| `23-ta/` (11 files) | Doc 03 (TA indicators), Doc 10 (HDC encoding) |
| `10-safety/` (5 files) | Doc 04 (safety stack) |
| `19-agents-skills/` (13 files) | Doc 05 (archetypes), Doc 02 (tool skills) |
| `01-golem/02-heartbeat.md` | Doc 06 (heartbeat) |
| `01-golem/16-risk-engine.md` | Doc 04 (safety), Doc 06 (heartbeat budget) |
| `01-golem/17-prediction-engine.md`, `17b` | Doc 03 (TA indicators) |
| `03-daimon/` (6 files) | Doc 08 (daimon integration) |
| `05-dreams/` (4+ files) | Doc 09 (dreams integration) |
| `22-oneirography/` (7 files) | Doc 09 (dream journals, replay) |
| `04-memory/` | Doc 10 (neuro knowledge) |
| `shared/hdc-*.md` (3 files) | Doc 03, Doc 10 (HDC patterns) |
| `13-runtime/` | Doc 06 (heartbeat), Doc 01 (chain runtime) |
| `20-styx/` (8 files) | Doc 01 (chain scope, deployment) |
| `appendices/performance-targets.md` | Doc 13 (benchmarks) |

---

## Product Layer Cross-Reference

> Each gap doc now includes a `## Product Layer` section mapping its capabilities to the 12 universal primitives defined in `nunchi-dashboard/docs/prd/23-universal-primitives.md`.

| Gap Doc | Key Primitives | Dashboard Location | PRD Reference |
|---------|---------------|-------------------|---------------|
| 01 — Chain Runtime | Connector (ChainRpc), Feed (block events), Gate (TxSimulator) | System → Connectors, Pulse → Event Stream | PRD 23 §Connector, §Feed |
| 02 — Tools | Connector (VenueAdapter), Extension (tool handlers), Gate (risk check) | System → Extensions, System → Connectors | PRD 23 §Connector, §Extension |
| 03 — TA Indicators | Recipe (indicator pipelines), Knowledge (regime codebook), Signal (regime change) | Forge → Recipes, Knowledge → Indicators | PRD 23 §Recipe |
| 04 — Safety | Gate (risk/MEV/circuit-breaker/custody), Extension (daimon risk), Signal (alerts) | System → Gates, Pulse → Alerts | PRD 23 §Gate |
| 05 — Agent Archetypes | Agent (archetype manifest), Group (delegation DAG), Gate/Extension (per-archetype) | Fleet → Templates, Fleet → Agents | PRD 23 §Agent |
| 06 — Heartbeat | Feed (tick clock), Recipe (CorticalState), Gate (shutdown), Knowledge (decisions) | Pulse → Heartbeat, Agent Detail → Heartbeat | PRD 23 §Feed, §Recipe |
| 07 — Learning Loops | Recipe (P&L/indicators/regime), Knowledge (playbooks), Eval (benchmarks), Signal (reward) | Measurements → Evals, Forge → Recipes | PRD 23 §Recipe, §Eval |
| 08 — Daimon Integration | Extension (tilt/sizing), Recipe (prospect value), Knowledge (somatic map), Signal (tilt alerts) | Agent Detail → Affect Panel, Extension Workshop | PRD 23 §Extension, §Recipe |
| 09 — Dreams Integration | Feed (dream triggers), Recipe (counterfactual/threats), Eval (calibration), Knowledge (journal) | Knowledge → Dream Cycles, Forge → Recipes | PRD 23 §Recipe, §Feed |
| 10 — Neuro/HDC | Recipe (HDC encoding/routing/transfer), Knowledge (regime codebook/decay), Signal (routing) | Knowledge → Entry Detail, System → Model Routing | PRD 23 §Recipe, §Knowledge |

### Primitive Coverage

Every primitive appears in at least one gap doc:

| Primitive | Gap Docs |
|-----------|----------|
| Agent | 05 |
| Extension | 02, 04, 05, 08 |
| Connector | 01, 02 |
| Gate | 01, 02, 04, 05, 06 |
| Feed | 01, 06, 09 |
| Recipe | 03, 06, 07, 08, 09, 10 |
| Knowledge Entry | 03, 06, 07, 08, 09, 10 |
| Arena | (not DeFi-gap-specific — see PRD 15) |
| Eval | 07, 09 |
| Signal | 03, 04, 07, 08, 10 |
| Group | 05 |
| Bounty | (not DeFi-gap-specific — see PRD 17) |

## Glossary

| Term | Definition | Where in Roko |
|------|-----------|---------------|
| **Signal** | Universal data unit — the noun of roko's architecture. All data flows as signals. | `roko-core/src/signal.rs` |
| **Substrate** | Storage trait — where signals live (JSONL files, databases, chains). | `roko-fs` (FileSubstrate) |
| **Scorer** | Evaluation trait — assigns quality/relevance scores to signals. | `roko-core/src/scorer.rs` |
| **Gate** | Validation trait — binary pass/fail quality checks (compile, test, clippy, diff, etc.). 7-rung pipeline. | `roko-gate/` |
| **Router** | Selection trait — picks which model/agent handles a task. CascadeRouter for model selection. | `roko-learn/src/cascade_router.rs` |
| **Composer** | Assembly trait — builds prompts from context, templates, enrichment. 9-layer SystemPromptBuilder. | `roko-compose/` |
| **Policy** | Governance trait — safety rules, resource limits, authorization. | `roko-agent/src/safety/` |
| **HDC** | Hyperdimensional Computing — 10,240-bit binary vectors for similarity-preserving encoding. | `roko-primitives/src/hdc.rs` |
| **ChainOracle** | TA indicator engine — currently SMA/EMA/RSI/BB. Needs 50+ indicators. | `roko-learn/src/oracles/chain.rs` |
| **VenueAdapter** | Protocol abstraction pattern from offchainservices-agent — normalizes DEX/CEX interfaces. | Batch 2.1 |
| **TradingReflect** | FIFO P&L attribution loop — maps trade outcomes back to agent decisions. | Batch 7.1 |
| **RegimeCodebook** | HDC-encoded library of canonical market regimes for fast regime classification. | Batch 10.2 |
| **MarketHdcEncoder** | Encodes market state (price, volume, volatility, funding) as 10,240-bit HDC vectors. | Batch 10.1 |
| **FifoMatcher** | First-in-first-out engine matching position entries to exits for P&L computation. | Batch 7.1 |
| **ArchetypeManifest** | TOML-based archetype definition with delegation, tool profiles, system prompt fragments. | Batch 5.1 |
| **DecisionCycleRecord** | Per-tick record of the 9-step decision pipeline outcome. | Batch 6.2 |
| **TxLifecycle** | State machine sequencing per-wallet transaction flow through gates. | Batch 4.3 |
| **DeFiRiskEngine** | Portfolio-level risk aggregation with Kelly sizing, drawdown tracking, exposure caps. | Batch 4.1 |
| **ProspectValueFunction** | Kahneman-Tversky asymmetric value function (1.6x loss aversion) for P&L-to-affect mapping. | Batch 8.1 |
| **APEX** | Orchestrator loop in offchainservices-agent — tick-based strategy execution. | Maps to roko heartbeat + conductor |
| **Gamma/Theta/Delta** | Tick frequencies: Gamma=250ms (fast signals), Theta=2s (strategy ticks), Delta=30s (rebalance). | `roko-chain/src/heartbeat_ext.rs` (partial) |
| **PAD Vector** | Pleasure-Arousal-Dominance — 3D affect representation in daimon. | `roko-daimon/src/lib.rs` |
| **IIT Phi** | Integrated Information Theory measure — consciousness/coherence metric. | `roko-primitives` (concept, not DeFi-wired) |
| **TradingReflect** | FIFO P&L attribution loop — maps outcomes back to decisions. | Not built (see Doc 07) |
| **WitnessEngine** | On-chain proof/attestation system. Partial in roko-chain. | `roko-chain/src/witness.rs` |
| **Somatic Marker** | Daimon's body-state signal that modulates decision confidence. | `roko-daimon/src/somatic_ta.rs` |
| **Ebbinghaus Decay** | Time-based knowledge freshness curve in neuro store. | `roko-neuro/src/temporal.rs` |
| **Hypnagogia** | Dream-state creativity mode — generates novel strategy combinations. | `roko-dreams/src/hypnagogia.rs` |
| **AgentMode** | Agent lifecycle mode: Persistent (runs until stopped), Ephemeral (one task), Reactive (trigger-based). | `roko-agent/src/archetype.rs` (Batch 5.1) |
| **IsolationLevel** | Execution isolation: InProcess (Tokio task in control plane) or FlyMachine (microVM with own volume). | `roko-agent/src/archetype.rs` (Batch 5.1) |
| **InferenceGateway** | Centralized LLM routing with cost tracking, caching, and model selection. All agents share one gateway. | `roko-serve/src/lib.rs` (architecture redesign Phase 1) |
| **MirageSimulator** | Implementation of `TxSimulator` trait using ephemeral mirage-rs for pre-trade simulation. | `roko-chain/src/mirage_simulator.rs` (Batch 0.1, used in 6.2) |
| **MirageTestHarness** | Test helper wrapping `spawn_mirage_test_instance()` for integration tests across all batches. | `roko-chain/src/mirage_harness.rs` (Batch 0.1) |
| **ChainBackend** | Enum selecting `Live` or `Mirage` chain provider for tool handlers. | `roko-chain/src/chain_backend.rs` (Batch 2.2) |

## Architecture Context

Roko's universal loop: **query → score → route → compose → act → verify → write → react**

For DeFi, this maps to:
1. **Query**: Chain event arrives (new block, price move, position change)
2. **Score**: TA indicators + regime detection evaluate the signal
3. **Route**: CascadeRouter picks model/agent for the decision
4. **Compose**: SystemPromptBuilder assembles context (market state, position, risk limits)
5. **Act**: Agent executes via tools (swap, hedge, rebalance)
6. **Verify**: Gate pipeline validates (risk check, slippage bound, balance reconciliation)
7. **Write**: Result persisted to substrate (episode log, knowledge store, chain witness)
8. **React**: Learning loops update (playbook evolution, router weights, indicator accuracy)

**Current state**: Steps 1-2 have minimal DeFi support (4 indicators, no live chain events). Steps 3-5 work for generic tasks but lack DeFi specialization. Steps 6-8 work for code tasks but not trading outcomes.

## Summary Gap Heatmap

| Area | Built | Wired | Gap | Priority |
|------|-------|-------|-----|----------|
| Chain Runtime | 30% | 15% | **70%** | P0 — foundation |
| DeFi Tools | 3% | 3% | **97%** | P0 — agents need tools |
| TA Indicators | 8% | 8% | **92%** | P1 — analysis capability |
| Safety Stack | 35% | 25% | **65%** | P0 — must be safe before trading |
| Agent Archetypes | 0% | 0% | **100%** | P1 — specialization |
| Heartbeat | 15% | 5% | **85%** | P1 — real-time requirement |
| Learning Loops | 40% | 30% | **60%** | P2 — self-improvement |
| Daimon Integration | 20% | 5% | **80%** | P3 — affect modulation |
| Dreams Integration | 25% | 0% | **75%** | P3 — offline discovery |
| Neuro/HDC | 30% | 10% | **70%** | P2 — knowledge routing |
