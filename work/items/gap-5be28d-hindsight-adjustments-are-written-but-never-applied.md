+++
id = "gap-5be28d"
kind = "gap"
title = "Hindsight adjustments are written but never applied: nothing reads episode-adjustments.jsonl"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli/runtime_feedback", "roko-learn/hindsight"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:feat/learning-completion-loops fb87e3738"
anchors = ["crates/roko-cli/src/runtime_feedback/hindsight.rs::HindsightSink", "crates/roko-learn/src/hindsight.rs::read_adjustments", "crates/roko-learn/src/hindsight.rs::append_new_adjustments", "crates/roko-cli/src/dispatch/prompt_cache.rs::load_episodes", "crates/roko-cli/src/commands/learn.rs::print_learn_episodes", "crates/roko-cli/src/runtime_feedback/routing.rs"]
links = { depends_on = [], blocks = [], related = ["gap-5fb9a7", "find-34a4b5", "reg-3f5969", "bug-86117a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_relabeled_success_no_longer_counts_as_a_success' crates/roko-cli/src && cargo test -p roko-cli --lib a_relabeled_success_no_longer_counts_as_a_success"
+++

## Problem

fb87e3738 wired `HindsightSink` into the Graph feedback facade (`plan_runner.rs:944`). When a later
verify failure blames a sibling task, the sink relabels that task's latest success and appends an
`EpisodeAdjustment` to `.roko/learn/episode-adjustments.jsonl`. Nothing reads the log.
`roko_learn::hindsight::read_adjustments` (`hindsight.rs:210`) is called only by
`append_new_adjustments`, to skip duplicates, and by tests. Every consumer therefore still treats the
relabeled episode as a success:

- **Prompt episode retrieval.** `dispatch/prompt_cache.rs::load_episodes`, then
  `collect_episode_knowledge_cached`, which ranks successes first (`prompt_builder.rs:2347`) and
  prints them as "passed".
- **`roko learn episodes`.** `commands/learn.rs::print_learn_episodes` (`:1505`) and
  `collect_episodes_json` (`:881`).
- **Incremental counters credited at completion, with no retraction:**
  - the router's category and bandit observations (`runtime_feedback/routing.rs:85`);
  - the playbook success counters (763596768);
  - the strategy fragment and reinforcement that `VerifiedKnowledgeSink` admitted from the verified
    attempt (189a14e65).

## Why it matters

Goal `learning`. A relabel that changes no decision and no display is an audit log, not a feedback
loop. The success that broke a sibling's gate keeps rewarding the model, the playbook and the
knowledge that produced it.

This is the follow-up that gap-5fb9a7's plan asked to be filed ("feeding adjustments back into
routing or playbook confidence is a separate decision"). Its step 4 (show adjustments in `roko learn
episodes`) was also not done when it closed. Related: find-34a4b5 (P3-32), reg-3f5969.

## Where

- Writer: `crates/roko-cli/src/runtime_feedback/hindsight.rs::HindsightSink::on_event`, which calls
  `append_new_adjustments` and logs how many it appended.
- Log API: `crates/roko-learn/src/hindsight.rs`: `EpisodeAdjustment` (`original_episode_id`,
  `adjustment_kind`, `old_value`, `new_value`, `reason`), `read_adjustments` and
  `append_new_adjustments`.
- Readers that should apply it: the prompt caches (`roko-cli/src/dispatch/prompt_cache.rs`,
  `roko-execution/src/prompt/cache.rs`), `commands/learn.rs`, the routing sink
  (`runtime_feedback/routing.rs`), `PlaybookStore::record_outcome` (`roko-learn/src/playbook.rs:1075`)
  and dream replay (`roko-dreams/src/runner.rs`).

## Current state

Checked at 33e107da1. `grep -rn 'read_adjustments\|DEFAULT_ADJUSTMENTS_FILE' crates/` finds only the
writer, its installer at `plan_runner.rs:946`, and tests. `append_new_adjustments` returns a count,
not the adjustments it added, so a caller cannot act on only the new ones.

## Plan

1. **Read side.** Add `roko_learn::hindsight::apply_adjustments(&mut [Episode], &[EpisodeAdjustment])`.
   For a `Regression` it sets `success = false` and records the reason in `extra`. Call it wherever
   episodes are loaded for prompts or display: both prompt caches, `roko learn episodes` and its
   JSON output (which also shows the adjustment count and the latest few), and dream replay.
2. **Write side.** Make `append_new_adjustments` return the adjustments it appended. For each new
   `Regression`, `HindsightSink` retracts the credit given at completion:
   - a playbook failure for the playbook the episode used (`extra.playbook_id`);
   - a failure observation for the episode's model and task category in the router;
   - optionally, a contradiction on the knowledge admitted from that attempt.
   Apply each correction once. The log already de-duplicates by episode and kind.
3. If a counter should not be corrected (for example, the router is judged too noisy), write that
   down here and skip only that counter.

## Done when

- The test `a_relabeled_success_no_longer_counts_as_a_success` (roko-cli lib) passes. It runs a plan
  in which T1 succeeds using playbook P, then T2's verify failure blames T1. It asserts that the
  prompt cache reads T1's episode as not successful, and that P records one failure.
- `roko learn episodes` shows the adjustment.
- The `[[verify]]` command passes.

## Notes

- Keep the log separate from `episodes.jsonl`, as fb87e3738 did (gap-5fb9a7, option B). Episode
  readers would otherwise count correction rows as episodes.
- `prompt_cache.rs` is also touched by bug-86117a. Do the two one after the other.
