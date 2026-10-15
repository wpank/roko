+++
id = "gap-6b6ec1"
kind = "gap"
title = "Whole-plan and post-merge regression checks publish no hub event while they run, so a host shows 'between tasks' for the whole check"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "26d836e38"
source = "tmp/hermes-roko-v5/findings/repro-will.md (row C2; roko-gates, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: a demo run from Hermes showed 'between tasks' for a 25-minute regression check"
anchors = ["crates/roko-cli/src/graph_execution/plan_verify.rs::run_plan_verify", "crates/roko-cli/src/graph_execution/delivery.rs::GitDeliveryBackend", "crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent"]
links = { depends_on = [], blocks = [], related = ["bug-a7e018", "bug-b3849c", "bug-7b62ff"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_check_publishes_start_heartbeat_and_finish' crates/roko-cli/ && cargo test -p roko-cli plan_check_publishes_start_heartbeat_and_finish"

[closed]
at = 2026-10-15
at_ts = "2026-10-15T21:23:24Z"
commit = "26d836e38"
forced = false
evidence = "Each step of a plan's whole-plan check publishes plan_check_started, plan_check_heartbeat (every 15 s) and plan_check_finished on the StateHub (GET /api/events, the run's events index): run_plan_verify (in_place) and GitDeliveryBackend::regression_output (regression) through PlanCheckEvents (graph_execution/plan_check_events.rs), built by plan_runner from the run's GraphTuiBridge; status.json phase 'check'; CLI output lines; /health features plan_check_events. Tests on main 4505cd54d + the item: plan_check_publishes_start_heartbeat_and_finish (the verify), regression_check_publishes_its_steps, check_scope_names_what_a_step_builds, graph_status_follows_a_run_through_its_phases pass; roko-cli lib 3645, roko-core lib 2021, roko-serve lib 1019 passed, 0 failed; clippy -p roko-core -p roko-cli -p roko-serve -D warnings clean; cargo +nightly fmt --check clean."
+++

## Problem

A plan's whole-plan check (its `[meta] verify`, or the default `cargo fmt`/`clippy`/`test` of a Cargo workspace)
and the delivery's post-merge regression check publish nothing on the StateHub while they run: no event when a
step starts, none while it runs, none when it ends. Only `tracing` lines ("delivery merged", "delivery regression
passed") record them, in roko's log.

Seen 2026-10-15 (`tmp/hermes-roko-v5/findings/repro-will.md` row C2): a demo task started from Hermes passed in
14 s, then the delivery's regression check ran `cargo fmt --all --check` + `cargo clippy --workspace` for
24 m 44 s. For those 25 minutes the Hermes page said "RUNNING · between tasks…": `GET /api/events` and the run's
events index carried no event, so no host could say what roko was doing.

## Why it matters

A host (the Hermes plugin, the portal, `roko dashboard`) cannot tell a long check from a hang. The person waits
without knowing why, cancels (which, before bug-7b62ff, leaves the check running), or starts another run.

## Where

- `crates/roko-cli/src/graph_execution/plan_verify.rs::run_plan_verify`: the check in the shared working tree.
- `crates/roko-cli/src/graph_execution/delivery.rs::GitDeliveryBackend::regression_output`: the check in the
  regression checkout.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::deliver_plan_to_batch` / `check_plan_in_place`: where the
  run's `GraphTuiBridge` is at hand.
- `crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent`: the hub's event type.

## Plan

Three `DashboardEvent`s, published per check step: `plan_check_started` (plan, where it runs, step n of m, its
phase, a short name such as `cargo clippy`, its scope: `workspace`, `packages` with the crates, or `command`),
`plan_check_heartbeat` every 15 s while it runs (elapsed time), and `plan_check_finished` (passed, exit code,
elapsed time). They reach `GET /api/events` and the run's events index like every hub event; the CLI prints the
start and the end.

## Done when

`plan_check_publishes_start_heartbeat_and_finish` passes (the `[[verify]]`), and a real run's events index holds
the three events for its regression check.

## Notes

The plugin renders them (hermes-update / plugin UI); the event shapes are in
`tmp/hermes-roko-v5/findings/roko-gates.md`.
