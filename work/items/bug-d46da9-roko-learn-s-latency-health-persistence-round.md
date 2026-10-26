+++
id = "bug-d46da9"
kind = "bug"
title = "roko-learn's latency_health_persistence_round_trip waits a fixed 250 ms for a debounced save, so it fails when the machine is busy"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-learn/latency"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 roko-recovery, bug-b7f6a8's post-merge verify (2026-10-26 21:28), the lead's ask"
discovered_from = "agent:roko-recovery (round v7)"
anchors = ["crates/roko-learn/src/latency.rs::latency_health_persistence_round_trip", "crates/roko-learn/src/latency.rs::spawn_save_worker"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'sleep(Duration::from_millis(250))' crates/roko-learn/src/latency.rs && cargo test -p roko-learn --lib -- latency_health_persistence_round_trip"
+++

## Problem

roko-learn's `latency::tests::latency_health_persistence_round_trip` fails when the machine is busy.
- **How it checks:** it records two latency observations, sleeps a fixed 250 ms, then asserts that the registry's
  debounced autosave has written `latency-stats.json`.
- **Why that's racy:** the save worker waits `LATENCY_SAVE_DEBOUNCE` (100 ms) after the last observation, then
  takes the save lock and writes the file with `roko_fs::atomic_write_json`. Under load, the thread's wakeup and the
  write can take longer than the 150 ms the sleep leaves.
- **Seen 2026-10-26 21:28:** in roko-recovery's post-merge verify of bug-b7f6a8 (`cargo test -p roko-core -p
  roko-learn -p roko-cli -p roko-serve --lib`), with three heavy.sh cargo jobs running. It failed with "debounced
  autosave should create the file" (latency.rs:578).
  - The same test binary then passed 5 of 5 runs on its own.
  - It had passed in the three earlier runs of the same suite that day.

## Why it matters

Every post-merge verify that runs roko-learn's lib suite can go red on a busy machine for no reason, and round v7
runs three cargo jobs at once most of the day. A red run that turns green on rerun costs a check and a re-queue, and
teaches people to ignore red.

## Where

- `crates/roko-learn/src/latency.rs`:
  - `latency_health_persistence_round_trip`: the fixed `std::thread::sleep(Duration::from_millis(250))` before
    `assert!(path.exists(), ...)`;
  - `spawn_save_worker` and `LATENCY_SAVE_DEBOUNCE`: the debounced save the test waits for.

## Plan

Wait on the save's completion instead of a fixed time. Either:
- poll for the file, and for the stats it should hold, until a generous deadline (a few seconds), returning as soon
  as they're there; or
- give the registry a test-only flush that asks the worker to save now and waits for it to finish.

Don't just lengthen the sleep.

## Done when

The test no longer sleeps a fixed time before its first check, and it passes with the machine under load: three
cargo jobs at once, as round v7 runs them.
