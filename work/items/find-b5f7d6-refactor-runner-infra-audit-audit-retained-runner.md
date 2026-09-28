+++
id = "find-b5f7d6"
kind = "finding"
title = "[refactor runner-infra-audit] Audit retained runner/ shared infrastructure (22 submodules) for dead paths"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/runner"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#4-audit-shared-runner-infrastructure-p2"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#4-audit-shared-runner-infrastructure-p2"
anchors = ["crates/roko-cli/src/runner/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
runner/ retains 22 submodules (gate dispatch, plan loading, TUI bridging, state management) some of which may hold code only reachable from the deleted event loop.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#4-audit-shared-runner-infrastructure-p2`

How to verify: List runner/ modules and check each for callers outside runner/ and tests.
