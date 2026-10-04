+++
id = "gap-1a8ee7"
kind = "gap"
title = "Homeostat replay's Evaluator has no CLI entry point; every R-H6 cell reports evaluated:false"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/homeostasis", "roko-cli/commands"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "8b51d004a"
source = "wave-10 follow-up reports 2026-10-04 (PK67 gap-ed1a08)"
discovered_from = "gap-ed1a08; checked against PK71's in-progress 8133 on work/gap-099513 at 3571b431a"
anchors = ["crates/roko-learn/src/homeostasis/replay.rs::Evaluator"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn homeostasis_replay_reports_the_evaluator_s_verdict' crates/roko-cli/ && cargo test -p roko-cli homeostasis_replay_reports_the_evaluator_s_verdict"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T09:31:24Z"
commit = "8b51d004a"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-04T07:57:41Z"
forced = false
evidence = "Gate 16a (merged 8b51d004a): verify homeostasis_replay_reports_the_evaluator_s_verdict passes; bench suite 742 passed. roko learn homeostasis replay --evaluate [--arm] [--seeds] runs Evaluator::evaluate over synthetic:<kind>@<t> or synthetic:all@<t> and writes ArmReport rows in the table shape replay_h6.py reads, so R-H6 cells with an iae and no note report evaluated:true (A3-gated and A3-mis wait for M3 predictions). Gate fixes 8d725969d and 5d0efe89f (the catalog's DisturbanceKind; a test-only import)."
+++

## Problem

The homeostat replay's evaluator (`Evaluator`, `crates/roko-learn/src/homeostasis/replay.rs:436,445`, built for
backlog task 8117) has no CLI or subprocess entry point anywhere — confirmed: zero references to it under
`crates/roko-cli/src/`. So every R-H6 cell that should report the evaluator's verdict reports `evaluated: false`
instead, regardless of how the controller itself performed.

PK71's own task 8133 (`roko learn homeostasis replay`) is the natural place this should be wired, but it isn't
there either: on its branch (`work/gap-099513`, checked at commit `3571b431a`), the command runs the controller
over a stream via `streams::replay_theta0` and never calls `Evaluator`. **Note: `work/gap-099513` (PK71) is
currently claimed/in progress** — this item does not edit that file; whoever is working it should see this
before merging, so 8133 lands wired to the evaluator rather than needing a follow-up fix.

Separately, X3 and X4 have no data behind them yet: no per-position lottery timeline exists, and no LOG1
gate-policy cells have been produced. (Not traced to a specific missing producer in this pass — flagged as
reported, for whoever picks this item up to locate precisely; may simply be a "hasn't been run yet" data gap
rather than a code defect.)

## Why it matters

Goal: cybernetic, M1 controller replay / S06 (backlog 8117/8133). Without the evaluator wired in, R-H6's whole
point — scoring the controller's replayed behavior against ground truth — can't produce a real verdict; every
cell reports `evaluated: false` by construction, not because the controller failed anything.

## Where

- `crates/roko-learn/src/homeostasis/replay.rs::Evaluator` (the unwired evaluator).
- `work/gap-099513` (PK71's branch for task 8133, `roko learn homeostasis replay`) — read-only reference; do not
  edit while it's claimed.
- X3/X4's data sources (lottery timeline, LOG1 gate-policy cells) — not yet located to a specific file.

## Current state

`Evaluator` exists and is unused anywhere in the CLI. 8133, in flight, doesn't call it either as of the checked
commit.

## Plan

1. Once 8133 merges (or as part of finishing it), wire `roko learn homeostasis replay` to call `Evaluator` and
   report its real verdict per cell, instead of the hardcoded `evaluated: false`.
2. Locate and produce the missing X3 (per-position lottery timeline) and X4 (LOG1 gate-policy cells) data, or
   confirm they're simply pending a benchmark run that hasn't happened yet.

## Done when

- `roko learn homeostasis replay` reports a real evaluator verdict per R-H6 cell.
- X3/X4 either have data, or are confirmed as pending a specific, named run.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK67's work (gap-ed1a08, done).
- 2026-10-04 (wave-13 follow-up): `work/gap-099513` (PK71) merged at `f15044a68`; `gap-099513` is now closed.
  `roko learn homeostasis replay` (`crates/roko-cli/src/commands/learn_homeostasis.rs::replay`) is real and
  shipped — it builds a `Controller` and runs it over the stream's resolutions — but still never constructs or
  calls `homeostasis::replay::Evaluator` (confirmed: zero matches for `Evaluator`/`evaluate` anywhere in that
  file at HEAD). This item's fix is no longer blocked on anyone's in-flight branch; it's a straightforward
  follow-up to wire the evaluator into the now-real command.
