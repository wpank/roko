+++
id = "bug-7c8a57"
kind = "bug"
title = "100-line cap on read_files context injection"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
subsystem = ["roko-cli/task_parser"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-06: Remove 100-line file injection cap"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-06: Remove 100-line file injection cap"
anchors = ["crates/roko-cli/src/task_parser.rs:565"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'lines().take(100)' crates/roko-cli/src/task_parser.rs"
+++
read_files entries without a line range are truncated with .take(100), so agents see only imports/declarations. Replace with a configurable byte budget (e.g. 64KB). Backlog #407.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-06: Remove 100-line file injection cap`
- `tmp/archive/plan-audit-2026-09-23/15-file-context-injection.md`
- `tmp/backlog/archive/407-file-context-injection-cap.md#407 — Remove 100-line cap on `read_files` context injection`

How to verify: grep -n 'take(100)' in task_parser.rs and roko-neuro context.rs. / Check: Files listed in `read_files` without a `lines` range are injected in full up to a; When the budget is exceeded, the injected content ends with a marker of the form; The budget is configurable under `[context]` in `roko.toml` as… [evidence: no status line; no index/roll-up evidence]

Merged 2 mined candidates: m3-114, m1-130.

Verified 2026-09-28: crates/roko-cli/src/task_parser.rs:565 still truncates read_files entries without a line range via `content.lines().take(100)`; the crates/roko-neuro/src/context.rs anchor no longer contains a take(100) cap.
