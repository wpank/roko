+++
id = "gap-ca1c92"
kind = "gap"
title = "An agent that runs its own loop (Claude, Codex, Gemini, Cursor CLIs) can still spend past a run's cap within one attempt"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/budget"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "f3ea5508c"
source = "tmp/hermes-roko-v5/findings/roko-caps.md (roko-caps, 2026-10-16)"
discovered_from = "bug-4fabd0: the cap now holds inside roko's own tool loop only"
anchors = ["crates/roko-cli/src/graph_task_dispatch/watchdog.rs::run_watched", "crates/roko-cli/src/graph_task_dispatch/budget.rs::AttemptAllowance"]
links = { depends_on = [], blocks = [], related = ["bug-4fabd0", "gap-8548e3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cli_agent_cannot_stream_a_run_past_its_cap' crates/roko-cli/src && cargo test -p roko-cli --lib a_cli_agent_cannot_stream_a_run_past_its_cap"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T09:12:09Z"
commit = "f3ea5508c"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-16T09:11:25Z"
forced = false
evidence = "f3ea5508c (+0216a4cbd): run_watched gets the attempt's allowance; every 250 ms, for an agent that runs its own loop (dispatch_v2::runs_own_tools), it prices what the call has streamed (price_target_usage, as an interrupted call is priced), tells the hold the spend since the last look as one model call and asks it again; a refusal drops the call as AttemptInterrupted::BudgetExhausted, a non-retryable budget error settled budget_exhausted with the streamed usage, and the plan starts no further attempt once less than an attempt needs is left. The spend stays within about one streamed message of the cap; unpriced streamed usage is left to the cost the agent reports at its end; agents that report usage only at the end (Codex, Gemini, Cursor CLIs) stay bounded by turn cap and timeout. Plan step 2 (the Claude CLI's own --max-budget-usd) is split out as gap-e14344. a_cli_agent_cannot_stream_a_run_past_its_cap passes (a fake Claude CLI streaming a $0.0315 message a second under a $0.10 cap stops at $0.0945). On the stacked tip 0216a4cbd (with gap-8548e3 and find-c3da60): fmt --check, clippy -D warnings and the lib tests of roko-core (2034), roko-agent (2353), roko-cli (3707) and roko-serve (1033) pass (tmp/hermes-roko-v5/roko-caps/logs/stack-checks.log). Live (roko-caps, 2026-10-16): a real Claude CLI attempt on claude-haiku-4-5 under a $0.04 cap was stopped budget_exhausted after 18 s at $0.0373 as the budget counts it, with no retry; its verdict prices the same usage at api_equiv $0.0549 because the streamed estimate prices Claude Code's 1-hour cache writes at the 5-minute rate: filed as bug-acfced."
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
