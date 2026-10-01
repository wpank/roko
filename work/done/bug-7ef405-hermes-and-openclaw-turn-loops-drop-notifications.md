+++
id = "bug-7ef405"
kind = "bug"
title = "Hermes and OpenClaw turn loops drop notifications queued just before the turn completes"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/hermes"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-f98a13"
anchors = ["crates/roko-agent/src/hermes/acp_agent.rs", "crates/roko-agent/src/openclaw/acp_agent.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-f98a13"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib turn_drains_notifications_before_completion"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:16Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:49:41Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

The Hermes and OpenClaw ACP turn loops break as soon as the completion arrives, without draining queued notifications. When select picks the turn-done branch first, text the agent sent just before completing is lost.

## Plan

On completion, drain the notification queue before ending the turn. Add a test named `turn_drains_notifications_before_completion`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-guard2, working on bug-f98a13, during the evening close-out round.
- 2026-10-01 (wk-guard2): implemented on work/bug-7f15df; cargo verification deferred to the batch check.
- The three turn loops (Hermes `run` and `run_streaming`, OpenClaw `run_acp_lifecycle`) now use a biased `select!` that polls the deadline first, then notifications, then the completion. The stdout reader queues lines in order, so every notification the agent sent before its completion is already queued when the completion is, and it is read first; no separate drain is needed. Polling the deadline first keeps a flood of notifications from starving it. Tests `hermes_turn_drains_notifications_before_completion`, `hermes_streaming_turn_drains_notifications_before_completion` and `openclaw_turn_drains_notifications_before_completion` run each loop against a shared `bash -c` stand-in (`harness::acp_client::test_servers`) that sends twenty notifications and the completion in one burst, and check the output is all twenty, in order.
