+++
id = "bug-37a893"
kind = "bug"
title = "Delivery's merge commit needs the person's git identity, unlike roko's other commits: with no user.name or user.email, a run of two plans delivers the first and fails the second ('git commit-tree failed: Author identity unknown', CI run 39408440267)"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-cli"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
source = "round v8 portal-v8, reading CI's portal check evidence (2026-11-19)"
discovered_from = "agent:portal-v8 (round v8)"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs", "crates/roko-cli/src/orchestrator/worktree/acceptance.rs::ACCEPT_IDENTITY"]
links = { depends_on = [], blocks = [], related = ["find-8cc7ac"], supersedes = [], duplicate_of = "" }
+++

## Problem

roko's delivery merges a second plan's work into its run's branch with
`git commit-tree` (`crates/roko-cli/src/graph_execution/delivery.rs`). That call takes the person's git identity,
unlike roko's other commits:

- worktree acceptance passes `ACCEPT_IDENTITY` (`-c user.name=roko -c user.email=roko@localhost`);
- batch commits pass `user.name=roko` too.

On a machine where git has no `user.name` or `user.email`, a run of two plans delivers the first and fails the
second, as GitHub's runners showed (run 39408440267, 2026-10-20): par-b's delivery failed with "git commit-tree
failed: Author identity unknown". The run then ended failed with one plan delivered.

## Why it matters

Goal release: a fresh container, a CI job or a new machine without a git identity gets a failed run from
roko's own merge, while roko's other commits work there. Most people have an identity, so this is p3.

## Where

`crates/roko-cli/src/graph_execution/delivery.rs`, the `commit-tree` call in the merge path.

## Plan

- Give the merge commit roko's identity when git has none, as acceptance does. If the person has an identity, use
  theirs; that choice is the fixer's, so say it in the item.
- Test: a two-plan delivery in a repo with no identity (HOME pointed at an empty directory) delivers both.

## Done when

- A run of two plans delivers both on a machine without a git identity.
- The `[[verify]]` passes; it's written with the fix.

## Notes

- Round v8 portal-v8, found reading CI's evidence for bug-044a2d. The CI job works around it with a git identity.
