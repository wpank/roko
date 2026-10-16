+++
id = "gap-ca1c92"
kind = "gap"
title = "An agent that runs its own loop (Claude, Codex, Gemini, Cursor CLIs) can still spend past a run's cap within one attempt"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/budget"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v5/findings/roko-caps.md (roko-caps, 2026-10-16)"
discovered_from = "bug-4fabd0: the cap now holds inside roko's own tool loop only"
anchors = ["crates/roko-cli/src/graph_task_dispatch/watchdog.rs::run_watched", "crates/roko-cli/src/graph_task_dispatch/budget.rs::AttemptAllowance"]
links = { depends_on = [], blocks = [], related = ["bug-4fabd0", "gap-8548e3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cli_agent_cannot_stream_a_run_past_its_cap' crates/roko-cli/src && cargo test -p roko-cli --lib a_cli_agent_cannot_stream_a_run_past_its_cap"
+++

## Problem

bug-4fabd0 holds a plan's cap inside an attempt by asking the plan budget before each model call of roko's own
tool loop (OpenAI-compatible, Cerebras, Anthropic API, Gemini, Perplexity providers). An agent that runs its own
loop (the Claude, Codex, Gemini and Cursor CLIs, Hermes, OpenClaw) is not reached: its attempt is admitted with a
hold on the plan budget, then runs every turn it wants, and its cost is settled only when it ends. Such an attempt
can still spend past the run's cap, by as much as its turn cap and timeout allow. Will's workspace ladder has
`claude-sonnet-4-6` on the Claude CLI as its top rung, so a task that climbs the ladder reaches it.

## Why it matters

The Hermes plugin promises that a run stays under the cap the person sets; that holds today only for the rungs
roko's tool loop runs.

## Where

- `crates/roko-cli/src/graph_task_dispatch/watchdog.rs::GraphTaskDispatcher::run_watched`: drives every attempt and
  can interrupt it (stall watchdog, conductor, operator stop); `AttemptProgress` already sums the usage a provider
  streams (bug-3a3b0f), which the Claude CLI reports per message.
- `crates/roko-cli/src/graph_task_dispatch/budget.rs::AttemptAllowance`: the attempt's hold on the plan budget.

## Plan

1. For an attempt whose provider runs its own loop (`dispatch_v2::runs_own_tools`), add a budget check to
   `run_watched`: every second, price the usage the call streamed (as `InterruptedCall::into_dispatch` prices it),
   and ask the allowance whether its hold can cover that spend plus one more message like the last; when it cannot,
   interrupt the attempt as `AttemptInterrupted::BudgetExhausted`, which settles as `AttemptOutcome::BudgetExhausted`
   with the streamed usage and is not retried.
2. Where a CLI has a native spend limit, pass the hold to it as well.
3. Test with a fake Claude CLI that streams priced messages forever: the plan's spend stays within one message of
   its cap.

## Done when

`a_cli_agent_cannot_stream_a_run_past_its_cap` passes (the `[[verify]]` command).

## Notes

The overshoot is bounded by one streamed message plus the check interval: a CLI cannot be asked before a call.
