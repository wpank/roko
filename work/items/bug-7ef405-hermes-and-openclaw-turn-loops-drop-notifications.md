+++
id = "bug-7ef405"
kind = "bug"
title = "Hermes and OpenClaw turn loops drop notifications queued just before the turn completes"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/hermes"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "60426839b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-f98a13"
anchors = ["crates/roko-agent/src/hermes/acp_agent.rs", "crates/roko-agent/src/openclaw/acp_agent.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-f98a13"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib turn_drains_notifications_before_completion"
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
