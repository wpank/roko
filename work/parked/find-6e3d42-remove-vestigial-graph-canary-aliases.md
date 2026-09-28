+++
id = "find-6e3d42"
kind = "finding"
title = "Remove Vestigial graph_canary Aliases"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-acp"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/382-graph-canary-alias-cleanup.md#382 — Remove Vestigial graph_canary Aliases"
discovered_from = "audit:tmp/backlog/archive/382-graph-canary-alias-cleanup.md#382 — Remove Vestigial graph_canary Aliases"
anchors = ["crates/roko-acp/src/runner.rs", "crates/roko-cli/src/run.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
dead code, no functional impact. The `graph_canary` feature flag was removed but compatibility aliases remain. `GraphCanary` enum variant exists in acp/runner.rs. A string match `Some("graph_canary") => "graph"` exists in cli/run.rs:196. Test references exist in graph_execution/workflow_caller.rs.

Imported without verification from:
- `tmp/backlog/archive/382-graph-canary-alias-cleanup.md#382 — Remove Vestigial graph_canary Aliases`

How to verify: Check whether the gap described in tmp/backlog/archive/382-graph-canary-alias-cleanup.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]
