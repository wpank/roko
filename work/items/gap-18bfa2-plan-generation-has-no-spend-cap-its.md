+++
id = "gap-18bfa2"
kind = "gap"
title = "Plan generation has no spend cap: its planner calls, retries, regenerations and model escalation are uncapped"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "plans-research, round v6 (tmp/hermes-roko-v6/findings/plans-research.md §3.4 and §5 item 9)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-cli/src/plan_authoring.rs::AuthoringSpend", "crates/roko-cli/src/commands/plan.rs::PlanCmd"]
links = { depends_on = [], blocks = [], related = ["bug-f0d60c", "gap-dc5d7c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_generation_stops_at_its_spend_cap' crates/roko-cli/src && cargo test -p roko-cli --lib -- plan_generation_stops_at_its_spend_cap"
+++

## Problem

`roko plan generate` (and `run --plan`, and serve's `plan_generate`) has no spend cap. Nothing caps the planner's
calls: retries after a crash, retries after a draft that fails validation (which escalate the model), and the
regenerations for hidden runner output, missing source checks and spec quality. In plans-research's round
(§3.4), every run had a cap except plan generation.

## Why it matters

The pitch is cheap models first, under explicit caps. Writing a plan is paid work. It should stop at a cap the
person can see (goal `golden-path`).

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: each planner call after the first.
- `crates/roko-cli/src/plan_authoring.rs::AuthoringSpend`: what the calls have cost so far.
- `crates/roko-cli/src/commands/plan.rs::PlanCmd`: `plan generate`'s flags.
- `[authoring]` in `crates/roko-core/src/config/authoring.rs`.

## Plan

1. Give plan generation a cap: `[authoring] max_generation_usd` (default $2.00) and `plan generate --max-usd`.
2. Before each planner call after the first, check the spend. If it has reached the cap:
   - a retry that would produce the plan stops the generation, with an error that names the cap and the spend;
   - an optional regeneration is skipped, and the plan already in hand is kept.
3. Report the cap in `progress` (`max_usd`), so serve's operation status shows it beside `cost_usd`.

## Done when

`[[verify]]` passes: a generation whose calls reach the cap stops before its next call and says why.
