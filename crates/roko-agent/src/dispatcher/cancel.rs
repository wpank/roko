//! Cancellation threading (§36.45).
//!
//! The dispatcher runs handlers inside a `tokio::select!` between two
//! futures: the timeout-wrapped handler, and this module's
//! [`wait_cancelled`] watcher. When either future resolves first, the
//! other is dropped.
//!
//! [`CancelToken`] is a **sync** trait (no async method; not a tokio
//! primitive), so we can't just `.await` a notification. Instead we poll
//! on a bounded cadence (default 50 ms) — cheap, wake-free, and matches
//! the conductor's own heartbeat granularity.

use std::time::Duration;

use roko_core::tool::CancelToken;

/// Default poll cadence for the cancellation watcher.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Await until `token.is_cancelled()` returns `true`.
///
/// If the token is already tripped, returns on the next scheduler tick
/// (no sleep). Otherwise polls every [`DEFAULT_POLL_INTERVAL`] until the
/// token trips. Never returns a cancel _result_ — it just completes so
/// the caller's `tokio::select!` branch can fire.
pub async fn wait_cancelled(token: &dyn CancelToken) {
    if token.is_cancelled() {
        return;
    }
    loop {
        tokio::time::sleep(DEFAULT_POLL_INTERVAL).await;
        if token.is_cancelled() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::tool::{AtomicCancel, CancelToken};
    use std::sync::Arc;
    use std::time::Instant;

    #[tokio::test]
    async fn wait_cancelled_returns_immediately_if_already_tripped() {
        let token = AtomicCancel::new();
        token.cancel();
        let started = Instant::now();
        wait_cancelled(&token as &dyn CancelToken).await;
        // Should complete in well under one poll interval because the
        // fast-path check doesn't sleep.
        assert!(
            started.elapsed() < Duration::from_millis(30),
            "pre-tripped token should return on fast path, took {:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn wait_cancelled_returns_after_trip() {
        let token = Arc::new(AtomicCancel::new());
        let tripper = token.clone();
        // Trip the token after ~80 ms from a background task.
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(80)).await;
            tripper.cancel();
        });
        let started = Instant::now();
        wait_cancelled(token.as_ref() as &dyn CancelToken).await;
        let elapsed = started.elapsed();
        assert!(
            elapsed >= Duration::from_millis(50),
            "watcher returned too early: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_millis(500),
            "watcher returned too late: {elapsed:?}"
        );
    }
}
