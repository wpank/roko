+++
id = "bug-b38546"
kind = "bug"
title = "pre_verify:no_changes says the attempt left the tree unchanged when the task's declared files are gitignored"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "9c0b9aed0"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "coordinator, from the evidence e2e failures on the batch-13 binary (2026-09-30)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/red_flags.rs"]
lane = "rust-cold"
parent = "spec-9230a9"
links = { depends_on = [], blocks = [], related = ["gap-b72761", "bug-809e22"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn no_changes_names_gitignored_declared_files' crates/roko-cli/src/ && cargo test -p roko-cli --lib no_changes_names_gitignored_declared_files"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 670a3bde2. The no_changes red flag names declared files that are gitignored and says roko's diff and delivery cannot see them. Batch 15a gate on 9005da604, re-assembled as 8a2ee8bca with only settle's rustfmt commit changing two files' formatting (MAIN 9c0b9aed0 has the same code): cargo check --workspace --tests clean; nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-compose -p roko-core -p roko-learn -p roko-serve --keep-going -D warnings clean; lib tests pass: roko-cli 3190 (two flakes, the turn_policy escalated-timeout test and graph_run_routing_observations_survive_a_crash, pass alone and in their module), roko-agent 2268, roko-core 1952, roko-learn 1204, roko-serve 986, roko-compose 560; cargo test -p roko-cli --test learning_wiring_census: 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
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
