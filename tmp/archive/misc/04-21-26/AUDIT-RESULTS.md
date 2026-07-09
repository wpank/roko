# Implementation Audit Results

> Generated 2026-04-22 by auditing all implementation plans in `PRDs/impl/` and `demo-parity/` against the actual codebase at `/Users/will/dev/nunchi/roko/roko`.

## Grand Summary

| Area | Done | Total | % |
|------|------|-------|---|
| Demo-Parity Stream B (Backend) | 7 | 61 | 11% |
| Demo-Parity Stream C (TUI) | 62 | 73 | 85% |
| 10-Dashboard/TUI + 11-Demo Sprint | 26 | 65 | 40% |
| 08-Surfaces/UX + 01-Runtime | 29 | 80 | 36% |
| 02-Cognitive Engine + 03-Context | 21 | 76 | 28% |
| 04-Knowledge thru 07-Chain | 24 | 133 | 18% |
| 09-Extensibility & Multichain | 1 | 39 | 3% |
| **GRAND TOTAL** | **170** | **527** | **32%** |

---

## Strongest Areas (>70% done)

- **C1 Marketplace tab**: 15/16 (94%)
- **C4 Config sub-views**: 14/15 (93%)
- **C2 Atelier tab**: 9/10 (90%)
- **Demo-Sprint 06 Backend+TUI stream**: 10/10 (100%)
- **08 CLI/Chat/TUI checklist**: 10/10 (100%)
- **10 Stabilization & Nexus**: 7/8 (88%)
- **C3 Inspect sub-views**: 14/14 (100%)
- **C5 Bug fixes**: 4/5 (80%)
- **07-chain InsightStore/tokenomics/HDC**: 6/8 (75%)

## Weakest Areas (0% done)

- **B1 Job type system in roko-core**: 0/3
- **B3 Incremental file watchers**: 0/7
- **B4 Server state persistence**: 0/7
- **B5 Research job execution pipeline**: 0/5
- **B6 Coding job execution pipeline**: 0/7
- **B7 Heartbeat protocol**: 0/7
- **05 Domain extensions, HF, market**: 0/8
- **06 Profile catalog, custom domains**: 0/16
- **06-ISFR Clearing runtime**: 0/9
- **06-ISFR Publication states/economics**: 0/16
- **09-EXT Ingestion/discovery/WorldGraph**: 0/9
- **09-EXT Fine-tuning integration**: 0/7
- **09-EXT Attention/publishing/ecosystem**: 0/15

---

# Detailed Checklists by Area

---

## Demo-Parity Stream B: Backend (7/61 = 11%)

### B1: Job type system in roko-core (0/3)

- [ ] Add `uuid` dependency to roko-core — not in Cargo.toml
- [ ] Create `roko-core/src/jobs.rs` with `JobType`, `JobState`, `Job`, `JobSubmission`, `FileJobStore`, etc. — file does not exist; `JobStatus` lives in `dashboard_snapshot.rs` instead
- [ ] Register module and re-exports in `roko-core/lib.rs` — absent

**Note:** Jobs are implemented differently than planned. `roko-serve/src/routes/jobs.rs` has `JobRecord`/`MarketplaceJob`/`JobStatus` but from `dashboard_snapshot.rs`, not a dedicated `jobs.rs` module.

### B2: Job API routes in roko-serve (2/5)

- [x] CORS fix — `CorsLayer::permissive()` when no origins configured
- [ ] ServerEvent variants `JobStateChanged`, `JobSubmitted`, `JobEvaluated` — actual events are `JobCreated { job: Value }`, `JobUpdated { job: Value }`, `JobTransitioned {job_id, from, to}` (different shape than planned)
- [ ] `FileJobStore` field in `AppState` — uses `StateHub` + direct file I/O instead
- [ ] 8 lifecycle endpoints (`/assign`, `/start`, `/submit`, `/evaluate`, `/cancel`, `/stats`) — only 5 routes exist: `GET /jobs`, `POST /jobs`, `GET /jobs/summary`, `GET /jobs/{id}`, `PATCH /jobs/{id}`
- [x] Wire `jobs::routes()` into the router — already merged in `build_router()`

### B3: Incremental file watchers (0/7)

- [ ] Add `IncrementalTailer<T>` to `jsonl_cursor.rs` — struct does not exist
- [ ] Add 6 new tests for `IncrementalTailer` — none exist
- [ ] Replace `efficiency_stamp` with `efficiency_tailer` — still uses `FileStamp`
- [ ] Replace `cfactor_stamp` with `cfactor_tailer` — still uses `FileStamp`
- [ ] Update `tick()` to use tailers — still uses file-stamp full-reads
- [ ] Add `summarize_efficiency_events` in-memory helper — not present
- [ ] Remove `DashboardDataStamps` efficiency/cfactor fields — still present

### B4: Server state persistence (0/7)

- [ ] `ServerStateSnapshot` struct in `state.rs` — does not exist
- [ ] `AppState::save_snapshot()` method — does not exist
- [ ] `AppState::restore_snapshot()` method — does not exist
- [ ] `spawn_auto_save()` background task — does not exist
- [ ] Save on graceful shutdown — not called
- [ ] Wire restore + auto-save into server startup — not wired
- [ ] 4 unit tests — not present

### B5: Research job execution pipeline (0/5)

- [ ] Create `crates/roko-cli/src/job_runner.rs` — file does not exist
- [ ] Register `pub mod job_runner;` — not present
- [ ] Wire `JobRunner` into `orchestrate.rs` — no import or usage
- [ ] `JobRunner` struct with `poll_and_execute()` — not present
- [ ] Research job subprocess spawning — not present

### B6: Coding job execution pipeline (0/7)

- [ ] Add `coding_task` match arm in `job_runner.rs` — B5 not done
- [ ] Add `execute_coding_job()` — not present
- [ ] Add `submit_failure()` — not present
- [ ] Add `parse_gate_results()` and `GateResult` struct — not present
- [ ] Add helper functions (`extract_first_error`, `extract_test_summary`, etc.) — not present
- [ ] PRD→plan→run→gate→submit pipeline — not present
- [ ] 8 new tests — not present

### B7: Heartbeat protocol (0/7)

- [ ] Create `roko-core/src/heartbeat.rs` with `HeartbeatPayload`, `NetworkStats` — file does not exist
- [ ] Register in `roko-core/lib.rs` — not present
- [ ] Add `heartbeats: RwLock<VecDeque<HeartbeatPayload>>` to `AppState` — not present
- [ ] Add `HeartbeatReceived` variant to `ServerEvent` — only has `Heartbeat` (server liveness), not `HeartbeatReceived`
- [ ] Create `routes/heartbeats.rs` — file does not exist
- [ ] Wire `heartbeats::routes()` into `mod.rs` — not present
- [ ] Emit heartbeats from `orchestrate.rs` — no usage found

### B8: WS event enrichment (2/7)

- [ ] Add test `job_events_serialize_with_type_tag` — not present (tests B2 variants that don't exist)
- [ ] Add test `heartbeat_event_serializes_with_type_tag` — not present (tests B7 variant)
- [ ] Add test `all_server_event_variants_have_type_tag` — not present
- [x] Verify WS filter matching via `matches_filter` — works for all `ServerEvent` variants
- [x] Verify event bus flow — `EventBus::publish()` → `broadcast::Sender` → `ws.rx.recv()` confirmed
- [ ] Manual WS verification (websocat) — not confirmed
- [ ] Document subscription protocol — not done

### B9: JWT Bearer auth middleware (3/9)

- [x] Read middleware file — exists
- [x] Check `base64` in deps — present in `roko-serve/Cargo.toml`
- [ ] Replace `require_api_key` with structural JWT validation — current impl accepts Bearer tokens as literal API keys, NOT structural JWT validation
- [ ] Add `extract_bearer_token()` helper — not present
- [ ] Add `is_structurally_valid_jwt()` helper — not present
- [ ] Add `is_base64url()` helper — not present
- [ ] Add `X-Auth-Method` response header — not implemented
- [ ] Add B9-specific integration tests — existing tests cover bearer-as-API-key only
- [x] Verify existing tests still pass — they do

### B10: Integration test script (4/4)

- [x] Create `tmp/04-21-26/demo-parity/integration-test.sh` — exists
- [x] Make script executable — `-rwxr-xr-x`
- [x] Create `crates/roko-serve/tests/job_lifecycle.rs` — exists and covers the lifecycle flow
- [x] Run smoke verification — `bash tmp/04-21-26/demo-parity/integration-test.sh` passed on 2026-04-22

**Follow-up correction:** The script was updated to use the current `HeartbeatPayload`
contract: numeric `frequency` and arbitrary numeric metrics under `metrics`.

---

## Demo-Parity Stream C: TUI (62/73 = 85%)

### C1: Marketplace tab (15/16)

- [x] `Tab::Marketplace` and `Tab::Atelier` variants exist
- [x] `Tab::ALL` is `[Tab; 9]`
- [x] F8 and F9 key bindings work in `fkey()` and `from_key()`
- [x] `next()`/`prev()` cycle through all 9 tabs
- [x] `SubView::JobList`, `JobDetail`, `CreateJob` exist
- [x] `SubView::PrdWorkshop`, `PlanExplorer` exist
- [x] `marketplace_view.rs` renders job list from `tui_state.marketplace_jobs`
- [x] Job type tags colored (research=rose, coding_task=bone, other=muted)
- [x] Job detail description word-wrapped
- [x] Empty state renders centered message
- [x] `atelier_view.rs` compiles and renders (full implementation)
- [x] `render_tab_content()` dispatches to both new views
- [x] Header bar shows F8/F9 labels
- [x] Status bar shows context-sensitive keybind hints
- [ ] `Block::bordered()` used throughout — `config_view.rs` still uses `Block::default().borders(Borders::ALL)` in some sub-views
- [x] All existing tests pass
- [x] Clippy/fmt clean

### C2: Atelier tab (9/10)

- [x] Full implementation replaces placeholder
- [x] Top stats bar renders 5 counters
- [x] Left panel lists PRDs with status badges and progress
- [x] PRD sorting by status weight
- [x] Right panel shows selected PRD metadata
- [x] Task list renders with status icons
- [x] Empty state renders
- [ ] TOML task parser handles `[[task]]` and `[[tasks]]` — architecture changed to StateHub-sourced data instead of local TOML parsing
- [x] `Block::bordered()` used throughout
- [x] Clippy/fmt clean, tests pass

### C3: Inspect sub-views (14/14)

- [x] `render()` dispatches on `view_state.sub_tab` (0=overview, 1=engram, 2=episode, 3=knowledge)
- [x] Overview sub_tab renders unchanged
- [x] `render_engram_dag()` renders with truncated hashes and ASCII tree
- [x] `render_engram_dag()` confidence bars — uses typed signal confidence with bounded scroll/selection helpers
- [x] `render_episode_replay()` renders with gate icon, task, role, model, turns, cost, timing
- [x] `render_knowledge_browse()` renders with topic, confidence bar, source, content
- [x] All three sub-views show empty-state messages
- [x] Case-insensitive filtering in KnowledgeBrowse — implemented via `ViewState::search_query`
- [x] `confidence_bar()` and `confidence_style()` shared functions exist
- [x] Scrolling via `view_state.scroll` works
- [x] Number keys 1-9 dispatch `TuiAction::SwitchSubView(index)`
- [x] `Block::bordered()` used throughout
- [x] Clippy/fmt clean
- [x] Tests pass

### C4: Config sub-views (15/15)

- [x] `render()` dispatches on sub_tab (0=editor, 1=provider health, 2=model comparison)
- [x] Config editor works identically to before
- [x] `render_provider_health()` reads from `tui_state.cascade_router`
- [x] Provider status green/amber/red/ghost per success rate
- [x] Em-dash for "no data" cells
- [x] `render_model_comparison()` aggregates from efficiency events + cascade router
- [x] Lowest-cost model bold green
- [x] Highest gate-rate model bold
- [x] Both sub-views render empty-state messages
- [x] `infer_provider()` and `infer_tier()` are standalone helpers
- [x] `truncate()`, `format_count()`, `center_rect()` helpers exist
- [x] Number keys switch sub-views
- [x] `Block::bordered()` used throughout
- [x] Clippy/fmt clean
- [x] Tests pass

### C5: Bug fixes (4/5)

- [x] Fix 1: Gate verdicts `vfy` column renders P/F/empty with correct colors
- [x] Fix 2: `cached_unified_log` rebuilt only when generation changes
- [ ] Fix 2 (dashboard reads cache): `dashboard_view.rs` does NOT use `tui_state.cached_unified_log` — cache is built but not consumed, O(N) per-frame rebuild may still exist
- [x] Fix 3: h/l/Enter on wave header dispatches `ExpandCollapse`
- [x] Fix 4: Git log uses NUL-separated format with proper parser
- [x] Fix 5: Dashboard tab hints show `R:retry D:diag` when `has_failures`

### C6: Header stats (12/12 + 1 N/A)

- [x] `TuiState` has `agents_online: usize` and `isfr: Option<f64>`
- [x] `agents_online` populated from `data.agents` active count
- [x] `count_online_from_files()` fallback when agents list empty
- [x] `isfr` computed from `data.gate_results` pass rate
- [x] Header bar renders `Nag` with SAGE/TEXT_GHOST style
- [x] Header bar renders `ISFR:XX%` with SAGE/WARNING/EMBER
- [x] `isfr == None` displays `ISFR:—` (em-dash)
- [n/a] HTTP polling (500ms timeout, fire-and-forget) — not wired; file/dashboard snapshot fallback is the implemented path
- [x] New section between system metrics and agent spinner
- [x] Existing header bar tests pass
- [x] Explicit ISFR tests added
- [x] Clippy/fmt clean
- [x] Tests pass

---

## Impl 10-Dashboard/TUI + 11-Demo Sprint (26/65 = 40%)

### 10-01: Stabilization and Nexus (7/8)

- [x] Audit backend truth sources — `truth_map.rs` covers all 16 entity kinds
- [x] Reduce duplicated state paths — projection-layer ownership established
- [x] Stabilize auth — `middleware.rs` + `auth.rs` with 3-level precedence
- [x] Add jobs backend with durable storage and state transitions — `jobs.rs` with `JobRecord`, durable `.roko/jobs/*.json`, `can_transition_to()` validation
- [x] Define Nexus as relay boundary — `relay.rs` with `RelayConnectionState`, `DataFreshness`, `RelayHealth`
- [x] Backend routes and WS events cover same entities — integration tested
- [x] Auth failure modes explicit and testable — 5 tests covering valid/invalid paths
- [ ] Nexus disconnects degrade to stale-state behavior — `RelayHealth` exists but no live relay client; room/subscription model absent

### 10-02: Dashboard rewrite (0/10 — external repo)

All 10 items are in `/Users/will/dev/nunchi/nunchi-dashboard`, not verifiable from the roko repo.

### 10-03: TUI polish and cross-surface verification (5/8)

- [x] Continue TUI work in existing tab/subview model — F8/F9 tabs, marketplace/atelier views
- [x] Finish subview parity (Provider Health, Model Comparison, Engram DAG, Episode Replay, Knowledge Browse) — all 5 implemented
- [x] Fix data refresh before cosmetic widgets — incremental cursors, `JsonlCursor`, `DashboardDataStamps` fingerprinting
- [ ] Add polish (command palette, density modes, widget ports, performance audits) — NOT implemented
- [ ] Run cross-surface parity checks — `parity.rs` and `surface_inventory.rs` exist as documentation, not runtime verification
- [x] Keybindings, labels, subview indices consistent — tests confirm
- [x] Data refresh no longer requires full re-read — incremental cursors
- [ ] TUI and dashboard show same backend truth for one e2e flow — no verified walkthrough

### 10-04: Page catalog, widgets, data contracts (4/11)

- [ ] Turn every PRD page group into implementation tasks — parity matrix documents status but not all page groups have task files
- [ ] Per-page specification (route, TUI mapping, loading/empty/error states) — partially done
- [ ] Widget catalog as shared component backlog — no unified backlog
- [ ] Data contracts as concrete backend/frontend tasks — `projection_contract.rs` partial
- [ ] Network-intelligence display tasks — ISFR in header, C-Factor in dashboard; knowledge density/network-size/domain breakdown NOT implemented
- [ ] Jobs-system integration beyond CRUD — chain-backed jobs/validator committee absent
- [x] Stabilization tasks from PRD-10 §14 — auth, state persistence, polling-to-streaming, aggregator cache, error handling all partially done
- [ ] Page-by-page parity tracking — static artifact only
- [ ] Widget state semantics — partially present
- [ ] Network-intelligence drilldowns — not implemented
- [x] StateHub/projection contract hardening — `projection_contract.rs` with versioning and recovery

### 11-01: Dashboard stream (0/9 — external repo)

All items in `nunchi-dashboard` repo.

### 11-02: Backend and TUI stream (10/10)

- [x] Typed jobs model with durable storage — `JobRecord` with `.roko/jobs/*.json`
- [x] Serve routes for job lifecycle + matching server events — 5 routes + 3 event types tested
- [x] State-machine transitions explicit and test-covered — `can_transition_to()` + 422 responses
- [x] Reuse existing WS/event infrastructure — `ServerEvent` enum extended, existing `EventBus` and `/ws` route
- [x] TUI marketplace/atelier wired to backend data — `StateHub` → `TuiState` → render
- [x] Finish enumerated subviews before adding new ones — all 9 tabs with subviews implemented
- [x] Visible demo metrics exist — heartbeats, plan/task progress, agent status, cost/provider health
- [x] Job routes exercisable from curl/integration test — `api_integration.rs` confirms
- [x] Matching server events appear on WS — `jobs_events_are_visible_over_websocket` test
- [x] TUI renders updated state without manual file surgery — `apply_snapshot()` from StateHub

### 11-03: Rehearsal and demo acceptance (0/9)

All items are runtime verification/acceptance criteria, not code artifacts. The code prerequisites exist but rehearsals have not been evidenced:

- [ ] Run backend locally and confirm health routes
- [ ] Run dashboard locally and confirm WS/API config
- [ ] Verify full research-style flow
- [ ] Verify full coding-style flow
- [ ] Verify heartbeat and telemetry visible
- [ ] Capture known demo fallbacks
- [ ] DEMO-ACC-01 through DEMO-ACC-04 — not evidenced

---

## Impl 08-Surfaces/UX + 01-Runtime (29/80 = 36%)

### 08-01: CLI, Chat, and TUI (10/10)

- [x] Audit existing CLI coverage — surface inventory via `roko status --surfaces`
- [x] Agent lifecycle commands (`start`, `list`, `stop`, `status`) backed by PID-registry
- [x] Persistent chat aligned with WS/event surfaces — WebSocket + exponential-backoff + HTTP fallback
- [x] Continue TUI work in existing modules
- [x] F8/F9 as real surfaces with modal flows
- [x] CLI tests for new commands — `cli_fallback.rs`
- [x] New commands in `--help`
- [x] TUI keybindings consistent
- [x] Chat reconnects/degrades cleanly
- [x] Critical actions doable from CLI without TUI/web

### 08-02: Web, MCP, Packaging, and DX (~3/7)

- [x] `roko-serve` as backend system of record — routes for all entities registered
- [ ] MCP extends existing server story — auto-registration to serve-level NOT wired; no MCP client integration test
- [ ] Package commands staged against registry — `install`/`remove` local-path only; no `search`/`publish`; no live registry
- [ ] CLI DX items — shell init NOT implemented; NO_COLOR partial (TUI only); CLICOLOR absent; richer `--version` absent; shell completions implemented
- [x] Web-surface work points to `nunchi-dashboard`, backend work here
- [x] New API surfaces have tests
- [ ] MCP integration works against sample flow — no test
- [ ] Shell completions and color behavior verified — partial

### 08-03: Product surfaces, deployment, onboarding, security, observability (~4/12)

- [ ] Break three product surfaces (AI Studio, Agent Studio, OpenClaw) into build tracks — none exist in codebase
- [ ] AI Studio backlog — not implemented
- [ ] Agent Studio backlog — not implemented
- [ ] OpenClaw backlog — not implemented
- [x] Deployment and gateway tasks — `roko init`, `roko update`, Docker files, cascade router with auth
- [ ] Onboarding tasks by persona — no persona-specific flows
- [ ] Security-model tasks — safety layer exists but no user-facing trust surfaces
- [x] Monitoring and observability — metrics, structured tracing, health probes, WS streaming, OpenAPI route
- [ ] Coordination/discovery tasks — partial; no serve-layer discovery endpoint
- [x] Deployment artifact parity — `check_deployment_parity()` in `doctor.rs`
- [x] `roko doctor` breadth — env vars, file layout, providers, dashboard/nexus reachability
- [ ] Observability retention — `retention.rs` implements TTL/compaction/export

### 01-01: Runtime foundation and extraction (2/13)

- [ ] Define runtime ownership boundary — implied by code structure, not formally documented
- [ ] Audit and document current type inventory — no type inventory document
- [ ] Introduce minimum runtime types — `CognitiveTier` in roko-agent not roko-runtime; no `ExtensionLayer` or `HeartbeatPipeline`
- [ ] Reuse existing runtime/event machinery — event bus reused but no filtered subscriptions
- [ ] Define extension contract — no `ExtensionLayer`, no cognitive extension chain
- [ ] Decide first extraction target from `orchestrate.rs` — no extraction occurred
- [ ] Create extension-chain assembly rules — not implemented
- [ ] Define domain profile loading — `DomainProfile`/`TypedContext` exist but no routing defaults or extension sets
- [x] Keep crate creation disciplined — no `roko-ext-*` crates
- [ ] Runtime events subscribable without CLI types — partially met, no filtered subscriptions
- [ ] No circular dependency — verified
- [ ] At least one extracted extension runs through lifecycle — not done
- [x] Event bus reuse — confirmed

### 01-02: Migration verification and cutover (~6/18)

- [ ] Transitional spawn/dispatch API — `spawn_agent_scoped`/`spawn_agent_with_layer` exist but are CLI-layer, not formal runtime transitional API
- [ ] Migrate one production path first — plan execution still in CLI layer
- [x] CLI surface only after runtime path is real — agent lifecycle commands backed by real state
- [x] Wire runtime events into existing surfaces — `event_bus` → serve WS → TUI
- [ ] Keep rollback simple — no feature flag for legacy dispatch
- [ ] Full lifecycle integration test — not found
- [ ] Domain profile integration test — unit tests only
- [ ] Type-state/lifecycle guard test — partially met
- [ ] Extension ordering test — not implemented
- [ ] Concurrent access test — not found
- [ ] Transitional and legacy both run in tests — only one dispatch path
- [x] CLI entrypoints reach expected execution path — `cli_fallback.rs`
- [x] Event subscribers still receive updates — verified
- [ ] Rollback switch documented and tested — not implemented
- [ ] Update architecture docs — no boundary formalized
- [ ] Update CLI help text — exists but not tied to cutover
- [x] `roko serve` and TUI consumers receive expected data — verified
- [ ] Remove dead compatibility code — no migration occurred

### 01-03: Heartbeat timescales, inference gateway, and ops (~4/20)

- [x] Three timescales (gamma/theta/delta) — `HeartbeatPolicy`, `theta_consumer.rs`, `delta_consumer.rs`
- [ ] Timescale configuration and persistence — `ClockConfig` exists but no domain overrides, jitter/backoff, or persisted timestamps
- [ ] Six concurrent cognitive mechanisms — attention salience partial; no habituation, sleep-pressure, homeostasis metrics, compensation/rollback
- [ ] Compensation and rollback semantics — not specified
- [x] `CorticalState` implementation — lock-free atomic storage, snapshot/export
- [ ] Event-fabric detail — partially typed payloads; no filtered subscriptions
- [x] Process supervision — `ProcessSupervisor`, `SupervisionStrategy` enum, `CancelToken`
- [ ] Inference gateway as runtime subsystem — no L1/L2/L3 cache, no intent routing
- [ ] Connect gateway to `roko-agent` — translator work exists but no unified gateway
- [ ] Performance benchmarks — none found
- [ ] RT-GAP-01 through RT-GAP-05 — none implemented
- [ ] Gamma/theta/delta independently observable — not tested
- [ ] Event-driven wakeups don't starve periodic loops — not tested
- [ ] Supervisor restart/kill deterministic — one test found
- [ ] Gateway cache/routing logged — routing log exists but no gateway layer
- [x] Translator behavior covered by tests — 10+ unit tests in `translate/openai.rs`

---

## Impl 02-Cognitive Engine + 03-Context (21/76 = 28%)

### 02-01: Prediction, gating, and triage (1/11)

- [ ] Define canonical types (`Observation`, prediction-error result, gate decision, triage decision) — no shared canonical types
- [ ] Identify real inputs already available — exist individually but no unified gate input
- [ ] Implement prediction-error as composable policy — only scalar proxy `1.0 - affect_confidence`
- [ ] Add habituation — not found anywhere
- [x] Wire somatic escalation — `behavioral_state_tier_shift()` + `apply_daimon_tool_policy_csv()` wired
- [ ] Define T0 triage for chain work — implemented in `roko-chain/src/triage.rs` but NOT wired into runtime
- [ ] Put thresholds behind config — `TierThresholds` is config-driven but many magic constants remain
- [ ] Unit tests for low/high novelty and habituated repeats — no habituation
- [ ] Somatic escalation testable without live model — partial
- [ ] Chain T0 triage classifies benign and urgent — tests exist but not wired to runtime
- [ ] Routing decisions explain themselves — `routing_reason` exists but no 4-component decomposition

### 02-02: Native harness, costs, and verification (6/10)

- [ ] Define native harness boundary — no `NativeHarness` abstraction
- [x] Per-tick cost accounting — `TurnAccounting`, `AgentEfficiencyEvent`, `CostTable`/`CostsDb`
- [x] Wire cost state into cognitive pipeline — TUI fields, cascade router integration, roko-learn consumers
- [x] Somatic/policy check before every tool call — `apply_daimon_tool_policy_csv()` + `SafetyLayer`
- [ ] Native harness as default after proven — no harness concept
- [x] Tool call path reaches safety layer — always in path
- [ ] Tier decision affects whether model call happens — no test for T0 skip
- [x] Cost totals match execution path — `cost_comparison.rs`
- [ ] Native-harness integration test — no such test
- [x] Cost telemetry inspectable after test run — `CacheStats`, efficiency log

### 02-03: Thresholds, cascade router, and measurement (2/17)

- [ ] Pluggable threshold policy family (EWMA, CUSUM, SPC, Hotelling) — only scalar EMA in roko-gate
- [ ] Tie threshold to domain profiles — no per-domain threshold profiles
- [x] Temperament-aware adjustments — `Temperament` enum, `temperament_tier_shift()`, tests
- [ ] Neuro-informed priors — explicitly unimplemented (CLAUDE.md item #13)
- [ ] Three clocks model — no three-clocks; burn rate probe exists separately
- [ ] Clarify "mortality integration" — `mortality.rs` still uses death semantics
- [x] Integrate with CascadeRouter by stage — three-stage cascade (Static/Confidence/UCB) fully implemented
- [ ] Arena-backed validation — not found
- [ ] Anomaly/regression dashboards — `drift.rs`/`regression.rs` exist but not for threshold tracking
- [ ] CE-GAP-01 through CE-GAP-05 — none implemented
- [ ] Threshold policy visible in config/logs — no pluggable family
- [ ] CascadeRouter stage transitions deterministic — integration tests exist (marking partial)
- [ ] Arena/replay benchmarks — not implemented
- [ ] Mortality language translated — not done

### 03-01: Workspace, bidders, and policy (6/11)

- [ ] Define canonical workspace model (`ContextCategory`, `CognitiveWorkspace`) — no `ContextCategory` or `CognitiveWorkspace` types
- [x] Data model reflects current reality — `ContextProvider`/`ResolvedContext` maps sections to prompt layers
- [x] Bidder contract with score/token/provenance — `AttentionBidder` enum, `LearningBidder` with beta-posterior
- [x] Start with bidders that have data — 8 bidder variants assigned in `orchestrate.rs`
- [ ] Future-facing bidders (chain, worldgraph) — not implemented
- [x] Wire auction output into prompt builder — `select_optional_candidates()` as auction, U-shaped ordering
- [ ] Add `ContextPolicy` — no such type; `SectionEffectivenessRegistry` is closest analog
- [x] Auction produces deterministic output — tested
- [x] Bidders can be turned on/off independently — per-section variant assignment
- [ ] Winning sections logged with scores — `AuctionDiagnostics` partially
- [ ] Policy change produces observable difference — no `ContextPolicy`

### 03-02: Caching, chain, and WorldGraph (3/10)

- [x] Section-effect tracking — `SectionEffectivenessRegistry` persisted, wired into dispatch
- [ ] Deterministic cache keys — caller-supplied fingerprint but no canonical recipe
- [ ] Cache tiers with concrete purpose — only in-process LRU; no semantic or provider-side tiers
- [ ] Wire chain-sourced context — no `InsightStore` client in roko-compose
- [ ] WorldGraph as future-facing — no crate or stub
- [x] U-shaped placement and complexity scaling through canonical path — wired; `social_foraging_boost()` bounded
- [ ] Cache hits/misses visible — `CacheStats` exists
- [ ] Cache invalidation deterministic — LRU tested
- [ ] Chain entries labeled separately — no chain context
- [ ] WorldGraph stubbed — no stub

### 03-03: Context mesh, measurement, and persistence (3/17)

- [x] `ContextMesh` as scoped shared surface — fully implemented with thread-safety, publish/query, deduplication, staleness eviction
- [ ] Publication entry types — only generic `topic: String`, no typed enum
- [x] Prevent echo and duplication — self-exclusion + Jaccard overlap dedup
- [ ] Section-effect outcomes for causal analysis — no confidence intervals or domain field
- [ ] Leave-one-out or Shapley attribution — `shapley.rs` exists for agents, not sections
- [ ] Persist learned context state — partially done (section-effects, context-packs, experiments yes; budget predictor, context policy, attention-curve no)
- [ ] Cache economics measurement — `CacheStats` exists but no dollar/savings calculation
- [ ] Surface diagnostics for operators — `AuctionDiagnostics` partial; `ContextMesh` not wired into runtime
- [ ] Mesh namespace scoping — no namespace model
- [ ] Context-pack explainability snapshots — no section order/winning bid persistence
- [ ] Prefix-alignment calibration by provider — only one regression test
- [ ] Social-foraging safeguards — math bounded (+0.3 cap) but not configurable
- [ ] Persistence compaction/GC — no compaction for learned context files
- [x] Mesh publications queryable/evictable under test — tested
- [ ] Learned context survives restart — partially (save/load exists for some files)
- [ ] Cache-hit metrics inspectable without debugging — not surfaced in TUI/serve
- [ ] Cross-agent sharing scoped — `ContextMesh` not wired into runtime

---

## Impl 04-Knowledge thru 07-Chain (24/133 = 18%)

### 04-01: Knowledge pipeline and HDC (1/10)

- [ ] Inventory current write paths — paths exist but not documented as inventory
- [ ] Wire knowledge pipeline stages explicitly — episode completion, distillation, clustering exist but not as explicit pipeline
- [ ] Strengthen fingerprints — only encodes `(prompt, outcome)`, no task description or tool-call sequence
- [x] Reuse `roko-primitives` HDC operations — `bind`, `bundle`, `similarity`, `fingerprint()` all exported and used
- [ ] Add PP-HDC behind clear module — not found anywhere
- [ ] Keep local retrieval quality measurable — no benchmarked query comparison
- [ ] Similar episodes cluster deterministically — tests exist but marking per original plan scope
- [ ] Fingerprint enrichment improves retrieval — no regression fixture
- [ ] PP-HDC round-trip tests — PP-HDC absent
- [ ] Knowledge queries bounded in latency — no latency test

### 04-02: Publishing, dreams, and chain (3/9)

- [ ] Define `KnowledgePublisher` — no such type
- [ ] Seven-layer publishing defense — none of 7 layers implemented as pipeline
- [x] Wire dream triggers using existing infrastructure — `DreamLoopConfig`, `orchestrate.rs` wiring, `StagingBuffer::promote_validated`, `AffectEvent::DreamOutcome`
- [ ] Local-first InsightStore integration — no client boundary or stub
- [x] Do not treat mesh/pheromone as already implemented — correctly treated as future work
- [ ] Each publishing-defense layer fails independently — no defense pipeline
- [ ] Dream scheduling triggerable in test — config exists but no deterministic test helper
- [x] Dream outputs become queryable knowledge entries — `promote_validated` → `KnowledgeStore`
- [ ] Chain/mirage query failures degrade to local-only — no client boundary with fallback

### 04-03: InsightStore, resonance, lifecycle, measurement (4/15)

- [x] Shared knowledge contract in current vocabulary — `KnowledgeKind` (6 variants) aligned across mirage-rs and roko-neuro
- [ ] On-chain entry format boundary — `InsightEntry` exists but no query envelope or documented chain vs local format contract
- [x] Pheromone dynamics concretely — `PheromoneField` with potency/decay/confirm fully implemented and tested
- [ ] Cross-domain resonance wired — `detect_cross_domain_resonance` and `detect_resonance_pairs` exist but only called in unit tests, not runtime
- [x] Temporal knowledge topology — `TemporalIndex` with `KnowledgeEpoch`, `AllenRelation` reasoning implemented (not runtime-used)
- [ ] Generalized benchmark and collective-intelligence measurement — C-Factor exists but no cross-agent lift benchmarks
- [ ] Network-effects and thousandth-agent-advantage — not implemented
- [ ] KN-GAP-01 through KN-GAP-05 — none implemented
- [ ] Shared-entry query envelopes match vocabulary — no query envelope
- [x] Pheromone reinforcement/demurrage simulated deterministically — tested
- [ ] Resonance output consumed somewhere observable — only in tests
- [ ] Collective-intelligence metrics recomputable — C-Factor runs but cross-agent lift not measured

### 05-01: Domain runtime and arenas (2/9)

- [ ] Define canonical `DomainProfile` surface — `TaskDomain` has 5 variants with `default_gates()` but no routing hints, tool allowlist, context mix, or extension sets
- [x] Load domain profile from config — `TaskDomain` loads from TOML, config, and role
- [x] Route observable behavior by domain — gate selection, git policy, compiled-gate selection, tool allowlist per role
- [ ] `Arena` trait — does not exist
- [ ] Two initial arenas — not implemented
- [ ] CLI when arena contract real — `roko bench arena` doesn't exist
- [ ] Domain config changes tool/gate in integration test — only unit tests
- [ ] Arena runs produce persisted scores — arenas not implemented
- [ ] Failed arena records enough state — arenas not implemented

### 05-02: Domain extensions, HF, and market (0/8)

- [ ] Domain-specific extensions behind shared contract — no contract
- [ ] Dedicated HuggingFace crate — only `tokenizers` crate used for token counting
- [ ] Small vertical slice (SWE-bench) — not implemented
- [ ] Work-market integration behind boundaries — not implemented
- [ ] Cross-arena transfer tracking — arenas don't exist
- [ ] One domain extension through shared chain — not implemented
- [ ] One benchmark dataset loaded e2e — not implemented
- [ ] Arena output supports reward/settlement/training — not implemented

### 05-03: Profile catalog, custom domains, scaling (0/16)

All 16 items not implemented (profile catalog, runtime controls per domain, lifecycle differences, wider arena catalog, custom-domain creation, benchmark index, scaling/flywheel, DA-GAP-01 through DA-GAP-05, verification items).

### 06-01: Oracle, prediction, and perps (1/11)

- [ ] Define `ISFRSource` trait — no per-source trait with `quote_fetch`, freshness, confidence
- [ ] Implement/stub initial sources (Aave, Compound, Ethena, ETH beacon) — none exist
- [x] Aggregation with adversarial assumptions — two-level weighted median with 3-sigma outlier exclusion
- [ ] Compute confidence explicitly — `std_deviation`/`excluded_count` but no composite confidence field
- [ ] Precompile/RPC interface — `HdcPrecompile` stub exists but no ISFR-specific interface
- [ ] CRPS-based prediction scoring — no CRPS module
- [ ] Yield perp math — no mark price, funding rate, margin, P&L functions
- [ ] Source failures don't produce false valid index — no source-failure handling
- [ ] Flash-loan/outlier adversarial test — only 3-sigma; no adversarial test
- [ ] Perp pricing tested with fixtures — not implemented
- [ ] Prediction scoring independent from settlement — no scoring module

### 06-02: Clearing runtime and verification (0/9)

All 9 items not implemented (`ClearingProfile`, solver interface, KKT certificate, `ClearingInsight`, EventFabric integration, large-agent scenarios, verification tests).

### 06-03: Publication states, economics, and credibility (0/16)

All 16 items not implemented (`PublicationState` enum, source-liveness tracking, solver economics, credibility path, EventFabric wiring, cross-domain usage, multi-chain sources, ISFR-GAP-01 through ISFR-GAP-05, verification items).

### 07-01: Consensus, execution, and precompiles (4/8)

- [x] Decide `roko-chain` vs `apps/mirage-rs` split — protocol types in chain, simulation in mirage
- [x] Consensus/execution types without overcommitting — `phase2.rs` stubs with `#[allow(dead_code)]`
- [ ] Precompile interfaces as versioned contracts — only generic `PrecompileConfig`; no specific contract interfaces
- [ ] Simulator-first coverage — `HdcPrecompile` methods all `todo!`
- [x] Validator/execution behind clear module boundaries — `phase2.rs` isolation
- [ ] Precompile emulation tests — none exist
- [ ] Failure behavior documented and tested — not implemented
- [x] Consensus types compile without pulling simulator code — feature-gated, compiles independently

### 07-02: InsightStore, tokenomics, and HDC (6/8)

- [x] Define InsightStore entry types against neuro vocabulary — 6 `KnowledgeKind` variants aligned
- [x] Local/simulated scoring first — pheromone weight, reputation scoring, HDC search (brute-force + HNSW)
- [x] Keep tokenomics separate from storage — `KoraiToken` with emission/demurrage independent of `InsightEntry`
- [ ] HDC/HTC precompile in pure functions — implementations in `roko-primitives` but `HdcPrecompile` methods all `todo!`
- [x] Align naming with roko-primitives/neuro — consistent `HdcVector`, `KnowledgeKind` vocabulary
- [ ] Similarity search comparable chain vs local — no comparative test
- [x] Demurrage/reputation multipliers unit tested — multiple `#[test]` functions
- [x] InsightStore query measurable in mirage/local — tests for post/confirm/challenge/decay/search

### 07-03: Identity, registries, proof log, and rollout (3/14)

- [x] Agent Passport backlog — `AgentPassport` with all fields, `AgentRegistry` with registration/timelock/tier
- [x] Reputation Registry backlog — `ReputationRegistry` with per-track EMA, 7 domains, decay, slashing
- [x] Validation Registry backlog — `ValidationRegistry` with `WorkProof`, `GateScore` attachment, query
- [ ] `PROOF_LOG` backlog — no `ProofLog` type, prediction commitments, or query API
- [ ] Stage each registry through mirage — no simulator integration
- [ ] Tie registries to product consumers — not consumed by marketplace/dashboard/ISFR
- [ ] CHAIN-ID-01 through CHAIN-ID-05 — none implemented
- [ ] Contract/simulator interfaces versioned and testable — unit tests exist but no versioned simulator
- [ ] Client code read/write without undeployed chain — no client stubs
- [ ] Proof-log data consumable by ISFR — no `PROOF_LOG`

---

## Impl 09-Extensibility & Multichain (1/39 = 3%)

### 09-01: Package and runtime loading (1/8)

- [ ] Package manifest — `PluginManifestFile` exists but missing package id, source type, capabilities, runtime compatibility
- [ ] Lockfile and storage layout — not implemented
- [x] Reuse existing plugin surfaces — single coherent vocabulary (`PluginManifestFile`/`PluginManifest`/`PluginBuilder`)
- [ ] QuickJS/Pi compatibility — not applicable/not implemented
- [ ] Multi-domain profile composition — not implemented
- [ ] Install/uninstall deterministic — no installer/uninstaller
- [ ] Composed profile loaded in tests — only single-manifest tests
- [ ] Version/integrity mismatches fail actionably — no version/integrity checking

### 09-02: Ingestion, discovery, and WorldGraph (0/9)

All 9 items not implemented. `ChainClient` trait exists but is not `ChainConnector`, has no canonical event schema, no finality/reorg handling. No contract discovery, predictive foraging, or WorldGraph.

### 09-03: Fine-tuning integration and acceptance (0/7)

All 7 items not implemented. No training-data extraction pipeline, no hub exporter, no dynamic fine-tuned model discovery, no vertical integration slices.

### 09-04: Attention allocation, publishing, and ecosystem (0/15)

All 15 items not implemented. No Gittins index, no predictive foraging, no active-inference attention policy (only model-tier EFE), no package publishing/registry, no package marketplace UX, no arenas-as-packages, no implementation phasing tasks.

---

## Key Divergences from Plan

1. **Jobs architecture**: Plan called for `roko-core/src/jobs.rs` with `FileJobStore`. Reality: jobs live in `dashboard_snapshot.rs` types + `roko-serve/src/routes/jobs.rs` with durable JSON files. Different shape, partially functional.

2. **Atelier data source**: Plan called for local TOML parsing. Reality: StateHub-sourced data from `DashboardSnapshot`. Functionally equivalent but different architecture.

3. **Product surfaces (AI Studio, Agent Studio, OpenClaw)**: Entirely absent from the Rust codebase. Zero implementation across all three.

4. **WorldGraph, predictive foraging, package ecosystem**: Zero implementation. These are Phase 2+ concepts that have no code presence.

5. **Nexus relay**: Health metadata structures exist. No live relay client, room/subscription model, or forwarding logic.

6. **Context mesh**: Fully built and tested in `roko-compose` but NOT wired into the runtime (`orchestrate.rs` never calls it).

7. **Chain T0 triage**: Fully built and tested in `roko-chain` but NOT wired into the runtime.

8. **Resonance detection**: Functions exist in `roko-primitives` and `roko-neuro` but only called from unit tests, never at runtime.
