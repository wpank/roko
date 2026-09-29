+++
id = "gap-4835e7"
kind = "gap"
title = "[git G04] Merge conflicts fail closed with no conflict-aware replan"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/merge"]
created = 2026-09-05
updated = 2026-09-28
source = "tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-cli/src/runner/merge.rs", "crates/roko-cli/src/runner/event_loop.rs", "fail_with_conflicts", "handle_merge_completion", "maybe_apply_gate_failure_plan_revision"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Fix: MergeBackendOutcome::fail_with_conflicts carries conflicted paths; handle_merge_completion triggers maybe_apply_gate_failure_plan_revision with conflict context when replan_on_gate_failure is enabled. BACKLOG.md still showed G04 OPEN on 2026-09-04. Register marks DONE (2026-09-04/05), but th...

Imported without verification from:
- `tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register`
- `tmp/archive/git-audit/impl-G04-conflict-replan.md`

A source claims this was fixed; confirm against current code before closing.

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check the Graph engine merge-completion path consumes conflicted_paths and triggers a conflict-aware plan revision.
