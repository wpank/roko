+++
id = "find-96f541"
kind = "finding"
title = "On easy tasks every agent answers Lesson: none, so plan runs write almost no knowledge and the knowledge loop has little to learn from"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "M"
subsystem = ["roko-cli/runtime-feedback", "roko-neuro"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-50"
anchors = ["crates/roko-cli/src/runtime_feedback/verified_knowledge.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

In agent H's live run (2026-10-15, HEAD 7b6a5af44), all five agents of run 1 ended with `Lesson: none`, so the run
wrote no knowledge. Over the four runs, the durable store got 3 entries: two change notes ("The repository now
includes a `unique_slug` function ...") and one real lesson. R3 (2026-10-02) wrote 8 generic notes. The lesson prompt
(task 4216) works as designed: it asks for a lesson and accepts "none". On small, easy tasks there is nothing new to
say, so the run-to-run knowledge path, which works at HEAD (an entry from run 2 was retrieved in run 3 and injected in
run 4, behind the holdout arm), carries almost nothing.

Related observation, not filed elsewhere (H, N7): stored entries rise to `consolidated` (confidence 0.904) through
retrieval-time reinforcement ("retrieved 3, gated 3, cited 0"), without being cited, and one confirmation records the
Roman-numeral entry as confirming an unrelated slug entry.

## Why it matters

Goal `learning`: the knowledge loop is one of the papers' feedback loops. If easy work writes nothing and change
notes pass as lessons, the store grows slowly and its confidence measures retrieval, not usefulness.

## Where

- `crates/roko-cli/src/runtime_feedback/verified_knowledge.rs`: turns gate-verified attempts into durable knowledge.
- The lesson prompt (4216) and the retrieval reinforcement in `crates/roko-neuro/`.

## Current state

The path is wired end to end and gated by verification. What flows is thin and partly mislabelled.

## Plan

1. Decide what a lesson is for: keep "none" as an honest answer, and stop storing change notes (a lesson must say
   something that applies beyond this diff), possibly with a cheap filter at admission.
2. Separately, make confidence rise on citation or use, not on retrieval next to a pass (N7). That part can become a
   bug with a test (`retrieved_but_uncited_entry_gains_no_confidence`) if the coordinator files it.
3. Re-measure on the pilot runs, whose tasks are harder.

## Done when

A decision on (1) is recorded here, and (2) is filed as its own bug or rejected with a reason.

## Notes

Evidence: `tmp/status-quo/2026-10-15/live/H-live-run.md:322-338` (learning-data table and the paragraph after it),
`:352` (N7). No verify yet: this is an observation.
