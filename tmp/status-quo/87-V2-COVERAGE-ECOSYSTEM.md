# V2 Ecosystem Specs (20-26, 28) + Guides — Coverage

> Status-quo audit · **re-verified against code 2026-07-08 @ HEAD 5852c93c0** (prior pass 2026-07-07) · sources: 8 specs (`docs/v2/2{0-6,8}-*.md`, 8,496 lines) + 5 guides (`docs/v2/{ACP-INTEGRATION,API-REFERENCE,ARCHITECTURE,CLI-REFERENCE,INTEGRATION}*.md`, 12,508 lines) + 8 sibling audits (15, 18, 42, 45, 51, 58, 59, 62) + ~50 targeted greps across 12 crates, 3 apps, `contracts/`, `demo/demo-app/`, `.roko/GAPS.md`. Status vocab: ✅ works end-to-end · 🔌 built-not-wired · 🟡 partial · ❌ missing · 🕰️ superseded/legacy.
>
> **2026-07-08 re-verify deltas** (grep-confirmed at HEAD 5852c93c0; recent commits `8f3497063` "scheduled cold archival", `bfe0f82d6` "knowledge-informed routing, registry cleanup"): (1) **Cold-substrate archival is now WIRED at runtime** — `start_cold_archival_timer` is called on serve boot (`roko-serve/src/lib.rs:344,800`), config-gated by `archival_interval_secs` (default 6h, `schema.rs:1587`), runs `run_cold_archival_tick` per interval (`lib.rs:2096-2134`); this closes the CLAUDE.md roadmap item "cold substrate archival: built but not instantiated." (2) **22-REGISTRIES has a partial live bridge** — `roko-serve/src/routes/agents.rs:486,512-525,678` now instantiates an alloy `OnChainAgentRegistry` from `config.chain.agent_registry` address, so agent registration/lookup CAN reach the on-chain contract behind config (upgrade the AgentRegistry row from pure 🔌 toward 🟡 when `[chain]` is populated). Still absent: `identity.json` auto-registration (grep=0), `agent_reputation` in cascade router (grep=0), TraceRank/x402/marketplace callers. (3) All vapor claims re-confirmed: Arena/Eval/Bounty types = 0 (only `bench.rs`), no `/api/arenas|defi|evals|bounties` route files, `CamelTag`/`CrossCutFunctor`/`ClearingHouse`/`ArenaRegistry` = 0. Serve routes still **288**; contracts still **13** (no ClearingHouse/ArenaRegistry).

## Summary

The seven ecosystem specs split into three bands. **Band 1 — real system, different shape** (20-SURFACES, 25-DEPLOYMENT): the CLI/TUI/serve/daemon/deploy stack is substantial and live, but almost none of it uses the specs' vocabulary — the TUI has 10 tabs (Dashboard…Learning, `tui/tabs.rs:10-31`), not the spec's 7 surface-mapped tabs; none of the five named surface contracts (`InboxCategory`, `UrgencyLevel`, `AutonomyConfig`, `FlowSummary`) exist as types (grep = 0); deployment shapes are separate subcommands, not config Signals. **Band 2 — big 🔌 inventory** (21-MARKETPLACE, 22-REGISTRIES, 24-DEFI): `roko-chain` has 31 modules (three ERC-8004 registries, TraceRank, witness, x402, korai_token, futures_market, nelson_siegel, ISFR keeper + 4 real sources) and `contracts/src/` has 13 Foundry contracts — but x402/KoraiToken/marketplace/TraceRank have **zero callers outside roko-chain**, and only ISFR + chain-read routes are served. **Band 3 — vapor** (23-ARENAS, 26-CROSS-CUTS formalism): no Arena/Eval/Bounty type anywhere (`grep Arena crates/` → only `roko-cli/src/bench.rs`), no `CrossCutFunctor`, no `/api/arenas|evals|bounties|defi` routes — though the cross-cut *behaviors* (memory/daimon/dreams/safety) are genuinely wired into the live v2 runner (`runner/event_loop.rs`).

The five guides are the inverse of the specs: **written from code and largely accurate** (CLI-REFERENCE's exit-code table matches `main.rs:77-82` constant-for-constant; ARCHITECTURE-GUIDE §2 openly teaches `Engram` + LinUCB + 9 operations, i.e. reality; INTEGRATION-GUIDE's known-limitations table matches GAPS). Their drift is omission (CLI-REFERENCE misses ~12 live commands; API-REFERENCE misses isfr/feeds/relay/workspaces/swe-bench families) plus one invention (`POST /api/auth/login` — `routes/auth.rs` only has `/api-keys`).

28-ROADMAP is stale in **both directions**: its Phase-0 "lacks" list is now wrong for ~8 items that have since landed (Pulse struct, Bus trait + PulseBus, demurrage `balance` on Engram, TypeSchema, CalibrationPolicy wired into the runner, daemon dream loop, section-effect tracking, CognitiveWorkspace), while Phase-2/3 items (Rack, marketplace CLI, type-state lifecycle, arenas, CamelTag, corrigibility, Merkle-CRDT) remain untouched. Serve route count today: **288 raw `.route(...)`** in roko-serve (CLAUDE.md's "~85" and the roadmap's copy of it are 2 generations stale).

## Coverage matrix

| Spec | Concept | Code | Status | Evidence |
|---|---|---|---|---|
| 20 §3 | 5 surface contracts (typed projections/events/invariants) | none — `InboxCategory`/`UrgencyLevel`/`AutonomyLevel`/`AutonomyConfig`/`FlowSummary` | ❌ | grep across `crates/` = 0 hits |
| 20 §1.2 | 7 StateHub core projections | 4 of 7 as named projections (`active_tasks`, `gate_pipeline`, `cost_meter`, `cohort_health`); no `knowledge_health`/`c_factor`/`agent_vitality` projections | 🟡 | `roko-serve/src/projection_contract.rs` (names); `roko-runtime/src/state_hub.rs:80`; c-factor metric itself real: `roko-core/src/cfactor.rs` |
| 20 §4.2 | TUI: 7 tabs F1-F7 (Workbench/Canvas/Flows/Inbox/Knowledge/System/Agents) + Transport strip | 10 tabs F1-F10: Dashboard/Plans/Agents/Git/Logs/Config/Inspect/Marketplace/Atelier/Learning | 🟡 different taxonomy | `crates/roko-cli/src/tui/tabs.rs:10-31` |
| 20 §4.1 | `roko run <graph>` + `run cancel/respond/list/show/logs/replay`; `roko inbox`; `roko autonomy` | `run` = prompt→`do` alias (main.rs:2340); no inbox/autonomy/run-subcommands | ❌ | `main.rs:313-889` command enum (45-CLI census); `Inbox\|Autonomy` grep = 0 |
| 20 §4.1 | Canvas CLI `graph list/new/edit/fork` | `graph run/validate/show` only | 🟡 | `roko-cli/src/commands/graph.rs:47` |
| 20 §4.1 | Verb sugar (`ingest/review/audit/test`) ; exit codes 10-16 | none; exit codes 0/1/2 only | ❌ | `main.rs:77-82` |
| 20 §5 | Conversation-as-plan-editor `POST /api/plans/{id}/chat` + `PlanMutation` | route exists, LLM returns 5 mutation ops (add/remove/update task, add_dependency, reorder) vs 9 spec'd; no `SetParallel`/`AddCheckpoint`/cycle-rejection contract | 🟡 | `roko-serve/src/routes/plans.rs:35,574-600` |
| 20 §4.3 | Web dashboard w/ Workbench/Agents/Work/Knowledge/**Arena**/System nav | `demo/demo-app` real (Bench, Builder, Explorer, Terminal, dashboard/, feeds/, isfr/ pages); no Arena anywhere. Note: a **`crates/roko-demo/` Rust crate** also exists (autonomous/benchmark/deploy/chain_ctx/bindings scenarios) driving the demo backend — distinct from the TS frontend | 🟡 | `demo/demo-app/src/pages/`; `crates/roko-demo/src/`; `grep -rli arena demo/demo-app/src` = 0 |
| 20 §4.3 | `/ws/events`, `/ws/runs/{id}`, `/sse/triggers`, `/sse/cost`, `/sse/inbox` | actual: `/events`, `/sse`, `/ws`, `/roko-ws` + `/api/workflow/ws` | 🟡 equivalents, different paths | `roko-serve/src/routes/{sse,ws}.rs`; 59-API-ROUTE-LEDGER |
| 20 §9 | Authoring API: 12 object types × CRUD+validate+deploy+publish | no uniform pattern; per-family CRUD for plans/prds/agents/jobs/deployments only | ❌ | `roko-serve/src/routes/` (44 modules, no `{object_type}` framework) |
| 20 — | code's own surface census | `surface_inventory.rs` (`roko status --surfaces`) w/ Wired/Partial/Stub/Missing | ✅ (stale names per 62) | `roko-cli/src/surface_inventory.rs:1-41` |
| 21 §16 | `roko market` CLI (14 subcommands) | none | ❌ | `grep -c 'Market' main.rs` = 0 |
| 21 §7/§9 | Publish/install pipeline Graphs + cells (Checksum/Signature/CapabilityIntersect…) | none | ❌ | grep `ChecksumVerifyCell\|ArtifactStoreCell` = 0 |
| 21 §8.3/§15 | `MarketplaceTrendLens`/`UsageLens`/`VerifiedRunCell` | no Lens abstraction at all | ❌ | 18-V2-DEPTH: `grep 'trait Lens\|struct .*Lens'` = 0 |
| 21 §2 | 5-tier SPI | T1 prompts≈role templates (roko-compose), T2 config profiles ✅ (layered loader), T3 declarative tools≈roko-plugin 🟡, T4 WASM ❌ (no wasmtime/wasm32 in any Cargo.toml), T5 native ✅ | 🟡 | `roko-core/src/config/loader.rs`; `roko-plugin/src/lib.rs:19-33` |
| 21 — | nearest real marketplace | **labor** marketplace: 11 job routes + auto-exec runner + TUI F8 "Marketplace: job browser" | ✅ (different product) | `roko-serve/src/routes/jobs.rs:18-33`; `job_runner.rs:47-142`; `tabs.rs:25-26`; 58-JOBS |
| 22 §2-4 | Three ERC-8004 registries (Rust) | `agent_registry.rs`, `reputation_registry.rs`, `validation_registry.rs` in roko-chain — in-memory + alloy feature-gated. **New 2026-07-08**: `roko-serve/src/routes/agents.rs:486,512-525,678` instantiates an alloy `OnChainAgentRegistry` from `config.chain.agent_registry` address → a serve→contract bridge exists behind config | 🔌→🟡 (when `[chain]` populated) | `crates/roko-chain/src/`; `roko-serve/src/routes/agents.rs:486`; 42-CHAIN |
| 22 §10-11 | 8 contracts on Mirage (hardhat) | 13 Foundry contracts: IdentityRegistry, AgentRegistry, ReputationRegistry, ValidationRegistry, ISFROracle, ISFRBountyPool, BountyMarket, InsightBoard(≈KnowledgeRegistry), RoleRegistry, WorkerRegistry, ConsortiumValidator, FeeDistributor, MockERC20; **no ClearingHouse, no ArenaRegistry**; deploy = single `Deploy.s.sol` (foundry, not hardhat) | 🔌 | `contracts/src/`, `contracts/script/Deploy.s.sol` |
| 22 §3.2 | TraceRank reputation Score-cell | `trace_rank.rs` exists (payment-edge variant, not attestation variant); zero callers outside roko-chain | 🔌 | `roko-chain/src/trace_rank.rs:43-138`; caller grep = 0 |
| 22 §3.5 | Reputation in cascade router (`RoutingContext.agent_reputation`) | agent-reputation still absent; but **knowledge-informed routing landed** (commit `bfe0f82d6`): `apply_knowledge_to_route` folds neuro-store advice into route selection on Theta frequency | ❌ (reputation) / 🟡 (adjacent knowledge-routing) | grep `agent_reputation` roko-learn = 0; `roko-learn/src/cascade_router.rs:623,950,991,1610` |
| 22 §2.4 | Identity auto-registration → `.roko/state/identity.json` | absent | ❌ | grep `identity.json` = 0 |
| 22 §5 | Chain witness anchoring | `witness.rs` built; CLAUDE.md itself says "Phase 2+" | 🔌 | `roko-chain/src/witness.rs` |
| 22 §6 | GossipCell / gossip networking | phase2.rs stubs only | ❌ | `roko-chain/src/phase2.rs`; 42-CHAIN |
| 22 §8/§11.2 | Event indexer + `roko indexer start` + :6678 REST API | none | ❌ | no `Indexer` struct/mod in roko-chain; no `indexer` clap command |
| 22 §4.6 | ZK-HDC proofs | none | ❌ | grep zk = 0 |
| 22 §10.2 | `[chain]` config: networks mirage(31337)/korai(88888) | code: chain **profiles** `"mirage"` (dev, chain_id 1) / `"daeji"` (testnet) — doc's token is "Daeji", code's token module is `korai_token.rs`: names swapped | 🟡 drift | `roko-core/src/config/chain.rs:16-23,141` |
| 22/24 | ISFR keeper + sources | `roko isfr start/status/sources` CLI + 4 serve routes; sources aave_v3/compound_v3/ethena/lido + mock (mock default; relay publish "Phase 2") | 🟡 | `roko-cli/src/commands/isfr.rs:51,98-113`; `roko-serve/src/routes/isfr.rs:23-26`; `roko-chain/src/isfr_sources/` |
| 23 all | Arena/Eval/Bounty types, 8 arenas, 7-step flywheel, leaderboards | nothing | ❌ | `grep -rln 'Arena'` → only `roko-cli/src/bench.rs` |
| 23 §13 | 37 `/api/arenas|evals|bounties` routes | none | ❌ | no such route files in `roko-serve/src/routes/` |
| 23 §16 | crate-mapping claims: `marketplace.rs` "Wired", `vcg_allocate` "Wired", `eval_generator.rs` exists | marketplace.rs has **no runtime callers** (overstated → 🔌); `vcg_allocate` built but greedy dominates at runtime; `roko-gate/src/eval_generator.rs` does exist | 🟡 self-aware but optimistic | 58-JOBS; CLAUDE.md VCG row; `ls crates/roko-gate/src` |
| 23 — | nearest real measurement surface | bench harness: 23 `/bench` routes, `roko bench swe --dataset`, Pareto module | ✅ (unnamed) | 59-ledger; `roko-learn/src/pareto.rs` |
| 24 §8 | 17 `/api/defi/*` routes | none; actual = `/api/isfr/*` (4) + `/api/chain/*` (7) | ❌ | `routes/isfr.rs:23-26`, `routes/chain.rs:38-44` |
| 24 §4 | Yield perps + ClearingHouse | none (`futures_market.rs` is *knowledge* futures, different thing); no ClearingHouse.sol | ❌ | grep `ClearingHouse` = 0 |
| 24 §5-6 | ChainDataSource/Aggregator, VenueAdapter, DeFiRiskEngine, TradingReflect, `prospect_value` | none of these types; partial analogs: `block_watcher.rs`, `apps/roko-chain-watcher`, `gate/{mev_gate,tx_sim_gate}.rs`, roko-daimon PAD affect | ❌ (analogs 🔌) | grep all five = 0; `roko-chain/src/gate/` |
| 24 §3 | ISFR rate curves/term structure | `nelson_siegel.rs` built, no `/curves` route | 🔌 | `roko-chain/src/nelson_siegel.rs` |
| 24 §12 | `[defi]` config section | absent; `[chain]` exists with different shape | ❌ | `roko-core/src/config/chain.rs` |
| 24 §9 | heartbeat gamma/theta/delta drives DeFi Hot Flows | heartbeat real (incl. new theta consumer); no DeFi flows | 🟡 clock only | `roko-runtime/src/heartbeat.rs:23-29`, `theta_consumer.rs` |
| 25 §1 | Scaling tiers as config (`BusTopology`, `ScalingTier`, tier-advisor Graph) | absent — tiers are separate subcommands | ❌ | grep = 0; 58-JOBS shape matrix |
| 25 §2 | local dev flow (init/serve/dashboard/agent create/self-host loop) | works | ✅ | 45-CLI, 58-JOBS |
| 25 §2.4 | Mirage local chain | `apps/mirage-rs` real EVM-sim app w/ relay proxy; no `roko mirage` CLI (only `deploy railway --with-mirage`) | 🟡 | `apps/mirage-rs/src/` routes; `main.rs:1673-1681` |
| 25 §3 | daemon lifecycle + installers | full: IPC, 5 loops (incl. **dream loop**), launchd+systemd; not expressed as Graph of Cells | ✅ (shape 🕰️v1) | `roko-cli/src/daemon.rs:322,339-340`; 58-JOBS |
| 25 §5 / 26 §3.1 | Cold-substrate archival (aged engrams → cold store) | **New 2026-07-08**: `start_cold_archival_timer` scheduled on serve boot, config-gated `archival_interval_secs` (6h default), migrates aged-out engrams per tick | ✅ (runtime-wired) | `roko-serve/src/lib.rs:344,800,2096-2134`; `roko-core/src/config/schema.rs:1587`; commit `8f3497063` |
| 25 §3.6 | self-healing supervisor Graph + `ROKO_SUPERVISOR_AUTOFIX` | absent; partial analog = conductor circuit breaker + systemd restart/backoff | ❌ | grep `SUPERVISOR_AUTOFIX\|CrashReport` = 0 |
| 25 §4 | WASM packaging (fuel, ABI, wit-bindgen) | zero wasm anywhere | ❌ | grep wasmtime/wasm32 in all Cargo.toml = 0 |
| 25 §5 | Brain export/import/Merkle-CRDT sync | analog exists: `knowledge backup --top-n` / `restore --min-confidence --generation` / `sync <peer> --direction` (mesh); **no** `.roko-brain` bundle, no learning-state export, no merkle/crdt | 🟡 | `main.rs:918-964`; grep merkle/crdt in roko-neuro = 0 |
| 25 §6 | Secrets: 3-tier priority + age encryption | secrets CLI + profile-aware store exist; **no age encryption found** (`age::` = 0 in cli/serve src) | 🟡 | 45-CLI (`config secrets set/get/list/rotate`); grep |
| 25 §7-9 | Railway/Fly/Docker deploy | real (Railway GraphQL 923-LOC backend; fly template; docker build+tag, **no push**) | ✅/🟡 | 58-JOBS rows; `commands/server.rs:427-445` |
| 25 §10 | Worker mode registers with relay | worker is HTTP task server + control-plane callback; no relay registration | 🟡 different design | `roko-cli/src/worker/handler.rs` (58-JOBS) |
| 25 §12 | Relay + Mirage backbone | both apps real; serve proxies relay | ✅ | `apps/agent-relay/src/` (12 `/relay/*` routes verified); `routes/relay_proxy.rs` |
| 26 §2 | `CrossCutFunctor` trait, endofunctors, natural transformations, VCG arbitration cell | none of the formalism | ❌ | grep `CrossCutFunctor\|pre_enrich` = 0 |
| 26 §3-5,8 | Memory/Daimon/Dreams/Safety *behaviors* in live loop | wired as direct calls in v2 runner + ACP: daimon/neuro/dreams referenced in `runner/{event_loop,types}.rs`, `dispatch/model_routing.rs`; SafetyLayer per dispatch; CaMeL dual-LLM built (`safety/data_llm.rs`) | ✅ behavior / ❌ shape | `roko-cli/src/runner/event_loop.rs`; 51-ACP; 18-V2-DEPTH §17 |
| 26 §3.1 | demurrage-bearing Signals | `balance` field + reset on Engram; reinforcement in learn (`playbook_rules.rs`, `runtime_feedback.rs`); serve demurrage timer | 🟡 | `roko-core/src/engram.rs:95-97,150`; `roko-serve/src/lib.rs:347` (58) |
| 26 §7 | VCG arbitration between cross-cuts | `vcg_allocate` exists only for context-section auction; greedy dominates; no cross-cut arbitration | ❌ | `roko-compose/src/auction.rs:380` (18-V2-DEPTH) |
| 26 §9 | 5 feedback loops | memory reinforcement 🟡, daimon adaptation ✅ (`roko-daimon/src/mortality.rs`), dream prioritization 🟡 (Mattar-Daw mostly bypassed), arbitration calibration ❌, contract evolution ❌ | 🟡 | 18-V2-DEPTH §11; grep |

## Per-spec notes

- **20-SURFACES**: The spec's *plumbing* landed (StateHub + projection contract + SSE/WS + plan-chat + TUI file watcher `tui/fs_watch.rs`), the *taxonomy* didn't. Nothing in code calls anything a "Workbench" or "Stigmergy Minimap". The autonomy story has no representation at all (no levels 0-4 anywhere); the closest safety analog is role auth + `AgentContract`. Plan-chat (`plans.rs:574`) is a genuine, live implementation of §5 with a reduced mutation grammar. The 12-primitive-object authoring system exists only as scattered per-family routes.
- **21-MARKETPLACE**: Two marketplaces are conflated across docs (58-JOBS open question #1). The **package** marketplace of this spec has zero code — no registry, publish, install, fork, lockfile, or trust machinery. The **labor** marketplace (jobs) is real and local-only. `roko config plugins list/install/remove/audit` is the only install-shaped surface today.
- **22-REGISTRIES**: The most misleading gap class — *both* sides exist (Rust in-memory registries AND Solidity contracts) but the bridge (alloy clients wired at runtime, deployment, indexer) doesn't. `get_logs` unsupported in the alloy client (42-CHAIN). Contract set drifted from spec: spec's 8 include ClearingHouse + ArenaRegistry (absent); repo has 5 extra the spec never mentions (RoleRegistry, WorkerRegistry, ConsortiumValidator, FeeDistributor, ISFRBountyPool). Korai/Daeji naming inverted between doc and code.
- **23-ARENAS**: Purely aspirational, and the only spec whose own §16 admits it ("Not yet implemented" rows) — but even its "Wired" rows overstate (marketplace.rs unreferenced at runtime). Bench + gates + c-factor are the raw material; nothing arena-shaped assembles them. Matches 18-V2-DEPTH §19 verdict exactly.
- **24-DEFI**: ISFR is the one real vertical (keeper, 4 protocol sources, oracle-submit, bootstrap, bounty-pool contract, CLI, 4 routes) — everything else (perps, clearing, venue adapters, risk engine, P&L reflection, affect sizing) is ❌. MEV/tx-sim gates and nelson_siegel exist as unwired building blocks.
- **25-DEPLOYMENT**: Strongest spec-to-code correspondence in *function* (daemon/deploy/worker/relay/mirage all run; daemon even auto-dreams) and weakest in *form* (no Graphs-of-Cells, no config-Signal shapes, no WASM, no brain format, no supervisor). Acceptance IDs D-1..D-9, D-30, D-32(analog), D-33 pass in spirit today; D-10..D-29, D-34..D-41 have no implementation.
- **26-CROSS-CUTS**: A formalization of behavior that already exists informally. Verdict: behaviors ✅ (in runner v2 + ACP + heartbeat consumers), category-theory shape ❌. If the functor API is wanted, it is a refactor, not new capability.

## Doc drift tables (CLI + API)

**CLI: documented-but-missing** (all from *specs*; CLI-REFERENCE guide documents none of these — it tracks code):

| Doc'd command | Source | Code reality |
|---|---|---|
| `roko market …` (browse/install/publish/fork ×14) | 21 §16 | ❌ no `Market` variant in `main.rs` |
| `roko inbox list/show/approve/reject/dismiss` | 20 §4.1 | ❌ |
| `roko autonomy show/set` (levels 0-4) | 20 §4.1 | ❌ |
| `roko run <graph>` + `run cancel/respond/resume/list/show/logs/replay` | 20 §4.1 | ❌ — `run` is a prompt alias for `do` (main.rs:2340) |
| `roko graph list/new/edit/fork` | 20 §4.1 | 🟡 only `graph run/validate/show` (graph.rs:47) |
| verb sugar `ingest/review/audit/test` | 20 §4.1 | ❌ |
| `roko indexer start` | 22 §11.2 | ❌ |
| `roko plan migrate <dir>` | 28 §7.2 | ❌ |
| `roko knowledge backup --agent/--min-tier/--since/--include-episodes/--output *.roko-brain` | 25 §5.3 | 🟡 actual: `backup <dest> --top-n --force` (main.rs:918-930) |
| `roko knowledge restore --decay-factor` | 25 §5.4 | 🟡 actual: `--min-confidence --generation` (main.rs:944-949) |
| `roko knowledge sync --peer wss://… --continuous` | 25 §5.5 | 🟡 actual: `sync <peer> --direction --max-send` (main.rs:952-964) |
| exit codes 10-16 | 20 §4.1 | ❌ 0/1/2 only (main.rs:77-82) |
| `roko chat` | CLAUDE.md | ❌ never existed (45-CLI) |

**CLI: existing-but-undocumented** — CLI-REFERENCE (74 documented commands, exit codes accurate) omits: `develop`, `show`, `setup`, `think`, `note`, `tune`, `demo`, `dev`, `graph`, `isfr`, `feed`, `history`, `config doctor/export/mcp`, `status --quick/--surfaces`, `plan run --engine/--max-tasks/--force-resume` (full list + evidence: 45-CLI-SURFACE doc-drift table). `isfr`/`feed`/`graph`/`layer-check` appear in **no** doc.

**API: documented-but-missing**:

| Doc'd route family | Source | Code reality |
|---|---|---|
| `/api/arenas/*` (13), `/api/evals/*` (10), `/api/bounties/*` (14) | 23 §13 | ❌ no route files |
| `/api/defi/isfr|positions|clearing|risk|chains` (17) | 24 §8 | ❌; nearest: `/api/isfr/*` (4, isfr.rs:23-26), `/api/chain/*` (7, chain.rs:38-44) |
| indexer `:6678 /api/index/*` (11) | 22 §8.4 | ❌ |
| `/ws/events`, `/ws/runs/{id}`, `/sse/{triggers,cost,inbox}` | 20 §4.3 | 🟡 actual: `/events`, `/sse`, `/ws`, `/roko-ws`, `/api/workflow/ws` |
| `/api/{object_type}/{id}/{validate,deploy,publish}` ×12 types | 20 §9 | ❌ |
| marketplace HTTP routes | 21/28 §3.8 | ❌ (`routes/marketplace.rs` doesn't exist) |
| `POST /api/auth/login` | API-REFERENCE (tail) | ❌ `routes/auth.rs:68-69` has only `/api-keys` CRUD |

**API: existing-but-undocumented** — API-REFERENCE (33 sections, current architecture diagram, StateHub push pattern) has **0 mentions** of: `/api/isfr/*`, workspaces, swe-bench, shared-runs, event-ingest; ≤1 passing mention of feeds, relay proxy, rpc/mirage proxy, connectors (grep counts). 59-API-ROUTE-LEDGER: OpenAPI output is even further behind. Counts: roko-serve raw routes **288** (grep verified today; 59-ledger counted 272 earlier — still growing), CLAUDE.md/roadmap say ~85 🕰️. Demo-frontend mismatches (`/api/share/{token}` vs `/api/shared/{token}`, `/api/bench/matrix`, `/api/isfr/stream`, `/ws/agents`): see 59-ledger.

## Roadmap reality check (28-ROADMAP, v3.0 of 2026-04-26)

**Phase 0** — copied from CLAUDE.md, inherits its staleness (~85 routes; 20-crate table vs 31 crates + 3 apps; "dreams… no runtime trigger" now false — daemon runs a dream loop, `daemon.rs:322,339-340`; roko-chain row wildly undersells 31 modules + 13 contracts).

**Phase 1 "Kernel Upgrade" — ~60% landed, contradicting its own §1.3 "lacks" list:**

| Item | Verdict | Evidence |
|---|---|---|
| 1.1 Pulse/Bus kernel | ✅ mostly | `roko-core/src/pulse.rs:75` + `graduate()`:138; `Bus` trait `traits.rs:385`; `PulseBus impl Bus` `roko-runtime/src/pulse_bus.rs:35,68` |
| 1.2 Predict-publish-correct | 🟡 wired | `roko-learn/src/calibration_policy.rs` consumed by `roko-cli/src/runner/event_loop.rs` (live v2 runner) |
| 1.3 Demurrage | 🟡 | Engram `balance` (`engram.rs:95-97`); reinforcement in roko-learn; serve timer; no full Gesell law module in roko-neuro |
| 1.4 Heuristic kind | 🟡 | `roko-learn/src/heuristics.rs` (falsifiers) — not `Kind::Heuristic` in roko-core |
| 1.5 EFE routing | ❌/🔌 | CascadeRouter still LinUCB (`roko-learn/src/wal.rs:21-25`); `active_inference.rs` exists but feeds compose/neuro, not routing |
| 1.6 Dream runtime trigger | ✅ | daemon dream loop (`daemon.rs:339` `DreamLoopConfig{auto_dream,…}`) + ACP ≥10-episode trigger (51-ACP) |
| 1.7 Observe + 10 Lenses | ❌ (trait only) | `Observe` trait `traits.rs:400`; zero `Lens` types (18-V2-DEPTH) |
| 1.8 Trigger/Connect protocols | 🟡 | traits `traits.rs:408,420`; production impls test-only; roko-plugin event sources are the de-facto triggers |
| 1.9 TypeSchema | 🟡 | `roko-core/src/cell.rs:63` (Any/OfKind + `compatible()`) — minimal vs spec enum |

**Phase 2 — the current battleground:** Hot Graph engine 🔌 (roko-graph is the *clap default* for `plan run` yet live dispatch is a dry-run stub — GAPS.md Tasks 101-103, `task_executor.rs:84-86`; the P0 of 45/62); `plan migrate` ❌; type-state lifecycle ❌; vitality 🟡 (in `roko-daimon/src/mortality.rs`, not roko-agent); CognitiveWorkspace ✅ type + event-logged (`roko-orchestrator/src/event_log.rs:20,254`); section effects ✅ (`roko-learn/src/section_effect.rs` → `system_prompt_builder.rs:344`); StateHub ✅/🟡 (4/7 projections); five surfaces ❌; Rack/Macro/Slot ❌ (grep 0 in roko-graph); 5-tier SPI 🟡 (T4 ❌); Marketplace v1 ❌ entirely.

**Phase 3 — untouched except accidental bridges:** L4 ❌; CaMeL 🟡 (dual-LLM `safety/data_llm.rs` built; `CamelTag` 0); corrigibility ❌ (grep 0); on-chain deployment 🟡 (13 contracts + `Deploy.s.sol`, undeployed/unwired); arenas ❌; brain export 🟡 (backup/restore/sync analogs; no merkle/crdt); cross-agent knowledge 🟡 (`knowledge sync` mesh + relay exist; no InsightStore discovery).

**Naming table (§7.1)**: further along than the table says — `Store/Score/Verify/Route/Compose/React` are already the trait names (`traits.rs:37`; `pub trait Substrate: Store {}` compat at `traits.rs:428`), but the noun inversion never happened: `Signal` is still an alias for `Engram` (`roko-core/src/signal.rs:6`) 🕰️, and ARCHITECTURE-GUIDE teaches Engram unapologetically.

## Migration checklist

- [ ] **[P0]** Retag 23-ARENAS §16 rows: `marketplace.rs`/`vcg_allocate` "Wired"→"Built-not-wired"; decide arenas = implement (Phase 3), or mark spec 🕰️ deferred like 19-arenas depth INDEX — verify: `grep -n 'Wired' docs/v2/23-ARENAS.md` shows corrected statuses
- [ ] **[P0]** Fix 28-ROADMAP Phase-0/1: strike the 8 landed "lacks" items (Pulse, Bus, demurrage balance, TypeSchema, CalibrationPolicy, dream trigger, section effects, CognitiveWorkspace), update route count 85→288, crates 20→31+3 apps — verify: each struck item cites its file:line
- [ ] **[P0]** Registries decision (42-CHAIN "Main Decision"): declare chain-backed identity/reputation/jobs as optional-mode / future / canonical. **Partial progress 2026-07-08**: the serve→contract bridge now EXISTS (`agents.rs:486` alloy `OnChainAgentRegistry` from `config.chain.agent_registry`), so the "wire one thin path" half is underway — remaining is (a) agent startup auto-registration writing `.roko/state/identity.json` (grep=0), (b) reputation into cascade router (`agent_reputation` grep=0), (c) reputation/validation-registry serve bridges (only agent-registry is wired). Verify: `roko agent start` with populated `[chain] agent_registry=0x…` registers on-chain + writes identity.json, OR docs mark (a)/(b)/(c) "future"
- [ ] **[P1]** Add `POST /api/auth/login` to serve or delete it from API-REFERENCE; add missing families (isfr, feeds, relay/rpc proxy, workspaces, swe-bench, shared-runs, event-ingest) — verify: every §route in API-REFERENCE greps to a `.route(` in `roko-serve/src` and vice versa (route-manifest CI per 59-ledger)
- [ ] **[P1]** 20-SURFACES: either add a "current rendering reality" appendix (10 TUI tabs, actual SSE/WS paths, plan-chat's 5 ops) or implement the minimal missing contracts (`InboxCategory`+`/sse/inbox`, autonomy levels) — verify: `roko status --surfaces` lists surface entries matching the spec table
- [ ] **[P1]** Reconcile chain naming: korai↔daeji, hardhat→foundry, contract table (add the 5 real contracts, drop/mark ClearingHouse+ArenaRegistry) in 22-REGISTRIES §10-11 — verify: `ls contracts/src` matches doc table
- [ ] **[P1]** Extend plan-chat toward spec: SetParallel/AddCheckpoint/UpdatePlanMeta ops + cycle-rejection + `rejected[]` response — verify: POST `/api/plans/{id}/chat` with a cyclic dep request returns it in `rejected`
- [ ] **[P2]** Marketplace scope decision: package-marketplace (21) vs labor-jobs — if package v1 proceeds, start with `roko config plugins` as install substrate + Cell manifest (28 §3.8) — verify: `roko market install <ref>` round-trips a Tier-1 prompt artifact
- [ ] **[P2]** DeFi spec triage: split 24-DEFI into "ISFR (shipping)" and "trading stack (design)"; wire `nelson_siegel.rs` behind `/api/isfr/curves`; add real-source default for `roko isfr start` — verify: `curl :6677/api/isfr/curves` returns a term structure from non-mock sources
- [ ] **[P2]** Brain-export gap: add learning-state (cascade-router/thresholds/experiments) to `knowledge backup` and a manifest, or amend 25 §5 to describe the actual backup/restore/sync — verify: backup dir contains `learn/` snapshot; restore rehydrates router state
- [ ] **[P2]** Cross-cuts: either implement `CrossCutFunctor` as a wrapper API over the existing runner calls, or downgrade 26 §2/§6/§7 to "design rationale" and keep §9 feedback loops as the implementable contract — verify: `grep -rn 'CrossCutFunctor' crates/` matches doc claim either way
- [ ] **[P3]** Secrets: implement age encryption (dep exists in Cargo.toml, unused) or fix 25 §6.2 — verify: `roko config secrets set X` writes non-plaintext file
- [ ] **[P3]** WASM (25 §4, 21 Tier-4): explicitly move to a phase gate in 28-ROADMAP with no code claim — verify: roadmap row marked "not started, no dependency holds Phase 2"
- [ ] **[P3]** Log this spec-family's gaps into `.roko/GAPS.md` (today it has only Tasks 101-103 graph items; zero marketplace/registry/arena/defi entries) — verify: `grep -ci 'arena\|marketplace\|defi' .roko/GAPS.md` > 0

## Open questions

1. **Are specs 21-24 commitments or explorations?** They carry acceptance criteria (implying commitment) but zero GAPS.md/backlog presence. If exploratory, they should be tagged like 21-roadmap depth docs ("research/futures") to stop inflating coverage audits.
2. **Which marketplace wins** — package (21) or labor (jobs, shipped)? They share the word, a TUI tab, and nothing else (58-JOBS Q1).
3. **Is the chain economy mirage-first-then-real, or demo inventory?** 13 contracts + 31 chain modules with no runtime path is a lot of unpowered machinery; `apps/mirage-rs` suggests sim-first, but nothing consumes registries even against mirage (42-CHAIN, 18-V2-DEPTH Q4).
4. **Surface taxonomy**: retrofit spec names onto the 10-tab TUI/demo-app (rename-only), implement the 5 contracts as types, or rewrite 20-SURFACES around what shipped? ACP also wants a seat as a 6th surface (51-ACP Q3).
5. **Roadmap phase accounting**: several Phase-2/3 items landed out of order (section effects, CognitiveWorkspace, CaMeL dual-LLM, dream trigger) while Phase-2's centerpiece (Graph engine live dispatch) is the current blocker — should 28-ROADMAP be re-baselined against GAPS Tasks 101-103 as the real critical path?
6. **Korai vs Daeji vs Mirage**: what is the canonical chain naming and chain_id story? Doc-22 (korai 88888, mirage 31337), code (`chain.rs` mirage=1, daeji testnet), token module `korai_token.rs`, contract `MockERC20` — four different answers.
