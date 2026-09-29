+++
id = "gap-88c547"
kind = "gap"
title = "The remaining learning consumers still read success instead of the settled learning label"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["learn"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (23:00, wk-settle's report on bug-07bc75 and gap-eb82c9)"
anchors = ["crates/roko-learn/src/", "crates/roko-cli/src/runtime_feedback/", "crates/roko-dreams/src/"]
lane = "rust-cold"
parent = "spec-6ac537"
links = { depends_on = ["gap-eb82c9"], blocks = [], related = ["gap-8f6206", "gap-eb82c9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn remaining_consumers_use_the_learning_label' crates/ && cargo test -p roko-learn --lib remaining_consumers_use_the_learning_label"
+++

## Problem

After gap-8f6206 and gap-eb82c9, the main learners read the settled `learning_label`. These consumers still read `success`, so an unverified attempt counts as a success in them (wk-settle, 2026-09-29):

- `provider_model_outcome`
- `pattern_discovery`
- `cfactor`
- `aggregate`
- `post_gate_reflection`
- roko-learn's `runtime_feedback/`
- the dream runner's `replay_insights` API, which has no production caller

## Why it matters

Every learning loop must learn from verified outcomes for the cybernetic claim to hold. Epic spec-6ac537.

## Where

Find them with `grep -rn '\.success' crates/roko-learn crates/roko-dreams crates/roko-cli/src/runtime_feedback`.

## Current state

They read `success`.

## Plan

1. Make each one read `Episode::learning_success` or the event's `learning_success`. Skip unlabelled rows; legacy rows keep `success`.
2. Mark `replay_insights` unused, or port it too.
3. Add a test per consumer group.

## Done when

- [ ] No consumer learns from an unverified success.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/gap-88c547` at `0ff2c2de7`; cargo verification deferred to the batch check. The branch is
  `work/bug-07bc75` (da1045989) with `413b0dcc9` merged in (`e93736a46`). Its one conflict was gap-eb82c9's notes,
  resolved by keeping both bullets. In the worktree's own target clone, `cargo check -p roko-cli -p roko-learn
  -p roko-dreams --lib --tests`, the targeted tests of roko-learn and roko-dreams, nightly fmt and clippy passed.
- **Decisions (2026-09-29):**
  - `provider_model_outcome`: `from_episode` records no outcome for an unlabelled episode, and its status reads
    the label.
  - `pattern_discovery`: the cross-episode consolidator encodes the outcome as success, failure or unlabelled from
    the label instead of dropping rows. That way its `episode_indices` still index the caller's list (dream routing
    advice maps them back), and the dream cycle feeds it learnable episodes only. The trigram miner never read
    `success`.
  - `cfactor`: `compute_cfactor`, `detect_pathologies` and `variance_inequality_check` count learnable episodes
    only. An unlabelled attempt, spend included, leaves the c-factor.
  - `aggregate` (`compute_compounding_metrics`, no production caller, gap-14f08e): passes and routing accuracy come
    from the label, `gate_pass_rate` is over labelled attempts, and cost per success is every attempt's spend.
  - `post_gate_reflection`: its `success` read is an LLM helper's result. `ReflectionInput::from_episode` also
    returns `None` for an unlabelled episode, which carries no gate verdict today.
  - roko-learn `runtime_feedback/`, three small hunks: after provider health records it, `record_completed_run`
    treats an unlabelled attempt like a skipped-gates run; `apply_affect_signature` skips the affect appraisal;
    `KnowledgeSeedRecord::from_successful_episode` needs label 1. `update_cascade_router` is untouched
    (wk-router2's bug-3ea1f5).
  - Dreams: `replay_insights` is ported and replays learnable episodes only.
- **Left open:** `ProviderModelOutcomeRecord::from_efficiency_event` reads the efficiency rows' `outcome`, which
  carries no label. Graph dispatch doesn't feed it. `EpisodeView::succeeded` has no reader, and
  `LearningRuntime::discover_cross_episode_patterns` has no caller.
