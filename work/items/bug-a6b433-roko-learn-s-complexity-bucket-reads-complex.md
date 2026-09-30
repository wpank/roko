+++
id = "bug-a6b433"
kind = "bug"
title = "roko-learn's complexity_bucket reads complex as architectural, while TaskTier reads it as integrative"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-learn/conductor", "roko-core/task"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tiers's report, branch work/gap-8c0a20 at dca6042b1)"
anchors = ["crates/roko-learn/src/conductor.rs", "crates/roko-core/src/task.rs"]
lane = "rust-hot"
parent = "spec-98f76d"
links = { depends_on = ["gap-8c0a20"], blocks = [], related = ["gap-8c0a20"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn complexity_bucket_agrees_with_task_tier' crates/roko-learn/src/ && cargo test -p roko-learn --lib complexity_bucket_agrees_with_task_tier"
+++

## Problem

gap-8c0a20's branch gives the plan parser, router, budget and turn caps one tier enum. Its parser maps `"integrative" | "complex" | "t2" | "2"` to `TaskTier::Integrative` (`roko-core/src/task.rs:173` on the branch). roko-learn's conductor has its own label table, `complexity_bucket` (`roko-learn/src/conductor.rs:423`), which reads `complex` as architectural (wk-tiers). A task labelled `complex` is integrative for dispatch, but architectural for the conductor's decisions (:260, :323, :466).

## Why it matters

Tier ladder and escalation (epic spec-98f76d): one label, two tiers. The conductor's interventions and the router's tier disagree for every `complex` task.

## Where

`complexity_bucket` in `conductor.rs`, and the shared tier parser in `task.rs`.

## Plan

1. Make `complexity_bucket` use the shared tier parser, mapping `TaskTier` to its buckets, instead of its own string table.
2. Add `complexity_bucket_agrees_with_task_tier` over every label the parser accepts.

## Done when

- [ ] Every tier label means the same tier in the conductor and in dispatch.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on gap-8c0a20's branch.
