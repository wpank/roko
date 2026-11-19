//! The showcase idle timer (S11 §4.6, G10, D37): serve exits after `[showcase.idle]
//! exit_after_secs` with no request and nothing running, through the graceful shutdown path.
//!
//! Fly's proxy starts the Machine again on the next request, and `[[restart]] policy =
//! "on-failure"` leaves a Machine that exited 0 stopped, so the showcase costs only the time it is
//! used. Every request but the health checks marks serve active ([`track_activity`]). A run, a
//! plan, a bench or matrix run still going, a hold file younger than two hours (sftp uploads
//! bypass HTTP), or a registered [`IdleBlocker`] keeps it up. The live slices' showcase stream,
//! which closes after `sse_max_idle_secs` of silence, lands with that stream.
//!
//! `roko serve --idle-exit SECS` uses the same timer outside showcase mode (gap-fb1594): serve
//! exits after SECS with no request but the health checks, no run or plan going, and no
//! [`OpenStream`] — a response still streaming, such as an event stream, or an open WebSocket.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, PoisonError, RwLock};
use std::task::{Context, Poll};
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use chrono::{DateTime, TimeDelta, Utc};
use hyper::body::{Body as HttpBody, Bytes, Frame, SizeHint};
use tracing::info;

use crate::state::{AppState, OperationStatus, PlanHandle, RunHandle};

/// How often the timer looks, at most.
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
    /// `--idle-exit` seconds; `0` never.
    exit_after_secs: AtomicU64,
    /// Responses still streaming and WebSockets still open ([`OpenStream`]).
    open_streams: AtomicUsize,
}

impl IdleTracker {
    /// A tracker whose last activity is `now`.
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            last_active_ms: AtomicI64::new(now.timestamp_millis()),
            blockers: RwLock::new(Vec::new()),
            exit_after_secs: AtomicU64::new(0),
            open_streams: AtomicUsize::new(0),
        }
    }

    /// `--idle-exit`: outside showcase mode, exit after `secs` idle seconds;
    /// `None` or `0` never.
    pub fn set_exit_after(&self, secs: Option<u64>) {
        self.exit_after_secs
            .store(secs.unwrap_or(0), Ordering::Release);
    }

    /// The `--idle-exit` seconds, when set.
    pub fn exit_after(&self) -> Option<u64> {
        Some(self.exit_after_secs.load(Ordering::Acquire)).filter(|secs| *secs > 0)
    }

    /// How many responses are still streaming and WebSockets still open.
    pub fn open_streams(&self) -> usize {
        self.open_streams.load(Ordering::Acquire)
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

    /// What a registered blocker says keeps serve up now.
    pub fn blocker(&self) -> Option<String> {
        let blockers = self.blockers.read().unwrap_or_else(PoisonError::into_inner);
        blockers.iter().find_map(|blocker| blocker.blocking())
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
        if let Some(reason) = self.blocker() {
            return Some(reason);
        }
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

/// The activity middleware of a showcase serve, and of one started with `--idle-exit`.
///
/// Every request but the health checks marks serve active, and stays an [`OpenStream`] until its
/// response body ends, so an event stream counts for as long as a client reads it.
pub async fn track_activity(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    if !counts_as_activity(req.uri().path()) {
        return next.run(req).await;
    }
    state.showcase_idle.touch(Utc::now());
    let open = OpenStream::new(Arc::clone(&state));
    next.run(req)
        .await
        .map(|body| Body::new(StreamingBody { body, _open: open }))
}

/// One response still streaming, or one WebSocket still open: an `--idle-exit` serve waits for it
/// to end, and counts its idle time from then (gap-fb1594).
pub struct OpenStream {
    state: Arc<AppState>,
}

impl OpenStream {
    /// Count a stream open on `state` until the returned value drops.
    pub fn new(state: Arc<AppState>) -> Self {
        state
            .showcase_idle
            .open_streams
            .fetch_add(1, Ordering::AcqRel);
        Self { state }
    }

    /// Run `connection`, such as a WebSocket's handler, as this open stream.
    pub async fn hold<F: Future>(self, connection: F) -> F::Output {
        let output = connection.await;
        drop(self);
        output
    }
}

impl Drop for OpenStream {
    fn drop(&mut self) {
        let tracker = &self.state.showcase_idle;
        tracker.touch(Utc::now());
        tracker.open_streams.fetch_sub(1, Ordering::AcqRel);
    }
}

/// A response body that keeps its request an [`OpenStream`] until it ends or is dropped.
struct StreamingBody {
    body: Body,
    _open: OpenStream,
}

impl HttpBody for StreamingBody {
    type Data = Bytes;
    type Error = axum::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.get_mut().body).poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

/// Start the idle timer when serve runs in showcase mode, or with `--idle-exit`.
pub fn start_idle_timer(state: &Arc<AppState>) {
    if tracks_activity(state) {
        tokio::spawn(run_idle_timer(Arc::clone(state)));
    }
}

/// Whether serve may exit when idle, so its requests must be tracked: in showcase mode, or with
/// `--idle-exit`.
pub fn tracks_activity(state: &AppState) -> bool {
    state.load_roko_config().showcase.enabled || state.showcase_idle.exit_after().is_some()
}

/// How often the timer looks: every [`CHECK_EVERY`], or a quarter of a shorter `--idle-exit`, but
/// at least every second.
fn check_every(state: &AppState) -> Duration {
    match state.showcase_idle.exit_after() {
        Some(secs) if !state.load_roko_config().showcase.enabled => {
            (Duration::from_secs(secs) / 4).clamp(Duration::from_secs(1), CHECK_EVERY)
        }
        _ => CHECK_EVERY,
    }
}

/// Every [`check_every`], cancel serve, which shuts it down gracefully, once
/// [`should_exit`] says so; stop when serve stops.
pub async fn run_idle_timer(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(check_every(&state));
    loop {
        tokio::select! {
            () = state.cancel.cancelled() => return,
            _ = interval.tick() => {}
        }
        let now = Utc::now();
        if should_exit(&state, now).await {
            let idle = state.showcase_idle.idle_for(now).num_seconds();
            info!(
                idle_secs = idle,
                "idle: no request and nothing running; exiting"
            );
            state.cancel.cancel();
            return;
        }
    }
}

/// Whether serve should exit at `now`.
///
/// A showcase serve exits once idle for `exit_after_secs`, with no blocker, no fresh hold file,
/// and no run, plan, bench or matrix run going. Outside showcase mode, a serve started with
/// `--idle-exit` exits once idle that long with no blocker, nothing going, and no [`OpenStream`].
pub async fn should_exit(state: &AppState, now: DateTime<Utc>) -> bool {
    let config = state.load_roko_config();
    let showcase = &config.showcase;
    let (exit_after_secs, hold_file) = if showcase.enabled {
        let hold = hold_file(state, &showcase.idle.hold_file);
        (showcase.idle.exit_after_secs, Some(hold))
    } else if let Some(secs) = state.showcase_idle.exit_after() {
        if state.showcase_idle.open_streams() > 0 {
            return false;
        }
        (secs, None)
    } else {
        return false;
    };
    let exit_after = i64::try_from(exit_after_secs)
        .ok()
        .and_then(TimeDelta::try_seconds)
        .unwrap_or(TimeDelta::MAX);
    if state.showcase_idle.idle_for(now) < exit_after {
        return false;
    }
    let blocked = match hold_file {
        Some(hold_file) => state.showcase_idle.blocked(&hold_file, now),
        None => state.showcase_idle.blocker(),
    };
    blocked.is_none() && !work_in_flight(state).await
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
        state
            .active_runs
            .write()
            .await
            .insert("r-1".to_string(), run);
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

    fn plain_state(workdir: &Path) -> Arc<AppState> {
        Arc::new(
            AppState::new(
                workdir.to_path_buf(),
                Arc::new(NoOpRuntime),
                RokoConfig::default(),
                Arc::new(ManualBackend::default()),
            )
            .expect("AppState::new"),
        )
    }

    /// gap-fb1594: `--idle-exit` exits a serve outside showcase mode once it
    /// has been idle that long, but never while a stream is open, and counts
    /// its idle time from when the last stream closed.
    #[tokio::test]
    async fn idle_exit_waits_for_quiet_time_and_open_streams() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = plain_state(dir.path());
        assert!(!tracks_activity(&state), "off without --idle-exit");
        state.showcase_idle.set_exit_after(Some(60));
        assert!(tracks_activity(&state));
        assert_eq!(check_every(&state), Duration::from_secs(15));
        let start = Utc::now();
        state.showcase_idle.touch(start);

        assert!(!should_exit(&state, at(start, 59)).await);
        assert!(should_exit(&state, at(start, 60)).await);

        let open = OpenStream::new(Arc::clone(&state));
        assert_eq!(state.showcase_idle.open_streams(), 1);
        assert!(!should_exit(&state, at(start, 3_600)).await);
        drop(open);
        assert_eq!(state.showcase_idle.open_streams(), 0);
        let closed = Utc::now();
        assert!(!should_exit(&state, at(closed, 30)).await);
        assert!(should_exit(&state, at(closed, 61)).await);

        state.showcase_idle.register(Arc::new(LiveRun));
        assert!(!should_exit(&state, at(closed, 61)).await);

        state.showcase_idle.set_exit_after(Some(0));
        assert!(!tracks_activity(&state), "0 never exits");
        assert!(!should_exit(&state, at(closed, 86_400)).await);
    }

    /// gap-fb1594: a request stays an open stream until its response body
    /// ends, so an event stream keeps an `--idle-exit` serve up for as long as
    /// a client reads it; the health checks never count.
    #[tokio::test]
    async fn a_streaming_response_counts_until_its_body_is_dropped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = plain_state(dir.path());
        state.showcase_idle.set_exit_after(Some(60));
        let (_tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(1);
        let stream = Arc::new(std::sync::Mutex::new(Some(rx)));
        let app = Router::new()
            .route("/health", get(|| async { "ok" }))
            .route(
                "/api/events",
                get(move || {
                    let rx = stream.lock().expect("lock").take().expect("one stream");
                    async move { Body::from_stream(tokio_stream_from(rx)) }
                }),
            )
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                track_activity,
            ))
            .with_state(Arc::clone(&state));
        let request = |uri: &str| {
            axum::http::Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request")
        };

        let health = app
            .clone()
            .oneshot(request("/health"))
            .await
            .expect("health");
        assert_eq!(state.showcase_idle.open_streams(), 0);
        drop(health);
        let events = app.oneshot(request("/api/events")).await.expect("events");
        assert_eq!(
            state.showcase_idle.open_streams(),
            1,
            "the body still streams"
        );
        drop(events);
        assert_eq!(state.showcase_idle.open_streams(), 0);
    }

    /// A body stream over `rx`.
    fn tokio_stream_from(
        mut rx: tokio::sync::mpsc::Receiver<Result<Bytes, std::io::Error>>,
    ) -> impl futures::Stream<Item = Result<Bytes, std::io::Error>> {
        futures::stream::poll_fn(move |cx| rx.poll_recv(cx))
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
