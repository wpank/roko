+++
id = "gap-9e09f6"
kind = "gap"
title = "Plan Detail Enrichment"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/238-plan-detail-enrichment.md#238 — Plan Detail Enrichment"
discovered_from = "audit:tmp/backlog/archive/238-plan-detail-enrichment.md#238 — Plan Detail Enrichment"
anchors = ["crates/roko-cli/src/tui/state.rs", "crates/roko-cli/src/tui/views/plans_view.rs", "crates/roko-cli/src/tui/modals/plan_detail.rs", "state.rs:526", "modals/plan_detail.rs:13", "state.rs:2700-2711", "runner/state.rs", "dashboard_snapshot.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The plan detail modal and F2 task list show only task ID, name, status, and agent_id; dependency chains, acceptance criteria, verification status, modified files, branch, worktree, and commit are all absent despite being available in the underlying data.. The `TaskEntry` bridge type at…

Imported without verification from:
- `tmp/backlog/archive/238-plan-detail-enrichment.md#238 — Plan Detail Enrichment`

Some cited files are gone: `crates/roko-cli/src/tui/state.rs`.

How to verify: Check: The plan detail modal shows dependency arrows between tasks.; Acceptance criteria are displayed as checkboxes below each task.; Branch name, worktree path, and last commit hash are visible in the plan header. [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 4 |]
