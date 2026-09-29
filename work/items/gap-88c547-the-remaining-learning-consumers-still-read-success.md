+++
id = "gap-88c547"
kind = "gap"
title = "The remaining learning consumers still read success instead of the settled learning label"
status = "open"
triage = "unverified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["learn"]
created = 2026-09-29
updated = 2026-09-29
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
