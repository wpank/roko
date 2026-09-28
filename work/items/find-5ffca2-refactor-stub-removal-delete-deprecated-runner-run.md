+++
id = "find-5ffca2"
kind = "finding"
title = "[refactor stub-removal] Delete deprecated runner::run() stub and runner_v2_guard.rs tests after migration"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/runner"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#2-remove-the-deprecated-runnerrun-stub-p1"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#2-remove-the-deprecated-runnerrun-stub-p1"
anchors = ["runner::run", "runner_v2_guard.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Once the 4 call sites migrate (#342), remove the #[deprecated] runner::run() stub and the runner_v2_guard.rs tests documenting the pending migration.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#2-remove-the-deprecated-runnerrun-stub-p1`

How to verify: Check whether runner::run and runner_v2_guard.rs still exist.
