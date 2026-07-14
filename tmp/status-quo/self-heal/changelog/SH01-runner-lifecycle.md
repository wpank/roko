# SH01 runner lifecycle changelog

SH01 is the only self-heal batch with completed implementation tasks in the
audited commit range. At audit start its manifest reports 26 of 28 tasks done.

## 2026-07-11 — lifecycle and terminalization foundation

### Canonical task-attempt state (SH01-T01)

- Added plan/task/attempt identity to runner lifecycle state.
- Added legal attempt transition handling and durable lifecycle projection.
- Separated sibling task state so one concurrent task cannot overwrite another
  task's phase.

Primary implementation commit: `ed5559931`.

### Exactly-once preflight and gate transitions (SH01-T02)

- Bound gate completion to the exact task attempt.
- Removed unconditional phase transitions that could re-enter a task after a
  passing preflight.
- Made transition failures explicit instead of silently continuing.

Primary commits: `e8177218e`, `05986ca31`.

### Idempotent terminalization (SH01-T03)

- Centralized task terminal effects across lifecycle state, DAG state, output
  recording, and the runtime ledger.
- Added idempotent terminal records keyed by plan/task/attempt.
- Reduced duplicate terminal-event paths in the event loop.

Primary commit: `0417afdfd`.

### DAG quiescence and blocked propagation (SH01-T04)

- Added explicit ready, active, blocked, and terminal progress classification.
- Propagated failed prerequisites to downstream blocked tasks.
- Added deterministic handling for cycles, missing dependencies, and a graph
  with no possible future progress.

Primary commit: `8578ee2f3`.

### Unified retry lifecycle (SH01-T05)

- Consolidated failure classification, retry eligibility, delay, exhaustion,
  and next-attempt allocation.
- Prevented permanent failures from entering retry state.
- Ensured exhausted attempts leave no stale active/retrying marker.

Primary commit: `6025aeecf`.

## 2026-07-11 — timeout configuration (SH01-T06A)

- Added distinct optional settings for hard-run, task-attempt, gate-effect,
  agent-silence, and scheduler-no-progress deadlines.
- Preserved omission versus explicit configuration through serde round trips.
- Kept authored task timeout precedence separate from parser-injected defaults.
- Added deterministic legacy plan-timeout migration behavior.

Primary commits: `2ad19e636`, `1a0725bc2`, `cf88a456a`, `86c957589`.

## 2026-07-12 — confirmable process cancellation (SH01-T06B1)

- Hardened process-tree termination so success requires confirmation for the
  root and captured descendants.
- Treated `EPERM` as a live Unix process and only `ESRCH` as absent.
- Preserved already-exited races as confirmed termination when a final wait
  proves absence.
- Added cancellation-safe gate commands that own subprocess groups.
- Required agent reader tasks to be awaited and surfaced reader panics as
  structured failures.

Primary commits: `34845164a`, `971109753`, `88f8be718`, `e8f30e86c`,
`c4d6752ea`, `009737cfd`, `c7ca176a9`.

## 2026-07-12 — exact attempt ownership (SH01-T06B2A–B2B2)

- Introduced a persistent ownership registry keyed by `TaskAttemptRef`.
- Coupled logical ownership with the concrete runtime resource and protected
  mutations with linear, nonce-backed claims.
- Added exact phase/effect validation, cancellation state, recoverable claim
  restoration, and aggregate metadata derived from surviving owners.
- Reworked natural agent wait so unconfirmed waits preserve the child, reader
  tasks, and PID registration.
- Migrated CLI and bridge agent producers, forwarders, readers, and concurrency
  permits into exact attempt ownership before dispatch.
- Classified only provider success or exit code zero as successful agent work;
  buffered stale events lose eligibility after ownership changes.

Primary commits: `37147f33d`, `fa0f485d8`, `45864c37e`, `1dbd05baa`,
`8f94dcf8f`, `501b1e2d7`, `ee25fc1f8`.

## 2026-07-12 — gate and merge effect ownership (SH01-T06B2C1–C3)

- Added gate effect identity containing attempt, completion kind, rung, and
  dispatch generation.
- Put post-agent gates and preflight gates behind start barriers so ownership is
  installed before work begins.
- Claimed gate outcomes before terminal lifecycle side effects.
- Migrated plan verification into the same exact-effect ownership model.
- Separated merge reservation, dormant setup, process launch, completion, and
  exceptional cleanup into recoverable ownership transitions.
- Added exact merge reservation tokens and ensured shutdown releases surviving
  reservations.

Primary commits: `a7965b2db`, `a8c85c4fa`, `1254e337c`, `043afad3e`,
`a06077f1f`, `4763c0114`, `1b6a4153b`, `376bf62a2`, `e36f06bf7`, and their
associated completion/test commits.

## 2026-07-12 — durable cancellation lifecycle (SH01-T06B2D1–D4)

- Persisted cancellation-requested and cancellation-failed lifecycle events.
- Preserved ownership when process termination could not be confirmed.
- Added cleanup for every resource variant, including gates, merges, permits,
  readers, and forwarders.
- Aggregated cancel-all outcomes and reconstructed runtime activity only from
  resources that actually survived cleanup.
- Retained failed or corrupted cancellation claims for later recovery instead
  of detaching them.

Primary commits: `50f2bee46`, `13b8c0fdb`, `ecba41fee`, `12466d632`,
`3e33082cb` and associated completion commits.

## 2026-07-12 — monotonic deadlines (SH01-T06C1–C3)

- Defined monotonic attempt, phase, and last-agent-activity clocks.
- Enforced task-attempt, gate-effect, and agent-silence deadlines against exact
  owners.
- Added independent hard-run and scheduler-no-progress deadlines.
- Restricted scheduler progress resets to durable lifecycle or DAG milestones;
  ordinary agent chatter only refreshes agent-silence state.
- Persisted typed global timeout state before cleanup and terminal snapshots.

Primary commits: `a04556835`, `d2b089f33`, `044e6fa47` and their completion
commits.

## Cross-cutting precursor changes

Commit `73d28a644` added plan-scoped worktree routing and initial live telemetry
stabilization before SH01 execution. These changes are useful foundations but do
not complete SH02 task-owned worktrees or the SH04 structured telemetry contract.

Additional fixes during the range included task-only verification support,
suppression of false spawn telemetry, exact gate completion binding, and focused
regression coverage around cancellation, gate claims, merge setup, and merge
settlement ordering.

## Pending SH01 work

### SH01-T06C4 — lost effects and timeout races

The manifest still reports this task as ready. The working tree contains an
in-progress implementation under audit, but completion requires deterministic
evidence for linear completion-versus-expiry, duplicate/stale events, missing
producers, ledger/projection equivalence, and resume semantics.

### SH01-T07 — truthful summaries

The manifest still reports this task as ready. Global and per-plan task totals
must be derived from one reconciled terminal classification, including blocked,
skipped, cancelled, and orphaned tasks.
