+++
id = "bug-f98a13"
kind = "bug"
title = "Hermes ACP turn timeout restarts on every notification, so a chatty agent never times out"
status = "open"
triage = "unverified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/hermes"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-7f15df"
anchors = ["crates/roko-agent/src/hermes/acp_agent.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-7f15df"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib hermes_acp_turn_timeout"
+++

## Problem

Hermes's ACP turn loop (`hermes/acp_agent.rs`, run and run_streaming) re-creates `tokio::time::sleep(timeout)` on every select iteration, so each notification restarts it. The turn timeout is really an idle timeout, and a chatty agent never times out. OpenClaw pins its sleep.

## Plan

Pin the sleep once per turn, as OpenClaw does. Add a test named `hermes_acp_turn_timeout_*`.

## Done when

- `cargo test -p roko-agent --lib hermes_acp_turn_timeout` passes.

## Notes

- Reported on 2026-10-01 by wk-guard2, working on bug-7f15df, during the evening close-out round.
