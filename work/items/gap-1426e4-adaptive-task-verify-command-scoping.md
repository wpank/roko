+++
id = "gap-1426e4"
kind = "gap"
title = "Adaptive Task-Verify Command Scoping"
status = "open"
triage = "verified"
severity = "p2"
goal = "features"
subsystem = ["roko-cli/runner"]
created = 2026-09-07
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/backlog/archive/170-adaptive-verify-scoping.md#170 — Adaptive Task-Verify Command Scoping"
discovered_from = "audit:tmp/backlog/archive/170-adaptive-verify-scoping.md#170 — Adaptive Task-Verify Command Scoping"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::settle_task_verification", "crates/roko-cli/src/runner/gate_dispatch.rs:923", "crates/roko-cli/src/runner/impact_analysis.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'impact_analysis::' crates/roko-cli/src/graph_task_dispatch.rs"
+++
smarter verify commands reduce false rejections and token waste. Plan task-verify commands are currently static strings written at plan creation time (e.g., `cargo test -p roko-gate`). These run the entire crate's test suite regardless of what the agent changed. This leads to:

Imported without verification from:
- `tmp/backlog/archive/170-adaptive-verify-scoping.md#170 — Adaptive Task-Verify Command Scoping`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: When an agent modifies a single file, the verify test command is scoped to that module.; When scoping is applied, it is logged in the gate output.; Pre-existing failures in unmodified modules do not cause gate rejection. [evidence: 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): S | 3 |]

Verified 2026-09-28: still true - the Graph dispatcher runs authored [[task.verify]] commands verbatim (graph_task_dispatch.rs settle_task_verification, ~1402); impact_analysis narrowing is only applied to FAST gate rungs (runner/gate_dispatch.rs:923). Severity lowered p1 -> p2: an optimisation, not a broken loop.
