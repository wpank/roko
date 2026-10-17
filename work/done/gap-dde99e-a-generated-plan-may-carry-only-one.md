+++
id = "gap-dde99e"
kind = "gap"
title = "A generated plan may carry only one verify step per task, so a spec with several acceptance checks per task fuses them into one && chain or drops them"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-16
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "0b5d222c7"
source = "plans-research, round v6 (tmp/hermes-roko-v6/findings/plans-research.md §3.2 and §5 item 4)"
anchors = ["crates/roko-cli/src/plan_policy.rs::PlanExecutionPolicy::generated", "crates/roko-cli/src/plan_generate.rs::render_tier_size_limits"]
links = { depends_on = [], blocks = [], related = ["bug-f0d60c", "gap-f8a6ea", "spec-e57870"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn generated_policy_keeps_a_step_per_acceptance_criterion' crates/roko-cli/src && cargo test -p roko-cli --lib -- generated_policy_keeps_a_step_per_acceptance_criterion"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T13:27:29Z"
commit = "c75361fd8"
by = "roko-planner"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Generated policy: min_verify_steps_per_task 1, max up to the tier limit (3/4/6/8 via TierSizeLimits), FAST keeps exactly one (generated_for_environment); prompt rule 3, implementer line, checklist and regeneration prompt ask for a step per acceptance criterion, never chained; tier size line states each tier verify-step limit; gap-f8a6ea limits line rewritten to match (c75361fd8, 8d8a0a0a6). Verify generated_policy_keeps_a_step_per_acceptance_criterion passes. Live: G1 and pocket-synth plans give each check its own step (up to 5 per task), all validating."
+++

## Problem

`PlanExecutionPolicy::generated` allows one verify step per task (`FAST_MAX_VERIFY_STEPS = 1`,
`require_one_verify = true`), and the planner's system prompt asks for "exactly one focused verify step" per task.
So a task with several acceptance criteria either fuses them into one `&&` chain, where the first failure hides the
rest, or drops them. In plans-research's G1 run (§3.2) gpt-oss-120b put seven "struct exists" greps and `cargo check`
in one chain, and none of the source's eight checks survived.

## Why it matters

Each acceptance criterion of a spec should be its own check, which fails and is reported on its own, so a retry and
the person both see which criterion failed (goal `golden-path`).

## Where

- `crates/roko-cli/src/plan_policy.rs::PlanExecutionPolicy::generated`, and `validate_plan_budgets`'s
  `PLAN_VERIFY_OWNER` rule.
- `crates/roko-cli/src/plan_generate.rs`: the planner's system prompt (its verify-step rule, the implementer role
  line, the checklist and the template guidance), and `render_tier_size_limits`.

## Plan

1. Let a generated task carry at least one verify step and at most its tier's limit (`TierSizeLimits`: mechanical 3,
   focused 4, integrative 6, architectural 8). The FAST lane keeps exactly one, and so does a plan generated for it.
2. Tell the planner to give each acceptance criterion its own verify step, naming it in `covers`, kept separate
   rather than chained; one command may cover several criteria when one run checks them all.
3. State each tier's verify-step limit in the prompt's size line, read from the policy.

## Done when

`[[verify]]` passes: a generated task with a step per acceptance criterion passes the generated-plan policy, a task
with none does not, and the FAST lane still allows one.
