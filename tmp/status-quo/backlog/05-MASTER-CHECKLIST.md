# 05 — Master Checklist

> **The single scannable checklist for the entire executable backlog.**
> - Repo HEAD: `5852c93c05` on `main` · authored 2026-07-10
> - Source: every task below is lifted verbatim (id · title · tier · deps) from
>   `backlog/epics/E01…E18-*.md` and `backlog/plans/E19…E48-*/tasks.toml`.
> - Scope: **48 epics · 447 tasks.** Every epic E01–E48 is represented; each task appears exactly once.
> - Ordering: by milestone/phase, then by epic, then by task id.

---

## Summary

| Phase / Milestone | Theme | Epics | Tasks | Notes |
|---|---|---|---|---|
| **M0** | Bootstrap & correctness | E01–E05 | **62** | Self-hosting gate — becomes real at E01-T01 |
| **M1** | Close live-path loops | E06–E08, E14, E15 | **47** | Effective + durable |
| **M2** | Surfaces, hygiene, ACP, docs/ops | E09–E12, E16–E18 | **57** | Observable + shippable |
| **M3+** | Long-horizon v2 spec-debt | E13 | **3** | Spec-complete |
| **Phase 1** | Kernel (Signal, Cell, Graph, Execution) | E19–E22 | **40** | Core abstractions |
| **Phase 2a** | Agent & Cognition | E23–E26 | **42** | Cognitive autonomy, memory, learning, inference |
| **Phase 2b** | Infrastructure | E27–E33 | **61** | Feeds, groups, connectivity, extensions, triggers, tools, telemetry |
| **Phase 2c** | Security & Auth | E34–E35 | **16** | IFC, auth protocol |
| **Phase 3a** | Economy | E36, E38 | **17** | Payments, marketplace |
| **Phase 3b** | Surfaces & Registries | E37, E39 | **17** | Surface projections, identity registries |
| **Phase 3c** | Evals & DeFi | E40–E41 | **16** | Arenas, DeFi products |
| **Phase 3d** | Cross-cutting & Operations | E42–E45 | **34** | Config evolution, deployment, cross-cut functors, mori parity |
| **Operational** | GitHub, resources, rate limits | E46–E48 | **35** | Workflow integration, disk mgmt, budgeting |
| | | **48** | **447** | |

> **Milestone assignment note:** M0–M3+ assignments follow the original epic files.
> Phase 1/2/3 assignments follow the v2 spec structure and dependency ordering.
> The task ids/tiers/deps are authoritative; milestone/phase buckets are a scheduling convenience.

### Legend

**Tiers** (ascending effort/risk): `mechanical` < `focused`/`small`/`standard` < `integrative`/`medium`/`complex` < `architectural`/`design`/`planning`.
Note: epics use slightly different tier vocabularies; they are reproduced as written. Tiers marked `*` were left implicit in the epic file and are inferred from the task description.

**Status** (all currently `ready` / unstarted):
- `[ ]` not started · `[x]` done
- **plan:** existing `plans/Pxx` coverage — `full` = plan aims correctly · `partial`/`shallow` = plan patches a symptom · `superseded` = plan retired by this task · `prereq` = plan must land first · `none` = pure gap (no plan)
- **deps:** task-level `depends_on` (epic-level cross-epic deps shown in parentheses, e.g. `(E01)`)
- **file:** primary target (see epic file for full `read_files`/context)

---

## M0 — Bootstrap & Correctness  *(62 tasks · E01–E05)*

### E01 — Execution Engine  *(make bare `plan run` do real work + close the live-path holdouts · 16 tasks)*

- [ ] **E01-T01** (mechanical) Flip `plan run` default engine to runner-v2 — deps: none — plan: P11(partial, missed line) — file: crates/roko-cli/src/main.rs:1361
- [ ] **E01-T02** (mechanical) Route `roko resume` to PlanEngine::RunnerV2 — deps: E01-T01 — plan: none — file: crates/roko-cli/src/main.rs:2699
- [ ] **E01-T03** (focused) Warn on Graph dry-run stub instead of fabricating SUCCESS — deps: E01-T01 — plan: none — file: crates/roko-cli/src/commands/plan.rs, roko-graph/src/cells/task_executor.rs
- [ ] **E01-T04** (architectural) Wire a real intra-plan DAG scheduler (or delete dead task_dag.rs) — deps: E01-T01 — plan: P12(shallow) — file: crates/roko-cli/src/runner/event_loop.rs, runner/task_dag.rs
- [ ] **E01-T05** (focused) Make agent concurrency configurable, decoupled from max_concurrent_plans=4 — deps: E01-T04 — plan: P12(shallow) — file: crates/roko-cli/src/runner/event_loop.rs, roko-core/src/defaults.rs:313
- [ ] **E01-T06** (integrative) Upgrade gate-failure replan from prompt-append to task/plan revision — deps: E01-T01 — plan: none (P15 adjacent) — file: crates/roko-cli/src/runner/event_loop.rs:1549
- [ ] **E01-T07** (integrative) Wire per-plan worktree isolation into the runner — deps: E01-T04 — plan: none — file: crates/roko-cli/src/runner/merge.rs, roko-orchestrator/src/worktree.rs
- [ ] **E01-T08** (integrative) Wire `enrich_rung_config` into live gate dispatch — deps: E01-T01 — plan: none (overlaps E05) — file: crates/roko-cli/src/runner/gate_dispatch.rs, runner/rung_dispatch.rs
- [ ] **E01-T09** (focused) Regression test: bare default `plan run` does real work — deps: E01-T01, E01-T02 — plan: none — file: crates/roko-cli/tests/ (new)
- [ ] **E01-T10** (mechanical) Reconcile CLAUDE.md + GAPS + docs to the real engine — deps: E01-T01, E01-T02 — plan: none — file: CLAUDE.md, .roko/GAPS.md
- [ ] **E01-T11** (focused) Enforce cost budget from config before plan execution starts — deps: E01-T01 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs, runner/types.rs, roko-core/src/config.rs
- [ ] **E01-T12** (integrative) Add retry-with-backoff for retriable agent dispatch failures (429/529) — deps: E01-T01 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs, roko-agent/src/dispatcher/mod.rs
- [ ] **E01-T13** (integrative) Wire roko-mcp-github as a default MCP server for plan execution — deps: E01-T01 — plan: none — file: crates/roko-cli/src/orchestrate.rs, runner/event_loop.rs, roko-mcp-github/src/main.rs
- [ ] **E01-T14** (focused) Auto-create git branches per plan run using worktree.rs branch naming — deps: E01-T04 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs, roko-orchestrator/src/worktree.rs
- [ ] **E01-T15** (focused) Add pre-run disk space check (refuse to start if <2GB free) — deps: E01-T01 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs, commands/plan.rs
- [ ] **E01-T16** (focused) Auto-trigger GcEngine at plan start — deps: E01-T01 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs, commands/plan.rs

### E02 — Storage Convergence  *(one canonical writer per durable concern; fix empty dashboards · 12 tasks)*

- [ ] **E02-T01** (integrative) Unify signal store: runner writes gate verdicts to canonical log, stops writing signals.jsonl — deps: none (soft E03) — plan: none — file: crates/roko-cli/src/runner/event_loop.rs:1147, roko-fs/src/layout.rs
- [ ] **E02-T02** (focused) Guard init migration against schema-mixing (only move rows that parse as Engram) — deps: none — plan: none — file: crates/roko-cli/src/commands/util.rs:135
- [ ] **E02-T03** (integrative) Repoint executor.json readers at state-snapshot.json; drop executor_snapshot() helper — deps: none — plan: none — file: crates/roko-serve/src/routes/workspaces.rs:322, dashboard_snapshot.rs, roko-fs/src/layout.rs
- [ ] **E02-T04** (focused) Materialize or repoint gate-thresholds.json — deps: E02-T03 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs or roko-serve/src/learning/mod.rs
- [ ] **E02-T05** (focused) Collapse episodes to root; repoint serve projection + layout label off memory/ — deps: none — plan: none — file: crates/roko-serve/src/lib.rs, roko-fs/src/layout.rs, feedback_service.rs
- [ ] **E02-T06** (focused) Consolidate daimon to daimon/affect.json; alias orchestrator state/daimon.json — deps: none — plan: none — file: crates/roko-orchestrator/src/service_factory.rs:236
- [ ] **E02-T07** (focused) Add retention for events.jsonl, roko.log, chain-watcher.log, run-ledger.jsonl; cap *.bak.* — deps: none — plan: none — file: crates/roko-serve/src/retention.rs:115
- [ ] **E02-T08** (integrative) Stop feed_tick heartbeats polluting events.jsonl — deps: none — plan: none — file: crates/roko-serve/src/feed_agents/mod.rs, state.rs
- [ ] **E02-T09** (focused) Resolve runtime-events.jsonl reader-without-file (wire JsonlLogger or drop 2 routes) — deps: none — plan: none — file: crates/roko-serve/src/lib.rs, routes/runs.rs, shared_runs.rs
- [ ] **E02-T10** (mechanical) Retire dead second-impls: state/run-state.json, state/events.json, roko_runtime::RunLedger — deps: E02-T03 — plan: none — file: crates/roko-cli/src/runner/persist.rs, roko-runtime/src/run_ledger.rs
- [ ] **E02-T11** (integrative) Introduce LayoutVersion::V2 + real .roko/VERSION migration + `roko doctor` state audit — deps: E02-T01, E02-T03, E02-T05, P24-T3(soft) — plan: P24-T3(pattern) — file: crates/roko-fs/src/layout.rs, roko-cli/src/doctor.rs
- [ ] **E02-T12** (focused) Make cold-substrate archival move-not-copy: prune archived engrams from the hot store after archive_batch + dedup the cold append (runtime-live hourly copy-not-move → unbounded growth) — deps: none — plan: none — file: crates/roko-serve/src/lib.rs:2166, roko-fs/src/cold_substrate.rs:218, roko-fs/src/file_substrate.rs:88

### E03 — Type Consolidation  *(canonicalize highest-blast duplicate type families · 7 tasks)*

- [ ] **E03-T01** (mechanical) Delete orphan `roko-core::state_hub` dead file — deps: none — plan: none — file: crates/roko-core/src/state_hub.rs *(exemplar EX03)*
- [ ] **E03-T02** (standard) GateVerdict: establish foundation canonical + From adapters into dashboard/episode views — deps: none — plan: none — file: crates/roko-core/src/foundation.rs, dashboard_snapshot.rs, roko-learn/src/episode_logger.rs
- [ ] **E03-T03** (standard) GateVerdict: rename divergent copies so exactly one bare `struct GateVerdict` remains — deps: E03-T02 — plan: none — file: crates/roko-core/src/dashboard_snapshot.rs, roko-learn/src/episode_logger.rs, roko-chain/src/identity_economy_identity.rs
- [ ] **E03-T04** (mechanical) DashboardSnapshot: rename thin/TUI copies (ProjectionSnapshot / TuiDashboardModel) — deps: none — plan: none — file: crates/roko-cli/src/runner/projection.rs, tui/dashboard.rs
- [ ] **E03-T05** (complex) DashboardSnapshot: TUI consumes core snapshot via watch::Receiver — deps: E03-T04 — plan: none — file: crates/roko-cli/src/tui/dashboard.rs
- [ ] **E03-T06** (standard) RetentionPolicy: introduce shared canonical + adapters into 3 engines — deps: none — plan: none — file: crates/roko-fs/src/gc.rs, roko-serve/src/retention.rs, roko-learn/src/episode_logger.rs
- [ ] **E03-T07** (mechanical) Delete dead `roko-chain::Engram` stub — deps: none — plan: none (coord E11-T04) — file: crates/roko-chain/src/identity_economy_markets.rs

### E04 — Security Perimeter  *(three P0s exploitable today + self-execution safety prerequisites · 19 tasks)*

- [ ] **E04-T01** (focused) Move relay proxy under the auth stack (F1/P0-1) — deps: none — plan: none — file: crates/roko-serve/src/routes/mod.rs:248, relay_proxy.rs
- [ ] **E04-T02** (focused) Deny-by-default scope fallback (F2/P0-2) — deps: none — plan: none — file: crates/roko-serve/src/routes/middleware.rs:385
- [ ] **E04-T03** (focused) CI test: every mutating route explicitly classified — deps: E04-T02 — plan: none — file: crates/roko-serve/src/routes/middleware.rs (+test)
- [ ] **E04-T04** (focused) Redact secrets in serialize_effective / `config show` (F6) — deps: none — plan: none — file: crates/roko-core/.../loader.rs:567, config_cmd.rs
- [ ] **E04-T05** (focused) Promote SecretLeak + PathEscape post-checks to Block (F5) — deps: none — plan: none — file: crates/roko-agent/src/safety/mod.rs:767
- [ ] **E04-T06** (integrative) Invoke safety funnel on default Claude-CLI path (F4) — deps: P16(all), E04-T05 — plan: P16(prereq) — file: crates/roko-agent/src/dispatch_v2.rs, orchestrate.rs, safety/mod.rs
- [ ] **E04-T07** (integrative) Real hash-chaining in custody append + verify (F7) — deps: none — plan: none — file: crates/roko-cli/src/custody.rs:183
- [ ] **E04-T08** (focused) Auth the worker result callback (F9) — deps: E04-T02 — plan: none — file: crates/roko-cli/src/worker.rs, roko-serve ingest route
- [ ] **E04-T09** (focused) Sanitize workspace prefix; write un-interpolated config (F8) — deps: none — plan: none — file: crates/roko-serve/src/routes/workspaces.rs:101
- [ ] **E04-T10** (focused) Privy JWT membership/role authorization (F10) — deps: none — plan: none — file: crates/roko-serve/src/routes/middleware.rs:202
- [ ] **E04-T11** (focused) Require scope for terminal even on loopback; drop token-in-env (F11) — deps: E04-T02 — plan: none — file: crates/roko-serve/src/routes/mod.rs:210, terminal.rs
- [ ] **E04-T12** (integrative) Add CognitiveEvent::PermissionRequest { …, reply: oneshot } (F3) — deps: P22-T2 — plan: P22(substrate) — file: crates/roko-acp/src/bridge_events.rs
- [ ] **E04-T13** (integrative) Answer PermissionRequest in parent loop via request_permission (F3) — deps: E04-T12 — plan: none — file: crates/roko-acp/src/bridge_events.rs:954
- [ ] **E04-T14** (integrative) Gate execute_acp_builtin_tool on the decision, fail-closed (F3) — deps: E04-T13 — plan: none (co-owned E17-T01) — file: crates/roko-acp/src/builtin_tools.rs:291
- [ ] **E04-T15** (focused) Producer-side SSE/WS secret scrub (F12) — deps: none — plan: none — file: agent output / trace / terminal / event-ingest producers
- [ ] **E04-T16** (integrative) Route ACP bash/web_fetch through network policy + SSRF block (F13) — deps: E04-T14 — plan: none — file: crates/roko-acp/src/builtin_tools.rs, roko-agent policy
- [ ] **E04-T17** (focused) Surface MCP command/env allowlist in `roko doctor` (F14) — deps: none — plan: none — file: crates/roko-agent/src/mcp/config.rs, doctor cmd
- [ ] **E04-T18** (focused) Per-API-key / per-IP rate limiter (F15) — deps: none — plan: none — file: crates/roko-serve/src/routes/mod.rs:90
- [ ] **E04-T19** (focused) Generate route→scope manifest from router assembly (F2 nav) — deps: E04-T02, E04-T03 — plan: none — file: crates/roko-serve/src/routes/mod.rs (+build/test)

### E05 — Gate Adaptivity On The Live Path  *(honest verdicts: real inputs, neutral stubs, per-rung EMA · 8 tasks)*

- [ ] **E05-T01** (focused) Persist GateThresholds to .roko/learn/gate-thresholds.json on snapshot (F5) — deps: (E01) — plan: none — file: crates/roko-cli/src/runner/event_loop.rs, persist.rs:230
- [ ] **E05-T02** (focused) Make stub verdicts neutral (Skipped), not Verdict::pass (F2) — deps: (E01) — plan: none — file: crates/roko-gate/src/rung_dispatch.rs:290
- [ ] **E05-T03** (focused) Exclude skipped verdicts from EMA + `passed` in the runner (F2/F3) — deps: E05-T02 — plan: none — file: crates/roko-cli/src/runner/gate_dispatch.rs:140, event_loop.rs
- [ ] **E05-T04** (integrative) Label GateCompletion with real inner rung; drive per-rung EMA (F3/F4) — deps: E05-T03 — plan: none — file: crates/roko-cli/src/runner/gate_dispatch.rs, event_loop.rs, types.rs
- [ ] **E05-T05** (architectural) Port enrich_rung_config → RungExecutionInputs into live gate_dispatch (F1/F7) — deps: E05-T04 — plan: P14(superseded; absorbs T3 test) — file: crates/roko-cli/src/runner/gate_dispatch.rs, roko-gate/src/rung_dispatch.rs
- [ ] **E05-T06** (integrative) Unify the three rung dialects onto rung_selector::Rung (F6) — deps: E05-T04 — plan: none — file: crates/roko-gate/src/registry.rs, roko-runtime/src/effect_driver.rs
- [ ] **E05-T07** (mechanical) Remove the dead enable_advanced_rungs toggle (closes P14-T1/T2) (F7) — deps: E05-T05 — plan: P14(superseded) — file: crates/roko-cli/src/orchestrate.rs, config
- [ ] **E05-T08** (integrative) Wire a VerdictPublisher on the live path (emit real Kind::GateVerdict engrams) (F9) — deps: E05-T04, E02 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs

---

## M1 — Close The Live-Path Loops  *(47 tasks · E06, E07, E08, E14, E15)*

### E06 — Compose / Prompt Unification  *(collapse 4 assembly surfaces to 1 canonical; close VCG warmup · 9 tasks)*

- [ ] **E06-T01** (planning) Decide canonical prompt-assembly surface + write ADR — deps: (E01) — plan: none — file: tmp/status-quo/backlog/decisions/E06-canonical-surface.md
- [ ] **E06-T02** (mechanical) Delete dormant surface E (templates/assembly.rs PromptAssembler) — deps: (E01) — plan: none — file: crates/roko-compose/src/templates/assembly.rs, mod.rs, lib.rs
- [ ] **E06-T03** (complex) Route Runner v2 Dispatcher prompt through build_role_system_prompt — deps: E06-T01 — plan: none — file: crates/roko-cli/src/dispatch/prompt_builder.rs:717, dispatch/mod.rs
- [ ] **E06-T04** (focused) Retire surface D authorship (fold or delete the markdown path) — deps: E06-T03 — plan: none — file: crates/roko-cli/src/dispatch/prompt_builder.rs
- [ ] **E06-T05** (focused) De-duplicate section-effectiveness onto compose effective_priority — deps: E06-T03 — plan: none — file: crates/roko-cli/src/dispatch/prompt_builder.rs:927
- [ ] **E06-T06** (focused) Persist LearningBidders store (.roko/learn/attention-bidders.json) — deps: (E01) — plan: none — file: crates/roko-cli/src/orchestrate.rs
- [ ] **E06-T07** (complex) Close VCG warmup: call update_bidders post-gate (or downgrade per ADR) — deps: E06-T06 — plan: none — file: crates/roko-compose/src/strategy.rs:79
- [ ] **E06-T08** (focused) Expose [prompt] composition_strategy + vcg_warmup_observations config — deps: E06-T06 — plan: none — file: crates/roko-core/src/config.rs
- [ ] **E06-T09** (mechanical) Verify single canonical entrypoint remains — deps: E06-T02, E06-T03, E06-T04 — plan: none — file: crates/ (structural sweep)

### E07 — Learning & Knowledge Loops  *(make write-only/inert loops durable and closed · 10 tasks)*

- [ ] **E07-T01** (integrative) Add export/import_linucb_snapshot to LinUCBRouter (a) — deps: (E01) — plan: none — file: crates/roko-learn/src/model_router.rs
- [ ] **E07-T02** (focused) Populate CascadeSnapshot.linucb_state in snapshot()/load_from (a) — deps: E07-T01 — plan: none — file: crates/roko-learn/src/cascade_router.rs:1795, cascade/persistence.rs
- [ ] **E07-T03** (focused) Cross-restart LinUCB persistence test + on-disk assertion (a) — deps: E07-T02 — plan: none — file: crates/roko-learn/src/cascade_router.rs
- [ ] **E07-T04** (integrative) Wire knowledge reinforcement into episode-completion path (b) — deps: (E01) — plan: none — file: crates/roko-cli/src/knowledge_helpers.rs, orchestrate.rs
- [ ] **E07-T05** (focused) Make balance/freshness a factor in score_entry_for_query (b) — deps: E07-T04 — plan: none — file: crates/roko-neuro/src/knowledge_store.rs
- [ ] **E07-T06** (focused) Route record_lifecycle_knowledge (4th writer) through admission gate (e) — deps: (E01) — plan: none — file: crates/roko-cli/src/knowledge_helpers.rs:199
- [ ] **E07-T07** (focused) Enable the hdc cargo feature in shipped binaries (d) — deps: (E01) — plan: none — file: crates/roko-cli/Cargo.toml, roko-serve/Cargo.toml
- [ ] **E07-T08** (focused) Backfill HDC vectors for existing knowledge entries (d) — deps: E07-T07 — plan: none — file: crates/roko-cli/src/commands/knowledge.rs
- [ ] **E07-T09** (integrative) Unify legacy vs Runner-v2 knowledge routing (parity, post-P19) (c) — deps: E07-T02, P19(landed) — plan: P19(prereq) — file: crates/roko-cli/src/runner/event_loop.rs:4231, orchestrate.rs
- [ ] **E07-T10** (focused) Incremental adaptive gate-threshold flush (f) — deps: (E01) — plan: none — file: crates/roko-cli/src/orchestrate.rs, roko-learn/src/runtime_feedback.rs:234

### E08 — Conductor Supervision For The Live Engine  *(wire the dark ~10K-LOC reactive layer into runner-v2 · 9 tasks)*

- [ ] **E08-T01** (focused) RunnerEvent/AgentEvent → Engram adapter for conductor watchers — deps: none (E01) — plan: none — file: crates/roko-cli/src/runner/conductor_adapter.rs (new), mod.rs
- [ ] **E08-T02** (focused) Add bounded conductor_ring fed by ConductorRingSink FeedbackSink decorator — deps: E08-T01 — plan: none — file: crates/roko-cli/src/runner/conductor_adapter.rs, runtime_feedback/mod.rs
- [ ] **E08-T03** (integrative) Construct Conductor::from_config and thread it + ring into the event loop — deps: E08-T02 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs, types.rs
- [ ] **E08-T04** (architectural) Add supervision select! branch (evaluate_full → cancel/re-queue) — deps: E08-T03 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs
- [ ] **E08-T05** (focused) Real conductor_load (kill the hardcoded 0.0) — deps: E08-T03 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs:4258, runtime_feedback/routing.rs:132
- [ ] **E08-T06** (integrative) Snapshot / restore circuit-breaker state on --resume — deps: E08-T03 — plan: none — file: crates/roko-cli/src/runner/snapshot_writer.rs, resume.rs, event_loop.rs
- [ ] **E08-T07** (focused) Close the loop into routing + docs/smoke — deps: E08-T04, E08-T05 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs, dispatch/model_routing.rs
- [ ] **E08-T08** (focused) Add DiskSpaceWatcher to conductor watchers — deps: E08-T03 — plan: none — file: crates/roko-conductor/src/watchers/disk_space.rs, watchers/mod.rs, conductor.rs
- [ ] **E08-T09** (focused) Add WorktreeCountWatcher to conductor watchers — deps: E08-T03 — plan: none — file: crates/roko-conductor/src/watchers/worktree_count.rs, watchers/mod.rs, conductor.rs

### E14 — Providers & Tools  *(honest, complete dispatch path every self-run flows through · 12 tasks)*

- [ ] **E14-T01** (focused) Stop advertising builtins that have no executable handler (c/P0) — deps: none (soft E01-T01) — plan: none — file: crates/roko-std/src/tool/handlers.rs:26, registry.rs, builtin/mod.rs
- [ ] **E14-T02** (integrative) Implement the 4 ISFR tool handlers (c) — deps: E14-T01 — plan: none — file: crates/roko-std/src/tool/builtin/isfr.rs, handlers.rs
- [ ] **E14-T03** (architectural) Implement or feature-gate the 17 chain-domain tool handlers (c) — deps: E14-T01 — plan: none — file: crates/roko-std/src/tool/handlers.rs, builtin/mod.rs, roko-chain/src/tools.rs
- [ ] **E14-T04** (integrative) Add streaming to the Gemini native backend (d) — deps: none — plan: none — file: crates/roko-agent/src/tool_loop/backends/gemini_native.rs:162
- [ ] **E14-T05** (focused) Add ProviderKind::GeminiCli and wire its dispatch (f) — deps: E14-T04 — plan: none — file: crates/roko-core/src/agent.rs:35, roko-agent/src/provider/mod.rs, pre_flight.rs
- [ ] **E14-T06** (integrative) Pass image blocks through non-Anthropic translators (e) — deps: none (soft P28-T3) — plan: P28(tail) — file: crates/roko-agent/src/translate/gemini.rs, mod.rs, provider/openai_compat.rs
- [ ] **E14-T07** (focused) Regression test: advertised builtins == executable handlers — deps: E14-T01, E14-T02, E14-T03 — plan: none — file: crates/roko-std/src/tool/registry.rs (or tests/)
- [ ] **E14-T08** (integrative) Implement per-provider rate limit tracking (RPM/TPM counters) — deps: none — plan: none — file: crates/roko-agent/src/dispatcher/mod.rs, provider/mod.rs, roko-core/src/config.rs
- [ ] **E14-T09** (focused) Add Retry-After header parsing for all LLM backends — deps: none — plan: none — file: crates/roko-agent/src/openai_compat_backend.rs, tool_loop/backends/gemini_native.rs, dispatcher/mod.rs
- [ ] **E14-T10** (integrative) Add provider health degradation tracking in CascadeRouter — deps: E14-T08 — plan: none — file: crates/roko-learn/src/cascade_router.rs, roko-agent/src/dispatcher/mod.rs
- [ ] **E14-T11** (focused) Register GitHub MCP tools in the builtin tool catalog — deps: E14-T01 — plan: none — file: crates/roko-std/src/tool/builtin/mod.rs, tool/handlers.rs, tool/registry.rs
- [ ] **E14-T12** (integrative) Add GitHub rate limit state to provider health tracking — deps: E14-T08 — plan: none — file: crates/roko-mcp-github/src/main.rs, roko-agent/src/provider/mod.rs, mcp/client.rs

### E15 — MCP Config & Passthrough  *(make MCP passthrough actually deliver tools · 7 tasks)*

- [ ] **E15-T1** (focused) Normalize McpConfig → Claude {"mcpServers":{}} in resolve_mcp_config_path (a) — deps: none — plan: none — file: crates/roko-cli/src/orchestrate.rs:4265
- [ ] **E15-T2** (focused) Pass per-server env via spawn_with_env in setup_mcp (b) — deps: none — plan: none — file: crates/roko-cli/src/orchestrate.rs:4172
- [ ] **E15-T3** (mechanical) Group MCP tools by '.' server prefix, not '__' (c) — deps: none — plan: none — file: crates/roko-cli/src/orchestrate.rs:4231
- [ ] **E15-T4** (integrative) Thread session mcp_servers into Claude-CLI ACP dispatch (d) — deps: E15-T1 — plan: P25-T4(superseded) — file: crates/roko-acp/src/bridge_events.rs
- [ ] **E15-T5** (focused) Emit readOnlyHint/openWorldHint annotations from tool_spec (e) — deps: none — plan: none — file: crates/roko-mcp-code/src/lib.rs:1545
- [ ] **E15-T6** (mechanical) Neutralize the dead C4 writer so it can't clobber C5's path (f) — deps: E15-T1 — plan: none — file: crates/roko-agent/src/process/mcp.rs
- [ ] **E15-T7** (focused) Auto-discover roko-mcp-github binary and configure it — deps: E15-T1, E15-T6 — plan: none — file: crates/roko-cli/src/orchestrate.rs, roko-agent/src/mcp/config.rs

---

## M2 — Surfaces, Hygiene, Pipeline, ACP & Docs/Ops  *(57 tasks · E09, E10, E11, E12, E16, E17, E18)*

### E09 — Observability  *(thread the built MetricRegistry live; bound logs; trim the firehose · 11 tasks)*

- [ ] **E09-T01** (mechanical) Thread MetricRegistry into RunConfig.metrics (THE fix) — deps: (E01) — plan: none — file: crates/roko-cli/src/commands/plan.rs:569
- [ ] **E09-T02** (small) Write runner metrics to .roko/metrics/prometheus.txt post-run — deps: E09-T01 — plan: none — file: crates/roko-cli/src/commands/plan.rs
- [ ] **E09-T03** (mechanical) Thread serve AppState MetricRegistry into serve_runtime RunConfig — deps: E09-T01 — plan: none — file: crates/roko-cli/src/serve_runtime.rs:628
- [ ] **E09-T04** (small) Stop persisting feed_tick/chain_block to events.jsonl — deps: (E01) — plan: none — file: crates/roko-serve/src/... state_hub.rs, dashboard_snapshot.rs
- [ ] **E09-T05** (small) Day-based rotation for roko.log and chain-watcher.log — deps: none — plan: none — file: crates/roko-cli/src/main.rs:2100, roko-serve/src/lib.rs:440
- [ ] **E09-T06** (mechanical) Make ROKO_LOG authoritative across all binaries — deps: none — plan: none — file: apps/roko-chain-watcher/src/main.rs, apps/agent-relay/src/main.rs
- [ ] **E09-T07** (small) GC/cap events.jsonl (size-based or split run-events.jsonl) — deps: E09-T04 — plan: none — file: crates/roko-fs layout + StateHub
- [ ] **E09-T08** (medium) Attach FsObservabilitySinks in runner-v2 tool loop (or delete dir bootstrap) — deps: (E01) — plan: none — file: crates/roko-cli/src/... runner-v2 tool loop, main.rs:3098
- [ ] **E09-T09** (design) Design the v2 telemetry-as-Lens pipeline (design doc, no runtime code) — deps: E09-T01…E09-T08 — plan: none (feeds E13) — file: design doc
- [ ] **E09-T10** (focused) Add log rotation for episodes.jsonl and signals.jsonl (>100MB) — deps: none — plan: none — file: crates/roko-fs/src/layout.rs, substrate.rs
- [ ] **E09-T11** (focused) Add target/ directory size tracking to MetricRegistry — deps: E09-T01 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs, roko-core/src/obs/metrics.rs

### E10 — Frontend / API Contract  *(fix 4 route 404s, casing drift, double SSE, replay · 7 tasks)*

- [ ] **E10-T01** (mechanical) Align Share.tsx fetch to /api/shared/{token} — deps: none — plan: none — file: demo/demo-app/src/pages/Share.tsx:103
- [ ] **E10-T02** (mechanical) Add /ws/agents WS alias to ws_upgrade — deps: none — plan: none — file: crates/roko-serve/src/routes/ws.rs
- [ ] **E10-T03** (integrative) Add POST /api/bench/matrix route wiring MatrixRun engine — deps: (E01) — plan: none — file: crates/roko-serve/src/routes/bench.rs
- [ ] **E10-T04** (integrative) Add GET /api/isfr/stream SSE route — deps: none — plan: none — file: crates/roko-serve/src/routes/isfr.rs
- [ ] **E10-T05** (focused) Adopt snake_case canonical wire; single deserialization adapter — deps: E03 — plan: none — file: demo/demo-app/src/transport/
- [ ] **E10-T06** (integrative) Remove deprecated EventStreamProvider; collapse to one SSE manager (24 consumers) — deps: E10-T05 — plan: none (P18 adjacent) — file: demo/demo-app/src/main.tsx, contexts/EventStreamContext.tsx
- [ ] **E10-T07** (mechanical) Make sse.rs honor ?n=/?lastEventId= as fallback for replay_from — deps: E10-T05 — plan: none — file: crates/roko-serve/src/routes/sse.rs:42

### E11 — Chain / ISFR  *(recover the missing queue, implement get_logs, deploy parity · 5 tasks)*

- [ ] **E11-T01** (mechanical) Recover architecture-core-queue tasks.toml into committed plans/ — deps: none — plan: unblocks architecture-defi-critical-path — file: plans/architecture-core-queue/tasks.toml (create)
- [ ] **E11-T02** (standard) Implement AlloyChainClient::get_logs via eth_getLogs — deps: none — plan: none — file: crates/roko-chain/src/alloy_impl.rs:147
- [ ] **E11-T03** (standard) Extend Deploy.s.sol to deploy all 13 contracts (ERC-8004 trio + FeeDistributor) — deps: none — plan: none — file: contracts/script/Deploy.s.sol
- [ ] **E11-T04** (mechanical) Delete dead Engram/Provenance/CustodyEntry stubs (coord E03-T07) — deps: none (mutually exclusive w/ E03-T07) — plan: none — file: crates/roko-chain/src/identity_economy_markets.rs
- [ ] **E11-T05** (standard) Wire-or-shelve decision doc for the 16 zero-caller modules + fence daeji — deps: none — plan: none — file: .roko/GAPS.md, CLAUDE.md, docs/v2/22-REGISTRIES.md

### E12 — Dead-Code & Legacy Cleanup  *(remove the ~52K-LOC island after owning epics mine its value · 9 tasks)*

- [ ] **E12-T01** (mechanical) Delete orphan roko-core files pulse_bus.rs and state_hub.rs — deps: none — plan: none — file: crates/roko-core/src/pulse_bus.rs, state_hub.rs *(exemplar EX03)*
- [ ] **E12-T02** (standard) Program runtime against GateRunner trait; drop roko-gate from runtime deps — deps: none — plan: none — file: crates/roko-runtime/Cargo.toml, effect_driver.rs, workflow_engine.rs
- [ ] **E12-T03** (standard) Remove legacy-runner-v2 feature facade; unconditionally compile gated tests — deps: none — plan: none — file: crates/roko-cli/Cargo.toml, tests/{cost_dedup,smoke,phase0_wiring,common/mod}.rs
- [ ] **E12-T04** (focused*) Prune #[allow(dead_code)] members (safe now, incremental) — deps: none — plan: none — file: 37 files across crates/
- [ ] **E12-T05** (standard*) De-dup roko-index HDC onto roko-primitives — deps: E03 (after) — plan: none — file: crates/roko-index/Cargo.toml + local HDC impl
- [ ] **E12-T06** (architectural*) Delete roko-orchestrator incl. duplicate safety/ — deps: E01, E04 (after) — plan: none — file: crates/roko-orchestrator/ (crate + Cargo edges)
- [ ] **E12-T07** (architectural*) Delete orchestrate.rs (23.7K LOC) — deps: E05, E06, E08 (after) — plan: none — file: crates/roko-cli/src/orchestrate.rs, lib.rs
- [ ] **E12-T08** (standard*) Remove legacy-orchestrate feature + run.rs legacy path — deps: E12-T07 (after) — plan: none — file: crates/roko-cli/Cargo.toml, run.rs cfg sites
- [ ] **E12-T09** (standard*) Retire roko-plugin facade (after consumer audit) — deps: audit — plan: none — file: crates/roko-plugin/ (crate + Cargo edges)

### E16 — PRD & Self-Hosting Pipeline (generative front-half)  *(2 gap tasks over P08/P09/P23 · 2 tasks)*

- [ ] **E16-T1** (focused) Rewrite perplexity_integration.rs for the single-query search API — deps: P08(same PR) — plan: P08(reconcile) — file: crates/roko-agent/tests/perplexity_integration.rs
- [ ] **E16-T2** (integrative) Add offline front-half self-hosting smoke test (idea→draft→plan) — deps: E16-T1, P23, P09 — plan: P23+P09(smoke) — file: crates/roko-cli/tests/e2e_self_host.rs

### E17 — ACP Completion  *(consent-gated, learning-informed, MCP-equipped, honest ACP turns · 8 tasks)*

- [ ] **E17-T01** (integrative) Reply-channel permission gate: emit PermissionRequest, gate exec fail-closed (Fa) — deps: E04, P22 (after P21) — plan: E04+P22(co-owned E04-T12→T14) — file: crates/roko-acp/src/bridge_events.rs, builtin_tools.rs
- [ ] **E17-T02** (integrative) Consult ExperimentStore for ACP prompt/model A/B (Fb) — deps: P19, E07 — plan: P19+E07(prereq) — file: crates/roko-acp/src/bridge_events.rs
- [ ] **E17-T03** (integrative) MCP session-tool parity: thread session_mcp_servers into Anthropic path (Fc) — deps: P25 — plan: P25(prereq; coord E15) — file: crates/roko-acp/src/bridge_events.rs
- [ ] **E17-T04** (focused) Derive tool_context.capabilities from role/session consent, not all-true (Fd) — deps: P22(T1) — plan: P22(supersedes T1) — file: crates/roko-acp/src/bridge_events.rs
- [ ] **E17-T05** (focused) Advertised-vs-accepted capability guard (image/audio) tying P28 + T04 (Fd) — deps: E17-T04, P28 — plan: P28(prereq) — file: crates/roko-acp/src/handler.rs, types.rs
- [ ] **E17-T06** (integrative) End-to-end ACP conformance test (consent/select/experiment/Anthropic MCP) — deps: E17-T01, E17-T02, E17-T03, E17-T04 — plan: none — file: crates/roko-acp/src/bridge_events.rs (tests)
- [ ] **E17-T07** (focused) Add cost budget display in ACP turns — deps: none — deps_plan: E01-execution-engine — plan: none — file: crates/roko-acp/src/bridge_events.rs, types.rs, handler.rs
- [ ] **E17-T08** (integrative) Add rate limit status in ACP provider selection — deps: none — deps_plan: E14-providers-tools — plan: none — file: crates/roko-acp/src/bridge_events.rs

### E18 — Docs, Config, CI & Ops Hygiene  *(make the repo truthful & the pipeline provable · 15 tasks)*

- [ ] **E18-T01** (mechanical) Bump workspace rust-version 1.85 → 1.91 (I2) — deps: none — plan: none — file: Cargo.toml:93
- [ ] **E18-T02** (small) Gate release.yml on clippy + test before building binaries (I1) — deps: none — plan: none — file: .github/workflows/release.yml
- [ ] **E18-T03** (small) Add cargo-deny workflow running `cargo deny check` (I3) — deps: none — plan: none — file: .github/workflows/deny.yml (new)
- [ ] **E18-T04** (mechanical) Drop --ignore-run-fail from coverage.yml (I4) — deps: none — plan: none — file: .github/workflows/coverage.yml:20
- [ ] **E18-T05** (small) Fix Docker clean-checkout: track docker/roko.toml (P1) — deps: none — plan: none — file: Dockerfile:77, docker/roko.toml (new)
- [ ] **E18-T06** (medium) Collapse the dual-config silent drop; warn on unknown keys (C1) — deps: none — plan: none — file: crates/roko-cli/src/config.rs:2896
- [ ] **E18-T07** (small) Redact secrets in CLI `config show`/--effective (C2) — deps: none — plan: none (closes E04 redaction) — file: crates/roko-cli/src/commands/config_cmd.rs:215
- [ ] **E18-T08** (medium) `roko deploy docker` push flag + align Fly/compose port & health (P2/P3) — deps: none — plan: none — file: deploy cmd, docker/docker-compose.yml, deploy-fly.yml
- [ ] **E18-T09** (small) Document/deprecate 2 runtime-dead config surfaces (conductor.watchers.*, context_pressure_enabled) + review the cold_storage config surface (C3) — deps: none — plan: none — note: cold_storage is runtime-live (hourly serve timer); its copy-not-move growth bug is owned by E02-T12 — file: crates/roko-core/.../schema.rs + doctor
- [ ] **E18-T10** (mechanical) Rewrite CLAUDE.md from status-quo truth (D1–D10) — deps: E01, E18-T06, E18-T07 — plan: none — file: CLAUDE.md
- [ ] **E18-T11** (mechanical) Rewrite README.md likewise (--engine runner-v2 in every example) — deps: E01, E18-T06, E18-T07, E18-T10 — plan: none — file: README.md
- [ ] **E18-T12** (small) Patch docs/v2/* + docker/README.md with engine/health/flag semantics — deps: E01, E18-T05, E18-T08 — plan: none — file: docs/v2/{CLI-REFERENCE,04-EXECUTION,INTEGRATION-GUIDE,25-DEPLOYMENT}.md, docker/README.md
- [ ] **E18-T13** (small) Add docs-lint CI job grep-guarding forbidden drift patterns (D11) — deps: E18-T10, E18-T11 — plan: none — file: .github/workflows/docs-lint.yml (new)
- [ ] **E18-T14** (focused) Create GitHub Actions workflow for roko plan validation — deps: E18-T02 — plan: none — file: .github/workflows/plan-validate.yml (new)
- [ ] **E18-T15** (mechanical) Document the GitHub integration setup in docs/v2 — deps: E18-T14 — deps_plan: E01-execution-engine, E15-mcp-config — plan: none — file: docs/v2/GITHUB-INTEGRATION.md (new)

---

## M3+ — Long-Horizon v2 Spec-Debt  *(3 tasks · E13)*

### E13 — v2 Spec-Debt  *(load-bearing survivors only; MUST NOT block M0–M2 · 3 tasks)*

- [ ] **E13-T01** (design) Define trait Lens + LensScope in roko-core (no consumers) — deps: E09-T09 — plan: none — file: crates/roko-core/src/obs/lens.rs, obs/mod.rs
- [ ] **E13-T02** (medium) Wrap MetricRegistry as the first CollectorLens feeding StateHub — deps: E13-T01, E09-T01 — plan: none — file: crates/roko-core/src/obs/lens.rs, obs/metrics.rs
- [ ] **E13-T03** (design) Resolve the Cell↔Block↔block naming drift (decision doc, no rename) — deps: (E01 engine decision) — plan: none — file: tmp/status-quo/references/ (decision doc)

---

## Phase 1 — Kernel  *(40 tasks · E19–E22)*

### E19 — Signal Protocol  *(graduate SignalStatus, Pulse, demurrage, IFC taint, HDC fingerprint · 10 tasks)*

- [ ] **E19-T01** (focused) Graduate SignalStatus with Missing → Ready → InFlight → Verified → Archived — deps: none — file: crates/roko-core/src/signal.rs
- [ ] **E19-T02** (focused) Add Pulse::graduate method transitioning from InFlight to Verified — deps: E19-T01 — file: crates/roko-core/src/signal.rs
- [ ] **E19-T03** (focused) Implement Signal demurrage with exponential decay balance — deps: E19-T01 — file: crates/roko-core/src/signal.rs
- [ ] **E19-T04** (integrative) Create KindRegistry for Signal kind registration and validation — deps: none — file: crates/roko-core/src/kind_registry.rs
- [ ] **E19-T05** (integrative) Add IFC taint lattice (4 levels) to Signal metadata — deps: E19-T01 — file: crates/roko-core/src/signal.rs
- [ ] **E19-T06** (focused) Add HDC fingerprint field to Signal with compute-on-create — deps: E19-T01 — file: crates/roko-core/src/signal.rs
- [ ] **E19-T07** (mechanical) Add lineage_hint optional field to Signal metadata — deps: none — file: crates/roko-core/src/signal.rs
- [ ] **E19-T08** (mechanical) Re-export new signal types from roko-core lib.rs — deps: E19-T01, E19-T04, E19-T05 — file: crates/roko-core/src/lib.rs
- [ ] **E19-T09** (focused) Add missing Serialize/Deserialize derives to signal types — deps: E19-T01 — file: crates/roko-core/src/signal.rs
- [ ] **E19-T10** (focused) Unit tests for signal protocol (graduation, demurrage, IFC) — deps: E19-T02, E19-T03, E19-T05 — file: crates/roko-core/src/signal.rs

### E20 — Cell Unification  *(ProtocolId, TypeSchema, Capabilities, predict-publish-correct, CellContext · 10 tasks)*

- [ ] **E20-T01** (focused) Define ProtocolId enum (Sense, Score, Route, Compose, Act, Verify, Write, React) — deps: none — file: crates/roko-core/src/cell.rs
- [ ] **E20-T02** (focused) Add TypeSchema validation struct for Cell input/output contracts — deps: E20-T01 — file: crates/roko-core/src/cell.rs
- [ ] **E20-T03** (focused) Add Capabilities bitmask struct to Cell trait — deps: E20-T01 — file: crates/roko-core/src/cell.rs
- [ ] **E20-T04** (integrative) Implement predict-publish-correct pattern on Cell trait — deps: E20-T01 — file: crates/roko-core/src/cell.rs
- [ ] **E20-T05** (focused) Add CostEstimate struct for budget-aware Cell execution — deps: E20-T01 — file: crates/roko-core/src/cell.rs
- [ ] **E20-T06** (focused) CellContext enrichment with task/plan/session metadata — deps: E20-T01 — file: crates/roko-core/src/cell.rs
- [ ] **E20-T07** (integrative) Implement Cell supertrait unifying 6 verb traits — deps: E20-T01, E20-T02, E20-T03 — file: crates/roko-core/src/cell.rs
- [ ] **E20-T08** (focused) Create CellRegistry for Cell type discovery — deps: E20-T07 — file: crates/roko-core/src/cell_registry.rs
- [ ] **E20-T09** (integrative) Implement Cell for existing roko-std types — deps: E20-T07 — file: crates/roko-std/src/
- [ ] **E20-T10** (mechanical) Re-export Cell types from roko-core lib.rs — deps: E20-T07, E20-T08 — file: crates/roko-core/src/lib.rs

### E21 — Graph Engine  *(typed edges, GraphPolicy, Workflow/Activity, parallel exec, snapshot/resume · 10 tasks)*

- [ ] **E21-T01** (focused) Add typed edge validation to Graph nodes — deps: none — file: crates/roko-graph/src/edge.rs
- [ ] **E21-T02** (integrative) Define GraphPolicy trait for execution constraints — deps: E21-T01 — file: crates/roko-graph/src/policy.rs
- [ ] **E21-T03** (integrative) Split Graph into Workflow (DAG) and Activity (sequential) types — deps: E21-T01 — file: crates/roko-graph/src/workflow.rs
- [ ] **E21-T04** (integrative) Implement parallel execution for independent Graph branches — deps: E21-T03 — file: crates/roko-graph/src/executor.rs
- [ ] **E21-T05** (focused) Add snapshot/resume to Graph execution state — deps: E21-T04 — file: crates/roko-graph/src/snapshot.rs
- [ ] **E21-T06** (focused) Implement nested loop support in Graph execution — deps: E21-T03 — file: crates/roko-graph/src/executor.rs
- [ ] **E21-T07** (integrative) Create GraphCell wrapper implementing Cell trait for Graph — deps: E21-T03 — file: crates/roko-graph/src/graph_cell.rs
- [ ] **E21-T08** (focused) Add edge condition predicates for conditional branching — deps: E21-T01 — file: crates/roko-graph/src/edge.rs
- [ ] **E21-T09** (integrative) Wire MergeQueue into Graph parallel execution — deps: E21-T04 — file: crates/roko-graph/src/merge.rs
- [ ] **E21-T10** (mechanical) Re-export Graph types from roko-graph lib.rs — deps: E21-T07, E21-T09 — file: crates/roko-graph/src/lib.rs

### E22 — Execution Runtime  *(7 cognitive loop cells, short-circuit, error taxonomy, budget, replay · 10 tasks)*

- [ ] **E22-T01** (integrative) Implement 7 cognitive loop cells (Sense, Score, Route, Compose, Act, Verify, React) — deps: none — file: crates/roko-runtime/src/cells/
- [ ] **E22-T02** (focused) Create cognitive-loop.toml defining the default 7-cell pipeline — deps: E22-T01 — file: crates/roko-runtime/src/cognitive_loop.toml
- [ ] **E22-T03** (focused) Implement T0 short-circuit for trivial queries — deps: E22-T01 — file: crates/roko-runtime/src/cells/sense.rs
- [ ] **E22-T04** (focused) Define CellError taxonomy with recovery strategies — deps: E22-T01 — file: crates/roko-runtime/src/error.rs
- [ ] **E22-T05** (integrative) Implement error recovery with retry/fallback per CellError — deps: E22-T04 — file: crates/roko-runtime/src/recovery.rs
- [ ] **E22-T06** (focused) Add budget enforcement to cognitive loop execution — deps: E22-T01 — file: crates/roko-runtime/src/budget.rs
- [ ] **E22-T07** (focused) Implement Activity replay from checkpoint — deps: E22-T01 — file: crates/roko-runtime/src/replay.rs
- [ ] **E22-T08** (integrative) Create FlowHandle for lifecycle management of running flows — deps: E22-T01 — file: crates/roko-runtime/src/flow_handle.rs
- [ ] **E22-T09** (focused) Emit lifecycle Pulses at each cognitive loop transition — deps: E22-T01 — file: crates/roko-runtime/src/cells/
- [ ] **E22-T10** (mechanical) Re-export runtime types from roko-runtime lib.rs — deps: E22-T08 — file: crates/roko-runtime/src/lib.rs

---

## Phase 2a — Agent & Cognition  *(42 tasks · E23–E26)*

### E23 — Agent Cognitive Autonomy  *(lifecycle type-state, behavioral phases, cortical state, energy, EFE routing · 10 tasks)*

- [ ] **E23-T01** (focused) Implement agent lifecycle type-state (Booting, Ready, Executing, Reflecting, Dormant) — deps: none — file: crates/roko-agent/src/lifecycle.rs
- [ ] **E23-T02** (focused) Add behavioral phase state machine (Explore, Exploit, Consolidate, Rest) — deps: E23-T01 — file: crates/roko-agent/src/phases.rs
- [ ] **E23-T03** (focused) Extend CorticalState with attention_focus, working_memory, executive_control — deps: E23-T01 — file: crates/roko-agent/src/cortical.rs
- [ ] **E23-T04** (focused) Implement CognitiveEnergy pool with recharge/depletion dynamics — deps: E23-T01 — file: crates/roko-agent/src/energy.rs
- [ ] **E23-T05** (integrative) Add Expected Free Energy (EFE) routing for goal-directed behavior — deps: E23-T04 — file: crates/roko-agent/src/efe_routing.rs
- [ ] **E23-T06** (integrative) Implement GoalTree with sub-goal decomposition and priority — deps: E23-T05 — file: crates/roko-agent/src/goal_tree.rs
- [ ] **E23-T07** (focused) Add cognitive timescale hierarchy (reactive, deliberative, reflective) — deps: E23-T02 — file: crates/roko-agent/src/timescales.rs
- [ ] **E23-T08** (focused) Wire energy-affect coupling between CognitiveEnergy and DaimonState — deps: E23-T04 — file: crates/roko-agent/src/energy.rs
- [ ] **E23-T09** (integrative) Wire cognitive autonomy into orchestrate.rs dispatch loop — deps: E23-T01, E23-T04, E23-T05 — file: crates/roko-cli/src/orchestrate.rs
- [ ] **E23-T10** (focused) Unit tests for cognitive autonomy (lifecycle, energy, EFE) — deps: E23-T01, E23-T04, E23-T05 — file: crates/roko-agent/src/

### E24 — Memory Advanced  *(heuristic kind, admission gate, Allen intervals, resonator, knowledge income · 10 tasks)*

- [ ] **E24-T01** (integrative) Implement Heuristic knowledge kind with built-in falsifier — deps: none — file: crates/roko-neuro/src/heuristic.rs
- [ ] **E24-T02** (focused) Upgrade admission gate with confidence + novelty + trust scoring — deps: E24-T01 — file: crates/roko-neuro/src/admission.rs
- [ ] **E24-T03** (focused) Add Allen interval temporal queries to KnowledgeStore — deps: none — file: crates/roko-neuro/src/knowledge_store.rs
- [ ] **E24-T04** (integrative) Implement Resonator Networks for associative knowledge retrieval — deps: none — file: crates/roko-neuro/src/resonator.rs
- [ ] **E24-T05** (focused) Add knowledge income policy (demurrage balance governs query budget) — deps: E24-T01 — file: crates/roko-neuro/src/knowledge_store.rs
- [ ] **E24-T06** (focused) Wire dream consolidation trigger from DaimonState — deps: none — file: crates/roko-dreams/src/trigger.rs
- [ ] **E24-T07** (focused) Implement demurrage ODE for continuous knowledge decay — deps: E24-T01 — file: crates/roko-neuro/src/demurrage.rs
- [ ] **E24-T08** (focused) Add calibration scoring for knowledge entry accuracy — deps: E24-T01 — file: crates/roko-neuro/src/calibration.rs
- [ ] **E24-T09** (integrative) Wire memory advanced features into knowledge store dispatch — deps: E24-T01, E24-T02, E24-T04 — file: crates/roko-neuro/src/knowledge_store.rs
- [ ] **E24-T10** (focused) Unit tests for memory advanced (heuristic, admission, resonator) — deps: E24-T01, E24-T02, E24-T04 — file: crates/roko-neuro/src/

### E25 — Learning Loops Advanced  *(HDC defrag, hindsight relabeling, c-factor governance, experiments · 10 tasks)*

- [ ] **E25-T01** (integrative) Implement L3 HDC defragmentation pass for vector space cleanup — deps: none — file: crates/roko-learn/src/hdc_defrag.rs
- [ ] **E25-T02** (focused) Add hindsight relabeling for episode outcomes — deps: none — file: crates/roko-learn/src/hindsight.rs
- [ ] **E25-T03** (integrative) Implement c-factor governance constraining learning rate changes — deps: none — file: crates/roko-learn/src/c_factor.rs
- [ ] **E25-T04** (focused) Add experiment significance testing (chi-square, Bayesian stopping) — deps: none — file: crates/roko-learn/src/experiments.rs
- [ ] **E25-T05** (focused) Implement playbook when/then pattern matching rules — deps: none — file: crates/roko-learn/src/playbook_rules.rs
- [ ] **E25-T06** (focused) Add Variance Inequality budget guard for learning updates — deps: E25-T03 — file: crates/roko-learn/src/variance_inequality.rs
- [ ] **E25-T07** (focused) Implement autocatalytic metrics tracking self-improvement rate — deps: E25-T03 — file: crates/roko-learn/src/autocatalytic.rs
- [ ] **E25-T08** (integrative) Wire PlaybookRules into dispatch prompt enrichment — deps: E25-T05 — file: crates/roko-learn/src/playbook_rules.rs
- [ ] **E25-T09** (integrative) Wire learning loops advanced into orchestrate.rs — deps: E25-T01, E25-T03, E25-T05 — file: crates/roko-cli/src/orchestrate.rs
- [ ] **E25-T10** (focused) Unit tests for learning loops (HDC defrag, c-factor, playbook) — deps: E25-T01, E25-T03, E25-T05 — file: crates/roko-learn/src/

### E26 — Inference Gateway  *(roko-gateway crate, loop detect, cache, tool prune, budget, thinking cap · 12 tasks)*

- [ ] **E26-T01** (integrative) Create roko-gateway crate scaffold with pipeline architecture — deps: none — file: crates/roko-gateway/
- [ ] **E26-T02** (focused) Implement LoopDetectCell for inference loop prevention — deps: E26-T01 — file: crates/roko-gateway/src/cells/loop_detect.rs
- [ ] **E26-T03** (focused) Implement CacheLookupCell for prompt/response caching — deps: E26-T01 — file: crates/roko-gateway/src/cells/cache.rs
- [ ] **E26-T04** (focused) Implement ToolPruneCell for context-aware tool filtering — deps: E26-T01 — file: crates/roko-gateway/src/cells/tool_prune.rs
- [ ] **E26-T05** (focused) Implement OutputBudgetCell for response length enforcement — deps: E26-T01 — file: crates/roko-gateway/src/cells/output_budget.rs
- [ ] **E26-T06** (integrative) Implement ThinkingCap + Convergence for reasoning depth control — deps: E26-T01 — file: crates/roko-gateway/src/cells/thinking.rs
- [ ] **E26-T07** (focused) Implement CostTrackCell for per-request cost attribution — deps: E26-T01 — file: crates/roko-gateway/src/cells/cost_track.rs
- [ ] **E26-T08** (integrative) Create InferenceHandle for lifecycle management of gateway requests — deps: E26-T01 — file: crates/roko-gateway/src/handle.rs
- [ ] **E26-T09** (focused) Implement KeyRing for API key rotation and management — deps: E26-T01 — file: crates/roko-gateway/src/keyring.rs
- [ ] **E26-T10** (focused) Add backpressure mechanism for inference request queuing — deps: E26-T08 — file: crates/roko-gateway/src/backpressure.rs
- [ ] **E26-T11** (focused) Add batch API support for bulk inference requests — deps: E26-T08 — file: crates/roko-gateway/src/batch.rs
- [ ] **E26-T12** (integrative) Assemble the full inference pipeline from cells — deps: E26-T02, E26-T03, E26-T04, E26-T05, E26-T06, E26-T07 — file: crates/roko-gateway/src/pipeline.rs

---

## Phase 2b — Infrastructure  *(62 tasks · E27–E33)*

### E27 — Feeds System  *(FeedInfo v2, FeedRegistration, Feed trait, recipes, marketplace · 10 tasks)*

- [ ] **E27-T01** (focused) Add FeedInfo v2 fields (pricing, metadata, capabilities) — deps: none — file: crates/roko-core/src/feed.rs
- [ ] **E27-T02** (focused) Implement FeedRegistration lifecycle (register, activate, deactivate) — deps: E27-T01 — file: crates/roko-core/src/feed.rs
- [ ] **E27-T03** (integrative) Define Feed trait with poll/push/transform methods — deps: E27-T01 — file: crates/roko-core/src/feed.rs
- [ ] **E27-T04** (focused) Implement feed recipes for common data transformation patterns — deps: E27-T03 — file: crates/roko-core/src/feed_recipes.rs
- [ ] **E27-T05** (focused) Add marketplace economics (pricing tiers, usage tracking) to feeds — deps: E27-T01 — file: crates/roko-core/src/feed.rs
- [ ] **E27-T06** (focused) Add feed config schema to roko.toml parsing — deps: E27-T01 — file: crates/roko-core/src/config.rs
- [ ] **E27-T07** (focused) Add subscribe/unsubscribe routes to roko-serve — deps: E27-T02 — file: crates/roko-serve/src/routes/
- [ ] **E27-T08** (mechanical) Add feed CLI commands (list, subscribe, unsubscribe, status) — deps: E27-T02 — file: crates/roko-cli/src/commands/
- [ ] **E27-T09** (integrative) Wire feed pulses to the Bus event system for downstream consumption — deps: E27-T01, E27-T02 — plan: none — file: crates/roko-core/src/feed_bus_bridge.rs
- [ ] **E27-T10** (focused) Add integration tests for feed registration, lifecycle, and discovery — deps: E27-T01, E27-T02, E27-T04, E27-T05, E27-T06 — plan: none — file: crates/roko-core/src/feed_runtime.rs, recipe.rs, roko-serve/src/routes/feeds.rs

### E28 — Groups Coordination  *(Group types, invitations, leader config, pheromone, API · 8 tasks)*

- [ ] **E28-T01** (focused) Define Group types (WorkGroup, Fleet, Swarm, Arena) — deps: none — file: crates/roko-core/src/group.rs
- [ ] **E28-T02** (focused) Implement GroupInvitation lifecycle (create, accept, reject, expire) — deps: E28-T01 — file: crates/roko-core/src/group.rs
- [ ] **E28-T03** (focused) Add LeaderConfig for group coordination parameters — deps: E28-T01 — file: crates/roko-core/src/group.rs
- [ ] **E28-T04** (integrative) Implement GroupPheromone for stigmergic coordination — deps: E28-T01 — file: crates/roko-core/src/group.rs
- [ ] **E28-T05** (focused) Add Group API routes to roko-serve — deps: E28-T01 — file: crates/roko-serve/src/routes/
- [ ] **E28-T06** (focused) Add group config section to roko.toml — deps: E28-T01 — file: crates/roko-core/src/config.rs
- [ ] **E28-T07** (focused) Implement GroupContextBidder for VCG attention auction — deps: E28-T04 — file: crates/roko-compose/src/
- [ ] **E28-T08** (mechanical) Define GroupEvent enum for event bus integration — deps: E28-T01 — file: crates/roko-core/src/group.rs

### E29 — Connectivity Relay  *(Connect trait, WireEnvelope, reconnection, finality, backpressure · 9 tasks)*

- [ ] **E29-T01** (integrative) Define Connect trait for relay protocol abstraction — deps: none — file: crates/roko-core/src/connect.rs
- [ ] **E29-T02** (focused) Implement ConnectorManifest for relay capability declaration — deps: E29-T01 — file: crates/roko-core/src/connect.rs
- [ ] **E29-T03** (focused) Define WireEnvelope message framing with content-addressed routing — deps: E29-T01 — file: crates/roko-core/src/wire.rs
- [ ] **E29-T04** (focused) Implement ReconnectionState with exponential backoff — deps: E29-T01 — file: crates/roko-core/src/connect.rs
- [ ] **E29-T05** (focused) Add FinalityLevel enum (Tentative, Confirmed, Finalized) — deps: none — file: crates/roko-core/src/finality.rs
- [ ] **E29-T06** (focused) Define exoskeleton types for relay metadata and health — deps: E29-T01 — file: crates/roko-core/src/exoskeleton.rs
- [ ] **E29-T07** (focused) Implement BackpressureStrategy (drop-oldest, reject, buffer) — deps: E29-T01 — file: crates/roko-core/src/backpressure.rs
- [ ] **E29-T08** (integrative) Implement MergedAgent for multi-relay agent composition — deps: E29-T01 — file: crates/roko-agent/src/merged.rs
- [ ] **E29-T09** (mechanical) Define WorkspaceHello handshake protocol — deps: E29-T01 — file: crates/roko-core/src/connect.rs

### E30 — Extension System  *(CaMeL IFC tags, Extension trait, manifest, fault isolation · 9 tasks)*

- [ ] **E30-T01** (focused) Implement CaMeL IFC tags for extension isolation — deps: none — file: crates/roko-core/src/camel_ifc.rs
- [ ] **E30-T02** (focused) Define FilterDecision and BudgetAction enums — deps: E30-T01 — file: crates/roko-core/src/extension.rs
- [ ] **E30-T03** (integrative) Define Extension trait with pre/post hooks for pipeline interception — deps: E30-T01 — file: crates/roko-core/src/extension.rs
- [ ] **E30-T04** (focused) Implement ExtensionManifest for extension metadata and permissions — deps: E30-T03 — file: crates/roko-core/src/extension.rs
- [ ] **E30-T05** (focused) Add dependency resolution for extension load ordering — deps: E30-T04 — file: crates/roko-core/src/extension.rs
- [ ] **E30-T06** (focused) Implement fault isolation wrapping extensions in panic/timeout guards — deps: E30-T03 — file: crates/roko-core/src/extension.rs
- [ ] **E30-T07** (integrative) Wire CaMeL tag propagation through the extension pipeline — deps: E30-T01, E30-T03 — file: crates/roko-core/src/extension.rs
- [ ] **E30-T08** (focused) Wire extension lifecycle into roko-serve startup/shutdown — deps: E30-T03 — file: crates/roko-serve/src/
- [ ] **E30-T09** (focused) Add GET /api/extensions status routes to roko-serve — deps: E30-T03, E30-T05 — plan: none — file: crates/roko-serve/src/routes/extensions.rs (new), routes/mod.rs

### E31 — Trigger System  *(TriggerProtocol trait, concurrency, auth, CellAdapter, TOML persistence · 8 tasks)*

- [ ] **E31-T01** (integrative) Define TriggerProtocol trait with evaluate/fire/reset methods — deps: none — file: crates/roko-core/src/trigger.rs
- [ ] **E31-T02** (focused) Implement ConcurrencyPolicy (Sequential, Parallel, Debounced, RateLimited) — deps: E31-T01 — file: crates/roko-core/src/trigger.rs
- [ ] **E31-T03** (focused) Add TriggerAuth for trigger-level permission scoping — deps: E31-T01 — file: crates/roko-core/src/trigger.rs
- [ ] **E31-T04** (integrative) Implement TriggerCellAdapter wrapping triggers as Cells — deps: E31-T01 — file: crates/roko-core/src/trigger.rs
- [ ] **E31-T05** (focused) Add TOML persistence for trigger definitions — deps: E31-T01 — file: crates/roko-core/src/trigger.rs
- [ ] **E31-T06** (focused) Add trigger CLI subcommands (list, create, enable, disable, fire) — deps: E31-T01 — file: crates/roko-cli/src/commands/
- [ ] **E31-T07** (focused) Add trigger API routes to roko-serve — deps: E31-T01 — file: crates/roko-serve/src/routes/
- [ ] **E31-T08** (focused) Wire trigger events into event Bus topics — deps: E31-T01 — file: crates/roko-runtime/src/

### E32 — Tool Plugin Ecosystem  *(PluginSdkTier, DynamicToolRegistry, declarative tools, sandbox · 8 tasks)*

- [ ] **E32-T01** (focused) Define PluginSdkTier enum (Core, Standard, Community, Experimental) — deps: none — file: crates/roko-core/src/tool.rs
- [ ] **E32-T02** (integrative) Implement DynamicToolRegistry for runtime tool registration — deps: E32-T01 — file: crates/roko-std/src/tool/registry.rs
- [ ] **E32-T03** (focused) Add DeclarativeTool conversion from TOML to executable tool — deps: E32-T01 — file: crates/roko-std/src/tool/
- [ ] **E32-T04** (focused) Implement capability binding for tool permission scoping — deps: E32-T01 — file: crates/roko-std/src/tool/
- [ ] **E32-T05** (focused) Add sandbox config for tool execution isolation — deps: E32-T01 — file: crates/roko-core/src/config.rs
- [ ] **E32-T06** (focused) Implement version resolution for plugin dependencies — deps: E32-T01 — file: crates/roko-std/src/tool/
- [ ] **E32-T07** (focused) Add tool catalog validation (schema check, capability audit) — deps: E32-T02 — file: crates/roko-std/src/tool/
- [ ] **E32-T08** (focused) Wire plugin CLI commands (install, remove, list, audit) — deps: E32-T02 — file: crates/roko-cli/src/commands/

### E33 — Telemetry Lens  *(Observe trait, Lens payloads, StateHub projections, circuit breaker · 9 tasks)*

- [ ] **E33-T01** (integrative) Define Observe trait with lens_id, observe, project methods — deps: none — file: crates/roko-core/src/observe.rs
- [ ] **E33-T02** (focused) Implement 4 core Lens payloads (AgentMetrics, TaskProgress, GateVerdict, BudgetUsage) — deps: E33-T01 — file: crates/roko-core/src/lens.rs
- [ ] **E33-T03** (focused) Implement Error, Drift, and Budget alert payloads — deps: E33-T01 — file: crates/roko-core/src/lens.rs
- [ ] **E33-T04** (focused) Implement C-Factor telemetry payload — deps: E33-T01 — file: crates/roko-core/src/lens.rs
- [ ] **E33-T05** (integrative) Create 7 StateHub projections from Lens data — deps: E33-T02 — file: crates/roko-core/src/state_hub_projections.rs
- [ ] **E33-T06** (focused) Implement Lens composition (combine multiple lenses into one) — deps: E33-T01 — file: crates/roko-core/src/observe.rs
- [ ] **E33-T07** (integrative) Wire StateHub projection pipeline into roko-serve — deps: E33-T05 — file: crates/roko-serve/src/
- [ ] **E33-T08** (focused) Add lens circuit breaker for high-frequency telemetry throttling — deps: E33-T01 — file: crates/roko-core/src/observe.rs
- [ ] **E33-T09** (focused) Add telemetry routes to roko-serve for lens data access — deps: E33-T07 — file: crates/roko-serve/src/routes/

---

## Phase 2c — Security & Auth  *(16 tasks · E34–E35)*

### E34 — Security IFC  *(taint lattice, taint tracker, immune pipeline, corrigibility, sandbox · 8 tasks)*

- [ ] **E34-T01** (focused) Implement TaintLevel lattice (Public, Internal, Sensitive, Critical) — deps: none — file: crates/roko-core/src/taint.rs
- [ ] **E34-T02** (focused) Upgrade TaintTracker with IFC lattice propagation rules — deps: E34-T01 — file: crates/roko-agent/src/safety/
- [ ] **E34-T03** (integrative) Implement immune pipeline for detecting taint violations — deps: E34-T02 — file: crates/roko-agent/src/safety/
- [ ] **E34-T04** (focused) Add corrigibility ordering for agent override hierarchy — deps: E34-T01 — file: crates/roko-agent/src/safety/
- [ ] **E34-T05** (focused) Define sandbox levels (None, Filesystem, Network, Full) — deps: E34-T01 — file: crates/roko-core/src/sandbox.rs
- [ ] **E34-T06** (focused) Implement capability intersection for multi-agent permission merging — deps: E34-T01 — file: crates/roko-agent/src/safety/
- [ ] **E34-T07** (focused) Add QuarantineVault for tainted artifact isolation — deps: E34-T03 — file: crates/roko-agent/src/safety/
- [ ] **E34-T08** (integrative) Wire IFC enforcement into agent dispatch pipeline — deps: E34-T02, E34-T03 — file: crates/roko-agent/src/

### E35 — Auth Protocol  *(API key lifecycle, bearer tokens, JWKS, RBAC, relay tokens · 8 tasks)*

- [ ] **E35-T01** (focused) Implement API key lifecycle (create, rotate, revoke, list) — deps: none — file: crates/roko-serve/src/auth/
- [ ] **E35-T02** (focused) Add agent bearer token issuance and validation — deps: E35-T01 — file: crates/roko-serve/src/auth/
- [ ] **E35-T03** (focused) Harden JWKS verification with key rotation support — deps: E35-T01 — file: crates/roko-serve/src/auth/
- [ ] **E35-T04** (focused) Define RBAC roles (Admin, Operator, Agent, Viewer, Guest) — deps: none — file: crates/roko-serve/src/auth/
- [ ] **E35-T05** (integrative) Implement RBAC middleware for route-level authorization — deps: E35-T04 — file: crates/roko-serve/src/routes/middleware.rs
- [ ] **E35-T06** (focused) Add relay tokens for cross-workspace authentication — deps: E35-T02 — file: crates/roko-serve/src/auth/
- [ ] **E35-T07** (focused) Implement invitation flow for workspace membership — deps: E35-T04 — file: crates/roko-serve/src/auth/
- [ ] **E35-T08** (focused) Add auth audit trail logging all auth events — deps: E35-T01, E35-T05 — file: crates/roko-serve/src/auth/

---

## Phase 3a — Economy  *(17 tasks · E36, E38)*

### E36 — Payments  *(payment domain types, settlement, MPP, pricing, cost tracking · 8 tasks)*

- [ ] **E36-T01** (focused) Define payment domain types (PaymentIntent, Receipt, PaymentState) — deps: none — file: crates/roko-chain/src/payments.rs
- [ ] **E36-T02** (focused) Implement settlement batching for payment aggregation — deps: E36-T01 — file: crates/roko-chain/src/payments.rs
- [ ] **E36-T03** (focused) Add MPP (Multi-Path Payment) session management — deps: E36-T01 — file: crates/roko-chain/src/payments.rs
- [ ] **E36-T04** (focused) Implement pricing tier resolution (Free, Basic, Pro, Enterprise) — deps: E36-T01 — file: crates/roko-chain/src/payments.rs
- [ ] **E36-T05** (focused) Add FeedInfo pricing fields for feed marketplace — deps: E36-T04 — file: crates/roko-core/src/feed.rs
- [ ] **E36-T06** (focused) Implement cost tracking per agent per session — deps: E36-T01 — file: crates/roko-chain/src/payments.rs
- [ ] **E36-T07** (integrative) Wire payment middleware into roko-serve route handlers — deps: E36-T01 — file: crates/roko-serve/src/routes/
- [ ] **E36-T08** (focused) Add payment dashboard events to StateHub — deps: E36-T06 — file: crates/roko-core/src/

### E38 — Marketplace  *(ArtifactKind, TraceRank, PackageTier, publish pipeline, economics · 9 tasks)*

- [ ] **E38-T01** (focused) Define ArtifactKind and ArtifactRef for marketplace listings — deps: none — file: crates/roko-chain/src/marketplace.rs
- [ ] **E38-T02** (focused) Implement TraceRank fork attribution for derivative tracking — deps: none — file: crates/roko-chain/src/trace_rank.rs
- [ ] **E38-T03** (focused) Add PackageTier enum (Free, Starter, Professional, Enterprise) — deps: none — file: crates/roko-chain/src/marketplace.rs
- [ ] **E38-T04** (integrative) Implement publish pipeline (validate, sign, register, index) — deps: E38-T01 — file: crates/roko-chain/src/marketplace.rs
- [ ] **E38-T05** (focused) Add economics types (RoyaltySchedule, RevenueShare, UsageFee) — deps: E38-T03 — file: crates/roko-chain/src/marketplace.rs
- [ ] **E38-T06** (focused) Add browse/search routes to roko-serve marketplace — deps: E38-T04 — file: crates/roko-serve/src/routes/
- [ ] **E38-T07** (focused) Implement fork lineage tracking with parent chain — deps: E38-T02 — file: crates/roko-chain/src/marketplace.rs
- [ ] **E38-T08** (focused) Add capability intersection for marketplace artifact permissions — deps: E38-T01 — file: crates/roko-chain/src/marketplace.rs
- [ ] **E38-T09** (mechanical) Add marketplace CLI stubs (list, publish, search, install) — deps: E38-T04 — file: crates/roko-cli/src/commands/

---

## Phase 3b — Surfaces & Registries  *(17 tasks · E37, E39)*

### E37 — Surfaces  *(surface projection contracts, InboxCategory, AutonomyLevel, TUI tabs · 9 tasks)*

- [ ] **E37-T01** (integrative) Define surface projection contracts (Surface trait, ProjectionSpec) — deps: none — file: crates/roko-core/src/surface.rs
- [ ] **E37-T02** (focused) Implement InboxCategory enum for message classification — deps: none — file: crates/roko-core/src/surface.rs
- [ ] **E37-T03** (focused) Define AutonomyLevel (Manual, Supervised, Autonomous, Full) — deps: none — file: crates/roko-core/src/surface.rs
- [ ] **E37-T04** (integrative) Wire TUI tab mapping to surface projections — deps: E37-T01 — file: crates/roko-cli/src/tui/
- [ ] **E37-T05** (focused) Implement Workbench events for task/build surface — deps: E37-T01 — file: crates/roko-core/src/surface.rs
- [ ] **E37-T06** (focused) Implement Inbox events for message/notification surface — deps: E37-T02 — file: crates/roko-core/src/surface.rs
- [ ] **E37-T07** (focused) Implement Autonomy events for control surface — deps: E37-T03 — file: crates/roko-core/src/surface.rs
- [ ] **E37-T08** (focused) Add surface projection routes to roko-serve — deps: E37-T01 — file: crates/roko-serve/src/routes/
- [ ] **E37-T09** (focused) Define 12 primitive surface types — deps: E37-T01 — file: crates/roko-core/src/surface.rs

### E39 — Registries & Identity  *(ERC-8004, DelegationCaveat, KnowledgeRegistry, gossip, reputation · 8 tasks)*

- [ ] **E39-T01** (focused) Convert AgentRegistry from soulbound to ERC-8004 transferable identity — deps: none — file: crates/roko-chain/src/agent_registry.rs
- [ ] **E39-T02** (focused) Implement DelegationCaveat enforcement in AgentRegistry — deps: E39-T01 — file: crates/roko-chain/src/agent_registry.rs
- [ ] **E39-T03** (integrative) Implement KnowledgeRegistry for on-chain InsightStore — deps: none — file: crates/roko-chain/src/knowledge_registry.rs, lib.rs
- [ ] **E39-T04** (focused) Add challenge and resolution protocol to KnowledgeRegistry — deps: E39-T03 — file: crates/roko-chain/src/knowledge_registry.rs
- [ ] **E39-T05** (focused) Integrate TraceRank composite scoring into ReputationRegistry — deps: none — file: crates/roko-chain/src/reputation_registry.rs
- [ ] **E39-T06** (focused) Add gossip peer discovery types and stub to roko-chain — deps: none — file: crates/roko-chain/src/gossip.rs, lib.rs
- [ ] **E39-T07** (focused) Wire Marketplace settlement into ReputationRegistry feedback — deps: none — file: crates/roko-chain/src/marketplace.rs
- [ ] **E39-T08** (focused) Add identity state file and auto-registration check at agent startup — deps: E39-T01 — file: crates/roko-chain/src/identity_economy_identity.rs

---

## Phase 3c — Evals & DeFi  *(16 tasks · E40–E41)*

### E40 — Arenas & Evals  *(Arena/Attempt types, scoring, ArenaRegistry, leaderboard, escrow, flywheel · 8 tasks)*

- [ ] **E40-T01** (integrative) Implement Arena and Attempt core types in roko-chain — deps: none — file: crates/roko-chain/src/arena.rs, lib.rs
- [ ] **E40-T02** (focused) Implement ScoringFunction, BinaryCriterion, ContinuousMetric, Normalization — deps: E40-T01 — file: crates/roko-chain/src/arena.rs
- [ ] **E40-T03** (integrative) Implement ArenaRegistry for arena lifecycle and attempt tracking — deps: E40-T01, E40-T02 — file: crates/roko-chain/src/arena.rs
- [ ] **E40-T04** (focused) Implement Leaderboard derivation from attempt scores — deps: E40-T03 — file: crates/roko-chain/src/arena.rs
- [ ] **E40-T05** (focused) Add bounty escrow to ArenaRegistry for prize-pool arenas — deps: E40-T04 — file: crates/roko-chain/src/arena.rs
- [ ] **E40-T06** (focused) Define FlywheelStage enum and FlywheelPipeline for arena-to-learning — deps: E40-T01 — file: crates/roko-chain/src/arena.rs
- [ ] **E40-T07** (mechanical) Wire arena completion into reputation effects — deps: E40-T03 — file: crates/roko-chain/src/arena.rs
- [ ] **E40-T08** (focused) Add arena REST route stubs to roko-serve — deps: E40-T01 — file: crates/roko-serve/src/routes/arenas.rs, mod.rs

### E41 — DeFi Products  *(VCG clearing cell, yield perps, VenueAdapter, risk engine, FIFO P&L · 8 tasks)*

- [ ] **E41-T01** (focused) Formalize VCG clearing into a standalone VcgClearingCell in roko-compose — deps: none — file: crates/roko-compose/src/auction.rs
- [ ] **E41-T02** (focused) Add YieldPerpPosition and ClearingRound types to roko-chain — deps: none — file: crates/roko-chain/src/yield_perps.rs, lib.rs
- [ ] **E41-T03** (integrative) Define VenueAdapter trait and MockVenue for DeFi protocol interaction — deps: none — file: crates/roko-chain/src/venue.rs, lib.rs
- [ ] **E41-T04** (focused) Implement DeFiRiskEngine with position limits and drawdown tracking — deps: none — file: crates/roko-chain/src/defi_risk.rs, lib.rs
- [ ] **E41-T05** (focused) Implement FifoMatcher for FIFO P&L attribution — deps: none — file: crates/roko-chain/src/trading_reflect.rs, lib.rs
- [ ] **E41-T06** (mechanical) Add prospect_value and affect_size_multiplier functions to roko-daimon — deps: none — file: crates/roko-daimon/src/lib.rs
- [ ] **E41-T07** (focused) Add [defi] config section to roko-core config parsing — deps: none — file: crates/roko-core/src/config.rs
- [ ] **E41-T08** (focused) Add DeFi REST route stubs to roko-serve — deps: none — file: crates/roko-serve/src/routes/mod.rs

---

## Phase 3d — Cross-cutting & Operations  *(34 tasks · E42–E45)*

### E42 — Config Evolution  *(ConfigSource priority, invariants, hot reload, migration, profiles · 8 tasks)*

- [ ] **E42-T01** (focused) Add ConfigSource::Evolved and ConfigSource::Composed variants with priority ordering — deps: none — file: crates/roko-core/src/config/provenance.rs
- [ ] **E42-T02** (focused) Implement the 7 config invariants as validation functions — deps: none — file: crates/roko-core/src/config/validation.rs
- [ ] **E42-T03** (focused) Add debounced file-watch trigger to hot_reload module — deps: E42-T02 — file: crates/roko-core/src/config/hot_reload.rs
- [ ] **E42-T04** (focused) Implement config migration chain with versioned migration functions — deps: none — file: crates/roko-core/src/config/loader.rs
- [ ] **E42-T05** (integrative) Implement per-field priority merge in config loading — deps: E42-T01, E42-T04 — file: crates/roko-core/src/config/loader.rs, provenance.rs
- [ ] **E42-T06** (focused) Wire validate_invariants into the config load path — deps: E42-T02, E42-T05 — file: crates/roko-core/src/config/loader.rs
- [ ] **E42-T07** (focused) Add DomainProfile struct with base-profile inheritance — deps: none — file: crates/roko-core/src/config/schema.rs
- [ ] **E42-T08** (focused) Add config staleness check with demurrage-based warnings — deps: E42-T02 — file: crates/roko-core/src/config/hot_reload.rs

### E43 — Deployment & Portability  *(brain export/import, daemon lifecycle, secrets rotation, Docker · 8 tasks)*

- [ ] **E43-T01** (focused) Implement brain export to portable knowledge bundle — deps: none — file: crates/roko-neuro/src/knowledge_store.rs
- [ ] **E43-T02** (focused) Implement brain import with selective decay and dedup — deps: E43-T01 — file: crates/roko-neuro/src/knowledge_store.rs
- [ ] **E43-T03** (focused) Wire daemon install to generate platform-specific service configs — deps: none — file: crates/roko-cli/src/daemon.rs
- [ ] **E43-T04** (focused) Implement daemon health check combining PID liveness and HTTP probe — deps: E43-T03 — file: crates/roko-cli/src/daemon.rs
- [ ] **E43-T05** (focused) Add log rotation for daemon stdout/stderr output — deps: E43-T03 — file: crates/roko-cli/src/daemon.rs
- [ ] **E43-T06** (focused) Implement secrets rotation with hot-swap signal to roko-serve — deps: none — file: crates/roko-cli/src/commands/server.rs
- [ ] **E43-T07** (focused) Implement deployment tier advisor recommending Solo/Team/Production — deps: none — file: crates/roko-cli/src/status.rs
- [ ] **E43-T08** (focused) Add Dockerfile and Railway deploy template — deps: none — file: crates/roko-cli/src/commands/util.rs

### E44 — Cross-Cut Functors  *(CrossCutFunctor trait, Memory/Daimon/Dreams/Safety functors, VCG arbitration · 8 tasks)*

- [ ] **E44-T01** (focused) Define CrossCutFunctor trait with pre_enrich and post_enrich hooks — deps: none — file: crates/roko-compose/src/lib.rs
- [ ] **E44-T02** (integrative) Implement MemoryFunctor wrapping neuro KnowledgeStore queries — deps: E44-T01 — file: crates/roko-compose/src/strategy.rs
- [ ] **E44-T03** (integrative) Implement DaimonFunctor wrapping PAD bias and somatic markers — deps: E44-T01 — file: crates/roko-compose/src/strategy.rs
- [ ] **E44-T04** (focused) Implement DreamsFunctor wrapping dream cycle output injection — deps: E44-T01 — file: crates/roko-compose/src/strategy.rs
- [ ] **E44-T05** (integrative) Implement 6 natural transformations forming the cross-cut triangle — deps: E44-T02, E44-T03, E44-T04 — file: crates/roko-compose/src/strategy.rs
- [ ] **E44-T06** (integrative) Wire VCG auction as tiebreaker when cross-cuts produce conflicting recommendations — deps: E44-T02, E44-T03 — file: crates/roko-compose/src/auction.rs
- [ ] **E44-T07** (focused) Implement SafetyFunctor as capability-level pre-filter outside VCG — deps: E44-T01, E44-T06 — file: crates/roko-compose/src/strategy.rs
- [ ] **E44-T08** (integrative) Implement gate failure cascade through Memory -> Daimon -> Dreams pipeline — deps: E44-T05, E44-T06 — file: crates/roko-cli/src/runner/event_loop.rs

### E45 — Orchestrator Mori Parity  *(review verdict, compile auto-fix, error sharing, reflection, warm pool · 10 tasks)*

- [ ] **E45-T01** (focused) Wire ReviewVerdict parsing from agent output into runner event loop — deps: none — file: crates/roko-cli/src/runner/agent_events.rs
- [ ] **E45-T02** (focused) Add cargo fix auto-fix path before agent retry on compile failures — deps: none — file: crates/roko-cli/src/runner/gate_dispatch.rs
- [ ] **E45-T03** (focused) Wire error pattern sharing via discovered-patterns.json for parallel agents — deps: none — file: crates/roko-cli/src/runner/event_loop.rs
- [ ] **E45-T04** (focused) Wire post-gate reflection loop with dedup and cost guard — deps: E45-T02 — file: crates/roko-cli/src/runner/event_loop.rs
- [ ] **E45-T05** (focused) Implement role-filtered context injection scoping — deps: none — file: crates/roko-cli/src/runner/event_loop.rs
- [ ] **E45-T06** (focused) Wire WarmPool into runner for pre-spawned agent transitions — deps: none — file: crates/roko-cli/src/runner/event_loop.rs
- [ ] **E45-T07** (focused) Wire KnowledgeStore queries into CascadeRouter for model bias — deps: none — file: crates/roko-learn/src/cascade_router.rs
- [ ] **E45-T08** (focused) Wire provider pass-rate metrics into CascadeRouter model scoring — deps: E45-T07 — file: crates/roko-learn/src/cascade_router.rs
- [ ] **E45-T09** (focused) Wire reflection-derived playbook rules with confidence decay — deps: E45-T04 — file: crates/roko-learn/src/playbook_rules.rs
- [ ] **E45-T10** (integrative) Wire A-MAC 5-factor admission gate into KnowledgeStore ingestion — deps: none — file: crates/roko-neuro/src/admission.rs

---

## Operational Capabilities  *(35 tasks · E46–E48)*

### E46 — GitHub Workflow Integration  *(config, signals, API client, GitHubOps trait, runner wiring, issues, PRs, CI, webhooks · 12 tasks)*

- [ ] `E46-T01` — Add [github] config section to RokoConfig schema
- [ ] `E46-T02` — Add CI and merge signal kind constants
- [ ] `E46-T03` — Extract GitHub API client into roko-mcp-github lib.rs
- [ ] `E46-T04` — Define GitHubOps trait for runner-level GitHub operations
- [ ] `E46-T05` — Implement LiveGitHubOps backed by GitHubClient
- [ ] `E46-T06` — Wire GitHubOps into RunContext and event loop
- [ ] `E46-T07` — Create GitHub issues for terminal task failures
- [ ] `E46-T08` — Post PR comments on task gate results
- [ ] `E46-T09` — Check CI status before PR merge
- [ ] `E46-T10` — Wire GitHub webhook events to plan triggers
- [ ] `E46-T11` — Add `roko github status` CLI command
- [ ] `E46-T12` — Integration test for GitHub workflow with mock GitHubOps

### E47 — Resource & Disk Management  *(config, disk monitor, GC hooks, worktree cleanup, log rotation, pressure watcher · 11 tasks)*

- [ ] `E47-T01` — Add [resources] config section with disk budget, thresholds, and GC policy
- [ ] `E47-T02` — Implement cross-platform disk space checker and runtime monitor
- [ ] `E47-T03` — Wire GcEngine into PlanRunner lifecycle hooks (pre-run, post-run, on-failure)
- [ ] `E47-T04` — Wire DiskMonitor pre-check into PlanRunner startup — refuse to run on low disk
- [ ] `E47-T05` — Implement Rust target/ directory scanner and cleanup for workspaces and worktrees
- [ ] `E47-T06` — Wire worktree auto-cleanup on plan completion, failure, and orphan detection
- [ ] `E47-T07` — Implement log rotation for episodes.jsonl, signals.jsonl, and efficiency.jsonl
- [ ] `E47-T08` — Add DiskPressureWatcher to conductor — pause/intervene on low disk during execution
- [ ] `E47-T09` — Track aggregate disk usage across parallel plan worktrees and serialize on pressure
- [ ] `E47-T10` — Add `roko doctor disk` subcommand for stale artifact detection and disk health report
- [ ] `E47-T11` — Wire target/ cleanup and log rotation into the plan lifecycle alongside GC

### E48 — Rate Limit & Token Budgeting  *(retry policy, RPM/TPM limits, pooling, budget halt, health registry, queuing, estimation · 12 tasks)*

- [ ] `E48-T01` — Wire RetryPolicy into provider dispatch loop for 429/529 retries
- [ ] `E48-T02` — Add per-provider RPM and TPM limits to ProviderRateLimiter
- [ ] `E48-T03` — Pool ProviderRateLimiter across parallel agents in orchestrator
- [ ] `E48-T04` — Enforce budget halt — pause plan execution when budget exceeded
- [ ] `E48-T05` — Integrate ProviderHealthRegistry into CascadeRouter model selection
- [ ] `E48-T06` — Queue tasks for retry on rate limit instead of failing
- [ ] `E48-T07` — Pre-dispatch token estimation to reject oversized prompts
- [ ] `E48-T08` — Proportional budget allocation per task based on tier and complexity
- [ ] `E48-T09` — Cost projection — estimate remaining plan cost from historical per-task averages
- [ ] `E48-T10` — Wire BudgetAction::RouteToCheaper to CascadeRouter model downgrade
- [ ] `E48-T11` — Record dispatch outcomes in ProviderHealthRegistry
- [ ] `E48-T12` — Expose plan cost and rate limit state via HTTP + TUI cost tab

---

## Quick start — the first 5 tasks to do

The M0 critical path. Do these in order; the first flips the whole system from fabricating success to
doing real work, the rest make that work honest and safe. Exemplar `.toml` files (drop-in,
`roko plan validate`-clean) live under `backlog/exemplars/`.

1. **Flip engine default** — `E01-T01` (mechanical) — clap `default_value = "graph"` → `"runner-v2"`.
   `crates/roko-cli/src/main.rs:1361`. **THE self-hosting unblock.** → exemplar **EX01-flip-engine-default.toml**
   *(EX01 also covers the paired resume fix E01-T02 + the lock-it-with-a-test E01-T09).*
2. **Honest gates** — `E05-T02` (focused) — make unwired advanced-rung stubs return **Skipped**, not
   `Verdict::pass`, so a stub-only pipeline can't report green. `crates/roko-gate/src/rung_dispatch.rs:290`.
   (Chains into E05-T03/T04/T05 for real per-rung EMA + `enrich_rung_config`.) → no exemplar
3. **Safety-enforce** — `E04-T05` (focused) — promote SecretLeak + PathEscape post-checks from `Warn` to
   **Block** so unattended agents are refused, not just logged. `crates/roko-agent/src/safety/mod.rs:767`.
   (Pairs with E04-T06 to run the funnel on the default Claude-CLI path.) → no exemplar
4. **Verify** — `E01-T09` (focused) — regression test that a bare-default `plan run` (no `--engine` flag)
   writes real episode + snapshot artifacts, locking the default so it can't silently regress to Graph.
   `crates/roko-cli/tests/` (new). → no exemplar
5. **Storage unify** — `E02-T01` (integrative) — write gate verdicts to the store the dashboards actually
   read (engrams path), stop writing `signals.jsonl`; makes gate panels non-empty.
   `crates/roko-cli/src/runner/event_loop.rs:1147`. → exemplar **EX02-unify-signal-store.toml**

> Warm-up option: **EX03-delete-orphan-statehub.toml** (= `E03-T01` / `E12-T01`) is a zero-risk mechanical
> deletion good for validating the executor end-to-end before touching load-bearing code.
