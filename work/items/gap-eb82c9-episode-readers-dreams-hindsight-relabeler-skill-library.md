+++
id = "gap-eb82c9"
kind = "gap"
title = "Episode readers (dreams, hindsight relabeler, skill library, curriculum) should read the settled learning label"
status = "open"
triage = "unverified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["learn"]
created = 2026-09-29
updated = 2026-09-29
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
