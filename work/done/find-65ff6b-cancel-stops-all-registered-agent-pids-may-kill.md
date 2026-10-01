+++
id = "find-65ff6b"
kind = "finding"
title = "Cancel stops all registered agent PIDs — may kill in-flight generation, revision or chat agents"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-serve/plans", "roko-runtime/process-supervisor"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "plan:portal-programme/03b-backend-workspace-server#T16"
discovered_from = "plan:portal-programme/03b-backend-workspace-server#T16"
anchors = ["crates/roko-serve/src/routes/plans.rs::cancel_plan", "crates/roko-cli/src/graph_execution/plan_runner.rs::live_agent_process_trees", "crates/roko-cli/src/graph_execution/plan_runner.rs::terminate_in_flight_agents", "crates/roko-agent/src/process/registry.rs::registered_pids"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'roko_agent::process::registered_pids()' crates/roko-cli/src/graph_execution/plan_runner.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:31Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:46Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`POST /api/plans/{id}/cancel` fires the plan's `PlanRunInterruptHandle`, which calls the interrupt's stop helpers. Those helpers signal every agent PID registered in the server's `ProcessSupervisor` (`roko-runtime/src/process_supervisor.rs::stop_all`). The cancel was verified to stop the plan's own agent process in the WORKSPACE-SERVER-CHECK ("PASS cancel stops the agent process").

**The question.** Do generation, revision or chat agents — which run concurrently with a plan execution in the server — also register their PIDs with the same `ProcessSupervisor`? If they do, a `cancel` of the plan run would kill them too. If they use a separate supervisor instance or no supervisor, they are unaffected.

**Investigation needed.** Check:
1. `POST /api/plans/generate` → which supervisor instance does the spawned generation agent register with?
2. `POST /api/plans/{id}/revise` → same question.
3. `POST /api/chat` (if present) → same question.

If all three share the server's single `ProcessSupervisor`, a cancel of any plan run is a server-wide SIGTERM for every running agent. The fix is to scope the interrupt handle and stop helpers to the run's own PIDs only, using a per-run supervisor or a PID set threaded through `GraphPlanRunParams`.

**Known behaviour (from REVIEW.md fix 1):** The `reqwest::blocking` panic fix in this plan changed `WorkspaceServerClient` to async; the blocking client was replaced with an async one. This is unrelated but confirms that agent spawning paths are async-clean.

Checked 2026-09-29: the answer to the question is yes. The stop helpers are not in a ProcessSupervisor (crates/roko-runtime/src/process_supervisor.rs does not exist); they are terminate_in_flight_agents and kill_in_flight_agents in crates/roko-cli/src/graph_execution/plan_runner.rs, which signal live_agent_process_trees(): every PID in the process-global roko_agent::process::registry (registered_pids, crates/roko-agent/src/process/registry.rs:242) that descends from the current process. Under roko serve, plan generation (serve_runtime.rs:223 -> prd::generate_plan_from_prd_isolated) and revision (serve_runtime.rs:698 -> plan_authoring::revise_plan_source) spawn agents in the same server process, and ClaudeCliAgent (claude_cli_agent.rs:879), exec.rs:519, cursor_cli_agent.rs:280 and child_process_runner.rs:183 all call register_spawned_pid, so cancelling a plan run signals them too. Fix: scope the stop helpers to PIDs registered by the run (a per-run PID set threaded through GraphPlanRunParams) instead of the global registry. The existing verify greps a file that does not exist and always passes.

## Notes

2026-10-01 (wk-runstate): implemented on work/find-8872ad; cargo verification deferred to the batch check. The roko-agent PID registry now tags each child with the spawn scope of the thread that registered it. The new API is `enter_spawn_scope`, `new_spawn_scope`, `current_spawn_scope` and `registered_pids_in_scope`; scopes are kept in memory only, and the on-disk records are unchanged. `run_plan_on_local_runtime` (serve) enters a fresh scope on the thread that hosts the run's current-thread runtime, so every agent the run spawns is tagged with it. The plan runner's `live_agent_process_trees` signals only the PIDs in the current thread's scope. A cancelled server run therefore leaves generation, revision and chat agents alone: they register outside any scope, on the server's own threads. A CLI run has no scope and still owns every unscoped agent. Test: `spawn_scopes_keep_agents_apart` (registry.rs). Limit: an agent spawned on a thread other than the run's, such as inside `spawn_blocking`, would register unscoped and be missed by the run's sweep; none of today's spawn sites do that, and dropping the attempt still kills its tree (`KillTreeOnDrop`).
