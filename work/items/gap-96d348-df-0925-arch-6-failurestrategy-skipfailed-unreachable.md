+++
id = "gap-96d348"
kind = "gap"
title = "DF-0925 arch-6: FailureStrategy::SkipFailed unreachable; FailFast hardcoded"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph/convert"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#Architecture recommendations"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#Architecture recommendations"
anchors = ["types.rs:142 FailureStrategy", "convert.rs:57 plan_to_graph"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
plan_to_graph never sets SkipFailed, so FailFast applies in practice and cannot be configured; plan granularity is the only blast-radius control.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#Architecture recommendations`

How to verify: Check plan_to_graph failure strategy wiring.
