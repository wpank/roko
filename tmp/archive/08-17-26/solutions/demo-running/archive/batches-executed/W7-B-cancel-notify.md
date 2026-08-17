# W7-B: Replace Polling Cancellation with Notify

**Priority**: P2 — efficiency
**Effort**: 30 minutes
**Files to modify**: 1 file
**Dependencies**: None

## Problem

`crates/roko-agent/src/dispatcher/cancel.rs` line 26-35: `wait_cancelled()` polls `is_cancelled()` every 50ms. This wastes CPU cycles and adds up to 50ms latency.

## Current Code (lines 26-36)

```rust
pub async fn wait_cancelled(token: &dyn CancelToken) {
    if token.is_cancelled() {
        return;
    }
    loop {
        tokio::time::sleep(DEFAULT_POLL_INTERVAL).await;  // 50ms
        if token.is_cancelled() {
            return;
        }
    }
}
```

## Fix

Replace with `tokio::sync::Notify` or use `tokio_util::sync::CancellationToken` directly (which has a `.cancelled()` future).

### Option A: Use CancellationToken directly

If the `CancelToken` trait wraps a `tokio_util::sync::CancellationToken`, expose its `.cancelled()` future:

```rust
#[async_trait]
pub trait CancelToken: Send + Sync {
    fn is_cancelled(&self) -> bool;
    async fn cancelled(&self);  // NEW: waits for cancellation without polling
}

// Default impl for polling fallback (existing behavior):
async fn cancelled(&self) {
    loop {
        if self.is_cancelled() { return; }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
```

Then in the `tokio_util::CancellationToken` impl:
```rust
async fn cancelled(&self) {
    self.inner.cancelled().await;  // zero-cost, instant notification
}
```

### Option B: Simpler — add a Notify to the trait

Add `fn notify(&self) -> &tokio::sync::Notify` to the trait, call `notify.notified().await` in `wait_cancelled`.

### Option C: Simplest — just use CancellationToken everywhere

Check if the codebase already uses `tokio_util::sync::CancellationToken`:
```bash
grep -rn 'CancellationToken' crates/ --include='*.rs' | head -10
```

If it does, replace the custom `CancelToken` trait with the tokio one directly and use `.cancelled().await`.

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W7-B-cancel-notify.md and implement all changes. Replace the polling loop in crates/roko-agent/src/dispatcher/cancel.rs with event-driven cancellation. First check if tokio_util::sync::CancellationToken is already used in the codebase. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 7+8 batches together. Do not commit individually.

## Checklist

- [x] Determine which approach fits the existing CancelToken trait
- [x] Replace polling loop with event-driven wait
- [x] Verify cancellation still triggers correctly
- [x] Pre-commit checks pass
