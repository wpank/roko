+++
id = "gap-b95d94"
kind = "gap"
title = "Hindsight relabels miss dreams, extra playbooks and the router"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "M"
subsystem = ["roko-learn", "roko-dreams"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "f4323cf9d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-5be28d"
anchors = ["crates/roko-cli/src/runtime_feedback/hindsight.rs", "crates/roko-dreams/src/cycle.rs", "crates/roko-learn/src/cascade_router.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-5be28d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn hindsight_retracts_everywhere_playbooks_and_router' crates/roko-cli/src && grep -rqw 'fn hindsight_retracts_everywhere_in_dream_replay' crates/roko-dreams/src && cargo test -p roko-cli --lib hindsight_retracts_everywhere && cargo test -p roko-dreams --lib hindsight_retracts_everywhere"
+++

## Problem

gap-5be28d applies hindsight adjustments, but three readers still miss them. Dream replay reads hindsight-relabeled successes as passes, because DreamCycle has no adjustments path. Episodes keep only the first playbook id, so hindsight retracts only that playbook. The router has no retraction API.

## Plan

Give the dream cycle the adjustments, keep every playbook id per episode, and add a router retraction. Add a test named `hindsight_retracts_everywhere_*`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on gap-5be28d, during the evening close-out round.
- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e; cargo verification deferred to the batch check.
  - Dreams: `DreamCycle` finds the adjustments log beside the episode log (as it already finds its staging buffer), and
    its new `replay_episodes` applies it (`roko_learn::hindsight::apply_adjustments_from`). `run_budgeted` replays
    through it, so a blamed success replays as a failure. Test: roko-dreams `hindsight_retracts_everywhere_in_dream_replay`.
  - Playbooks: Graph episodes now record every injected playbook (`extra.playbook_ids`), and `HindsightSink` moves each
    one's success to a failure. It falls back to `extra.playbook_id` for older episodes.
  - Router: new `CascadeRouter::retract_success(model, category, confidence)` moves one credited success to a failure
    in the category stats and, unless an operator override took the credit, in the confidence stats. Episodes record
    the routing category (`extra.routing_category`, with the routing sink's fallback category when there is no context)
    and `extra.model_override`. A failed-over attempt earned no credit and is skipped. `plan_runner` hands the run's
    router to the sink (`HindsightSink::with_router`). Tests: roko-learn
    `retract_success_moves_a_credited_success_to_a_failure`; roko-cli `hindsight_retracts_everywhere_playbooks_and_router`.
  - Not corrected: the `LinUCB` bandit keeps its update, because the dispatch-time context features it folded in are
    not kept. The retraction is in memory until the run saves the router: unlike the routing sink's outcomes it is not
    journaled, so a crash before that save loses it while the journal replays the original success.
  - The verify now also runs the dreams test, with grep guards.
