# 04 — Buffered JSONL Event Logger (B11 + B13)

> Bottleneck: `JsonlLogger::write_event` flushes the writer **after every
> single event**. A typical run emits 20–40 events → 60–150 ms of
> synchronous disk I/O. A second, smaller win comes from avoiding
> redundant `RuntimeEventEnvelope::new` allocations.
>
> Target savings: 35–60 ms / run.
> Effort: ≈2 h. Risk: low (events are advisory; durability is preserved
> via explicit flush at run completion + Drop).

---

## Goal & success criteria

After this change, the runtime event log:

1. Buffers writes inside an 8 KiB `BufWriter` and **does not flush per
   event** — flushes happen on explicit `JsonlLogger::flush()`,
   periodic background interval, or `Drop`.
2. Reuses a thread-local `String` buffer for envelope serialization to
   avoid per-event allocation.
3. Survives crashes the same way the substrate does: a partial last
   line is tolerated by readers (already true; verify with a test).

Done when:

- A new test writes 1 000 events without explicit flush, then asserts
  all 1 000 are readable after `JsonlLogger::flush()` returns.
- A new test verifies that a `JsonlLogger` dropped without explicit
  flush still persists buffered events (relies on `BufWriter::drop`).
- The macro-benchmark p50 wall-time drops by ≥30 ms vs baseline (after
  plans 01–02).

---

## Background

- Bottleneck source: `BOTTLENECK-ANALYSIS.md` §B11 + §B13,
  `OPTIMIZATION-PLAYBOOK.md` §5 + §11.
- Current implementation (verified live):

  ```rust
  // crates/roko-runtime/src/jsonl_logger.rs
  fn write_event(&self, event: &RuntimeEvent) -> std::io::Result<()> {
      self.ensure_writer()?;
      let envelope = RuntimeEventEnvelope::new(...);
      let json = serde_json::to_string(&envelope)?;
      let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
      if let Some(ref mut w) = *writer {
          writeln!(w, "{json}")?;
          w.flush()?;            // ← per-event flush
      }
      Ok(())
  }
  ```

- The reader path (`runtime-events.jsonl` consumers in `roko-cli` and
  `roko-serve`) tolerates partial last lines because every reader uses
  the JSONL "ignore non-parsing lines" pattern (mirrors
  `crates/roko-fs/src/file_substrate.rs::replay_log`). Verify before
  removing the flush.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-runtime/src/jsonl_logger.rs` (full file is 127 LOC) | Primary edit site. |
| `crates/roko-core/src/runtime_event.rs` | `RuntimeEventEnvelope` shape; informs serialization buffer sizing. |
| `crates/roko-runtime/src/event_bus.rs` | `EventConsumer` is wired through `runtime_event_bus()`; the logger is one of many consumers. |
| `crates/roko-runtime/src/workflow_engine.rs` | Where the engine should call `flush()` at run completion. |
| `crates/roko-fs/src/file_substrate.rs::replay_log` | Reference for partial-line tolerance pattern. |

---

## Code-level plan

### Step 1 — Increase the BufWriter capacity and remove per-event flush

```rust
// crates/roko-runtime/src/jsonl_logger.rs

use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

const BUFFER_CAPACITY_BYTES: usize = 8 * 1024;
const SCRATCH_CAPACITY_BYTES: usize = 512;

thread_local! {
    static SCRATCH: RefCell<String> = RefCell::new(String::with_capacity(SCRATCH_CAPACITY_BYTES));
}

pub struct JsonlLogger {
    path: PathBuf,
    seq: AtomicU64,
    writer: Mutex<Option<std::io::BufWriter<std::fs::File>>>,
}

impl JsonlLogger {
    fn ensure_writer(&self) -> std::io::Result<()> {
        let mut writer = self.writer.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if writer.is_none() {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = std::fs::OpenOptions::new()
                .create(true).append(true).open(&self.path)?;
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
            // Direct serialization into the reused buffer.
            serde_json::to_writer(unsafe { scratch.as_mut_vec() }, &envelope)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
            scratch.push('\n');

            let mut writer = self.writer.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(ref mut w) = *writer {
                w.write_all(scratch.as_bytes())?;
                // NOTE: NO flush() here — flush happens via flush(), periodic task, or Drop.
            }
            Ok(())
        })
    }

    /// Force buffered events to disk. Call at run completion.
    pub fn flush(&self) -> std::io::Result<()> {
        let mut writer = self.writer.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(ref mut w) = *writer {
            w.flush()?;
        }
        Ok(())
    }
}

impl Drop for JsonlLogger {
    fn drop(&mut self) {
        let _ = self.flush();   // best-effort durability on shutdown
    }
}
```

> **Safety note on `as_mut_vec`.** `serde_json::to_writer` always emits
> valid UTF-8 (a serializer invariant). Wrapping it in `unsafe { ...
> as_mut_vec() }` is sound because we never expose the bytes between
> the write and the subsequent `push('\n')`. If reviewers push back, an
> equivalent safe form is `let mut bytes = Vec::with_capacity(...);
> serde_json::to_writer(&mut bytes, &envelope)?;` and write `bytes`.

### Step 2 — Call `flush()` at run completion

In `crates/roko-runtime/src/workflow_engine.rs`, locate the
`WorkflowEngine::run` exit. After all events are emitted (typically
after `RuntimeEvent::WorkflowCompleted`), call:

```rust
if let Some(ref logger) = self.jsonl_logger {
    if let Err(err) = logger.flush() {
        tracing::warn!(error = %err, "failed to flush runtime event log");
    }
}
```

If the workflow engine does not directly own the logger (it is wired
through `EventBus` consumers), expose a `flush_consumers()` helper on
the bus that walks all `EventConsumer`s and dispatches a synthetic
`Flush` envelope, OR (simpler) ensure the logger's owner explicitly
calls `flush()` from its `run` exit.

### Step 3 — Periodic background flush (optional, recommended for `roko serve`)

For long-lived processes (`roko serve`), an idle process could leave
events in the buffer for hours. Add a 5 s periodic flush task during
serve startup:

```rust
// crates/roko-serve/src/runtime.rs (or wherever serve owns the logger)
let logger = Arc::clone(&jsonl_logger);
tokio::spawn(async move {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        tick.tick().await;
        if let Err(err) = logger.flush() {
            tracing::warn!(error = %err, "periodic flush failed");
        }
    }
});
```

For one-shot `roko run`, the explicit flush + `Drop` is enough — no
background task needed.

### Step 4 — Verify reader tolerance for partial last lines

Read `crates/roko-cli` and `crates/roko-serve` consumers of
`runtime-events.jsonl` (`rg "runtime-events.jsonl"`). Confirm each one
matches the pattern:

```rust
for line in reader.lines() {
    let Ok(line) = line else { continue; };
    let Ok(envelope) = serde_json::from_str::<RuntimeEventEnvelope>(&line) else {
        continue;     // tolerate partial / corrupt last line
    };
    // ...
}
```

If any reader uses `?` to fail on bad lines, file a separate fix; do
not block this PR on it. (Document any offenders in the PR description.)

---

## Step-by-step execution

1. `git checkout -b perf/04-buffered-jsonl-logger`.
2. Apply Step 1 to `jsonl_logger.rs`.
3. Apply Step 2 to the workflow engine.
4. (Serve only) Apply Step 3.
5. Run audit (Step 4); document findings.
6. Add the new tests below.
7. Macro-benchmark before/after.
8. Open PR `perf(runtime): buffer JSONL event log; flush on completion (B11+B13)`.

---

## Anti-patterns / things NOT to do

- **Do NOT remove the `Drop` flush.** If a panic happens before the
  explicit flush, buffered events would be lost. `Drop` is the safety
  net.
- **Do NOT use `tokio::fs::File`** for the writer. The writer lives
  inside a synchronous `Mutex<Option<...>>`. Mixing tokio file IO with
  a sync mutex deadlocks the runtime.
  If you need async, use `tokio::sync::Mutex<tokio::io::BufWriter<...>>`
  *and* make `write_event` async. That's a larger refactor; skip it for
  this plan.
- **Do NOT use `std::io::Write::write` (single byte)** anywhere. Always
  `write_all` to ensure full envelope writes.
- **Do NOT set the buffer capacity above 64 KiB.** Larger buffers delay
  flushes too long and lose more events on crash. 8 KiB ≈ 30–50
  envelopes is the sweet spot.
- **Do NOT spawn a flush task per logger.** One periodic task per
  `roko serve` process is enough; multiple loggers can share it.
- **Do NOT swap to `tracing-appender`** in this plan. It is a tempting
  ready-made async logger, but switching event sinks is a much larger
  change with tooling (Cursor IDE plugin, ACP bridge) implications.
- **Do NOT log the flush itself with `tracing::info!`.** That re-enters
  the same writer through any tracing-to-jsonl bridge and creates a
  feedback loop. Use `tracing::debug!` only.

---

## Test plan

```rust
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
        // No explicit flush — relies on Drop.
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
        writeln!(f, r#"{{"seq":0,"payload":{{"kind":"agent_spawned"}}}}"#).unwrap();
        write!(f, r#"{{"seq":1,"partia"#).unwrap();   // truncated
    }
    let count = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .filter(|l| serde_json::from_str::<serde_json::Value>(l).is_ok())
        .count();
    assert_eq!(count, 1, "tolerant reader should ignore truncated lines");
}
```

Macro-benchmark:

```bash
RUST_LOG=roko=info /usr/bin/time -l ./target/release/roko run \
  --model gpt-4.1-nano --gates none "Reply hello" 2>&1 | tee bench.log
```

Expect ≥30 ms wall-clock improvement vs the plan-02 baseline.

---

## Rollback plan

- The change is local to `jsonl_logger.rs` + a single `flush()` call in
  the workflow engine. `git revert` is safe.
- If a downstream tool (Cursor plugin, ACP bridge) depends on per-event
  flush latency (e.g., it polls the file every 50 ms), restore the
  per-event flush behind a feature flag `jsonl-eager-flush` while you
  negotiate a better protocol with the consumer.

---

## Status check (acceptance)

- [ ] `JsonlLogger::write_event` no longer calls `flush()` per event.
- [ ] `JsonlLogger::flush()` is public, called from workflow completion.
- [ ] `Drop` flush implemented and tested.
- [ ] All three new tests green.
- [ ] No reader regression (audit recorded in PR description).
- [ ] Macro-benchmark improvement of ≥30 ms recorded.
