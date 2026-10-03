+++
id = "bug-2410e1"
kind = "bug"
title = "A resumed run in a new process redraws its chains' arm sets; --srm has no row for per-section bandit draws"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch", "roko-learn/telemetry"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-7 follow-up reports 2026-10-03 (PK41 gap-c2b1a3)"
discovered_from = "gap-c2b1a3"
anchors = ["crates/roko-cli/src/graph_task_dispatch/attempt.rs::RunAttempts", "crates/roko-learn/src/telemetry/report.rs::srm_check", "crates/roko-learn/src/section_effect.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn resumed_run_in_a_new_process_keeps_its_arm_sets' crates/roko-cli/ && cargo test -p roko-cli resumed_run_in_a_new_process_keeps_its_arm_sets"
+++

## Problem

`RunAttempts::open` (`crates/roko-cli/src/graph_task_dispatch/attempt.rs`, ~lines 74-82) computes a fresh
`epoch = chrono::Utc::now().format("%Y-%m-%d").to_string()` every time it's called, and starts `arm_sets`
(`parking_lot::Mutex::new(HashMap::new())`) empty every time — neither is restored from anything on disk. Later,
`arm_set` (~lines 119-127) draws a chain's arms with `ArmDraws::new(ARM_SEED, self.epoch.clone())`, caching the
result in `arm_sets` keyed by `key.chain_key()` — but only within the current process's in-memory map.

So: a plan resumed in a **new process**, on a **later UTC day** than the one its earlier attempts ran on, redraws
every chain's arm set from scratch — `self.epoch` differs from the original attempt's epoch, and the in-memory
cache that would have reused the original draw doesn't survive the process boundary at all, so even a same-day
resume in a new process redraws (the epoch happening to match doesn't help, since `arm_sets` starts empty either
way; the epoch only makes it worse by also changing the *seed input* across a day boundary). `roko learn telemetry
check --srm` reports these as "redrawn" units (per `srm_check`'s own doc comment: "a unit whose rows carry another
draw is a problem, since a retry must keep its chain's arms" — `telemetry/report.rs:850-851`).

Separately: `--srm`'s layer check (`srm_check`, `telemetry/report.rs:854`) is fully generic — it reads whatever
`decision_point` a `content_decisions` row carries and treats each distinct value as a layer to check. It is not
hardcoded to skip "sections." But PK38's per-section bandit draw mechanism (`crates/roko-learn/src/section_effect.rs`)
never writes a `content_decisions` row with a `decision_point`/`arm_set` at all (confirmed: zero matches for
"decision_point", "arm_set" or "ContentDecision" in that file) — so `--srm` has nothing to check for the
per-section bandit's own draws, not because of an omission in the checker, but because the writer side doesn't
yet emit a compatible row.

## Why it matters

Goal: cybernetic/learning (S02 SC3, `--srm` exists specifically to catch arm-assignment bugs like the first one).
A chain that switches arms mid-run (because its run resumed in a new process on a later day) corrupts the
randomization every downstream learning signal assumes holds — the whole point of drawing once per chain and
keeping it is that outcomes can be attributed to one arm, not a mix. And if the per-section bandit's own draws
are invisible to `--srm`, nothing would catch the same class of bug there even once it exists in volume.

## Where

- `crates/roko-cli/src/graph_task_dispatch/attempt.rs::RunAttempts::open` (the epoch and the empty `arm_sets`
  map) and `::arm_set` (the draw/cache logic that depends on both).
- `crates/roko-learn/src/telemetry/report.rs::srm_check` and `SrmTally::read` (the generic layer-detection logic;
  not itself at fault for the second half).
- `crates/roko-learn/src/section_effect.rs` (the per-section bandit; writes no content-decision row at all).

## Current state

Unfixed on both halves. `RunAttempts::open`'s epoch and arm-set cache are purely in-memory and per-process, with
no persistence keyed by `run_id` that a resumed process could read back. `section_effect.rs`'s bandit draws are
not wired into the content-decision/arm-set logging path `srm_check` reads.

## Plan

1. Persist the run's epoch (computed once, at the run's true start, not at every process's `open` call) and each
   chain's drawn arm set somewhere `RunAttempts::open` reads back on a resume — e.g. alongside `attempts.jsonl`
   in the run directory, recovered the same way `AttemptOrdinals::load` already recovers ordinals.
2. Add a regression test: open a run, draw a chain's arms, simulate a day boundary (or just a fresh process/struct
   with a different `epoch`), re-open the same `run_id`, and assert the same chain gets the same arm set.
3. Give `section_effect.rs`'s bandit draws a `content_decisions` row with `decision_point = "sections"` (or a
   distinct point name) and an `arm_set`, so `--srm` can check it like every other content-decision layer —
   no change needed to `srm_check` itself, only to the writer.

## Done when

- A plan resumed in a new process, any number of UTC days later, keeps every chain's original arm set.
- `roko learn telemetry check --srm` reports a layer for the per-section bandit's draws.
- The `[[verify]]` command passes.

## Notes

- The epoch/arm-set persistence fix (step 1) and the section-bandit wiring fix (step 3) are independent and can
  land separately; this item covers both because both were reported together from the same PK41 investigation
  (gap-c2b1a3), but a worker may split them into two commits/closes if that's cleaner.
