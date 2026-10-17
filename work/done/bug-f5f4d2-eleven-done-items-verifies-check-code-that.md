+++
id = "bug-f5f4d2"
kind = "bug"
title = "Eleven done items' verifies check code that has moved or are written wrongly, so the regression sweep reports 11 FAILs for fixes that still hold"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "M"
subsystem = ["tools/work"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "df35b36f4"
source = "status-quo-2026-10-15#finding-35"
discovered_from = "gap-6ee3c5: the verifies still failing after the superseded ones were retired"
anchors = ["work/done/bug-28c193-the-cli-error-hint-blames-the-api.md", "work/done/bug-5b897b-docs-06-a4-knowledge-sync-direction-unvalidated.md", "work/done/bug-7d7200-agents-and-verify-commands-inherit-roko-s.md", "work/done/bug-c8153a-fix-external-actions-safety-false-positive-requiretoolbefore.md", "work/done/bug-fade34-df-0918-issue-18-knowledge-query-results.md", "work/done/gap-28ceb9-dispatch-v2-has-no-provider-failover-and.md", "work/done/gap-853b31-frontier-planner-everywhere-an-authoring-planner-model.md", "work/done/gap-b07969-execute-resume-cannot-report-skipped-tasks-at-submit.md", "work/done/reg-3f5969-graph-dispatch-drops-selected-playbook-ids-outcomes.md", "work/done/bug-f1f814-graph-task-dispatch-s-fake-provider-clis-use.md", "work/done/gap-c8e1f1-split-graph-task-dispatch-rs-into-modules.md"]
links = { depends_on = [], blocks = [], related = ["gap-6ee3c5", "reg-48b7ba"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tools/work.py verify --static-only bug-28c193 bug-5b897b bug-7d7200 bug-c8153a bug-fade34 gap-28ceb9 gap-853b31 gap-b07969 reg-3f5969 bug-f1f814 gap-c8e1f1 && ! python3 tools/work.py verify --static-only bug-28c193 bug-5b897b bug-7d7200 bug-c8153a bug-fade34 gap-28ceb9 gap-853b31 gap-b07969 reg-3f5969 bug-f1f814 gap-c8e1f1 | grep -q 'nothing checks them now'"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T19:59:43Z"
commit = "df35b36f4"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-16T19:19:37Z"
forced = false
evidence = "df35b36f4: all 11 fixes still hold. Each stale verify is retired with retire-verify by the commit that moved or broke what it checked, and each item gets a new [[verify]] at the fix's current place, so none is left with every verify retired: bug-28c193, bug-5b897b, bug-fade34 by e80843ea0 (tests to main_tests.rs, enums to commands/knowledge.rs); bug-7d7200 by 74ad452f7 (env_clear in roko-core apply_gate_env); bug-c8153a by 4165f6485 (TOOL_HISTORY_SERVICE in safety/contract.rs); gap-28ceb9 by d562b56b2 (shared classify_failure_text); gap-853b31 verify 2 by 7f2776fc1 (prd.rs gone; verify 3 checks plan_generate/pipeline.rs); gap-b07969 by 5772b602b (routes/plans/run_control.rs); reg-3f5969 by a729fb911 (module split; 6b2118da1 deleted graph_execution/feedback.rs; the negative greps now cover the whole tree); bug-f1f814 verify 1 by df746d76c (substring matched connect_timeout_ms; verify 3 uses grep -w); gap-c8e1f1 by a3e9767fc (the cap counted mod tests; verify 2 counts lines before mod tests). Spot checks: the new bug-f1f814, reg-3f5969 and gap-c8e1f1 checks fail on the pre-fix code (f1db380c6~1, 763596768~1, a729fb911~1) and pass now. The item's verify passes in this worktree (every entry PASS or RETIRED, no 'nothing checks them now'); test_work.py 45 OK; check 0 problems."
+++

## Problem

`python3 tools/work.py verify --static-only bug-28c193 bug-5b897b bug-7d7200 bug-c8153a bug-fade34 gap-28ceb9 gap-853b31
gap-b07969 reg-3f5969 bug-f1f814 gap-c8e1f1` prints 11 FAIL lines, one per item, yet every one of these fixes still
holds. The 2026-10-15 assessment read the code at each new location
(`tmp/status-quo/2026-10-15/backlog/B-top-level-and-regressions.md`, part 2b): nine verifies grep a place the fix has
since moved out of (MOVED; for gap-853b31 that place is `crates/roko-cli/src/prd.rs`, which 7f2776fc1 deleted with the
PRD pipeline), and two are written wrongly (VERIFY-WRONG).

## Why it matters

Goal `tooling`: the regression sweep (`work.py verify --closed-since`) is only useful while a FAIL means a regression.
gap-6ee3c5 retired the 51 verifies that later changes made unrunnable. These 11 are most of what is left of the
assessment's 60, and none of them is a regression.

## Where

The 11 item files (the anchors). B's table gives the place each fix lives now:

| Item | Where the fix lives now (B, 2026-10-15) |
|---|---|
| bug-28c193 | `crates/roko-cli/src/main.rs` (`error_classify::mentions_http_401`); the test is in `crates/roko-cli/src/main_tests.rs` |
| bug-5b897b | `crates/roko-cli/src/commands/knowledge.rs` (`direction: KnowledgeSyncDirection`, a value_enum) |
| bug-7d7200 | `crates/roko-gate/src/gate_env.rs` calls `roko_core::child_env::apply_gate_env`, which runs `env_clear()` (`crates/roko-core/src/child_env.rs`) |
| bug-c8153a | `TOOL_HISTORY_SERVICE` in `crates/roko-agent/src/safety/contract.rs`; the dispatcher calls `contract::begin_tool_turn` |
| bug-fade34 | `crates/roko-cli/src/commands/knowledge.rs` (`verbose`, the two-line preview and the test) |
| gap-28ceb9 | `classify_provider_error` delegates to `roko_agent::provider::error_classify::classify_failure_text`; `detect_provider_exhaustion` is in roko-agent |
| gap-853b31 | verify 2's clause on `prd.rs`, which 7f2776fc1 deleted; its other two clauses and verify 1 still pass |
| gap-b07969 | `crates/roko-serve/src/routes/plans/run_control.rs` (`skippable_task_ids`); the test is in `routes/plans/tests.rs` |
| reg-3f5969 | `crates/roko-cli/src/graph_task_dispatch/feedback.rs` (the test, and `playbook_ids`) |
| bug-f1f814 | verify 1's `! grep -q 'timeout_ms: Some(5_000)'` matches `connect_timeout_ms: Some(5_000)`; use `grep -w` or anchor the pattern |
| gap-c8e1f1 | `wc -l < 3000` counts the inline `mod tests` (about 1,925 of 3,820 lines); count non-test lines, or move the tests out |

## Current state

Each item is done and its fix holds (B read the code). Nothing tracks the stale checks.

## Plan

For each item, confirm the fix at the place above. Then retire the stale entry with
`python3 tools/work.py retire-verify <id> --index N --reason "…" --by <the commit that moved or removed it>` and add a
new `[[verify]]` table that checks the new place (for a test, `grep -rqw 'fn <test>' crates/<crate>/ && cargo test …`).
For gap-853b31 and bug-f1f814, whose other verify still checks the fix, retiring the broken entry is enough; a corrected
replacement is better. Do not edit the old commands in place: they record what proved the fix when it closed.

## Done when

The `[[verify]]` command passes: no FAIL for these items, and none of them is left with every verify retired.

## Notes

Item files only (`work/done/`); no code changes. Find the moving commits with `git log --diff-filter=D -- <old path>`
or `git log -S'<symbol>'`. The rest of the 60: bug-d1dea8 (the stale route snapshot) is a real regression and is
reg-48b7ba's; gap-c4d630's stale claims aggregate is reg-3621b1's; bug-8417d9's verify passes again since reg-c7db5d.
Eight more failing done items outside the 60 are bug-ec244b's.
