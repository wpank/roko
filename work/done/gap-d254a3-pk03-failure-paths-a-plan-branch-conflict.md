+++
id = "gap-d254a3"
kind = "gap"
title = "PK03 Failure paths: A plan-branch conflict tells the next attempt what it conflicted with (+5 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
rank = 3
size = "L"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "059450273"
source = "tmp/backlog/2026-10-02-complete-and-wire PK03"
anchors = ["crates/roko-cli/src/graph_execution/workspaces.rs", "crates/roko-cli/src/graph_task_dispatch/attempt.rs", "crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs", "crates/roko-cli/src/graph_task_dispatch/failover.rs", "crates/roko-cli/src/graph_task_dispatch/red_flags.rs", "crates/roko-cli/src/graph_task_dispatch/watchdog.rs", "crates/roko-cli/src/orchestrator/worktree/mod.rs", "crates/roko-cli/src/runner/tui_bridge.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-learn/src/telemetry/records.rs"]
lane = "rust-hot"
parent = "spec-65c828"
links = { depends_on = ["gap-e00238"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn conflict_retry_prompt_names_the_conflict' crates/roko-cli/ && cargo test -p roko-cli conflict_retry_prompt_names_the_conflict"

[[verify]]
command = "grep -rqw 'fn outside_scope_ignores_sibling_accepted_commits' crates/roko-cli/ && cargo test -p roko-cli outside_scope_ignores_sibling_accepted_commits"

[[verify]]
command = "grep -rqE 'scope_findings|out_of_scope' crates/roko-learn/src/telemetry/records.rs"

[[verify]]
command = "grep -rqw 'fn scope_findings_reach_the_attempt_verdict' crates/roko-cli/ && cargo test -p roko-cli scope_findings_reach_the_attempt_verdict"

[[verify]]
command = "grep -rqw 'fn hung_codex_attempt_is_cancelled_by_stall_watchdog' crates/roko-cli/ && cargo test -p roko-cli hung_codex_attempt_is_cancelled_by_stall_watchdog"

[[verify]]
command = "grep -rqw 'fn agent_spawned_names_planned_provider' crates/roko-cli/ && cargo test -p roko-cli agent_spawned_names_planned_provider"

[[verify]]
command = "grep -rqw 'fn failover_publishes_fallback_slug_to_hub' crates/roko-cli/ && cargo test -p roko-cli failover_publishes_fallback_slug_to_hub"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T04:05:57Z"
commit = "059450273"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-03T01:27:37Z"
forced = false
evidence = "Gate 6b (work/backlog-batch-6b, merged into main as 059450273): cargo check --workspace --tests, nightly fmt, cargo clippy --workspace -D warnings on the chainless default build, nextest --lib 11,842 passed over 10 crates (roko-acp, -agent, -agent-server, -cli, -core, -gate, -graph, -learn, -runtime, -serve), roko-cli bin 429 passed and the golden-path canaries pass (plan_validate: only bug-2a31bc's two known alias tests fail), roko-agent sse_replay + provider_parity and roko-learn legacy_rule_live + loop_audit_cs_reference pass, PK79's CI feature checks pass; every [[verify]] passes. PK03 6/6. Gate fixes in 68af1c4de: the pre-dispatch agent_spawned row names the resolved slug as the failover row does; the watchdog test opts into allow_unguarded_agents_in_checkout; the conflict-retry provider records its arguments (where the retry feedback travels)."
+++

## Problem

This package delivers 6 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK03, slice 11xx, phase 1), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 1123 | S | p2 | A plan-branch conflict tells the next attempt what it conflicted with | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1123-conflict-retry-names-the-conflict.md` |
| 2 | 1124 | S | p2 | An attempt's diff starts from the commit its worktree was created from, so siblings' accepted work is not flagged | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1124-attempt-diff-base-is-worktree-start.md` |
| 3 | 1125 | M | p3 | Out-of-scope findings are recorded on the attempt verdict, not only logged | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1125-persist-scope-findings-on-verdicts.md` |
| 4 | 1126 | M | p2 | The stall watchdog bounds Codex CLI and Cursor CLI attempts that report nothing until they finish | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1126-watch-cli-attempts-that-report-at-end.md` |
| 5 | 1127 | S | p3 | The pre-dispatch `agent_spawned` event names the planned provider, not a backend family | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1127-agent-spawned-names-planned-provider.md` |
| 6 | 1128 | S | p3 | After a failover the dashboard shows the model and provider that ran | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1128-failover-updates-dashboard-model.md` |

## Why it matters

Phase 1: safe runs. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1100-make-provider-failures-safe.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/graph_execution/workspaces.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs`, `crates/roko-cli/src/graph_task_dispatch/failover.rs`, `crates/roko-cli/src/graph_task_dispatch/red_flags.rs`, `crates/roko-cli/src/graph_task_dispatch/watchdog.rs`, `crates/roko-cli/src/orchestrator/worktree/mod.rs`, `crates/roko-cli/src/runner/tui_bridge.rs`, `crates/roko-core/src/config/schema.rs`, `crates/roko-learn/src/telemetry/records.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK02 (gap-e00238).
- Suggested model: opus.

## Progress

Implemented on `work/gap-d254a3` from BASE e55d4c20f; cargo verification deferred to the batch gate.

- 1123: implemented at 537ee1188. The conflict arm of `accept_attempt` records retry feedback: the refusal's reason,
  then the conflicting paths it names (`WorktreeError::Conflict` words them), else the paths the attempt changed (from
  the pre-verify screen's `take_changed_files`). It stays raw text (`from_raw`, lifted lists cleared), so a path that
  reads like a failing test does not hide the rest. The stub-conflict test now also checks the changed-paths fallback.
- 1124: implemented at e0630d3e7. `WorktreeHandle::base_commit` holds the commit `create_locked` resolved for the
  checkout; `lease_from_handle` reports it, falling back to the configured base for a re-attached checkout (start
  left unknown, as the spec allows). The bug hides behind a `base_branch` of `HEAD`, which resolves inside the
  worktree, so the test uses `main`.
- 1125: implemented at cb7a71ef0. `AttemptVerdictRecord::scope_findings` ({path, kind}, at most 50, with
  `scope_findings_omitted`; both skipped when empty) and `set_scope_findings` keep the cap in one place. The findings
  travel from `attempt_diff_red_flag` through `screen_attempt` and `VerificationReport` to the attempt, so this also
  touched `verification.rs`, `streaming.rs`, `graph_task_dispatch.rs` (one line each) and `telemetry/mod.rs`.
- 1126: implemented at 6f4707337. Option A: every call's silence counts from its start once a first-output grace has
  passed, with `[conductor] report_at_end_stall_secs` (default 900, 0 = the old behaviour) as the grace of kinds that
  may report only at the end. This bounds a first API call (OpenAI-compatible, Anthropic) that reports nothing too.
- 1127: implemented at 1013f307f. The pre-dispatch `agent_spawned` names
  `ProviderDispatchResolver::resolve(request.model_key).provider_id`, what `resolve_candidate` (private to `failover`)
  returns for the planned key; an unresolved key keeps the kind label.
- 1128: implemented at 8c7d3e97e. The hub upserts a second `AgentSpawned` for a running id (model and provider
  replaced, no second active count), so no new event: `run_bridge_with_failover` takes a `DashboardRow` (plan/cell id
  and role) and republishes it with the model and provider that run once failover has passed a model over. Side
  effect: the run event log's `total_agent_calls` counts that republish too.
