+++
id = "find-65ff6b"
kind = "finding"
title = "Cancel stops all registered agent PIDs — may kill in-flight generation, revision or chat agents"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/plans", "roko-runtime/process-supervisor"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "plan:portal-programme/03b-backend-workspace-server#T16"
discovered_from = "plan:portal-programme/03b-backend-workspace-server#T16"
anchors = ["crates/roko-serve/src/routes/plans.rs::cancel_plan_endpoint", "crates/roko-runtime/src/process_supervisor.rs::stop_all"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -n "stop_all\|registered_pids\|register_pid\|agent_pid" crates/roko-runtime/src/process_supervisor.rs | head -10'
+++

`POST /api/plans/{id}/cancel` fires the plan's `PlanRunInterruptHandle`, which calls the interrupt's stop helpers. Those helpers signal every agent PID registered in the server's `ProcessSupervisor` (`roko-runtime/src/process_supervisor.rs::stop_all`). The cancel was verified to stop the plan's own agent process in the WORKSPACE-SERVER-CHECK ("PASS cancel stops the agent process").

**The question.** Do generation, revision or chat agents — which run concurrently with a plan execution in the server — also register their PIDs with the same `ProcessSupervisor`? If they do, a `cancel` of the plan run would kill them too. If they use a separate supervisor instance or no supervisor, they are unaffected.

**Investigation needed.** Check:
1. `POST /api/plans/generate` → which supervisor instance does the spawned generation agent register with?
2. `POST /api/plans/{id}/revise` → same question.
3. `POST /api/chat` (if present) → same question.

If all three share the server's single `ProcessSupervisor`, a cancel of any plan run is a server-wide SIGTERM for every running agent. The fix is to scope the interrupt handle and stop helpers to the run's own PIDs only, using a per-run supervisor or a PID set threaded through `GraphPlanRunParams`.

**Known behaviour (from REVIEW.md fix 1):** The `reqwest::blocking` panic fix in this plan changed `WorkspaceServerClient` to async; the blocking client was replaced with an async one. This is unrelated but confirms that agent spawning paths are async-clean.
