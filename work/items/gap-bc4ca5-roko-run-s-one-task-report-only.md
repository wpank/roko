+++
id = "gap-bc4ca5"
kind = "gap"
title = "roko run's one-task report only reads root episodes.jsonl, so a frozen run reports no turns, tokens or cost"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/run"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "gate-13c follow-up reports 2026-10-04 (gap-127263)"
discovered_from = "gap-127263 (closed; fix scoped to the bench driver, not roko run's own report)"
anchors = ["crates/roko-cli/src/run.rs::task_episodes_since", "crates/roko-core/src/config/learning.rs::LearningConfig"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn frozen_one_task_run_reports_real_turns_tokens_and_cost' crates/roko-cli/ && cargo test -p roko-cli frozen_one_task_run_reports_real_turns_tokens_and_cost"
+++

## Problem

`roko run`'s one-task report reads only the root episodes log, so under `[learning] frozen =
true` it reports no turns, tokens or cost. `task_episodes_since` (`crates/roko-cli/src/run.rs:867-887`)
is called with `path = layout.root_episodes_path()` (`.roko/episodes.jsonl`, set at
`run.rs:594`). But `gap-127263` (closed) made a frozen run's episode sink write to the run's own
`.roko/runs/<run_id>/episodes.jsonl` instead (`EpisodeSink::per_run`, `20eb73a96`) specifically
so a frozen run's episodes never land in the shared root log — documented in
`crates/roko-core/src/config/learning.rs:199-200`'s own doc comment: "A frozen run writes its
episodes to its own `.roko/runs/<run_id>/episodes.jsonl` instead, which no later run reads
(gap-127263)." `gap-127263`'s fix updated the ViabilityBench driver
(`benchmarks/viabilitybench/driver/run_roko.py`) to read that per-run log, but `roko run`'s own
CLI summary report was a different consumer it never touched: `task_episodes_since` still only
ever reads root, so under a frozen config `episodes` comes back empty, `episodes.last()` is
`None`, and every field the `WorkflowRunReport` derives from the last episode (model, turns,
tokens, cost) reports as empty/zero — not because nothing happened, but because the report is
looking in the wrong file.

## Why it matters

Goal: truth, `roko run`'s one-task report accuracy. A frozen workspace is explicitly meant for
reproducible benchmark/demo runs (per other wave-5 reports cited in `bug-dd20bd`), which is
exactly when an accurate report matters most for comparing runs. Right now the report silently
looks empty instead of either reporting real numbers or saying why it can't.

## Where

- `crates/roko-cli/src/run.rs::task_episodes_since`, and its one call site (`run.rs:664`,
  `episodes_path` from `run.rs:594`).
- `crates/roko-learn/src/telemetry/report.rs::RunRecords::load` (the existing, tested pattern
  for reading a run's own `runs/<run_id>/episodes.jsonl` — already used elsewhere, e.g.
  `crates/roko-cli/src/graph_execution/learning_commit.rs::settled_outcomes`).
- `crates/roko-core/src/config/learning.rs:193-200` (`frozen`, the doc comment that already
  names this exact gap).

## Current state

Confirmed by reading `run.rs`'s `episodes_path`/`task_episodes_since` call chain: it reads only
`layout.root_episodes_path()`, with no fallback or additional read of
`layout.run_dir(&run_id).join("episodes.jsonl")`.

## Plan

1. When the root-log read for this `run_id` comes back empty (or always, merging both), also
   read `layout.run_dir(&run_id).join("episodes.jsonl")` — the same per-run path
   `RunRecords::load` already reads elsewhere — and use those episodes for the report.
2. Add a regression test: a frozen one-task `roko run`, asserting the printed/returned report
   has a real model, turn count, token count and cost rather than empty/zero fields.

## Done when

- A frozen `roko run`'s one-task report shows real turns, tokens, cost and model.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (gate-13c follow-up, gap-127263, main HEAD `908f7ec40`): `gap-127263` is closed;
  its fix was scoped to the ViabilityBench driver, not `roko run`'s own CLI report, so this is a
  genuine follow-up rather than a regression. See also `bug-dd20bd` (same file, the
  `record_workflow_feedback` frozen-episode question) — noted there too, since the per-run log
  this item reads from may be relevant to how that bug gets fixed.
