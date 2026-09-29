+++
id = "gap-5fb9a7"
kind = "gap"
title = "Hindsight relabeling module is completely unwired"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "learning"
subsystem = ["roko-learn/hindsight"]
created = 2026-09-01
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F035"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F035"
anchors = ["crates/roko-learn/src/hindsight.rs::HindsightRelabeler", "crates/roko-cli/src/runtime_feedback/episodes.rs", "crates/roko-cli/src/runtime_feedback/plan_completion.rs::DreamConsolidationSink", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body", "crates/roko-learn/src/episode_logger.rs::Episode"]
links = { depends_on = [], blocks = [], related = ["bug-121c35", "find-34a4b5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'HindsightRelabeler' crates/roko-cli/src/ && grep -rqw 'fn hindsight_sink_writes_regression_adjustment' crates/roko-cli/src/ && cargo test -p roko-cli --lib hindsight_sink_writes_regression_adjustment"
+++

## Problem

`HindsightRelabeler` (`crates/roko-learn/src/hindsight.rs`) is never constructed or called. No runtime path
re-examines past episodes, so an episode's outcome stays at the value written when the task finished, even
when later evidence contradicts it:

- a task marked successful whose files a later task broke (a later gate failure on the same task or files);
- a failed episode whose approach a later successful run reused;
- an episode that sourced a playbook rule which was later contradicted.

Observable: after any number of `roko plan run` executions there is no `episode_adjustment` record anywhere,
and `grep -rn HindsightRelabeler crates` finds only the module itself.

## Why it matters

- Goal `learning` (learning loops on the Graph path). Outcome labels are the training signal for routing,
  playbooks and distillation. Without retrospective correction, a "success" that caused a regression keeps
  rewarding the model and approach that produced it.
- This is one of the closures lost with Runner-v2 and listed in `find-34a4b5` (16 learning closures wired into
  the deleted `event_loop.rs`); that item points here for the hindsight part.
- Related: `bug-121c35` (parked): the regression rule uses time order plus shared files, not data flow, so
  concurrent tasks sharing a noise file such as `Cargo.toml` would be blamed for each other's failures. Keep
  that in mind when choosing what goes into `extra["files"]`.

## Where

- `crates/roko-learn/src/hindsight.rs`: `HindsightRelabeler::scan(&[Episode], &[Rule]) -> Vec<EpisodeAdjustment>`
  (30-day window) and `HindsightRelabeler::append(&EpisodeLogger, &[EpisodeAdjustment])`, which writes each
  adjustment as an `Episode` with `kind = "episode_adjustment"`, agent id `"hindsight"`, and the
  adjustment in `extra["adjustment"]`. One unit test.
- `crates/roko-learn/src/lib.rs:94`: `pub mod hindsight;` (the only reference outside the file).
- `crates/roko-learn/src/episode_logger.rs::Episode` (line 186): fields the scan reads: `success`, `timestamp`,
  `task_id`, `gate_verdicts`, `extra["files"]`, `extra["source_episode_ids"]`, `extra["reused_episode_id"]`.
- Graph-path episode writers (entry point `roko plan run <dir>`):
  - `crates/roko-cli/src/runtime_feedback/episodes.rs` (facade sink, `on_event` ~line 51) handles
    `FeedbackEvent::TaskCompleted` from `graph_task_dispatch.rs::emit_feedback`. Writes success, usage, model,
    `plan_id`, knowledge/playbook ids, failure class. No `gate_verdicts`, no `files`.
  - `crates/roko-cli/src/graph_execution/feedback.rs::EpisodeSink` (~line 250): settlement sink; writes only
    provider, task id, success and an idempotency key.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body` (~lines 900-1000): builds the `FeedbackFacade` and
  attaches the plan-completion sinks (`DreamConsolidationSink`, `DaimonPersistenceSink`,
  `ThetaReflectionSink`, `DeltaConsolidationSink`). This is where a hindsight sink would be attached.
- `crates/roko-cli/src/runtime_feedback/plan_completion.rs`: `DreamConsolidationSink`, the model for a
  sink that reacts to `FeedbackEvent::PlanCompleted`.
- `crates/roko-learn/src/playbook_rules.rs::PlaybookRules` / `Rule` (`source_episodes` at line 89,
  `contradictions` updated at ~580).

## Current state

- Unchanged since it was added (last touched in `310860465`). No reference outside its own file.
- Even if called today, two of the three rules could never fire on Graph episodes:
  - Regression needs a later episode with a failed entry in `gate_verdicts`. No Graph writer fills
    `gate_verdicts` (only `demo_seed.rs:566` does).
  - SuccessfulReuse needs `extra["source_episode_ids"]` or `extra["reused_episode_id"]`. Nothing writes them.
  - HeuristicFalsified needs playbook rules with `contradictions > 0` whose `source_episodes` contain the
    episode id. `PlaybookRules` already lowers confidence by 0.10 on each contradiction, so this adjustment is
    informational; applying it again would double-count.
- Risk in the module's own design: `append` writes into the same `episodes.jsonl`. A grep of roko-learn,
  roko-dreams, roko-neuro and roko-cli found no episode reader that skips a `kind` (the only kind check is
  `feedback_service.rs:970`, for `"workflow_complete"`), and `Episode::new` defaults `success = false`.
  Adjustment rows would likely be counted as failed episodes by agent `"hindsight"` in success rates, dreams
  and `roko learn episodes`. Check each reader before relying on this.
- The CLAUDE.md "E25 hindsight adjustments wired" claim has been removed; nothing claims this works any more.

## Plan

1. Give Graph episodes the inputs the scan needs, in the facade sink (`runtime_feedback/episodes.rs`):
   - `gate_verdicts`: carry the gate outcome of the attempt. `emit_feedback` (called at ~3807, ~3891, ~4453)
     writes the episode before the gate result is handled; the gate outcome is handled later in
     `graph_task_dispatch.rs` (the gate-fail/gate-pass efficiency rows at ~2433 and ~2706). Either emit a second, gate-result episode there (`kind = "gate"`), or delay the episode
     until the gate result is known. Recommend the first: smaller change, and the scan treats any later failed
     verdict on the same task or files as the signal.
   - `extra["files"]`: the task's declared `files` from `TaskDef` (cheap, known up front). Exclude workspace
     noise files (`Cargo.toml`, `Cargo.lock`) to limit the `bug-121c35` false positives.
2. Decide where adjustments are stored:
   - A: the module's design, `.roko/episodes.jsonl` with `kind = "episode_adjustment"`. Needs every episode
     reader to skip that kind (EpisodeLogger readers, dreams, neuro, TUI, serve); easy to miss one.
   - B (recommended): a separate log, e.g. `.roko/learn/episode-adjustments.jsonl`. No API change:
     `HindsightRelabeler::append` takes an `&EpisodeLogger`, so pass one opened on the new path. Readers opt in.
3. Add a `HindsightSink` in `crates/roko-cli/src/runtime_feedback/` that reacts to the plan-completion event
   (`FeedbackEvent::PlanCompleted`, the event `DreamConsolidationSink` in
   `runtime_feedback/plan_completion.rs` handles), loads recent episodes from `.roko/episodes.jsonl` and rules
   from `PlaybookRules`, calls `HindsightRelabeler::new().scan(..)`, de-duplicates against adjustments already
   written (same `original_episode_id` + `adjustment_kind`), and appends the rest. Best-effort: log and continue
   on error, like the other sinks. Attach it in `plan_runner.rs` next to `DreamConsolidationSink`.
4. Make the adjustments visible: have `roko learn episodes` (or `roko learn all`) print the adjustment count
   and the latest few. Feeding adjustments back into routing or playbook confidence is a separate decision;
   file a follow-up item rather than doing it here.
5. Tests: a roko-cli unit test (for example `hindsight_sink_writes_regression_adjustment`) that writes a
   successful episode and a later failed gate episode sharing a file, fires the plan-completion event, and
   asserts one `Regression` adjustment in the adjustments log and no new rows in `episodes.jsonl`.

## Done when

- A plan run in which a later task's gate fails on files an earlier successful task touched leaves one
  `regression` adjustment for the earlier episode in the adjustments log.
- `.roko/episodes.jsonl` gains no `episode_adjustment` rows (if option B), or all readers skip them (option A).
- `roko learn episodes` (or the chosen command) shows the adjustments.
- The verify command passes. The current one is weak: any mention of `HindsightRelabeler` outside the module
  and tests (a doc comment, an unused import) satisfies it. Suggested replacement:
  `grep -rq 'HindsightRelabeler' crates/roko-cli/src/ && grep -rqw 'fn hindsight_sink_writes_regression_adjustment' crates/roko-cli/src/ && cargo test -p roko-cli --lib hindsight_sink_writes_regression_adjustment`

## Notes

- Persistence risk: step 2 option A changes what every episode reader sees. If A is chosen, audit every reader of
  `episodes.jsonl` first. Option B avoids that.
- Two Graph writers already append episodes for the same attempt (facade sink and settlement sink); the scan's
  `later.task_id == episode.task_id` test can match such pairs. Unknown whether that produces false
  adjustments; check with the test data.
- Touches `runtime_feedback/episodes.rs` and `plan_runner.rs`, which other learning items (`find-34a4b5`
  follow-ups) also edit; coordinate if run in parallel. No dependency must land first.

## Original notes

`HindsightRelabeler` is defined in `crates/roko-learn/src/hindsight.rs` (210 LOC, 1 test) but is never instantiated or called from any runtime path. Episode labels are never corrected for retrospective information (regressions, contradictions). The entire module is dead code.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F035`
- `tmp/archive/provider-audit/17-learning-loop.md`

How to verify: E25 claims hindsight adjustments wired; check hindsight relabeling callers. Confirm in crates/roko-learn/src/hindsight.rs whether still true: Hindsight relabeling module is completely unwired

Verified 2026-09-28: HindsightRelabeler (crates/roko-learn/src/hindsight.rs:43, scan at :64) has no reference outside its own file. roko-learn/src/lib.rs:94 only declares the module, and no crate uses roko_learn::hindsight. This contradicts CLAUDE.md's E25 claim that hindsight adjustments are wired.

Rechecked 2026-09-29: still unwired. CLAUDE.md no longer contains the E25 claim that hindsight adjustments are wired, so that sentence is now only historical.
