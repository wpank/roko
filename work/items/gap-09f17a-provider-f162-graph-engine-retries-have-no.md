+++
id = "gap-09f17a"
kind = "gap"
title = "[provider F162] Graph engine retries have no backoff between attempts"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph/cells"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F162"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F162"
anchors = ["crates/roko-graph/src/cells/task_executor.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`TaskExecutorCell` retries on any error without delay. Consecutive retry attempts fire immediately, creating a retry storm against transient provider errors.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F162`
- `tmp/archive/provider-audit/20-graph-integration.md`

How to verify: Check roko-graph retry loop for backoff. Confirm in crates/roko-graph/src/cells/task_executor.rs whether still true: Graph engine retries have no backoff between attempts
