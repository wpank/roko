+++
id = "spec-98f76d"
kind = "spec"
title = "Epic: tier ladder and escalation"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "L"
subsystem = ["roko-cli/dispatch", "roko-cli/graph_task_dispatch", "roko-core/config"]
created = 2026-09-29
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "6a08f9e2c"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e5"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P1 #8); tldr/04 steps 4 and 8, design rules 1 and 5"
anchors = ["crates/roko-cli/src/graph_task_dispatch/routing_context.rs::build_routing_context", "crates/roko-cli/src/dispatch/model_routing.rs::ModelRouter::route", "crates/roko-core/src/config/routing.rs::RoutingConfig"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "rust-hot"
links = { depends_on = ["gap-8c0a20", "gap-0f3980", "gap-9cbf35", "gap-dbf2a6", "gap-460230", "gap-b62e95", "gap-e21595", "bug-a6b433", "bug-cae1e1"], blocks = [], related = ["gap-a791b4", "bug-35379d", "gap-1d1fa6", "gap-853b31"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn tier_ladder_canary' crates/roko-cli/tests/ && cargo test -p roko-cli --test tier_ladder_canary"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T07:43:05Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "all children done; its verify (C8 tier_ladder_canary) passed in gate 6h2 at 285282248, merged in 6a08f9e2c"
+++

## Problem

Roko's core claim is that a frontier model plans and cheap models execute. The Graph path cannot run that way:

- **The tier never reaches the router.** `build_routing_context` knows only `fast|t0|0` and `complex|t2|2|premium`.
  All 541 tiered tasks in `plans/` use `mechanical`, `focused`, `integrative` or `architectural`, so every task routes
  as `Standard`. The budget, turn caps, express mode and generator each parse tiers their own way.
- **A person picks the model.** `ModelRouter::route` takes `--model`, then `model_hint` (425 slugs in `plans/`), then
  the cascade router (its LinUCB stage has learned from 1 observation, B3), then `claude-sonnet-4-6`. Generated plans
  lose their hints.
- **Nothing escalates.** A failed gate retries the same model. Escalation was deleted with Runner-v2 (`6b5da8616`).

## Why it matters

The author wants "cheaper at equal quality" proven first (PLAN.md §1). The benchmark's cheap-executor arm (tldr/04,
"How to prove it", arm C) cannot run until tasks route by tier and escalate on failure. This is W8's gate G8.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`: `build_routing_context` (:3234), `is_express_task`,
  `task_turn_limit` and `task_budget_ceiling_usd`. In `dispatch()` the routing context is built (:3569) before
  `next_retry_attempt` numbers the attempt (:3614).
- `crates/roko-cli/src/dispatch/model_routing.rs`: `ModelRouter::route` (:341).
- roko-core: `config/routing.rs::RoutingConfig`, `config/budget.rs`, `config/gates.rs` and `task.rs`.
- Plan authoring: the `plan_generate.rs` prompts, and `prd.rs::validate_and_fix_generated_plan`, which strips hints
  (:2841).

## Current state

Checked at `41c7ffbd6`.
- `routing.{standard,complex}_task_model` are display-only; `fast_task_model` serves only express mode, which is off.
  `ModelRegistry::upgrade_tier`, `MODEL_ESCALATION_LADDER` and `TaskComplexityBand::escalate` have no Graph caller.
- Today's merges changed retries, not the model. `ce3bdcbb8` numbers attempts and keeps their gate feedback across
  resume; `99adacd6d` and `41c7ffbd6` give un-authored tasks 3–5 retries; `d4be4e872` retries a timeout with 1.5× the
  time. `ModelRouter::route` ignores `DispatchContext.attempt`, and the streaming path sends `attempt: 0` (:4416).
- gap-b62e95 is still open (`iteration: 0`, `has_prior_failure: false`), but its inputs now exist in `dispatch()`.
- gap-0f3980 is still open and L-sized; a split is proposed in Notes.
- `roko.toml` has no GLM-4.7 or gpt-5.4-mini model entry.
- E10's gap-a791b4: the HTTP tool loop, which every cheap rung uses, ignores tier turn caps.

## Plan

This is the implementation plan.

1. **Tier enum** (gap-8c0a20, M): one `TaskTier` shared by the router, budget, turn caps, express mode and generator;
   `plan validate` rejects unknown tiers. After E4.2 (gap-96f7ed), which also edits `dispatch()`.
2. **Routing metadata** (gap-0f3980, parts a and b of the split): after step 1, since both edit
   `build_routing_context`.
3. **Ladder config** (gap-9cbf35, S): named rungs and a start rung per role and tier, with D11's cascade plus Sonnet
   as the default. The ladder decides; the learner's pick is only logged.
4. **Rung hints** (gap-dbf2a6, S): plans pin a rung, never a model, and generated plans keep rung hints.
5. **Verify-then-escalate** (gap-460230, M): two agent-blamed failures move a task one rung up, at most twice. A
   failure on the top rung is flagged for a split or replan. Every attempt records its rung, model and reason. After
   E4.2.
6. **Router context** (gap-b62e95, S): the learner sees a retry as a retry.
7. **Exit check C8** (gap-e21595, M).

Order: 1, then 3, then 4 and 5 in parallel (different files), then 6, then 7. Step 2 follows 1. Every step except 7
edits hot files: one writer per file at a time.

## Done when

- [x] gap-8c0a20: One tier enum shared by the plan parser, router, budget and turn caps
- [x] gap-0f3980: TaskDef Routing Metadata Wiring (24-field gap) (existing item)
- [x] gap-9cbf35: Routing ladder config: role and tier map to a model rung, with the D11 cascade as the default
- [x] gap-dbf2a6: Plan hints pin a ladder rung instead of a model name, and generated plans keep their hints
- [x] gap-460230: Verify-then-escalate: two failed attempts move a task one rung up the ladder
- [x] gap-b62e95: The router's context treats every retry as a first attempt (existing item)
- [x] gap-e21595: Integration test C8: the ladder routes by tier and escalates after two failures
- [x] bug-a6b433: roko-learn's complexity_bucket reads complex as architectural, while TaskTier reads it as integrative
- [x] bug-cae1e1: The streaming dispatch path doesn't mark retries in the routing context and ignores preferred_provider
- [ ] The epic's `[[verify]]` command (test C8) passes on the merged branch.

## Notes

- **Proposed split of gap-0f3980 (L):**
  - (a) carry the 25 fields in a `TaskHints` struct, with the test `parses_every_task_routing_field` (M);
  - (b) routing reads the authored category, band, reasoning level, preferred model and preferred provider
    (`routing_context_uses_authored_task_metadata`), without its step 5, which gap-460230 and gap-b62e95 own (M);
  - (c) the prompt fields are not needed for the ladder; move them to E8 or `core` (M).
- **Existing children keep their goal and severity:** gap-0f3980 (`core`, p1) and gap-b62e95 (`learning`, p3).
  PLAN.md §5 proposes moving both to `golden-path`.
- **Out of scope:** the planner model (gap-853b31), failover substitutions (bug-35379d), `cost_source` (S01.P0-6), and
  S04.T12's learned start rung, low-confidence escalation and verify depth. This epic builds S04's baseline H4-B1.
- **For the author (tldr/05 §6, decisions 1–3):** should the ladder be on by default? Recommended: yes, skipping rungs
  whose model has no configured provider, so a workspace with only Claude keys routes as it does today.
- **Hot files:** `graph_task_dispatch.rs` (a split is planned, E15.4) and `dispatch/`. Start after the portal
  session's branches merge and the split lands.
- **Decided 2026-09-29 (Will):** the ladder is on by default. Tasks without a model hint use it, with start rungs as in gap-9cbf35. Tasks without an authored `max_retries` get 5 retries while it is on (gap-460230).
