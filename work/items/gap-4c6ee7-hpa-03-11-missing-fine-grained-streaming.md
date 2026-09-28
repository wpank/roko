+++
id = "gap-4c6ee7"
kind = "gap"
title = "HPA-03 §11: Missing fine-grained streaming signals for web UX"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/dashboard"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/03-roko-streaming-architecture.md#What's Missing for World-Class UX"
discovered_from = "audit:tmp/hermes-product-audit/03-roko-streaming-architecture.md#What's Missing for World-Class UX"
anchors = ["DashboardEvent", "EfficiencyEvent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
No per-task progress percentage, per-turn streaming token counter, diff/patch events when agents modify files, per-request cost streaming, or partial-output delivery cursor; UIs rely on coarse EfficiencyEvent and a separate git watcher.

Imported without verification from:
- `tmp/hermes-product-audit/03-roko-streaming-architecture.md#What's Missing for World-Class UX`

How to verify: Check DashboardEvent for per-turn usage/diff events.
