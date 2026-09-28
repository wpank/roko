+++
id = "find-5f6fa5"
kind = "finding"
title = "Clean up residue test plans in plans/ (18 dirs from portal/API testing)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["plans"]
created = 2026-09-25
updated = 2026-09-28
source = "plans/test"
discovered_from = "audit:plans/test"
anchors = ["plans/test", "plans/test-check", "plans/scratch-test", "plans/modal-test", "plans/audit-test", "plans/audit-gen", "plans/audit-gen2", "plans/flow-test-exec", "plans/flow-test-gen", "plans/flow-test-gen2"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
plans/ holds 18 throwaway dirs from portal/API testing on 2026-09-24/25 (test, test-check, scratch-test, modal-test, audit-*, flow-*, hello-world, example-hello-world, test-fibonacci, test-greeting-module, test-plan-generation, e2e-smoke-test); 17 are untracked and inflate INDEX totals.

Imported without verification from:
- `plans/test`
- `plans/test-check`
- `plans/scratch-test`
- `plans/modal-test`
- `plans/audit-test`
- `plans/audit-gen`
- `plans/INDEX.md`

How to verify: Confirm with owner before deleting (e2e-smoke-test is tracked); consider archiving or a tmp workspace for portal tests.
