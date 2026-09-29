+++
id = "gap-fdd27f"
kind = "gap"
title = "Prompt experiments are never assigned on the Graph execution path"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-cli/graph-dispatch", "roko-learn/experiments"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:3539", "crates/roko-cli/src/graph_task_dispatch.rs:4256", "crates/roko-cli/src/graph_task_dispatch.rs:1766", "crates/roko-cli/src/dispatch/prompt_builder.rs:1762"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "prompt_experiment: None" crates/roko-cli/src/graph_task_dispatch.rs'

[[verify]]
command = "! grep -q 'prompt_experiment: None' crates/roko-cli/src/graph_task_dispatch.rs && cargo test -p roko-cli --lib graph_dispatch_prepares_and_settles_prompt_assignment"
+++

Both Graph dispatch paths build their `DispatchContext` with `prompt_experiment: None` (`graph_task_dispatch.rs:1791`, `:3200`).
Settlement code exists (`:1192-1210`), but with no prepared assignment there is nothing to settle, so the crash-durable prompt A/B engine (`roko-learn/src/prompt_experiment.rs`) collects no data from plan runs, the sole production executor.
Fix: prepare an assignment per attempt before dispatch (bucketed by the attempt key), inject the assigned sections, and settle after gates.

2026-09-29: re-verified at d9e79e9d8. Still open; the None sites moved to graph_task_dispatch.rs:3539 (batch) and 4256 (streaming), and settlement is at 1762-1780.
