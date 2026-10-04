//! The showcase idle timer (S11 §4.6, G10, D37): serve exits after `[showcase.idle]
//! exit_after_secs` with no request and nothing running, through the graceful shutdown path.
//!
//! Fly's proxy starts the Machine again on the next request, and `[[restart]] policy =
//! "on-failure"` leaves a Machine that exited 0 stopped, so the showcase costs only the time it is
//! used. Every request but the health checks marks serve active ([`track_activity`]). A run, a
//! plan, a bench or matrix run still going, a hold file younger than two hours (sftp uploads
//! bypass HTTP), or a registered [`IdleBlocker`] keeps it up. The live slices' showcase stream,
//! which closes after `sse_max_idle_secs` of silence, lands with that stream.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, PoisonError, RwLock};
use std::time::Duration;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use chrono::{DateTime, TimeDelta, Utc};
use tracing::info;

use crate::state::{AppState, OperationStatus, PlanHandle, RunHandle};

/// How often the timer looks.
pub const CHECK_EVERY: Duration = Duration::from_secs(30);

/// A hold file younger than this many seconds (two hours) keeps serve up.
pub const HOLD_FRESH_SECS: i64 = 2 * 3_600;

/// Something that keeps serve running while it lasts, such as a live action or an unsettled
/// reservation (9352, 9357).
pub trait IdleBlocker: Send + Sync {
    /// Why serve must stay up now, or `None`.
    fn blocking(&self) -> Option<String>;
}

/// When the last request arrived, and what else keeps serve up.
pub struct IdleTracker {
    last_active_ms: AtomicI64,
    blockers: RwLock<Vec<Arc<dyn IdleBlocker>>>,
}

impl IdleTracker {
    /// A tracker whose last activity is `now`.
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            last_active_ms: AtomicI64::new(now.timestamp_millis()),
            blockers: RwLock::new(Vec::new()),
        }
    }

    /// Note activity at `now`.
    pub fn touch(&self, now: DateTime<Utc>) {
        self.last_active_ms
            .store(now.timestamp_millis(), Ordering::Release);
    }

    /// How long serve has gone without a request at `now`.
    pub fn idle_for(&self, now: DateTime<Utc>) -> TimeDelta {
        let last = self.last_active_ms.load(Ordering::Acquire);
        TimeDelta::milliseconds(now.timestamp_millis().saturating_sub(last))
    }

    /// Keep serve up while `blocker` says so.
    pub fn register(&self, blocker: Arc<dyn IdleBlocker>) {
        self.blockers
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .push(blocker);
    }

    /// What keeps serve up at `now`: a registered blocker, or a fresh `hold_file`.
    pub fn blocked(&self, hold_file: &Path, now: DateTime<Utc>) -> Option<String> {
        let blockers = self
            .blockers
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(reason) = blockers.iter().find_map(|blocker| blocker.blocking()) {
            return Some(reason);
        }
        drop(blockers);
        let modified = std::fs::metadata(hold_file)
            .and_then(|meta| meta.modified())
            .ok()?;
        let age = now.signed_duration_since(DateTime::<Utc>::from(modified));
        let fresh = age < TimeDelta::seconds(HOLD_FRESH_SECS);
        fresh.then(|| format!("hold file {} is fresh", hold_file.display()))
    }
}

/// Whether a request to `path` counts as activity: the health checks do not.
pub fn counts_as_activity(path: &str) -> bool {
    !matches!(path, "/health" | "/ready")
}

/// The activity middleware of a showcase serve: every request but the health checks marks it
/// active.
pub async fn track_activity(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    if counts_as_activity(req.uri().path()) {
        state.showcase_idle.touch(Utc::now());
    }
    next.run(req).await
}

/// Start the idle timer when serve runs in showcase mode.
pub fn start_idle_timer(state: &Arc<AppState>) {
    if state.load_roko_config().showcase.enabled {
        tokio::spawn(run_idle_timer(Arc::clone(state)));
    }
}

/// Every [`CHECK_EVERY`], cancel serve, which shuts it down gracefully, once
/// [`should_exit`] says so; stop when serve stops.
pub async fn run_idle_timer(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(CHECK_EVERY);
    loop {
        tokio::select! {
            () = state.cancel.cancelled() => return,
            _ = interval.tick() => {}
        }
        let now = Utc::now();
        if should_exit(&state, now).await {
            let idle = state.showcase_idle.idle_for(now).num_seconds();
            info!(idle_secs = idle, "showcase idle: no request and nothing running; exiting");
            state.cancel.cancel();
            return;
        }
    }
}

/// Whether a showcase serve should exit at `now`: idle for `exit_after_secs`, with no blocker,
/// no fresh hold file, and no run, plan, bench or matrix run going.
pub async fn should_exit(state: &AppState, now: DateTime<Utc>) -> bool {
    let config = state.load_roko_config();
    let showcase = &config.showcase;
    if !showcase.enabled {
        return false;
    }
    let exit_after = i64::try_from(showcase.idle.exit_after_secs)
        .ok()
        .and_then(TimeDelta::try_seconds)
        .unwrap_or(TimeDelta::MAX);
    if state.showcase_idle.idle_for(now) < exit_after {
        return false;
    }
    let hold_file = hold_file(state, &showcase.idle.hold_file);
    state.showcase_idle.blocked(&hold_file, now).is_none() && !work_in_flight(state).await
}

/// `hold_file`, against the workspace when it is relative.
fn hold_file(state: &AppState, hold_file: &str) -> PathBuf {
    let path = Path::new(hold_file);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        state.workdir.join(path)
    }
}

/// Whether a run, plan, bench run or matrix run is still going.
async fn work_in_flight(state: &AppState) -> bool {
    let running = |run: &RunHandle| matches!(run.status, OperationStatus::Running);
    let runs = state.active_runs.read().await;
    if runs.values().any(running) {
        return true;
    }
    drop(runs);
    let plans = state.active_plans.read().await;
    if plans.values().any(PlanHandle::is_live) {
        return true;
    }
    drop(plans);
    !state.active_bench_runs.read().await.is_empty()
        || !state.active_matrix_runs.read().await.is_empty()
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::routing::get;
    use roko_core::config::schema::RokoConfig;
    use tower::ServiceExt as _;

    use super::*;
    use crate::deploy::manual::ManualBackend;
    use crate::runtime::NoOpRuntime;

    fn showcase_state(workdir: &Path) -> Arc<AppState> {
        let mut config = RokoConfig::default();
        config.showcase.enabled = true;
        Arc::new(
            AppState::new(
                workdir.to_path_buf(),
                Arc::new(NoOpRuntime),
                config,
                Arc::new(ManualBackend::default()),
            )
            .expect("AppState::new"),
        )
    }

    fn at(start: DateTime<Utc>, secs: i64) -> DateTime<Utc> {
        start + TimeDelta::seconds(secs)
    }

    struct LiveRun;

    impl IdleBlocker for LiveRun {
        fn blocking(&self) -> Option<String> {
            Some("run r-1 is live".to_string())
        }
    }

    #[tokio::test]
    async fn showcase_idle_exits_after_twenty_quiet_minutes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = showcase_state(dir.path());
        let start = Utc::now();
        state.showcase_idle.touch(start);

        assert!(!should_exit(&state, at(start, 1_199)).await);
        assert!(should_exit(&state, at(start, 1_200)).await);
        state.showcase_idle.touch(at(start, 1_300));
        assert!(!should_exit(&state, at(start, 2_000)).await);
    }

    #[tokio::test]
    async fn showcase_idle_exit_waits_for_active_run() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = showcase_state(dir.path());
        let start = Utc::now();
        state.showcase_idle.touch(start);
        let quiet = at(start, 3_600);

        let handle = tokio::spawn(std::future::pending::<()>());
        let run = crate::state::RunHandle {
            id: "r-1".to_string(),
            prompt: "say hello".to_string(),
            status: OperationStatus::Running,
            result: None,
            verdict: None,
            cancel: None,
            handle,
        };
        state.active_runs.write().await.insert("r-1".to_string(), run);
        assert!(!should_exit(&state, quiet).await);
        let finished = state.active_runs.write().await.remove("r-1").expect("run");
        finished.handle.abort();
        assert!(should_exit(&state, quiet).await);

        // A registered blocker, such as a live action, holds it too.
        state.showcase_idle.register(Arc::new(LiveRun));
        assert!(!should_exit(&state, quiet).await);
    }

    #[tokio::test]
    async fn showcase_idle_fresh_hold_file_keeps_serve_up() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = showcase_state(dir.path());
        let hold = dir.path().join(".roko").join("showcase").join("hold");
        std::fs::create_dir_all(hold.parent().expect("parent")).expect("hold dir");
        std::fs::write(&hold, "").expect("hold file");
        let now = Utc::now();
        state.showcase_idle.touch(at(now, -3_600));

        assert!(!should_exit(&state, now).await);
        let stale = std::time::SystemTime::now() - Duration::from_secs(3 * 3_600);
        let file = std::fs::File::options()
            .write(true)
            .open(&hold)
            .expect("open hold file");
        file.set_modified(stale).expect("age the hold file");
        assert!(should_exit(&state, now).await);
    }

    #[tokio::test]
    async fn showcase_idle_health_checks_do_not_count() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = showcase_state(dir.path());
        let ok = || async { "ok" };
        let app = Router::new()
            .route("/health", get(ok))
            .route("/ready", get(ok))
            .route("/api/showcase/manifest", get(ok))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                track_activity,
            ))
            .with_state(Arc::clone(&state));
        let long_ago = at(Utc::now(), -7_200);
        state.showcase_idle.touch(long_ago);
        let request = |uri: &str| {
            axum::http::Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request")
        };

        for uri in ["/health", "/ready"] {
            app.clone().oneshot(request(uri)).await.expect("response");
        }
        assert!(state.showcase_idle.idle_for(Utc::now()) >= TimeDelta::hours(1));
        app.oneshot(request("/api/showcase/manifest"))
            .await
            .expect("response");
        assert!(state.showcase_idle.idle_for(Utc::now()) < TimeDelta::minutes(1));
    }

    #[tokio::test]
    async fn showcase_idle_never_exits_outside_showcase_mode() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                RokoConfig::default(),
                Arc::new(ManualBackend::default()),
            )
            .expect("AppState::new"),
        );
        state.showcase_idle.touch(at(Utc::now(), -86_400));
        assert!(!should_exit(&state, Utc::now()).await);
    }
}
