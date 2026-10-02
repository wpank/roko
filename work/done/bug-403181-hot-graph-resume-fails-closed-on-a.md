+++
id = "bug-403181"
kind = "bug"
title = "Hot Graph resume fails closed on a torn last Activity line, which plan resume now sets aside"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-graph"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-dc1d16"
anchors = ["crates/roko-graph/src/hot.rs", "crates/roko-graph/src/replay.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-dc1d16"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-graph --lib hot_resume_sets_aside_a_torn_activity"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:11Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T17:36:58Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
