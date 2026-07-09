# Roadmap

**Date**: 2026-07-08 · HEAD `5852c93c05` on `main`.

This roadmap is ordered by operational risk, not by how interesting the architecture work is. Every item has a proof gate in [25-PROOF-GATES.md](25-PROOF-GATES.md). Use [27-IMPLEMENTATION-BACKLOG.md](27-IMPLEMENTATION-BACKLOG.md), [28-DEFINITION-OF-DONE.md](28-DEFINITION-OF-DONE.md), and [29-RISK-REGISTER.md](29-RISK-REGISTER.md) as the operational companion docs. The two dominant themes are **engine drift** ([95-ENGINE-DRIFT.md](95-ENGINE-DRIFT.md)) and **security** ([75-SECURITY-AUTH-SCOPE-MATRIX.md](75-SECURITY-AUTH-SCOPE-MATRIX.md)).

## Phase 0: Stop Misleading Execution And Close Exploitable Holes

- [ ] **P0.1 Make default `roko plan run` honest.** Either flip the Clap default back to `runner-v2` (`main.rs:1362`) or make Graph refuse non-dry-run plan execution until `TaskExecutorCell` is live (or register the already-built `AgentCell`). See [95](95-ENGINE-DRIFT.md).
- [ ] **P0.2 Fix `roko resume`.** Route resume to Runner v2 (auto-resumes from snapshot); do not hardcode Graph (`main.rs:2699`) while Graph ignores snapshots.
- [ ] **P0.3 Add a smoke test for the default plan path.** A default run must either dispatch a real agent/gate sequence or clearly fail as unsupported. The test must fail if output contains the `task-output:stub:` marker.
- [ ] **P0.4 Update user-facing docs and help text.** `CLAUDE.md`, `README.md`, `docs/v2/27-ORCHESTRATOR.md`, and CLI examples must not tell users to run a stub path (and must stop naming dead-by-default `orchestrate.rs` as the wired hub).
- [ ] **P0.5 Fix known surface hard breaks.** The 4 frontend 404s (share vs shared, bench/matrix, isfr/stream, ws/agents), camelCase/snake_case event drift, and relay response-shape mismatches must be fixed or intentionally aliased before demo/docs work continues.
- [ ] **P0.6 Authenticate the relay proxy.** Nest `/relay/*` under `/api` or wrap it in `require_api_key`+`require_scope` (`routes/mod.rs:248`); add a 401-without-key test.
- [ ] **P0.7 Deny-by-default the scope fallback.** Replace the `read` fallback (`middleware.rs:385`) with `write`/deny; add a CI test that fails when a mutating route lacks explicit classification.
- [ ] **P0.8 Fix `research search`.** Correct the Perplexity request body (batch → non-batch, HTTP 422) and add a live (non-mock) test so the false-green mock cannot mask a regression.
- [ ] **P0.9 Close (or ratify) the default-provider safety bypass.** The `ToolDispatcher`→`SafetyLayer` 9-policy pre-check runs only on the OpenAI-compat `ToolLoop`; the default Claude-CLI (and Codex) subprocess loop never per-tool safety-gates ([99](99-TRACE-AGENT-TURN.md) §7). Either route Claude/Codex tool calls through a roko-side pre-check or ratify the [BYPASS] as a documented boundary where `build_settings_json` encodes equivalent policy — with an integration test proving the same denial on CLI + `ToolLoop`. See [26](26-CANONICAL-DECISIONS.md) D16.

## Phase 1: Declare The Runtime Contract

- [ ] **P1.1 Choose the strategic engine.** Document whether Runner v2 remains the production plan engine while Graph matures, or Graph becomes production only after parity.
- [ ] **P1.2 Consolidate execution result types.** Normalize `DispatchPlan`, `RunnerDispatchPlan`, `RunLedger`, `GateStatus`, `CommitOutcome`, `RoutingContext`, and dispatch resolver concepts into one foundation contract or a documented split.
- [ ] **P1.3 Normalize state files.** Converge signals↔engrams (gate verdicts write `signals.jsonl`; dashboards read `engrams.jsonl` → empty panels); make serve read the canonical `state-snapshot.json` (it still tries `state/executor.json` → error); **cap/rotate the 44 MB / 97% `feed_tick` `events.jsonl` firehose — it is write-only, nothing reads it back (`FeedTick` apply is a no-op; bootstrap reads `state/events.json`, [97](97-TRACE-SERVE-LIFECYCLE.md))**, so either stop persisting no-op `DashboardEvent`s or hydrate feed panels from the snapshot on reconnect; pick canonical paths for episodes (root/learn/memory triplication), run ledgers, gate thresholds, Daimon state, and knowledge candidates. See [60](60-STATE-PERSISTENCE-LEDGER.md).
- [ ] **P1.4 Wire StateHub and EventBus semantics.** Decide which event type is canonical and which bridges are compatibility layers.
- [ ] **P1.5 Convert the newest tmp feedback into a tracked issue ledger.** Reverify `tmp/tmp-feedback/2` and move still-open items into `.roko/GAPS.md` or a first-class issue file.
- [ ] **P1.6 Fix crate boundary inversions.** Remove or invert `roko-runtime -> roko-gate`, quarantine stale core StateHub/PulseBus copies, and regenerate stale command/crate inventory.
- [ ] **P1.7 Create a source-doc manifest.** Every file in `docs/v1`, `docs/v2`, `docs/v2-depth`, v2-depth research prompts, and v1 references needs a status tag, owner, and status-pack source.
- [ ] **P1.8 Convert the active plan queue into tracked issues.** Reconcile the 29 executable plans / 120 ready tasks with `24`, `25`, and `.roko/GAPS.md`.
- [ ] **P1.9 Rewrite maintained root docs.** Rewrite `README.md`, `CLAUDE.md`, v2 CLI/execution/integration docs, and deploy docs from `81`/`82`.

## Phase 2: Close Live Runtime Gaps

- [ ] **P2.1 Runner v2 parity cleanup.** Port or explicitly drop the confirmed holdouts — conductor supervision loop (`conductor_load` hardcoded 0.0, `event_loop.rs:4258`), agent-driven gate-failure replan (currently prompt-enrichment only via `set_replan_context`, no `tasks.toml` rewrite), and worktree isolation (built in roko-orchestrator, unwired in runner — agents share one dirty tree, "merge"=cargo-check) — plus gate oracles, dreams triggers, and custody/attestation. See [92](92-RUNNER-V2-MODULE-FAMILY.md), [96](96-TRACE-RUNNER-V2-EXECUTION.md). Note: daimon/dreams/learning/efficiency/neuro/gate-dispatch already fire in `event_loop.rs` on runner-v2 (dead on the Graph default).
- [ ] **P2.1a Give Runner v2 a real task DAG (or ratify per-plan).** The live scheduler is a flat `task_index` HashMap + per-plan FSM; concurrency is per-plan (`max_concurrent_plans=4`, one agent per plan) and `max_concurrent_tasks` only sizes the gate semaphore ([96](96-TRACE-RUNNER-V2-EXECUTION.md)). Either wire `runner/task_dag.rs::TaskDag` into `tick`/dispatch so ready tasks within a plan parallelize, or delete `task_dag.rs`/`UnifiedTaskDag` as dead and document the per-plan model as intended.
- [ ] **P2.1b Persist CascadeRouter LinUCB state on the live path.** Resolve the dual writers to `cascade-router.json` (dispatch `plan.rs:440` + learning subscriber `event_loop.rs:754`) to a single owner/file-lock and make the learned arm state survive a restart (currently resets toward identity).
- [ ] **P2.2 Graph Engine minimum parity.** Live `TaskExecutorCell`, graph snapshots/resume, real gate nodes or gate-pipeline cell, conditional edges, budget enforcement, workspace locks, events, and episode/signal persistence.
- [ ] **P2.3 Gate correctness — move adaptivity onto the live path.** The live runner gate path uses `RungExecutionInputs::default()` and never calls `enrich_rung_config`; adaptive thresholds (SPC/CUSUM/EWMA/Hotelling), oracles 4-6, ratchet, and `VerdictPublisher` live only on the dead `orchestrate.rs` `PlanRunner` ([101](101-TRACE-GATE-PIPELINE.md)). Port enrichment into `gate_dispatch::run_gate_once`; make stubs report `Skipped`/`NotWired` and exclude them from the EMA (today rungs 3-6 stub-pass `Verdict::pass`); label verdicts with the real inner rung (today EMA only updates rung 2); call `GateThresholds::save` so `.roko/learn/gate-thresholds.json` is actually written; unify the three rung dialects and wire verdict pulses. See [26](26-CANONICAL-DECISIONS.md) D17.
- [ ] **P2.4 Learning correctness.** Collapse episode roots, preserve model-choice source fidelity, stop zeroing plan totals, and make feedback loss/backpressure explicit.
- [ ] **P2.5 ACP/editor hardening.** Truthful capabilities, permission enforcement for builtin tools, durable session lifecycle, MCP env support, image capability consistency, and no dropped JSON-RPC requests while a prompt is active.
- [ ] **P2.6 Server and surface cleanup.** Fix auth scope matrix, lossy event bridges, WS replay/filter semantics, in-memory-only registries, stale API docs, and the 4 frontend 404s. Note the SPA is **embedded-served** by `roko serve` via `rust-embed` (`routes/mod.rs:250`, not standalone), so a `demo/demo-app` `npm run build` must precede `cargo build -p roko-serve` to avoid a stale embedded `dist/`; retire the **two parallel SSE managers** (DataHub camelCase + legacy EventStreamManager snake_case both open `/api/events`) and finish the DataHub migration (43 files still on deprecated hooks). See [105](105-FRONTEND-DEMO-APP.md).
- [ ] **P2.6a Converge to one prompt-assembly surface.** Runner-v2 builds prompts with the CLI-side `PromptAssembler` (`dispatch/prompt_builder.rs:717`), not `SystemPromptBuilder`/12-slot/VCG; two assembly surfaces coexist ([103](103-DUPLICATE-TYPES-CENSUS.md) row 12). Either route the live path through the canonical builder (and warm VCG bidders) or declare the CLI assembler canonical and retire the compose-template path. See [26](26-CANONICAL-DECISIONS.md) D15.
- [ ] **P2.7 CI/release proof.** Add deny, frontend, Foundry, deterministic runtime smoke, Docker health, feature matrix, and release preflight gates before advertising migration completion.
- [ ] **P2.8 Schema and event contract proof.** Build a schema registry for event JSONL, SSE/WS, snapshots, OpenAPI DTOs, and TypeScript fixtures.
- [ ] **P2.9 Security trust-boundary hardening.** Beyond the Phase 0 relay/scope P0s: wire the built-but-uncalled ACP `request_permission` gate before write/edit/bash (`builtin_tools.rs:291`), promote SecretLeak/PathEscape post-checks from `Warn` to `Block` (`safety/mod.rs:767`), replace the false-assurance custody verify with the real hash-chained check (`custody.rs:206`), redact secrets in `config show --effective` (`config_cmd.rs:222`), add an auth header to the worker callback, scope terminal/workspace operations, and document MCP code-execution boundaries. See [75](75-SECURITY-AUTH-SCOPE-MATRIX.md).
- [ ] **P2.10 Deployment proof repair.** Fix root `roko.toml`/Docker/Railway assumptions, compose flags, Fly config drift, image boot checks, and release artifact policy.
- [ ] **P2.11 Env/config manifest enforcement.** Make direct env reads fail docs CI unless `83` or a generated successor owns them.

## Phase 3: Migrate To The New Paradigms

- [ ] **P3.1 Cell/Graph convergence.** Reconcile the Engram-based `Cell` trait with NodeOutput-style agent/compose cells, then replace `NoopCell`/`PassthroughCell` defaults with real cells.
- [ ] **P3.2 Bus/Pulse convergence.** Make `PulseBus`, runtime `EventBus`, StateHub, server event bus, learn event bus, and dashboard projections a deliberate layered model.
- [ ] **P3.3 Store/Signal naming convergence.** Decide whether `Signal` or `Engram` is the public noun and make docs/code aliases point one way.
- [ ] **P3.4 Knowledge/dreams/daimon loops.** Decide HDC on/off, wire reinforcement into retrieval/routing, consume dream routing advice, reconcile Daimon state paths, and remove stale `.roko/memory` duplicates.
- [ ] **P3.5 Tool/provider convergence.** Fix the tool-alias casing bug that strips ALL tools on non-Claude providers (`openai_compat.rs:252,348`, PascalCase vs snake_case — breaks research analyze/enhance/prd on OpenAI/Gemini/Ollama), then make the shared dispatcher and safety/tool metrics (37 builtin tools) apply consistently across Claude CLI, Codex, Cursor, OpenAI-compatible providers, MCP, and roko-std tools.
- [ ] **P3.6 Chain/ISFR/job convergence.** Decide one authority for identity, reputation, job settlement, ISFR rates, and contract addresses; wire chain tools into normal agent workflows.

## Phase 4: Delete Or Quarantine Old Layers

- [ ] **P4.1 Feature-gate or remove dead orchestrator modules.** `worktree`, `mesh_relay`, old safety copies, event log, post-merge, repair, and progress modules need consumers or quarantine.
- [ ] **P4.2 Remove stale static/demo surfaces.** Move `demo/demo-web` and `tmp/demo-uis/*` to archive unless they are intentionally supported demos.
- [ ] **P4.3 Clean `.roko/` residue.** Remove stale worktrees, backup snapshot accumulation, tmp cascade-router files, duplicate memory roots, and empty dormant dirs.
- [ ] **P4.4 Archive old tmp scratch.** Keep authoritative tmp sources and sample fixtures; move scratch folders to `tmp/archive/`.
- [ ] **P4.5 Run the docs convergence plan.** Use `tmp/doc-convergence` or a descendant to reconcile docs/v1, docs/v2, docs/v2-depth, tmp designs, and code into a canonical spec.

## Do Not Start Yet

- Do not add more LLM providers until the shared dispatcher/tool/safety path is consistent.
- Do not expand Graph features before the default execution truth problem is fixed.
- Do not build new chain markets before the existing local jobs, chain registry, ISFR, and deploy paths have one authority.
- Do not rewrite the whole runtime from scratch. Port behavior behind proof gates.

## Milestones

| Milestone | Done when |
|---|---|
| Execution honest | Default `roko plan run` either does real work or refuses with an explicit unsupported message. |
| Perimeter closed | Relay proxy is authed, scope fallback is deny-by-default, ACP permission gate is called, and post-checks block secret leaks. |
| Resume works | `roko resume` uses a snapshot-capable engine and skips completed tasks. |
| One runtime contract | Result/status/routing types have one canonical layer and compatibility adapters. |
| One event/state story | StateHub, events, episodes, signals, and run ledger agree on source of truth. |
| Surface contracts enforced | Frontend/API/relay route manifests pass and known hard breaks are fixed or intentionally compatible. |
| Graph parity | `--engine graph` dispatches agents, gates results, persists outputs, emits events, and resumes. |
| Docs trustworthy | Root docs and v2 docs no longer contradict the status-quo pack on engines, safety, routes, gates, ACP, or chain. |
