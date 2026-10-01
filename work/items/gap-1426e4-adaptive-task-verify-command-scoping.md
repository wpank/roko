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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/backlog/archive/170-adaptive-verify-scoping.md#170 — Adaptive Task-Verify Command Scoping"
discovered_from = "audit:tmp/backlog/archive/170-adaptive-verify-scoping.md#170 — Adaptive Task-Verify Command Scoping"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::settle_task_verification", "crates/roko-cli/src/graph_task_dispatch/verify_focus.rs::focus_steps", "crates/roko-cli/src/runner/gate_dispatch.rs:923", "crates/roko-cli/src/runner/impact_analysis.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'impact_analysis::' crates/roko-cli/src/graph_task_dispatch && grep -rqw 'fn focused_verify_scopes_an_authored_cargo_test_to_the_changed_target' crates/roko-cli/src && cargo test -p roko-cli --lib focused_verify_scopes_an_authored_cargo_test_to_the_changed_target"
+++
smarter verify commands reduce false rejections and token waste. Plan task-verify commands are currently static strings written at plan creation time (e.g., `cargo test -p roko-gate`). These run the entire crate's test suite regardless of what the agent changed. This leads to:

Imported without verification from:
- `tmp/backlog/archive/170-adaptive-verify-scoping.md#170 — Adaptive Task-Verify Command Scoping`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: When an agent modifies a single file, the verify test command is scoped to that module.; When scoping is applied, it is logged in the gate output.; Pre-existing failures in unmodified modules do not cause gate rejection. [evidence: 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): S | 3 |]

Verified 2026-09-28: still true - the Graph dispatcher runs authored [[task.verify]] commands verbatim (graph_task_dispatch.rs settle_task_verification, ~1402); impact_analysis narrowing is only applied to FAST gate rungs (runner/gate_dispatch.rs:923). Severity lowered p1 -> p2: an optimisation, not a broken loop.

## Notes

2026-10-01 (wk-gates): implemented on work/bug-951930; cargo verification deferred to the batch check. The scoping is
opt-in. With `[gates] mode = "focused"`, Graph verify (`run_verify_steps` via `focus_verify_steps` in
`graph_task_dispatch/verify_focus.rs`) runs the runner's impact analysis on the attempt's changes. Each authored
`cargo test -p <package>` step for the one impacted target is then narrowed with `scoped_test_command`, now
`pub(crate)`: `--test <name>`, or `--lib -- <module>::` when one source file with tests changed. Workspace rungs and
every other step run as written, and any impact fallback or timeout leaves all steps as authored. Each scoping is
logged, and the gate output leads with the scoped command. Failures that also happen on the run's start commit were
already filtered by the baseline check (gap-161be1). `gates.mode = "focused"` and the `gates.impact_*` keys are no
longer reported as inert on the Graph engine. Default (`full`) verification is unchanged; making scoping the default
is a product decision, because narrowing tests can miss breakage elsewhere in the crate. The `[[verify]]` now
searches the `graph_task_dispatch/` directory and runs `focused_verify_scopes_an_authored_cargo_test_to_the_changed_target`.
