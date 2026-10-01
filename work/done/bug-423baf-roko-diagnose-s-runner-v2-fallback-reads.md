+++
id = "bug-423baf"
kind = "bug"
title = "roko diagnose's Runner-v2 fallback reads current_phase as a string, so phase is always null"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/diagnose-graph-runs 05f8854ce"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::build_legacy_report", "crates/roko-cli/src/commands/diagnose.rs::derive_status", "crates/roko-core/src/phase.rs::PlanPhase"]
links = { depends_on = [], blocks = [], related = ["bug-165b22"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_runner_snapshot_reports_its_phase' crates/roko-cli/src && cargo test -p roko-cli --lib a_runner_snapshot_reports_its_phase"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:14Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:13:21Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

For a plan that has no Graph checkpoint, `roko diagnose` falls back to the Runner-v2 snapshot and reads the plan's
phase with `plan_state.get("current_phase").and_then(Value::as_str)`. Runner-v2 wrote `current_phase` as a
`PlanPhase`, an internally tagged enum (`#[serde(tag = "kind", rename_all = "kebab-case")]`), so the value is an
object such as `{"kind": "implementing"}`. `as_str` returns `None`, so `phase` is always `null`. The status then
comes only from `last_error` ("failed" or "unknown"), and the "Plan is paused" suggestion never fires.

## Why it matters

The fallback exists for workspaces that still have Runner-v2 state. On those it reports the wrong status (a
running or finished plan shows as "unknown") and no phase.

## Where

- `crates/roko-cli/src/commands/diagnose.rs::build_legacy_report`: phase read at :1088-1091, status from
  `derive_status` at :1114.
- `derive_status` (:1258) matches plain names ("done", "implementing", …). It does not know the kebab-case
  kinds `complete`, `auto-fixing`, `doc-revision`, `regenerating-verify`, `merging`, `enriching`, `reviewing`
  or `skipped`.
- `build_recovery_suggestions` (:1672) checks `phase == "paused"`, but pausing is a separate
  `PlanState.paused` bool (`crates/roko-cli/src/orchestrator/executor/plan_state.rs:40`), not a phase.
- Writer: `PlanState.current_phase: PlanPhase` (`plan_state.rs:24`), enum at `crates/roko-core/src/phase.rs:155-157`.

## Current state

Checked at 33e107da1. The fallback test `a_plan_without_a_graph_checkpoint_uses_the_runner_snapshot`
(`diagnose.rs:2518`) writes `"current_phase": { "kind": "implementing" }`. It asserts `status == "failed"`, which
comes from `last_error`, and never checks `phase`.

## Plan

1. Read `current_phase` as a string (older snapshots) or as an object's `kind`.
2. Map the kebab-case kinds in `derive_status`, and take the paused suggestion from the `paused` flag.
3. Add `a_runner_snapshot_reports_its_phase`: a snapshot with `{"kind": "implementing"}`, no `last_error` and
   `paused: false` reports phase `implementing` and status `running`.
4. Alternative: delete the Runner-v2 fallback, since Runner-v2 was removed on 2026-09-06. If so, change the
   `[[verify]]` to a test that such a plan gets the "no run state" error.

## Done when

- The fallback reports the snapshot's phase and a status derived from it.
- The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-climain): implemented on work/bug-28c193; cargo verification deferred to the batch check.
  `build_legacy_report` reads the phase through `phase_name` (a string, or the object's `kind`), `derive_status` maps
  the kebab-case kinds (`complete`, `enriching`, `reviewing`, `doc-revision`, `auto-fixing`, `regenerating-verify`,
  `merging`, `skipped`), and the snapshot's `paused` flag turns a running, gating or pending status into `paused`.
  `build_recovery_suggestions` lost its `phase` parameter and keys the paused suggestion on that status. The existing
  fallback test now expects `running` for its `implementing` fixture (its old `failed` came from the unread phase).
  New test: `a_runner_snapshot_reports_its_phase`. The Runner-v2 fallback was kept rather than deleted (Plan step 4).
