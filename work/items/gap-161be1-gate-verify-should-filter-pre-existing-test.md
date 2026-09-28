+++
id = "gap-161be1"
kind = "gap"
title = "Gate Verify Should Filter Pre-Existing Test Failures"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/runner"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/166-gate-verify-preexisting-filter.md#166 — Gate Verify Should Filter Pre-Existing Test Failures"
discovered_from = "audit:tmp/backlog/archive/166-gate-verify-preexisting-filter.md#166 — Gate Verify Should Filter Pre-Existing Test Failures"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::settle_task_verification", "crates/roko-cli/src/runner/gate_dispatch.rs::run_gate_once", "crates/roko-cli/src/runner/gate_dispatch.rs::spawn_gate"]
links = { depends_on = [], blocks = [], related = ["gap-a534e4"], supersedes = [], duplicate_of = "" }
+++
agents waste tokens and retries on failures they didn't cause. Task verify commands in plans (e.g., `cargo test -p roko-gate`) run the entire crate's test suite, including pre-existing broken tests that the agent was never assigned to fix. When a pre-existing test fails, the gate rejects the…

Imported without verification from:
- `tmp/backlog/archive/166-gate-verify-preexisting-filter.md#166 — Gate Verify Should Filter Pre-Existing Test Failures`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.3 Proof Case 3: Baseline gate rejection filtering`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: A pre-existing test failure does not cause a gate rejection for an unrelated task.; A genuine regression (test that passed before the agent's changes and now fails); The gate logs which failures were filtered as pre-existing. [evidence: 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): S | 3 |]

Verified 2026-09-28: The baseline filter exists only in crates/roko-cli/src/runner/gate_dispatch.rs::run_gate_once (filter_preexisting_failures, run_focused_baseline_verify), reachable through spawn_gate from tests only. The Graph path's graph_task_dispatch.rs::settle_task_verification runs authored [[task.verify]] steps with no baseline. Live confirmation: tmp/dogfood/2026-09-25-portal-programme-run.md N-8 (whole-crate gates inherit pre-existing failures; the tests had to be #[ignore]d).
