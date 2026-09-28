+++
id = "gap-573584"
kind = "gap"
title = "DF-0925 P2-4: Verify loop has no fail-fast"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-4. No fail-fast in the verify loop"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-4. No fail-fast in the verify loop"
anchors = ["graph_task_dispatch.rs:2081-2121"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
All verify steps run before deciding, so a failed structural grep still pays a full cargo check.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-4. No fail-fast in the verify loop`

How to verify: Read verify loop control flow.
