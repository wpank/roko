# 88 — HandlerFutureLifecycle Double-Poll Causes Panic in Tool Dispatch

**Priority**: P1 — reliability (panic in tool dispatch hot path crashes the agent)
**Size**: XS (1 hour)
**Crates**: `crates/roko-agent` (`src/dispatcher/mod.rs`)
**Depends on**: None

---

## Background

The tool dispatcher in `roko-agent` runs each tool handler inside a `HandlerFutureLifecycle<F>` wrapper. This wrapper serves two purposes: it polls the inner handler future and it catches panics from the handler during both polling and drop via `HandlerPanicPollGuard` and `guarded_drop_handler_future`.

The pattern used for "owned future that becomes None after completion" is: the `future` field is `Option<Pin<Box<F>>>`. When the future completes with `Poll::Ready`, the wrapper calls `this.future.take()` to remove it and then calls `guarded_drop_handler_future` to drop it safely under the panic guard. After this point, `this.future` is `None`.

If a broken async executor — or a race condition in cancellation, or executor-specific edge cases in the Tokio runtime — causes the wrapper to be polled again after it already returned `Poll::Ready`, the `.expect("handler future is unavailable only after completion")` at line 182 panics. This panic happens inside the tool dispatch loop and crashes the agent process.

The same file already handles the `None` case gracefully in `guarded_drop_handler_future` at line 209 (it returns `Ok(())` immediately if the Option is already None). The pattern for a safe None check exists 27 lines below the problem. This fix converts the panic into a typed `ToolResult::err`, which the dispatcher can handle gracefully without crashing.

## Current State

1. File: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs`

2. `HandlerFutureLifecycle` struct at lines 151-153:
```rust
struct HandlerFutureLifecycle<F> {
    future: Option<std::pin::Pin<Box<F>>>,
}
```

3. The `Future` impl's `poll` method at lines 167-199. The problematic `.expect()` is at line 182:
```rust
fn poll(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
) -> std::task::Poll<Self::Output> {
    let this = self.get_mut();
    let outcome = {
        let _guard = HandlerPanicPollGuard::enter();
        this.future
            .as_mut()
            .expect("handler future is unavailable only after completion")  // ← line 182
            .as_mut()
            .poll(cx)
    };
    match outcome {
        std::task::Poll::Pending => std::task::Poll::Pending,
        std::task::Poll::Ready(result) => {
            if guarded_drop_handler_future(this.future.take()).is_err() {  // ← takes the Option
                std::task::Poll::Ready(ToolResult::err(ToolError::HandlerPanic(
                    "tool handler panicked".to_string(),
                )))
            } else {
                std::task::Poll::Ready(result)
            }
        }
    }
}
```

4. The `guarded_drop_handler_future` function at lines 209-215 already handles `None` safely:
```rust
fn guarded_drop_handler_future<F>(future: Option<std::pin::Pin<Box<F>>>) -> Result<(), ()> {
    let Some(future) = future else {
        return Ok(());  // ← graceful None handling already exists
    };
    let _guard = HandlerPanicPollGuard::enter();
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(future))).map_err(|_| ())
}
```

5. `ToolError::HandlerPanic` is used at line 191 in the poll method itself, confirming this variant exists and is the correct type to use for error reporting in this context.

6. `ToolResult::err(ToolError::HandlerPanic(...))` is the correct way to construct the error result — it is already used at line 190-192.

## Implementation Plan

### Step 1: Replace the `.expect()` with a `let...else` that returns a typed error

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs`, replace the block at lines 178-185:

**Before:**
```rust
let outcome = {
    let _guard = HandlerPanicPollGuard::enter();
    this.future
        .as_mut()
        .expect("handler future is unavailable only after completion")
        .as_mut()
        .poll(cx)
};
```

**After:**
```rust
let Some(fut) = this.future.as_mut() else {
    return std::task::Poll::Ready(ToolResult::err(ToolError::HandlerPanic(
        "handler future polled after completion".to_string(),
    )));
};
let outcome = {
    let _guard = HandlerPanicPollGuard::enter();
    fut.as_mut().poll(cx)
};
```

This converts the double-poll panic into a `Poll::Ready(ToolResult::err(...))`, which the dispatcher handles through its normal error reporting path.

### Step 2: Add a test that verifies double-poll does not panic

Add a test in the `#[cfg(test)]` block in `dispatcher/mod.rs`. The test needs to:
1. Create a `HandlerFutureLifecycle` wrapping a future that immediately returns `Poll::Ready`.
2. Poll it once with a no-op waker context — this should return `Poll::Ready(result)`.
3. Poll it a second time — before the fix this panics; after the fix it should return `Poll::Ready(ToolResult::err(...))`.

To poll a future manually in a test, you need a `std::task::Context` backed by a no-op waker. Tokio provides `tokio_test::task::spawn` or you can use `futures::task::noop_waker_ref()`. Check what test utilities are already used in this file's `#[cfg(test)]` section.

```rust
#[test]
fn double_poll_returns_error_not_panic() {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    // A future that completes immediately.
    let fut = std::future::ready(ToolResult::ok(serde_json::json!({"done": true})));
    let mut lifecycle = HandlerFutureLifecycle::new(fut);

    // Build a no-op waker.
    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);

    // First poll: should complete normally.
    let first = Pin::new(&mut lifecycle).poll(&mut cx);
    assert!(matches!(first, Poll::Ready(_)), "first poll must return Ready");

    // Second poll: future.is_none() after take(); should return error, not panic.
    let second = Pin::new(&mut lifecycle).poll(&mut cx);
    match second {
        Poll::Ready(result) => {
            assert!(result.is_err(), "double-poll result must be an error");
        }
        Poll::Pending => panic!("double-poll must not return Pending"),
    }
}
```

If `futures::task::noop_waker` is not available as a dependency, use the `tokio_test` crate's waker or look at what the existing tests in this file use.

## Acceptance Criteria

1. The `.expect("handler future is unavailable only after completion")` at line 182 is replaced with a `let Some(...) else { return Poll::Ready(ToolResult::err(...)) }` guard.
2. Polling a `HandlerFutureLifecycle` after it returned `Poll::Ready` returns `Poll::Ready(ToolResult::err(ToolError::HandlerPanic(...)))` instead of panicking.
3. A test named `double_poll_returns_error_not_panic` (or similar) passes, demonstrating the double-poll case returns an error.
4. Existing dispatcher tests pass.
5. `cargo test -p roko-agent` passes.
6. `cargo clippy -p roko-agent -- -D warnings` passes.

## Verification Checklist

- [ ] Read `dispatcher/mod.rs` lines 151-215 to understand the full structure before editing
- [ ] Make the replacement at lines 178-185
- [ ] Verify the `ToolResult::err(ToolError::HandlerPanic(...))` syntax matches how it is used on line 190-192
- [ ] Write the double-poll test
- [ ] Check what waker/context utility is available in the test dependencies
- [ ] Run `cargo test -p roko-agent`
- [ ] Run `cargo clippy -p roko-agent -- -D warnings`

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs` | Replace `.expect()` at line 182 with `let Some(...) else { return Poll::Ready(ToolResult::err(...)) }`; add double-poll test |
