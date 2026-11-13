+++
id = "bug-b364d9"
kind = "bug"
title = "A try the plan's budget stopped reads plain passed though the local-only check skipped its task's model checks (the LLM judge, a judge rung, the verify depth's B1 and B3): settle it not fully checked, with the reason, as the lead's approval of bug-3e04a6 requires"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
last_verified_rev = "48fb93ca2"
source = "round v8: the lead's condition approving bug-3e04a6's change to gap-2c9412 item 3"
discovered_from = "agent:team-lead (round v8)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/helper_calls.rs::HelperCalls", "crates/roko-cli/src/graph_task_dispatch/judge_step.rs::judge_attempt", "crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs::judge_rung", "crates/roko-cli/src/graph_task_dispatch/verify_depth.rs::deep_check"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-3e04a6", "gap-2c9412", "gap-6df2bf", "gap-85f102"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_try_the_budget_stopped_with_model_checks_skipped_is_not_fully_checked' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_try_the_budget_stopped"
+++

## Problem

Since bug-3e04a6 (merged 48456be7c), the task's local checks judge a try the plan's budget stopped, and no model is
called. The verdict is plain `passed` whenever the checks that ran pass, even when the task has model checks that
the local-only scope skipped:
- `[gates] llm_judge`: `judge_attempt` finds no helper model and does not judge;
- a pack's `judge` rung: `judge_rung` is skipped with "no helper model can judge";
- the verify depth's B1, the hidden suite, and B3, the cross-family review: `deep_check` skips them with "the plan's
  budget stopped the try, and this check calls a model".

The lead approved bug-3e04a6's change to gap-2c9412 on one condition. A try whose task has model checks that this
skips may not read plain "passed". It is settled as not fully checked, with the reason in its record ("model checks
skipped: the plan's budget was spent"), the way a self-checked step is shown. A try whose checks are all local, as
all of Undertow's are, is a real pass. The merged code does not meet this condition yet.

## Why it matters

- A "passed" that skipped the task's own judge or depth checks claims more than roko checked. Every label feeds the
  portal, Hermes, serve's run state and learning.
- Goal: truth.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`: `dispatch`, where a budget-stopped try's verdict settles.
- `crates/roko-cli/src/graph_task_dispatch/helper_calls.rs`: `HelperCalls`, the local-only scope.
- `crates/roko-cli/src/graph_task_dispatch/judge_step.rs`: `judge_attempt`.
- `crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs`: `judge_rung`.
- `crates/roko-cli/src/graph_task_dispatch/verify_depth.rs`: `deep_check`.
- `crates/roko-cli/src/graph_task_dispatch/streaming.rs`: the same on the streaming path.

## Plan

1. The local-only scope notes each model check it skips: the judge, a judge rung, B1 and B3.
2. A budget-stopped try whose checks pass but skipped one settles `unverified`. Its record gives the reason "model
   checks skipped: the plan's budget was spent", and names each check skipped, as a self-checked step names its run.
3. A try whose checks are all local passes as before.
4. Add the test `a_try_the_budget_stopped_with_model_checks_skipped_is_not_fully_checked`, with `[gates] llm_judge`
   on: the try reads unverified, with the reason. The existing test `a_try_the_budget_stopped_passes_when_its_checks_pass`
   covers the all-local case.

## Done when

- The `[[verify]]` command passes.

## Notes

- The lead's condition, approving bug-3e04a6's change to gap-2c9412's plan item 3 (round v8, 2026-11-13).
- Related: bug-3e04a6, gap-2c9412, gap-6df2bf (how a self-checked step is shown), gap-85f102 (the LLM judge).
