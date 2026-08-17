//! Periodic Lens observation for the long-running server lifecycle.
//!
//! The observer samples the server's shared
//! [`MetricRegistry`](roko_core::obs::metrics::MetricRegistry) immediately at
//! startup and every 30 seconds thereafter. Samples are derived telemetry, so
//! failures are best-effort and never affect request handling. Each observation
//! is persisted as one JSONL record under `.roko/metrics/`; the canonical
//! resource log-size limit controls rotation.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use roko_core::obs::lens::{LensRegistry, default_registry};
use roko_core::obs::telemetry_observe::{PeriodicObserver, TelemetryObservation, TelemetryObserve};
use roko_runtime::cancel::CancelToken;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;

use crate::state::AppState;

const DEFAULT_OBSERVATION_INTERVAL: Duration = Duration::from_secs(30);

trait TelemetryObservationSink: Send + Sync {
    fn emit(&self, observations: &[TelemetryObservation]) -> io::Result<()>;
}

#[derive(Debug)]
struct JsonlTelemetryObservationSink {
    path: PathBuf,
    max_mb: u64,
}

impl JsonlTelemetryObservationSink {
    fn new(path: impl Into<PathBuf>, max_mb: u64) -> Self {
        Self {
            path: path.into(),
            max_mb,
        }
    }
}

impl TelemetryObservationSink for JsonlTelemetryObservationSink {
    fn emit(&self, observations: &[TelemetryObservation]) -> io::Result<()> {
        if observations.is_empty() {
            return Ok(());
        }

        // Serialize the complete cycle before touching the file. Appending the
        // batch under one rotation lock prevents lenses from being split across
        // archived and live generations by another writer.
        let mut batch = Vec::new();
        for observation in observations {
            serde_json::to_writer(&mut batch, observation)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            batch.push(b'\n');
        }

        roko_fs::log_rotation::append_jsonl_line_sync(&self.path, &batch, self.max_mb)?;
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
    let max_mb = state.load_roko_config().resources.log_rotation_max_mb;
    let path = state.layout.telemetry_observations_path();
    tracing::debug!(
        path = %path.display(),
        interval_secs = DEFAULT_OBSERVATION_INTERVAL.as_secs(),
        max_mb,
        "periodic telemetry observer started"
    );
    let sink = Arc::new(JsonlTelemetryObservationSink::new(path, max_mb));

    spawn_periodic_observer(
        registry,
        sink,
        state.cancel.clone(),
        DEFAULT_OBSERVATION_INTERVAL,
    )
}

fn spawn_periodic_observer(
    registry: Arc<LensRegistry>,
    sink: Arc<dyn TelemetryObservationSink>,
    cancel: CancelToken,
    observation_interval: Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // A zero interval is never used by production, but clamping makes the
        // internal helper safe for embedders and tests instead of panicking.
        let observation_interval = observation_interval.max(Duration::from_millis(1));
        let mut ticker = tokio::time::interval(observation_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let observer = PeriodicObserver::new();

        loop {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                _ = ticker.tick() => {}
            }

            let observations = observer.observe(&registry);
            let sink = Arc::clone(&sink);
            match tokio::task::spawn_blocking(move || sink.emit(&observations)).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    tracing::warn!(%error, "periodic telemetry observation persist failed");
                }
                Err(error) => {
                    tracing::warn!(%error, "periodic telemetry observation task failed");
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

    #[test]
    fn jsonl_sink_persists_one_parseable_record_per_lens() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("telemetry-observations.jsonl");
        let sink = JsonlTelemetryObservationSink::new(&path, 100);
        let observer = PeriodicObserver::new();
        let registry = default_lenses();

        sink.emit(&observer.observe(&registry)).expect("emit");

        let contents = std::fs::read_to_string(path).expect("read telemetry JSONL");
        let observations = contents
            .lines()
            .map(|line| serde_json::from_str::<TelemetryObservation>(line).expect("parse line"))
            .collect::<Vec<_>>();
        assert_eq!(observations.len(), 3);
        assert_eq!(observations[0].lens_name, "token-usage");
        assert_eq!(observations[1].lens_name, "latency");
        assert_eq!(observations[2].lens_name, "cost");
    }
}
