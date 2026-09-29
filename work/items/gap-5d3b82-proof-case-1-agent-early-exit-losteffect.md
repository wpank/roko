+++
id = "gap-5d3b82"
kind = "gap"
title = "Proof Case 1: Agent early exit / LostEffect"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "core"
subsystem = ["roko-cli/runner"]
created = 2026-09-01
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.1 Proof Case 1: Agent early exit / LostEffect"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.1 Proof Case 1: Agent early exit / LostEffect"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs:1014", "crates/roko-agent/src/claude_cli_agent.rs:1049", "crates/roko-agent/src/exec.rs", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::reconcile_attempt", "crates/roko-cli/src/graph_execution/agent_slots.rs::AgentSlotDispatcher", "crates/roko-runtime/src/run_ledger.rs:523"]
links = { depends_on = [], blocks = [], related = ["gap-01b2ff"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn agent_exit_before_first_event_fails_the_attempt_promptly' crates/roko-cli/ && cargo test -p roko-cli agent_exit_before_first_event_fails_the_attempt_promptly && grep -rqw 'fn exited_agent_with_open_stdout_does_not_hang' crates/roko-agent/ && cargo test -p roko-agent exited_agent_with_open_stdout_does_not_hang"
+++

## Problem

Proof case 1 from the dogfood audit has never been shown on the Graph engine. The case: the agent process
exits before it emits its first message or tool event (crash on start, bad flag, auth failure, killed). The
run must then:

- notice promptly (not wait for the task timeout);
- free the agent slot;
- record a terminal failed attempt with a readable reason in the Graph checkpoint;
- retry or fail the task within its deadline.

The item was written for Runner-v2, whose run ledger classified this as a `LostEffect` timeout terminal.
Runner-v2 was deleted on 2026-09-06 (`6b5da8616`), and the Graph path never uses that ledger. So "detect
`LostEffect`" is the wrong target; the observable behaviour above is the right one.

In dogfood (2026-08-26 debrief F7: "agent process dies silently, runner waits forever"; F2: "runner hangs after
agent completion") the old runner hung in these situations. Nobody has shown whether the Graph path handles
them.

## Why it matters

Goal `core`. A run that hangs or burns its whole task timeout when a provider dies on start blocks the plan,
holds an agent slot (`[conductor] max_agents`), and needs an operator. Related:

- `gap-01b2ff` (parked): crash/resume and provider proof matrices, runner hang after worktree edits, agent
  death before the first event;
- `gap-415c54`: proof case 2, normal diff + gate + merge;
- `gap-161be1`: proof case 3, baseline filtering.

## Where

- `crates/roko-agent/src/claude_cli_agent.rs`, the Claude CLI provider:
  - spawns the CLI in its own process group (`set_process_group`, :414);
  - waits with `timeout(self.timeout_ms, child.wait())` (:1014);
  - then awaits the stdout/stderr reader tasks with no deadline (`stdout_handle.await`, :1049-1050);
  - a non-zero exit becomes `failure("exit <code>: <reason>")` (:1060-1071), and an empty output becomes
    "claude produced an empty response" (:1081-1093).
- `crates/roko-cli/src/graph_task_dispatch.rs`, `GraphTaskDispatcher`:
  - sets the agent timeout from `spec.timeout_secs`, falling back to `timeouts.agent_dispatch_secs`
    (:3626-3631);
  - turns an unsuccessful provider result into `RokoError::Agent` (about :4556);
  - `reconcile_attempt` (:4568) handles crash/resume idempotence: a previous attempt with started but no
    terminal evidence gives `AttemptReconciliation::FailAmbiguous`. This is the Graph-path analog of
    `LostEffect` for a crashed runner.
- `crates/roko-graph/src/cells/task_executor.rs`: `TaskExecutorCell` retries a failed attempt up to the task's
  `max_retries`, then fails the task.
- `crates/roko-cli/src/graph_execution/agent_slots.rs::AgentSlotDispatcher`: the `[conductor] max_agents`
  slots. A slot is freed when the inner dispatch returns.
- `crates/roko-runtime/src/run_ledger.rs:523` (`TimeoutTerminalKind::LostEffect`) and
  `crates/roko-cli/src/runner/types.rs:714`: Runner-v2-era types with no Graph caller.
  `crates/roko-core/src/config/timeouts.rs` `lost_effect_secs` has no reader.

Entry point: `roko plan run <dir>` → Graph engine → `TaskExecutorCell` → `AgentSlotDispatcher` →
`GraphTaskDispatcher` → provider (`ClaudeCliAgent`).

## Current state

Checked at `a17d9d766`:

- An early non-zero exit or an empty output already becomes a failed provider result, so the Graph attempt
  fails and is retried. No test proves the whole chain for an exit before the first event: prompt failure,
  slot released, terminal recorded in `checkpoint.json` / `activities.jsonl`, retry count respected, no
  orphan process.
- Known risk, unproven: after `child.wait()` returns, the reader tasks are awaited without a deadline. If the
  CLI exits but a grandchild in its process group (a background shell, an MCP server) still holds
  stdout/stderr, the reader never sees EOF, and the attempt could hang past the task timeout. That matches
  dogfood F2.
- Related tests already in `graph_task_dispatch.rs`:
  - `streaming_verify_failure_is_a_failed_terminal_attempt`;
  - `streaming_dispatch_settles_cost_on_provider_failure`;
  - `exhausted_provider_without_usable_fallback_fails_once_with_fix_hint`;
  - `reconcile_returns_fail_ambiguous_for_started_evidence`;
  - fake providers are written as `fake-claude.sh` scripts in temp dirs (see the tests near :5994 and :6114).

## Plan

1. Add a Graph-path test, `agent_exit_before_first_event_fails_the_attempt_promptly`, next to the existing
   fake-claude tests in `graph_task_dispatch.rs`. Use a fake `claude` that exits 1 at once with no stdout (and
   a variant exiting 0 with no output) and a task with `max_retries = 1` and a long timeout. Assert:
   - each attempt fails in well under the timeout;
   - the reason names the exit;
   - there are exactly `max_retries + 1` provider invocations;
   - the task ends failed with a terminal record;
   - a second task behind a `max_agents = 1` slot still runs (slot released).
2. Add `exited_agent_with_open_stdout_does_not_hang` in `roko-agent`. A fake `claude` starts
   `sleep 600 &` (inheriting stdout), then exits. If it hangs:
   - after `child.wait()` returns, drain the readers with a bounded grace period (a few seconds);
   - on expiry, `kill_tree` the process group and return a failure such as "agent exited but its output pipe
     stayed open".

   Check the other CLI providers (`codex_agent.rs`, `cursor_cli_agent.rs`, `gemini/`) and the shared runner
   in `crates/roko-agent/src/exec.rs` (which also awaits `stdout_handle`) for the same pattern and apply
   the same fix.
3. Do not add `LostEffect` to the Graph path for its own sake. If a named terminal kind is wanted in the
   checkpoint, map this case to a provider-failure class (for example "agent exited before first event") in
   the failure reason, so `roko diagnose` and the dashboard can show it.
4. Record one live proof as closing evidence. In a scratch repo, point `[providers.claude_cli] command` at a
   script that exits at once, run `roko plan run <scratch-plan>`, and record the elapsed time, the attempt
   count and the checkpoint terminal.

## Done when

- A Graph task whose agent exits before its first event:
  - fails each attempt within seconds;
  - is retried exactly `max_retries` times, then fails with a reason naming the early exit;
  - leaves a terminal record in the plan checkpoint;
  - frees its agent slot.
- An agent that exits while a grandchild holds its stdout does not hang the attempt, and the grandchild is
  killed.
- The suggested verify passes:
  `grep -rqw 'fn agent_exit_before_first_event_fails_the_attempt_promptly' crates/roko-cli/ && cargo test -p roko-cli agent_exit_before_first_event_fails_the_attempt_promptly && grep -rqw 'fn exited_agent_with_open_stdout_does_not_hang' crates/roko-agent/ && cargo test -p roko-agent exited_agent_with_open_stdout_does_not_hang`.
  The current `[[verify]]` (the word `LostEffect` appearing under `graph_execution/` or in
  `graph_task_dispatch.rs`) tests a Runner-v2 name, not the behaviour, and a comment would satisfy it.

## Notes

- Process-kill code is risky. Kill only the agent's own process group (`kill_tree` on the tracked child), never
  by name, and keep the PID registry (`register_spawned_pid` / `unregister_pid`) consistent.
- `graph_task_dispatch.rs` is a hot file. Coordinate with `gap-161be1`, `gap-415c54` and `gap-4ec59f` if they
  run at the same time. The `roko-agent` part (step 2) is independent and can land separately.
- Crash-of-the-runner itself (as opposed to crash of the agent) is covered by `reconcile_attempt` and resume.
  It is out of scope here (see `gap-01b2ff`).

## Original notes

Agent exits before its first message/tool event; runner detects `LostEffect`, releases capacity, persists a terminal, and either retries or fails within the configured deadline.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.1 Proof Case 1: Agent early exit / LostEffect`

How to verify: Source: Dogfood audit, proof case 1. Check the described code path for: Agent exits before its first message/tool event; runner detects `LostEffect`, releases capacity, persists a terminal, and either retries or fails within the…

Verified 2026-09-28: still open - LostEffect exists only in roko-runtime/src/run_ledger.rs:523/839 and the legacy runner/types.rs; the Graph path (graph_execution/, graph_task_dispatch.rs, commands/plan.rs) never uses the run ledger, so an agent that exits before its first event gets no LostEffect terminal.
