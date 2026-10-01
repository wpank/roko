+++
id = "gap-b95d94"
kind = "gap"
title = "Hindsight relabels miss dreams, extra playbooks and the router"
status = "open"
triage = "unverified"
severity = "p3"
goal = "learning"
size = "M"
subsystem = ["roko-learn", "roko-dreams"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-5be28d"
anchors = ["crates/roko-cli/src/runtime_feedback/hindsight.rs", "crates/roko-dreams/src/cycle.rs", "crates/roko-learn/src/cascade_router.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-5be28d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib hindsight_retracts_everywhere"
+++

## Problem

gap-5be28d applies hindsight adjustments, but three readers still miss them. Dream replay reads hindsight-relabeled successes as passes, because DreamCycle has no adjustments path. Episodes keep only the first playbook id, so hindsight retracts only that playbook. The router has no retraction API.

## Plan

Give the dream cycle the adjustments, keep every playbook id per episode, and add a router retraction. Add a test named `hindsight_retracts_everywhere_*`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on gap-5be28d, during the evening close-out round.
