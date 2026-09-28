+++
id = "gap-6ca8fb"
kind = "gap"
title = "[status-quo #138/#284] Process-boundary crash/resume harness CR01–CR08 missing"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-10
updated = 2026-09-28
source = "tmp/archive/status-quo-audit-2026-09-21/11-crash-resume-analysis.md#8. Gaps relative to #138's prescribed harness"
discovered_from = "audit:tmp/archive/status-quo-audit-2026-09-21/11-crash-resume-analysis.md#8. Gaps relative to #138's prescribed harness"
anchors = ["crates/roko-cli/tests/graph_crash_resume.rs", "ActivityReplayer", "GraphCostLedgerCheckpoint", "backlog #138 #284 #259"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Contract mechanisms are sound but #138's CR01-CR08 kill-point harness (tests/graph_crash_resume.rs) does not exist; blocked on #284 fixture directory/kill points. Also missing crash-resume-matrix.json and #258 ReplayOnly cross-check.

Imported without verification from:
- `tmp/archive/status-quo-audit-2026-09-21/11-crash-resume-analysis.md#8. Gaps relative to #138's prescribed harness`
- `tmp/archive/status-quo-audit-2026-09-21/11-crash-resume-analysis.md#10. Recommendations`

Warning: every file this item cites is gone (`crates/roko-cli/tests/graph_crash_resume.rs`) — likely obsolete or moved.

How to verify: Check for tests/graph_crash_resume.rs and #284 fixtures.
