+++
id = "find-734246"
kind = "finding"
title = "[status-quo P2-A] .unwrap() cleanup (8,342 calls, rising)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["workspace"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/archive/status-quo-audit-2026-09-21/09-completion-tracking.md#P2 items: 8/10 done"
discovered_from = "audit:tmp/archive/status-quo-audit-2026-09-21/09-completion-tracking.md#P2 items: 8/10 done"
anchors = ["grep -rn '\\.unwrap()' crates/ --include='*.rs'"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
unwrap count grew from 3,101 (roko-cli at audit) to 8,342 workspace-wide (7,623 excl. test dirs) by 2026-09-14 as E23-E48 code landed. Incremental cleanup deferred.

Imported without verification from:
- `tmp/archive/status-quo-audit-2026-09-21/09-completion-tracking.md#P2 items: 8/10 done`
- `tmp/archive/status-quo-audit-2026-09-21/07-tech-debt.md#TD-03: 3,101 `.unwrap()` calls in roko-cli (Severity: Medium-High)`

How to verify: Count non-test .unwrap() on production paths; prioritize serve/runtime/graph_execution.
