+++
id = "gap-eb82c9"
kind = "gap"
title = "Episode readers (dreams, hindsight relabeler, skill library, curriculum) should read the settled learning label"
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
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (22:20, wk-settle's report on gap-8f6206)"
anchors = ["crates/roko-dreams/", "crates/roko-learn/src/episodes.rs", "crates/roko-cli/src/runtime_feedback/episodes.rs"]
lane = "rust-cold"
parent = "spec-6ac537"
links = { depends_on = ["gap-8f6206"], blocks = [], related = ["gap-8f6206"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn episode_readers_use_the_learning_label' crates/ && cargo test --workspace --lib episode_readers_use_the_learning_label"
+++

## Problem

gap-8f6206 adds `extra.outcome`, `blame` and `learning_label` to episodes, but episode `success` keeps its old meaning, in which an unverified attempt counts as success. `roko diagnose` joins on it, and a turn_policy test pins it. The episode readers still read `success`: dreams, the hindsight relabeler, the skill library and the curriculum.

## Why it matters

Offline consolidation and curriculum learn from the wrong label. Epic spec-6ac537.

## Where

Each reader of `.roko/episodes.jsonl`. Find them with `grep -rn 'episodes.jsonl\|EpisodeLogger\|read_episodes' crates/`.

## Current state

The readers use `success`.

## Plan

1. Make each reader use `extra.learning_label`. Skip unlabelled episodes. Legacy rows without a label keep `success`.
2. Add the test the verify names.

## Done when

- [ ] No episode reader learns from an unverified success.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-07bc75` at `de3d1e22b`; cargo verification deferred to the batch check. In the
  worktree's own target clone, `cargo check -p roko-cli -p roko-learn -p roko-dreams --lib --tests` and the
  targeted tests of the three crates passed, among them `episode_readers_use_the_learning_label` (roko-learn) and
  `dreams_skip_episodes_without_a_learning_label` (roko-dreams).
- **Decisions (2026-09-29):**
  - roko-learn gains `Episode::learning_success`, `learnable_episodes` and `LEARNING_LABEL_KEY`, which the episode
    sink now writes with. A labelled episode counts as its label says, an unlabelled one (`null`) is skipped, and
    a legacy row without the key keeps `success`.
  - For a labelled row, `success` already equals `label == 1`. So readers that keep reading `success` only need
    the unlabelled rows dropped at their boundary.
  - The hindsight relabeler reads labels in both branches and does not drop rows, so an unverified later attempt
    still shadows an earlier pass as the task's latest.
  - The skill library reads labels in its candidate check, Voyager extraction and `evolve_skills`. A provider
    failure no longer downgrades skills.
  - The curriculum (`RoleToolProfile::from_episodes`) counts learnable episodes only.
  - Dreams (`DreamCycle::run_budgeted`) replay and compare learnable episodes only, but `processed_through` still
    passes the skipped ones. The runner's heartbeat and backlog counters count activity and read every episode.
- **Left open:** other episode readers outside this item still read `success`: `provider_model_outcome`,
  `pattern_discovery`, `cfactor`, `aggregate`, `post_gate_reflection`, roko-learn's `runtime_feedback/` (wk-router2's
  code) and the dream runner's `replay_insights` API, which has no production caller.
- 2026-09-29 (wk-model-truth, from bug-31438d): `work/bug-31438d` (not merged at 7fa54b873) adds more episode fields that these readers should honour: `extra.turns_unknown` (the turn count is unknown, not 1) and `extra.model_reported`, `extra.model_mismatch` and `extra.substituted_from` (the served model differs from the dispatched one). Dreams, the hindsight relabeler, the skill library and the curriculum should skip or down-weight mismatched episodes, and treat unknown turns as unknown.
