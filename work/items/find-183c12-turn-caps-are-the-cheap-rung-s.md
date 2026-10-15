+++
id = "find-183c12"
kind = "finding"
title = "Turn caps are the cheap rung's main failure and cost 40% of the live run's spend; each capped attempt then passed with a raised cap"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-49"
anchors = ["crates/roko-cli/src/graph_task_dispatch/ladder.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["find-2ba86d", "gap-f30b8e"], supersedes = [], duplicate_of = "" }
+++

## Problem

In agent H's capped live run (2026-10-15, HEAD 7b6a5af44), 3 of 13 gpt-oss-120b attempts on the cheap rung hit
their turn caps: `roman-range` and `wrap` at 25, and the trivial `slug-initials` at 20. Those three attempts cost
$0.343, 40% of all recorded spend ($0.862). Each task then passed on its next attempt, with the cap raised ("previous
attempt hit its turn cap; resuming with a raised cap previous_cap=25 max_turns=38"). R3 (2026-10-02) saw the same
thing, with 7 caps. Cerebras bills cached input at the full input rate, so every extra turn re-pays the whole
context: gpt-oss-120b used 2.15M prompt tokens, 1.85M of them cached, in 203 calls.

A turn-cap retry resumes in the same worktree on purpose ("Its edits are still in the working tree"). Decision 3
(2026-10-15) confirmed this as written (D11). It also carried `wrap`'s gamed approach from attempt 1 into attempt 2,
which only had to remove the forbidden word (finding 45, find-2ba86d).

## Why it matters

Goal `golden-path`: "cheap models execute" is a cost claim, and the cheap rung's main failure mode is running out of
turns rather than wrong work. Whether the cap is too low for this model, or the model loops, decides the fix. One
small run can't tell them apart.

## Where

- Turn caps per task kind (20/25/30 for mechanical/focused/integrative in H's config) and the raised-cap retry in
  `crates/roko-cli/src/graph_task_dispatch/` (the retry message above; the ladder is in `ladder.rs`).

## Current state

The capped attempts are priced and recorded, and turn caps are not counted as provider failures. No error pattern
is learned from a turn cap (4208).

## Plan

1. Measure first: over the gap-f30b8e and pilot runs (decision 1 and 5 spend), report turn-cap rate, turns used and
   cost by rung, model and task kind.
2. Then pick one: a per-model cap factor in the ladder config (raise gpt-oss-120b's start cap), a cap learned from
   that model's verdict history for the task kind, or no-progress detection that stops a looping attempt early. The
   choice depends on whether capped attempts were making progress, which the proxy logs can show.
3. Turn this finding into a gap with a verify once the measurement picks the mechanism.

## Done when

The measurement is recorded on this item and a follow-up gap with a verify exists, or the item is closed with the
reason the caps are right as they are.

## Notes

Evidence: `tmp/status-quo/2026-10-15/live/H-live-run.md:179-181`, `:303-308` (cost), `:355` (N10) and `:271` (D11).
No verify yet: this is an observation that needs data before a fix.
