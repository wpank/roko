+++
id = "gap-8a4d00"
kind = "gap"
title = "DF-0926 R-6: apps/portal has no test runner"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["apps/portal"]
created = 2026-09-26
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#R-6. Cross-plan contradictions the audits surfaced"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#R-6. Cross-plan contradictions the audits surfaced"
anchors = ["apps/portal/package.json"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
No test script, vitest or jest and zero test files in apps/portal, so plan tasks ordering unit tests cannot run them.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#R-6. Cross-plan contradictions the audits surfaced`

How to verify: Check package.json scripts for test.
