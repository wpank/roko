+++
id = "gap-e411e5"
kind = "gap"
title = "demo_seed.rs still writes task-metrics.jsonl, which the dashboard no longer reads"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/demo-seed", "roko-cli/tui"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "7789cfee6"
source = "wave-4 follow-up reports 2026-10-02 (PK08)"
discovered_from = "gap-a0043b (its own Progress note on task 2126, commit 1ef44eb93, names this exact residual with no task to fix it)"
anchors = ["crates/roko-cli/src/demo_seed.rs::build_task_metrics", "crates/roko-cli/src/tui/dashboard_model.rs::attempt_ledger_metrics"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rqw 'fn build_task_metrics' crates/roko-cli/src/demo_seed.rs || grep -rqw 'fn demo_seed_feeds_the_attempt_ledger_dashboard' crates/roko-cli/ && cargo test -p roko-cli demo_seed_feeds_the_attempt_ledger_dashboard"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T11:30:40Z"
commit = "7789cfee6"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T09:32:44Z"
forced = false
evidence = "Gate 17a (merged 7789cfee6): its verify passes, with workspace check, clippy, lib tests and the learning wiring census green. Part of the p3 cleanup branch (work/gap-9d32d0): L-dream-bias retired from loops.toml and the census, TaskSpeedPriority docs, the dead vcg_warmup_observations field, error_pattern_store's module doc, and demo_seed feeding the attempt ledger instead of task-metrics.jsonl."
+++

## Problem

`roko init --demo`'s seeder (`crates/roko-cli/src/demo_seed.rs`) still builds and writes a full set of
`TaskMetric` records to `.roko/learn/task-metrics.jsonl` (`build_task_metrics`, line 647; written via
`layout.memory_dir().join("task-metrics.jsonl")` at line 216), but nothing that a demo user would see reads that
file any more:

- The dashboard's task-metrics panel used to read it, but backlog task 2126 (part of PK08's own package,
  `gap-a0043b`, implemented at `1ef44eb93`) switched it to `attempt_ledger_metrics`
  (`crates/roko-cli/src/tui/dashboard_model.rs:1979`), which derives `TaskMetric`s fresh from each run's
  `.roko/runs/<run>/attempts.jsonl` ledger instead. `commands/dashboard.rs:205` and
  `tui/dashboard_model.rs:182` both call this function, not anything that reads `task-metrics.jsonl`.
- `demo_seed.rs` does not seed any `attempts.jsonl` / `AttemptVerdictRecord` data (grepped the whole file for
  `attempts.jsonl`, `AttemptVerdictRecord` and `verdict`: the only hit is `episode_gate_verdicts`, which builds
  `EpisodeGateVerdict` rows for the episode log, a different record entirely), so the dashboard's demo view shows
  nothing derived from the task-metrics write either way.
- gap-a0043b's own Progress notes already record this exact observation verbatim: "2126: implemented at
  `1ef44eb93` (`demo_seed.rs` still writes `memory/task-metrics.jsonl`, which nothing reads now)" — but that
  package has no task that cleans it up; it is twelve other tasks (2112-2128), none of which touch `demo_seed.rs`.

The `LearningRuntime::open()`/`open_with_models()` constructors (`crates/roko-learn/src/runtime_feedback/mod.rs:491,565`)
do still call `load_task_metrics(&paths.task_metrics_jsonl)` and keep the result in a `task_metrics` field, so the
file is not entirely inert at the storage layer — but nothing in this slice's dashboard or CLI surfaces present
that loaded state to a user, so for `demo_seed.rs`'s actual purpose (populating what `roko init --demo` shows),
the write produces nothing visible.

## Why it matters

Goal: tooling/demo hygiene. A seeded demo workspace writes a file that does no work (wasted effort, a stale
shape that could silently drift from `TaskMetric`'s real schema without anyone noticing, and a red herring for
anyone reading `demo_seed.rs` trying to understand what feeds the dashboard). Low severity: no functional harm
today, since `attempt_ledger_metrics` works whether or not this file exists.

## Where

- `crates/roko-cli/src/demo_seed.rs::build_task_metrics` (line 647) and its write at line 216.
- `crates/roko-cli/src/tui/dashboard_model.rs::attempt_ledger_metrics` (line 1979) — the function the dashboard
  actually uses now, reading `attempts.jsonl` per run.
- `crates/roko-learn/src/runtime_feedback/mod.rs` (lines 491, 565, 1513) — the only remaining reader/writer of
  `task_metrics_jsonl`, unrelated to the demo dashboard.

## Current state

`demo_seed.rs` still calls `build_task_metrics` and writes its result; nothing in the demo-facing dashboard path
reads it. Confirmed at HEAD on the `tooling` goal's working branch.

## Plan

1. Decide whether demo-seeded data should instead populate `attempts.jsonl` for a fake run (so
   `attempt_ledger_metrics` picks it up and the dashboard shows seeded headline data again), or whether the
   task-metrics write should simply be deleted as dead weight (if the demo dashboard doesn't need a populated
   headline panel, or seeds it another way already — check `seeded_groups`/`skipped_groups` reporting and whatever
   the demo app itself expects).
2. If deleting: remove `build_task_metrics`, its write call, and the now-unused `roko_core::metric::{ConfigHash,
   TaskMetric}` import if nothing else in the file needs them.
3. If replacing: seed a demo run directory with `attempts.jsonl`/verdict records matching `AttemptVerdictRecord`'s
   shape instead, reusing `build_task_metrics`'s per-task data as input.

## Done when

- `demo_seed.rs` either writes data the dashboard's current code path (`attempt_ledger_metrics`) actually reads,
  or no longer writes `task-metrics.jsonl` at all.
- The `[[verify]]` command passes.

## Notes

- Do not touch `LearningRuntime::open()`'s own `load_task_metrics` call — that's a separate, real consumer
  unrelated to the demo dashboard; this item is scoped to `demo_seed.rs`'s now-pointless write for demo-display
  purposes only.
- Related: gap-a0043b (PK08's own open package, where this was first noticed as a Progress-note aside, not as a
  tracked task).
