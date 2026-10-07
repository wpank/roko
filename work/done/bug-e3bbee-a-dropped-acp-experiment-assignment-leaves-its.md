+++
id = "bug-e3bbee"
kind = "bug"
title = "A dropped ACP experiment assignment leaves its receipt Prepared forever, once per dispatch now"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn/prompt-experiment"]
created = 2026-10-04
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "19f76451c"
source = "wave-18 follow-up reports 2026-10-04 (bug-a3f005, work/bug-a3f005)"
discovered_from = "bug-a3f005 (open; own Progress note names this exact residual)"
anchors = ["crates/roko-acp/src/bridge_events/experiments.rs::applicable_acp_experiment", "crates/roko-learn/src/prompt_experiment.rs::ExperimentStore"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn dropped_acp_assignment_settles_as_abandoned' crates/roko-acp/ && cargo test -p roko-acp dropped_acp_assignment_settles_as_abandoned"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T15:52:40Z"
commit = "19f76451c"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-05T09:03:13Z"
forced = false
evidence = "Gate 19 (merged 19f76451c): verify passes (dropped_acp_assignment_settles_as_abandoned). applicable_acp_experiment settles a dropped assignment's receipt as Abandoned; the other open-receipt exits are bug-897879."
+++

## Problem

`bug-a3f005`'s fix (on `work/bug-a3f005`, not yet merged) made ACP prepare and serve the same
drawn variant from one durable receipt per dispatch, but a dispatch that
`applicable_acp_experiment` drops leaves its receipt `Prepared` forever, and this now happens
once per such dispatch rather than once per session.

`assign_acp_experiment` (`crates/roko-acp/src/bridge_events/experiments.rs:64-100`)
unconditionally calls `ExperimentStore::prepare_attempt_assignments` for every dispatch,
writing a durable receipt before anything decides whether the drawn variant will actually be
used. `applicable_acp_experiment` (lines 173-205) then filters: if the variant is a model
variant and `model_selection_explicit && resolve_model(...).model_key != candidate` (the
session's model was picked explicitly, and differs from the drawn variant's model), it returns
`(None, None)` — the assignment is dropped and never served — with no call back into the
receipt machinery. The receipt `prepare_attempt_assignments` already wrote stays in
`PromptAssignmentState::Prepared` indefinitely; nothing ever calls
`ExperimentStore::settle_attempt(path, attempt_key, AssignmentSettlement::Abandoned)` for it,
even though that exact settlement variant already exists and is used elsewhere in this file for
the analogous failed/skipped case (per the module's own comment table, "failed/skipped |
settle_attempt (Abandoned, no stats)").

Before `bug-a3f005`'s fix, every ACP dispatch of a session reused attempt 1
(`PromptAttemptKey::new(session_id, "acp", mode, 1)`), so a dropped assignment orphaned at most
one receipt per session. The fix gives each dispatch its own attempt number via
`ExperimentStore::next_attempt_for`, so the same drop-and-never-settle gap now orphans one
receipt *per dispatch* instead — the underlying bug isn't new, but its frequency scales with
dispatch count now, not session count.

## Why it matters

Severity p3. An orphaned `Prepared` receipt is dead weight in the experiment store (never
settled, never counted as an outcome, never cleaned up) — not incorrect data (it was correctly
never served), but an accumulating leak that grows with every explicit-model-override dispatch
of an active model experiment, and makes the store's own bookkeeping (counts of
prepared-vs-settled) permanently inaccurate for these experiments.

## Where

- `crates/roko-acp/src/bridge_events/experiments.rs::applicable_acp_experiment` (where the drop
  decision is made, with no settlement call).
- `crates/roko-acp/src/bridge_events/mod.rs:377` (the caller; has `config`/the assignment in
  scope, would need `path`/`attempt_key` threaded in too).
- `crates/roko-learn/src/prompt_experiment.rs::ExperimentStore::settle_attempt`,
  `AssignmentSettlement::Abandoned` (the existing mechanism to call).

## Current state

Confirmed on `work/bug-a3f005` (not yet merged): `applicable_acp_experiment` returns `(None,
None)` on a drop with no settlement call anywhere in the function or its one caller.

## Plan

1. Thread `path` and the dispatch's `attempt_key` into `applicable_acp_experiment` (or have its
   caller settle on its behalf when it returns `None` for an assignment that had one).
2. Call `ExperimentStore::settle_attempt(path, attempt_key, AssignmentSettlement::Abandoned)`
   when a prepared assignment is dropped.
3. Regression test: a dispatch whose drawn model variant is dropped because the session's model
   was explicitly selected settles its receipt as `Abandoned`, not left `Prepared`.

## Done when

- A dropped ACP experiment assignment's receipt settles as `Abandoned`, not left `Prepared`
  forever.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-18 follow-up, bug-a3f005, work/bug-a3f005 not yet merged): confirmed
  directly on the branch. `bug-a3f005`'s own Progress note names this exact residual: "Left as
  it was: a dispatch that `applicable_acp_experiment` drops... leaves its receipt Prepared.
  That now happens once per such dispatch, not once per session." Filed separately since that
  item's own `[[verify]]` command covers only the serve/settle-match fix, not this.

## Progress

- 2026-10-05 (w4-length): implemented on `work/gap-d10a97` at cf65e7802; cargo verification deferred to the
  batch gate. `applicable_acp_experiment` now takes the experiment store's path. Both of its drops, a model
  variant whose model is not configured and one that would override a model the session selected explicitly,
  settle the receipt as `Abandoned` (the new `abandon_acp_experiment`; no trial counts). Test:
  `dropped_acp_assignment_settles_as_abandoned` covers both drop reasons.
- Not in this item's scope, for the filer: other early returns between preparation and settlement in
  `bridge_events/mod.rs` leave receipts open too. The no-usable-provider return (~l.789) and the pre-dispatch
  safety violation (~l.706) come after `mark_acp_experiment_dispatched`, so their receipt stays `Dispatched`. The
  image-validation errors (~l.465, l.574) leave it `Prepared`.
