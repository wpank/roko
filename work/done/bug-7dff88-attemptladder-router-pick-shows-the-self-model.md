+++
id = "bug-7dff88"
kind = "bug"
title = "AttemptLadder.router_pick shows the self-model's pick under the router's name on L-M3 rows"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-04
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "19f76451c"
source = "wave-17b follow-up reports 2026-10-04 (bug-78e5ce, work/backlog-batch-17b)"
discovered_from = "bug-78e5ce (open; own Progress note names this exact fix, facet 1 of 3)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/ladder.rs::record_attempt_ladder", "crates/roko-cli/src/dispatch/model_routing.rs::SELF_MODEL_LOOP"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn router_pick_does_not_show_the_self_models_pick_on_l_m3_rows' crates/roko-cli/ && cargo test -p roko-cli router_pick_does_not_show_the_self_models_pick_on_l_m3_rows"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T15:52:34Z"
commit = "19f76451c"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-05T09:03:04Z"
forced = false
evidence = "Gate 19 (merged 19f76451c): verify passes (router_pick_does_not_show_the_self_models_pick_on_l_m3_rows). router_pick skips the learned proposal on L-M3 rows; SELF_MODEL_LOOP is pub(crate)."
+++

## Problem

`AttemptLadder.router_pick` still shows the self-model's pick under the router's name — the
first of `bug-78e5ce`'s three facets, confirmed not done on `work/backlog-batch-17b`:
`crates/roko-cli/src/graph_task_dispatch/ladder.rs:227` still has `router_pick:
plan....and_then(|decision| decision.proposals.learned.clone())`, unconditional on the route
row's loop. When M3 is active and picks the start rung (an L-M3 route decision), it writes its
own pick into that same `proposals.learned` field the cascade router's shadow pick normally
occupies, so `router_pick` reads as the self-model's choice under the router's name on those
rows, with no way for a reader to tell the two apart.

`bug-78e5ce`'s own Progress note already spells out the fix: "in `ladder.rs::record_attempt_ladder`,
take `router_pick` from the route decision only when its `audit.loop_id` is not L-M3
(`SELF_MODEL_LOOP` in `dispatch/model_routing.rs`, which would need to become `pub(crate)`),
since L-M3 rows carry the self-model's pick (a^L) as `proposals.learned`." Confirmed:
`SELF_MODEL_LOOP: &str = "L-M3"` (`crates/roko-cli/src/dispatch/model_routing.rs:1240`) is a
private `const` today, and `row.audit.loop_id`/`row.audit.loop_ids` are set to it at lines
1389-1390 — `ladder.rs` is a different module and can't reference it without the visibility
change.

## Why it matters

Goal: cybernetic, M3 self-model (S04), same goal as `bug-78e5ce`. `router_pick` is exactly the
field anyone debugging routing would check to see the cascade's own shadow pick versus the
active choice; on an L-M3 row it currently shows something else entirely under that name, which
is most misleading exactly when M3's influence is what's being investigated.

## Where

- `crates/roko-cli/src/graph_task_dispatch/ladder.rs::record_attempt_ladder` (line 227, the fix
  site).
- `crates/roko-cli/src/dispatch/model_routing.rs::SELF_MODEL_LOOP` (line 1240; needs `pub(crate)`).

## Current state

Unfixed, confirmed on `work/backlog-batch-17b` (tip `00a28ea0c`): `router_pick`'s sourcing is
unchanged from `bug-78e5ce`'s original report.

## Plan

1. Make `SELF_MODEL_LOOP` `pub(crate)` in `dispatch/model_routing.rs`.
2. In `record_attempt_ladder`, source `router_pick` from `decision.proposals.learned` only when
   the route decision's `audit.loop_id` is not `SELF_MODEL_LOOP`; `None` (or a distinctly-named
   field) on L-M3 rows.
3. Regression test: an L-M3 route decision's ladder row has no `router_pick` claiming the
   self-model's choice (or carries it under a clearly distinct name).

## Done when

- `router_pick` never shows the self-model's pick under the router's name.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-17b follow-up, bug-78e5ce, work/backlog-batch-17b not yet merged): confirmed
  directly against the branch. `bug-78e5ce` itself is still open (its other two facets are done
  or resolved-as-non-issue on this branch) and already names this exact fix in its own Progress
  note; filed separately in case that item closes on its single named `[[verify]]` command
  (which covers facet 2 only) before this facet lands. A matching note has been added to
  `bug-78e5ce`.

## Progress

- bug-7dff88: implemented at 55f736563 on `work/bug-7dff88`; cargo verification deferred to the batch gate.
  `ladder.rs::router_pick` returns the route row's learned proposal except on rows whose `audit.loop_id` is
  `SELF_MODEL_LOOP` (now `pub(crate)` in `dispatch/model_routing.rs`). L-M3 rows carry the self-model's pick
  there; the prediction row and the route row keep it. Test: `router_pick_does_not_show_the_self_models_pick_on_l_m3_rows`.
