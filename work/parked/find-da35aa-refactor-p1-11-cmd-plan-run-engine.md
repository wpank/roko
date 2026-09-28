+++
id = "find-da35aa"
kind = "finding"
title = "[refactor P1-11] cmd_plan_run_engine() 886-line monolith needs 11-stage decomposition"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/commands"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p1-high-blocks-self-hosting-significant-tech-debt"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p1-high-blocks-self-hosting-significant-tech-debt"
anchors = ["crates/roko-cli/src/commands/plan.rs", "cmd_plan_run_engine", "crates/roko-cli/src/graph_execution/plan_runner.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
event_loop.rs is deleted, but cmd_plan_run_engine (commands/plan.rs:2324-3209 at audit time) is the sole real execution entry and performs 11 sequential stages; decomposition plan documented, not implemented. 18+ runner/ helper modules (~31K lines) remain.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p1-high-blocks-self-hosting-significant-tech-debt`
- `tmp/archive/refactoring-audit-2026-09-21/P1-11-EVENT-LOOP-EXTRACTION.md`

How to verify: Measure cmd_plan_run_engine length; check whether the new graph_execution/plan_runner.rs lib entry point extracted its stages.
