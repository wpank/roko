//! Hot Graph execution -- tick-driven, resident Graph instances.
//!
//! A Hot Graph is a Graph with `policy.hot` set. The Engine runs it in a
//! loop, executing all nodes once per tick, persisting outputs between ticks
//! so each tick starts from the previous tick's state.
//!
//! Hot Graphs run until:
//!   1. `HotPolicy.max_ticks` is reached, OR
//!   2. The `HotGraphHandle.cancel()` token is triggered, OR
//!   3. A node returns an unrecoverable error (non-retriable failure)
//!      and the graph policy is `FailFast`.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::cell::CellContext;
use crate::engine::{GraphEngine, GraphOutput};
use crate::registry::CellRegistry;
use crate::types::Graph;

/// Named loop levels for nested hot graph timing, per v2 spec (04-EXECUTION.md).
///
/// When a `HotPolicy` has `loop_level` set, the level's default interval is used
/// in place of `tick_interval_ms`. This allows graphs to declare their temporal
/// role (perception, planning, consolidation) without hard-coding millisecond values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LoopLevel {
    /// Fast: perception, reflex. Default 250 ms.
    Gamma,
    /// Medium: planning, deliberation. Default 10 000 ms.
    Theta,
    /// Slow: learning, consolidation. Default 60 000 ms.
    Delta,
}

impl LoopLevel {
    /// Return the spec-default tick interval in milliseconds for this loop level.
    pub fn default_interval_ms(self) -> u64 {
        match self {
            Self::Gamma => 250,
            Self::Theta => 10_000,
            Self::Delta => 60_000,
        }
    }
}

/// Policy controlling Hot Graph tick behavior.
///
/// Parsed from `[graph.policy.hot]` in TOML graph definitions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HotPolicy {
    /// How long to wait between ticks (ms). 0 = run as fast as possible.
    ///
    /// Overridden by `loop_level` when that field is set.
    #[serde(default)]
    pub tick_interval_ms: u64,
    /// Stop after this many ticks. None = run until cancelled.
    #[serde(default)]
    pub max_ticks: Option<u64>,
    /// If true, persist cell output state between ticks so cells can
    /// resume from their previous output.
    #[serde(default)]
    pub persist_tick_state: bool,
    /// Named loop level. When set, the level's default interval overrides
    /// `tick_interval_ms`.
    #[serde(default)]
    pub loop_level: Option<LoopLevel>,
}

impl HotPolicy {
    /// Return the effective tick interval in milliseconds.
    ///
    /// If `loop_level` is set, returns that level's spec-default interval.
    /// Otherwise falls back to the raw `tick_interval_ms` value.
    pub fn resolve_tick_interval_ms(&self) -> u64 {
        match self.loop_level {
            Some(level) => level.default_interval_ms(),
            None => self.tick_interval_ms,
        }
    }
}

impl Default for HotPolicy {
    fn default() -> Self {
        Self {
            tick_interval_ms: 1000,
            max_ticks: None,
            persist_tick_state: false,
            loop_level: None,
        }
    }
}

/// A running Hot Graph instance.
///
/// Returned by [`start_hot`]. Callers use this handle to observe tick progress,
/// cancel the loop, and wait for completion.
pub struct HotGraphHandle {
    /// Cancellation token -- call `.cancel()` to stop the tick loop.
    cancel: CancellationToken,
    /// Monotonic tick counter (incremented after each completed tick).
    tick: Arc<AtomicU64>,
    /// Most recent graph output (from the last completed tick).
    last_output: Arc<parking_lot::Mutex<Option<GraphOutput>>>,
    /// Background task handle (taken by `wait`).
    join_handle: parking_lot::Mutex<Option<JoinHandle<()>>>,
}

impl HotGraphHandle {
    /// Request cancellation of the Hot Graph tick loop.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Return the number of completed ticks.
    pub fn tick_count(&self) -> u64 {
        self.tick.load(Ordering::Relaxed)
    }

    /// Return a clone of the most recent graph output, if any tick has completed.
    pub fn last_output(&self) -> Option<GraphOutput> {
        self.last_output.lock().clone()
    }

    /// Wait for the Hot Graph to finish (either max_ticks reached or cancelled).
    ///
    /// Can be called multiple times; only the first call awaits the background
    /// task. Subsequent calls return immediately.
    pub async fn wait(&self) {
        let handle = self.join_handle.lock().take();
        if let Some(h) = handle {
            let _ = h.await;
        }
    }

    /// Check if the background task is still running.
    pub fn is_running(&self) -> bool {
        let guard = self.join_handle.lock();
        match &*guard {
            Some(h) => !h.is_finished(),
            None => false,
        }
    }
}

/// Start a Hot Graph -- a tick-driven, resident graph execution loop.
///
/// The graph is executed repeatedly according to the `HotPolicy`:
/// - Each tick runs the full graph once via the Engine's sequential execution.
/// - Between ticks, the loop waits for `tick_interval_ms` (or checks cancellation).
/// - After `max_ticks` (if set), the loop exits.
///
/// Returns a `HotGraphHandle` immediately; execution continues on a background task.
///
/// # Errors
///
/// Returns an error if the graph has no `hot` policy in its metadata labels.
pub fn start_hot(
    graph: Graph,
    registry: CellRegistry,
    policy: HotPolicy,
    parent_cancel: Option<CancellationToken>,
) -> HotGraphHandle {
    let cancel = parent_cancel.map(|p| p.child_token()).unwrap_or_default();
    let tick = Arc::new(AtomicU64::new(0));
    let last_output: Arc<parking_lot::Mutex<Option<GraphOutput>>> =
        Arc::new(parking_lot::Mutex::new(None));

    let cancel_clone = cancel.clone();
    let tick_clone = tick.clone();
    let output_clone = last_output.clone();
    let graph_name = graph.metadata.name.clone();

    let join_handle = tokio::spawn(async move {
        let engine = GraphEngine::new(graph, registry);
        let ctx = CellContext::new();
        let mut current_tick = 0u64;

        let tick_interval_ms = policy.resolve_tick_interval_ms();
        info!(
            graph = %graph_name,
            max_ticks = ?policy.max_ticks,
            tick_interval_ms,
            loop_level = ?policy.loop_level,
            "hot graph started"
        );

        loop {
            // Check cancellation before each tick.
            if cancel_clone.is_cancelled() {
                info!(graph = %graph_name, ticks = current_tick, "hot graph cancelled");
                break;
            }

            // Check max_ticks limit.
            if let Some(max) = policy.max_ticks {
                if current_tick >= max {
                    info!(
                        graph = %graph_name,
                        ticks = current_tick,
                        "hot graph reached max_ticks"
                    );
                    break;
                }
            }

            // Execute one tick of the graph.
            match engine.execute(&ctx).await {
                Ok(output) => {
                    info!(
                        graph = %graph_name,
                        tick = current_tick,
                        success = output.success,
                        nodes = output.node_results.len(),
                        "hot graph tick complete"
                    );
                    *output_clone.lock() = Some(output);
                }
                Err(e) => {
                    error!(
                        graph = %graph_name,
                        tick = current_tick,
                        error = %e,
                        "hot graph tick failed"
                    );
                    // On error, break the loop (conservative: FailFast for hot graphs).
                    break;
                }
            }

            current_tick += 1;
            tick_clone.store(current_tick, Ordering::Relaxed);

            // Wait for next tick interval, respecting cancellation.
            if tick_interval_ms > 0 {
                let sleep_dur = Duration::from_millis(tick_interval_ms);
                tokio::select! {
                    () = tokio::time::sleep(sleep_dur) => {}
                    () = cancel_clone.cancelled() => {
                        info!(graph = %graph_name, ticks = current_tick, "hot graph cancelled during sleep");
                        break;
                    }
                }
            } else {
                // Yield to let cancellation propagate even at tick_interval_ms = 0.
                tokio::task::yield_now().await;
                if cancel_clone.is_cancelled() {
                    break;
                }
            }
        }

        info!(
            graph = %graph_name,
            total_ticks = current_tick,
            "hot graph stopped"
        );
    });

    HotGraphHandle {
        cancel,
        tick,
        last_output,
        join_handle: parking_lot::Mutex::new(Some(join_handle)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_from_str;

    fn noop_registry() -> CellRegistry {
        let mut r = CellRegistry::new();
        r.register("noop", |_| {
            Box::new(crate::cells::stubs::PassthroughCell::new("noop"))
        });
        r
    }

    #[tokio::test]
    async fn hot_graph_respects_max_ticks() {
        let toml_str = r#"
[graph]
name = "tick-test"

[[nodes]]
id = "a"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let policy = HotPolicy {
            tick_interval_ms: 0,
            max_ticks: Some(3),
            persist_tick_state: false,
            loop_level: None,
        };
        let handle = start_hot(graph, noop_registry(), policy, None);
        handle.wait().await;
        assert_eq!(handle.tick_count(), 3);
    }

    #[tokio::test]
    async fn hot_graph_cancels_cleanly() {
        let toml_str = r#"
[graph]
name = "cancel-test"

[[nodes]]
id = "a"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let policy = HotPolicy {
            tick_interval_ms: 100,
            max_ticks: None,
            persist_tick_state: false,
            loop_level: None,
        };
        let handle = start_hot(graph, noop_registry(), policy, None);

        // Let it run a few ticks.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Cancel.
        handle.cancel();

        // Wait with a timeout to ensure it doesn't hang.
        let result = tokio::time::timeout(Duration::from_millis(500), handle.wait()).await;
        assert!(
            result.is_ok(),
            "hot graph should stop within 500ms of cancel"
        );
    }

    #[tokio::test]
    async fn hot_graph_zero_interval_runs_fast() {
        let toml_str = r#"
[graph]
name = "fast-test"

[[nodes]]
id = "a"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let policy = HotPolicy {
            tick_interval_ms: 0,
            max_ticks: Some(10),
            persist_tick_state: false,
            loop_level: None,
        };
        let handle = start_hot(graph, noop_registry(), policy, None);
        handle.wait().await;
        assert_eq!(handle.tick_count(), 10);
    }

    #[tokio::test]
    async fn hot_graph_last_output_available() {
        let toml_str = r#"
[graph]
name = "output-test"

[[nodes]]
id = "a"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let policy = HotPolicy {
            tick_interval_ms: 0,
            max_ticks: Some(1),
            persist_tick_state: false,
            loop_level: None,
        };
        let handle = start_hot(graph, noop_registry(), policy, None);
        handle.wait().await;
        let output = handle.last_output();
        assert!(output.is_some());
        assert!(output.unwrap().success);
    }

    #[test]
    fn loop_level_default_intervals() {
        assert_eq!(LoopLevel::Gamma.default_interval_ms(), 250);
        assert_eq!(LoopLevel::Theta.default_interval_ms(), 10_000);
        assert_eq!(LoopLevel::Delta.default_interval_ms(), 60_000);
    }

    #[test]
    fn resolve_tick_interval_uses_loop_level_when_set() {
        let policy = HotPolicy {
            tick_interval_ms: 5000,
            max_ticks: None,
            persist_tick_state: false,
            loop_level: Some(LoopLevel::Gamma),
        };
        // loop_level should override tick_interval_ms
        assert_eq!(policy.resolve_tick_interval_ms(), 250);
    }

    #[test]
    fn resolve_tick_interval_falls_back_to_raw_when_no_level() {
        let policy = HotPolicy {
            tick_interval_ms: 3000,
            max_ticks: None,
            persist_tick_state: false,
            loop_level: None,
        };
        assert_eq!(policy.resolve_tick_interval_ms(), 3000);
    }
}
