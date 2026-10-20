+++
id = "gap-0fb7d9"
kind = "gap"
title = "roko's repairs to its planner's draft exist only in stderr and tracing: the plan-writing result and plan.md do not say what roko changed"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7-plugin-wow#repairs"
discovered_from = "agent:plugin-wow (round v7)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-serve/src/runtime.rs::PlanGenerationResult"]
links = { depends_on = [], blocks = [], related = ["gap-33d458"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_repaired_draft_returns_its_repair_lines' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_repaired_draft_returns_its_repair_lines"
+++

## Problem

When roko writes a plan, it repairs the planner's draft before it validates it. Examples:
- "T4 step 4: `grep -q 'AllocationCounter' core/src/engine.cpp && tools/spec.sh …` became `tools/spec.sh …`, the
  source's own commands alone";
- "T1: `core/src/envelope.h` became `core/include/undertow/envelope.h` in its files and its text";
- "T3: added a step running `tools/spec.sh 2_1 engine 'P2.1.09*'`".

These lines go only to stderr ("Repaired the draft: …") and to tracing, from the `repairs` Vec in
`plan_generate/pipeline.rs`. So a host cannot show them: the plan-writing result that serve returns in the
operation's result has none, and neither does the plan's `plan.md`. A person who reads the plan cannot tell which
parts the planner wrote and which roko changed.

Found by plugin-wow (round v7), for the plugin's plan view and the guide.

## Why it matters

Trust. roko's pitch is that it checks and fixes its own planner's work. A person who can see the fixes can check
them, and can see why a plan differs from what the planner said.

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs`: `extract_and_validate` builds `repairs`; the plan is written in
  the "writing" stage.
- `crates/roko-learn/src/runtime_feedback/records.rs`: `GenerationOutcome`.
- `crates/roko-serve/src/runtime.rs`: `PlanGenerationResult`; `crates/roko-serve/src/routes/plans/authoring.rs`:
  the operation's result.

## Plan

1. Carry the repair lines of the draft that is written out of `extract_and_validate`, keyed by the validated plan,
   as `accept_for` and `folded_for` are.
2. `GenerationOutcome.repairs` and `PlanGenerationResult.repairs` (`[string]`, left out when empty); the
   plan-generation operation's result carries `repairs`.
3. `plan.md` gets a short "Repairs" section listing them, whether the planner wrote it or roko did.

## Done when

The test in `[[verify]]` passes: generating a plan whose draft roko repairs returns those lines in the outcome, and
the plan's `plan.md` lists them.
