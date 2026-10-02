+++
id = "gap-843aef"
kind = "gap"
title = "PK05 Operator control and guards: Provider CLIs and MCP servers keep exported secrets whose names roko does not recognise (+7 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
rank = 5
size = "L"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "d426c86d7"
source = "tmp/backlog/2026-10-02-complete-and-wire PK05"
anchors = ["apps/portal/src/api/contracts.ts", "apps/portal/src/api/queries.ts", "apps/portal/src/components/stage/TaskList.tsx", "crates/roko-agent/src/exec.rs", "crates/roko-agent/src/provider/claude_cli.rs", "crates/roko-agent/src/safety/sandbox.rs", "crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs", "crates/roko-cli/src/graph_task_dispatch/failover.rs", "crates/roko-cli/src/tui/app/actions.rs", "crates/roko-cli/src/tui/app/channels.rs", "crates/roko-cli/src/tui/app/tests.rs", "crates/roko-cli/src/tui/state/mod.rs", "crates/roko-core/src/child_env.rs", "crates/roko-core/src/config/schema.rs"]
lane = "rust-hot"
parent = "spec-65c828"
links = { depends_on = ["gap-198c9c"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn credential_scrub_strips_unknown_secret_names' crates/roko-core/ && cargo test -p roko-core credential_scrub_strips_unknown_secret_names"

[[verify]]
command = "grep -rqw 'fn codex_keeps_workspace_sandbox_with_skip_permissions' crates/roko-agent/ && cargo test -p roko-agent codex_keeps_workspace_sandbox_with_skip_permissions"

[[verify]]
command = "grep -rqw 'fn codex_agent_cannot_read_key_files' crates/roko-agent/ && cargo test -p roko-agent codex_agent_cannot_read_key_files"

[[verify]]
command = "grep -rqw 'fn unguarded_cli_agents_never_run_in_the_shared_checkout' crates/roko-cli/ && cargo test -p roko-cli unguarded_cli_agents_never_run_in_the_shared_checkout"

[[verify]]
command = "grep -rqw 'fn held_task_snapshot_shows_awaiting_approval' crates/roko-cli/ && cargo test -p roko-cli held_task_snapshot_shows_awaiting_approval"

[[verify]]
command = "grep -rqw 'fn approve_command_records_the_held_tasks_review' crates/roko-cli/ && cargo test -p roko-cli approve_command_records_the_held_tasks_review"

[[verify]]
command = "grep -rqw 'fn tui_offers_a_held_task_for_approval' crates/roko-cli/ && cargo test -p roko-cli tui_offers_a_held_task_for_approval"

[[verify]]
command = "test -f apps/portal/src/components/stage/ReviewPane.accept.test.tsx && grep -rqE 'tasks/.*/review|/diff' apps/portal/src --include='*.ts*' && cd apps/portal && npx vitest run src/components/stage/ReviewPane.accept.test.tsx"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T19:08:26Z"
commit = "d426c86d7"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-02T16:02:51Z"
forced = false
evidence = "Gate 3b on work/backlog-batch-3b (merged into main as d426c86d7, tree identical to the gated one): cargo check --workspace --tests, clippy -D warnings (roko-agent/cli/core/learn/serve), nextest --lib 10,077 passed, golden-path canaries 13/13, portal vitest 4/4 + tsc --noEmit, ViabilityBench verifier CI (f2,f3,f5,f7,f8) and audit pytest; every [[verify]] passes."
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK05, slice 12xx, phase 1), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 1212 | M | p2 | Provider CLIs and MCP servers keep exported secrets whose names roko does not recognise | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1212-provider-clis-and-mcp-lose-unknown-secret-names.md` |
| 2 | 1213 | M | p2 | dangerously_skip_permissions turns off Codex's own OS sandbox, and SandboxLevel::Restrict permits it | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1213-codex-keeps-its-workspace-sandbox-under-skip-permissions.md` |
| 3 | 1215 | S | p2 | The Codex stream broker never checks command text, so a Codex agent can read key files | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1215-codex-broker-checks-command-text-with-rokos-guards.md` |
| 4 | 1216 | M | p2 | Codex, Cursor and Gemini CLI agents run unguarded in the operator's shared checkout | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1216-unguarded-cli-agents-never-run-in-the-shared-checkout.md` |
| 5 | 1217 | S | p3 | A held task shows as a failed review gate while it waits for approval | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1217-held-task-shows-awaiting-approval-not-a-failed-gate.md` |
| 6 | 1218 | S | p2 | A Graph run rejects Approve and Reject commands for a held task | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1218-graph-run-takes-approval-from-approve-commands.md` |
| 7 | 1219 | M | p2 | The TUI cannot approve or reject a held task | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1219-tui-offers-a-held-task-for-approval.md` |
| 8 | 1220 | M | p2 | Held tasks cannot be reviewed from the portal | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1220-portal-reviews-held-tasks.md` |

## Why it matters

Phase 1: safe runs. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1200-operator-control-held-items-off-secrets-and-guards.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `apps/portal/src/api/contracts.ts`, `apps/portal/src/api/queries.ts`, `apps/portal/src/components/stage/ReviewPane.accept.test.tsx`, `apps/portal/src/components/stage/ReviewPane.tsx`, `apps/portal/src/components/stage/TaskList.tsx`, `crates/roko-agent/src/exec.rs`, `crates/roko-agent/src/provider/claude_cli.rs`, `crates/roko-agent/src/safety/sandbox.rs`, `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs`, `crates/roko-cli/src/graph_task_dispatch/failover.rs`, `crates/roko-cli/src/tui/app/actions.rs`, `crates/roko-cli/src/tui/app/channels.rs`, `crates/roko-cli/src/tui/app/tests.rs`, `crates/roko-cli/src/tui/state/mod.rs`, `crates/roko-core/src/child_env.rs`, `crates/roko-core/src/config/schema.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

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

- Waits on: PK04 (gap-198c9c).
- Suggested model: opus.

## Progress

Implemented on `work/gap-843aef`; cargo verification deferred to the batch gate. The portal change (1220) was written
by hand: npm, vitest and `tsc` were not run.

- 1212: implemented at d75891cb3
- 1213: implemented at e847cea8b
- 1215: implemented at c29d33858
- 1216: implemented at a97022723
- 1217: implemented at de9465618
- 1218: implemented at d9e1acadc
- 1219: implemented at 073ca943c
- 1220: implemented at c77015269
