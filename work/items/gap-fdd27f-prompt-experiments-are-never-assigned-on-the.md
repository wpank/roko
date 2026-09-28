+++
id = "gap-fdd27f"
kind = "gap"
title = "Prompt experiments are never assigned on the Graph execution path"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/graph-dispatch", "roko-learn/experiments"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:1791", "crates/roko-cli/src/graph_task_dispatch.rs:3200"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "prompt_experiment: None" crates/roko-cli/src/graph_task_dispatch.rs'
+++

Both Graph dispatch paths build their `DispatchContext` with `prompt_experiment: None` (`graph_task_dispatch.rs:1791`, `:3200`).
Settlement code exists (`:1192-1210`), but with no prepared assignment there is nothing to settle, so the crash-durable prompt A/B engine (`roko-learn/src/prompt_experiment.rs`) collects no data from plan runs, the sole production executor.
Fix: prepare an assignment per attempt before dispatch (bucketed by the attempt key), inject the assigned sections, and settle after gates.
