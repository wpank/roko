+++
id = "bug-8208a6"
kind = "bug"
title = "roko plan cancel/pause/resume/retry write control.json which the Graph engine never reads"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/graph-execution", "roko-graph/engine"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "plan:portal-programme/03b-backend-workspace-server#T16"
discovered_from = "plan:portal-programme/03b-backend-workspace-server#T16"
anchors = ["crates/roko-cli/src/commands/plan.rs:1226", "crates/roko-cli/src/runner/types.rs::ControlCommand", "crates/roko-cli/src/execution_control.rs::control_command_to_execution", "crates/roko-cli/src/serve_client.rs::cancel_plan_run"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'grep -rn "control\.json\|ControlFile\|PlanControl" crates/roko-cli/src --include="*.rs" | grep -v target | head -10'

[[verify]]
command = "grep -rq 'ControlCommand::poll' crates/roko-cli/src/graph_execution || ! grep -q 'cmd.write(&state_dir)' crates/roko-cli/src/commands/plan.rs"
+++

`roko plan cancel`, `roko plan pause`, `roko plan resume` and `roko plan retry` write a control signal to `.roko/state/control.json`. The Graph engine (`crates/roko-graph/src/engine.rs`) does not read this file; it receives cancellation and pause signals only through an in-memory `PlanRunInterruptHandle` / cancellation token that is threaded through `GraphPlanRunParams`.

**Effect.** On a standalone `roko plan run` (no server), the CLI control sub-commands are silently ineffective: `roko plan cancel` writes the file but the running Graph engine ignores it and the plan continues to completion.

On a server-owned run, `roko plan cancel` / `pause` / `resume` / `retry` operate correctly because they call the REST API (`POST /api/plans/{id}/cancel` etc.), which reaches the in-memory interrupt handle. The file write is harmless but misleading.

**Fix path.** Either (a) remove the `control.json` write from the CLI control sub-commands for graph runs, or (b) add a file-watcher side-channel in the standalone plan runner that polls `control.json` and forwards changes to the in-memory interrupt handle. Option (a) is simpler and avoids the polling overhead; the file may be useful for a future persistent-state feature, so the decision is left open.

Re-verified 2026-09-29 at d9e79e9d8: unchanged. Correction to the body: the `roko plan pause/resume/cancel/retry` handlers (commands/plan.rs:1226-1290) do not route to the REST API when a roko-serve owns the workspace. They always write control.json, so they are ineffective on server-owned runs as well. The anchor plan_runner.rs::control_path does not exist.
