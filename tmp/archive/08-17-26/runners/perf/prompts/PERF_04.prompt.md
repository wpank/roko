# PERF_04: Buffered JSONL event logger (B11+B13)

## Task

`JsonlLogger::write_event` calls `flush()` after **every** event. A
typical run emits 20-40 events → 60-150 ms of synchronous disk IO.
Buffer the writer (8 KiB), flush only at run completion / Drop / periodic
tick. Also: reuse a thread-local serialization buffer to avoid per-event
allocation (B13).

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_04](../ISSUE-TRACKER.md#perf_04)
- Plan: `tmp/solutions/perf/implementation/04-buffered-jsonl-logger.md`
- Bottlenecks: B11 + B13 (BOTTLENECK-ANALYSIS.md)
- Performance contract: **C-4** (≤2 disk syncs per typical run)
- Priority: P1
- Effort: ≈2 h
- Depends on: none
- Wave: 1

## Problem

`crates/roko-runtime/src/jsonl_logger.rs` (the entire file is 127 LOC):

```rust
fn write_event(&self, event: &RuntimeEvent) -> std::io::Result<()> {
    self.ensure_writer()?;

    let envelope = RuntimeEventEnvelope::new(
        event.run_id(),
        self.seq.fetch_add(1, Ordering::Relaxed),
        "jsonl_logger",
        event.clone(),
    );

    let json = serde_json::to_string(&envelope)?;     // allocates a String

    let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ref mut w) = *writer {
        writeln!(w, "{json}")?;
        w.flush()?;                                    // ← FLUSH PER EVENT
    }
    Ok(())
}
```

Two problems:

1. **B11.** `flush()` per event = 1 fsync per event. 30 events × ~3 ms =
   ~90 ms.
2. **B13.** `serde_json::to_string` allocates a fresh `String` per
   event. ~30 allocations on the hot path.

The reader path (`runtime-events.jsonl` consumers in `roko-cli` and
`roko-serve`) tolerates partial last lines because every reader uses the
JSONL "ignore non-parsing lines" pattern (mirrors
`crates/roko-fs/src/file_substrate.rs::replay_log`).

## Exact Changes

### Step 1 — Rewrite `jsonl_logger.rs`

Replace the body of `crates/roko-runtime/src/jsonl_logger.rs` (preserve
the `consume` impl + the existing test):

```rust
//! JsonlLogger -- persists RuntimeEvents to a JSONL file.
//!
//! Each event is serialized as a single JSON line with a timestamp,
//! enabling replay and state reconstruction.
//!
//! # Buffering & flush semantics (perf contract C-4)
//!
//! The internal `BufWriter` has an 8 KiB buffer (≈30-50 envelopes).
//! `write_event` does NOT flush after each event; flushing happens via:
//!
//! - explicit [`JsonlLogger::flush`] call at run completion (the
//!   workflow engine calls this on every `WorkflowEngine::run` exit),
//! - periodic background tick (set up by `roko serve` for long-running
//!   processes),
//! - `Drop` impl (best-effort durability on process shutdown).
//!
//! Crash safety: if the process panics before the explicit flush, the
//! `Drop` impl flushes; if the OS kills the process before `Drop` runs,
//! the JSONL log may have a partial last line. Readers tolerate this
//! (see `crates/roko-fs/src/file_substrate.rs::replay_log` for the
//! reference pattern).

use roko_core::RuntimeEvent;
pub use roko_core::foundation::EventConsumer;
use roko_core::runtime_event::RuntimeEventEnvelope;
use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

const BUFFER_CAPACITY_BYTES: usize = 8 * 1024;
const SCRATCH_CAPACITY_BYTES: usize = 512;

thread_local! {
    /// Per-thread reusable buffer for serializing one envelope at a
    /// time. Avoids the per-event String allocation that the previous
    /// implementation paid (B13).
    static SCRATCH: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(SCRATCH_CAPACITY_BYTES));
}

/// Logger that writes RuntimeEvents as JSONL (one JSON object per line).
pub struct JsonlLogger {
    path: PathBuf,
    seq: AtomicU64,
    writer: Mutex<Option<std::io::BufWriter<std::fs::File>>>,
}

impl JsonlLogger {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            seq: AtomicU64::new(0),
            writer: Mutex::new(None),
        }
    }

    pub fn from_roko_dir(roko_dir: &Path) -> Self {
        Self::new(roko_dir.join("runtime-events.jsonl"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn ensure_writer(&self) -> std::io::Result<()> {
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if writer.is_none() {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            *writer = Some(std::io::BufWriter::with_capacity(BUFFER_CAPACITY_BYTES, file));
        }
        Ok(())
    }

    fn write_event(&self, event: &RuntimeEvent) -> std::io::Result<()> {
        self.ensure_writer()?;

        let envelope = RuntimeEventEnvelope::new(
            event.run_id(),
            self.seq.fetch_add(1, Ordering::Relaxed),
            "jsonl_logger",
            event.clone(),
        );

        SCRATCH.with(|cell| -> std::io::Result<()> {
            let mut scratch = cell.borrow_mut();
            scratch.clear();
            // Serialize directly into the reused buffer (no allocation
            // for the envelope JSON). serde_json::to_writer writes
            // valid UTF-8 by construction.
            serde_json::to_writer(&mut *scratch, &envelope)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
            scratch.push(b'\n');

            let mut writer = self
                .writer
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(ref mut w) = *writer {
                w.write_all(&scratch)?;
                // NOTE: NO flush() here. See module-level docs for the
                // flush story (explicit flush, periodic tick, or Drop).
            }
            Ok(())
        })
    }

    /// Force any buffered events to disk. Call at run completion.
    pub fn flush(&self) -> std::io::Result<()> {
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(ref mut w) = *writer {
            w.flush()?;
        }
        Ok(())
    }
}

impl Drop for JsonlLogger {
    fn drop(&mut self) {
        // Best-effort durability on shutdown. Errors are intentionally
        // swallowed: there's no useful recovery in Drop.
        let _ = self.flush();
    }
}

impl EventConsumer for JsonlLogger {
    fn consume(&self, event: &RuntimeEvent) {
        let _ = self.write_event(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_events_to_file() {
        // Existing test — keep verbatim.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let logger = JsonlLogger::new(path.clone());
        logger.consume(&RuntimeEvent::AgentSpawned {
            run_id: "r1".into(),
            agent_id: "a1".into(),
            role: "implementer".into(),
            model: "model".into(),
        });
        logger.consume(&RuntimeEvent::GatePassed {
            run_id: "r1".into(),
            gate_name: "compile".into(),
            duration_ms: 100,
        });
        logger.flush().unwrap();   // explicit flush in test
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        let first: RuntimeEventEnvelope = serde_json::from_str(lines[0]).unwrap();
        let second: RuntimeEventEnvelope = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(first.payload.kind(), "agent_spawned");
        assert_eq!(second.payload.kind(), "gate_passed");
    }

    #[test]
    fn buffered_writes_persist_after_explicit_flush() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let logger = JsonlLogger::new(path.clone());
        for i in 0..1000 {
            logger.consume(&RuntimeEvent::AgentSpawned {
                run_id: format!("r{i}"),
                agent_id: format!("a{i}"),
                role: "implementer".into(),
                model: "m".into(),
            });
        }
        logger.flush().unwrap();
        let lines = std::fs::read_to_string(&path).unwrap();
        assert_eq!(lines.lines().count(), 1000);
    }

    #[test]
    fn dropped_logger_persists_buffered_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        {
            let logger = JsonlLogger::new(path.clone());
            for i in 0..50 {
                logger.consume(&RuntimeEvent::AgentSpawned {
                    run_id: format!("r{i}"),
                    agent_id: format!("a{i}"),
                    role: "implementer".into(),
                    model: "m".into(),
                });
            }
            // Intentionally no explicit flush: relies on Drop.
        }
        let count = std::fs::read_to_string(&path).unwrap().lines().count();
        assert_eq!(count, 50);
    }

    #[test]
    fn reader_tolerates_partial_last_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        {
            use std::io::Write as _;
            let mut f = std::fs::File::create(&path).unwrap();
            // One valid envelope:
            let env = RuntimeEventEnvelope::new(
                "r0".into(), 0, "src",
                RuntimeEvent::AgentSpawned {
                    run_id: "r0".into(),
                    agent_id: "a0".into(),
                    role: "x".into(),
                    model: "y".into(),
                },
            );
            writeln!(f, "{}", serde_json::to_string(&env).unwrap()).unwrap();
            // Truncated trailing line:
            write!(f, r#"{{"seq":1,"partia"#).unwrap();
        }
        let count = std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .filter(|l| serde_json::from_str::<serde_json::Value>(l).is_ok())
            .count();
        assert_eq!(count, 1, "tolerant reader should ignore truncated lines");
    }
}
```

### Step 2 — Call `flush()` at workflow completion

`crates/roko-runtime/src/workflow_engine.rs::WorkflowEngine::run`. After
the workflow completes (i.e., after the final `RuntimeEvent::Workflow*`
emission), add:

```rust
// Persist buffered runtime events before returning.
if let Some(ref logger) = self.jsonl_logger {
    if let Err(err) = logger.flush() {
        tracing::warn!(error = %err, "failed to flush runtime event log");
    }
}
```

If `WorkflowEngine` does not directly own the `JsonlLogger` (it may be
wired through `EventBus` consumers), find the owner that constructs the
logger and add the flush call to that owner's run-exit. Search:

```bash
rg -n 'JsonlLogger::' crates/ --type rust
```

The flush must run at least once per `WorkflowEngine::run` exit.

### Step 3 — Verify reader tolerance (no code change)

Run:

```bash
rg -n 'runtime-events.jsonl' crates/ --type rust -A 3
```

For each consumer, confirm it uses one of the tolerant patterns:

- `lines.filter_map(|l| serde_json::from_str(&l).ok())`
- `for line in reader.lines() { let Ok(env) = serde_json::from_str(...) else { continue; }; }`

If you find a consumer that uses `?` or `unwrap()` on the parse, file a
follow-up note in the commit body. **Do not** "fix" it in this batch —
that's outside scope.

### Step 4 — (DEFERRED to PERF_11) Periodic flush in `roko serve`

The plan recommends a 5 s periodic flush task in `roko serve`. **Skip
that here** — `roko serve` startup is touched comprehensively by
PERF_11. Add a TODO comment in the workflow engine instead:

```rust
// TODO(PERF_11): roko serve should also schedule a periodic
// JsonlLogger::flush() task (every 5 s) so long-idle processes do
// not leave events in the buffer for hours.
```

## Write Scope

- `crates/roko-runtime/src/jsonl_logger.rs`
- `crates/roko-runtime/src/workflow_engine.rs`

## Read-Only Context

- `crates/roko-runtime/src/event_bus.rs`
- `crates/roko-core/src/runtime_event.rs`
- `crates/roko-fs/src/file_substrate.rs` (reference for partial-line tolerance)
- `tmp/solutions/perf/implementation/04-buffered-jsonl-logger.md`
- `tmp/runners/perf/context-pack/00-RULES.md`
- `tmp/runners/perf/context-pack/02-ANTI-PATTERNS.md` (AP-PERSIST-1)

## Acceptance Criteria

- [ ] `JsonlLogger::write_event` no longer calls `flush()` per event.
- [ ] `JsonlLogger::flush()` is `pub` and called from workflow completion in `WorkflowEngine::run` (or its owner).
- [ ] `Drop for JsonlLogger` flushes best-effort.
- [ ] `BufWriter` capacity raised to 8 KiB (`with_capacity(8 * 1024, file)`).
- [ ] Thread-local `SCRATCH` (`Vec<u8>`) used in `write_event`; envelope serialized via `serde_json::to_writer`.
- [ ] Test `buffered_writes_persist_after_explicit_flush` (1 000 events) passes.
- [ ] Test `dropped_logger_persists_buffered_events` passes.
- [ ] Test `reader_tolerates_partial_last_line` passes.
- [ ] No reader regression (audit recorded in commit body per Step 3).
- [ ] TODO comment for PERF_11 periodic flush added (Step 4).

## Verify

```bash
# Confirm no per-event flush remains in jsonl_logger.rs:
rg -n 'w\.flush\(\)' crates/roko-runtime/src/jsonl_logger.rs
# Expected: matches ONLY inside `pub fn flush(&self)`.

# Reader-tolerance audit:
rg -n 'runtime-events\.jsonl' crates/ --type rust -A 3 \
  | rg -B 1 -A 3 'serde_json::from_str.*\?'
# Expected: empty (no `?`-on-parse readers).

# Macro-benchmark (post-merge):
cargo test -p roko-runtime --release jsonl_logger
```

## Do NOT

- Do NOT remove the `Drop` flush. Panics before explicit flush would
  lose buffered events otherwise.
- Do NOT use `tokio::fs::File` for the writer. The writer lives inside
  a synchronous `Mutex<Option<...>>`; mixing tokio file IO with a sync
  mutex deadlocks the runtime. (See AP-ASYNC-1 / AP-ASYNC-2.)
- Do NOT use `std::io::Write::write` (one-shot single-byte write); use
  `write_all` to ensure full envelope writes.
- Do NOT raise `BUFFER_CAPACITY_BYTES` above 64 KiB. Larger buffers
  delay flushes too long and lose more events on crash.
- Do NOT spawn a flush task here for `roko serve`. That's PERF_11's job.
- Do NOT swap to `tracing-appender`. It is a tempting ready-made async
  logger, but switching event sinks affects the Cursor IDE plugin and
  ACP bridge — separate, larger change.
- Do NOT log the flush itself with `tracing::info!` — re-entrancy via
  any tracing-to-jsonl bridge creates a feedback loop. `tracing::debug!`
  only.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_04 done <commit-sha>
```
