+++
id = "gap-a47d07"
kind = "gap"
title = "Router commit's held-out set always falls back to the last 20%, never M2's holdout arm"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "517ab9d19"
source = "wave-13 follow-up reports 2026-10-04 (PK71 gap-099513, task 8136)"
discovered_from = "gap-099513 (closed; own done-note at line 124 already flagged this as a known gap)"
anchors = ["crates/roko-learn/src/router_commit.rs::held_out_attempts", "crates/roko-cli/src/graph_execution/learning_commit.rs::settled_outcomes"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn router_commit_scores_against_m2_holdout_arm' crates/ && cargo test -p roko-learn router_commit_scores_against_m2_holdout_arm"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T07:56:43Z"
commit = "517ab9d19"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-04T06:53:59Z"
forced = false
evidence = "Gate 15a (merged 517ab9d19): roko-learn and roko-cli lib tests pass; verify router_commit_scores_against_m2_holdout_arm passes. router_commit::settled_outcomes (now in roko-learn) marks an attempt holdout when its route decision drew M2's holdout arm (π⁰), so the router commit's held-out check scores against it instead of the run's last 20%."
+++

## Problem

The router commit's held-out set never uses M2's holdout arm; it always falls back to the run's
last 20% of attempts. `held_out_attempts` (`crates/roko-learn/src/router_commit.rs:71-82`) is
documented to prefer "those on M2's holdout arm, else the run's last 20%," and filters on a
per-attempt `SettledOutcome.holdout: bool` field (`router_commit.rs:67`) to find them. But the
only production code that builds `SettledOutcome`s — `settled_outcomes` in
`crates/roko-cli/src/graph_execution/learning_commit.rs:166-187` — hardcodes `holdout: false`
unconditionally for every attempt (line 184), reading nothing from the run's records that could
mark an attempt as having run on M2's holdout arm (`HarnessHoldout`,
`crates/roko-learn/src/homeostasis/holdout.rs`, which this function never touches). So the
`.filter(|attempt| attempt.holdout)` at `router_commit.rs:76-78` always yields an empty vec in
practice, and every router commit check falls through to the last-20%-of-attempts branch — the
"M2 holdout arm" path is reachable in the type signature but dead in every real call.

## Why it matters

Goal: cybernetic, M1/M2 integration, router commit guard (backlog 8136/8139-adjacent). The
whole reason for preferring M2's holdout arm over a plain last-20% split is that the holdout arm
is held out *by design* (never used to fit the candidate), while "the last 20% of attempts" can
still overlap with what trained the candidate being checked — weakening exactly the guarantee
the held-out check exists to provide (candidate no worse than LKG on data it didn't see). This
was flagged and accepted as a known gap when 8138 shipped (`work/done/gap-099513-...md:124`:
"held-out attempts are the plans' last 20% (M2's holdout arm is not identified yet)"), but no
follow-up item tracks closing it.

## Where

- `crates/roko-learn/src/router_commit.rs::held_out_attempts`, `::SettledOutcome`.
- `crates/roko-cli/src/graph_execution/learning_commit.rs::settled_outcomes` (the one real
  constructor, always `holdout: false`).
- `crates/roko-learn/src/homeostasis/holdout.rs::HarnessHoldout` (M2's holdout arm — exists,
  but `settled_outcomes` never consults it).

## Current state

`SettledOutcome.holdout` is a real field with real filter logic in `held_out_attempts`, but it
is wired to a constant in the only place that matters. The mechanism looks connected end to end
but every router commit check runs on the last-20% fallback, never on M2's actual holdout arm.

## Plan

1. Give `settled_outcomes` (or its caller) a way to know, per settled attempt, whether that
   attempt ran on M2's holdout arm — likely by reading a flag already recorded on the run's
   `RunRecords`/verdict (if `HarnessHoldout` already tags attempts there) or by cross-
   referencing the run's holdout assignment at dispatch time.
2. Set `SettledOutcome.holdout` from that real signal instead of the literal `false`.
3. Add a regression test with at least one attempt marked as holdout-arm, and assert
   `held_out_attempts` returns the holdout subset rather than the last-20% fallback.

## Done when

- A run with attempts on M2's holdout arm has its router commit check scored against that arm,
  not the last-20% fallback.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-13 follow-up, PK71 8136): confirmed at main HEAD `b7ad508ce`. Verified by
  reading `held_out_attempts` and `SettledOutcome` in full, then grepping every non-test
  `SettledOutcome {` construction and every non-test `holdout:` field assignment across
  `crates/` — `learning_commit.rs:166-187` is the only production constructor, and it is
  unconditional `holdout: false`. Matches the gap already called out in `gap-099513`'s own
  done-note at closure.
