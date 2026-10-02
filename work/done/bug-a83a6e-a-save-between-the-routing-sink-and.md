+++
id = "bug-a83a6e"
kind = "bug"
title = "A save between the routing sink and the journal can count a category outcome twice on replay"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "efb9acf44"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-a6a3cd"
anchors = ["crates/roko-learn/src/runtime_feedback/routing.rs", "crates/roko-learn/src/wal.rs", "crates/roko-cli/src/runtime_feedback/routing.rs", "crates/roko-learn/src/model_call_feedback.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-a6a3cd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-learn --lib category_counted_once_across_a_save"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T01:38:31Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-02T00:42:09Z"
forced = false
evidence = "Gate 6g on 698ca793d, merged as efb9acf44 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-cli, roko-core, roko-learn and roko-neuro; lib tests pass (roko-cli 3431, roko-core 1986, roko-learn 1234, roko-neuro 239); all eight canaries, golden_path_suite, secret_canary and C2 pass; graph_timeout_matrix 7/7 including the worktree-mode case; bin 445; portal tsc clean and vitest 807/807; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

The journal assumes its caller already moved the category counts, as the Graph routing sink does (`runtime_feedback/routing.rs:140`). A save between the two calls lets a replay count the category twice.

## Plan

Record the category counts inside the journal's lock, and add a test named `category_counted_once_across_a_save`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-02 by wk-settle, working on bug-a6a3cd, during the overnight close-out round.
- 2026-10-02 (wk-settle): implemented on work/bug-f9ae3e; cargo verification deferred to the batch check.
  `ModelCallJournal::observe_task_outcome` and `observe_override_outcome` now move the task category's counts
  themselves, under the journal's lock with the WAL append and the observation; an untracked model's counts still
  move, unjournaled. The Graph routing sink (`crates/roko-cli/src/runtime_feedback/routing.rs`) records the category
  only on its paths without a journal. Test: `category_counted_once_across_a_save`;
  `journaled_category_counts_survive_a_crash` no longer moves the counts by hand.
- The anchor `crates/roko-learn/src/runtime_feedback/routing.rs` is roko-learn's routing module; the sink is
  roko-cli's, added above.
