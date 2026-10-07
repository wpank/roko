//! Periodic Lens observation for the long-running server lifecycle.
//!
//! The observer samples the server's shared
//! [`MetricRegistry`](roko_core::obs::metrics::MetricRegistry) immediately at
//! startup and every 30 seconds thereafter, and checks the rolling cost rate
//! for a spike. Samples are derived telemetry, so failures are best-effort and
//! never affect request handling. A sample goes to the debug log only: the
//! JSONL file serve used to keep under `.roko/metrics/` had no reader
//! (backlog 2124).

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use roko_core::obs::lens::{LensRegistry, default_registry};
use roko_core::obs::telemetry_observe::{PeriodicObserver, TelemetryObservation, TelemetryObserve};
use roko_learn::costs_log::CostsLog;
use roko_runtime::cancel::CancelToken;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;

use crate::state::AppState;

const DEFAULT_OBSERVATION_INTERVAL: Duration = Duration::from_secs(30);

/// Default cost-spike threshold in USD per minute.
///
/// When the 15-minute rolling cost rate exceeds this value, a warning is
/// logged. $1/min = $60/hr is a conservative ceiling for most workloads.
const DEFAULT_COST_SPIKE_THRESHOLD: f64 = 1.0;

trait TelemetryObservationSink: Send + Sync {
    fn emit(&self, observations: &[TelemetryObservation]) -> io::Result<()>;
}

/// Logs each Lens sample at debug level and keeps none (backlog 2124).
#[derive(Debug, Default)]
struct DebugLogTelemetryObservationSink;

impl TelemetryObservationSink for DebugLogTelemetryObservationSink {
    fn emit(&self, observations: &[TelemetryObservation]) -> io::Result<()> {
        for observation in observations {
            tracing::debug!(
                lens = %observation.lens_name,
                data = %observation.data,
                "telemetry lens sample"
            );
        }
        Ok(())
    }
}

/// Start the production periodic observer for a live server.
///
/// Only cloneable lifecycle components are captured; the task does not retain
/// `AppState`, so it cannot form an ownership cycle with the server. The
/// server's cancellation token terminates the task during graceful shutdown.
pub(crate) fn start_periodic_telemetry_observer(state: &AppState) -> JoinHandle<()> {
    let registry = Arc::new(default_registry(state.metrics_sink()));
    let costs_path = state.layout.learn_dir().join("costs.jsonl");
    tracing::debug!(
        interval_secs = DEFAULT_OBSERVATION_INTERVAL.as_secs(),
        "periodic telemetry observer started"
    );
    let sink = Arc::new(DebugLogTelemetryObservationSink);

    spawn_periodic_observer(
        registry,
        sink,
        state.cancel.clone(),
        DEFAULT_OBSERVATION_INTERVAL,
        Some(costs_path),
    )
}

fn spawn_periodic_observer(
    registry: Arc<LensRegistry>,
    sink: Arc<dyn TelemetryObservationSink>,
    cancel: CancelToken,
    observation_interval: Duration,
    costs_path: Option<PathBuf>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // A zero interval is never used by production, but clamping makes the
        // internal helper safe for embedders and tests instead of panicking.
        let observation_interval = observation_interval.max(Duration::from_millis(1));
        let mut ticker = tokio::time::interval(observation_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let observer = PeriodicObserver::new();
        let costs_log = costs_path.map(CostsLog::at);

        loop {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                _ = ticker.tick() => {}
            }

            let observations = observer.observe(&registry);
            let lens_count = observations.len();
            tracing::debug!(
                lens_count,
                "periodic observer sampled event-driven Lens state"
            );
            let sink = Arc::clone(&sink);
            match tokio::task::spawn_blocking(move || sink.emit(&observations)).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    tracing::warn!(%error, "periodic telemetry observation emit failed");
                }
                Err(error) => {
                    tracing::warn!(%error, "periodic telemetry observation task failed");
                }
            }

            // Cost spike detection: check the 15-minute rolling cost rate.
            if let Some(ref log) = costs_log {
                match log.is_cost_spike(DEFAULT_COST_SPIKE_THRESHOLD).await {
                    Ok(true) => {
                        tracing::warn!(
                            threshold_usd_per_min = DEFAULT_COST_SPIKE_THRESHOLD,
                            "cost spike detected: 15-minute rolling cost rate exceeds threshold"
                        );
                    }
                    Ok(false) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => {
                        tracing::debug!(%error, "cost spike check failed");
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use roko_core::obs::metrics::MetricRegistry;
    use tokio::sync::Notify;

    use super::*;

    #[derive(Default)]
    struct RecordingSink {
        batches: Mutex<Vec<Vec<TelemetryObservation>>>,
        emitted: Notify,
    }

    impl RecordingSink {
        fn batches(&self) -> Vec<Vec<TelemetryObservation>> {
            self.batches
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }
    }

    impl TelemetryObservationSink for RecordingSink {
        fn emit(&self, observations: &[TelemetryObservation]) -> io::Result<()> {
            self.batches
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(observations.to_vec());
            self.emitted.notify_waiters();
            Ok(())
        }
    }

    fn default_lenses() -> Arc<LensRegistry> {
        Arc::new(default_registry(Arc::new(MetricRegistry::new())))
    }

    #[tokio::test]
    async fn periodic_observer_emits_all_lens_snapshots() {
        let registry = default_lenses();
        let sink = Arc::new(RecordingSink::default());
        let emitted = sink.emitted.notified();
        let cancel = CancelToken::new();
        let handle = spawn_periodic_observer(
            registry,
            Arc::clone(&sink) as Arc<dyn TelemetryObservationSink>,
            cancel.clone(),
            Duration::from_secs(3_600),
            None,
        );

        tokio::time::timeout(Duration::from_secs(2), emitted)
            .await
            .expect("initial telemetry snapshot was not emitted");
        cancel.cancel();
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("telemetry observer did not shut down")
            .expect("telemetry observer panicked");

        let batches = sink.batches();
        assert_eq!(batches.len(), 1);
        let observations = &batches[0];
        let names = observations
            .iter()
            .map(|observation| observation.lens_name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, ["token-usage", "latency", "cost"]);
        assert!(
            observations
                .windows(2)
                .all(|pair| pair[0].timestamp == pair[1].timestamp),
            "one observation cycle must use one timestamp"
        );
        assert!(
            observations
                .iter()
                .all(|observation| !observation.data.is_null())
        );
    }

    #[tokio::test]
    async fn cancellation_stops_waiting_task_and_releases_captured_state() {
        let registry = default_lenses();
        let registry_weak = Arc::downgrade(&registry);
        let sink = Arc::new(RecordingSink::default());
        let sink_weak = Arc::downgrade(&sink);
        let emitted = sink.emitted.notified();
        let cancel = CancelToken::new();
        let handle = spawn_periodic_observer(
            Arc::clone(&registry),
            Arc::clone(&sink) as Arc<dyn TelemetryObservationSink>,
            cancel.clone(),
            Duration::from_secs(3_600),
            None,
        );

        tokio::time::timeout(Duration::from_secs(2), emitted)
            .await
            .expect("initial telemetry snapshot was not emitted");
        drop(registry);
        drop(sink);
        cancel.cancel();
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("telemetry observer leaked after cancellation")
            .expect("telemetry observer panicked");

        assert!(registry_weak.upgrade().is_none());
        assert!(sink_weak.upgrade().is_none());
    }
}
