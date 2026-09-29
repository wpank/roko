+++
id = "gap-45cf4a"
kind = "gap"
title = "#398 — Automatic Implement-Review-Merge Pipeline for Plan Runner"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/398-auto-review-pipeline.md##398 — Automatic Implement-Review-Merge Pipeline for Plan Runner"
discovered_from = "audit:tmp/backlog/archive/398-auto-review-pipeline.md##398 — Automatic Implement-Review-Merge Pipeline for Plan Runner"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "tasks.toml", "crates/roko-acp/src/runner.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-cli/src/task_parser.rs", "crates/roko-acp/src/runner.rs:1497-1559", "crates/roko-gate/src/acceptance_contract.rs", "crates/roko-cli/tests/graph_dispatch_review_tests.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The plan runner (graph engine, `crates/roko-cli/src/graph_task_dispatch.rs`) does not automatically inject reviewer agents after gates pass. Reviewer tasks must be manually declared in `tasks.toml`, which never happens in practice. All 124 completed plans executed without any review step.

Imported without verification from:
- `tmp/backlog/archive/398-auto-review-pipeline.md##398 — Automatic Implement-Review-Merge Pipeline for Plan Runner`

Some cited files are gone: `crates/roko-cli/tests/graph_dispatch_review_tests.rs`.

How to verify: Check: After all gate rungs pass for a task, the dispatcher auto-classifies the task complexity; Trivial and Simple tasks proceed to success without a reviewer (existing behavior preserved).; Standard tasks spawn one `QuickReviewer` agent using… [evidence: no status line; no index/roll-up evidence]
