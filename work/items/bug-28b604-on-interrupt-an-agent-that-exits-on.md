+++
id = "bug-28b604"
kind = "bug"
title = "On interrupt, an agent that exits on SIGTERM within the drain settles as ProviderError and TaskExecutorCell retries it while the run is stopping"
status = "open"
triage = "unverified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-graph"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on bug-2b1ddc)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-graph/src/cells/task_executor.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-2b1ddc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_sigterm_exit_during_the_drain_settles_as_cancelled' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_sigterm_exit_during_the_drain_settles_as_cancelled"
+++

## Problem

bug-2b1ddc made a cancelled attempt settle as Cancelled and stopped TaskExecutorCell retrying RokoError::Cancelled. But an agent that exits on its own SIGTERM within the drain settles as ProviderError (counted against provider health), and TaskExecutorCell then retries it, launching a new agent while the run is stopping (it runs until the 3 s stop).

## Why it matters

Provider health and the router learn from a shutdown, and a stopping run starts new agents.

## Plan

Treat an exit caused by the run's own SIGTERM (or any exit after the stop began) as Cancelled; no retry while stopping.

## Done when

- [ ] A SIGTERM exit during the drain settles as Cancelled and is not retried
- [ ] The `[[verify]]` command passes.
