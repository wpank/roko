+++
id = "gap-8a4d00"
kind = "gap"
title = "DF-0926 R-6: apps/portal has no test runner"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
subsystem = ["apps/portal"]
created = 2026-09-26
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#R-6. Cross-plan contradictions the audits surfaced"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#R-6. Cross-plan contradictions the audits surfaced"
anchors = ["apps/portal/package.json", "apps/portal/package.json:11"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
commit = "39e72deef"
by = "triage check 2026-09-28"
evidence = "apps/portal/package.json now has `\"test\": \"vitest run\"` (:11) and a `vitest` ^5.0.2 devDependency (:33), and apps/portal/src contains 17 *.test.ts(x) files (e.g. src/lib/bootstrap.test.ts, operation.test.ts, keymap.test.ts), all added by the portal commit. (Static check against 3d0ee4d02; tests not re-run.)"
+++
No test script, vitest or jest and zero test files in apps/portal, so plan tasks ordering unit tests cannot run them.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#R-6. Cross-plan contradictions the audits surfaced`

How to verify: Check package.json scripts for test.

Fixed in 39e72deef (checked 2026-09-28).
