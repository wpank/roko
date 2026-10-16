+++
id = "bug-4fabd0"
kind = "bug"
title = "A run's spend cap is checked only between attempts, so one attempt can spend many times what is left (a $0.50 run spent $0.753)"
status = "done"
triage = "verified"
severity = "p1"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/budget", "roko-agent/tool-loop"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "eb26a6fe5"
source = "tmp/hermes-roko-v5/findings/dogfood-desktop.md#P2 (dogfood-desktop, 2026-10-16)"
discovered_from = "hermes-roko-v5 round: run 27d30ffc started from the Hermes desktop page"
anchors = ["crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetLedger", "crates/roko-agent/src/tool_loop/mod.rs::ToolLoop", "crates/roko-cli/src/dispatch_v2.rs::AgentDispatchRequest"]
links = { depends_on = [], blocks = [], related = ["gap-8548e3", "bug-1bc222", "bug-f5470e", "find-183c12"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_looping_agent_cannot_push_a_run_past_its_cap' crates/roko-cli/src && cargo test -p roko-cli --lib a_looping_agent_cannot_push_a_run_past_its_cap"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T03:25:04Z"
commit = "eb26a6fe5"
by = "commit trailer"
executor = "unknown"
forced = false
evidence = "eb26a6fe5: the plan budget holds inside an attempt. roko's tool loop asks a TurnBudget before each model call; dispatch_v2 prices each call for the attempt's CallAllowance; budget.rs keeps a hold per attempt that grows into the unreserved budget and is trimmed after each call, and refuses a call the plan cannot pay for; an attempt starts only with room for 3 calls at the costliest so far (min $0.01); a budget stop settles as AttemptOutcome::BudgetExhausted and is not retried. a_looping_agent_cannot_push_a_run_past_its_cap and 6 more tests pass; roko-agent and roko-cli lib suites pass; clippy clean. Live, scratch project, $0.03 cap: main 1630fa1bf spent $0.0434, this change $0.0264 with the verified task delivered (tmp/hermes-roko-v5/findings/roko-caps.md)."
+++

## Problem

A run's spend cap is checked only between attempts, so one attempt can spend many times what the run has left.
Seen live on main 1630fa1bf (dogfood-desktop, 2026-10-16). Run `27d30ffc`, plan `wordy-sort-and-min`, started from
the Hermes desktop page with `max_usd` 0.50, spent **$0.753** and ended FAILED with "budget exceeded:
plan_cost_micro_usd (used 753000, limit 500000)". Its first task ran on gpt-oss-120b (Cerebras):

- attempt 1: $0.467 in 53 turns (about 45 failed `apply_patch` calls), cache_read 1,072,128 tokens;
- attempt 2: started with $0.033 of the cap left, then cost $0.285 in 53 turns.

The workspace has `[conductor] max_agents = 8` and no `max_turn_usd`, so each attempt reserved $0.0625 (1/8 of the
cap, backlog 3102) and spent 7.5 times that before anything looked at the budget again.

## Why it matters

The cap is the person's guarantee about spend. The Hermes plugin says a run stays "under the cap you set",
Settings says "The app never spends past them", and Run plan… says roko "stops when it reaches the cap". A run a
chat host starts is the one nobody watches.

## Where

- `crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetLedger`: per-plan ledger. `try_reserve`
  admits an attempt with a reservation (`max_turn_usd`, else the ceiling's share per `max_agents`); `settle` adds
  the attempt's whole cost once it has ended. Nothing in between.
- `crates/roko-cli/src/graph_task_dispatch.rs::dispatch`: reserves, runs the agent, settles. The reservation's
  routing budget only steers model choice.
- `crates/roko-agent/src/tool_loop/mod.rs::ToolLoop`: roko's own turn loop (OpenAI-compatible, Cerebras,
  Anthropic API, Gemini, Perplexity). It has a budget hook checked before each model call (LIFE-03,
  `with_budget`), which no production code sets.
- `crates/roko-cli/src/dispatch_v2.rs`: builds each agent's `AgentOptions`, and prices usage
  (`fill_usage_cost_from_pricing`: price snapshot, then profile, then registry).

## Current state

- The tool loop's `with_budget` hook takes a daily/lifetime `BudgetTracker`; it is wired nowhere.
- `AttemptOutcome::BudgetExhausted` exists in the verdict schema (blamed on the harness, no learning label);
  dispatch never produces it.
- The Cerebras adapter's tool loop has no model profile, so its turns are not priced inside the loop; dispatch
  prices the whole attempt afterwards from the snapshot.
- A reservation refusal is `RokoError::BudgetExceeded`, which the task executor retries (each retry fails at once),
  and whose text is in micro-USD.

## Plan

1. Generalise the tool loop's budget hook into a `TurnBudget` trait (asked before each model call, told each
   call's usage after it), keeping `BudgetTracker` as one implementation. Carry it in `AgentOptions` and attach it
   in every adapter that builds a tool loop, beside `data_llm`.
2. dispatch_v2 binds an attempt's allowance to the model that serves it: it prices each turn as settlement will
   (snapshot, profile, registry) and passes the cost on.
3. In the plan ledger, an attempt's reservation becomes a hold that tracks its calls: before each call the hold
   must cover what the attempt spent plus one more call like its last, growing into the plan's unreserved budget;
   after each call the hold is trimmed to that, so parallel attempts can grow into the rest. A call the plan cannot
   cover is refused, and the attempt stops `budget_exhausted` with the reason. A turn with no price stops it too
   (decision 2110).
4. An attempt starts only with a floor left: room for 3 calls at the costliest call the plan has made, at least
   $0.01 (never more than the cap). Below it the run stops as budget-exhausted (`dispatch_stop`), and the refusal
   is a non-retryable budget error with a plain sentence.
5. A budget-stopped attempt settles as `AttemptOutcome::BudgetExhausted`, keeps its checkout, is not retried, and
   is not a provider failure. Tasks that passed keep their verified work.

## Done when

- A scripted model that keeps calling tools cannot push a run past its cap by more than one turn's cost
  (`a_looping_agent_cannot_push_a_run_past_its_cap`, the `[[verify]]` command).
- A live run on a small scratch project with a tight cap stops at the cap, where main overran it.

## Notes

- CLI agents (Claude, Codex, Gemini, Cursor CLIs) run their own loop, so this does not reach inside them; their
  attempts are still bounded by the reservation and the plan stops after them. Follow-up item if needed.
- Related: gap-8548e3 (unpriced models under a cap), bug-1bc222 (free vs unpriced), bug-f5470e (apply_patch
  envelope that made gpt-oss-120b loop), find-183c12 (turn caps of the cheap rung).
