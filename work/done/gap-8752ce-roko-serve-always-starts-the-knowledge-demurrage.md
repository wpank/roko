+++
id = "gap-8752ce"
kind = "gap"
title = "roko serve always starts the knowledge-demurrage timer, so two held decay bugs run on every server: park it behind a feature (backlog 9231)"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
size = "S"
subsystem = ["roko-serve/neuro", "roko-cli/features"]
created = 2026-10-15
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "f94014685"
source = "status-quo-2026-10-15#decision-12"
anchors = ["crates/roko-serve/src/lib.rs::start_demurrage_timer", "crates/roko-serve/Cargo.toml", "crates/roko-cli/Cargo.toml"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-dd5dca", "bug-429315", "dec-e70592"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn serve_starts_no_demurrage_timer_by_default' crates/roko-serve/ && cargo test -p roko-serve serve_starts_no_demurrage_timer_by_default"

[[verify]]
command = "grep -q '^knowledge-decay *=' crates/roko-serve/Cargo.toml && grep -B2 'fn start_demurrage_timer' crates/roko-serve/src/lib.rs | grep -q 'knowledge-decay'"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T10:07:06Z"
commit = "f94014685"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-11-12T09:05:41Z"
forced = false
evidence = "f94014685. crates/roko-serve: the knowledge-demurrage timer is parked behind a knowledge-decay feature, off by default and forwarded from roko-cli (Cargo.toml in both); start_knowledge_decay starts it only with the feature, and start_demurrage_timer compiles only with it; CLAUDE.md's parked table lists the feature. Red first on a0089c19c (the seam starting the timer unconditionally, as the old start-up did): serve_starts_no_demurrage_timer_by_default failed (a timer started); green: it passes; lib suites roko-cli 4099/0, roko-serve 1091/0; clippy -D warnings clean with and without --features knowledge-decay; nightly fmt clean (tmp/hermes-roko-v8/roko-learnguard/logs/8752ce-chain3.log). Why it matters, measured on a copy of Undertow's store: all 27 lessons from v7 phases 0-4 are at confidence 0, and no prompt from phase 4.1 on included one (tmp/hermes-roko-v8/data/learning). bug-dd5dca and bug-429315 stay held; a default build no longer runs them. Lessons already at 0 stay at 0 (a migration is a separate decision)."
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
