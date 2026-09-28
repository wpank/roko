+++
id = "gap-05ee6d"
kind = "gap"
title = "DA-11: Verification dedup cannot see equivalence behind shell wrappers"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/gate-dispatch"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/dev-audit/11-implementation-status.md#Explicit residuals"
discovered_from = "audit:tmp/dev-audit/11-implementation-status.md#Explicit residuals"
anchors = ["runner/gate_dispatch.rs", "crates/roko-cli/src/graph_execution/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Duplicate-verification removal is exact-match and FAST-only; equivalent checks hidden behind arbitrary shell wrappers are not deduplicated (explicit residual), so gates can still compile the same target twice.

Imported without verification from:
- `tmp/dev-audit/11-implementation-status.md#Explicit residuals`
- `tmp/dev-audit/10-p0-implementation.md#Still deferred`
- `tmp/dev-audit/03-verification-policy.md#One semantic gate`

How to verify: Inspect gate dedup in the Graph gate path for wrapper-equivalence handling.
