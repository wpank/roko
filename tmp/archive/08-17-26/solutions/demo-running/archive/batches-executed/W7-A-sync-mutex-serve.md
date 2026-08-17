# W7-A: Replace parking_lot::Mutex with tokio in serve

**Priority**: P2 — concurrency correctness
**Effort**: 30 minutes
**Files to modify**: 1 file
**Dependencies**: None

## Problem

`crates/roko-serve/src/state.rs` line 361 uses `parking_lot::Mutex<DaimonState>` which blocks the OS thread when held from async handlers. This can cause latency spikes under concurrent requests.

## Fix

### File: `crates/roko-serve/src/state.rs`

### Change 1: Import

```rust
// BEFORE:
use parking_lot::Mutex;

// AFTER:
use tokio::sync::Mutex;
```

### Change 2: Field type stays the same

```rust
pub affect_engine: Mutex<DaimonState>,  // now tokio::sync::Mutex
```

### Change 3: Update all accesses

`parking_lot::Mutex::lock()` is synchronous. `tokio::sync::Mutex::lock()` is async.

```bash
grep -n 'affect_engine.lock()' crates/roko-serve/src/ -r
```

For each access:
```rust
// BEFORE:
let state = self.affect_engine.lock();

// AFTER:
let state = self.affect_engine.lock().await;
```

**Important**: Only change accesses that are already in async functions. If any sync functions access `affect_engine`, they'll need to be made async or use `try_lock()`.

### Check for other parking_lot::Mutex uses

```bash
grep -rn 'parking_lot::Mutex\|parking_lot::RwLock' crates/roko-serve/src/ --include='*.rs'
```

If there are others in async handlers, convert them too. But do NOT convert mutexes that are held only briefly and never across `.await` points — those are fine with parking_lot.

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W7-A-sync-mutex-serve.md and implement all changes. Replace parking_lot::Mutex with tokio::sync::Mutex for affect_engine in crates/roko-serve/src/state.rs. Update all .lock() calls to .lock().await. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 7+8 batches together. Do not commit individually.

## Checklist

- [x] Replace `parking_lot::Mutex` with `tokio::sync::Mutex` for `affect_engine`
- [x] Update all `.lock()` calls to `.lock().await`
- [x] Verify async functions using the mutex compile
- [x] Check for other parking_lot mutexes in async handlers
- [ ] Pre-commit checks pass
