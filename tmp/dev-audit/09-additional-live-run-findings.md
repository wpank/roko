# Additional live-run findings

This appendix records findings from the same 2026-08-31 `doctor-network-v2` run that were not
already covered in the main audit. It does not replace the scheduler, Cargo, gate, task-shaping,
or timeout findings in [01-baseline-evidence.md](01-baseline-evidence.md) and
[04-roko-self-hosting.md](04-roko-self-hosting.md).

Evidence below is tied to run `run-1788163153495` unless stated otherwise.

## Provider identity was correct in the final run

The earlier unforced run dispatched Gemma, which explains the first screenshot. The final forced
run did not use Gemma:

- `agent.dispatch.started` requested the `codex` alias for both T1 and T2.
- `agent.dispatch.completed` resolved both launches to `gpt-5.6-sol`.
- `agent.started` identified the provider as `codex-cli`.

Provider selection was therefore not the cause of the final run's slowness. The evidence bundle
should always persist requested alias, resolved provider, resolved model, and fallback reason on
the task attempt. The current snapshot loses some of this proof: T1's task usage has an empty
model, and T2 has empty model and provider fields.

## Timeout loses provider usage and cost

> **Integration status:** source-implemented in the hard-deadline batch. Provider identity and the
> latest monotonic cumulative usage snapshot now survive timeout/cancellation settlement. Final
> timeout-fixture evidence on the rebased tree remains pending.

T2's raw Codex session emitted a final cumulative usage snapshot before the runner killed it:

| Field | Raw Codex value | Roko final task usage |
|---|---:|---:|
| Input tokens | 3,980,185 | 0 |
| Cached input tokens | 3,756,288 | not represented |
| Output tokens | 13,832 | 0 |
| Model | `gpt-5.6-sol` | empty |
| Provider | Codex CLI | empty |

The plan report consequently counted only T1: 408,156 input tokens, 2,270 output tokens, and
$0.296488. This materially understates resource use and makes timeout-heavy experiments look
cheaper than successful runs.

Required design:

- Treat provider token updates as monotonic cumulative snapshots keyed by run/task/attempt.
- Persist the latest snapshot during the turn, not only after a successful final result.
- On timeout, cancellation, crash, or lost process, finalize usage from the latest observed value.
- Keep current context occupancy separate from cumulative input and cached input. A multi-million
  cumulative total does not mean one prompt exceeded the model context window.
- Include partial/estimated cost with an explicit completeness flag.

Add a timeout fixture whose provider emits usage and never emits a final answer. The run must still
retain model, provider, input, cached input, output, and cost evidence.

## Codex tool policy was advisory, not binding

> **Integration status:** partially closed. FAST plan/prompt scope and budgets are binding in Roko,
> native provider limits are used where supported, and strict unsupported Codex restriction
> requests fail closed instead of claiming enforcement. Codex still has no native binding
> operation allowlist for all built-ins; a Roko-owned operation-level broker remains open.

The task contract allowed only `grep`, `read_file`, and `write_file`, and Roko disallowed
`web_fetch` and `web_search`. At every final-run Codex launch, `dispatch_v2` logged:

    codex CLI cannot enforce tool policy; proceeding without enforcement

T2 then made 44 Codex custom-tool calls, expanded into 70 Roko `agent.tool_call` events. It used
the general execution surface, searched the repository's event history and other worktrees,
searched Codex session history under the user's home directory, ran `git fsck`, and issued a web
search. Some of that exploration was a rational response to the missing PRD, but it violated the
declared capability boundary and consumed most of the turn's tokens.

There is also a code-contract contradiction: the `CliDispatchRequest` documentation says Codex
rejects unsupported restrictions fail-closed, while `build_codex_invocation` warns and proceeds.

Required design:

- A restrictive safety contract must never silently degrade to prompt-only guidance.
- Either reject the dispatch as unsupported or route all Codex operations through a Roko-owned
  broker that enforces tool name, read roots, write roots, network policy, command policy, and
  per-tool budgets.
- Do not tell the agent that named MCP-style tools are available when the actual surface is a
  general execution wrapper. Surface discovery itself cost calls and prompt tokens here.
- Bound search scope to the attempt worktree plus explicit read-only artifacts. Searching other
  worktrees, all session history, unreachable Git objects, or the web should require an explicit
  task capability.
- Record requested policy, effective policy, degradation, and every denied call in the run bundle.

This is both a performance issue and a safety/correctness issue. Robust enforcement is the largest
implementation uncertainty in this audit because the Codex CLI has no native binding allowlist for
its built-in tools.

## The connected TUI does unnecessary idle work

> **Integration status:** source-implemented in `88c724744`. Connected sessions honor the
> configured cadence, redraw on visible changes/animation needs, omit the broad watcher when
> StateHub is authoritative, and exclude worktrees/targets/caches/archives from fallback recursion.
> Final idle/active CPU and interaction measurements remain pending.

The operator-owned TUI should remain open until the operator exits it. Keeping it open does not
require continuous full redraws or stale running state.

Current behavior in `crates/roko-cli/src/tui/app.rs`:

- Starts with a 16ms event interval, approximately 60 FPS.
- Drops only to 50ms after five seconds without input, still 20 FPS.
- Calls a full terminal draw on every loop iteration.
- Does not use `tui.refresh_rate_ms` for this loop, even though the setting is exposed in config.
- Starts a recursive `.roko` filesystem watcher even when connected to the in-process StateHub.

`crates/roko-cli/src/tui/fs_watch.rs` recursively watches all of `.roko`. At inspection time that
tree contained about 10,132 files, including about 9,697 files below `.roko/worktrees`. In the
connected path, StateHub already carries live state and most filesystem notifications are merely
coalesced and drained unless disk replay is enabled.

Required design:

- Redraw on input, resize, a StateHub revision, animation deadline, notification expiry, or a
  configured low-frequency metrics tick.
- Track a dirty flag and skip terminal draws when no visible state changed.
- Honor `tui.refresh_rate_ms`, with a conservative idle cap rather than hardcoded 20 FPS.
- In connected mode, omit the broad filesystem watcher or watch only artifacts not delivered by
  StateHub. Exclude worktrees, targets, large logs, and archives.
- Keep the terminal screen available after completion, but freeze elapsed/ETA, clear active-agent
  presentation, and display the terminal outcome and duration.

Measure idle and active TUI CPU, draw count, watcher event count, and input latency. A terminal run
left open for postmortem navigation should settle close to zero redraw CPU.

## Final persistence contradicts the terminal event

> **Integration status:** source-implemented in `52d5f4df4` plus the bounded conductor-settlement
> handoff. Terminal report/event/snapshot/status/ledger/task/agent/PID projections are idempotent,
> elapsed time freezes, and unconfirmed process ownership remains explicitly degraded rather than
> being cleared. The final success/failure/timeout/cancellation/restart matrix is still verification
> work.

The event stream correctly ended with `run.completed: failed` at 08:17:23. The other durable
projections did not converge on that truth:

- `state-snapshot.json` was timestamped immediately before `run.completed`; its lifecycle still
  says `running` and its plan lifecycle says `started`, although the executor phase says `failed`.
- `status.json` says phase `dispatch`, `active_agents: 1`, and last event `task:T2`.
- `agent-pids.json` retained five dead PIDs.
- `run-ledger.jsonl` wrote two identical final `run_summary` entries.
- Both summaries report `agent_outcomes: 0`, despite two real Codex launches and one completed turn.
- T1's task-usage model is empty; T2's entire usage/provider/model record is zero-filled.

In the normal all-plans-terminal path, the event loop saves the snapshot before it emits
`run.completed`, then breaks without a terminal snapshot save. The general exit path also persists
the run ledger again after the completion branch already wrote it.

Required design:

1. Compute one immutable terminal report.
2. Transition lifecycle, tasks, attempts, active agents, timers, and process ownership to terminal.
3. Emit one terminal event using that report.
4. Atomically persist all terminal projections from the same report/version.
5. Clear a PID only after exit is confirmed; retain unconfirmed survivors with an explicit status.
6. Make repeated finalization idempotent by run ID and terminal sequence number.

Acceptance must compare event, snapshot, status, PID registry, ledger, CLI exit, and TUI. All must
agree after success, failure, timeout, cancellation, provider crash, and resume.

## Global event storage is already degrading

> **Integration status:** closed for new records in `5f689d66e`. New runner/canonical records have
> bounded hashed per-run indexes, cursor queries, lifecycle-boundary flushes, and run-filtered SSE.
> Historical global segments are intentionally not scanned by requests or startup; an explicit
> bounded offline repair command remains open.

The active event log was 89MB and 282,580 lines; the archive was another 100MB. Prompt-experiment
reconciliation warned four times on startup that it could not parse archive line 1. The connected
TUI also watched this log tree while the scheduler produced thousands of redundant events.

This strengthens the run-scoped storage recommendation in
[05-feedback-harness.md](05-feedback-harness.md):

- Validate a segment before promoting it to an archive.
- Quarantine malformed segments without rescanning them on every run.
- Store a run/sequence index and query by cursor instead of scanning global JSONL.
- Rotate and compact off the plan critical path.
- Bound tool output and keep large raw provider streams as referenced artifacts, not duplicated
  inside the central event log.

## Worktree safety constraint

None of the performance fixes above requires pruning unrelated worktrees. Cleanup must distinguish
Roko-owned attempt worktrees from operator-created or externally owned worktrees and must protect
any worktree with a live owner/session. Do not use blanket `git worktree prune`, broad
`remove_all`, or age alone as an ownership signal while another Codex or human session may be
working there.

Per-run worktree records should include owner, run ID, attempt ID, creation time, last heartbeat,
base commit, dirty state, and cleanup eligibility. A useful dirty or timed-out patch is an artifact
to preserve, not garbage.

## Implementation sizing

Status in the expanded integration:

- [x] Move scheduler admission ahead of preparation and fix actual-launch accounting.
- [x] Make the FAST agent patch-only and give gates one conservative target-aware Cargo path.
- [x] Preserve provider identity/usage on timeout and split provider ownership from salvaged gate
  ownership through durable typed transitions.
- [x] Persist one idempotent terminal truth across event/dashboard/status/ledger/PID/TUI projections
  while allowing postmortem navigation.
- [x] Bound conductor Restart/Fail pre-cancel settlement and fail closed with degraded ownership
  evidence when process absence cannot be proved.

Representative runtime fixtures and the coordinator's final compile/test/clippy/smoke batch remain
unchecked; source implementation is not a claim that those commands passed.

The original high-impact-subset estimate was approximately 4–6 working days. The remaining
estimates below describe the broader audit and are planning estimates, not completed work:

A robust implementation of the full audit is approximately 12–18 working days, with 3–4 calendar
weeks allowing integration and soak testing. A rough engineering split is:

| Area | Estimate |
|---|---:|
| Cargo sandbox/cache and verification ownership | 2–3 days |
| Scheduler admission/idempotency | 1–2 days |
| Prompt/context packaging | 1–2 days |
| Binding Codex tool enforcement | 2–4 days |
| Timeout and incremental usage accounting | 1–2 days |
| TUI redraw/watch behavior | 1–2 days |
| Terminal persistence, PID cleanup, and ledger idempotency | 1–2 days |
| Event storage, frontend/build graph, and disk/cache hardening | 1–2 days |
| Integration fixtures and soak runs | 2–3 days |

The estimates overlap; they should not be summed mechanically. Tool enforcement and terminal
cross-projection consistency carry the most design risk.
