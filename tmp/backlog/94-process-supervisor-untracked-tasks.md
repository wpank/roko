# 94 — ProcessSupervisor Fire-and-Forget Spawned Tasks Not Tracked

**Priority**: P2 — Reliability: untracked cancellation-watcher tasks hold Arc references that can delay cleanup and cannot be awaited during shutdown
**Size**: S (half day)
**Crates**: `crates/roko-runtime/` (`src/process.rs`)
**Depends on**: None

---

## Background

`ProcessSupervisor` manages long-running subprocesses (agents, sidecars) and provides `shutdown_all()` and `Drop` paths to clean them up. When a process is spawned with an external `CancellationToken` argument, the supervisor creates a separate async task to watch for cancellation: when the external token fires, the watcher task removes the process from the supervisor's handles map and shuts it down.

The problem is that the `JoinHandle` returned by `tokio::spawn(...)` for this watcher task is immediately dropped with `std::mem::drop(...)`. This means:

1. The supervisor has no reference to the watcher tasks and cannot await or abort them.
2. Each watcher task holds an `Arc::clone(&self.handles)` reference. The `handles` map is a `Arc<Mutex<HashMap<ProcessId, ProcessHandle>>>`. Even after the supervisor itself is dropped, the watcher tasks keep the `Arc` alive until they complete.
3. During `Drop`, the supervisor cancels its root `CancelToken`, which triggers the child cancellation cascade and will eventually wake the watcher tasks — but since they are not awaited, `Drop` returns before they finish. This leaves a window where the watcher tasks run on an Arc-extended-lifetime handles map after the supervisor struct is gone.

In practice this is rarely observable because the Arc prevents use-after-free, and the cancellation cascade means the watcher tasks do complete quickly. But the pattern is logically incorrect, prevents testing, and can cause delayed cleanup in resource-constrained environments.

## Current State

All code references are in `crates/roko-runtime/src/process.rs`.

1. **Fire-and-forget spawn** (lines 924–933):
   ```rust
   if let Some(token) = external_cancellation {
       let handles = Arc::clone(&self.handles);
       std::mem::drop(tokio::spawn(async move {
           token.cancelled().await;
           let mut handle = { handles.lock().remove(&id) };
           if let Some(mut handle) = handle.take() {
               let _ = handle.shutdown().await;
           }
       }));
   }
   ```

2. **`ProcessSupervisor` struct** (lines 839–840):
   ```rust
   pub struct ProcessSupervisor {
       handles: Arc<Mutex<HashMap<ProcessId, ProcessHandle>>>,
       // ...
   }
   ```
   There is currently no field for tracking cancellation watcher handles.

3. **`shutdown_all`** (lines 951–963): Cancels the root token, drains the `handles` map, and shuts down each `ProcessHandle`. The watcher tasks will eventually wake up (because the root cancel cascades to child tokens), but `handles.lock().remove(&id)` on a now-drained map returns `None`, so they do nothing useful.

4. **`Drop`** (lines 1248–1269): Cancels the root token and force-kills remaining tracked processes. Cannot await the watcher tasks.

## Implementation Plan

### Step 1: Add a watcher task storage field to `ProcessSupervisor`

The simplest approach is a separate `Mutex<Vec<JoinHandle<()>>>` field. A `Mutex<Vec<...>>` (not a `HashMap`) is sufficient because we only need to abort/await all watchers at shutdown, not address individual ones by ID:

```rust
pub struct ProcessSupervisor {
    handles: Arc<Mutex<HashMap<ProcessId, ProcessHandle>>>,
    // NEW: track cancellation watcher tasks so they can be aborted on shutdown
    watcher_tasks: Mutex<Vec<JoinHandle<()>>>,
    cancel: CancelToken,
    // ... existing fields
}
```

Initialize in the constructor (search for `ProcessSupervisor::new` or the struct initializer):
```rust
watcher_tasks: Mutex::new(Vec::new()),
```

### Step 2: Store the JoinHandle instead of dropping it

Replace the `std::mem::drop(tokio::spawn(...))` pattern:

```rust
// Before:
if let Some(token) = external_cancellation {
    let handles = Arc::clone(&self.handles);
    std::mem::drop(tokio::spawn(async move {
        token.cancelled().await;
        let mut handle = { handles.lock().remove(&id) };
        if let Some(mut handle) = handle.take() {
            let _ = handle.shutdown().await;
        }
    }));
}

// After:
if let Some(token) = external_cancellation {
    let handles = Arc::clone(&self.handles);
    let watcher = tokio::spawn(async move {
        token.cancelled().await;
        let mut handle = { handles.lock().remove(&id) };
        if let Some(mut handle) = handle.take() {
            let _ = handle.shutdown().await;
        }
    });
    self.watcher_tasks.lock().push(watcher);
}
```

### Step 3: Abort watchers in `shutdown_all`

After draining the handles map in `shutdown_all` (line 954–955), abort all watcher tasks:

```rust
pub async fn shutdown_all(&self) -> Vec<ProcessOutcome> {
    self.cancel.cancel();
    let handles: Vec<_> = {
        let mut map = self.handles.lock();
        map.drain().map(|(_, h)| h).collect()
    };

    // NEW: abort and collect all cancellation watcher tasks.
    let watchers: Vec<_> = self.watcher_tasks.lock().drain(..).collect();
    for watcher in watchers {
        watcher.abort();
    }

    let mut outcomes = Vec::with_capacity(handles.len());
    for mut handle in handles {
        outcomes.push(handle.shutdown().await);
    }
    outcomes
}
```

Abort (rather than await) is appropriate here because `shutdown_all` already shut down the processes directly. The watcher tasks would find an empty handles map and do nothing anyway — aborting is faster and equivalent.

### Step 4: Abort watchers in `Drop`

In the `Drop` impl (line 1248), abort the watchers after cancelling the token:

```rust
impl Drop for ProcessSupervisor {
    fn drop(&mut self) {
        self.cancel.cancel();

        // NEW: abort all pending watcher tasks so they release Arc references promptly.
        for watcher in self.watcher_tasks.lock().drain(..) {
            watcher.abort();
        }

        let children = {
            let mut handles = self.handles.lock();
            if handles.is_empty() {
                return;
            }
            warn!(
                count = handles.len(),
                "ProcessSupervisor dropped with live children; force-killing"
            );
            std::mem::take(&mut *handles)
        };

        for (_, mut handle) in children {
            handle.force_kill_sync();
        }
    }
}
```

### Step 5: Add a test

In the `tests` module at the bottom of `crates/roko-runtime/src/process.rs`, add:

```rust
#[tokio::test]
async fn spawn_with_external_cancellation_no_lingering_arc() {
    use tokio_util::sync::CancellationToken;

    let root_cancel = CancellationToken::new();
    let supervisor = ProcessSupervisor::new(root_cancel.clone());

    // Spawn a no-op process with an external cancellation token.
    let external = CancellationToken::new();
    let _id = supervisor
        .spawn(
            SpawnConfig {
                label: "test-process".to_string(),
                program: "true".to_string(), // exits immediately
                args: vec![],
                grace_period: std::time::Duration::from_millis(100),
                ..SpawnConfig::default()
            },
            Some(external.clone()),
        )
        .await
        .expect("spawn");

    // Verify that at least one watcher task was registered.
    assert_eq!(supervisor.watcher_tasks.lock().len(), 1);

    // shutdown_all should drain both handles and watchers.
    supervisor.shutdown_all().await;

    assert!(supervisor.watcher_tasks.lock().is_empty());
    assert!(supervisor.handles.lock().is_empty());
}
```

Note: `SpawnConfig::default()` may not exist; check the existing test helpers in the module (e.g. `spawn_and_reap` at line 1275) for the correct way to build a `SpawnConfig` in tests. Use the same pattern.

## Acceptance Criteria

1. `ProcessSupervisor` has a `watcher_tasks: Mutex<Vec<JoinHandle<()>>>` field (or equivalent).
2. Every `tokio::spawn(...)` call for a cancellation watcher stores its `JoinHandle` in `watcher_tasks`.
3. `shutdown_all()` aborts (or awaits) all watcher tasks and clears `watcher_tasks`.
4. `Drop` aborts all watcher tasks before the struct is freed.
5. After `shutdown_all()` returns, `Arc::strong_count` on the `handles` arc is 1 (held only by the supervisor — no watcher tasks holding clones).
6. All existing `ProcessSupervisor` tests pass without modification.
7. New test `spawn_with_external_cancellation_no_lingering_arc` passes.

## Verification Checklist

- [ ] `cargo test -p roko-runtime -- process` passes
- [ ] `cargo clippy -p roko-runtime -- -D warnings` passes
- [ ] Confirm `std::mem::drop(tokio::spawn(...))` pattern no longer exists in `process.rs`
- [ ] Confirm `watcher_tasks` is properly initialized in all `ProcessSupervisor` constructors (search for `ProcessSupervisor {` in the file)

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-runtime/src/process.rs` | Add `watcher_tasks: Mutex<Vec<JoinHandle<()>>>` field; initialize in constructor; store handle instead of dropping in external-cancellation spawn path; abort watchers in `shutdown_all` and `Drop`; add new test |
