+++
id = "gap-999649"
kind = "gap"
title = "[plan-audit T3-10] Warm reviewer pre-spawn (WarmPool unwired)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/pool"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
anchors = ["WarmPool"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
WarmPool container exists but is not wired to the provider bridge, so reviewers cold-start after gates instead of being pre-spawned during gate execution.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)`
- `tmp/archive/plan-audit-2026-09-23/01-GAP-MATRIX.md`

How to verify: grep WarmPool non-test usages.
