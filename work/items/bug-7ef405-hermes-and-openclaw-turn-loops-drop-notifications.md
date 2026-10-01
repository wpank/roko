+++
id = "bug-7ef405"
kind = "bug"
title = "Hermes and OpenClaw turn loops drop notifications queued just before the turn completes"
status = "open"
triage = "unverified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/hermes"]
created = 2026-10-01
updated = 2026-10-01
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
