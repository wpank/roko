# 96 — TRACE: Runner v2 Execution (deep second pass)

**Verification header**
- Repo: `/Users/will/dev/nunchi/roko/roko`
- Git HEAD: `5852c93c05a4f1bda8ff880fc752d9fba2ba453e` (branch `main`)
- Date: 2026-07-08
- Method: opened every hop's source; every `file:line` below was read, not inferred.
- Scope: a full `roko plan run plans/ --engine runner-v2` execution, CLI parse → provider call → gate → persistence → learning/daimon/dream → merge → snapshot → resume.

**Status tag legend**: **[WIRED]** live on this path · **[PARTIAL]** present but degraded/hardcoded · **[STUB]** placeholder value · **[HOLDOUT]** built elsewhere, not driven here · **[DEAD]** compiled but never called on this path.

> Ground-truth correction up front: the module family docs (37, 92) imply the plan is compiled into a `UnifiedTaskDag` and scheduled by `max_concurrent_tasks`. **Neither is true on the Runner v2 live path.** See §4, §5, and the correction ledger at the end.

---

## 0. Executive trace summary

Runner v2 is a single `tokio::select!` event loop (`runner/event_loop.rs:780`) with six branches, driven by a per-plan finite-state-machine executor (`roko_orchestrator::ParallelExecutor`) that emits **one `SpawnAgent{task:"next"}` action per non-terminal plan per tick**. The runner resolves `"next"` to a concrete ready task **inline** by walking `task_index` (a `HashMap`, not a DAG type) and calling `TaskDef::is_ready_with_plan_deps`. Concurrency is **per-plan** (`max_concurrent_plans = 4`, a hardcoded constant), **not** per-task. Everything else (compose, dispatch, gate, persist, learn) hangs off that loop.

```
CLI (main.rs / commands/plan.rs)
   └─ event_loop::run(plans, run_config, state_hub, cancel)
        loop { tokio::select! {
          B1 agent_rx.recv()   → handle_agent_event → episodes/events, budget, completion/failure
          B2 gate_rx.recv()    → verdicts → signals.jsonl, thresholds, retry/replan/merge/verify
          B3 tick_interval     → executor.tick() → dispatch_action(SpawnAgent|RunGate|RunVerify|Merge…)
          B4 flush_interval    → save_snapshot + agent-pids
          B5 plan_timeout      → handle_plan_timeout
          B6 cancel.cancelled()→ stop_all_agents + final snapshot
        }}
        → shutdown_subsystems → dream consolidation → compact episodes → RunReport
```

---

## 1. CLI parse & engine selection **[WIRED]**

1. `roko plan run plans/ --engine runner-v2` parses into `PlanCmd::Run` — `crates/roko-cli/src/main.rs:1356` (enum `PlanEngine`, `main.rs:1294-1304`).
2. **Drift trap**: `PlanEngine::default()` = `RunnerV2` (`main.rs:1301-1303`) but the clap arg is `#[arg(long, default_value = "graph", …)]` (`commands/plan.rs:1361`, mirrored `main.rs:2699`). The clap literal wins, so a bare `roko plan run plans/` uses **Graph** (dry-run stub), and `--engine runner-v2` is required to get the real path. Confirms doc 95's engine-drift claim.
3. Dispatch handler: `commands/plan.rs:220` (`PlanCmd::Run`). Workdir resolved `plan.rs:236`; plans dir `plan.rs:240`; mandatory `validate_before_run` `plan.rs:248`; `--dry-run` short-circuits to `cmd_plan_dry_run` `plan.rs:253`.
4. Engine branch: `if matches!(engine, PlanEngine::Graph)` → `cmd_plan_run_engine` (`plan.rs:258-266`). Otherwise fall through to the **Runner v2 block** at `plan.rs:269`.

## 2. Runner v2 setup (still in `commands/plan.rs`) **[WIRED]**

5. Exclusive workspace lock: `workspace_lock::acquire_workspace_lock(layout.root())` — `plan.rs:272`.
6. `--fresh` archives `executor.json`, `orchestrator.json`, `run-state.json`, `state-snapshot.json` to `.bak.<ts>` — `plan.rs:274-307`.
7. `prepare_runtime_hooks` + unified config load (`RokoBootstrap`, `BootOpts{require_workspace:true}`) — `plan.rs:309-321`.
8. Preflight: provider-for-default-model (`preflight_provider_for_model` `plan.rs:327`) and gate-tool presence (`preflight_gate_deps` `plan.rs:335`).
9. `max_concurrent_tasks` resolved from `--max-tasks` → `runner.max_concurrent_tasks` → `executor.max_concurrent_tasks` → `4` — `plan.rs:342-356`. **Note (§5): this value never throttles agents; it only sizes the gate semaphore.**
10. Auto-resume seeding: copies an explicit `--resume-plan` path onto `layout.executor_snapshot()` unless `--fresh` — `plan.rs:361-376`.
11. Auto-git-init if no `.git` (agents need git tools) — `plan.rs:384-409`.
12. **Plan discovery/load** (§4): `runner::plan_loader::load_plans(&resolved_plans_dir)` — `plan.rs:411`. Missing-crate scaffolding: `scaffold_missing_crates` — `plan.rs:416`.
13. Phase-0 subsystems built: `CascadeRouter::load_or_new(.roko/learn/cascade-router.json)` `plan.rs:440`; `ExtensionChain` `plan.rs:445`; `ConnectorRegistry`/`FeedRegistry` `plan.rs:448-451`; `Projection` (run UUID) `plan.rs:460-463`; **`FeedbackFacade`** with three sinks — `EpisodeSink` → `episodes.jsonl`, `RoutingObservationSink` → cascade router, `KnowledgeIngestionSink` → knowledge-candidates + `NeuroKnowledgeIngestor(KnowledgeStore::for_workdir)` — `plan.rs:471-489`.
14. `RunConfig` assembled `plan.rs:491-570`: model, timeouts, `max_retries` default **2** (`plan.rs:499`), `max_gate_rung` = `2` unless `gates.skip_tests` (`plan.rs:525-529`), MCP resolution `.roko/mcp.json` → auto-discovery (`plan.rs:505-523`), daimon state (`plan.rs:543`), `dangerously_skip_permissions: true` (`plan.rs:503`), output sink selection (`plan.rs:552-567`).
15. Optional approval TUI thread (`App::new_connected_with_page`, stderr redirected to log) — `plan.rs:575-611`. Ctrl-C → `CancellationToken` — `plan.rs:614-619`.
16. **Entry into the loop**: `runner::event_loop::run(plans, &run_config, &state_hub, cancel)` — `plan.rs:653-654`. JSON report emitted `plan.rs:661+`.

## 3. `event_loop::run` prologue **[WIRED]**

17. `pub async fn run` — `runner/event_loop.rs:237`. Config fallback via `load_config_unified` if `roko_config` absent (`event_loop.rs:253-262`); `HttpEventSink::from_env` (`:264`).
18. `ExecutorConfig` built — `event_loop.rs:271-277`: `max_concurrent_plans = DEFAULT_RUNNER_MAX_CONCURRENT_PLANS` (**= 4**, `roko-core/src/defaults.rs:313`), `max_concurrent_tasks` from config, `max_auto_fix_iterations = max_retries`, `task_timeout_secs`.
19. `PersistPaths::from_workdir` maps every artifact path (`runner/persist.rs:30-83`): `episodes_jsonl`, `efficiency_jsonl`, `events_jsonl`, `run_ledger_jsonl`, state snapshots.
20. `SnapshotWriter::new(4)` (debounced writer) `event_loop.rs:279`; orphaned-agent cleanup `:280`; gate thresholds loaded `:281`.
21. **Strict resume**: `load_state_snapshot` (unified `state-snapshot.json`) → fallback `load_run_state` (legacy `run-state.json`) — `event_loop.rs:306-367`. Fingerprint validation (§14) at `:379`.
22. Per-run **gate semaphore** = `Semaphore::new(gate_concurrency)` — `event_loop.rs:472-473` (this is where `max_concurrent_tasks` actually lands).
23. `load_executor` seeds `executor` + `merge_queue` from snapshot if plan-set matches — `event_loop.rs:479-481`.

## 4. Plan → executor state (NOT a DAG type) **[PARTIAL — misnamed in prior docs]**

24. Plan load: `plan_loader::load_plans` — `runner/plan_loader.rs:99`. Case 1: `dir/tasks.toml` is itself a plan (`:101`); Case 2: scan immediate subdirs for `tasks.toml` (`:111-136`). **Only `tasks.toml` is read; `.md` files are ignored** (module docstring `plan_loader.rs:1`).
25. Each plan is registered as a **flat FSM state**, not a compiled DAG: `OrcPlanState::new(&plan.id)` + `executor.add_plan(orc_state)` — `event_loop.rs:487-490`.
26. Tasks are indexed into a nested `HashMap<plan_id, HashMap<task_id, TaskDef>>` = `task_index` — `event_loop.rs:484-497`. **This is the runner's only "graph"; edges (`depends_on`) are evaluated lazily at spawn time.**
27. **[DEAD on this path]** `runner/task_dag.rs` defines a real `TaskDag`/`PlanDag` with `ready_tasks`, `next_ready_task`, `mark_running`, topo helpers (`task_dag.rs:147-365`), but `event_loop.rs` imports **only** `task_status_is_terminal` from it (`event_loop.rs:62`). The `TaskDag` struct is never instantiated in the live loop.
28. **[HOLDOUT]** `UnifiedTaskDag` exists in `roko-orchestrator/src/dag.rs` and `roko-core/src/task.rs` and is used by the **legacy `orchestrate.rs`** — not by Runner v2. (`grep UnifiedTaskDag` hits: core/task.rs, orchestrate.rs, orchestrator/{lib,dag}.rs, README — **not** `runner/`.)

## 5. Scheduling / concurrency **[PARTIAL — per-plan, not per-task]**

29. Branch 3 fires on `tick_interval` and calls `executor.tick()` — `event_loop.rs:1743-1749`.
30. `ParallelExecutor::tick` — `roko-orchestrator/src/executor/mod.rs:464-501`: iterates `self.queue`, skips terminal/paused plans and unsatisfied cross-plan deps, then `active_count += 1; if active_count > max_concurrent_plans { break }` (`:488-492`), then asks `PlanStateMachine::next_action(state)` (`:495`).
31. `next_action` returns exactly one action per plan based on phase — `executor/state_machine.rs:240-275` (e.g. `Implementing → SpawnAgent{task:"next"}`, `AutoFixing → SpawnAgent{task:"fix"}`, `Reviewing/DocRevision/Enriching → SpawnAgent{…}`).
32. **Consequence**: within one plan, at most one agent runs at a time (further enforced by the `active_agent_tasks`/`agent_handles` guards, `event_loop.rs:4002-4019`). Parallelism = up to **4 plans** concurrently. `--max-tasks`/`max_concurrent_tasks` does **not** run N tasks of one plan in parallel; it only sets `gate_concurrency` (`event_loop.rs:473`) and `ExecutorConfig.max_concurrent_tasks` (which `tick` never reads). Prior docs conflate the two.
33. `"next"`/`"fix"` sentinel → concrete task: SpawnAgent handler walks `task_index`, sorts by `sequence`, picks first not-completed/not-failed/non-terminal task satisfying `is_ready_with_plan_deps(completed, completed_plans)` — `event_loop.rs:3920-3950`.

## 6. Per-task dispatch: compose → route → spawn **[WIRED]**

Dispatch entry: `dispatch_action` `SpawnAgent` arm — `event_loop.rs:3917`.
34. Guards: duplicate-spawn suppression (`:4002-4019`), retry cooldown (`:4021-4029`), **per-plan budget** (`plan_spent >= max_plan_usd` → `Fatal`, `:4034-4057`).
35. **Preflight verify skip**: for retryable/verify tasks, `gate_dispatch::run_gate_once` runs the pipeline first; if it already passes, the agent is skipped and the plan is advanced with a synthetic `GateCompletion` — `event_loop.rs:4089-4208`.
36. **Compose**: knowledge routing advice from neuro store (`build_knowledge_routing_advice`, `event_loop.rs:4230-4236`); daimon hook (`daimon_task_hook`, `:4247`); `RoutingContext` built (`:4249-4273`) — note **`conductor_load: 0.0` hardcoded** (`:4258`), `active_agents: 0`, `ready_queue_depth: 0` are stubs.
37. `DispatchContext` built with gate feedback + dependency outputs — `event_loop.rs:4274-4289`. `dispatcher.plan(task_def, &dispatch_ctx)` assembles the **system+user prompt** and picks a baseline model — `event_loop.rs:4292-4310`. NOTE (corrected): the live runner prompt is assembled by the CLI-side `PromptAssembler` (`dispatch/prompt_builder.rs:717`), **not** the canonical `SystemPromptBuilder`/12-slot/VCG stack — see docs `34` and `103`.
38. **Model modulation** (layered): knowledge-store nudge (`:4315-4341`) → daimon modulation (turn limit + effort + model, `:4344-4370`). Prompt diagnostics/playbook IDs stashed per attempt for later gate-outcome learning — `:4372-4384`.
39. Daimon prompt context + replan context appended to prompt — `event_loop.rs:4388-4419`; full prompt stashed for episodes (`current_prompt_text`, `:4420`). Pre-inference extension hook — `:4422-4432`.
40. **Provider resolve**: `factory.resolve_runtime(&requested_model)` → `ResolvedAgentRuntime::{Cli, Bridge}`, with fallback to default model — `event_loop.rs:4434-4467`. Runtime resolution logic lives in `dispatch_v2.rs` (`ProviderRouter::resolve` `dispatch_v2.rs:653/754`, `create_agent` `:764`).
41. Lifecycle events emitted to `events.jsonl` (`task_attempt_started`, `prompt_assembled`, `agent_dispatch_started`) — `event_loop.rs:4472-4512`.
42. **Spawn — CLI path** `ResolvedAgentRuntime::Cli`: `AgentSpawnConfig::from_run_config` (`:4520`) then `factory.dispatcher().spawn_streaming_cli_agent(&spawn_config, agent_tx)` — `event_loop.rs:4533-4537`. Handle (pid) stored in `agent_handles`; `active_agent_tasks` marked; agent feed registered — `:4539-4571`. Returns `AgentStarted`.
43. **Spawn — Bridge path** `ResolvedAgentRuntime::Bridge` (in-process API providers): `AgentDispatchRequest` built (`CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=2` env) then `factory.spawn_shared_agent_bridge(request, agent_tx)` — `event_loop.rs:4628-4682`.

## 7. Provider call + tool loop → agent events **[WIRED]**

44. The spawned agent streams `--output-format stream-json`; each line becomes an `AgentEvent` on `agent_tx` (parsed in `runner/agent_stream.rs`; forwarding types in `runner/agent_events.rs`). Provider selection + the actual chat/tool loop live under `roko-agent` (dispatcher) invoked through `SharedAgentFactory`.
45. **Branch 1** consumes them: `agent_rx.recv()` — `event_loop.rs:790`. Per event: `handle_agent_event` updates `RunState` + sink (`:796`), `append_agent_event` writes to `events.jsonl` (`:797`, impl `:2536-2554`), `publish_learning_agent_event` fans to the learning bus (`:798`, impl `:2556-2600`), and `ToolCall`/`MessageDelta` mirror to the optional HTTP sink (`:804-823`).
46. On `TurnCompleted`: per-turn budget check (`:829-841`), `agent_completed` lifecycle event (`:843-879`), post-inference extension hook (`:881-898`), then either `handle_agent_failure` (turn error, `:906-915`) or `apply_agent_completion` (`:917`) which drives the executor `ImplementationDone` transition, then `save_snapshot` (`:919`).
47. On `Exited`: reaps pid (`handle.wait()` + `unregister_pid`, `:924-933`), maps exit code → completion/failure — `event_loop.rs:923-996`.

## 8. Gate dispatch → verdict **[WIRED]**

48. When the agent completes, the executor advances the plan to `Gating`; next `tick` yields `ExecutorAction::RunGate{rung}` handled at `event_loop.rs:4686`.
49. Skips: no `Cargo.toml` + no custom rungs → record skipped rung (`:4692-4703`); read-only role → synthetic pass sent via **spawned** task to avoid select-loop deadlock (`:4747-4787`).
50. Otherwise `gate_dispatch::spawn_gate(plan, task, rung, workdir, gates_config, complexity, verify_steps, timeout, gate_tx, gate_sem, target_crates)` — `event_loop.rs:4792-4804`.
51. `spawn_gate` (`runner/gate_dispatch.rs:28`) runs the **roko-gate rung pipeline** then declared verify steps under a timeout: `verdicts = [pipeline.verify(...)] + run_verify_steps(...)` (`gate_dispatch.rs:121-126`), `passed = verdicts.all(passed)` (`:140`), classifies `failure_kind` (`:142`, `classify_failure_kind:410`), builds `GateVerdictSummary` list, and sends a `GateCompletion` on `gate_tx` (`:144-178`). Plan-level verify: `spawn_plan_verify` `:185`.

## 9. Verdict handling: signals, thresholds, retry/replan, merge, verify **[WIRED / PARTIAL]**

**Branch 2** `gate_rx.recv()` — `event_loop.rs:1000`.
52. Per-verdict metrics + sink rendering — `:1015-1090`. Gate-threshold EMA update `update_gate_thresholds` (`:1128-1132`) → thresholds event (`:1133`).
53. Learning bus `GateResult` published (`:1138-1145`).
54. **`signals.jsonl` append**: a `{"kind":"GateVerdict", plan_id, task_id, rung, passed, …}` line to `layout.signals_path()` — `event_loop.rs:1147-1168`. (This is Runner v2's only signal-log write; the canonical noun is **Engram**, but the JSONL still tags `kind:"GateVerdict"`.)
55. `on_gate` extension hook `:1171`. Merge completions → `handle_merge_completion` (`:1173-1189`); plan-verify completions → `handle_plan_verify_completion` (`:1191-1204`); intermediate rung pass → advance to next rung and `continue` (`:1206-1216`).
56. **SectionOutcome + playbook feedback** recorded on terminal verdicts → `section-outcomes.jsonl` (`:1223-1247`) and `PlaybookStore.record_outcome` (`:1254-1272`).
57. **Task pass** (`event_loop.rs:1324`): mark completed, snapshot task output diffs (`git_diff_entries_since_task_start`, `:1336-1350`), run-ledger `task_completed` (`:1355-1376`), `task_attempt_completed` event (`:1380-1394`), daimon outcome (`:1395-1402`), **`commit_task_changes`** commits generated code so downstream tasks can diff (`:1406-1411`), then either advance to next ready task (force phase back to `Implementing`, `:1455-1468`) or run plan verify (`GatePassed`, `:1469-1486`).
58. **Task fail** (`event_loop.rs:1487`): `failure_kind` classified; `can_retry` = `iteration <= retry_budget && failure_kind.is_retryable()` (`:1491-1494`). If retryable → `GateFailed` transition + backoff + **`retry_decision` event** (`:1496-1531`).
59. **[PARTIAL] Gate-failure "replan" is prompt-only**: `build_gate_retry_context(gate_output, agent_output, attempt)` plus `lessons_from_post_gate_reflections` are concatenated and stored via `state.set_replan_context(...)` — `event_loop.rs:1549-1590`. On the next dispatch this text is appended to the user prompt (`take_replan_context`, `:4417-4418`). **No plan/DAG mutation, no task re-generation** — unlike legacy `orchestrate.rs::build_gate_failure_plan_revision`.

## 10. Merge queue **[PARTIAL — in-place, no worktree isolation]**

60. `ExecutorAction::MergeBranch` — `event_loop.rs:4957`: builds `MergeRequest::new(plan, "roko/plan/<id>", files_changed, 0)` (`:4963`), `PlanMerger::submit` (`:4969-4973`).
61. `runner/merge.rs` git backend: checks `git rev-parse --verify <branch>` (`merge.rs:164`); **because agents write directly to the main working tree, the branch is normally absent → "in-place runner mode"** validates the dirty working tree instead of merging (`merge.rs:146-205`), then a post-merge `cargo check` verdict (`:305-327`). Module docstring admits "batch branch handling still lives outside this module" (`merge.rs:8-12`).
62. **[HOLDOUT] Worktree isolation is unwired**: no `git worktree` creation per plan/task anywhere in `runner/`. All plans mutate one shared tree; the "merge" is a cargo-check on that tree.

## 11. Persistence artifacts **[WIRED]**

Every save flows through `save_snapshot` (`event_loop.rs:3341`) or targeted appends. `save_snapshot` serializes the executor snapshot + `OrchestratorSnapshot.with_merge_queue` + `RunStateSnapshot` (incl. `cascade_router_json`, fingerprints) and hands them to `SnapshotWriter` (`event_loop.rs:3350-3392`).

| Step | Artifact | Writer (`file:line`) |
|---|---|---|
| Gate verdict | `.roko/signals.jsonl` | `event_loop.rs:1159-1166` |
| Agent/runner lifecycle | `.roko/events.jsonl` | `append_agent_event event_loop.rs:2551`; `append_runner_event persist.rs:281-282` |
| Episodes | `.roko/episodes.jsonl` | `FeedbackFacade`/`EpisodeSink` (`plan.rs:474`); compaction `event_loop.rs:1961` |
| Efficiency (per-turn) | `.roko/learn/efficiency.jsonl` | learning subscriber `event_loop.rs:764-773` |
| Model routing | `.roko/learn/cascade-router.json` | `load_or_new plan.rs:440`; subscriber persist `event_loop.rs:765` |
| Knowledge candidates + neuro store | `.roko/learn/…candidates…` + `KnowledgeStore` | `KnowledgeIngestionSink`/`NeuroKnowledgeIngestor` `plan.rs:481-488` |
| Prompt-section learning | `.roko/learn/section-outcomes.jsonl` | `persist.rs:612-615`, spawned `event_loop.rs:1242` |
| Unified state snapshot | `.roko/state/state-snapshot.json` | `save_state_snapshot persist.rs:329` |
| Executor snapshot | `.roko/state/executor.json` | `save_executor_snapshot persist.rs:286` |
| Orchestrator snapshot | `.roko/state/orchestrator.json` | `save_orchestrator_snapshot persist.rs:292` |
| Run state | `.roko/state/run-state.json` | `save_run_state persist.rs:309` |
| Run ledger | `.roko/state/run-ledger.jsonl` | `append_ledger_entry event_loop.rs:1804, 1365` |
| Agent PIDs | `.roko/state/agent pids` | `save_agent_pids persist.rs:303`, `event_loop.rs:1830` |

Snapshots are written on: every task turn completion (`:919, :995`), every executor-action dispatch region, `flush_interval` (`:1826`), plan-complete (`:4922`), timeout (`:1857`), cancel (`:1857`), and final flush (`:1935`).

## 12. Learning / efficiency / neuro writeback **[WIRED]**

63. Learning bus + subscriber spawned at loop start: `EventBus::new(256)` (`event_loop.rs:742`), `run_learning_subscriber(rx, health, latency, router, anomaly, costs, efficiency_path, router_persist_path)` (`:766-775`). It owns its **own** `CascadeRouter` (`:754`) that also persists to `cascade-router.json` — **two writers to the same file** (dispatch-time router from `plan.rs:440` + subscriber router). Flagged as a concurrency smell (§ holdouts).
64. Feedback facade fan-out: `emit_runner_event_with_facades` converts terminal runner events to a feedback object (`runner_event_to_feedback`) and fire-and-forgets `facade.on_event` → EpisodeSink/RoutingObservationSink/KnowledgeIngestionSink — `event_loop.rs:2747-2786`.
65. Subscriber drained on shutdown to flush `efficiency.jsonl`: `drop(learning_event_bus); learning_subscriber_handle.await` — `event_loop.rs:1944-1947`.

## 13. Daimon + dream hooks **[WIRED / PARTIAL]**

66. **Daimon**: hook per task (`daimon_task_hook` `:4247`), modulates model/effort/turn-limit (`daimon_dispatch_modulation` `:4344`), records outcome per task (`record_daimon_task_outcome` `:1395, :4607`). Loaded once via `RunConfig::daimon_state_with_strategy` (`plan.rs:543`). **[WIRED]**
67. **Dream**: `dream_completion_pending` is set only when `total_agent_calls > 0` after a plan completes (`event_loop.rs:1318-1323, 1478-1485`); consolidation runs **once at end of run** via `run_dream_consolidation_if_enabled` (`:1952-1953`, impl `:5473-5488`). **[PARTIAL]** — no cron/periodic trigger; end-of-run only, gated on config enable.
68. **Conductor**: **[HOLDOUT]** the 10-watcher `roko-conductor` loop is not driven here. Only vestiges appear: `conductor_load: 0.0` stub in `RoutingContext` (`:4258`) and the `"conductor"→AgentRole::Conductor` string map (`:5145`). No circuit-breaker/watcher tick runs in the event loop.

## 14. Snapshot + resume semantics **[WIRED]**

69. Fingerprints computed once at startup (`state.task_fingerprints`, `event_loop.rs:592-600`) and stored into every `RunStateSnapshot` (`:3387`).
70. On resume, `resume::prepare_resume_with_force` compares current `tasks.toml` fingerprints against the snapshot (`runner/resume.rs:136-233`): drifted completed tasks are reported (`DriftedTask`, `:218`) and re-queued instead of aborting; `--force-resume` skips validation (`:196-201`). JSONL logs (`episodes/events/efficiency`) are truncated to their last valid line by `recover_jsonl` (`persist.rs:376`).
71. Executor + merge-queue restored by `load_executor` only if the snapshot's plan-set matches (`event_loop.rs:479`); `restore_state_from_resume_snapshot` reseeds `RunState` (`:603-616`). Cascade router preferentially restored from the embedded snapshot JSON over the file (`:446-472`).

---

## Data-flow diagram (ASCII)

```
 plans/*/tasks.toml
        │ load_plans (plan_loader.rs:99)
        ▼
   task_index HashMap ─────────────────────────────┐ (NOT a DAG; §4)
        │ add_plan → ParallelExecutor FSM           │
        ▼                                           │
 ┌─────────────────── event_loop::run (780) ───────┼───────────────────────────┐
 │ tokio::select!                                    │                           │
 │  B3 tick → executor.tick() (mod.rs:464)           │  max_concurrent_plans=4   │
 │      └─ SpawnAgent{"next"} ── resolve via task_index (3920) ─┐               │
 │                                                              ▼               │
 │  compose+route: dispatcher.plan (4293) → resolve_runtime (4434)             │
 │      ├─ knowledge nudge (4315)  ├─ daimon modulate (4344)                    │
 │      ▼                                                                       │
 │  spawn_streaming_cli_agent (4536)  |  spawn_shared_agent_bridge (4654)       │
 │      │ stream-json                                                            │
 │  B1 agent_rx (790) ─ handle/append → events.jsonl(2551), learn bus(798)      │
 │      │ TurnCompleted → apply_agent_completion(917) → ImplementationDone       │
 │      ▼                                                                        │
 │  RunGate (4686) → gate_dispatch::spawn_gate (4792) → roko-gate pipeline       │
 │      │ GateCompletion(gate_tx)                                                │
 │  B2 gate_rx (1000) ─ verdicts                                                 │
 │      ├─ signals.jsonl (1159)  ├─ thresholds EMA (1128)  ├─ section/playbook   │
 │      ├─ pass → commit_task_changes(1406) → next task | plan verify           │
 │      └─ fail → retry + set_replan_context (1586, prompt-only §9)              │
 │  MergeBranch (4957) → PlanMerger in-place (merge.rs:146, no worktree §10)     │
 │  B4 flush → save_snapshot (3341) → state/*.json + run-ledger.jsonl            │
 │  B5 timeout / B6 cancel → stop_all_agents + final snapshot                    │
 └───────────────────────────────────────────────────────────────────────────┘
        │ end-of-run
        ├─ learning subscriber flush → efficiency.jsonl (764)
        ├─ dream consolidation (1952, if agent_calls>0)  [PARTIAL]
        ├─ feedback facade → episodes.jsonl + neuro KnowledgeStore
        └─ compact_episodes_if_needed (1961) → RunReport
```

---

## Holdouts vs legacy `orchestrate.rs`

| Capability | Legacy `orchestrate.rs` | Runner v2 live path | Status |
|---|---|---|---|
| Plan compiled to `UnifiedTaskDag` | Yes (`orchestrate.rs` uses it) | No — flat `task_index` + FSM; `runner/task_dag.rs` TaskDag unused | **[PARTIAL]** doc-drift |
| Intra-plan task parallelism | DAG-scheduled | 1 agent/plan; parallelism only across ≤4 plans | **[PARTIAL]** |
| `max_concurrent_tasks` throttles agents | Yes | No — only sizes gate semaphore | **[PARTIAL]** |
| Gate-failure replan | `build_gate_failure_plan_revision` (regenerates tasks) | Prompt-only `set_replan_context` | **[PARTIAL]** |
| Worktree isolation per plan | (intended) | In-place shared tree; merge = cargo-check | **[HOLDOUT]** |
| Conductor 10-watcher loop | referenced | Not driven; `conductor_load:0.0` stub | **[HOLDOUT]** |
| Dream consolidation trigger | — | End-of-run only, no cron | **[PARTIAL]** |
| CascadeRouter single writer | — | Two writers to `cascade-router.json` (dispatch + subscriber) | **[PARTIAL]** risk |
| Knowledge-informed routing | — | Wired (`build_knowledge_routing_advice`) | **[WIRED]** |
| Signal noun | Engram (canonical) | JSONL still tags `kind:"GateVerdict"` | cosmetic drift |

---

## Checklist to close the holdouts

- [ ] Replace flat `task_index` scheduling with an actual DAG so intra-plan tasks parallelize; either wire `runner/task_dag.rs::TaskDag` into `tick`/dispatch or delete it as dead code (`event_loop.rs:62` is its only consumer).
- [ ] Make `max_concurrent_tasks` throttle agents (currently only `gate_concurrency`); decouple from the fixed `max_concurrent_plans = 4` (`defaults.rs:313`) or make it configurable.
- [ ] Upgrade gate-failure replan from prompt-append (`event_loop.rs:1586`) to true task/plan revision (port `build_gate_failure_plan_revision`).
- [ ] Implement per-plan `git worktree` isolation in `runner/merge.rs` so plans don't share one dirty tree; make `MergeBranch` merge a real branch (`merge.rs:146-205`).
- [ ] Drive `roko-conductor` watchers/circuit-breaker from the event loop and feed real `conductor_load`/`active_agents`/`ready_queue_depth` into `RoutingContext` (`event_loop.rs:4258-4260`).
- [ ] Add a periodic dream-consolidation trigger (currently end-of-run only, `event_loop.rs:1952`).
- [ ] Resolve the dual `CascadeRouter` writers to `cascade-router.json` (`plan.rs:440` vs `event_loop.rs:754/765`) — single owner or file lock.
- [ ] Fix the engine-default mismatch: `PlanEngine::default()=RunnerV2` but clap `default_value="graph"` (`main.rs:1301` vs `commands/plan.rs:1361`) — pick one so `roko plan run plans/` uses the live engine.

---

## Corrections to prior status-quo docs

1. **Doc 37 / 92 (Runner-v2 module family)** — the plan is **not** compiled into `UnifiedTaskDag` on the Runner v2 path. It is a flat `task_index: HashMap` + per-plan FSM (`event_loop.rs:484-497`), and `runner/task_dag.rs::TaskDag` is imported for one helper only (`event_loop.rs:62`). `UnifiedTaskDag` is a legacy-`orchestrate.rs`/core type.
2. **Scheduling docs** — "max_concurrent_tasks controls parallel task execution" is wrong for Runner v2: agent concurrency is fixed at `max_concurrent_plans = 4` (`defaults.rs:313`), one agent per plan; `max_concurrent_tasks` only sizes the gate semaphore (`event_loop.rs:473`).
3. **Engine-default docs** — `roko plan run plans/` defaults to **Graph** (clap `default_value="graph"`, `commands/plan.rs:1361`), despite `PlanEngine::default()=RunnerV2`. Runner v2 requires explicit `--engine runner-v2`.
