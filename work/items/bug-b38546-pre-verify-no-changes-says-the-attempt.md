+++
id = "bug-b38546"
kind = "bug"
title = "pre_verify:no_changes says the attempt left the tree unchanged when the task's declared files are gitignored"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "coordinator, from the evidence e2e failures on the batch-13 binary (2026-09-30)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/red_flags.rs"]
lane = "rust-cold"
parent = "spec-9230a9"
links = { depends_on = [], blocks = [], related = ["gap-b72761", "bug-809e22"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn no_changes_names_gitignored_declared_files' crates/roko-cli/src/ && cargo test -p roko-cli --lib no_changes_names_gitignored_declared_files"
+++

## Problem

The pre-verify screen (gap-b72761) rejects an implementer attempt whose git-visible diff is empty as `pre_verify:no_changes`, with the message "its attempts have left the working tree as they found it". When the task's declared `files` are gitignored, the agent may well have written them. The rejection is right, because roko's diff and delivery are git-based and cannot carry an ignored file. The message is wrong, though, and sends the user looking for an agent that did nothing. This is how `scripts/test_run_evidence_graph.py` broke: its fixture ignored `out/`.

## Why it matters

A misleading red flag costs a debugging session. Epic spec-9230a9.

## Where

`crates/roko-cli/src/graph_task_dispatch/red_flags.rs`, the no-changes check and its message.

## Current state

The rejection happens and the message is generic.

## Plan

1. When the diff is empty, check each declared file with `git check-ignore`. For each ignored file that exists on disk, name it in the message and say roko's diff and delivery cannot see changes to it.
2. Keep the rejection.
3. Test: `no_changes_names_gitignored_declared_files`.

## Done when

- [ ] The message names gitignored declared files.
- [ ] The `[[verify]]` command passes.
