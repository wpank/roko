# ACP: Concurrency & Race Condition Fixes

> **Source**: All roko-acp modules
> **References**: `tmp/acp-features/00-ACP-FEATURES.md`
> **Created**: 2026-08-15

---

## Global Lock Bottlenecks

### 1. CASCADE_ROUTER_IO_LOCK serializes all cascade router I/O across sessions

- **File**: `crates/roko-acp/src/bridge_events.rs:152`
- **Code**:
  ```rust
  static CASCADE_ROUTER_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
  ```
  Used at line 1129 (`record_cascade_observation`) inside `task::spawn_blocking`:
  ```rust
  let _guard = CASCADE_ROUTER_IO_LOCK
      .get_or_init(|| Mutex::new(()))
      .lock()
      .unwrap_or_else(|error| error.into_inner());
  let router = CascadeRouter::load_or_new(&router_path, model_slugs);
  // ...
  router.observe(context_vec, model_idx, reward);
  router.save(&router_path)?;
  ```
  Also used in `cascade_select_model` (line 1031) for the read path, but **without** the lock --
  `CascadeRouter::load_or_new` is called directly, creating a read/write race with the
  `record_cascade_observation` writer.

- **Problem**: The global `Mutex<()>` serializes all cascade router writes across every
  ACP session in the process. If two sessions finish prompts at the same time, the second
  blocks on a `spawn_blocking` thread until the first finishes its disk I/O.
  Additionally, the *read* path in `cascade_select_model` is not guarded by this lock,
  so it can race with the *write* path: the reader loads a half-written JSON file, or
  reads a stale snapshot that the writer is about to overwrite.

- **Fix**: Replace the global `Mutex<()>` with a `tokio::sync::RwLock<()>` or, better,
  move the cascade router to a dedicated actor (channel + `spawn` task) that owns the
  file and serializes reads/writes internally. This eliminates blocking thread
  contention and the read/write race.
  As a minimal fix: wrap the read path in `cascade_select_model` with a shared (read)
  guard from the same lock, and switch the lock to `std::sync::RwLock<()>`.

### 2. EXPERIMENT_STORE_IO_LOCK serializes all experiment store I/O across sessions

- **File**: `crates/roko-acp/src/bridge_events.rs:153`
- **Code**:
  ```rust
  static EXPERIMENT_STORE_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
  ```
  Used at line 783 (`experiment_store_lock`) and consumed by both `assign_acp_experiment`
  (line 794) and `record_acp_experiment_outcome` (line 889).

- **Problem**: Same global serialization issue as CASCADE_ROUTER_IO_LOCK. All sessions
  contend on a single `std::sync::Mutex` for experiment store reads and writes.
  Because the lock is a `std::sync::Mutex` held across synchronous file I/O, it blocks
  the OS thread. If called from an async context without `spawn_blocking`, it blocks the
  tokio runtime. `assign_acp_experiment` is called at line 1687 from `handle_session_prompt_inner`,
  which runs on the async runtime -- this means the `std::sync::Mutex` lock + file I/O
  will block a tokio worker thread.

- **Fix**: Either:
  (a) Wrap the `assign_acp_experiment` call in `spawn_blocking` (consistent with
      `record_cascade_observation`), or
  (b) Move the experiment store to a `tokio::sync::Mutex` with `.await`-based locking,
      or an actor model.
  The same actor approach recommended for the cascade router applies here.

---

## Race Conditions

### 3. Session busy check-then-act (is_busy -> begin_prompt)

- **File**: `crates/roko-acp/src/bridge_events.rs:1638-1643`
- **Code**:
  ```rust
  if session.is_busy() {                        // line 1638 -- LOAD
      return Err(BridgeEventsError::SessionBusy(session.session_id.clone()));
  }
  session.ensure_provider_runtime(workdir, roko_config);
  session.begin_prompt();                        // line 1643 -- STORE
  ```
  Where `is_busy()` (session.rs:665-666) and `begin_prompt()` (session.rs:653-655) are:
  ```rust
  pub fn is_busy(&self) -> bool {
      self.busy.load(Ordering::Acquire)
  }
  pub fn begin_prompt(&mut self) {
      self.cancel_token = CancelToken::new();
      self.busy.store(true, Ordering::Release);
  }
  ```

- **Problem**: Classic check-then-act TOCTOU race. Even though `busy` is an `AtomicBool`,
  the check (`is_busy`) and the set (`begin_prompt`) are two separate operations with
  non-atomic user code between them (`ensure_provider_runtime`). If two tasks could
  call `handle_session_prompt` concurrently for the same session, both could pass the
  `is_busy()` check before either calls `begin_prompt()`.

  In the current architecture this is mitigated because the handler loop in
  `handler.rs` is a sequential `loop { read_message().await; handle_request().await; }`
  -- there is only one concurrent handler per transport. However:
  (a) The `busy` field is `Arc<AtomicBool>` specifically designed for cross-task sharing.
  (b) The session comment at session.rs:1142-1145 explicitly acknowledges future concurrent
      transports: "If ACP gains concurrent transports, wrap it..."
  (c) `session.cancel()` (line 647-649) sets `busy = false` from a *different* code path
      (the notification handler), which could interleave with a prompt that is
      between `is_busy()` and `begin_prompt()`.

- **Fix**: Replace the two-step check-then-act with an atomic `compare_exchange`:
  ```rust
  pub fn try_begin_prompt(&mut self) -> bool {
      let was_idle = self.busy.compare_exchange(
          false, true,
          Ordering::AcqRel, Ordering::Acquire,
      ).is_ok();
      if was_idle {
          self.cancel_token = CancelToken::new();
      }
      was_idle
  }
  ```
  Then in `handle_session_prompt`:
  ```rust
  if !session.try_begin_prompt() {
      return Err(BridgeEventsError::SessionBusy(session.session_id.clone()));
  }
  ```

### 4. CancelToken replacement race in begin_prompt vs. cancel

- **File**: `crates/roko-acp/src/session.rs:647-655`
- **Code**:
  ```rust
  pub fn cancel(&mut self) {
      self.cancel_token.cancel();          // signals the OLD token
      self.busy.store(false, Ordering::Release);
  }

  pub fn begin_prompt(&mut self) {
      self.cancel_token = CancelToken::new();  // replaces the token
      self.busy.store(true, Ordering::Release);
  }
  ```

- **Problem**: If `cancel()` and `begin_prompt()` execute in close succession (e.g., a
  cancel notification arrives just as a new prompt is starting), the following interleaving
  can occur:

  1. Thread A: `begin_prompt()` -- creates new CancelToken T2, stores `busy = true`
  2. Thread B (notification): `cancel()` -- cancels T2 (the new prompt's token!), stores `busy = false`

  The new prompt's token is cancelled immediately, even though the cancellation was
  intended for the previous (already-finished) prompt. The `busy = false` store also
  incorrectly marks the freshly-started prompt as idle.

  Additionally, the cognitive task spawned at line 1964 captures `session.cancel_token.clone()`
  *after* `begin_prompt()`, while the streaming loop at line 1880 also clones it. If
  `cancel()` fires between `begin_prompt()` and those clones, the cancel is lost (the
  old token was cancelled, the new one was replaced).

- **Fix**: Use a monotonic generation counter alongside the cancel token. `cancel()` should
  only cancel the token matching the current generation. Or use a single long-lived
  `CancelToken` per session with a `reset()` method that atomically transitions from
  cancelled-or-idle to active, rejecting concurrent `cancel()` calls that target a
  stale generation.

### 5. Cascade router read/write race in cascade_select_model

- **File**: `crates/roko-acp/src/bridge_events.rs:1031` (read) vs. `bridge_events.rs:1128-1156` (write)
- **Code** (read, line 1031):
  ```rust
  let router = CascadeRouter::load_or_new(&router_path, model_slugs);
  ```
  (write, lines 1128-1156):
  ```rust
  task::spawn_blocking(move || {
      let _guard = CASCADE_ROUTER_IO_LOCK...lock()...;
      let router = CascadeRouter::load_or_new(&router_path, model_slugs);
      router.observe(context_vec, model_idx, reward);
      router.save(&router_path)?;
  });
  ```

- **Problem**: The read in `cascade_select_model` is not guarded by the lock. A concurrent
  `record_cascade_observation` call could be mid-`save()` when `cascade_select_model` reads
  the file. Depending on the OS and filesystem, this could yield a truncated or corrupt JSON
  read, causing `load_or_new` to silently fall back to a fresh router (losing learned state
  for that routing decision).

- **Fix**: Guard the read path with the same lock (using `RwLock` read guard), or use
  atomic file writes (`write-to-temp + rename`) in `CascadeRouter::save()` so readers
  always see a complete file.

---

## Channel Issues

### 6. Cognitive event channel capacity may cause backpressure stalls

- **File**: `crates/roko-acp/src/bridge_events.rs:1864`
- **Code**:
  ```rust
  let (event_sender, event_receiver) = mpsc::channel(64);
  ```

- **Problem**: The cognitive event channel has a capacity of 64. The cognitive task (spawned
  at line 1964) produces events from the model stream, tool calls, and other subsystems.
  If the consumer (`stream_events_to_editor`, line 2139) stalls due to transport backpressure
  (e.g., the editor is slow to read from stdout), the producer blocks on `event_sender.send()`
  once the buffer fills. This can cause the model stream to stall, potentially triggering
  provider-side timeouts.

  Meanwhile, the `AcpWorkflowEventConsumer::publish()` method (runner.rs:657) uses
  `try_send` which silently drops events on backpressure:
  ```rust
  fn publish(&self, event: CognitiveEvent) {
      let _ = self.sender.try_send(event);
  }
  ```
  This means workflow events (token chunks, plan updates) are silently lost when the
  channel is full, leading to incomplete streaming output in the editor.

- **Fix**:
  (a) Increase the channel capacity to 256 or 512 for the main event channel.
  (b) Change `publish()` to use `send().await` or at minimum log when events are dropped.
  (c) Consider using an unbounded channel for cognitive events since the consumer is
      always running (the producer is the bottleneck, not memory).

### 7. Permission reply channel drop semantics

- **File**: `crates/roko-acp/src/bridge_events.rs:222-269`
- **Code** (PermissionReplyChannel):
  ```rust
  pub struct PermissionReplyChannel {
      inner: Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<PermissionDecision>>>>,
  }
  ```

- **Problem**: The `PermissionReplyChannel` wraps a oneshot sender behind `Arc<Mutex<Option<>>>`.
  It is `Clone`, and the doc comment says "the first call to `reply` takes the sender...
  subsequent calls return false." But the `Clone` trait means multiple tasks could hold
  references and try to `reply()` concurrently. While the `Mutex` ensures the `take()` is
  atomic, there is a semantic issue: the channel is used in `request_permission_for_event`
  (line 1401-1447), which polls `reply.receiver_is_closed()` in a loop with 25ms sleeps.
  `receiver_is_closed()` acquires the same `std::sync::Mutex`:
  ```rust
  pub fn receiver_is_closed(&self) -> bool {
      self.inner
          .lock()
          .expect("PermissionReplyChannel mutex poisoned")
          ...
  }
  ```
  This is a `std::sync::Mutex` polled from an async context every 25ms. While unlikely
  to cause significant contention in practice, it creates a spin-poll pattern on a
  blocking mutex from an async task, which is the kind of code that triggers
  "blocking in async context" warnings from tokio instrumentation.

- **Fix**: Replace the `std::sync::Mutex<Option<oneshot::Sender>>` with
  `tokio::sync::Mutex` or refactor to use a `tokio::sync::watch` channel that
  the async poller can `.changed().await` on, eliminating the 25ms sleep-poll entirely.

---

## Thread Safety Gaps

### 8. std::sync::Mutex held in async context (experiment store)

- **File**: `crates/roko-acp/src/bridge_events.rs:782-787`
- **Code**:
  ```rust
  fn experiment_store_lock() -> std::sync::MutexGuard<'static, ()> {
      EXPERIMENT_STORE_IO_LOCK
          .get_or_init(|| Mutex::new(()))
          .lock()
          .unwrap_or_else(std::sync::PoisonError::into_inner)
  }
  ```
  Called from `assign_acp_experiment` (line 794) which is invoked at line 1687 from
  `handle_session_prompt_inner` -- an async function running on the tokio runtime.

- **Problem**: A `std::sync::Mutex` guard is held across synchronous file I/O
  (`ExperimentStore::load_or_new(path)` reads a JSON file from disk) while running
  on a tokio worker thread. If the file I/O blocks (e.g., NFS, slow disk), the tokio
  worker thread is blocked and cannot service other tasks. This violates tokio's
  cooperative scheduling contract.

- **Fix**: Wrap `assign_acp_experiment` in `tokio::task::spawn_blocking()` so the
  blocking file I/O and mutex acquisition happen on the blocking thread pool, not
  the async runtime. Alternatively, switch to `tokio::sync::Mutex` and use async
  file I/O.

### 9. workflow_cost_sink uses std::sync::Mutex across spawn boundary

- **File**: `crates/roko-acp/src/bridge_events.rs:1961-1962`
- **Code**:
  ```rust
  let workflow_cost_sink: Arc<Mutex<Option<f64>>> = Arc::new(Mutex::new(None));
  let workflow_cost_sink_task = Arc::clone(&workflow_cost_sink);
  ```
  Written inside `tokio::spawn` (line 2064-2068):
  ```rust
  if let Some(cost) = report.cost
      && let Ok(mut sink) = workflow_cost_sink_task.lock()
  {
      *sink = Some(cost);
  }
  ```
  Read at line 2248:
  ```rust
  let cost_override = workflow_cost_sink.lock().ok().and_then(|g| *g);
  ```

- **Problem**: The `std::sync::Mutex` is locked from both the spawned task and the
  parent async context. While the critical section is trivial (just writing an `Option<f64>`),
  if the mutex were poisoned by a panic in the spawned task, the `.ok()` silently
  swallows the error and falls back to `None`, losing the actual cost data. More
  importantly, this is a `std::sync::Mutex` used from async code -- if the spawned
  task holds the lock when the parent reads it, the parent blocks a tokio worker thread.

- **Fix**: Replace with `tokio::sync::oneshot` channel. The spawned task sends the cost
  once, the parent `.await`s it. This is the natural pattern for a single value transfer
  from a spawned task to its parent.

---

## Transport Layer Issues

### 10. Transport read/write interleaving during permission requests

- **File**: `crates/roko-acp/src/bridge_events.rs:1250-1394`
- **Code**: In `request_permission()`:
  ```rust
  let mut request_transport = transport.clone();
  let request_future = request_transport.send_request("session/request_permission", params);
  // ...
  loop {
      tokio::select! {
          response = &mut request_future => { ... }
          inbound = transport.read_message() => { ... }
      }
  }
  ```

- **Problem**: The transport is cloned and used for both writing (via `request_transport`)
  and reading (via `transport`) concurrently in the same `select!`. The `StdioTransport`
  uses `Arc<AsyncMutex<W>>` for the writer and `Arc<AsyncMutex<BufReader<R>>>` for the
  reader (transport.rs:48-49), so concurrent access to the same underlying stream is
  serialized. However, the `send_request` future holds the writer lock while waiting
  for `flush()`, and `read_message` holds the reader lock -- these are independent.

  The real issue is that `handle_incoming_response` (called at line 1329) and the
  pending request registry are accessed from the `inbound` branch using `transport`
  (the original), while `send_request` inserts into the registry using
  `request_transport` (the clone). Both share the same `Arc<Mutex<HashMap>>>`
  (`pending_requests`). The `std::sync::Mutex` used for `pending_requests` is fine
  for synchronization, but the pattern of reading messages on the main loop while
  the handler loop in `handler.rs` also reads messages is concerning: during a
  permission request, the main handler loop is blocked (it `await`s `handle_request`
  which `await`s `handle_session_prompt` which `await`s `request_permission`), so
  there is no actual concurrent reader. But this is fragile -- any refactoring that
  makes the handler loop concurrent would create a double-reader bug.

- **Fix**: Document the invariant that only one reader may be active at a time. Consider
  adding a `debug_assert!` or reader-lock guard to enforce this. In a future concurrent
  handler, the transport reader would need to be split into a dedicated read task that
  dispatches messages to the correct handler.

### 11. next_id counter uses Ordering::Relaxed

- **File**: `crates/roko-acp/src/transport.rs:190`
- **Code**:
  ```rust
  let request_id = self.next_id.fetch_add(1, Ordering::Relaxed);
  ```

- **Problem**: `Ordering::Relaxed` provides no synchronization guarantees beyond atomicity.
  If two threads concurrently call `send_request`, each gets a unique ID (the `fetch_add`
  is atomic), but the subsequent `pending_requests.lock().insert(request_id, sender)` and
  `write_message(&request)` may execute in a different order than the IDs were assigned.
  This is not a correctness bug (each ID is unique and the pending map is mutex-protected),
  but `Ordering::Relaxed` on a monotonic counter is unnecessarily weak. On x86 this is
  harmless, but on ARM it could (in theory) allow reordering that makes debugging confusing
  when IDs appear out of order in logs.

- **Fix**: Use `Ordering::Relaxed` is actually fine here since uniqueness is the only
  requirement. The `Mutex` on `pending_requests` provides the necessary happens-before
  for the map insertion. **No change needed** -- this is a false positive, documented
  here for completeness.

---

## Atomics Ordering Audit

### 12. AtomicBool ordering is correct but inconsistent with usage pattern

- **File**: `crates/roko-acp/src/session.rs:649-666`
- **Code**:
  ```rust
  // cancel():
  self.busy.store(false, Ordering::Release);
  // begin_prompt():
  self.busy.store(true, Ordering::Release);
  // finish_prompt():
  self.busy.store(false, Ordering::Release);
  // is_busy():
  self.busy.load(Ordering::Acquire)
  ```

- **Problem**: The Acquire/Release pairing is correct for a simple flag, but it provides
  no guarantee that the *other* fields modified alongside `busy` (e.g., `cancel_token` in
  `begin_prompt`) are visible to readers. In practice, since the handler loop is
  single-threaded, this does not cause bugs today. But the `Arc<AtomicBool>` makes
  the field shareable across tasks, and `cancel()` can be called from the notification
  handler path (handler.rs:497), creating a cross-task mutation pattern where the
  `Acquire/Release` on `busy` does not establish a happens-before for `cancel_token`.

- **Fix**: If `busy` is ever read from a different task than the one that called
  `begin_prompt()`, the `cancel_token` replacement must also be visible. Either:
  (a) Keep the current single-threaded handler loop and document the invariant.
  (b) Wrap `busy` + `cancel_token` in a single synchronized structure (e.g.,
      `tokio::sync::Mutex<PromptState>`) if concurrent access is needed.

---

## Summary

| # | Category | Severity | File | Line | Status |
|---|----------|----------|------|------|--------|
| 1 | Global lock | Medium | bridge_events.rs | 152 | Open |
| 2 | Global lock | Medium | bridge_events.rs | 153 | Open |
| 3 | Race condition | Medium | bridge_events.rs | 1638-1643 | Mitigated (single-threaded loop) |
| 4 | Race condition | High | session.rs | 647-655 | Open (cancel interleaving) |
| 5 | Race condition | Medium | bridge_events.rs | 1031 vs 1128 | Open |
| 6 | Channel | Low | bridge_events.rs | 1864 | Open (silent event drops) |
| 7 | Thread safety | Low | bridge_events.rs | 222-269 | Open (spin-poll on sync mutex) |
| 8 | Thread safety | Medium | bridge_events.rs | 782-787 | Open (sync mutex in async) |
| 9 | Thread safety | Low | bridge_events.rs | 1961 | Open (could use oneshot) |
| 10 | Transport | Low | bridge_events.rs | 1250-1394 | Mitigated (fragile invariant) |
| 11 | Atomics | None | transport.rs | 190 | Not a bug |
| 12 | Atomics | Low | session.rs | 649-666 | Mitigated (single-threaded) |

**Priority fixes**: Issues 4 (cancel race), 8 (blocking async), 5 (read/write race), 1-2 (global locks).
