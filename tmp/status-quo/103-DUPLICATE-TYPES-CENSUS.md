# 103 — Duplicate Types Census

> **Verification header**
> - Repo HEAD: `5852c93c05a4f1bda8ff880fc752d9fba2ba453e` (branch `main`)
> - Date: 2026-07-08
> - Method: `rg` frequency census of `pub struct`/`pub enum` names across `crates/` (excluding `target/`), then every definition site opened and compared field-by-field; `impl From<…>` grep for conversion coverage.
> - Scope: workspace `/Users/will/dev/nunchi/roko/roko/crates/` (18 crates)
> - Status tags: `[SPLIT-BRAIN]` competing runtime shapes · `[ORPHAN-DUP]` one copy dead/unused · `[SEMANTIC-COLLIDE]` same name, unrelated concept · `[NEAR-CLONE]` byte-near copies · `[BENIGN]` local/test-only, no runtime impact

This doc consolidates and extends `47-FOUNDATION-TYPES`, `76-DATA-CONTRACTS-SCHEMAS`, and `08-TECH-DEBT`. It is the canonical duplicate-type register. Ground-truth corrections vs prior notes are flagged inline.

---

## 0. Method note & raw census

Frequency census (`pub struct|enum` name, count ≥ 3 shown; full ≥2 list mined):

```
16 Handler        9 Foo(test)     5 TaskDef        4 GateVerdict    4 EventBus
 4 AgentState     3 ToolsConfig   3 ToolCallSummary 3 TaskSummary   3 TaskContext
 3 RetentionPolicy 3 RateLimiter  3 PlanEntry      3 Plan           3 Observation
 3 ModelPricing   3 JsonRpcError  3 GateFeedback   3 DashboardSnapshot 3 CostTable
 3 Config         3 BudgetConfig  3 AgentConfig
```

`Handler`, `Foo`, and most `*Config`/`*Error`/JSON-RPC repeats are **[BENIGN]** — per-crate local structs or test fixtures with no shared contract. This census keeps the families that carry a **cross-crate runtime contract** (state that flows between plan execution, gates, dashboard, storage, learning, chain). 21 such families found.

**Conversion coverage is almost nil.** The only `impl From<…>` touching any of these families are *one-directional adapters into learning-side outcome types* (`From<&GateVerdict> for ProviderModelGateOutcome`, `From<&GateVerdict> for GateStatus`, `From<&Episode> for RuntimeEpisodeObservation`) and each StateHub crate's own `From<StateHub> for SharedStateHub`. **Zero** `From`/`Into` exist *between* competing definitions of the same-named family. Every crossing is a manual re-map or a hard wall.

---

## 1. Master table — sorted by blast radius

| # | Type family | Sites | Tag | Conversions between sites | Runtime consequence | Canonical owner | Difficulty |
|---|---|---|---|---|---|---|---|
| 1 | **DashboardSnapshot** | 3 | SPLIT-BRAIN | none | Dashboard emptiness: web/serve reads StateHub-fed `roko-core` snapshot; TUI renders its **own** file-scraping snapshot; projection has a third. Three "current state" truths. | `roko-core::dashboard_snapshot` | High |
| 2 | **StateHub** (+`SharedStateHub`,`StateHubSender`) | 2 | ORPHAN-DUP / NEAR-CLONE | own-crate only | `roko-core` copy is a byte-near orphan; everyone imports `roko-runtime`'s. Dead maintenance surface that will drift. | `roko-runtime::state_hub` | Low |
| 3 | **GateVerdict** | 4 | SPLIT-BRAIN | none | Gate result can't flow uniformly: exec uses core, learning re-hashes its own, dashboard has a 3rd, chain a 4th. Every hop re-maps by hand → lossy. | `roko-core::foundation` | High |
| 4 | **RetentionPolicy** | 3 | SPLIT-BRAIN | none | Storage split-brain: three GC/compaction engines (fs GC, serve rotation, learn compaction) with independent, non-shared policies → episodes pruned under 3 different rules. | *new shared* (see roadmap) | Med |
| 5 | **AgentState** | 4 | SPLIT-BRAIN / NEAR-CLONE | none | Agent status fragmented across dashboard/runtime/agent/sidecar; two are near-clones (runtime vs agent lifecycle). No single agent-health truth. | `roko-runtime::lifecycle` | Med |
| 6 | **TaskStatus** | 3 | SPLIT-BRAIN | none | Task lifecycle enums disagree on variants (core 4 / runtime 5 / tui 5). Status shown in TUI ≠ scheduler's ≠ core's. Manual match arms everywhere. | `roko-core::task` | Med |
| 7 | **GateFeedback** | 3 | SPLIT-BRAIN | none | Retry-prompt feedback re-parsed 3 ways (gate rung feedback / compose retry / cli dispatch). Feedback quality depends on which parser ran. | `roko-gate::feedback` | Med |
| 8 | **EventBus** | 4 | SPLIT-BRAIN | none | Four event-fan-out impls + separate event vocabularies. No unified event stream; each subsystem has its own bus + event enum. | `roko-runtime::event_bus` | High |
| 9 | **DispatchPlan** (semantic) | 3 | SEMANTIC-COLLIDE | n/a | `roko-core::DispatchPlan` (composition), `ExecutorAction::DispatchPlan` (action variant), cli `RunnerDispatchPlan` (renamed). Same word, 3 unrelated concepts → reader confusion, grep noise. | keep separate, rename | Low |
| 10 | **Engram** | 2 | ORPHAN-DUP | none | `roko-chain` copy is a "forensic replay stub", never wired; the real one is `roko-core`. Dead duplicate. | `roko-core::engram` | Low |
| 11 | **Cell** (trait) | 2 | SPLIT-BRAIN | none | `roko-core::cell::Cell` (supertrait of the 6 verb traits) vs `roko-graph::cell::Cell` (graph-node backing). A core Cell impl is **not** a graph node without an adapter. Two execution kernels. | `roko-core::cell` | High |
| 12 | **PromptAssembler** | 2 | SPLIT-BRAIN | none | 4th prompt-assembly surface: `roko-compose` templates vs cli `dispatch::prompt_builder`. Prompt shape depends on path. | `roko-compose` | Med |
| 13 | **PlanState** | 2 | SPLIT-BRAIN | none | Dashboard `PlanState` (view model) vs orchestrator `plan_state::PlanState` (execution state). Overlap, no bridge. | context-dependent | Med |
| 14 | **TaskState** | 2 | SEMANTIC-COLLIDE | none | Dashboard `struct TaskState` (view) vs agent-server `enum TaskState` (lifecycle). Same name, struct vs enum. | keep separate, rename | Low |
| 15 | **GateResult** | 2 | SPLIT-BRAIN | none | `roko-acp::workflow` vs orchestrator `plan_state`. Adjacent to GateVerdict family; another gate-result shape. | fold into GateVerdict | Med |
| 16 | **Verdict** | 2 | SEMANTIC-COLLIDE | none | `roko-core::verdict::Verdict` (struct) vs `roko-learn::heuristics::Verdict` (enum). | keep separate, rename | Low |
| 17 | **Outcome** | 2 | SEMANTIC-COLLIDE | none | `roko-core::verdict::Outcome` (struct) vs `roko-chain::identity_economy_markets::Outcome` (enum). | keep separate, rename | Low |
| 18 | **Plan** | 3 | SEMANTIC-COLLIDE | none | cli `plan.rs`, cli `runner/plan_loader.rs`, serve `plan_types.rs` — three plan view/loader models. | consolidate cli-side | Med |
| 19 | **TaskDef** | 2 | NEAR-CLONE | none | cli `task_parser::TaskDef` vs compose `symbol_resolver` test literal. (Compose one is a test fixture string → mostly BENIGN.) | `roko-cli::task_parser` | Low |
| 20 | **Config** | 3+ | BENIGN | n/a | cli / lang-rust / others — unrelated per-crate config roots. No shared contract. | n/a | n/a |
| 21 | **BudgetConfig / AgentConfig / ModelPricing / CostTable / TaskSummary / ToolCallSummary** | 2–3 each | BENIGN→WATCH | none | Mostly per-crate views of budget/cost/summary. Low blast radius today but converging concepts worth watching for future contract drift. | per-crate | Low |

**Total cross-contract duplicated families: 19** (rows 1–19; rows 20–21 are benign/watch-list). Highest blast radius: **DashboardSnapshot** (row 1) — the direct cause of dashboard emptiness, coupled to StateHub.

---

## 2. Family detail

### 2.1 DashboardSnapshot ×3 `[SPLIT-BRAIN]` — highest blast radius

| Site | Shape | Fed by | Read by |
|---|---|---|---|
| `crates/roko-core/src/dashboard_snapshot.rs:759` | Rich: `plans/tasks/agents` maps, `gates` ring, diagnoses, experiment_winners, efficiency/cfactor trends, episodes, errors, event log | `StateHub` via `watch::send_modify` | **serve** (`routes/workflows.rs:22`, `status/health.rs:204`) |
| `crates/roko-cli/src/runner/projection.rs:124` | Thin: `events: VecDeque<ProjectionEvent>` only | `Projection` broadcast | runner projection tests / TUI event feed |
| `crates/roko-cli/src/tui/dashboard.rs:3308` | File-scraper: `episode_count`, `success_rate`, `efficiency_events`, `experiments`, `adaptive_thresholds`, `cascade_snapshot`, `skills`… all read directly from `.roko/learn/*.json` + `.roko/engrams.jsonl` | direct file reads | **TUI render** |

**Consequence.** `serve` renders the StateHub-populated core snapshot; the **TUI ignores it** and re-derives state by scraping `.roko/` files. So the "live" HTTP dashboard and the "live" terminal dashboard are fed by two entirely different pipelines, and if StateHub isn't being published to (or the TUI's files are stale), each shows a different — often empty — picture. This is the structural root of "dashboard emptiness." Fixing it means the TUI must consume `roko-core::DashboardSnapshot` via a `watch::Receiver` instead of scraping files.

### 2.2 StateHub ×2 `[ORPHAN-DUP]`/`[NEAR-CLONE]`

- `crates/roko-core/src/state_hub.rs:71` — fields `snapshot_tx/rx`, `event_bus`, `event_log`. **Orphan**: no `roko_core::…::StateHub` importer found outside its own file/docs.
- `crates/roko-runtime/src/state_hub.rs:80` — byte-near identical, plus `with_event_log(...)`. This is the live one: `roko-serve/src/lib.rs:59` re-exports it, `roko-cli/src/lib.rs:37` re-exports `roko_runtime::state_hub::*` as `crate::state_hub`.

Both carry parallel `SharedStateHub` + `StateHubSender` + `From<StateHub> for SharedStateHub`. The core copy is dead weight that will silently drift from the runtime one. **Delete the core copy** (or make core re-export runtime — but that inverts the dep graph, so deletion is cleaner).

### 2.3 GateVerdict ×4 `[SPLIT-BRAIN]`

*(Correction vs prior ground-truth note: the dashboard variant lives in `roko-core/src/dashboard_snapshot.rs:290`, not `roko-serve`.)*

| Site | Fields |
|---|---|
| `roko-core/src/foundation.rs:368` (canonical, gate exec) | `gate_name, passed, skipped, skip_reason, output, duration_ms` |
| `roko-core/src/dashboard_snapshot.rs:290` (dashboard ring) | `plan_id, task_id, gate, passed, ts_millis` |
| `roko-learn/src/episode_logger.rs:90` (episode record) | `gate, passed, signature` (hashed, never raw) |
| `roko-chain/src/identity_economy_identity.rs:1600` (futures stub) | `gate: GateType, passed, score, detail` |

Four incompatible field sets. A gate runs once (foundation shape) but must be manually re-projected into the dashboard shape, the episode shape, and (theoretically) the chain shape — each hop dropping fields (`output`, `duration_ms`, `skip_reason` are lost by episode & dashboard). No `From` chain links them. `GateReport { verdicts: Vec<GateVerdict> }` binds only to the foundation one.

### 2.4 RetentionPolicy ×3 `[SPLIT-BRAIN]` — storage split-brain

| Site | Fields | Governs |
|---|---|---|
| `roko-fs/src/gc.rs:32` | `max_episodes(200), max_run_age_days(7), max_archive_age_days(30), size_threshold_mb(500), max_cache_entries(2000)` | whole `.roko/` GC sweep |
| `roko-serve/src/retention.rs:20` | `artifact, path, max_age_hours, max_size_bytes, strategy: CompactionStrategy` | per-artifact rotation/compaction |
| `roko-learn/src/episode_logger.rs:1229` | `max_episodes(200), max_age_days(90)` | episode-log compaction |

Three engines prune the same data (`episodes.jsonl`) under three unrelated policies — fs GC caps at 200 episodes / 7-day runs, learn caps at 200 / 90 days, serve rotates by hours+bytes. Whichever runs "wins," and none of them knows about the others. Result: nondeterministic retention, and config set in one place is invisible to the others.

### 2.5 AgentState ×4 `[SPLIT-BRAIN]`/`[NEAR-CLONE]`

| Site | Shape |
|---|---|
| `roko-core/src/dashboard_snapshot.rs:259` | view: `agent_id, role, active, bytes…` |
| `roko-runtime/src/lifecycle.rs:430` | `agent_id: Option, resources, neuro_initialized, routing_configured…` |
| `roko-agent/src/lifecycle.rs:486` | near-clone of runtime: `resources, neuro_initialized, routing_configured, tool_profile…` |
| `roko-agent-server/src/state.rs:465` | sidecar: `agent_id, owner, version, capabilities, log_path, routes, started_at…` |

runtime and agent lifecycle copies are near-duplicates that should be one; the dashboard and sidecar shapes are legitimately different *views* but share no common core → agent health is assembled differently in every surface.

### 2.6 TaskStatus ×3 `[SPLIT-BRAIN]`

- `roko-core/src/task.rs:66`: `Pending, Active, Done, Blocked`
- `roko-runtime/src/task_scheduler.rs:25`: `Blocked, Ready, Running, Completed, Failed{…}`
- `roko-cli/src/tui/state.rs:101`: `Pending, Active, Done, Failed, Blocked`

Three disjoint variant sets for one concept. The scheduler's `Failed` has no core equivalent; the TUI's `Failed` is a 3rd spelling. Every boundary needs a hand-written match. **Unify on a superset in `roko-core::task`**, have runtime/tui map from it.

### 2.7 GateFeedback ×3 `[SPLIT-BRAIN]`

- `roko-gate/src/feedback.rs:53`: `rung: u8, passed, errors, warnings, suggestions`
- `roko-compose/src/gate_feedback.rs:9`: `rung: u32, summary, diagnostics` (`from_raw`, bounded ≤24 lines)
- `roko-cli/src/dispatch/prompt_builder.rs:558`: `compile_errors, test_failures, clippy_warnings, raw_output`

Three parsers of the same cargo output for the same purpose (retry-prompt enrichment). Note even `rung` disagrees on type (`u8` vs `u32`).

### 2.8 EventBus ×4 + event vocabularies `[SPLIT-BRAIN]`

- `roko-runtime/src/event_bus.rs:236` `EventBus<E>` (generic, canonical)
- `roko-serve/src/event_bus.rs:25` `EventBus<E>` (generic, duplicate)
- `roko-learn/src/events.rs:80` `EventBus` (concrete)
- `roko-agent/src/task_runner.rs:101` `EventBus` (concrete)

Four fan-out implementations, each paired with its own event enum (`DashboardEvent`, projection events, learn `FeedbackEvent`, agent task events). No unified stream — a core reason cross-subsystem observability is patchy.

### 2.9 Cell trait ×2 `[SPLIT-BRAIN]` — two execution kernels

- `roko-core/src/cell.rs:91` — supertrait of Substrate/Scorer/Gate/Router/Composer/Policy (the "1 noun + 6 verbs" kernel).
- `roko-graph/src/cell.rs:74` — near-identical trait, but declared as the backing for every graph node.

Nearly the same method set (`cell_id`, `cell_name`, `cell_version`, `protocols`, …) defined twice. A type implementing `roko-core::Cell` is not automatically a `roko-graph::Cell` graph node → the protocol layer and the graph executor are two kernels bridged by adapters. High blast radius because it sits under everything.

### 2.10 DispatchPlan ×3 `[SEMANTIC-COLLIDE]`

- `roko-core/src/dispatch_plan.rs:75` — composition-time dispatch plan (model/route selection).
- `roko-orchestrator ExecutorAction::DispatchPlan {plan_id}` — an executor *action* variant.
- `roko-cli/src/dispatch/mod.rs:201 RunnerDispatchPlan` — cli already renamed to dodge the clash.

Not a data-contract split (they're unrelated), but a naming hazard: `rg DispatchPlan` returns three concepts. Keep separate, finish the rename discipline (`CompositionDispatchPlan` / `RunnerDispatchPlan` / action variant).

### 2.11 Engram ×2 `[ORPHAN-DUP]`

- `roko-core/src/engram.rs:63` — canonical: `id: ContentHash, fingerprint: Option<HdcFingerprint>, kind, body: Body`, serde.
- `roko-chain/src/identity_economy_markets.rs:653` — "forensic replay stub": `hash, kind, body: Vec<u8>, author, tags`. **Dead** — no runtime path constructs it. Remove or feature-gate.

---

## 3. Consolidation roadmap (ordered)

Ordered to unblock the two flagship symptoms first — **dashboard emptiness** (rows 1,2,3,5,8) and **storage split-brain** (row 4) — then reduce naming hazards.

1. **Kill orphan StateHub (row 2)** — *Low.* Delete `roko-core/src/state_hub.rs` (+ its `SharedStateHub`/`Sender`/`From`). Confirms `roko-runtime` as sole owner. Zero behavior change; removes a drift trap. **Do first — it de-risks step 2.**
2. **Unify DashboardSnapshot on `roko-core` (row 1)** — *High.* Make the TUI consume `roko-core::DashboardSnapshot` via a `watch::Receiver` from the runtime StateHub instead of scraping `.roko/*.json` in `tui/dashboard.rs`. Keep `projection::DashboardSnapshot` renamed to `ProjectionWindow` (it's a different thing). **This is the single highest-value fix — directly resolves dashboard emptiness.**
3. **Collapse GateVerdict to one core type + projections (row 3)** — *High.* Canonicalize `roko-core::foundation::GateVerdict`; add `From<&GateVerdict>` → dashboard summary and → `episode_logger::GateVerdict` (hashed) so the exec shape flows without hand re-maps. Fold `GateResult` (row 15) in. Delete/feature-gate the chain stub.
4. **Introduce one shared RetentionPolicy (row 4)** — *Med.* Create a single retention config (likely in `roko-fs` or `roko-core`) that fs GC, serve rotation, and learn compaction all read. Have the three engines dispatch off *one* policy struct. Unblocks storage split-brain and makes retention configurable in one place.
5. **Merge runtime/agent AgentState near-clones (row 5)** — *Med.* Fold `roko-agent::lifecycle::AgentState` into `roko-runtime::lifecycle::AgentState`; keep dashboard + sidecar as explicit `From`-derived views.
6. **Superset TaskStatus in core (row 6)** — *Med.* Extend `roko-core::task::TaskStatus` to the union (add `Ready/Running/Failed`), map runtime + tui via `From`.
7. **Unify GateFeedback (row 7)** — *Med.* One retry-feedback type in `roko-gate`; compose + cli parse into it. Fix `rung` to a single width.
8. **One EventBus (row 8)** — *High.* Standardize on `roko-runtime::event_bus::EventBus<E>`; serve drops its clone; learn/agent adopt it with their own `E`.
9. **Rename semantic collides (rows 9,14,16,17,18)** — *Low, mechanical.* `DispatchPlan`→context-qualified, `TaskState`/`Verdict`/`Outcome`/`Plan` disambiguated. Pure clarity, no contract change.
10. **Remove dead Engram + unify Cell (rows 10,11)** — *Low / High.* Delete chain `Engram`. Cell-trait unification (make `roko-graph` reuse `roko-core::Cell`) is high-effort and can trail the observability work.

---

## 4. Verification checklist

- [x] HEAD `5852c93c05` confirmed on `main`.
- [x] Frequency census run: `rg -o 'pub struct|enum \w+' | sort | uniq -c` (≥2 mined, ≥3 tabled).
- [x] Every tabled site opened and compared field-by-field (foundation/dashboard/episode/chain GateVerdict; 3× RetentionPolicy; 4× AgentState; 3× TaskStatus; 3× GateFeedback; 3× DashboardSnapshot; 2× StateHub; 2× Cell; 2× Engram).
- [x] Conversion coverage checked via `rg 'impl From<'` — confirmed **no** From/Into between competing same-name definitions; only one-way adapters into learning outcome types.
- [x] Consumer confirmed: serve reads `roko_core::dashboard_snapshot::DashboardSnapshot`; TUI reads its own file-scraper; cli/serve import `roko_runtime` StateHub (core copy orphan).
- [x] Ground-truth correction: dashboard `GateVerdict` is in `roko-core` (:290), not `roko-serve`.
- [ ] Not yet traced: whether `StateHub::publish` is actually called on every plan-run path (would confirm *why* the core snapshot is empty for the TUI even after step 2) — see `98-TRACE-SELF-HOSTING-LOOP`.
