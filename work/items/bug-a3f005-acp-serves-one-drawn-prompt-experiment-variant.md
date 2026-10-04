+++
id = "bug-a3f005"
kind = "bug"
title = "ACP serves one drawn prompt-experiment variant but settles its receipt against a differently-drawn one"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-acp", "roko-learn/prompt-experiment"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-7 follow-up reports 2026-10-03 (PK42 gap-2b3c1b)"
discovered_from = "gap-2b3c1b"
anchors = ["crates/roko-acp/src/bridge_events/experiments.rs::assign_acp_experiment", "crates/roko-learn/src/prompt_experiment.rs::prepare_attempt_assignments_unlocked"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn acp_settles_the_same_variant_it_served' crates/roko-acp/ && cargo test -p roko-acp acp_settles_the_same_variant_it_served"
+++

## Problem

ACP's prompt-experiment assignment (`crates/roko-acp/src/bridge_events/experiments.rs::assign_acp_experiment`)
draws a variant through `experiment.assign_variant()` (no arguments) at line 76, and serves that variant's
content to the user (`variant.content`/`variant.slug` end up in the returned `AcpExperimentAssignment`). It then
separately calls `ExperimentStore::prepare_attempt_assignments` (line ~81) to create a durable receipt for
settlement — but that function's inner `prepare_attempt_assignments_unlocked`
(`crates/roko-learn/src/prompt_experiment.rs:1165`) draws its own variant via `experiment.draw_variant(attempt_key)`
(~line 1211), a **different** method from `assign_variant()`, keyed deterministically by `attempt_key` rather than
whatever stateful draw `assign_variant()` performs. So the receipt that gets read back at settlement time can name
a different variant than the one actually served to the user in that ACP turn.

## Why it matters

Goal: learning (prompt-experiment validity). An experiment's whole point is attributing outcomes to the variant
the user actually experienced. If settlement sometimes reads a different (re-drawn) variant than what was served,
every such mismatch dilutes the measured treatment effect — it adds noise that looks like "no difference between
variants" even when a real difference exists, biasing experiment conclusions toward the null.

## Where

- `crates/roko-acp/src/bridge_events/experiments.rs::assign_acp_experiment` (both calls).
- `crates/roko-learn/src/prompt_experiment.rs::Experiment::assign_variant` (what's actually served) vs.
  `::draw_variant` (what `prepare_attempt_assignments_unlocked` uses for the receipt, ~line 1211).

## Current state

Unfixed. Confirmed both methods exist and are genuinely different draw mechanisms, called independently for the
same ACP turn.

## Plan

1. Have `assign_acp_experiment` prepare the receipt from the **already-drawn** variant (pass it into
   `prepare_attempt_assignments`/thread it through to `prepare_attempt_assignments_unlocked` as the chosen
   variant for this section, rather than letting that function independently re-draw via `draw_variant`), so ACP
   serves and settles the same variant.
2. Add a regression test: an ACP assignment's served `variant_id` matches the variant id the resulting receipt
   settles against, across many draws (not just one, since the bug is about draw mechanisms disagreeing, which a
   single lucky draw could mask).

## Done when

- The variant ACP serves and the variant its receipt settles against are always the same one.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK42's work on the loop state machine (gap-2b3c1b, done).

## Progress

- 2026-10-04 (w4-length): implemented on `work/bug-a3f005` at c12e7e12b; cargo verification deferred to the
  batch gate.
  - Premise, refined: `assign_variant()` is not a second random draw. For a running experiment it always returns
    the control, its first active variant, so ACP always served the control while the receipt drew uniformly from
    the attempt key.
  - The receipt's draw is now the only one. `assign_acp_experiment` prepares first, then serves the receipt's
    experiment and variant with the content the receipt snapshotted, so it marks dispatched and settles that same
    variant. With no receipt (preparation failed, or prepared nothing for the section), ACP serves the control and
    takes the legacy direct-record path. Before, an empty preparation kept an attempt key with no ids, and its
    outcome was lost at settlement (`AttemptNotFound`).
  - Also fixed, since it broke "one draw per dispatch": every ACP dispatch used attempt 1 of its session and mode,
    so a session's later turns got the first turn's settled receipt, and the conflicting `mark_attempt_dispatched`
    dropped their outcomes. The new `ExperimentStore::next_attempt_for(run, plan, task)` numbers each dispatch
    after the last attempt prepared for it.
  - Tests: `acp_settles_the_same_variant_it_served` (40 dispatches of one session over four variants: every served
    variant equals its receipt's settled variant, every attempt is distinct, the draws vary, and 40 trials
    count) and `next_attempt_for_follows_the_last_prepared_attempt` (roko-learn).
  - Left as it was: a dispatch that `applicable_acp_experiment` drops (a model variant beside an explicitly
    selected model) leaves its receipt Prepared. That now happens once per such dispatch, not once per session.
