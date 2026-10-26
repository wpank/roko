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
updated = 2026-10-26
last_verified = 2026-10-26
source = "tmp/hermes-roko-v5/findings/roko-delivery.md (roko-delivery, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: testing bug-74164c"
anchors = ["crates/roko-cli/src/graph_task_dispatch/routing_context.rs::CheapFactoryAgent", "crates/roko-cli/src/graph_task_dispatch.rs::cheap_agent", "crates/roko-cli/tests/worktree_task_diff.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --test worktree_task_diff worktree_task_that_fails_verify_keeps_its_worktree_and_merges_nothing"

[[verify]]
command = "grep -rqw 'fn cheap_agent_runs_its_provider_outside_the_operators_checkout' crates/roko-cli/src/ && cargo test -p roko-cli --lib cheap_agent_runs_its_provider_outside_the_operators_checkout"
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
- Every helper call goes through `CheapFactoryAgent`: the error diagnosis after a failed verify step
  (`verification.rs`), the LLM-judge step (`judge_step.rs`), the evidence judge of a pack rung (`pack_rungs.rs`,
  through `helper_agent`) and the audit's B3 reviewers (`audit_select.rs::audit_phase_b`). Each prompt carries the
  text the helper reads (failure output, diff, artefacts), so none needs a checkout.

## Current state

Re-checked 2026-10-26 with the prebuilt binary of main (built 10-21): the fixture of the test above, run by hand,
logs its provider's working directories as the attempt worktree and then the repo root, and leaves `?? feature.txt`
in the repo root.

## Plan

Run helper calls in a scratch directory (or the attempt's checkout), never in the operator's checkout; or refuse
CLI providers as helper models. Then the test passes as written.

Chosen: a scratch directory per call (`tempfile`, prefix `roko-helper-`, removed when the call ends), as the
audit's B1 suite author already does. The attempt's checkout is no answer: without `--worktree-per-task` it is the
operator's checkout, and a helper writing into a passing attempt's checkout would change what it delivers. Refusing
CLI providers would leave a Claude-Code-only setup with no error diagnosis. The call names the workspace as its
`immune_root`, so its immune state, its model-call learning row and its price snapshot stay with the workspace, as
a task attempt's do. A call that cannot get a scratch directory fails (callers fall back); it never falls back to
the checkout.

## Done when

`cargo test -p roko-cli --test worktree_task_diff` passes with a scratch `HOME`, and the unit test
`cheap_agent_runs_its_provider_outside_the_operators_checkout` shows a fake Claude CLI helper running outside the
checkout, its scratch directory removed and the workspace still holding the call's efficiency row (both
`[[verify]]` commands).

## Notes

- A helper in a scratch directory no longer picks up the checkout's `.mcp.json` (Claude Code reads the working
  directory's), which matches "no tools, no MCP" in the adapter's doc.
