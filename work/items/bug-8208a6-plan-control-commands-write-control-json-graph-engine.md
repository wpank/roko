+++
id = "bug-8208a6"
kind = "bug"
title = "roko plan cancel/pause/resume/retry write control.json which the Graph engine never reads"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/graph-execution", "roko-graph/engine"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "plan:portal-programme/03b-backend-workspace-server#T16"
discovered_from = "plan:portal-programme/03b-backend-workspace-server#T16"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::control_path", "crates/roko-graph/src/engine.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'grep -rn "control\.json\|ControlFile\|PlanControl" crates/roko-cli/src --include="*.rs" | grep -v target | head -10'

[[verify]]
command = 'grep -rn "control\.json\|read_control\|ControlFile" crates/roko-graph/src --include="*.rs" | grep -q .'
+++

`roko plan cancel`, `roko plan pause`, `roko plan resume` and `roko plan retry` write a control signal to `.roko/state/control.json`. The Graph engine (`crates/roko-graph/src/engine.rs`) does not read this file; it receives cancellation and pause signals only through an in-memory `PlanRunInterruptHandle` / cancellation token that is threaded through `GraphPlanRunParams`.

**Effect.** On a standalone `roko plan run` (no server), the CLI control sub-commands are silently ineffective: `roko plan cancel` writes the file but the running Graph engine ignores it and the plan continues to completion.

On a server-owned run, `roko plan cancel` / `pause` / `resume` / `retry` operate correctly because they call the REST API (`POST /api/plans/{id}/cancel` etc.), which reaches the in-memory interrupt handle. The file write is harmless but misleading.

**Fix path.** Either (a) remove the `control.json` write from the CLI control sub-commands for graph runs, or (b) add a file-watcher side-channel in the standalone plan runner that polls `control.json` and forwards changes to the in-memory interrupt handle. Option (a) is simpler and avoids the polling overhead; the file may be useful for a future persistent-state feature, so the decision is left open.
