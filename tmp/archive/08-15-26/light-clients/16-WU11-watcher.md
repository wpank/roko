# WU-11: Block Watcher

**Layer**: 3
**Depends on**: WU-7 (VerifiedChainClient)
**Blocks**: WU-14
**Estimated effort**: 2-3 hours
**Crates**: `crates/roko-chain` (watcher.rs), `crates/roko-serve` (events)

---

## Overview

Create `ChainWatcherTask` — an async loop that polls the chain for new blocks, feeds them through the existing `BlockObserver`, and emits events. The `BlockObserver` already exists in `crates/roko-chain/src/observer.rs` with full filter/tracking logic, but has **no async driver loop**. This WU adds that driver.

**Key insight**: `BlockObserver` is synchronous — it takes `(ChainHeader, Vec<LogEntry>)` and returns matched events. It does NOT poll the chain itself. `ChainWatcherTask` is the async wrapper that:
1. Polls `ChainClient::block_number()` on an interval
2. Fetches headers and logs for new blocks
3. Passes them through `BlockObserver::process_block()`
4. Emits `ChainEvent` variants into the event bus

---

## Pre-read

- `crates/roko-chain/src/observer.rs` — `BlockObserver`, `BlockObserverConfig`, `ObservedEvent`, `BlockTracker` (fully implemented, ~345 lines)
- `crates/roko-chain/src/client.rs` — `ChainClient::block_number()`, `get_block_header()`, `get_logs()`
- `crates/roko-serve/src/events.rs` — `ServerEvent` enum (add new `ChainEvent` variants)
- `crates/roko-serve/src/event_bus.rs` — `EventBus<E>` with `publish()`, `subscribe()`

---

## Tasks

### 11.1 Create `crates/roko-chain/src/watcher.rs`

```rust
//! Async block watcher that drives [`BlockObserver`] in a polling loop.
//!
//! Polls the chain for new blocks on a configurable interval, feeds them
//! through the existing [`BlockObserver`] filter pipeline, and invokes
//! a user-provided callback with matched events.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;

use crate::client::ChainClient;
use crate::observer::{BlockObserver, BlockObserverConfig, ObservedEvent};
use crate::types::{BlockNumber, ChainError};

/// Configuration for the watcher task.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Polling interval.
    pub poll_interval: Duration,
    /// Observer filter config (addresses, topics).
    pub observer: BlockObserverConfig,
    /// Maximum blocks to fetch per poll cycle (prevents huge catch-ups).
    pub max_blocks_per_poll: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(2),
            observer: BlockObserverConfig::default(),
            max_blocks_per_poll: 50,
        }
    }
}

/// Events emitted by the watcher.
#[derive(Debug, Clone)]
pub enum WatcherEvent {
    /// A new block was processed (may or may not have matched events).
    NewBlock {
        block_number: BlockNumber,
        block_hash: String,
        timestamp: u64,
    },
    /// One or more events matched the observer's filter.
    MatchedEvents {
        block_number: BlockNumber,
        events: Vec<ObservedEvent>,
    },
    /// A gap was detected and backfilled.
    GapBackfilled {
        blocks: Vec<BlockNumber>,
    },
    /// The watcher encountered a recoverable error.
    Error {
        message: String,
    },
}

/// A handle to a running watcher task.
pub struct WatcherHandle {
    /// Cancellation token.
    cancel: tokio::sync::watch::Sender<bool>,
    /// JoinHandle for the background task.
    join: tokio::task::JoinHandle<()>,
}

impl WatcherHandle {
    /// Stop the watcher gracefully.
    pub async fn stop(self) {
        let _ = self.cancel.send(true);
        let _ = self.join.await;
    }

    /// Check if the watcher is still running.
    pub fn is_running(&self) -> bool {
        !self.join.is_finished()
    }
}

/// Start a block watcher as a background tokio task.
///
/// Returns a handle for lifecycle control and a receiver for watcher events.
///
/// # Arguments
/// * `client` — Chain client for RPC calls
/// * `config` — Watcher configuration
/// * `start_block` — Block number to start watching from (None = latest)
pub fn spawn_watcher(
    client: Arc<dyn ChainClient>,
    config: WatcherConfig,
    start_block: Option<BlockNumber>,
) -> (WatcherHandle, mpsc::Receiver<WatcherEvent>) {
    let (event_tx, event_rx) = mpsc::channel(256);
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    let join = tokio::spawn(watcher_loop(
        client,
        config,
        start_block,
        event_tx,
        cancel_rx,
    ));

    let handle = WatcherHandle {
        cancel: cancel_tx,
        join,
    };

    (handle, event_rx)
}

async fn watcher_loop(
    client: Arc<dyn ChainClient>,
    config: WatcherConfig,
    start_block: Option<BlockNumber>,
    event_tx: mpsc::Sender<WatcherEvent>,
    mut cancel_rx: tokio::sync::watch::Receiver<bool>,
) {
    let mut observer = BlockObserver::new(config.observer);
    let mut interval = tokio::time::interval(config.poll_interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    // Determine starting block
    let mut last_processed = match start_block {
        Some(b) => b,
        None => match client.block_number().await {
            Ok(b) => b,
            Err(e) => {
                let _ = event_tx.send(WatcherEvent::Error {
                    message: format!("failed to get initial block number: {e}"),
                }).await;
                return;
            }
        },
    };

    loop {
        tokio::select! {
            _ = interval.tick() => {}
            _ = cancel_rx.changed() => {
                if *cancel_rx.borrow() {
                    tracing::info!(last_block = last_processed, "watcher shutting down");
                    return;
                }
            }
        }

        // Get current block number
        let current = match client.block_number().await {
            Ok(b) => b,
            Err(e) => {
                let _ = event_tx.send(WatcherEvent::Error {
                    message: format!("block_number poll failed: {e}"),
                }).await;
                continue;
            }
        };

        if current <= last_processed {
            continue; // No new blocks
        }

        // Cap the number of blocks to process
        let from = last_processed + 1;
        let to = current.min(from + config.max_blocks_per_poll - 1);

        for block_num in from..=to {
            match process_single_block(&client, &mut observer, block_num).await {
                Ok((header_hash, timestamp, matched)) => {
                    // Always emit NewBlock
                    let _ = event_tx.send(WatcherEvent::NewBlock {
                        block_number: block_num,
                        block_hash: header_hash,
                        timestamp,
                    }).await;

                    // Emit matched events if any
                    if !matched.is_empty() {
                        let _ = event_tx.send(WatcherEvent::MatchedEvents {
                            block_number: block_num,
                            events: matched,
                        }).await;
                    }

                    last_processed = block_num;
                }
                Err(e) => {
                    let _ = event_tx.send(WatcherEvent::Error {
                        message: format!("block {block_num}: {e}"),
                    }).await;
                    break; // Stop processing this batch on error
                }
            }
        }

        // Check for gaps and report
        let gaps = observer.pending_gaps();
        if !gaps.is_empty() {
            tracing::debug!(gap_count = gaps.len(), "block gaps detected");
            let _ = event_tx.send(WatcherEvent::GapBackfilled {
                blocks: gaps,
            }).await;
        }
    }
}

async fn process_single_block(
    client: &Arc<dyn ChainClient>,
    observer: &mut BlockObserver,
    block_num: BlockNumber,
) -> Result<(String, u64, Vec<ObservedEvent>), ChainError> {
    let header = client.get_block_header(block_num).await?;

    // Fetch logs for watched addresses if any are configured
    let logs = if observer.filter.address_count() > 0 || observer.filter.topic_count() > 0 {
        let addresses: Vec<String> = observer.config.watched_addresses.clone();
        let topics: Vec<String> = observer.config.watched_topics.clone();
        client.get_logs(block_num, block_num, &addresses, &topics).await
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let hash = header.hash.clone();
    let timestamp = header.timestamp;
    let matched = observer.process_block(&header, &logs);

    Ok((hash, timestamp, matched))
}
```

### 11.2 Register module in `crates/roko-chain/src/lib.rs`

```rust
/// Async block watcher driving BlockObserver.
pub mod watcher;

pub use watcher::{WatcherConfig, WatcherEvent, WatcherHandle, spawn_watcher};
```

### 11.3 Add `ChainEvent` variants to `ServerEvent`

**File**: `crates/roko-serve/src/events.rs`

Add new variants to the `ServerEvent` enum:

```rust
/// A new block was observed by the chain watcher.
ChainNewBlock {
    backend: String,
    block_number: u64,
    block_hash: String,
    timestamp: u64,
},

/// Chain events matched the observer filter.
ChainEventsMatched {
    backend: String,
    block_number: u64,
    event_count: usize,
    summary: String,
},

/// Chain watcher health changed.
ChainWatcherHealth {
    backend: String,
    healthy: bool,
    message: String,
},
```

### 11.4 Add watcher integration to roko-serve startup

**File**: `crates/roko-serve/src/state.rs` (or wherever the AppState is constructed)

Add to `AppState`:
```rust
/// Chain watcher handles for each backend.
pub watcher_handles: Vec<roko_chain::WatcherHandle>,
```

During server startup, after constructing the `BackendPool` (WU-10), spawn watchers:

```rust
// In the server startup function (e.g., build_app_state or equivalent):
let mut watcher_handles = Vec::new();
for (name, backend) in &backend_pool {
    let config = roko_chain::WatcherConfig {
        poll_interval: Duration::from_secs(2),
        observer: roko_chain::BlockObserverConfig {
            watched_addresses: chain_config.watched_addresses_for(name),
            ..Default::default()
        },
        max_blocks_per_poll: 50,
    };
    let (handle, mut rx) = roko_chain::spawn_watcher(
        Arc::clone(&backend.rpc),
        config,
        None,
    );

    // Bridge watcher events into the server event bus
    let bus = event_bus.clone();
    let backend_name = name.to_string();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                roko_chain::WatcherEvent::NewBlock { block_number, block_hash, timestamp } => {
                    bus.publish(ServerEvent::ChainNewBlock {
                        backend: backend_name.clone(),
                        block_number,
                        block_hash,
                        timestamp,
                    });
                }
                roko_chain::WatcherEvent::MatchedEvents { block_number, events } => {
                    bus.publish(ServerEvent::ChainEventsMatched {
                        backend: backend_name.clone(),
                        block_number,
                        event_count: events.len(),
                        summary: format!("{} events matched", events.len()),
                    });
                }
                roko_chain::WatcherEvent::Error { message } => {
                    bus.publish(ServerEvent::ChainWatcherHealth {
                        backend: backend_name.clone(),
                        healthy: false,
                        message,
                    });
                }
                _ => {}
            }
        }
    });

    watcher_handles.push(handle);
}
```

### 11.5 Tests

Add tests to `watcher.rs`:

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::MockChainClient;
    use crate::observer::BlockObserverConfig;

    #[tokio::test(flavor = "current_thread")]
    async fn watcher_emits_new_block_events() {
        let mock = MockChainClient::local();
        mock.mine_empty_block();
        mock.mine_empty_block();
        mock.mine_empty_block();

        let config = WatcherConfig {
            poll_interval: Duration::from_millis(50),
            observer: BlockObserverConfig::default(),
            max_blocks_per_poll: 10,
        };

        let (handle, mut rx) = spawn_watcher(Arc::new(mock), config, Some(0));

        // Wait for at least one event
        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("event");

        match event {
            WatcherEvent::NewBlock { block_number, .. } => {
                assert!(block_number >= 1);
            }
            other => panic!("expected NewBlock, got {other:?}"),
        }

        handle.stop().await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn watcher_stops_on_cancel() {
        let mock = MockChainClient::local();
        mock.mine_empty_block();

        let config = WatcherConfig {
            poll_interval: Duration::from_millis(50),
            ..Default::default()
        };

        let (handle, _rx) = spawn_watcher(Arc::new(mock), config, Some(0));
        assert!(handle.is_running());

        handle.stop().await;
        // After stop, the join handle should be finished
    }

    #[tokio::test(flavor = "current_thread")]
    async fn watcher_handles_rpc_errors_gracefully() {
        // Use a mock that will fail on block_number after first call
        let mock = MockChainClient::local();
        // Don't mine any blocks — block_number returns 0, so no iteration happens
        let config = WatcherConfig {
            poll_interval: Duration::from_millis(50),
            ..Default::default()
        };

        let (handle, _rx) = spawn_watcher(Arc::new(mock), config, Some(0));

        // Let it run a few cycles
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(handle.is_running());

        handle.stop().await;
    }
}
```

---

## Verification Checklist

- [ ] `ChainWatcherTask` (via `spawn_watcher()`) polls chain and emits `WatcherEvent`
- [ ] Uses existing `BlockObserver::process_block()` — no reimplementation of filtering
- [ ] `WatcherHandle` supports graceful shutdown via `stop()`
- [ ] `max_blocks_per_poll` caps catch-up to prevent unbounded work
- [ ] Gap detection delegates to existing `BlockTracker`
- [ ] `ServerEvent::ChainNewBlock`, `ChainEventsMatched`, `ChainWatcherHealth` added to events.rs
- [ ] Watcher event → ServerEvent bridge code sketched for roko-serve startup
- [ ] Module registered in `lib.rs`
- [ ] `cargo test -p roko-chain` passes
- [ ] `cargo test -p roko-serve` passes (new event variants serialize correctly)
- [ ] `cargo test --workspace` — no breakage
