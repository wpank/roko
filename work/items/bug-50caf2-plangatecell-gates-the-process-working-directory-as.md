+++
id = "bug-50caf2"
kind = "bug"
title = "PlanGateCell gates the process working directory as attempt 0"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
subsystem = ["roko-graph/cells", "roko-gate"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-graph/src/cells/plan_gate.rs:86", "crates/roko-graph/src/cells/plan_gate.rs:150"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "std::env::current_dir()" crates/roko-graph/src/cells/plan_gate.rs'

[[verify]]
command = 'cargo test -p roko-graph plan_gate'
+++

`PlanGateCell` builds its gate request with `attempt_id: 0` (`cells/plan_gate.rs:86`) and runs gates in `std::env::current_dir()` (`:150`) instead of the attempt's worktree.
In rich-topology plans the gate therefore verifies whatever tree the process started in, not the agent's changes; per a local audit, a run where every rung is skipped counts as a pass.
Fix: take worktree and attempt id from the executor's output (fail closed when absent) and send attempt id plus plan/run context with every gate request.
