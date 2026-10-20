+++
id = "gap-c332f6"
kind = "gap"
title = "Dead learning modules remain after their callers went: roko-learn's holdout.rs, post_gate_reflection.rs, and retrieval_outcome.rs (which still feeds a serve route and a TUI view from a file nothing writes)"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-learn", "roko-serve/routes", "roko-cli/tui"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-42"
anchors = ["crates/roko-learn/src/holdout.rs", "crates/roko-learn/src/post_gate_reflection.rs::generate_post_gate_reflection", "crates/roko-learn/src/retrieval_outcome.rs", "crates/roko-serve/src/routes/neuro.rs", "crates/roko-cli/src/tui/views/learning_view.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["spec-9a3131", "q-3a1648"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test ! -e crates/roko-learn/src/holdout.rs && test ! -e crates/roko-learn/src/post_gate_reflection.rs && test ! -e crates/roko-learn/src/retrieval_outcome.rs && python3 tools/http_route_inventory.py --check-snapshot"
+++

## Problem

Three `roko-learn` modules outlived their callers (status-quo assessment 2026-10-15 at 7b6a5af44, finding 42;
`tmp/status-quo/2026-10-15/backlog/A05-41-42-loops-and-prompts.md` point 3, and rows 4101 and 4129):

- `crates/roko-learn/src/holdout.rs` (`HoldoutExperiment`): 4101 retired the legacy holdout from Graph runs
  (`learning_wiring_census.rs:328` asserts L-holdout retired); nothing uses the type.
- `crates/roko-learn/src/post_gate_reflection.rs::generate_post_gate_reflection`: only its own tests call it.
- `crates/roko-learn/src/retrieval_outcome.rs`: 4129 stopped writing `retrieval-outcomes.jsonl`, but
  `GET /api/retrieval/stats` (`crates/roko-serve/src/routes/neuro.rs`) and the TUI's learning view
  (`crates/roko-cli/src/tui/views/learning_view.rs`) still read it, so old workspaces show stale numbers and new ones
  show nothing.

## Why it matters

Goal `tooling`: CLAUDE.md's rule 1 ("never reimplement") sends agents searching for existing code; dead modules with
tests look alive and get wired back in by mistake.

## Where

The three files above, `crates/roko-learn/src/lib.rs` (module declarations), the serve route and the TUI view.

## Current state

None of the tasks that retired the callers required deleting the modules (A05). Note the different, live file
`crates/roko-learn/src/homeostasis/holdout.rs` (the M1 holdout arm, q-3a1648): keep it.

## Plan

1. Delete `holdout.rs` and `post_gate_reflection.rs` with their tests and `lib.rs` entries; fix doc mentions in
   `loop_audit/` that name them as live.
2. For retrieval stats: point the route and the TUI view at the exposure records that replaced the file (4129), or
   remove the route and the view; then delete `retrieval_outcome.rs`.
3. If the route changes, refresh the route snapshot (`python3 tools/http_route_inventory.py --refresh`) in the same
   commit.

## Done when

The `[[verify]]` command passes and the workspace builds and tests clean.

## Notes

Keep `crates/roko-learn/src/homeostasis/holdout.rs`. The census in `loop_audit/census.rs` may name the retired file
as history; that is fine. Parent: the hygiene epic spec-9a3131.
