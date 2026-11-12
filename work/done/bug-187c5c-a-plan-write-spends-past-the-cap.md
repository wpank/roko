+++
id = "bug-187c5c"
kind = "bug"
title = "A plan write spends past the cap a person approved: the cap is checked only once the calls so far cost it, so phase 1's write spent $0.258 against $0.25, and nothing bounds a planner call's own tool loop"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/plan_generate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "acb74de21"
source = "round-v8 hermes-pilot-v8, phase 1 plan write op 19bfbbb8 (Undertow 19943c9, roko bd93521ee), tmp/hermes-roko-v8/pilot/evidence/p1-plan-writing-status.json"
discovered_from = "agent:hermes-pilot-v8 (round v8)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-cli/src/agent_exec.rs::AgentExecOpts", "crates/roko-cli/src/agent_spawn.rs::SpawnAgentSpec", "crates/roko-cli/src/plan_authoring.rs::AuthoringSpend"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-18bfa2", "bug-4fabd0", "gap-e14344", "bug-6ebc03"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_generation_never_spends_past_its_cap' crates/roko-cli/src/ && cargo test -p roko-cli --lib plan_generation_never_spends_past_its_cap"

[[verify]]
command = "grep -rqw 'fn the_generation_budget_refuses_the_call_that_would_pass_the_cap' crates/roko-cli/src/ && cargo test -p roko-cli --lib the_generation_budget_refuses_the_call_that_would_pass_the_cap"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T16:03:41Z"
commit = "122355614"
by = "roko-planner-v8"
via = "manual"
executor = "claude-agent"
size = "M"
forced = false
evidence = "Between planner calls a call is made only while what is spent and the costliest call so far stay within the cap (AuthoringSpend keeps the costliest); an improvement it leaves out is skipped and the plan in hand written, a retry it needs stops with 'would pass its spend cap'. Inside a planner call AgentExecOpts carries a SpendLimit, what is left of the cap: roko's tool loop refuses a model call when the run's spend and the generation's costliest model call would pass it, priced as the call's spend is, and a Claude CLI planner gets it as --max-budget-usd; a run stopped there is the crash class BudgetExhausted, not retried. Verify plan_generation_never_spends_past_its_cap ($0.13 calls: under $0.25 the regeneration is skipped at $0.13 and a needed retry stops with the cap's error; under $0.30 both calls run with --max-budget-usd 0.300000 then 0.170000) and the_generation_budget_refuses_the_call_that_would_pass_the_cap pass on the trial merge 5419c421e (main ba5866664 + bug-6ebc03 + this branch), with plan_generation_stops_at_its_spend_cap, all 96 pipeline tests and nightly fmt --check; roko-cli lib 4131 passed on the combined tree before the test fix, and cargo clippy --workspace --no-deps -D warnings is clean."
+++

## Problem

A plan write spends past the cap a person approved.

Round v8's phase 1 plan write (op 19bfbbb8) was capped at `max_usd = 0.25` and spent $0.258102 in two planner calls,
3% over. The Hermes card said "$0.26 of $0.25".

The cap is checked only after money is spent. `generate_plan`'s `cap_reached` refuses a planner call once the calls so
far cost the cap or more (gap-18bfa2). So a call that starts below the cap may cost any amount. Here the second call,
the coverage regeneration, started with about half the cap spent, and cost about as much as the first. Inside a
planner call nothing is checked at all: the planner's agent runs its whole tool loop, every model call of it,
whatever the cap.

The test `plan_generation_stops_at_its_spend_cap` asserts the overrun: "A weak plan is written as it is", with
`outcome.cost_usd > CAP`.

## Why it matters

- A person approves a plan write at a price. The guide and the Hermes card show it as a cap, and it must hold.
- The round's budget is $40 for every plan write and run together.
- Goal: core (plan writing).

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: `cap`, `cap_reached`, `skip_for_cap`. Every
  planner call goes through `run_agent_capture_silent_with_usage`.
- `crates/roko-cli/src/agent_exec.rs::AgentExecOpts`, `crates/roko-cli/src/agent_spawn.rs::SpawnAgentSpec`: no way
  to give the planner's agent a spend limit. `AgentOptions` has one (`turn_budget`, which roko's tool loop asks
  before each model call, and `max_budget_usd` for the Claude CLI), and plan runs set it (bug-4fabd0, gap-e14344).
- `crates/roko-cli/src/plan_authoring.rs::AuthoringSpend`: what the calls cost.

## Plan

1. **Between planner calls.** A call is made only when what is spent, plus what the costliest planner call so far
   cost, stays within the cap. That is the plan runs' rule, "as much as its last" (bug-4fabd0), taken from the
   costliest call. A retry the plan needs then stops with the cap's error. An improvement (a regeneration) is
   skipped, and the plan in hand is written.
2. **Inside a planner call.** The planner's agent gets a turn budget. roko's tool loop asks it before each model call,
   and it refuses a call when what the generation spent, plus the costliest model call so far, would pass the cap.
   A Claude CLI planner gets `--max-budget-usd`, set to what is left of the cap. A planner call that stops for the cap
   is not retried.
3. **The first call.** When the cap is below what the first model call's input alone costs (the prompt's tokens at
   the model's price), generation refuses to start and names both numbers.
4. **Tests, red first.**
   - `plan_generation_never_spends_past_its_cap`: a fake planner whose calls cost $0.13 each, under a $0.25 cap. The
     coverage regeneration is not asked for, and the plan in hand is written at $0.13. A planner that would need a
     second call stops with the cap's error, at $0.13.
   - A turn budget that refuses the model call that would pass the cap.

## Done when

- The `[[verify]]` command passes.
- No plan write's recorded spend is over its cap in the test suite, and `plan_generation_stops_at_its_spend_cap`
  asserts the cap holds.

## Notes

- Found by hermes-pilot-v8 (evidence: `tmp/hermes-roko-v8/pilot/evidence/p1-plan-writing-status.json`: `cost_usd`
  0.258102, `max_usd` 0.25).
- The overrun's second call was the coverage regeneration that bug-6ebc03's false "missing" asked for.
- Related: gap-18bfa2 (the generation cap), bug-4fabd0 and gap-e14344 (plan runs' per-call allowance and the CLI's
  own limit).
