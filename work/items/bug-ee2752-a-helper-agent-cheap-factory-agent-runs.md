+++
id = "bug-ee2752"
kind = "bug"
title = "A helper agent (cheap-factory-agent) runs with the operator's checkout as its working directory, so a CLI helper model can write there"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v5/findings/roko-delivery.md (roko-delivery, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: testing bug-74164c"
anchors = ["crates/roko-cli/src/graph_task_dispatch/routing_context.rs::CheapFactoryAgent", "crates/roko-cli/src/graph_task_dispatch.rs::cheap_agent", "crates/roko-cli/tests/worktree_task_diff.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --test worktree_task_diff worktree_task_that_fails_verify_keeps_its_worktree_and_merges_nothing"
+++

## Problem

`worktree_task_that_fails_verify_keeps_its_worktree_and_merges_nothing` (`crates/roko-cli/tests/worktree_task_diff.rs`)
fails on main a0c7d0982 even with a scratch `HOME`: the operator's checkout ends with an untracked `feature.txt`.

After the task's verify step fails, a helper call (`CheapFactoryAgent`, agent id `cheap-factory-agent`, model `fake`)
runs the workspace's only provider, the fixture's `claude_cli` fake, with `workdir` = the operator's checkout. That
fake writes `feature.txt` in its working directory whatever it is asked, so the helper call leaves a file in the
operator's checkout. Reproduced by hand: `roko --json plan run plans/proof --worktree-per-task --promote release
--no-tui` in the fixture leaves `?? feature.txt` in the repo root; its log shows `agent started
agent=cheap-factory-agent` right after the failed attempt.

## Why it matters

The run's promise is that it never changes the operator's checkout. Helper calls (error enrichment, gate
reflections, the judge) run a provider with the operator's checkout as their working directory. With a CLI provider
(Claude Code, Codex) as the cheap helper model, that agent has its own tools there; only its permission mode stands
between it and the person's files.

## Where

- `crates/roko-cli/src/graph_task_dispatch/routing_context.rs::CheapFactoryAgent` (`workdir: self.workdir`).
- `crates/roko-cli/src/graph_task_dispatch.rs::cheap_agent`.

## Plan

Run helper calls in a scratch directory (or the attempt's checkout), never in the operator's checkout; or refuse
CLI providers as helper models. Then the test passes as written.

## Done when

`cargo test -p roko-cli --test worktree_task_diff` passes with a scratch `HOME`.
