+++
id = "bug-05d1ac"
kind = "bug"
title = "verification.rs quotes each step's full command in skipped-step lists, progress events and gate output, so a pinned step repeats its script in retry feedback"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch/verification"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-specq's report)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["gap-1b5636", "gap-d14a43"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn pinned_steps_are_quoted_by_their_header_in_feedback' crates/roko-cli/src/ && cargo test -p roko-cli --lib pinned_steps_are_quoted_by_their_header_in_feedback"
+++

## Problem

`crates/roko-cli/src/graph_task_dispatch/verification.rs` puts a step's whole `step.command` into:

- the skipped-steps list (:89);
- the progress event (`message: format!("verify: {}", step.command)`, :96);
- the published gate output (`published_gate_output(&step.command, …)`, :195).

A pinned acceptance step's command is its generated script of about 25 lines. When it fails, the script is repeated in the retry feedback the agent reads. gap-1b5636 fixed the prompt, not this feedback.

## Why it matters

Specs a cheap model can execute (epic spec-e57870): retry feedback should say what failed, not repeat harness plumbing.

## Where

The three call sites in `verification.rs`.

## Plan

1. Label a pinned step by its header line (`# roko accept: <test>`), as gap-1b5636 does for prompts, in all three places.
2. Add `pinned_steps_are_quoted_by_their_header_in_feedback`.

## Done when

- [ ] No feedback or event repeats a pinned step's script.
- [ ] The `[[verify]]` command passes.
