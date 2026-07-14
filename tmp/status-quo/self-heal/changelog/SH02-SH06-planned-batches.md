# SH02–SH06 planned batch changelog

These five manifests were added on 2026-07-11, but all of their tasks remain
`ready` at audit start. This file records the contracts added to the repository
without representing them as implemented product changes.

## SH02 isolation and recovery — 0 / 6 done

Planned changes:

- enforce effective per-plan concurrency;
- create task-owned worktrees with immutable gate inputs;
- require a durable task commit before success;
- make worktree resume/reacquisition idempotent;
- replace capacity polling with queued wakeups; and
- recover stale locks and attributable dirty worktrees.

Existing precursor: the runner currently creates plan-scoped worktrees and the
working tree contains resume reattachment work. A plan worktree shared by its
tasks is not the task-owned isolation required by SH02-T02.

## SH03 persistence integrity — 0 / 6 done

Planned changes:

- reconcile before writing complete terminal snapshots;
- rotate transition checkpoints;
- make the lifecycle ledger complete and idempotent;
- repair fresh-run seeded task semantics;
- recover atomic-write debris; and
- make StateHub publication ordered, gap-aware, and resynchronizable.

Existing precursor: SH01 writes attempt lifecycle and timeout events, and the
audit includes focused atomic-file recovery review. Those pieces do not by
themselves satisfy snapshot, checkpoint, ledger replay, or StateHub acceptance.

## SH04 runtime telemetry and TUI — 0 / 8 done

Planned changes:

- carry structured run/plan/task/attempt/agent identity;
- preserve typed output channels and severity;
- connect approval mode to structured runner events;
- expose preflight progress and diagnoses;
- show agent liveness and estimated/final token progress;
- enforce route and phase layout invariants;
- refresh Git data asynchronously with bounded caches; and
- write operational event-health logs without prompt/output payloads.

Existing precursor: commit `73d28a644` improved live agent display and state.
That work predates the batch and does not satisfy the complete structured event
pipeline.

## SH05 configuration and dispatch — 0 / 4 done

Planned changes:

- reject ambiguous model configuration before dispatch;
- normalize dispatch lifecycle and bounded transient retry;
- harden process supervision, cancellation, and bounded channels; and
- enforce real per-attempt cost attribution and budgets.

Existing precursor: SH01 substantially improved process-tree cancellation. The
model routing, transient provider retry, and cost-budget contracts remain open.

## SH06 regression harness — 0 / 5 done

Planned changes:

- deterministic crash-chain replay with fake agents and gates;
- interruption/resume tests at agent, gate, commit, snapshot, and merge edges;
- connected TUI buffer and operational-log replay tests;
- subsystem-wide regression and quality gates; and
- a network-free self-host smoke repair plan.

Focused unit and integration tests were added throughout SH01, but the named
SH06 end-to-end fixtures do not yet exist as completed tasks.
