+++
id = "gap-634954"
kind = "gap"
title = "Set the tier size limits from a recorded sizing report (at least 20 verified first tries per bucket)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
hold = "waits on real run data: at least 20 verified first tries per tier and size bucket (decision 3203)"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "tmp/backlog/2026-10-02-complete-and-wire 3227 (blocked in wave 5, PK19)"
anchors = ["crates/roko-cli/src/plan_policy.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'roko learn sizing' crates/roko-cli/src/plan_policy.rs && grep -rqw 'fn tier_size_limits_match_the_recorded_sizing_report' crates/roko-cli/src/ && cargo test -p roko-cli --lib tier_size_limits_match_the_recorded_sizing_report"
+++

## Problem

Task 3227 of PK19 (gap-de0b87, merged in gate 5a) sets the generator's tier size limits from what tasks of each size
actually did: `roko learn sizing` (3226) reports, per tier and size bucket, how often a first attempt verified.
Decision 3203 needs at least 20 verified first tries per bucket before the limits move. The workspace has no attempt
records yet, so the calibration can't run.

## Why it matters

The size limits are the planner's guard against tasks too big for a cheap executor (S07). Set from guesses, they
either over-split plans (more planner calls) or let cheap models drown.

## Where

`crates/roko-cli/src/plan_policy.rs` (the tier size limits), `crates/roko-cli/src/commands/learn_sizing.rs` (3226).

## Current state

At the gate-5a merge the limits are the backlog's defaults; `roko learn sizing` exists and reports nothing until
attempts are recorded.

## Plan

1. After enough real plan runs (LOG1, the pilots, or dogfood), run `roko learn sizing` and check each bucket has at
   least 20 verified first tries.
2. Set the limits from the report as task 3227 says, and record the report's numbers in the commit message.
3. Write `tier_size_limits_match_the_recorded_sizing_report`.

## Done when

- [ ] The limits match a recorded sizing report with at least 20 verified first tries per bucket.
- [ ] The `[[verify]]` command passes.

## Notes

- Full spec: `tmp/backlog/2026-10-02-complete-and-wire/3227-*.md`.
