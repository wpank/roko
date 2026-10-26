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
command = "grep -qw 'fn a_helper_call_after_a_failed_verify_leaves_the_operators_checkout_alone' crates/roko-cli/tests/worktree_task_diff.rs && cargo test -p roko-cli --test worktree_task_diff -- --include-ignored"

[[verify]]
command = "! grep -q 'ignore = \"bug-ee2752' crates/roko-cli/tests/worktree_task_diff.rs && grep -rqw 'fn cheap_agent_runs_its_provider_outside_the_operators_checkout' crates/roko-cli/src/ && cargo test -p roko-cli --lib cheap_agent_runs_its_provider_outside_the_operators_checkout"
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
  through `helper_agent`) and the audit's B3 reviewers (`audit_select.rs::audit_reviewers`). Each prompt carries the
  text the helper reads (failure output, diff, artefacts), so none needs a checkout.

## Current state

Re-checked 2026-10-26 with a binary of main built 10-21: the original fixture (the task's provider as the only
model), run by hand, logs its provider's working directories as the attempt worktree and then the repo root, and
leaves `?? feature.txt` in the repo root.

## Plan

Run helper calls in a scratch directory (or the attempt's checkout), never in the operator's checkout; or refuse
CLI providers as helper models. Then remove the `#[ignore]` from
`a_helper_call_after_a_failed_verify_leaves_the_operators_checkout_alone`, which passes as written.

Chosen: a scratch directory per call (`tempfile`, prefix `roko-helper-`, removed when the call ends), as the
audit's B1 suite author already does. The attempt's checkout is no answer: without `--worktree-per-task` it is the
operator's checkout, and a helper writing into a passing attempt's checkout would change what it delivers. Refusing
CLI providers would leave a Claude-Code-only setup with no error diagnosis. The call names the workspace as its
`immune_root`, so its immune state, its model-call learning row and its price snapshot stay with the workspace, as
a task attempt's do. A call that cannot get a scratch directory fails (callers fall back); it never falls back to
the checkout.

## Done when

`cargo test -p roko-cli --test worktree_task_diff -- --include-ignored` passes with a scratch `HOME`, and
`a_helper_call_after_a_failed_verify_leaves_the_operators_checkout_alone` is no longer ignored. The unit test
`cheap_agent_runs_its_provider_outside_the_operators_checkout` shows a fake Claude CLI helper running outside the
checkout, its scratch directory removed and the workspace still holding the call's efficiency row (both
`[[verify]]` commands).

## Notes

- 2026-10-26 (bug-c63cca): `worktree_task_that_fails_verify_keeps_its_worktree_and_merges_nothing` no longer shows
  this bug. To check gap-415c54's behaviour on any machine, its helper calls and `default_model` now go to a second
  model on the tests' mock agent, which writes nothing, and its task names its own model (`model_hint = "fake"`). This
  bug's check moved to the new `a_helper_call_after_a_failed_verify_leaves_the_operators_checkout_alone`: the original
  fixture, with the task's provider as the only model. It is `#[ignore]`d until this is fixed, and the verify runs it
  with `--include-ignored`. Reproduced by hand on 15335e2f9 with no keys and no `claude`. With the original fixture,
  the provider's second call runs in the repository root and leaves `?? feature.txt`. With the mock helper, that call
  goes to the mock (model `claude-haiku-4-5`), still with the operator's checkout as its working directory.
- A helper in a scratch directory no longer picks up the checkout's `.mcp.json` (Claude Code reads the working
  directory's), which matches "no tools, no MCP" in the adapter's doc.
