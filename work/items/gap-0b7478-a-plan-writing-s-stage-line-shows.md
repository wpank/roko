+++
id = "gap-0b7478"
kind = "gap"
title = "A plan writing's stage line shows no spend while its one planner call runs: the tool loop's turns are not priced as they finish"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/plan_generate", "roko-agent/tool_loop"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "bug-1602d4 (R2-6 of the release check, tmp/hermes-roko-v5/findings/dogfood-will.md), roko-planner 2026-10-16"
discovered_from = "bug-1602d4"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs::TurnBudget", "crates/roko-cli/src/agent_exec.rs::run_agent_capture_impl", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan"]
links = { depends_on = [], blocks = [], related = ["bug-1602d4", "gap-11d870"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_writing_reports_its_spend_turn_by_turn' crates/roko-cli/src && cargo test -p roko-cli --lib -- plan_writing_reports_its_spend_turn_by_turn"
+++

## Problem

bug-1602d4 reports a plan writing's stage again as each planner call ends, with the spend so far. Within one call
nothing is reported. A planner on an API model runs several model turns in roko's own tool loop: it reads the
project, then writes the plan. The stage line shows only the spend of calls that already ended. So a one-call plan, the
usual case (the release check's R2-6: one call of about 21 s), still shows $0.00 until its call ends. That is a moment
before the plan is written.

## Why it matters

A host's plan-writing card cannot show what the writing is spending while it works (goal `hermes`).

## Where

- `crates/roko-agent/src/tool_loop/mod.rs::TurnBudget`: the loop calls `record_turn` after each of its model calls.
- `crates/roko-cli/src/agent_exec.rs::run_agent_capture_impl`: builds the planner's agent through `SpawnAgentSpec`,
  which leaves `AgentOptions.turn_budget` unset.
- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: reports the stages.

## Plan

1. Let a planner call pass a `TurnBudget` that never refuses a turn and records each one. It prices the turn: the
   loop's own price, else the config's billed rate for the slug. It then reports the current stage with the spend so
   far plus the call's finished turns.
2. When the call ends, its recorded spend replaces that running estimate (the stage report bug-1602d4 added).
3. A CLI planner (Claude Code) runs its own loop and offers no such hook. Its streamed usage could feed the same report
   later.

## Done when

`[[verify]]` passes: a planner call whose tool loop makes two model turns reports the stage with the first turn's
spend before the call ends.
