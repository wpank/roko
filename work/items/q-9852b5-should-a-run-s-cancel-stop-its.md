+++
id = "q-9852b5"
kind = "question"
title = "Should a run's cancel stop its gate commands now that they join the agent PID registry?"
status = "open"
triage = "unverified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-b367bf"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::live_agent_process_trees", "crates/roko-gate/src/cancel_safe_command.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-b367bf"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -n 'register_spawned_pid' crates/roko-gate/src/cancel_safe_command.rs"
+++

## Problem

With gap-b367bf, gate commands register in roko_agent's PID registry, so find-65ff6b's cancel (which stops a run's registered agent processes, scoped per serve run) now also stops that run's gate commands. That is probably the desired behaviour, but it is untested, and an unscoped CLI run owns every unscoped process.

## Plan

Confirm the intended behaviour, then add a test that a run's cancel stops its own gate command and leaves another run's alone.

## Done when

- A decision is recorded, and the test exists.

## Notes

- Reported on 2026-10-01 by the worker on gap-b367bf, during the evening close-out round.
