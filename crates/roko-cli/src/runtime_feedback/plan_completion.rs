//! Plan-completion feedback sinks — dream consolidation, daimon affect
//! persistence, theta reflection, and delta consolidation.
//!
//! ## Backlog items
//!
//! - **#143**: Wire dream consolidation trigger after plan completion.
//! - **#144**: Wire daimon affect persistence on plan completion.
//!
//! All hooks fire only on [`FeedbackEvent::PlanCompleted`] and run
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
            tracing::debug!(plan_id, "dream consolidation skipped: disabled by config");
            return Ok(());
        }

        // Prevent concurrent dreams.
        {
            let mut guard = self.running.lock().unwrap_or_else(|e| e.into_inner());
            if *guard {
                tracing::info!(plan_id, "dream consolidation skipped: already running");
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
                let mut runner = roko_dreams::DreamRunner::new(workdir, dream_config);
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
    pub fn new(affect_path: PathBuf, daimon_state: Arc<Mutex<roko_daimon::DaimonState>>) -> Self {
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
            let guard = self.daimon_state.lock().unwrap_or_else(|e| e.into_inner());
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
// Theta reflection sink
// ---------------------------------------------------------------------------

/// Runs a theta reflective cycle on [`PlanCompleted`], summarising recent
/// work and updating affect/calibration/progress signals.
///
/// The theta consumer is stateful (it buffers gamma records), so it is
/// shared behind an `Arc<Mutex<_>>` and passed to the sink at construction.
/// The tick is synchronous and lightweight (no LLM calls), so it runs
/// inline on `spawn_blocking` rather than spawning a long background task.
pub struct ThetaReflectionSink {
    theta: Arc<Mutex<roko_runtime::theta_consumer::ThetaConsumer>>,
    cortical: Arc<roko_runtime::heartbeat::CorticalState>,
}

impl std::fmt::Debug for ThetaReflectionSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThetaReflectionSink")
            .field("theta", &"<ThetaConsumer>")
            .field("cortical", &"<CorticalState>")
            .finish()
    }
}

impl ThetaReflectionSink {
    /// Construct the sink.
    ///
    /// - `theta`: shared theta consumer that accumulates gamma records.
    /// - `cortical`: shared cortical state for affect reads/writes.
    #[must_use]
    pub fn new(
        theta: Arc<Mutex<roko_runtime::theta_consumer::ThetaConsumer>>,
        cortical: Arc<roko_runtime::heartbeat::CorticalState>,
    ) -> Self {
        Self { theta, cortical }
    }
}

#[async_trait]
impl FeedbackSink for ThetaReflectionSink {
    fn name(&self) -> &'static str {
        "theta_reflection"
    }

    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(event, FeedbackEvent::PlanCompleted { .. })
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        let FeedbackEvent::PlanCompleted {
            plan_id,
            tasks_completed,
            tasks_failed,
            ..
        } = event
        else {
            return Ok(());
        };

        let theta = Arc::clone(&self.theta);
        let cortical = Arc::clone(&self.cortical);
        let plan_id = plan_id.clone();
        let total_tasks = tasks_completed + tasks_failed;
        let completed = *tasks_completed;

        tokio::task::spawn_blocking(move || {
            let mut guard = theta.lock().unwrap_or_else(|e| e.into_inner());
            let ctx = roko_runtime::theta_consumer::ThetaContext {
                cortical: &cortical,
                current_task_id: None,
                total_tasks,
                completed_tasks: completed,
            };
            let outcome = guard.tick(&ctx);

            if outcome.plan_progress.stalled {
                tracing::warn!(
                    plan_id,
                    consecutive_failures = outcome.plan_progress.consecutive_failures,
                    "theta reflection: plan appears stalled"
                );
            }
            if outcome.calibration.drift_detected {
                tracing::info!(
                    plan_id,
                    accuracy = outcome.calibration.current_accuracy,
                    drifting = ?outcome.calibration.drifting_categories,
                    "theta reflection: calibration drift detected"
                );
            }
            if outcome.affect_update.significant_shift {
                tracing::info!(
                    plan_id,
                    pleasure_before = outcome.affect_update.pad_before.pleasure,
                    pleasure_after = outcome.affect_update.pad_after.pleasure,
                    "theta reflection: significant affect shift"
                );
            }
            if !outcome.meta_cognition.issues.is_empty() {
                tracing::info!(
                    plan_id,
                    issues = outcome.meta_cognition.issues.len(),
                    "theta reflection: meta-cognition issues detected"
                );
            }
            tracing::debug!(
                plan_id,
                gamma_ticks = outcome.gamma_summary.tick_count,
                success_rate = outcome.gamma_summary.success_rate,
                "theta reflection cycle completed"
            );
        })
        .await
        .map_err(|e| anyhow::anyhow!("theta reflection join error: {e}"))?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Delta consolidation consumer sink
// ---------------------------------------------------------------------------

/// Checks whether the [`DeltaConsumer`] trigger conditions are met on
/// [`PlanCompleted`], and if so runs a synchronous delta dream cycle.
///
/// This bridges `roko_runtime::delta_consumer` into the feedback pipeline.
/// The delta consumer tracks episode counts and idle durations internally;
/// the sink records each plan completion as an episode and checks the
/// trigger on every event. When a cycle fires, the three stub phases
/// (NREM/REM/integration) produce diagnostic telemetry that will be
/// connected to `roko-dreams` phases as they mature.
pub struct DeltaConsolidationSink {
    delta: Arc<Mutex<roko_runtime::delta_consumer::DeltaConsumer>>,
    cortical: Arc<roko_runtime::heartbeat::CorticalState>,
}

impl std::fmt::Debug for DeltaConsolidationSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeltaConsolidationSink")
            .field("delta", &"<DeltaConsumer>")
            .field("cortical", &"<CorticalState>")
            .finish()
    }
}

impl DeltaConsolidationSink {
    /// Construct the sink.
    ///
    /// - `delta`: shared delta consumer that tracks episode counts/idle timers.
    /// - `cortical`: shared cortical state for regime and arousal checks.
    #[must_use]
    pub fn new(
        delta: Arc<Mutex<roko_runtime::delta_consumer::DeltaConsumer>>,
        cortical: Arc<roko_runtime::heartbeat::CorticalState>,
    ) -> Self {
        Self { delta, cortical }
    }
}

#[async_trait]
impl FeedbackSink for DeltaConsolidationSink {
    fn name(&self) -> &'static str {
        "delta_consolidation"
    }

    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(event, FeedbackEvent::PlanCompleted { .. })
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        let FeedbackEvent::PlanCompleted { plan_id, .. } = event else {
            return Ok(());
        };

        let delta = Arc::clone(&self.delta);
        let cortical = Arc::clone(&self.cortical);
        let plan_id = plan_id.clone();

        tokio::task::spawn_blocking(move || {
            let mut guard = delta.lock().unwrap_or_else(|e| e.into_inner());

            // Record the plan completion as an episode for threshold tracking.
            guard.record_episode();

            // Check trigger conditions.
            let trigger = match guard.should_trigger() {
                Some(t) => t,
                None => {
                    tracing::debug!(
                        plan_id,
                        episodes_since = guard.config().episode_threshold,
                        "delta consolidation: trigger conditions not met"
                    );
                    return;
                }
            };

            // Only run in low-activity states.
            if !roko_runtime::delta_consumer::DeltaConsumer::is_low_activity(&cortical) {
                tracing::debug!(
                    plan_id,
                    "delta consolidation: skipped (high-activity regime)"
                );
                return;
            }

            tracing::info!(plan_id, trigger = ?trigger, "starting delta consolidation cycle");
            let report = guard.run_cycle(trigger, &cortical);
            tracing::info!(
                plan_id,
                cycle = guard.cycle_count(),
                episodes_replayed = report.nrem.episodes_replayed,
                patterns = report.nrem.patterns_extracted,
                counterfactuals = report.rem.counterfactuals_generated,
                promoted = report.integration.entries_promoted,
                "delta consolidation cycle completed"
            );
        })
        .await
        .map_err(|e| anyhow::anyhow!("delta consolidation join error: {e}"))?;

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
        let sink = DreamConsolidationSink::new(PathBuf::from("/tmp/test"), true, true);
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
        let sink = DaimonPersistenceSink::new(PathBuf::from("/tmp/test/affect.json"), state);
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

    #[tokio::test]
    async fn theta_sink_skips_non_plan_events() {
        let theta = Arc::new(Mutex::new(
            roko_runtime::theta_consumer::ThetaConsumer::default(),
        ));
        let cortical = Arc::new(roko_runtime::heartbeat::CorticalState::default());
        let sink = ThetaReflectionSink::new(theta, cortical);
        let event = FeedbackEvent::IdleTick {
            ticks_since_last_work: 1,
        };
        assert!(!sink.interested(&event));
    }

    #[tokio::test]
    async fn theta_sink_runs_on_plan_completed() {
        let theta = Arc::new(Mutex::new(
            roko_runtime::theta_consumer::ThetaConsumer::default(),
        ));
        let cortical = Arc::new(roko_runtime::heartbeat::CorticalState::default());
        let sink = ThetaReflectionSink::new(theta, cortical);
        let event = FeedbackEvent::PlanCompleted {
            plan_id: "p".into(),
            succeeded: true,
            tasks_completed: 5,
            tasks_failed: 1,
            total_cost_usd: 0.5,
        };
        assert!(sink.interested(&event));
        sink.on_event(&event).await.unwrap();
    }

    #[tokio::test]
    async fn delta_sink_skips_non_plan_events() {
        let delta = Arc::new(Mutex::new(
            roko_runtime::delta_consumer::DeltaConsumer::default(),
        ));
        let cortical = Arc::new(roko_runtime::heartbeat::CorticalState::default());
        let sink = DeltaConsolidationSink::new(delta, cortical);
        let event = FeedbackEvent::IdleTick {
            ticks_since_last_work: 1,
        };
        assert!(!sink.interested(&event));
    }

    #[tokio::test]
    async fn delta_sink_records_episode_on_plan_completed() {
        let delta = Arc::new(Mutex::new(
            roko_runtime::delta_consumer::DeltaConsumer::new(
                roko_runtime::delta_consumer::DeltaConfig {
                    // Set a high threshold so we do not trigger a full cycle.
                    episode_threshold: 100,
                    idle_timeout_secs: 9999,
                    ..roko_runtime::delta_consumer::DeltaConfig::default()
                },
            ),
        ));
        let cortical = Arc::new(roko_runtime::heartbeat::CorticalState::default());
        let sink = DeltaConsolidationSink::new(delta, cortical);
        let event = FeedbackEvent::PlanCompleted {
            plan_id: "p".into(),
            succeeded: true,
            tasks_completed: 3,
            tasks_failed: 0,
            total_cost_usd: 0.1,
        };
        assert!(sink.interested(&event));
        sink.on_event(&event).await.unwrap();
    }
}
