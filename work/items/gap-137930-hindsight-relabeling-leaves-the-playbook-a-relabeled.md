+++
id = "gap-137930"
kind = "gap"
title = "Hindsight relabeling leaves the playbook a relabeled pass created or credited: retract the producer's write too"
status = "open"
triage = "unverified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/runtime_feedback"]
created = 2026-10-16
updated = 2026-10-16
source = "gap-89f985 worker (work-batch 2026-10-16 1010)"
discovered_from = "gap-89f985"
anchors = ["crates/roko-cli/src/runtime_feedback/hindsight.rs::HindsightSink", "crates/roko-cli/src/runtime_feedback/verified_playbooks.rs::VerifiedPlaybookSink"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-89f985", "spec-7a71d6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn hindsight_retracts_the_playbook_a_relabeled_pass_wrote' crates/roko-cli/ && cargo test -p roko-cli hindsight_retracts_the_playbook_a_relabeled_pass_wrote"
+++

## Problem

A Graph run's verified pass now writes L-play's playbooks (gap-89f985): the `verified_playbooks` sink creates a
playbook from the attempt's tool calls, or credits the stored playbook they repeat. When a later verify failure blames
the task (`verify: blocked_by_sibling = <task>`), `HindsightSink` relabels that success and retracts the credit it
earned at completion: the playbooks its episode lists in `extra.playbook_ids` (those its prompt included, which
dispatch credited, W07) and the router's credit. A playbook the relabeled pass created, or credited through the
producer, keeps its success: the episode does not name it, because the producer runs after the episode is written and
records its write only as a `playbook_written` event.

## Why it matters

Goal `learning`. A playbook's success count breaks ties in prompt ranking and is shown in prompts ("successes N,
failures M"); a playbook that a pass blamed for breaking a sibling's gate keeps a success it did not earn, and the
next prompt on its topic may recommend that procedure. Related: gap-89f985 (the producer), spec-7a71d6 (stage 1).

## Where

- `crates/roko-cli/src/runtime_feedback/hindsight.rs::HindsightSink` (`retract_credit` reads `episode_playbook_ids`).
- `crates/roko-cli/src/runtime_feedback/verified_playbooks.rs::VerifiedPlaybookSink` (creates or credits a playbook
  on `TaskVerified`, after `EpisodeSink` has written the attempt's episode).

## Current state

Hindsight retracts only the credits named on the relabeled episode. The producer's writes are keyed by plan and task
in the `playbook_written` event (`.roko/events.jsonl`), not by attempt key, so hindsight cannot find them.

## Plan

Options:

1. Record each producer write per attempt (a row keyed by the attempt key, e.g. under `.roko/learn/`) and have
   hindsight retract it: a playbook the pass created loses that success (moved to a failure, as
   `PlaybookStore::relabel_success_as_failure` does), and a producer credit moves to a failure.
2. Create playbooks only when the plan closes (`PlanCompleted`), after sibling blame has settled.

Option 1 keeps the producer per attempt and reuses hindsight's retraction path.

## Done when

The `[[verify]]` command passes: a verified pass that wrote a playbook and is later relabeled by a sibling-blamed
verify failure leaves that playbook with the success moved to a failure.

## Notes

- Check whether verified knowledge has the same shape (a relabeled pass keeps the lesson it admitted) before choosing;
  one per-attempt write record could serve both.
- Found while doing gap-89f985 (2026-10-16).
