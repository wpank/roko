//! Plan-completion feedback sink — triggers dream consolidation and persists
//! daimon affect state when a plan finishes.
//!
//! ## Backlog items
//!
//! - **#143**: Wire dream consolidation trigger after plan completion.
//! - **#144**: Wire daimon affect persistence on plan completion.
//!
//! Both hooks fire only on [`FeedbackEvent::PlanCompleted`] and run
//! non-blocking: the dream cycle is spawned with a bounded timeout so it
//! never blocks the runner event loop, and the daimon persist is a
//! fire-and-forget blocking write on the tokio thread pool.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use super::{FeedbackEvent, FeedbackSink};

/// Maximum wall-clock time for a background dream consolidation before it
/// is aborted. Dreams are heavyweight (they dispatch an LLM review agent),
/// so the default is generous.
const DREAM_TIMEOUT: Duration = Duration::from_secs(300);

// ---------------------------------------------------------------------------
// Dream consolidation sink (#143)
// ---------------------------------------------------------------------------

/// Spawns a non-blocking dream consolidation cycle on [`PlanCompleted`].
///
/// The sink checks two config flags before firing:
/// - `learning.dream_on_completion` (legacy top-level toggle)
/// - `learning.dreams.trigger_on_plan_complete` (per-subsystem toggle)
///
/// Both must be `true` (the default) for the trigger to fire. A single
/// background dream is allowed at a time; additional triggers are dropped.
#[derive(Debug)]
pub struct DreamConsolidationSink {
    workdir: PathBuf,
    dream_on_completion: bool,
    trigger_on_plan_complete: bool,
    /// Guard: `true` while a dream cycle is already in flight.
    running: Arc<Mutex<bool>>,
}

impl DreamConsolidationSink {
    /// Construct the sink.
    ///
    /// - `workdir`: workspace root (passed to `DreamRunner`).
    /// - `dream_on_completion`: value of `learning.dream_on_completion`.
    /// - `trigger_on_plan_complete`: value of `learning.dreams.trigger_on_plan_complete`.
    #[must_use]
    pub fn new(
        workdir: PathBuf,
        dream_on_completion: bool,
        trigger_on_plan_complete: bool,
    ) -> Self {
        Self {
            workdir,
            dream_on_completion,
            trigger_on_plan_complete,
            running: Arc::new(Mutex::new(false)),
        }
    }
}

#[async_trait]
impl FeedbackSink for DreamConsolidationSink {
    fn name(&self) -> &'static str {
        "dream_consolidation"
    }

    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(event, FeedbackEvent::PlanCompleted { .. })
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        let FeedbackEvent::PlanCompleted { plan_id, .. } = event else {
            return Ok(());
        };

        // Both flags must be true.
        if !self.dream_on_completion || !self.trigger_on_plan_complete {
            tracing::debug!(
                plan_id,
                "dream consolidation skipped: disabled by config"
            );
            return Ok(());
        }

        // Prevent concurrent dreams.
        {
            let mut guard = self.running.lock().unwrap_or_else(|e| e.into_inner());
            if *guard {
                tracing::info!(
                    plan_id,
                    "dream consolidation skipped: already running"
                );
                return Ok(());
            }
            *guard = true;
        }

        let running = Arc::clone(&self.running);
        let workdir = self.workdir.clone();
        let plan_id = plan_id.clone();

        tokio::spawn(async move {
            tracing::info!(plan_id, "starting background dream consolidation");

            let result = tokio::time::timeout(DREAM_TIMEOUT, async {
                // Build a minimal DreamRunner with defaults. The dream cycle
                // reads episodes and knowledge from the workdir layout.
                let dream_config = roko_dreams::DreamLoopConfig {
                    auto_dream: true,
                    idle_threshold_mins: 0,
                    min_episodes_for_dream: 1,
                    schedule: roko_dreams::DreamSchedulePolicy::default(),
                    agent: roko_dreams::DreamAgentConfig {
                        command: "claude".to_string(),
                        args: vec![],
                        model: None,
                        bare_mode: true,
                        effort: "low".to_string(),
                        fallback_model: None,
                        timeout_ms: 120_000,
                        env: vec![],
                    },
                };
                let mut runner = roko_dreams::DreamRunner::new(
                    workdir,
                    dream_config,
                );
                runner.consolidate_async().await
            })
            .await;

            match result {
                Ok(Ok(report)) => {
                    tracing::info!(
                        plan_id,
                        episodes = report.processed_episodes,
                        knowledge = report.knowledge_entries_written,
                        "dream consolidation completed"
                    );
                }
                Ok(Err(err)) => {
                    tracing::warn!(
                        plan_id,
                        error = %err,
                        "dream consolidation failed (non-fatal)"
                    );
                }
                Err(_elapsed) => {
                    tracing::warn!(
                        plan_id,
                        timeout_secs = DREAM_TIMEOUT.as_secs(),
                        "dream consolidation timed out (non-fatal)"
                    );
                }
            }

            // Release the running guard.
            if let Ok(mut guard) = running.lock() {
                *guard = false;
            }
        });

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Daimon affect persistence sink (#144)
// ---------------------------------------------------------------------------

/// Persists the current [`DaimonState`] to disk on [`PlanCompleted`].
///
/// The persist is dispatched to `spawn_blocking` so it never blocks the
/// async event loop. Errors are logged but not propagated — affect
/// persistence is best-effort.
#[derive(Debug)]
pub struct DaimonPersistenceSink {
    affect_path: PathBuf,
    daimon_state: Arc<Mutex<roko_daimon::DaimonState>>,
}

impl DaimonPersistenceSink {
    /// Construct the sink.
    ///
    /// - `affect_path`: path to `.roko/daimon/affect.json`.
    /// - `daimon_state`: shared mutable daimon state used by the runner.
    #[must_use]
    pub fn new(
        affect_path: PathBuf,
        daimon_state: Arc<Mutex<roko_daimon::DaimonState>>,
    ) -> Self {
        Self {
            affect_path,
            daimon_state,
        }
    }
}

#[async_trait]
impl FeedbackSink for DaimonPersistenceSink {
    fn name(&self) -> &'static str {
        "daimon_persistence"
    }

    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(event, FeedbackEvent::PlanCompleted { .. })
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        let FeedbackEvent::PlanCompleted { plan_id, .. } = event else {
            return Ok(());
        };

        // Snapshot the state under the lock, then persist outside the lock.
        let snapshot = {
            let guard = self
                .daimon_state
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            guard.clone()
        };

        let path = self.affect_path.clone();
        let plan_id = plan_id.clone();

        tokio::task::spawn_blocking(move || {
            use roko_daimon::AffectEngine;
            match snapshot.persist(&path) {
                Ok(()) => {
                    tracing::info!(plan_id, path = %path.display(), "daimon affect state persisted");
                }
                Err(err) => {
                    tracing::warn!(
                        plan_id,
                        path = %path.display(),
                        error = %err,
                        "failed to persist daimon affect state (non-fatal)"
                    );
                }
            }
        })
        .await
        .map_err(|e| anyhow::anyhow!("daimon persist join error: {e}"))?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dream_sink_skips_non_plan_events() {
        let sink = DreamConsolidationSink::new(
            PathBuf::from("/tmp/test"),
            true,
            true,
        );
        let event = FeedbackEvent::IdleTick {
            ticks_since_last_work: 1,
        };
        assert!(!sink.interested(&event));
    }

    #[tokio::test]
    async fn dream_sink_skips_when_disabled() {
        let sink = DreamConsolidationSink::new(
            PathBuf::from("/tmp/test"),
            false, // dream_on_completion disabled
            true,
        );
        let event = FeedbackEvent::PlanCompleted {
            plan_id: "p".into(),
            succeeded: true,
            tasks_completed: 1,
            tasks_failed: 0,
            total_cost_usd: 0.001,
        };
        // Should return Ok without spawning.
        sink.on_event(&event).await.unwrap();
    }

    #[tokio::test]
    async fn daimon_sink_skips_non_plan_events() {
        let state = Arc::new(Mutex::new(roko_daimon::DaimonState::new()));
        let sink = DaimonPersistenceSink::new(
            PathBuf::from("/tmp/test/affect.json"),
            state,
        );
        let event = FeedbackEvent::IdleTick {
            ticks_since_last_work: 1,
        };
        assert!(!sink.interested(&event));
    }

    #[tokio::test]
    async fn daimon_sink_persists_on_plan_completed() {
        let tmp = tempfile::TempDir::new().unwrap();
        let affect_path = tmp.path().join("daimon").join("affect.json");
        let state = Arc::new(Mutex::new(roko_daimon::DaimonState::new()));

        let sink = DaimonPersistenceSink::new(affect_path.clone(), state);
        let event = FeedbackEvent::PlanCompleted {
            plan_id: "p".into(),
            succeeded: true,
            tasks_completed: 1,
            tasks_failed: 0,
            total_cost_usd: 0.001,
        };
        sink.on_event(&event).await.unwrap();

        assert!(affect_path.exists(), "affect.json should be created");
        let contents = std::fs::read_to_string(&affect_path).unwrap();
        assert!(
            contents.contains("half_life_hours"),
            "should contain DaimonState fields"
        );
    }
}
