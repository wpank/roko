+++
id = "gap-8c0a20"
kind = "gap"
title = "One tier enum shared by the plan parser, router, budget and turn caps"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-core/task", "roko-cli/graph_task_dispatch", "roko-core/config"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "8b26e4839"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e5"
discovered_from = "tmp/cybernetic-harness/tldr/research/B1-plan-authoring.md (Tier → model routing row); tldr/05 P1 #8"
anchors = ["crates/roko-core/src/task.rs::TaskComplexityBand", "crates/roko-cli/src/graph_task_dispatch/routing_context.rs::build_routing_context", "crates/roko-cli/src/graph_task_dispatch/turn_policy.rs::is_express_task", "crates/roko-core/src/config/budget.rs::BudgetConfig::task_limit_usd", "crates/roko-core/src/config/gates.rs::PipelineConfig::for_tier", "crates/roko-cli/src/plan_generate.rs::TaskTier", "crates/roko-cli/src/dispatch/model_routing.rs::tier_to_complexity"]
lane = "rust-hot"
parent = "spec-98f76d"
links = { depends_on = ["gap-96f7ed"], blocks = [], related = ["gap-0f3980", "gap-1d1fa6", "gap-5a6e01"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn task_tier_parses_every_alias' crates/roko-core/src/ && cargo test -p roko-core --lib task_tier_parses_every_alias"

[[verify]]
command = "grep -rqw 'fn plan_tiers_reach_router_budget_and_turn_caps' crates/roko-cli/src/ && cargo test -p roko-cli --lib plan_tiers_reach_router_budget_and_turn_caps"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in a13873ad2. Batch 11 gate on the merged tree (MAIN 8b26e4839 has the same tree as gated 194658fed): cargo check --workspace --tests, nightly fmt --check and clippy -p (10 crates) --no-deps -D warnings clean; lib tests pass: roko-cli 3126, roko-agent 2257, roko-core 1936, roko-learn 1188, roko-serve 958, roko-gate 688, roko-graph 471, roko-dreams 252, roko-std 223, roko-execution 100. Verify: task_tier_parses_every_alias (roko-core lib) and plan_tiers_reach_router_budget_and_turn_caps (roko-cli lib) pass."
+++

## Problem

`TaskDef.tier` is a free string (default `"focused"`). Five places turn it into behaviour, each with its own
vocabulary:

| Consumer | Where (HEAD) | Mapping |
|---|---|---|
| Router | `build_routing_context`, `graph_task_dispatch.rs:3264` | `fast/t0/0` → Fast; `complex/t2/2/premium` → Complex; else Standard |
| Budget | `BudgetConfig::task_limit_usd`, `budget.rs:140` | mechanical; integrative or complex; architectural or expert; else standard |
| Turn caps | `PipelineConfig::for_tier`, `gates.rs:545` | the four plan tiers; unknown → focused |
| Express mode | `is_express_task`, `graph_task_dispatch.rs:699` | mechanical or trivial |
| Generator | `plan_generate::TaskTier`, `plan_generate.rs:117` | the four plan tiers, with LOC limits |

A sixth, `model_routing::tier_to_complexity` (:523), is dead code. The 541 tiered tasks in `plans/` use only the four
plan tiers, so the router sees every one of them as `Standard`. A typo such as `"mechancial"` is silently accepted.

## Why it matters

The ladder (gap-9cbf35) starts each task on a rung chosen by its tier. That fails while the router, budget and turn
caps disagree on what a tier is. gap-1d1fa6 (size limits per tier) and gap-5a6e01 (limits from history) need the same
enum. This is step 1 of epic spec-98f76d.

## Where

- `crates/roko-core/src/task.rs`: the new `TaskTier`, beside `TaskComplexityBand`.
- The five consumers in the table, and `plan_validate.rs` for a lint.
- Entry point: `roko plan run` → `GraphTaskDispatcher::dispatch`.

## Current state

Checked at `41c7ffbd6`: as described above. Nothing validates tier strings.

## Plan

1. Move `plan_generate::TaskTier` (with `max_loc` and `label`) into `roko-core/src/task.rs`. Give it a trimmed,
   case-insensitive `parse` over one alias table:
   - mechanical: `trivial`, `fast`, `quick`, `t0`, `0`;
   - focused: `standard`, `t1`, `1`;
   - integrative: `complex`, `t2`, `2`;
   - architectural: `premium`, `expert`, `deep`, `t3`, `3`.
2. `TaskTier::complexity_band()`: mechanical → Fast, focused → Standard, integrative and architectural → Complex.
3. Keep `TaskDef.tier: String`, so no struct literal changes, and add `TaskDef::tier_class()` (unknown → Focused, as
   today). Route every consumer through it, and delete `tier_to_complexity`.
4. `plan validate`: a new `PLAN_0xx` code for an unknown tier, an error under `--strict` and a warning otherwise.
5. Write the canonical label to `CostRecord.complexity_band` (`graph_task_dispatch.rs:1770`).
6. Tests: `task_tier_parses_every_alias` (roko-core), and `plan_tiers_reach_router_budget_and_turn_caps`, which checks
   for each tier that the routing band, budget multiplier, turn cap and express eligibility agree.

## Done when

- [ ] Every consumer reads the tier through one parser; `mechanical` routes as Fast and `integrative` as Complex.
- [ ] `plan validate --strict` rejects an unknown tier, and `tier_to_complexity` is gone.
- [ ] Both `[[verify]]` commands pass.

## Notes

- Integrative → Complex matches the budget's existing multiplier. Tasks that all routed as `Standard` now get other
  bands, which shifts cascade-router statistics; that is intended.
- **Hot file:** `graph_task_dispatch.rs`. This item waits for gap-96f7ed (E4.2), which also edits `dispatch()`. It runs
  before gap-0f3980's routing part, which edits `build_routing_context` too.
- 2026-09-29 (wk-tiers): implemented on `work/gap-8c0a20` at `c7bbc58e0`; cargo verification deferred to the batch check. Locally, `cargo check -p roko-cli --lib --tests` and both `[[verify]]` tests pass (targeted `cargo test`).
- Premise partly false at `ed0c33bd5`: unknown tiers were already rejected. `TasksFile::validate_against_schema` (since `d89c20293`) makes one a PLAN_035 error in `plan validate`, and `load_plan` refuses the plan in `plan run`. No new PLAN code was added. That check now reads through `TaskTier::parse`, so aliases (`T0`, `premium`) pass and typos still fail (`unknown_tier_is_a_schema_error_and_tier_aliases_pass`).
- Plan step 5 not done: `CostRecord.complexity_band` and `VerifiedAttempt.task_type` still carry the raw tier, because `graph_task_dispatch/feedback.rs` belongs to wk-settle. The follow-up after both merge is one line each: `task.tier_class().label().to_string()`.
- Other tier tables remain outside the five consumers: roko-learn's `conductor::complexity_bucket` reads `complex` as architectural (TaskTier reads it as integrative); `cost_projection::normalize_tier` and the TUI's `default_model_for_tier` keep their own.
