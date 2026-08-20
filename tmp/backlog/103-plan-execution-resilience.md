# 103 — Plan Execution Unhandled Failure Modes

**Priority**: P2 — reliability; four gaps cause silent data loss or missed recovery opportunities during long-running plan execution
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/` (`roko-cli`), `crates/roko-agent/` (`roko-agent`)
**Depends on**: None

---

## Background

The plan execution loop in roko (`roko plan run`) handles many failure modes well: it checks
disk space before starting, prevents concurrent runs via file locking, classifies agent
crashes, and supports snapshot-based resume. However, four specific gaps exist where a
failure either causes silent data loss or produces a worse outcome than necessary.

**Gap 1 — Disk exhaustion during a run.** Disk space is checked once at startup. A plan
that generates many worktrees, large agent outputs, or episodic artifacts can fill the disk
mid-run. The next write (snapshot, gate artifact, worktree checkout) fails silently or with
an opaque I/O error rather than a clear "disk full, run halted" message.

**Gap 2 — `Retry-After` headers ignored.** When an LLM provider responds with HTTP 429 (rate
limit) or 529 (overloaded), it includes a `Retry-After` header in seconds. The agent HTTP
layer parses this into `HttpPostError::retry_after_secs` and providers propagate it as
`ProviderError::RateLimit { retry_after_ms }`. However the runner's retry scheduler uses a
fixed exponential backoff that ignores the provider's hint. This means roko may retry before
the provider is ready (causing more 429s) or may wait much longer than necessary.

**Gap 3 — Worktree creation failures have no retry.** Git worktree creation can fail
transiently due to filesystem races, stale lock files from a crashed previous run, or
temporary branch conflicts. Currently the first failure causes an immediate permanent task
failure with a generic "worktree unavailable" error. Adding up to 3 retries with exponential
backoff and an actionable error message ("try: roko doctor disk") resolves most transient
cases.

**Gap 4 — Snapshot write failures do not halt the run.** The snapshot writer thread
increments `fail_streak` and logs at `ERROR` when writes fail, but the plan continues
executing tasks. After `DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT` (value: 3) consecutive
failures the log level escalates to "snapshot persistence degraded" — but the run never
stops. A plan can complete dozens of tasks with no durable record, meaning a crash loses
all of that work.

## Current State

### Gap 1: Disk check only runs at startup

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`, line 1879

```rust
disk_pre_check(&config.workdir, resources_cfg, config.force_disk_check)?;
```

This is the only call to `disk_pre_check`. The function is defined at line 22231:

```rust
fn disk_pre_check(workdir: &Path, resources: Option<&ResourcesConfig>, force: bool) -> Result<()> {
    let monitor = roko_fs::DiskMonitor::new(min_free_mb, warn_mb);
    match monitor.check_pre_run(workdir) { /* ... */ }
}
```

`check_pre_run` checks whether free space is below the configured minimum. No equivalent
check is called between tasks, before gate evaluation, or before snapshot writes.

### Gap 2: `Retry-After` parsed but not consumed by the runner

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/http.rs`, line 34

```rust
pub struct HttpPostError {
    pub retry_after_secs: Option<u64>,  // populated from Retry-After header
}
```

Providers (e.g., `crates/roko-agent/src/provider/anthropic_api.rs:122`,
`crates/roko-agent/src/openai_compat_backend.rs:52`) convert `retry_after_secs` to
`ProviderError::RateLimit { retry_after_ms: Option<u64> }`.

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs`, line 730

```rust
pub fn retry_delay(failure_kind: RunnerFailureKind, attempt: u32) -> Duration {
    let base = failure_kind.retry_cooldown_secs();
    // Exponential backoff only — no provider hint considered.
}
```

The `RetryDecision` struct at line 662 has a `cooldown_ms` field computed purely from
`retry_delay()`. The `retry_after_ms` from `ProviderError` never reaches this calculation.
The runner builds `RetryDecision::for_failure(failure_kind, attempt, budget, reason)` at
event_loop.rs line 5787 without consulting the agent's output for a Retry-After hint.

### Gap 3: Worktree creation has no retry

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`, lines 1585-1598

```rust
async fn ensure_attempt_workdir(
    worktrees: &WorktreeManager,
    attempt: &TaskAttemptRef,
) -> std::result::Result<PathBuf, String> {
    let handle = match worktrees.get_attempt(&attempt.plan_id, &attempt.task_id, attempt.attempt) {
        Some(handle) => handle,
        None => worktrees
            .create_for_attempt(&attempt.plan_id, &attempt.task_id, attempt.attempt)
            .await
            .map_err(|err| format!("worktree unavailable for attempt {}: {err}", attempt.key()))?,
    };
    worktrees.touch(&handle.id);
    Ok(handle.path)
}
```

A single failure from `create_for_attempt` immediately propagates as a task failure. There
is no retry logic, no exponential backoff, and no guidance in the error message.

### Gap 4: Snapshot write failures do not halt the run

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/snapshot_writer.rs`, lines 239-253

```rust
fn write_payload(payload: &SnapshotPayload, fail_streak: &mut u32) {
    if let Err(e) = write_all_files(payload) {
        *fail_streak += 1;
        if *fail_streak >= DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT {
            error!(streak = *fail_streak, "snapshot persistence degraded");
        } else {
            error!(error = %e, "failed to write snapshot");
        }
    } else {
        *fail_streak = 0;
    }
}
```

`DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT` is 3 (defined in
`crates/roko-core/src/defaults.rs:325`). After 3 consecutive failures the log message
changes but no action is taken — the writer loop continues accepting and discarding payloads.

The `SnapshotWriter` communicates with the event loop via a `SyncSender<WriterMsg>`. The
writer thread is entirely independent; there is no back-channel to halt the event loop.

## Implementation Plan

### Step 1: Add a mid-run disk check before each gate evaluation

The cleanest injection point is in the gate dispatch function. Each gate run is bounded and
occurs between task execution and snapshot writing.

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/gate_dispatch.rs`

At the beginning of `run_gate_once` (line 460), before the gate signal is built, add:

```rust
// Mid-plan disk guard: fail fast if disk is critically low.
{
    use roko_fs::DiskMonitor;
    let monitor = DiskMonitor::new(
        gates_config.min_free_disk_mb.unwrap_or(0),
        0,  // warn threshold: not used in mid-plan check
    );
    if let Err(e) = monitor.check_pre_run(&workdir) {
        return GateCompletion {
            plan_id,
            task_id,
            rung,
            outcome: GateOutcome::Failed,
            summary: format!("disk space insufficient before gate evaluation: {e}"),
            // ... fill remaining fields with zero/default values
        };
    }
}
```

Consult the `GatesConfig` struct (found in `crates/roko-cli/src/runner/types.rs` or
`crates/roko-gate/src/`) to determine where `min_free_disk_mb` lives. If `GatesConfig`
does not have this field, thread the `workdir` and resources config into the gate dispatch
call or use the `RunConfig` that is already accessible in the event loop where
`run_gate_once` is called.

### Step 2: Propagate Retry-After into the runner retry scheduler

The `Retry-After` hint from providers must flow from the agent dispatch result to the
`RetryDecision` calculation. Here is the chain to follow:

1. In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/mod.rs`, the
   `ProviderError::RateLimit { retry_after_ms }` variant already carries the hint (line 981).

2. The agent dispatch result flows into the runner through `AgentDispatchOutcome` in
   `crates/roko-cli/src/runner/types.rs`. Check whether `AgentDispatchOutcome` has a field
   for `retry_after_ms`. If not, add one:
   ```rust
   pub struct AgentDispatchOutcome {
       // ... existing fields ...
       /// Provider-requested retry delay in milliseconds, if any.
       /// Populated from `Retry-After` headers on 429/529 responses.
       pub retry_after_ms: Option<u64>,
   }
   ```

3. In the event loop at line 5787 where `RetryDecision::for_failure` is called, check for
   the provider hint and override the calculated cooldown:
   ```rust
   let decision = RetryDecision::for_failure(failure_kind, attempt.attempt, decision_budget, "");
   // If the provider specified a Retry-After, override the backoff.
   let decision = if let Some(hint_ms) = outcome.retry_after_ms {
       let capped_ms = hint_ms.min(3_600_000); // cap at 1 hour
       tracing::info!(
           hint_ms,
           "respecting provider Retry-After hint"
       );
       RetryDecision { cooldown_ms: capped_ms, ..decision }
   } else {
       decision
   };
   ```

4. Find where `AgentDispatchOutcome` is constructed (search for
   `AgentDispatchOutcome {` in `crates/roko-cli/src/`) and populate `retry_after_ms`
   from the underlying provider error when available.

### Step 3: Add retry with backoff to worktree creation

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

Replace the `ensure_attempt_workdir` function (lines 1585-1598) with:

```rust
async fn ensure_attempt_workdir(
    worktrees: &WorktreeManager,
    attempt: &TaskAttemptRef,
) -> std::result::Result<PathBuf, String> {
    // Fast path: reuse an existing worktree for this attempt.
    if let Some(handle) = worktrees.get_attempt(&attempt.plan_id, &attempt.task_id, attempt.attempt) {
        worktrees.touch(&handle.id);
        return Ok(handle.path);
    }

    // Slow path: create a new worktree with up to 3 attempts.
    let mut last_err = String::new();
    for retry in 0..3_u32 {
        if retry > 0 {
            let delay = Duration::from_secs(1u64 << (retry - 1)); // 1s, 2s
            tracing::warn!(
                attempt_key = %attempt.key(),
                retry,
                delay_secs = delay.as_secs(),
                last_error = %last_err,
                "worktree creation failed, retrying"
            );
            tokio::time::sleep(delay).await;
        }
        match worktrees
            .create_for_attempt(&attempt.plan_id, &attempt.task_id, attempt.attempt)
            .await
        {
            Ok(handle) => {
                worktrees.touch(&handle.id);
                return Ok(handle.path);
            }
            Err(e) => {
                last_err = e.to_string();
            }
        }
    }
    Err(format!(
        "worktree unavailable for attempt {} after 3 attempts: {last_err}. \
        Check disk space with: roko doctor disk",
        attempt.key()
    ))
}
```

Add `use std::time::Duration;` at the top of the file if not already imported.

### Step 4: Halt the run on persistent snapshot failures

The snapshot writer runs on a dedicated OS thread and communicates with the event loop via
a `SyncSender<WriterMsg>`. To signal a fatal condition back to the event loop, the writer
thread needs a back-channel.

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/snapshot_writer.rs`

Add a halt sender to `SnapshotWriter`:

```rust
pub struct SnapshotWriter {
    tx: Option<SyncSender<WriterMsg>>,
    handle: Option<JoinHandle<()>>,
    flush_rx: std::sync::mpsc::Receiver<()>,
    /// Receives a `true` signal when the writer thread detects a fatal
    /// failure streak and the event loop should halt.
    pub halt_rx: std::sync::mpsc::Receiver<bool>,
}
```

In `SnapshotWriter::new`, create the halt channel:
```rust
let (halt_tx, halt_rx) = std::sync::mpsc::channel::<bool>();
```
Pass `halt_tx` to the writer loop, and add to `write_payload`:

```rust
const MAX_SNAPSHOT_FAIL_STREAK: u32 = 5;  // configurable constant

fn write_payload(
    payload: &SnapshotPayload,
    fail_streak: &mut u32,
    halt_tx: &std::sync::mpsc::SyncSender<bool>,
) {
    if let Err(e) = write_all_files(payload) {
        *fail_streak += 1;
        if *fail_streak >= MAX_SNAPSHOT_FAIL_STREAK {
            error!(
                error = %e,
                streak = *fail_streak,
                "snapshot persistence broken — signaling halt to prevent data loss"
            );
            let _ = halt_tx.try_send(true);
        } else {
            error!(error = %e, streak = *fail_streak, "failed to write snapshot");
        }
    } else {
        *fail_streak = 0;
    }
}
```

In the event loop, poll `snapshot_writer.halt_rx` after each snapshot enqueue (or in the
main select loop) and abort with a clear error if `true` is received:

```rust
if snapshot_writer.halt_rx.try_recv().is_ok() {
    return Err(anyhow::anyhow!(
        "halting plan run: snapshot writer has failed {} consecutive times. \
        Check disk space and permissions on .roko/state/",
        MAX_SNAPSHOT_FAIL_STREAK
    ));
}
```

## Acceptance Criteria

1. Disk space is checked before each gate rung evaluation, not only at startup. A critically
   low disk causes the gate to return `Failed` with a clear message and the run to halt.
2. When an agent response includes a `Retry-After` header (429/529), the runner waits at
   least that many milliseconds before retrying (capped at 1 hour).
3. Worktree creation is retried up to 3 times with exponential backoff (1s, 2s) before
   the task fails permanently. The final error message includes "roko doctor disk".
4. After `MAX_SNAPSHOT_FAIL_STREAK` (5) consecutive snapshot write failures, the event loop
   halts with a clear error message referencing `.roko/state/`.
5. `cargo test -p roko-cli` passes with no new failures.
6. New test: create a mock dispatcher that returns a `ProviderError::RateLimit { retry_after_ms: Some(30_000) }` and verify the runner's `RetryDecision.cooldown_ms` equals 30,000.

## Verification Checklist

- [ ] Run a plan against a filesystem approaching its limit and confirm the mid-plan disk check produces a clear error before an opaque I/O failure
- [ ] Inspect `crates/roko-agent/src/provider/mod.rs` around line 981-1030 to confirm `retry_after_ms` flows through `ProviderError::RateLimit`
- [ ] Confirm `AgentDispatchOutcome` carries `retry_after_ms` and the event loop at line 5787 reads it
- [ ] Run `cargo test -p roko-cli -- ensure_attempt_workdir` to verify the retry test passes
- [ ] Verify `snapshot_writer.halt_rx` is polled in the event loop main select; a writer failure after 5 attempts halts the run
- [ ] Run `cargo clippy -p roko-cli --no-deps -- -D warnings` and confirm clean

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/gate_dispatch.rs` | Add mid-plan disk check at start of `run_gate_once` (line 460) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs` | Add `retry_after_ms: Option<u64>` field to `AgentDispatchOutcome` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | Replace `ensure_attempt_workdir` (lines 1585-1598) with retry version; use `retry_after_ms` in `RetryDecision` construction (around line 5787); poll `halt_rx` after snapshot enqueue |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/snapshot_writer.rs` | Add `halt_rx` field to `SnapshotWriter`; signal halt after `MAX_SNAPSHOT_FAIL_STREAK` consecutive failures |
