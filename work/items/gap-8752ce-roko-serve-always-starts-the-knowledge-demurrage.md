+++
id = "gap-8752ce"
kind = "gap"
title = "roko serve always starts the knowledge-demurrage timer, so two held decay bugs run on every server: park it behind a feature (backlog 9231)"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "S"
subsystem = ["roko-serve/neuro", "roko-cli/features"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#decision-12"
anchors = ["crates/roko-serve/src/lib.rs::start_demurrage_timer", "crates/roko-serve/Cargo.toml", "crates/roko-cli/Cargo.toml"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-dd5dca", "bug-429315", "dec-e70592"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn serve_starts_no_demurrage_timer_by_default' crates/roko-serve/ && cargo test -p roko-serve serve_starts_no_demurrage_timer_by_default"

[[verify]]
command = "grep -q '^knowledge-decay *=' crates/roko-serve/Cargo.toml && grep -B2 'fn start_demurrage_timer' crates/roko-serve/src/lib.rs | grep -q 'knowledge-decay'"
+++

## Problem

Every `roko serve` starts the knowledge-demurrage timer: `start_server` spawns `start_demurrage_timer`
unconditionally (`crates/roko-serve/src/lib.rs:469`; the function at :2632). It ticks every 40 s and, every 250 ticks
(about 2.8-2.9 h), runs roko-runtime's `DemurrageConsumer` over the whole knowledge store. That is how two held bugs run
on every server even though knowledge decay is held (dec-e70592):

- bug-dd5dca: every knowledge entry loses confidence every ~2.8 hours;
- bug-429315: the balance tax charges an entry's whole age on every pass.

## Why it matters

Will's decision 12 (2026-10-15, `tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md`): build backlog task 9231 so `roko
serve` starts no demurrage timer by default; the held subsystems stay held. NEW-FINDINGS finding 7. Knowledge written
by plan runs (the verified-lesson loop, E2b LM3) decays silently on any machine that runs the server, which also skews
L-know's measured benefit (gap-03fcd0).

## Where

- `crates/roko-serve/src/lib.rs::start_demurrage_timer` and its spawn in the server start-up.
- `crates/roko-serve/Cargo.toml` `[features]`; `crates/roko-cli/Cargo.toml` (forward the feature, as `chain`/`groups`
  are forwarded).
- Backlog task: `tmp/backlog/2026-10-02-complete-and-wire/9231-held-serve-starts-no-demurrage-timer-by-default.md`.

## Current state

9231 was written held (hold: dec-e70592) with `depends_on = ["9230", "9229"]` (park decay in roko-neuro and roko-runtime;
park dreams out of serve and ACP), both still held and not started. Decision 12 releases 9231 alone.

## Plan

1. Add a `knowledge-decay` feature to roko-serve, off by default, and forward it from roko-cli. Do not wait for 9230:
   if roko-neuro/roko-runtime have no such feature yet, this one gates only serve's timer.
2. Put `start_demurrage_timer` and its spawn behind `#[cfg(feature = "knowledge-decay")]`.
3. Add `serve_starts_no_demurrage_timer_by_default` (from 9231): a default build's server start-up spawns no demurrage
   task (for example a start-up task list or a counter the test can read), and a store's confidences are unchanged
   after the interval elapses under a paused clock.
4. Add the feature to CLAUDE.md's "Parked (off the default build)" table.

## Done when

Both `[[verify]]` commands pass. Then note on bug-dd5dca and bug-429315 that a default build no longer runs them (they
stay held).

## Notes

- `roko knowledge gc` (on demand) is unchanged.
- Parking pitfalls (memory: static feature-gating): pub items in non-pub modules go dead without the feature; test
  builds deny warnings.
