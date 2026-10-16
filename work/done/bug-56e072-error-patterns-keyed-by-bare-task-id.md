+++
id = "bug-56e072"
kind = "bug"
title = "Error patterns keyed by bare task id leak into other plans' tasks with the same id: key them by plan and task"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
size = "S"
subsystem = ["roko-learn/error_pattern_store"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "c3fb80cc5"
source = "status-quo-2026-10-15#finding-6"
anchors = ["crates/roko-learn/src/error_pattern_store.rs::keyed_to", "crates/roko-learn/src/error_pattern_store.rs::relevance_score", "crates/roko-learn/src/error_pattern_store.rs::ErrorPattern", "crates/roko-learn/src/error_pattern_store.rs::FailurePatternQuery", "crates/roko-cli/src/dispatch/factory.rs::error_patterns_for_task"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn keyed_summary_skips_the_same_task_id_in_another_plan' crates/roko-learn/ && cargo test -p roko-learn keyed_summary_skips_the_same_task_id_in_another_plan"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T01:21:05Z"
commit = "c3fb80cc5"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-15T16:33:44Z"
forced = false
evidence = "c3fb80cc5: ErrorPattern::task_keys records each observation's <plan>/<task> (serde default); keyed_to and relevance_score count the same task only for the same key (seen_on_task), and a query without a plan matches no task. The same-verify-command rule and plan_ids/task_ids are unchanged, so graduation (4128) and the planner block (4126) count plans as before. Old patterns without keys still load: one that saw exactly one plan gets that plan paired with each of its task ids on load; one seen in several plans gets no key and is selected by its verify command only. Checks, on this worktree's own target through heavy.sh: the verify passes (cargo test -p roko-learn keyed_summary_skips_the_same_task_id_in_another_plan, which failed before the fix: plan B's T1 got plan A's pattern); cargo test -p roko-learn --lib error_pattern_store 23/23, including load_keys_an_old_patterns_tasks_only_when_it_saw_one_plan and graduation_candidates_need_recurrence_and_a_resolution; cargo test -p roko-cli --test prompt_relevance_canary 2/2 with a new seeded pattern from another plan's T01 (4220); cargo test -p roko-compose --lib failure_patterns_bidder cold_start_static_bidders 2/2; cargo clippy -p roko-learn --no-deps -- -D warnings clean; cargo +nightly fmt --all. Filed gap-94cef2 (stale module doc names)."
+++

## Problem

A discovered error pattern is injected into the prompt of any task that shares its bare task id, in any plan.
`ErrorPattern::keyed_to` (`crates/roko-learn/src/error_pattern_store.rs:848-859`) counts a pattern as "the same task"
when `self.task_ids.contains(task_id)`, ignoring the plan; `relevance_score` (:819-829) adds 8 on the same bare match.
Plan task ids like `T1` repeat in almost every plan. The pattern keeps `plan_ids` and `task_ids` as two separate sets
(:131-135), so the pair is not even stored.

Seen live (A05 §4 S1, tally runs 2026-10-14): one pattern from run `223fe7e3`, task `T1` (a REPORT.md sign-off
rejection), was injected into `T1` of four other plans (`tally-budgets-3/4/5`, `35b94652`) on 5 `error_patterns`
decision rows. Expected: a pattern is keyed to the same (plan, task), or to one of the task's own verify commands.

## Why it matters

Goal `learning` (L13, discovered error patterns). A wrong pattern in a prompt is misleading guidance: it tells the
agent about a failure of another plan's work. NEW-FINDINGS finding 6. The relevance canary (4220) cannot catch it:
its task ids (`T01`/`T02`/`T03`) all differ. Stage 1 of the epic: the shadow data M1 and M3 gather in stage 2 should
come from prompts without this leak.

## Where

- `crates/roko-learn/src/error_pattern_store.rs::keyed_to` and `relevance_score`: the selection.
- `crates/roko-learn/src/error_pattern_store.rs::ErrorPattern` (`plan_ids`, `task_ids`) and the observation fold
  (:470-505) that fills them.
- `crates/roko-learn/src/error_pattern_store.rs::FailurePatternQuery`: already carries `plan_id`.
- Callers pass the plan already: `crates/roko-cli/src/dispatch/factory.rs::error_patterns_for_task`
  (`plan_id: Some(plan_id)`), reached from `graph_task_dispatch/verification.rs::task_error_patterns`.

## Current state

4209 (keyed selection) and 4210 (inject only keyed patterns) are done; the plan was simply never part of the key.

## Plan

1. Record the (plan id, task id) pair on each observation, for example a `task_keys: BTreeSet<String>` of
   `"<plan>/<task>"` with `#[serde(default)]`. For an old pattern with no pairs: if it has exactly one plan id, pair
   that plan with each task id on load; otherwise it matches by verify command only.
2. In `keyed_to` and `relevance_score`, "same task" means the same pair. The same-verify-command rule stays.
3. Test `keyed_summary_skips_the_same_task_id_in_another_plan`: a pattern seen on plan A `T1` is selected for A `T1`,
   skipped for plan B `T1` with a different verify command, and still selected for plan B `T1` when B's verify command
   is the pattern's gate.
4. Add a canary case with a repeated task id in another plan to the relevance canary (4220).

## Done when

The `[[verify]]` command passes, and a planner block or graduation listing (4126, 4128) still counts plans as before.

## Notes

F1 skips this finding; F2 owns it. `error-patterns.json` is shared across plans by design; do not split the file.

2026-10-16 (claude-agent, `work/bug-56e072`), how it was done and what old patterns do:

- Each observation with a task records `"<plan>/<task>"` in `ErrorPattern::task_keys` (`#[serde(default)]`), the
  form Graph dispatch keys a plan's task by. `plan_ids` and `task_ids` keep their meaning and fill as before, so
  `graduation_candidates` (4128), the planner block (4126) and `roko-mcp-code`'s pass-through count plans as before.
- `keyed_to` and `relevance_score` count "same task" only for the same key (`ErrorPattern::seen_on_task`). A query
  with a task but no plan matches no pattern's task; the one prompt caller, `error_patterns_for_task`, always passes
  the plan. The same-verify-command rule is unchanged.
- **Old, unkeyed patterns** (saved before `task_keys`): `load` keeps reading them. A pattern that saw exactly one
  plan gets that plan paired with each of its task ids, since every task id it holds was that plan's. A pattern
  that saw several plans gets no key, because which plan's task hit it is unknown: only its verify command selects
  it for a prompt, never a bare task id. A later observation adds its own key as usual. Binaries without this
  change ignore the new field.
- Canary (4220): `prompt_relevance_canary.rs` seeds a pattern another plan's `T01` hit, with a command no task runs;
  run 1's `T01` must show no pattern on its first attempt and no prompt may show that command.
