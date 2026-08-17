# Multi-Process Locking

**Priority**: P2 — data corruption from concurrent writes, silent lost updates
**Size**: S (1 day)
**Crates**: `crates/roko-cli/`, `crates/roko-learn/`, `crates/roko-serve/`

---

## Problem

Multiple roko processes can run concurrently against the same `.roko/` directory.
Common scenarios:

- `roko plan run` in one terminal + `roko serve` in another (serve reads/writes the
  same learning and state files)
- `roko plan run` in one terminal + `roko prd draft` in another (both write to
  `.roko/prd/`)
- `roko plan run` + `roko plan run` (two plan runners writing `state-snapshot.json`
  concurrently)
- `roko daemon` background process + any foreground CLI command

There is no file locking to prevent concurrent writes to shared state files. Two
processes writing `cascade-router.json` at the same time can interleave bytes, producing
invalid JSON. Two plan runners can both claim the same task, execute it twice, and write
conflicting snapshots.

---

## Where to look

- `crates/roko-cli/src/runner/event_loop.rs` — plan runner main loop, should acquire
  exclusive lock
- `crates/roko-cli/src/runner/persist.rs` — state snapshot writes
- `crates/roko-cli/src/runner/state.rs` — runner state management
- `crates/roko-learn/src/costs_db.rs` — cascade router file I/O
- `crates/roko-learn/src/feedback_service.rs` — gate threshold file I/O
- `crates/roko-learn/src/episode_logger.rs` — episode JSONL appends
- `crates/roko-serve/src/runtime.rs` — serve startup, should check for conflicts
- `crates/roko-cli/src/daemon.rs` — daemon lifecycle

---

## What to do

**Step 1.** Add `fs2` or `fd-lock` as a workspace dependency. Both provide cross-platform
advisory file locking. `fs2` is more established; `fd-lock` is smaller.

**Step 2.** Create a lock manager that acquires a lock file at `.roko/runtime/roko.lock`:

```rust
pub struct ProcessLock {
    _file: std::fs::File,
}

impl ProcessLock {
    /// Acquire an exclusive lock. Returns Err if another process holds it.
    pub fn try_exclusive(roko_dir: &Path, timeout: Duration) -> Result<Self> {
        let lock_path = roko_dir.join("runtime/roko.lock");
        fs::create_dir_all(lock_path.parent().unwrap())?;
        let file = File::create(&lock_path)?;
        // Try with timeout
        if !file.try_lock_exclusive()? {
            return Err(anyhow!(
                "Another roko process is running (lock held on {}). \
                 Stop it first or wait for it to finish.",
                lock_path.display()
            ));
        }
        Ok(Self { _file: file })
    }

    /// Acquire a shared lock (for read-only operations).
    pub fn try_shared(roko_dir: &Path) -> Result<Self> { ... }
}
// Lock released automatically when ProcessLock is dropped (file closed).
```

**Step 3.** Integrate the lock at these entry points:

| Command | Lock type | Why |
|---|---|---|
| `roko plan run` | Exclusive | Writes state-snapshot, learning files, episodes |
| `roko serve` | Exclusive | Writes learning files, runs feed agents |
| `roko daemon` | Exclusive | Long-running, writes state |
| `roko prd draft` | Shared | Writes only to `.roko/prd/`, low conflict risk |
| `roko status` | Shared | Read-only |
| `roko learn all` | Shared | Read-only |
| `roko dashboard` | Shared | Read-only (TUI watches files) |
| `roko doctor` | None | Diagnostic, should always work |

**Step 4.** Handle the stale lock case. Advisory locks are released when the process
exits (even on crash), so `fs2`/`fd-lock` handles this automatically — the lock is tied
to the file descriptor, not the file contents. No PID-file cleanup logic is needed.

---

## Acceptance criteria

- [ ] Lock file created at `.roko/runtime/roko.lock`
- [ ] `roko plan run` acquires exclusive lock before entering event loop
- [ ] `roko serve` acquires exclusive lock before binding port
- [ ] Read-only commands (`status`, `learn all`, `dashboard`) acquire shared locks
- [ ] Second `roko plan run` fails immediately with a clear error message naming the
  lock file and suggesting to stop the other process
- [ ] Lock is released on normal exit, SIGTERM, SIGINT, and process crash
- [ ] `roko doctor` works without acquiring any lock
- [ ] All existing tests pass (`cargo test --workspace`)

### Verify

1. Start `roko plan run` in terminal A
2. Run `roko plan run` in terminal B
3. Terminal B should print: "Another roko process is running (lock held on
   .roko/runtime/roko.lock). Stop it first or wait for it to finish."
4. Kill terminal A's process
5. Run `roko plan run` in terminal B — it should acquire the lock and proceed
6. Run `roko status` while `roko plan run` is active — it should succeed (shared lock)

### Not in scope

- Atomic file writes (backlog item 36 — complementary but independent)
- Distributed locking across machines (not relevant for local `.roko/`)
- Fine-grained per-file locking (the single process lock is sufficient for now)

---

**Origin**: redesign-plan.md (Phase 13)
