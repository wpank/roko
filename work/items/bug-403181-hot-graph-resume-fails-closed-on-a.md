+++
id = "bug-403181"
kind = "bug"
title = "Hot Graph resume fails closed on a torn last Activity line, which plan resume now sets aside"
status = "open"
triage = "unverified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-graph"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-dc1d16"
anchors = ["crates/roko-graph/src/hot.rs", "crates/roko-graph/src/replay.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-dc1d16"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-graph --lib hot_resume_sets_aside_a_torn_activity"
+++

## Problem

gap-dc1d16 made plan resume set aside a torn (half-written) last activity record via `replay::set_aside_uncommitted_activities`. Hot Graph resume (`roko-graph/src/hot.rs`) still fails closed on the same torn line, so a crash mid-write blocks a hot resume.

## Plan

Call `replay::set_aside_uncommitted_activities` from the hot resume path, and add a test named `hot_resume_sets_aside_a_torn_activity`.

## Done when

- `cargo test -p roko-graph --lib hot_resume_sets_aside_a_torn_activity` passes.

## Notes

- Reported on 2026-10-01 by wk-tamper, working on gap-dc1d16.
