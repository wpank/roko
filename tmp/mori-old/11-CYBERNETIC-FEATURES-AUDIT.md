# Cybernetic Features Audit: Beyond Mori

Audit date: 2026-08-19

This document evaluates every "cybernetic" feature that roko claims beyond what mori
offered. For each: does it actually work end-to-end, is it wired, can a user observe
it, and what is the honest state?

Rating scale:
- **WORKING**: end-to-end execution path exists, wired into runtime, observable via CLI/TUI/HTTP
- **PARTIAL**: code is real and some paths execute, but key wiring or observability is missing
- **STUBBED**: types/contracts exist, tests pass in isolation, but the runtime never calls it
- **BROKEN**: code exists but does not execute or produces incorrect results

---

## 1. Daimon / Affect Engine

**Crate**: `crates/roko-daimon/` (8,246 LOC, ~50 unit tests)

**Rating: WORKING**

### What it claims
PAD (Pleasure-Arousal-Dominance) affect state, somatic markers, dispatch modulation,
three-layer ALMA temporal model (Gebhard 2005), goal tree, cognitive energy tracking,
behavioral phases (Thriving/Stable/Conservation/Declining/Terminal), and prospect
theory value functions.

### What actually happens

The affect engine is genuinely wired into the runner-v2 event loop:

1. **DaimonState is loaded at plan start** (`RunConfig::daimon_state_for_workdir` in
   `crates/roko-cli/src/runner/types.rs:1975`). It deserializes from
   `.roko/state/daimon-state.json` or creates a fresh neutral state.

2. **Task outcomes update affect** (`record_daimon_task_outcome` at event_loop.rs:7750).
   Every completed task calls `daimon.appraise(AffectEvent::TaskOutcome { ... })` and
   records somatic markers at the task's strategy coordinates.

3. **Dispatch is modulated** (`cognitive_dispatch_policy` at event_loop.rs:7318). Before
   each task dispatch, the runner reads `behavioral_phase()` and `cognitive_energy.current`
   to build a `CognitiveDispatchPolicy` that:
   - Caps model tiers when vitality is low (phase-capped at event_loop.rs:7380)
   - Applies cognitive cost adjustments preferring cheaper models (event_loop.rs:7488)
   - Merges a routing bias into the cascade router (event_loop.rs:7324)

4. **EFE routing is active** (`EfeRouter::default().route()` at event_loop.rs:7447).
   The active inference router selects model tiers based on surprise rate, regime, and
   task difficulty.

5. **AffectPolicy trait** (`crates/roko-daimon/src/policy.rs`): A proper trait adapter
   wraps DaimonState for the WorkflowEngine path, implementing `pre_dispatch`,
   `on_task_outcome`, `on_gate_result`, and `modulate_dispatch` with real tier bias,
   turn limit, and exploration rate adjustments.

### User observability

- **CLI**: `roko explain daimon` describes the system. No dedicated `roko daimon` command
  to inspect current affect state.
- **TUI**: No dedicated daimon/affect tab. The heartbeat pulsing in the progress bar
  (`tui/atmosphere.rs`) is cosmetic animation, not derived from DaimonState.
- **Files**: `.roko/state/daimon-state.json` can be inspected manually.

### Honest assessment

The affect engine genuinely modulates dispatch -- it is not theater. When cognitive energy
drops, cheaper models are selected. When tasks fail, the behavioral phase degrades and
somatic markers record the negative outcome at those strategy coordinates. Goal tree
promotion and seed decay also run. However:

- There is no CLI command to query current affect state
- The TUI does not surface affect data
- Somatic TA integration (IIT Phi metric, synergy detection) exists as code but has
  no consumer in the runner
- GoalTree exists and is maintained but nothing in the CLI or TUI surfaces emergent goals

---

## 2. Dreams Consolidation

**Crate**: `crates/roko-dreams/` (9,556 LOC, ~48 unit tests)

**Rating: WORKING**

### What it claims
Offline dream-cycle runtime with three phases (hypnagogia, imagination, cycle),
episode replay, threat rehearsal, routing advice generation, staging buffers,
journal/archive, and configurable triggers.

### What actually happens

1. **Dream consolidation runs after plan completion** (`run_dream_consolidation_if_enabled`
   at event_loop.rs:13905). When `learning_config.dream_after_plan` is true (checked at
   :13914), the runner spawns a `DreamRunner::new(workdir, dream_config).consolidate_now()`.

2. **CLI commands are wired** (`roko knowledge dream run|report|schedule|journal|archive`
   at main.rs:1080-1107). The `dream run` command creates a `DreamRunner` and runs
   `consolidate_now()` synchronously.

3. **The cycle does real work**: `DreamCycle` in `cycle.rs` batches completed episodes,
   clusters them by plan/task shape, calls `HypnagogiaEngine` for light review, runs
   `synthesize_hypotheses` for imagination, and writes outputs:
   - Knowledge entries to the durable neuro store
   - Playbooks to the playbook store
   - C-Factor regression analysis
   - Routing advice to `.roko/learn/dream-routing-advice.json`
   - Dream reports as JSON
   - Journal entries to `.roko/dreams/journal.jsonl`

4. **Cross-cut cascade triggers dreams** (`spawn_cross_cut_gate_failure_cascade` at
   event_loop.rs:7666). When a gate fails and the memory functor cascade fires, it
   can trigger a targeted dream consolidation with `DreamRunner::consolidate_now()`,
   then feed results back via `eta_DM` (dream-to-memory) and `eta_DN`
   (dream-to-daimon) natural transforms.

5. **Agent dispatch for review** (`DreamAgentConfig::build_agent` in runner.rs:85). The
   dream cycle can optionally dispatch review prompts through a configured LLM agent
   using the standard provider infrastructure.

### User observability

- **CLI**: `roko knowledge dream run` -- runs a cycle. `roko knowledge dream report` --
  shows latest. `roko knowledge dream journal` -- shows recent entries.
  `roko knowledge dream schedule` -- shows next scheduled fire.
- **TUI**: Dreams tab exists under Operations (`tui/pages/operations.rs:135`, PageId::Dreams).
  The `render_dreams_page` function at dashboard.rs:5348 reads journal.jsonl and
  archive.jsonl and renders cycle IDs, phases, summaries. `DreamSnapshot` widget exists
  in `tui/widgets/dream_view.rs`.
- **Files**: `.roko/dreams/journal.jsonl`, `.roko/dreams/archive.jsonl`,
  `.roko/learn/dream-routing-advice.json`

### Honest assessment

This is genuinely one of the most complete cybernetic features. The dream cycle runs
real episode analysis, produces real knowledge entries that affect subsequent dispatch,
and has full CLI and TUI observability. The only caveat is that the LLM-powered review
phase requires a configured provider, and the hypnagogia/imagination phases are more
modest pattern analysis than their names suggest -- they run deterministic statistical
clustering and hypothesis generation, not hallucinatory creative synthesis.

---

## 3. HDC Vectors

**Crate**: `crates/roko-primitives/` (HDC portion)

**Rating: WORKING**

### What it claims
10,240-bit hyperdimensional computing vectors with XOR bind, majority bundle, Hamming
similarity, deterministic seeding, codebook with role-filler binding, and cross-domain
resonance detection.

### What actually happens

1. **Episode fingerprinting is wired** (`attach_episode_hdc_fingerprint` at
   `runtime_feedback/episodes.rs:100`). Every completed episode gets an HDC fingerprint
   computed from its task title via `fingerprint_episode()` and stored in the
   `hdc_fingerprint` field. Test at line 170 verifies the field appears in output.

2. **Knowledge store uses HDC vectors** (when `hdc` feature is enabled). The MemoryFunctor
   in `compose/memory_functor.rs` has HDC-aware retrieval behind `#[cfg(feature = "hdc")]`
   that uses `text_fingerprint` for streaming lookup alongside keyword search.

3. **Backfill command exists** (`roko knowledge backfill-hdc` at main.rs:1068). This
   atomically adds HDC vectors to existing store entries that lack them.

4. **Graph fingerprinting uses HDC** (in roko-graph for Activity resume).

### User observability

- **CLI**: `roko knowledge backfill-hdc` is the direct command. Episode fingerprints
  appear in `.roko/episodes.jsonl` entries.
- **Files**: HDC vectors are stored inline in knowledge entries and episodes.

### Honest assessment

The HDC vectors are real computational primitives that get computed and stored. The
codebook, role-filler binding, TDA (persistent diagrams), tropical algebra, sheaf
Laplacian, and manifold modules exist as mathematical implementations with tests, but
most of these advanced primitives are not consumed by the runtime -- they are
infrastructure waiting for consumers. The core `text_fingerprint` and similarity
operations are genuinely used.

---

## 4. Knowledge Tiers

**Crate**: `crates/roko-neuro/` (tier_progression.rs, knowledge_store.rs)

**Rating: WORKING**

### What it claims
Four-tier progression (Transient -> Working -> Consolidated -> Persistent), distillation
from episodes to insights to heuristics to playbooks, temporal decay with kind-specific
half-lives, GC, backup/restore with genomic bottleneck.

### What actually happens

1. **Tier progression runs in the runner** (event_loop.rs:7189):
   `store.apply_tier_progression(&TierProgressionConfig::default())` is called during
   the gate-completion knowledge ingestion path.

2. **Dream consolidation drives tier changes**: The `DreamCycle` in cycle.rs runs the
   D1/D2/D3 distillation pipeline (episodes -> insights -> heuristics -> playbook).
   It writes knowledge entries at appropriate tiers and promotes successful patterns.

3. **Knowledge kinds have distinct half-lives**: Insight (30d), Heuristic (90d),
   Warning (1h), CausalLink (60d), StrategyFragment (14d). Each kind's `KnowledgeTier`
   multiplier affects retention.

4. **CLI commands are wired**: `roko knowledge query/stats/gc/backup/restore/sync`.

### User observability

- **CLI**: `roko knowledge query <topic>` searches the store. `roko knowledge stats` shows
  tier distribution. `roko knowledge gc` runs garbage collection.
- **TUI**: Knowledge health appears in dashboard projections.
- **Files**: `.roko/learn/knowledge/` directory contains the durable store.

### Honest assessment

Knowledge tiers are genuinely functional. Entries progress through tiers based on
confirmation evidence from gate results. The distillation pipeline (D1/D2/D3) runs
during dream cycles. Half-life decay, reinforcement/demurrage, and falsification all
execute. The main gap is that the Falsifier integration (entries with `falsifiable: true`
and observation/demotion tracking) exists but the automated observation pipeline requires
more runtime wiring for continuous background evaluation.

---

## 5. Trigger Runtime (E31)

**Crate**: `crates/roko-serve/src/trigger_runtime.rs` (3,337 LOC, ~16 tests)
**Types**: `crates/roko-core/src/trigger.rs`

**Rating: PARTIAL**

### What it claims
Seven trigger sources (cron, webhook, file watch, signal, manual, chain/EVM, API),
IANA/DST cron, durable history, Space/capability enforcement, mTLS, and live Graph
execution on fire.

### What actually happens

1. **Trigger runtime lives in roko-serve** (`trigger_runtime.rs`). The coordinator owns
   source lifetimes, filter/concurrency state, durable lifecycle evidence, and dispatch
   into the server's CLI runtime bridge.

2. **Real implementations exist for**: TimezoneCronEventSource (with `chrono_tz::Tz` and
   `cron::Schedule`), webhook sources, file watch sources, and manual/signal triggers.

3. **EVM ABI parsing exists** (`alloy::dyn_abi`, `alloy::json_abi` imports at line 13-14,
   `alloy_primitives::B256`). The trigger_runtime has `QUASI_FINALITY_CONFIRMATIONS` and
   `FINAL_CONFIRMATIONS` constants and `FinalityRequirement` types.

4. **CLI commands**: `roko trigger list|show|create|fire|history` (in
   `commands/trigger.rs`). These manage TOML files in `.roko/triggers/`.

5. **HTTP routes**: `routes/triggers.rs` exposes CRUD and lifecycle endpoints.

6. **Durable history**: `TriggerLifecycleEvent` writes to `.roko/triggers/<name>/history.jsonl`.

### User observability

- **CLI**: `roko trigger list` -- list bindings. `roko trigger fire <name>` -- manual fire.
  `roko trigger history <name>` -- show firing history.
- **HTTP**: Full CRUD routes on `/api/triggers/*`.

### Honest assessment

The trigger system is substantial and tested. Cron with timezone support and file watching
work. However:

- **EVM/chain triggers**: The alloy imports and finality constants exist, but the chain
  indexer/watcher that would actually detect on-chain events and fire triggers is described
  as "product work" in CLAUDE.md. The ABI parsing code is present, the runtime
  infrastructure to actually listen to a chain is not.
- **mTLS**: `trigger_tls.rs` exists but actual deployment of CA-verified mTLS connections
  for trigger sources requires infrastructure that is configuration-dependent.
- **Graph execution on fire**: The binding references a graph, but the trigger runtime
  dispatches through the server's CLI bridge rather than executing a Graph directly.

---

## 6. Cross-Cut Functors (E44)

**Crate**: `crates/roko-compose/src/` (cross_cut.rs, memory_functor.rs,
daimon_functor.rs, dreams_functor.rs, safety_functor.rs)

**Rating: PARTIAL**

### What it claims
Memory/Daimon/Dreams/Safety functors, six transforms, conflict VCG, and a live
non-blocking gate-failure cascade.

### What actually happens

1. **The trait and EnrichedCell wrapper are real** (cross_cut.rs). `CrossCutFunctor` has
   `pre_enrich` / `post_enrich` methods. `EnrichedCell::execute` runs pre-hooks in
   order, the inner operation, then post-hooks in reverse. One test verifies ordering.

2. **Four functor implementations exist**:
   - `MemoryFunctor`: queries KnowledgeStore, injects relevant entries as pre-enrichment
     signals, writes gate-feedback reinforcement in post. ~3 tests.
   - `DaimonFunctor`: reads PAD vector, somatic landscape, applies prospect-theory
     valuation. ~3 tests.
   - `DreamsFunctor`: injects dream routing advice and replay candidates. ~2 tests.
   - `SafetyFunctor`: checks taint levels, quarantine status. ~2 tests.

3. **Gate-failure cascade is wired** (`spawn_cross_cut_gate_failure_cascade` at
   event_loop.rs:7666). When a gate fails, this spawns a background task that runs the
   `run_gate_failure_cascade` natural transform, which chains Memory -> Daimon -> Dreams
   through `eta_DM` and `eta_DN` natural transforms.

4. **Natural transforms exist** (natural_transforms.rs via compose/lib.rs): `eta_DM`
   converts dream reports to knowledge entries. `eta_DN` converts dream reports to
   affect events with optional depotentiation.

### User observability

- **CLI**: None. No command to inspect functor state or force a cascade.
- **TUI**: None.
- **Observation**: The cascade is a background fire-and-forget task with only debug/warn
  log lines.

### Honest assessment

The gate-failure cascade is genuinely wired and runs real code that chains
Memory -> Dreams -> Daimon. This is the one path where functors execute end-to-end. But:

- The `EnrichedCell` wrapper is **not used by the main dispatch path**. The runner's
  `dispatch_agent_with` does not wrap agent calls through an `EnrichedCell`. The functors
  are only exercised through the gate-failure cascade path.
- The "six transforms" exist as `eta_*` functions but only `eta_DM` and `eta_DN` are
  called by the runtime.
- VCG conflict resolution exists in `auction.rs` but is not invoked by the runtime.

---

## 7. Agent Cognitive Autonomy (E23)

**Crate**: `crates/roko-runtime/src/heartbeat.rs` (2,717 LOC),
`crates/roko-learn/src/active_inference.rs` (508 LOC),
`crates/roko-daimon/src/` (cognitive energy, behavioral phases, goal tree)

**Rating: PARTIAL**

### What it claims
Lifecycle type-state, behavioral vitality, CorticalState energy fields, energy
accounting, adaptive timescales, energy/affect coupling, EFE routing, GoalTree,
SlotManager, revisioned mode owners, and phase-aware runner dispatch.

### What actually happens

1. **CorticalState is fully implemented** (heartbeat.rs:353). A lock-free atomic struct
   with 20+ fields: PAD vector, accuracy, surprise rate, creative mode, regime, cognitive
   energy, fatigue penalty, EFE last tier, behavioral state. All fields have typed
   accessors. Tests exist.

2. **EFE routing is wired** (event_loop.rs:7447). `EfeRouter::default().route()` selects
   model tiers based on surprise rate, regime, and task difficulty. Test at line 16620
   verifies the routing is inert at default energy and caps when active.

3. **Cognitive energy tracking is wired** through DaimonState. `cognitive_energy.current`
   and `behavioral_phase()` are read at dispatch time (event_loop.rs:7320).

4. **Phase-aware dispatch is real**: `phase_capped_model` at event_loop.rs:7380 and
   `cognitive_cost_adjusted_model` at event_loop.rs:7488 use behavioral phase to cap
   model tier selection.

5. **GoalTree exists in daimon** (goals.rs, 681 LOC). Seeds, evidence accumulation,
   promotion, hierarchy, pruning, and decay are all implemented with tests.

6. **SlotManager exists** (lifecycle.rs:809 in roko-agent). Capacity-bounded named
   slots with activate/block/complete/reset transitions.

7. **ThetaConsumer and DeltaConsumer** exist as heartbeat rhythm processors
   (theta_consumer.rs, delta_consumer.rs). They compute affect updates, calibration
   checks, and low-activity detection.

### User observability

- **CLI**: No dedicated cognitive autonomy command. The effects are visible only through
  model selection behavior in plan execution logs.
- **TUI**: No dedicated tab. Heartbeat animation is cosmetic, not CorticalState-derived.

### What is NOT wired

- **CorticalState is never instantiated by the runner or serve**. The runner uses
  DaimonState's cognitive energy directly, not the CorticalState atomic struct. Neither
  roko-cli nor roko-serve imports or creates a CorticalState.
- **ThetaConsumer and DeltaConsumer are never instantiated** by any production code path.
  They exist as independently tested modules.
- **Lifecycle type-state** (Created -> Funded -> Active -> etc.) exists as types in
  roko-agent/lifecycle.rs but the runner does not manage agents through these transitions.
- **GoalTree** is maintained inside DaimonState but nothing surfaces emergent goals to the
  user.

### Honest assessment

The cognitive autonomy claim is the most inflated of the twelve features. The real
runtime effect is narrow: DaimonState's cognitive energy and behavioral phase modulate
model tier selection at dispatch time, and EFE routing provides a belief-state tier
selector. These are legitimate. But CorticalState (the crown jewel at 2,717 LOC), the
heartbeat consumers, lifecycle type-state, and revisioned mode owners are all built but
never instantiated by any production code path. They are tested infrastructure waiting
for a consumer.

---

## 8. Continuous Feeds (E27)

**Crate**: `crates/roko-core/src/feed*.rs` (1,542 LOC total), `crates/roko-serve/src/routes/feeds.rs`

**Rating: PARTIAL**

### What it claims
Cell-composed runtime feeds, discovery/lifecycle, Bus bridging, built-in source feeds,
validated recipe DAG persistence/evaluation, HTTP routes, and CLI commands.

### What actually happens

1. **Feed types and contracts exist** (feed.rs, feed_cell.rs, feed_runtime.rs,
   feed_bus_bridge.rs). `FeedInfo`, `FeedKind`, `FeedAccess`, `FeedPricingConfig`,
   `FeedRuntimeStatus` are all defined.

2. **HTTP routes are wired** (routes/feeds.rs, 995 LOC). Full CRUD plus runtime
   status, discovery, search, health, start/stop endpoints. The routes manipulate
   `AppState` feed registries.

3. **CLI commands are wired** (commands/feed.rs). `roko feed list|status|start|stop|
   health|discover|search`. These are HTTP client commands that query `roko serve`.

4. **Recipe routes exist** (routes/recipes.rs) for DAG recipe management.

### User observability

- **CLI**: `roko feed list`, `roko feed status <id>`, `roko feed start <id>`.
- **HTTP**: Full REST API at `/api/feeds/*`.

### Honest assessment

The feed infrastructure is real but thin. The types, routes, and CLI are wired. But:

- Feeds depend on `roko serve` running. Without the server, feeds do not exist.
- The actual feed data sources (what produces feed data) are registration stubs --
  you can register and start/stop feeds, but the built-in source feeds (provider health,
  file watch) produce data only when the corresponding serve subsystem is active.
- The "Cell-composed" aspect (feed_cell.rs) defines composition contracts but does not
  have a demonstrated composition chain in production.
- Bus bridging (feed_bus_bridge.rs, 121 LOC) is a small adapter module.

---

## 9. Agent Groups (E28)

**Crate**: `crates/roko-core/src/groups.rs` (585 LOC),
`crates/roko-serve/src/group_runtime.rs` (1,407 LOC, 4 tests),
`crates/roko-serve/src/routes/groups.rs` (867 LOC)

**Rating: PARTIAL**

### What it claims
Persisted invitations/membership, permissions, coordination, knowledge/pheromone/message/
event flows, Bus publication, HTTP/CLI operations, and privacy-filtered group prompt
context.

### What actually happens

1. **Group types are complete** (groups.rs). `Group`, `GroupId`, `GroupConfig`,
   `CoordinationMode`, `MemberRole`, `MemberPermissions`, `InviteRequest`,
   `GroupEvent`, `KnowledgePolicy` are all defined.

2. **Group runtime is implemented** (group_runtime.rs, 1,407 LOC). `GroupMutation` enum
   handles Create/Update/Delete groups, invite/accept/reject/remove members, update
   permissions, publish knowledge/pheromones/messages. 4 integration tests verify
   group creation, invitation flow, knowledge publication, and pheromone deposit.

3. **HTTP routes are full** (routes/groups.rs). Endpoints for groups CRUD, invite,
   accept/reject, members CRUD, knowledge publish/list, pheromone deposit/list,
   message publish, event listing.

4. **System prompt context exists** (compose/system_prompt_builder.rs,
   compose/context_provider.rs). Group membership context is injected into agent
   prompts with privacy filtering.

### User observability

- **CLI**: **No `roko group` command exists**. Groups are HTTP-only via `roko serve`.
- **HTTP**: Full REST API at `/api/groups/*`.

### Honest assessment

The HTTP layer for groups is complete and tested. The types and mutation logic are real.
But:

- There is no CLI command for groups -- you must use curl/HTTP against `roko serve`.
- Groups are never used by the runner-v2 plan execution loop. No plan task is aware of
  agent group membership.
- Pheromone flows are a data model (publish/read) but there is no consumer that uses
  pheromone data to influence behavior.
- Knowledge publication to groups writes entries but the knowledge is not automatically
  retrieved by group members during dispatch.
- Coordination modes (Consensus, LeaderFollower, etc.) exist as enum variants but have
  no behavioral implementation.

---

## 10. Telemetry Lens (E33)

**Crate**: `crates/roko-runtime/src/lens_executor.rs`, `crates/roko-serve/src/routes/projections.rs`

**Rating: WORKING**

### What it claims
11 built-in Lens executors, bounded queued delivery, breaker controls, typed StateHub
aggregation, REST/SSE, restart-durable history, resolution queries, configurable 7-day
retention, and all 39 production variants.

### What actually happens

1. **LensExecutor is implemented** (lens_executor.rs). It binds the declarative routing
   table to named `TelemetryObserve` implementations. Raw-event fan-out is concurrent.
   Circuit breaker policy (overhead budget, sample threshold, disable threshold) isolates
   Lens failures. Backpressure queue with configurable capacity (default 1,024) and
   drop-oldest policy.

2. **Built-in Lens types exist**: `AnomalyLens`, `CollectiveIntelligenceLens`,
   `EfficiencyLens`, `LatencyLens`, `QualityLens`, `TrendLens`, `UsageLens`, plus a
   health Lens factory. These are imported and instantiated.

3. **HTTP routes are wired** (projections.rs:38-60). `statehub/lens-runtimes`,
   `statehub/lens-runtimes/{id}`, and per-lens reset/enable/disable endpoints.

4. **StateHub aggregation and SSE streaming** exist (projections.rs:29-30,
   `stream_telemetry`, `stream_projection`).

5. **Runner integration** (event_loop.rs:3725 via gate_dispatch.rs). Telemetry events
   from gate completion flow into the Lens system.

6. **Tests**: 10 tests in `roko-runtime/tests/lens_executor.rs`.

### User observability

- **HTTP**: `/api/projections/telemetry`, `/api/projections/telemetry/stream` (SSE),
  `/api/statehub/lens-runtimes`, per-lens control endpoints.
- **TUI**: Telemetry data feeds into dashboard projections.
- **CLI**: No dedicated `roko telemetry` command, but data is available through
  projections when `roko serve` is running.

### Honest assessment

The Lens system is genuinely functional. Circuit breakers, backpressure, concurrent
dispatch, and REST/SSE exposure are all real. The 39 production event variants are
defined in roko-core and emitted by various subsystems. The main caveat is that full
Lens utilization requires `roko serve` to be running, and the "11 built-ins" vary in
sophistication -- some are substantial (LatencyLens, QualityLens), others are thin
aggregators.

---

## 11. Named Surfaces (E37)

**Crate**: `crates/roko-serve/src/projection_contract.rs` (1,415 LOC route file)

**Rating: PARTIAL**

### What it claims
Typed Workbench/Inbox/Canvas/Minimap/Autonomy projections, five dedicated
StateHub-backed routes, OpenAPI, events, object types, and legacy-tab mapping.

### What actually happens

1. **Five projection structs are defined** (projection_contract.rs):
   - `WorkbenchProjection`: active flows, pending approvals, recent completions
   - `InboxProjection`: inbox items with categories and urgency levels
   - `CanvasProjection`: cost meters, gate pipeline, active tasks, cohort health
   - `MinimapProjection`: plan progress, task tree, cost breakdown
   - `AutonomyProjection`: config struct passthrough

2. **Five HTTP routes are wired** (projections.rs:31-35):
   `/api/projections/workbench`, `/api/projections/inbox`, `/api/projections/canvas`,
   `/api/projections/minimap`, `/api/projections/autonomy`.

3. **Implementation functions exist** (projection_contract.rs:934-1055):
   `workbench_surface()`, `inbox_surface()`, `canvas_surface()`, `minimap_surface()`,
   `autonomy_surface()`. These read from `RuntimeProjectionSet` which loads from
   the canonical snapshot, episodes, efficiency events, and cost logs.

4. **TUI has surface references** (tui/tabs.rs:11-14). `V2Surface::Workbench`,
   `V2Surface::Canvas`, `V2Surface::Inbox` are defined and mapped to tab groups
   (Dashboard -> Workbench+Inbox, Plans -> Canvas+Flows).

### User observability

- **HTTP**: Five named REST endpoints return typed projection data.
- **TUI**: Surface names are mapped to tabs, but the actual TUI rendering reads from
  the snapshot file and local state rather than calling the HTTP projection endpoints.

### Honest assessment

The HTTP API layer for named surfaces is real and returns structured projection data.
But:

- **TUI rendering is NOT driven by named surfaces**. The TUI reads `.roko/state/`
  snapshots directly rather than consuming the projection API. The surface names in
  TUI tabs are labels, not data sources.
- `AutonomyProjection` is just a passthrough of `AutonomyConfig` -- it does not project
  any actual autonomy state.
- The "OpenAPI" claim refers to structural JSON responses, not a published OpenAPI spec.
- The projections populate from whatever data happens to be in the snapshot files, not
  from live event streaming in most cases.

---

## 12. Inference Gateway (E26)

**Crate**: `crates/roko-gateway/` (16 source files, 1,474 LOC main files, ~13 tests)

**Rating: PARTIAL**

### What it claims
Nine-stage gateway owning routing/fallback, exact and semantic caches, tool/output/
thinking controls, convergence, cost accounting, key rotation, three-level backpressure,
handles, batches, events, and authenticated serve routes.

### What actually happens

1. **Nine pipeline stages are defined** (gateway.rs:42-61):
   `LoopDetect`, `CacheLookup`, `ToolPrune`, `OutputBudget`, `ThinkingCap`,
   `ConvergenceDetect`, `ProviderCall`, `CacheStore`, `CostTrack`.

2. **Each stage has an implementation module**: backpressure.rs, cache.rs (with simhash),
   convergence.rs, cost_track.rs, handle.rs (InferenceHandle), loop_detect.rs,
   output_budget.rs, provider.rs (ProviderBackend trait), thinking_cap.rs, tool_prune.rs.

3. **GatewayConfig is constructed** and the gateway is instantiated by `roko serve`
   (state.rs:884-893). `InferenceGateway::new(gateway_config)` creates the pipeline.

4. **HTTP routes exist** (routes/gateway.rs, 1,768 LOC). Inference request endpoint,
   batch submit, batch flush, stats, provider health, and key rotation endpoints.

5. **InferenceClient trait and streaming** exist (lib.rs). The gateway exposes both
   request-response and streaming interfaces.

### User observability

- **HTTP**: Gateway routes at `/api/gateway/*` when `roko serve` is running.
- **CLI**: No dedicated `roko gateway` command.

### What is NOT wired

- **The runner does NOT use the gateway**. Runner-v2 dispatches agents through
  `roko-agent` provider infrastructure directly. There is no `roko_gateway` import
  anywhere in `roko-cli/src/runner/`. The gateway is only accessible through `roko serve`
  HTTP routes.
- **Key rotation** exists as a route but the actual rotation logic depends on the
  provider backend implementations.
- **Semantic cache** (`simhash` in cache.rs) exists but the effectiveness depends on
  query volume through the serve path.

### Honest assessment

The gateway is the canonical example of "built but not wired into the main code path."
All nine stages are implemented with tests. The HTTP serve path instantiates and uses it.
But the primary execution path -- `roko plan run` through runner-v2 -- **completely
bypasses the gateway** and dispatches directly through roko-agent providers. The gateway
only runs when you route inference requests through `roko serve`'s HTTP API, which is
not what the plan executor does.

---

## Summary Table

| # | Feature | Rating | Runtime Wired? | CLI/TUI Observable? | Main Gap |
|---|---------|--------|---------------|-------------------|----------|
| 1 | Daimon/Affect | **WORKING** | Yes (dispatch modulation) | Partial (no status cmd) | No CLI to inspect affect state |
| 2 | Dreams | **WORKING** | Yes (post-plan, cascade) | Yes (CLI + TUI tab) | LLM review needs configured provider |
| 3 | HDC Vectors | **WORKING** | Yes (episodes, knowledge) | Partial (file-level) | Advanced math modules unused |
| 4 | Knowledge Tiers | **WORKING** | Yes (progression, decay) | Yes (CLI commands) | Falsifier automation incomplete |
| 5 | Trigger Runtime | **PARTIAL** | Yes (serve only) | Yes (CLI + HTTP) | EVM chain listener not implemented |
| 6 | Cross-Cut Functors | **PARTIAL** | Gate-failure only | No | Main dispatch path does not use EnrichedCell |
| 7 | Cognitive Autonomy | **PARTIAL** | Narrow (energy, EFE) | No | CorticalState, heartbeat consumers never instantiated |
| 8 | Continuous Feeds | **PARTIAL** | Serve only | Yes (CLI + HTTP) | Thin; depends on serve |
| 9 | Agent Groups | **PARTIAL** | Serve only | HTTP only (no CLI) | Not used by runner; coordination modes unimplemented |
| 10 | Telemetry Lens | **WORKING** | Yes (gate events) | Yes (HTTP/SSE) | Requires serve for full value |
| 11 | Named Surfaces | **PARTIAL** | HTTP layer only | HTTP only | TUI does not consume projections API |
| 12 | Inference Gateway | **PARTIAL** | Serve only | HTTP only | Runner bypasses gateway entirely |

### Overall Assessment

**4 of 12 features are genuinely WORKING** end-to-end (Daimon, Dreams, HDC, Knowledge Tiers).
These four have real runtime effects on plan execution behavior, produce observable outputs,
and are accessible via CLI.

**8 of 12 features are PARTIAL**. They have real, tested implementations but are either:
- Only accessible through `roko serve` HTTP and not used by the primary runner-v2 execution
  path (Triggers, Feeds, Groups, Surfaces, Gateway)
- Have impressive infrastructure that is never instantiated by production code
  (CorticalState, heartbeat consumers, EnrichedCell in main dispatch)

**None are STUBBED or BROKEN** -- every feature has real code that compiles, has tests,
and does something. The gap is consistently in the "wired into the main execution path"
dimension rather than in code quality.

### The Pattern

The recurring pattern across all 12 features is the same one CLAUDE.md warns about:
"built but never connected." The HTTP serve layer gets the wiring because routes are
easy to add. The runner-v2 event loop gets selective wiring for the features that
directly modulate dispatch (Daimon, HDC, Knowledge). But the features that would need
architectural changes to the dispatch loop (gateway as intermediary, EnrichedCell wrapping
every call, CorticalState as the shared perception surface) remain standalone modules
with test coverage but no production callers.
