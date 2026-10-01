+++
id = "bug-f98a13"
kind = "bug"
title = "Hermes ACP turn timeout restarts on every notification, so a chatty agent never times out"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/hermes"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "f4323cf9d"
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
- 2026-10-01 (wk-guard2): implemented on work/bug-7f15df; cargo verification deferred to the batch check.
- `run` and `run_streaming` in hermes/acp_agent.rs now pin one `sleep(timeout)` per turn, created once the prompt is sent, as OpenClaw does; notifications no longer restart it. Tests `hermes_acp_turn_timeout_ends_a_chatty_turn` and `hermes_acp_turn_timeout_ends_a_chatty_streaming_turn` drive the agent against a `bash -c` stand-in that streams a notification every 50 ms and never finishes; with a 300 ms timeout each turn must end within 5 s. Hermes still reports a timed-out turn as a success with partial output (OpenClaw fails it); that is unchanged here.
