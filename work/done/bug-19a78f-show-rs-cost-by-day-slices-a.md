+++
id = "bug-19a78f"
kind = "bug"
title = "show.rs cost_by_day slices a timestamp by bytes and panics on non-ASCII"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-6c11d1"
anchors = ["crates/roko-cli/src/commands/show.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-6c11d1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn cost_by_day_never_slices_a_timestamp_by_bytes' crates/roko-cli/src/commands/show.rs && cargo test -p roko-cli --bin roko cost_by_day"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:07Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:15:15Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

`show.rs::cost_by_day` takes `timestamp[..10]` by bytes, so a malformed timestamp with a multi-byte character before byte 10 panics `roko show costs`.

## Plan

Parse the date (or use `get(..10)`), and skip malformed rows. Add a test named `cost_by_day_*`.

## Done when

- `cargo test -p roko-cli --bin roko cost_by_day` passes.

## Notes

- Reported on 2026-10-01 by wk-tuiv, working on bug-6c11d1, during the evening close-out round.
- 2026-10-01 (wk-tuiv): implemented on work/bug-6c11d1; cargo verification deferred to the batch check.
  `cost_by_day` takes each event's UTC date from its parsed RFC 3339 timestamp (`event_time`); a timestamp that
  does not parse counts as `undated` rather than being sliced, so its cost stays in the breakdown instead of
  being skipped. Days are UTC dates now, where the old prefix used the timestamp's own offset. Test
  `cost_by_day_never_slices_a_timestamp_by_bytes`. The verify command gained a grep guard: a bare filter passes
  when no test matches.
