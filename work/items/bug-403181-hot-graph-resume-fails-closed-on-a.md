+++
id = "bug-403181"
kind = "bug"
title = "Hot Graph resume fails closed on a torn last Activity line, which plan resume now sets aside"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-graph"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "2293a5c63"
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
- 2026-10-01 (wk-tamper): implemented on work/gap-7147bb; cargo verification deferred to the batch check.
  `load_hot_checkpoint` (`hot.rs`) now calls `replay::set_aside_uncommitted_activities` after the manifest checks and
  before `load_scoped`. A torn last line moves to `activities.jsonl.uncommitted.<ms>` with a warning, and the resume
  replays the committed records. A corrupt line that does end in a newline still fails closed
  (`drift_and_corrupt_activity_logs_fail_closed`). Test: `hot_resume_sets_aside_a_torn_activity`.
