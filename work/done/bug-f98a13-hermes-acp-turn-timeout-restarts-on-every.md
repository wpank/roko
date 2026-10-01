+++
id = "bug-f98a13"
kind = "bug"
title = "Hermes ACP turn timeout restarts on every notification, so a chatty agent never times out"
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
discovered_from = "bug-7f15df"
anchors = ["crates/roko-agent/src/hermes/acp_agent.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-7f15df"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib hermes_acp_turn_timeout"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:27Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:14:43Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
