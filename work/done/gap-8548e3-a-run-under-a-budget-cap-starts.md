+++
id = "gap-8548e3"
kind = "gap"
title = "A run under a budget cap starts with models it cannot price, and the cap stops it only after the first unpriced call ran"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/budget"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "ad869974f"
source = "roko-init round v5, 2026-10-15 (team lead: plugin wrote unpriced [models.*], caps did not hold)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/budget.rs::unpriced_plan_stop", "crates/roko-cli/src/dispatch_v2.rs::model_has_price", "crates/roko-core/src/config/serve.rs::chat_run_refusal"]
links = { depends_on = [], blocks = [], related = ["bug-4e336b", "bug-1bc222", "gap-ceffb3", "gap-2bec05", "gap-cf427e", "gap-c8f388"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn capped_run_with_an_unpriced_model_is_refused_up_front' crates/ && cargo test -p roko-cli --lib capped_run_with_an_unpriced_model_is_refused_up_front"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T09:11:02Z"
commit = "ad869974f"
by = "commit trailer"
executor = "unknown"
size = "M"
claimed_at = "2026-10-16T08:10:23Z"
forced = false
evidence = "ad869974f: under a budget cap (a plan ceiling or max_daily_usd; --no-budget lifts it) failover passes over a model roko has no price for (no snapshot row, no [models.*] rates, no built-in rate) before any call, as a definitive unpriced_under_cap refusal. A priced fallback takes the task, even on the same provider; a --model pin is refused the same way; with nothing priced left the task fails at once, not retried, naming the model and the fix (cost_input_per_m and cost_output_per_m, 0 for a free local model, or --no-budget). The Claude CLI is exempt: it reports what each run cost. A chat host's run is refused up front with refusal code no_price (structuredContent.error.data.refusal and refusals) when none of the models it can run on has a price; model_registry::model_has_price in roko-core is the one rule for dispatch, serve and roko doctor (model_prices, chat_host_runs). capped_run_with_an_unpriced_model_is_refused_up_front, a_chat_run_needs_a_model_with_a_price, chat_run_refused_without_a_priced_model and the doctor chat-host test pass. On the stacked tip 0216a4cbd (with find-c3da60 and gap-ca1c92): fmt --check, clippy -D warnings and the lib tests of roko-core (2034), roko-agent (2353), roko-cli (3707) and roko-serve (1033) pass (tmp/hermes-roko-v5/roko-caps/logs/stack-checks.log)."
+++

## Problem

A run under a budget cap starts even when a model it can be routed to has no price: no price-snapshot row, no
`[models.*]` rates, and no built-in rate (`dispatch_v2::model_has_price`). roko records such a call at an unknown
cost, counted as $0. The cap catches this only after the call has run. `budget.rs::dispatch_stop` then stops
the plan: "plan budget cannot be enforced: N unpriced call(s) against max_plan_usd …" (decision 2110,
backlog 2111). That one call's spend is already outside the cap. With several tasks in flight, there can be
several such calls.

This holds for both kinds of capped run:
- a CLI plan run under `[budget] max_plan_usd`;
- a run a chat host starts over `/mcp` with `max_usd`. serve's up-front checks (`chat_run_refusal`:
  `no_data_boundary`, `no_tool_loop_model`) don't look at prices.

Seen 2026-10-15: the Hermes plugin's `hermes roko init` first wrote `[models.*]` tables without prices, and chat
runs on those models went through with their `max_usd` caps unenforced. The plugin now writes prices; roko should
not depend on that.

## Why it matters

A cap is the person's guarantee about spend, and a chat-started run is the one nobody watches. "Stops after the
first unpriced call" lets spend through before the stop.

## Where

- `crates/roko-cli/src/graph_task_dispatch/budget.rs::unpriced_plan_stop`: today's after-the-fact stop.
- `crates/roko-cli/src/dispatch_v2.rs::model_has_price`: what counts as a price.
- `crates/roko-core/src/config/serve.rs::chat_run_refusal`: serve's up-front refusals for chat-started runs.
  roko-chathost owns this code this round.
- The plan run's start (graph_execution), where a CLI run with a ceiling could refuse, or warn, before its first
  dispatch.

## Plan

1. Decide what "can be routed to" means: the ladder's runnable rungs, the default model, `routing.fallback_models`
   and failover's candidates. Sharing one function with `roko doctor`'s `model_prices` check is preferable.
2. Under a ceiling, refuse a run up front when one of those models has no price. The reason names the models and
   the fix: rates on their `[models.*]` entries, 0 for a free local model. Or drop unpriced candidates from
   routing for that run. `--no-budget` keeps today's behaviour.
3. For chat runs, add a `no_price` refusal (`structuredContent.error.data.refusal`) beside `no_tool_loop_model`,
   so the Hermes plugin can show it as a setup problem.
4. Tests: a capped plan whose only rung is unpriced is refused before any dispatch, and a chat run over `/mcp`
   gets the `no_price` refusal.

## Done when

`capped_run_with_an_unpriced_model_is_refused_up_front` passes (the `[[verify]]` command).
