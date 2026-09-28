+++
id = "gap-c3add8"
kind = "gap"
title = "[cli-audit plan] `--approval`/`--tui` rejected by Graph engine"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/commands/plan"]
created = 2026-08-31
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/02-plan.md"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/02-plan.md"
anchors = ["crates/roko-cli/src/commands/plan.rs", "backlog #255"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
plan run rejects --approval with "not yet supported by the Graph Engine"; with Graph now the sole engine, approval TUI flow is unavailable. #255 graph approval/control is listed as an implemented prerequisite.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/02-plan.md`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep plan.rs for 'not yet supported by the Graph'; run plan run --approval.
