+++
id = "gap-f118b3"
kind = "gap"
title = "Deliver `roko inject` Through the Canonical Acknowledged Control Transport"
status = "done"
triage = "verified"
severity = "p1"
size = "L"
goal = "features"
subsystem = ["roko-cli/inject"]
created = 2026-09-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "c9272dde2"
source = "tmp/backlog/archive/361-inject-acknowledged-control-transport.md#361 — Deliver `roko inject` Through the Canonical Acknowledged Control Transport"
discovered_from = "audit:tmp/backlog/archive/361-inject-acknowledged-control-transport.md#361 — Deliver `roko inject` Through the Canonical Acknowledged Control Transport"
anchors = ["crates/roko-cli/src/commands/util.rs::cmd_inject", "crates/roko-cli/src/inject.rs::InjectRequest", "crates/roko-cli/src/execution_control.rs::ExecutionCommandKind", "crates/roko-cli/src/graph_execution/control_adapter.rs", "crates/roko-cli/src/runner/types.rs::ControlCommand", "crates/roko-cli/src/main.rs:5105"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn inject_fails_without_executor_ack' crates/roko-cli/src/ && cargo test -p roko-cli --lib --bin roko inject_fails_without_executor_ack"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T08:47:37Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "L"
claimed_at = "2026-10-01T16:12:55Z"
forced = false
evidence = "roko inject delivers through a per-run owner-only socket with a token handshake (transport b). The run acks Accepted once it queues the directive, the next dispatched prompt carries it once under Operator directive, duplicates are not redelivered, abort cancels only the addressed run, and every failure exits non-zero, writing nothing. inject_fails_without_executor_ack and the new transport, directive and log tests pass (wk-childenv 26c60b5a3); serve's REST route (c) is noted as a follow-up; gate 6j passed at b35631307 (cargo check, clippy -D warnings, lib tests of roko-cli 3,460, roko-graph 491, roko-serve 1,018, canaries C1-C8 and integration tests, graph_budget_resume, resume_cycle_e2e, graph_plan_callers, 448 bin tests, run_evidence py); merged in c9272dde2"
+++

## Problem

`roko inject <session> <payload> [--kind directive|context|abort]` reports success for a message nobody
receives. Run it in any workspace, with or without a live plan run: it prints `Injected directive ->
session <s> (control action: Resume)` (JSON: `{"code":"inject_delivered",..}`) and exits 0. What it
actually does:

- directive/context: writes `.roko/state/inject.json` (kind, session, payload, timestamp), then writes
  `.roko/state/control.json` with `ControlAction::Resume`;
- abort: writes `.roko/state/control.json` with `ControlAction::Cancel` (a whole-run cancel, if anything
  read it);
- the `session` argument targets nothing (`plan_id: None`, `task_id: None`).

Nothing reads either file: `ControlCommand::poll` has no caller, and there is no reader of
`inject.json`. Expected (backlog #361): an operator message reaches the named live executor exactly once,
and the command returns 0 only after that executor acknowledges it; otherwise a non-zero, typed error.

## Why it matters

Goal `features`, but the false success is a correctness and safety defect: scripts, CI and editor
integrations take exit 0 as "delivered", and `--kind abort` does not stop anything. Related:
`bug-8208a6` (the same unread `control.json` makes `roko plan pause/resume/cancel/retry` no-ops for
standalone runs; the two should share one transport), `gap-1555ac` (client TUI keys not forwarded to a
server-owned run).

## Where

- `crates/roko-cli/src/commands/util.rs::cmd_inject` (line 1551): the handler described above (bin
  target: `commands` is a module of `src/main.rs`). Clap definition `Command::Inject` in `main.rs` line
  1071.
- `crates/roko-cli/src/inject.rs`: `InjectKind` (Directive, Abort, Context), `InjectRequest` with
  `validate()` and an unused `socket_path()` (`.roko/run/roko-<session>.sock`, never bound).
- `crates/roko-cli/src/runner/types.rs::ControlCommand`: `write`/`poll` of `.roko/state/control.json`;
  `poll` has no caller.
- `crates/roko-cli/src/execution_control.rs`: the #233 executor-neutral channel. `ExecutionCommand {
  command_id, correlation_id, run_id, plan_id, task_id, attempt, issued_at_ms, kind }`,
  `ExecutionCommandKind` (Pause, Resume, SoftRetry, Repair, ReverifyGates, Skip, Cancel, Approve,
  RejectApproval, Reset; no inject variant), `CommandAck`/`CommandAckStatus`, `ExecutionCommandSender`,
  `control_command_to_execution` (test-only use). `FakeExecutionCommandAdapter` is in its `#[cfg(test)]`
  module.
- `crates/roko-cli/src/graph_execution/control_adapter.rs`: #255 adapter, maps `ExecutionCommandKind`
  to `roko_graph` control and effects back to `CommandAck`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs` about lines 1188-1192 and 1777-1834: the only live
  channel, `ExecutionCommandSender::channel("graph-engine")`, in-process between the TUI and the run.
- Possible ingress points: `crates/roko-cli/src/daemon.rs` (`daemon.sock`; `DaemonCmd` has Status, Stop,
  Restart, Reload and subscription commands, and no registry of live plan runs);
  `crates/roko-cli/src/state_hub_ipc.rs` (`.roko/runtime/hub.sock`, one-way event stream, 0600,
  length-prefixed JSON frames, started only by `roko serve`/`roko up`); `crates/roko-serve/src/routes/plans.rs`
  (`/plans/{id}/pause|resume|cancel` for server-owned runs).

## Current state

- #325 (2026-09-03) made inject fail closed (`inject_transport_unavailable`, non-zero, no I/O). Commit
  `cf598e0f2` (2026-09-06) replaced that with the file transport above and flipped the tests: the
  `inject_fail_closed_*` tests in `src/main.rs` (about lines 5105-5222) now assert `EXIT_SUCCESS` and that
  `control.json` exists. This is a regression of #325's contract ("a command that delivered nothing may
  never print queued, accepted, or any other success-shaped response").
- #233 (command/ack types) and #255 (Graph control adapter) exist, so the original blockers are gone; what
  is missing is cross-process ingress to a live run and an executor-side consumer for directive/context.
- The frozen #361 contract (`DaemonControlRequestV1`/`DaemonControlAckV1` over `.roko/daemon.sock`)
  assumed the daemon knows live executors. It does not, and no such types exist.

## Plan

1. Fail closed now (small, independent): after `request.validate()`, return non-zero with code
   `inject_transport_unavailable` and the hint "No live command transport is installed; use plan
   pause/cancel controls where applicable." Write no files. Text to stderr; in `--json` one error object
   (`code`, `message`, `hint`). Rewrite the `inject_fail_closed_*` tests to assert failure and no
   `control.json`/`inject.json`, and add `inject_fails_without_executor_ack`.
2. Choose the transport (record the decision in this item):
   - (a) #361 as written: daemon ingress. Plan runs would have to register with the daemon and the daemon
     relay commands and acks. The daemon is optional and knows nothing of runs today. Most work.
   - (b) a per-run control socket bound by `roko plan run` (for example `.roko/runtime/control.sock`,
     0600, the `state_hub_ipc` framing) that forwards `ExecutionCommand` into the run's
     `ExecutionCommandSender` and writes back the `CommandAck`. Covers standalone runs, and also fixes
     `bug-8208a6`.
   - (c) for server-owned runs, a REST route on `roko serve` (for example `POST /api/runs/{run_id}/commands`)
     with the same request/ack body.
   Recommended: (b) first, then (c), with one shared request/ack schema. Session = run ID. Unknown run,
   finished run, or no socket gives non-zero `rejected`/`inject_transport_unavailable`.
3. Executor side: add `ExecutionCommandKind::Inject { kind, payload }` for directive/context (abort maps
   to `Cancel`). The control adapter queues payloads per run (bounded count and size). `GraphTaskDispatcher`
   appends pending directives to the next dispatched task's prompt under a clear "Operator directive"
   heading. The ack is `Accepted` when queued (not "completed").
4. Deduplicate by request/command ID for the run's lifetime, so a retried send is not delivered twice.
5. Tests with a temp workspace and a fake executor (no provider): delivered exactly once, duplicate ID not
   redelivered, unknown/finished/timeout/mismatched ack all non-zero, payload never logged.

## Done when

- With no live run, `roko inject s1 hello` exits non-zero with `inject_transport_unavailable` and
  writes nothing under `.roko/state/`.
- With a live run, `roko inject <run-id> <text>` exits 0 only after the run's `Accepted` ack, and the next
  dispatched task's prompt contains the text once.
- `abort` cancels only the addressed run.
- Verify: `grep -rqw 'fn inject_fails_without_executor_ack' crates/roko-cli/src/ && cargo test -p roko-cli --lib --bin roko inject_fails_without_executor_ack`

## Notes

- The existing verify used `--lib` only, but `cmd_inject` and its tests live in the `roko` bin target, so
  a test added there would be skipped and the command would pass without running it. The suggested
  verify also runs the bin's unit tests.
- Security: the socket must be owner-only, frame size must be bounded before allocation, and payload text
  must not be logged (it may be sensitive). A directive becomes prompt text, so only the local workspace
  owner may send one.
- Step 1 is safe to do now and in parallel with anything outside `commands/util.rs`/`main.rs` tests.
  Steps 2-5 should be coordinated with `bug-8208a6` so the plan control commands and inject share one
  transport.
- 2026-10-01 (wk-childenv): partial on work/gap-1555ac (Plan step 1); cargo verification deferred to the batch
  check. `cmd_inject` (commands/util.rs) fails closed again after validation: exit 1 with
  `inject_transport_unavailable` and the hint (with `--json`, one object with `code`, `message`, `hint`), and it
  writes neither `.roko/state/control.json` nor `inject.json`. The `inject_fail_closed_*` tests in main.rs assert
  that; new test `inject_fails_without_executor_ack`. Surface inventory says Stub; docs/v2 and inject.rs updated.
- The `[[verify]]` covers only the first Done-when bullet, so it passes now: do not close on it. Left: steps 2-5
  (a transport to a live run, shared with bug-8208a6; `ExecutionCommandKind::Inject` and its prompt delivery;
  dedup). The `i` key in the TUI has the same false success: it appends `roko.inject.directive` to
  `.roko/signals.jsonl` and toasts "Injected", but nothing reads that kind.
- 2026-10-02 (wk-childenv): Plan steps 2-5 on work/gap-1555ac; cargo verification deferred to the batch check.
  - Transport decision: (b), a per-run socket. Each `roko plan run` binds `.roko/runtime/inject/<pid>.sock`
    (`inject/transport.rs`, `listen_for_inject`) and writes the token a client must present first to
    `<pid>.token`. Both are `0600`, and the token and frames are the StateHub IPC socket's (frame size bounded
    before allocation). `roko inject` asks each listening run in turn, and the one running the plan the session
    names answers. The session is a running plan's id or its Graph checkpoint run id
    (`GraphTaskDispatcher::inject_target`). No socket, an unknown session, a refusal, a run that has finished
    and a run that does not acknowledge in time all exit non-zero, with `inject_transport_unavailable`,
    `inject_unknown_session` or `inject_rejected`; nothing is written under `.roko/state/`.
  - Executor side: the socket sends `ExecutionCommandKind::Inject { kind, text }` (directive or context;
    `InjectedText`'s `Debug` and the kind's `Display` hide the text) or `Cancel` for an abort, with the
    resolved plan id, on a second command channel of the run. The plan-set driver routes that channel with
    `route_execution_commands`, as it does the TUI's. An inject command for a running plan is queued in
    `OperatorDirectives` (graph_task_dispatch/operator_directives.rs: at most 8 KiB per text and 8 waiting
    per plan) and acked `Accepted`. `plan_dispatch` appends what waits to the next prompt of that plan, once,
    under "## Operator directive" or "## Operator context". An abort cancels that plan only. The control
    adapter (#255) rejects inject commands, since the graph control service has none.
  - Dedup: the command id is the request id. The socket answers a request id it already accepted from its
    remembered answer without sending it again, and the queue ignores a request id it has seen. Both last for
    the run, bounded at 1024 ids.
  - Tests: `a_directive_reaches_the_run_and_gets_its_acknowledgement`, `a_request_sent_again_is_not_delivered_again`,
    `unknown_silent_and_finished_runs_refuse` and `a_client_without_the_token_reaches_nothing` (inject/transport.rs);
    `a_text_waits_for_one_prompt_and_is_queued_once`, `texts_and_queues_are_bounded` and
    `a_queued_directive_reaches_the_next_prompt_once` (operator_directives.rs, the last through a real dispatch
    against a fake provider script that logs its prompt); `inject_commands_queue_for_the_running_plan_and_never_log_their_text`
    (plan_runner.rs, with every log line captured); `inject_succeeds_only_on_the_runs_acknowledgement` and
    the existing `inject_fail_closed_*` / `inject_fails_without_executor_ack` (main_tests.rs, bin).
  - Every Done-when line is now covered. Not done, as follow-ups: (c) a REST route on `roko serve` for
    server-owned runs (`POST /api/runs/{run_id}/commands` with the same request and answer); the TUI's `i` key,
    which still fails closed and could send `ExecutionCommandKind::Inject` on the TUI's own command channel; and a
    shared transport for `roko plan pause/resume/cancel/retry` (bug-8208a6 still writes `control.json`, which
    the driver polls).

## Original notes

[blocked] Blocked on #233, #255, and #325 —

Imported without verification from:
- `tmp/backlog/archive/361-inject-acknowledged-control-transport.md#361 — Deliver `roko inject` Through the Canonical Acknowledged Control Transport`

Warning: every file this item cites is gone (`.roko/daemon.sock`, `.roko/run/roko-<session>.sock`, `tmp/cli-audit/14-status-replay-inject.md`) — likely obsolete or moved.

How to verify: Check: Live receiver observes exact kind/payload/session once.; Success is impossible without a matching executor acknowledgement.; Duplicate request IDs do not duplicate delivery. [evidence: own status: Blocked on #233, #255, and #325]

Verified 2026-09-28: still true. cmd_inject (crates/roko-cli/src/commands/util.rs:1556) writes the payload file (:1597) and a control command into the state dir (:1610), then returns EXIT_SUCCESS (:1630) without waiting for any executor acknowledgement. No DaemonControlRequestV1 or DaemonControlAckV1 type and no request-ID dedup exist anywhere. Subsystem corrected from roko-neuro to roko-cli/inject. The status of blockers #233, #255 and #325 was not checked.

Re-verified 2026-09-29: still open, and worse than described. Nothing reads .roko/state/control.json at HEAD: ControlCommand::poll has no caller and control_command_to_execution is used only in a test. `roko inject` therefore reports success for a command no executor ever sees. The same missing reader affects roko plan pause/resume/cancel/retry, tracked as bug-8208a6. The #233 executor-neutral command channel with acknowledgements (execution_control::ExecutionCommandSender / CommandAck) exists, but only in-process between the TUI and the plan runner. It is the likely transport to reuse, so the DaemonControlAckV1 name in the verify may never appear.
