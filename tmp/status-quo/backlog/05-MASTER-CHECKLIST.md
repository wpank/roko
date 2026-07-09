# 05 — Master Checklist

> **The single scannable checklist for the entire executable backlog.**
> - Repo HEAD: `5852c93c05` on `main` · authored 2026-07-09
> - Source: every task below is lifted verbatim (id · title · tier · deps) from
>   `backlog/epics/E01…E18-*.md`. Plan mapping from `backlog/02-PLANS-RECONCILIATION.md`.
> - Scope: **18 epics · 149 tasks.** Every epic E01–E18 is represented; each task appears exactly once.
> - Ordering: by milestone (M0 → M1 → M2 → M3+), then by epic, then by task id.

---

## Summary

| Milestone | Theme | Epics | Tasks | Self-execution possible after? |
|---|---|---|---|---|
| **M0** | Bootstrap & correctness (honest, safe self-execution) | E01, E02, E03, E04, E05 | **56** | **YES** — becomes real at E01-T01, honest by E05, safe by E04 (this milestone *is* the self-hosting gate) |
| **M1** | Close the live-path loops (compose, learning, conductor, providers, MCP) | E06, E07, E08, E14, E15 | **39** | YES + effective/durable |
| **M2** | Surfaces, hygiene, pipeline, ACP, docs/ops | E09, E10, E11, E12, E16, E17, E18 | **51** | YES + observable/shippable |
| **M3+** | Long-horizon v2 spec-debt | E13 | **3** | YES + spec-complete |
| | | **18** | **149** | |

> **Milestone assignment note:** only **E01** (explicit *M0*) and **E13** (explicit *M3+*) carry a
> milestone in their epic files. M0 membership (E01–E05) follows the "Minimum Viable Self-Hosting"
> gate in `04-EXECUTION-READINESS.md` and the M0 critical path (flip-engine / honest-gates /
> safety-enforce / verify / storage-unify). M1/M2 placement follows the dependency DAG and the
> ordering in `02-PLANS-RECONCILIATION.md §4`. Adjust freely — the task ids/tiers/deps are authoritative, the milestone buckets are a scheduling convenience.

### Legend

**Tiers** (ascending effort/risk): `mechanical` < `focused`/`small`/`standard` < `integrative`/`medium`/`complex` < `architectural`/`design`/`planning`.
Note: epics use slightly different tier vocabularies; they are reproduced as written. Tiers marked `*` were left implicit in the epic file (E12-T04…T09) and are inferred from the task description.

**Status** (all currently `ready` / unstarted):
- `[ ]` not started · `[x]` done
- **plan:** existing `plans/Pxx` coverage — `full` = plan aims correctly · `partial`/`shallow` = plan patches a symptom · `superseded` = plan retired by this task · `prereq` = plan must land first · `none` = pure gap (no plan)
- **deps:** task-level `depends_on` (epic-level cross-epic deps shown in parentheses, e.g. `(E01)`)
- **file:** primary target (see epic file for full `read_files`/context)

---

## M0 — Bootstrap & Correctness  *(56 tasks · E01–E05)*

### E01 — Execution Engine  *(make bare `plan run` do real work + close the live-path holdouts · 10 tasks)*

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

## M1 — Close The Live-Path Loops  *(39 tasks · E06, E07, E08, E14, E15)*

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

### E08 — Conductor Supervision For The Live Engine  *(wire the dark ~10K-LOC reactive layer into runner-v2 · 7 tasks)*

- [ ] **E08-T01** (focused) RunnerEvent/AgentEvent → Engram adapter for conductor watchers — deps: none (E01) — plan: none — file: crates/roko-cli/src/runner/conductor_adapter.rs (new), mod.rs
- [ ] **E08-T02** (focused) Add bounded conductor_ring fed by ConductorRingSink FeedbackSink decorator — deps: E08-T01 — plan: none — file: crates/roko-cli/src/runner/conductor_adapter.rs, runtime_feedback/mod.rs
- [ ] **E08-T03** (integrative) Construct Conductor::from_config and thread it + ring into the event loop — deps: E08-T02 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs, types.rs
- [ ] **E08-T04** (architectural) Add supervision select! branch (evaluate_full → cancel/re-queue) — deps: E08-T03 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs
- [ ] **E08-T05** (focused) Real conductor_load (kill the hardcoded 0.0) — deps: E08-T03 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs:4258, runtime_feedback/routing.rs:132
- [ ] **E08-T06** (integrative) Snapshot / restore circuit-breaker state on --resume — deps: E08-T03 — plan: none — file: crates/roko-cli/src/runner/snapshot_writer.rs, resume.rs, event_loop.rs
- [ ] **E08-T07** (focused) Close the loop into routing + docs/smoke — deps: E08-T04, E08-T05 — plan: none — file: crates/roko-cli/src/runner/event_loop.rs, dispatch/model_routing.rs

### E14 — Providers & Tools  *(honest, complete dispatch path every self-run flows through · 7 tasks)*

- [ ] **E14-T01** (focused) Stop advertising builtins that have no executable handler (c/P0) — deps: none (soft E01-T01) — plan: none — file: crates/roko-std/src/tool/handlers.rs:26, registry.rs, builtin/mod.rs
- [ ] **E14-T02** (integrative) Implement the 4 ISFR tool handlers (c) — deps: E14-T01 — plan: none — file: crates/roko-std/src/tool/builtin/isfr.rs, handlers.rs
- [ ] **E14-T03** (architectural) Implement or feature-gate the 17 chain-domain tool handlers (c) — deps: E14-T01 — plan: none — file: crates/roko-std/src/tool/handlers.rs, builtin/mod.rs, roko-chain/src/tools.rs
- [ ] **E14-T04** (integrative) Add streaming to the Gemini native backend (d) — deps: none — plan: none — file: crates/roko-agent/src/tool_loop/backends/gemini_native.rs:162
- [ ] **E14-T05** (focused) Add ProviderKind::GeminiCli and wire its dispatch (f) — deps: E14-T04 — plan: none — file: crates/roko-core/src/agent.rs:35, roko-agent/src/provider/mod.rs, pre_flight.rs
- [ ] **E14-T06** (integrative) Pass image blocks through non-Anthropic translators (e) — deps: none (soft P28-T3) — plan: P28(tail) — file: crates/roko-agent/src/translate/gemini.rs, mod.rs, provider/openai_compat.rs
- [ ] **E14-T07** (focused) Regression test: advertised builtins == executable handlers — deps: E14-T01, E14-T02, E14-T03 — plan: none — file: crates/roko-std/src/tool/registry.rs (or tests/)

### E15 — MCP Config & Passthrough  *(make MCP passthrough actually deliver tools · 6 tasks)*

- [ ] **E15-T1** (focused) Normalize McpConfig → Claude {"mcpServers":{}} in resolve_mcp_config_path (a) — deps: none — plan: none — file: crates/roko-cli/src/orchestrate.rs:4265
- [ ] **E15-T2** (focused) Pass per-server env via spawn_with_env in setup_mcp (b) — deps: none — plan: none — file: crates/roko-cli/src/orchestrate.rs:4172
- [ ] **E15-T3** (mechanical) Group MCP tools by '.' server prefix, not '__' (c) — deps: none — plan: none — file: crates/roko-cli/src/orchestrate.rs:4231
- [ ] **E15-T4** (integrative) Thread session mcp_servers into Claude-CLI ACP dispatch (d) — deps: E15-T1 — plan: P25-T4(superseded) — file: crates/roko-acp/src/bridge_events.rs
- [ ] **E15-T5** (focused) Emit readOnlyHint/openWorldHint annotations from tool_spec (e) — deps: none — plan: none — file: crates/roko-mcp-code/src/lib.rs:1545
- [ ] **E15-T6** (mechanical) Neutralize the dead C4 writer so it can't clobber C5's path (f) — deps: E15-T1 — plan: none — file: crates/roko-agent/src/process/mcp.rs

---

## M2 — Surfaces, Hygiene, Pipeline, ACP & Docs/Ops  *(51 tasks · E09, E10, E11, E12, E16, E17, E18)*

### E09 — Observability  *(thread the built MetricRegistry live; bound logs; trim the firehose · 9 tasks)*

- [ ] **E09-T01** (mechanical) Thread MetricRegistry into RunConfig.metrics (THE fix) — deps: (E01) — plan: none — file: crates/roko-cli/src/commands/plan.rs:569
- [ ] **E09-T02** (small) Write runner metrics to .roko/metrics/prometheus.txt post-run — deps: E09-T01 — plan: none — file: crates/roko-cli/src/commands/plan.rs
- [ ] **E09-T03** (mechanical) Thread serve AppState MetricRegistry into serve_runtime RunConfig — deps: E09-T01 — plan: none — file: crates/roko-cli/src/serve_runtime.rs:628
- [ ] **E09-T04** (small) Stop persisting feed_tick/chain_block to events.jsonl — deps: (E01) — plan: none — file: crates/roko-serve/src/... state_hub.rs, dashboard_snapshot.rs
- [ ] **E09-T05** (small) Day-based rotation for roko.log and chain-watcher.log — deps: none — plan: none — file: crates/roko-cli/src/main.rs:2100, roko-serve/src/lib.rs:440
- [ ] **E09-T06** (mechanical) Make ROKO_LOG authoritative across all binaries — deps: none — plan: none — file: apps/roko-chain-watcher/src/main.rs, apps/agent-relay/src/main.rs
- [ ] **E09-T07** (small) GC/cap events.jsonl (size-based or split run-events.jsonl) — deps: E09-T04 — plan: none — file: crates/roko-fs layout + StateHub
- [ ] **E09-T08** (medium) Attach FsObservabilitySinks in runner-v2 tool loop (or delete dir bootstrap) — deps: (E01) — plan: none — file: crates/roko-cli/src/... runner-v2 tool loop, main.rs:3098
- [ ] **E09-T09** (design) Design the v2 telemetry-as-Lens pipeline (design doc, no runtime code) — deps: E09-T01…E09-T08 — plan: none (feeds E13) — file: design doc

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

### E17 — ACP Completion  *(consent-gated, learning-informed, MCP-equipped, honest ACP turns · 6 tasks)*

- [ ] **E17-T01** (integrative) Reply-channel permission gate: emit PermissionRequest, gate exec fail-closed (Fa) — deps: E04, P22 (after P21) — plan: E04+P22(co-owned E04-T12→T14) — file: crates/roko-acp/src/bridge_events.rs, builtin_tools.rs
- [ ] **E17-T02** (integrative) Consult ExperimentStore for ACP prompt/model A/B (Fb) — deps: P19, E07 — plan: P19+E07(prereq) — file: crates/roko-acp/src/bridge_events.rs
- [ ] **E17-T03** (integrative) MCP session-tool parity: thread session_mcp_servers into Anthropic path (Fc) — deps: P25 — plan: P25(prereq; coord E15) — file: crates/roko-acp/src/bridge_events.rs
- [ ] **E17-T04** (focused) Derive tool_context.capabilities from role/session consent, not all-true (Fd) — deps: P22(T1) — plan: P22(supersedes T1) — file: crates/roko-acp/src/bridge_events.rs
- [ ] **E17-T05** (focused) Advertised-vs-accepted capability guard (image/audio) tying P28 + T04 (Fd) — deps: E17-T04, P28 — plan: P28(prereq) — file: crates/roko-acp/src/handler.rs, types.rs
- [ ] **E17-T06** (integrative) End-to-end ACP conformance test (consent/select/experiment/Anthropic MCP) — deps: E17-T01, E17-T02, E17-T03, E17-T04 — plan: none — file: crates/roko-acp/src/bridge_events.rs (tests)

### E18 — Docs, Config, CI & Ops Hygiene  *(make the repo truthful & the pipeline provable · 13 tasks)*

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

---

## M3+ — Long-Horizon v2 Spec-Debt  *(3 tasks · E13)*

### E13 — v2 Spec-Debt  *(load-bearing survivors only; MUST NOT block M0–M2 · 3 tasks)*

- [ ] **E13-T01** (design) Define trait Lens + LensScope in roko-core (no consumers) — deps: E09-T09 — plan: none — file: crates/roko-core/src/obs/lens.rs, obs/mod.rs
- [ ] **E13-T02** (medium) Wrap MetricRegistry as the first CollectorLens feeding StateHub — deps: E13-T01, E09-T01 — plan: none — file: crates/roko-core/src/obs/lens.rs, obs/metrics.rs
- [ ] **E13-T03** (design) Resolve the Cell↔Block↔block naming drift (decision doc, no rename) — deps: (E01 engine decision) — plan: none — file: tmp/status-quo/references/ (decision doc)

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
