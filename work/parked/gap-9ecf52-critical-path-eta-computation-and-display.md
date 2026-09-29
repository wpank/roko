+++
id = "gap-9ecf52"
kind = "gap"
title = "Critical Path ETA Computation and Display"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/196-critical-path-eta.md#196 — Critical Path ETA Computation and Display"
discovered_from = "audit:tmp/backlog/archive/196-critical-path-eta.md#196 — Critical Path ETA Computation and Display"
anchors = ["task_dag.rs", "crates/roko-cli/src/tui/", "crates/roko-cli/src/runner/task_dag.rs", "crates/roko-cli/src/task_parser.rs", "crates/roko-cli/src/tui/header.rs", "crates/roko-cli/src/commands/status.rs", "TaskDef::depends_on", "TaskDef", "crates/roko-cli/src/tui/state.rs", "views/dashboard_view.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
operators have no visibility into expected plan completion time; they must manually trace dependencies to estimate remaining work. The plan DAG in `crates/roko-cli/src/runner/task_dag.rs` tracks per-plan task dependencies, running tasks, skipped tasks, and retry state. The runner knows which tasks…

Imported without verification from:
- `tmp/backlog/archive/196-critical-path-eta.md#196 — Critical Path ETA Computation and Display`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#UXP-02 (UX/TUI Parity: Partial Items (13 items f)`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#2.6 Display critical path ETA`

Some cited files are gone: `crates/roko-cli/src/tui/header.rs`, `crates/roko-cli/src/tui/state.rs`.

How to verify: Check: `critical_path()` returns the correct longest dependency chain for a given plan DAG; ETA updates dynamically as tasks complete; TUI header shows "ETA: ~Xm" (or "ETA: done" when all tasks are complete) [evidence: CONSOLIDATED UXP-02: Computation+field+display exist; field never written; 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): XS | 7 |]
