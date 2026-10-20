+++
id = "gap-e14344"
kind = "gap"
title = "A capped run's Claude CLI attempt is launched without its native spend limit (--max-budget-usd), so only roko's streamed-spend check bounds it"
status = "done"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/budget"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "395b435d1"
source = "tmp/hermes-roko-v5/findings/roko-caps.md (roko-caps, 2026-10-16)"
discovered_from = "gap-ca1c92: plan step 2, split out"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::build_command", "crates/roko-cli/src/dispatch_v2.rs::build_claude_invocation", "crates/roko-cli/src/graph_task_dispatch/budget.rs::AttemptAllowance"]
links = { depends_on = [], blocks = [], related = ["gap-ca1c92", "bug-4fabd0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_capped_claude_cli_attempt_gets_a_native_spend_limit' crates/ && cargo test -p roko-cli --lib a_capped_claude_cli_attempt_gets_a_native_spend_limit"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T12:19:38Z"
commit = "395b435d1"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-16T12:15:37Z"
forced = false
evidence = "395b435d1 (+c1f20e6e9): a capped attempt's allowance gives the most it can still spend, its hold beyond its spend plus the plan's unreserved budget (GraphPlanBudgetLedger::headroom); dispatch puts it on AgentOptions::max_budget_usd when it makes the agent, and the Claude CLI adapter passes it as --max-budget-usd, a second fence behind gap-ca1c92's streamed check. No version probe: --max-budget-usd arrived in Claude Code 2.0.28 and roko already passes --effort, which first appears in the changelog at 2.1.72. A run the CLI stops there (result subtype error_max_budget_usd, checked live on 2.1.282: exit 1, total_cost_usd, terminal_reason budget_exhausted) fails as \"Budget exhausted: the Claude CLI stopped at its spend limit (--max-budget-usd $X) after $Y\", tagged stop_reason budget_exhausted, and both graph paths settle it budget_exhausted at the CLI's cost, non-retryable and not a provider failure. a_capped_claude_cli_attempt_gets_a_native_spend_limit (scripted CLI: the flag at 0.100000 under a $0.10 cap, settled at the CLI's $0.06, one run, budget_exhausted; no flag without a cap), an_attempts_spend_limit_is_its_hold_and_the_unreserved_budget, a_spend_limit_reaches_the_cli_as_max_budget_usd and error_max_budget_usd_is_a_budget_stop pass. At 31e147d4d (main merged in): fmt --check, clippy -D warnings (roko-core, roko-agent, roko-cli, roko-serve, roko-dreams), roko-cli lib tests 3725 passed and the [[verify]] command passes; roko-agent lib tests 2358 passed at 395b435d1."
+++

## Problem

gap-ca1c92 stops a CLI agent's attempt once what its call streamed reaches what the plan budget can pay for: every
250 ms the attempt's watch prices the usage the agent streamed and tells the attempt's hold. That is the only limit
on such an attempt. The Claude CLI has a native spend limit, `--max-budget-usd <amount>` (print mode only; present
in Claude Code 2.1.282), and roko never passes it. So when roko's watch cannot price what the CLI streams (a model
with no rate) or a burst arrives between two looks, nothing inside the CLI stops it before its turn cap and timeout.
Agents that report their usage only at the end (the Codex, Gemini and Cursor CLIs) are never seen by the watch at
all; their only bounds are the turn cap and the timeout.

## Why it matters

The Hermes plugin promises that a run stays under the cap the person sets. The watch keeps a Claude CLI attempt
within about one streamed message of the cap. A native limit would also hold when the watch misses something.

## Where

- `crates/roko-agent/src/claude_cli_agent.rs::ClaudeCliAgent::build_command`: builds the CLI's arguments
  (`--max-turns` today).
- `crates/roko-cli/src/dispatch_v2.rs::CliProviderConfig::build_claude_invocation`: the bare CLI path's arguments.
- `crates/roko-cli/src/graph_task_dispatch/budget.rs::AttemptAllowance`: what the attempt may spend.

## Plan

1. When the attempt starts, compute the most it can spend (its hold plus the plan's unreserved budget). Put that
   on the dispatch request, and pass it as `--max-budget-usd` to a Claude CLI that supports the flag. Older CLIs
   reject the flag, so probe the CLI's version first.
2. Settle the CLI's own budget stop (check which result subtype the stream reports) as
   `Settlement::budget_exhausted`, not as a provider failure that fails over to another model.
3. Test with a fake Claude CLI that checks it got the flag and ends with that result.

## Done when

`a_capped_claude_cli_attempt_gets_a_native_spend_limit` passes (the `[[verify]]` command).

## Notes

Found while closing gap-ca1c92 (roko-caps, 2026-10-16). Its plan's step 2 ("where a CLI has a native spend limit,
pass the hold to it as well") is split out here because it needs plumbing, a version probe and a result mapping.
That would take gap-ca1c92 past its size.
