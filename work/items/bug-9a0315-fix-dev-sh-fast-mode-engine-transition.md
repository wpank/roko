+++
id = "bug-9a0315"
kind = "bug"
title = "Fix dev.sh FAST Mode Engine Transition"
status = "superseded"
triage = "verified"
severity = "p0"
subsystem = ["roko-cli"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/376-fix-dev-sh-fast-engine-transition.md#376 — Fix dev.sh FAST Mode Engine Transition"
discovered_from = "audit:tmp/backlog/archive/376-fix-dev-sh-fast-engine-transition.md#376 — Fix dev.sh FAST Mode Engine Transition"
anchors = ["dev.sh:307", "crates/roko-cli/src/commands/plan.rs:597"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "bug-f7943a" }

[closed]
at = 2026-09-28
evidence = "duplicate of bug-f7943a (still true: dev.sh:307 passes --engine runner-v2, which crates/roko-cli/src/commands/plan.rs:597-604 rejects with a removal error)"
+++
the primary FAST development loop is broken. dev.sh line 307 passes `--engine runner-v2`, but Runner-v2 was deleted on Sep 6 (commit 6b5da8616). Every FAST invocation exits immediately with EXIT_FAILURE. All FAST-specific Rust code (gate mode narrowing, impact analysis, targeted checks, agent turn…

Imported without verification from:
- `tmp/backlog/archive/376-fix-dev-sh-fast-engine-transition.md#376 — Fix dev.sh FAST Mode Engine Transition`

How to verify: Check whether the gap described in tmp/backlog/archive/376-fix-dev-sh-fast-engine-transition.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]

Verified 2026-09-28: still true (dev.sh:307); duplicate of bug-f7943a (verified, in_progress).
