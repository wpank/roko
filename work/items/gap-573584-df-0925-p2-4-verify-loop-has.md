+++
id = "gap-573584"
kind = "gap"
title = "DF-0925 P2-4: Verify loop has no fail-fast"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-4. No fail-fast in the verify loop"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-4. No fail-fast in the verify loop"
anchors = ["graph_task_dispatch.rs:2081-2121", "crates/roko-cli/src/graph_task_dispatch.rs:1746"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
commit = "725f21e05"
by = "triage check 2026-09-28"
evidence = "Fixed in 725f21e05: the Graph verify loop now fails fast. Once a step has failed (or the attempt is declared doomed), each later step is recorded as skipped and `continue`d without running (graph_task_dispatch.rs:1746-1752, skipped_steps at :1740), so a failed structural grep no longer pays for a following cargo check. (Static check against 3d0ee4d02; tests not re-run.)"
+++
All verify steps run before deciding, so a failed structural grep still pays a full cargo check.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-4. No fail-fast in the verify loop`

How to verify: Read verify loop control flow.

Fixed in 725f21e05 (checked 2026-09-28).
