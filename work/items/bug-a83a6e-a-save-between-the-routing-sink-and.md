+++
id = "bug-a83a6e"
kind = "bug"
title = "A save between the routing sink and the journal can count a category outcome twice on replay"
status = "open"
triage = "unverified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn"]
created = 2026-10-02
updated = 2026-10-02
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-a6a3cd"
anchors = ["crates/roko-learn/src/runtime_feedback/routing.rs", "crates/roko-learn/src/wal.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-a6a3cd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-learn --lib category_counted_once_across_a_save"
+++

## Problem

The journal assumes its caller already moved the category counts, as the Graph routing sink does (`runtime_feedback/routing.rs:140`). A save between the two calls lets a replay count the category twice.

## Plan

Record the category counts inside the journal's lock, and add a test named `category_counted_once_across_a_save`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-02 by wk-settle, working on bug-a6a3cd, during the overnight close-out round.
