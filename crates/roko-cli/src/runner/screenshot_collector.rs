//! Bounded, non-blocking screenshot collection for `plan run --screenshots`.
//!
//! The runner only clones the latest [`DashboardSnapshot`] and attempts to
//! enqueue a capture request. A dedicated worker owns all TUI rendering and
//! filesystem I/O, so a slow disk or an expensive frame cannot stall plan
//! execution.

use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, bail};
use chrono::Utc;
use roko_core::DashboardSnapshot;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tracing::warn;

use crate::tui::{App, Tab};

const DEFAULT_WIDTH: u16 = 180;
const DEFAULT_HEIGHT: u16 = 55;
const MIN_WIDTH: u16 = 40;
const MIN_HEIGHT: u16 = 12;
const MAX_WIDTH: u16 = 500;
const MAX_HEIGHT: u16 = 200;
const DEFAULT_QUEUE_CAPACITY: usize = 128;
const DEFAULT_MIN_FREE_DISK_MB: u64 = 500;
const DEFAULT_MAX_CAPTURES: usize = 10_000;

/// Settings for one continuous screenshot run.
#[derive(Debug, Clone)]
pub struct ScreenshotCollectorConfig {
    /// Repository root used to initialize the headless [`App`].
    pub workdir: PathBuf,
    /// Exact run directory requested by the operator. `None` creates a unique
    /// timestamped directory below `.roko/screenshots`.
    pub run_dir: Option<PathBuf>,
    /// Maximum delay between periodic full-frame captures.
    pub interval: Duration,
    /// Virtual terminal width.
    pub width: u16,
    /// Virtual terminal height.
    pub height: u16,
    /// Bound on pending render requests.
    pub queue_capacity: usize,
    /// Stop adding files once this many capture attempts have been recorded.
    pub max_captures: usize,
    /// Skip rendering when the capture filesystem has less free space.
    pub min_free_disk_mb: u64,
}

impl ScreenshotCollectorConfig {
    /// Build the shipping configuration for a plan run.
    #[must_use]
    pub fn for_plan_run(
        workdir: impl Into<PathBuf>,
        run_dir: Option<PathBuf>,
        interval: Duration,
    ) -> Self {
        Self {
            workdir: workdir.into(),
            run_dir,
            interval,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
            max_captures: DEFAULT_MAX_CAPTURES,
            min_free_disk_mb: DEFAULT_MIN_FREE_DISK_MB,
        }
    }

    fn validate(&self) -> Result<()> {
        if !(MIN_WIDTH..=MAX_WIDTH).contains(&self.width) {
            bail!(
                "screenshot width must be in {MIN_WIDTH}..={MAX_WIDTH} (got {})",
                self.width
            );
        }
        if !(MIN_HEIGHT..=MAX_HEIGHT).contains(&self.height) {
            bail!(
                "screenshot height must be in {MIN_HEIGHT}..={MAX_HEIGHT} (got {})",
                self.height
            );
        }
        if self.interval.is_zero() {
            bail!("screenshot interval must be greater than zero");
        }
        if self.queue_capacity == 0 {
            bail!("screenshot queue capacity must be greater than zero");
        }
        if self.max_captures == 0 {
            bail!("screenshot max captures must be greater than zero");
        }
        Ok(())
    }
}

/// One capture in the durable timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestCapture {
    /// Monotonic capture index.
    pub index: usize,
    /// Stable event label.
    pub label: String,
    /// Event-specific identifier, such as `plan/task/gate`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Wall-clock capture time.
    pub timestamp: String,
    /// Time since collector construction.
    pub elapsed_secs: f64,
    /// Requested tabs, in render order.
    pub tabs: Vec<String>,
    /// Capture directory relative to the run directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Text frames relative to the capture directory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    /// `captured`, `skipped`, or `error`.
    pub status: String,
    /// A bounded explanation for skipped/error entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Durable timeline for a continuous screenshot run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Manifest schema version.
    pub schema_version: u32,
    /// Rendering path used for every text frame.
    pub renderer: String,
    /// Stable run directory name.
    pub run_id: String,
    /// Wall-clock collector start time.
    pub started_at: String,
    /// Absolute or workdir-relative display path.
    pub run_dir: String,
    /// Virtual terminal dimensions.
    pub width: u16,
    pub height: u16,
    /// Configured periodic interval.
    pub interval_secs: f64,
    /// Requests rejected because the bounded queue was full.
    pub dropped_requests: u64,
    /// Ordered event/capture timeline.
    pub captures: Vec<ManifestCapture>,
}

#[derive(Debug)]
struct CaptureRequest {
    label: String,
    detail: Option<String>,
    tabs: Vec<Tab>,
    snapshot: DashboardSnapshot,
}

enum WorkerMessage {
    Capture(CaptureRequest),
    Shutdown(DashboardSnapshot),
}

struct CollectorInner {
    sender: mpsc::SyncSender<WorkerMessage>,
    snapshot_rx: watch::Receiver<DashboardSnapshot>,
    dropped_requests: Arc<AtomicU64>,
    worker: Mutex<Option<JoinHandle<()>>>,
    run_dir: PathBuf,
}

impl Drop for CollectorInner {
    fn drop(&mut self) {
        // Shutdown is deliberately reliable. Ordinary captures never block,
        // but the final owner waits for queue capacity so the worker can append
        // a final full-state frame and atomically persist the manifest.
        let snapshot = self.snapshot_rx.borrow().clone();
        let _ = self.sender.send(WorkerMessage::Shutdown(snapshot));
        if let Some(worker) = self
            .worker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
            let _ = worker.join();
        }
    }
}

/// Clone-safe, non-blocking capture handle used by the runner and TUI bridge.
#[derive(Clone)]
pub struct ScreenshotCollector {
    inner: Arc<CollectorInner>,
}

impl ScreenshotCollector {
    /// Create the run directory, initialize a durable empty manifest, update
    /// `.roko/screenshots/latest`, and start the render worker.
    pub fn start(
        mut config: ScreenshotCollectorConfig,
        snapshot_rx: watch::Receiver<DashboardSnapshot>,
    ) -> Result<Self> {
        config.validate()?;
        if let Some(path) = config.run_dir.take() {
            config.run_dir = Some(if path.is_absolute() {
                path
            } else {
                config.workdir.join(path)
            });
        }

        let run_dir = create_unique_run_dir(&config)?;
        let run_dir = fs::canonicalize(&run_dir).unwrap_or(run_dir);
        update_latest_symlink(&config.workdir, &run_dir);

        let dropped_requests = Arc::new(AtomicU64::new(0));
        let manifest = new_manifest(&config, &run_dir, 0);
        write_manifest(&run_dir, &manifest).context("initialize screenshot manifest")?;

        let (sender, receiver) = mpsc::sync_channel(config.queue_capacity);
        let worker_run_dir = run_dir.clone();
        let worker_dropped = Arc::clone(&dropped_requests);
        let worker_snapshot_rx = snapshot_rx.clone();
        let worker = std::thread::Builder::new()
            .name("roko-screenshot-collector".to_string())
            .spawn(move || {
                worker_loop(
                    config,
                    worker_run_dir,
                    worker_snapshot_rx,
                    receiver,
                    worker_dropped,
                    manifest,
                );
            })
            .context("spawn screenshot collector worker")?;

        Ok(Self {
            inner: Arc::new(CollectorInner {
                sender,
                snapshot_rx,
                dropped_requests,
                worker: Mutex::new(Some(worker)),
                run_dir,
            }),
        })
    }

    /// Queue an event capture using the state visible at this exact call.
    /// Returns false when the bounded worker queue is full or disconnected.
    #[must_use]
    pub fn capture_event(&self, label: &str, detail: Option<String>, tabs: &[Tab]) -> bool {
        let request = CaptureRequest {
            label: sanitize_label(label),
            detail: detail.map(|value| bounded_detail(&value)),
            tabs: deduplicate_tabs(tabs),
            snapshot: self.inner.snapshot_rx.borrow().clone(),
        };
        match self.inner.sender.try_send(WorkerMessage::Capture(request)) {
            Ok(()) => true,
            Err(mpsc::TrySendError::Full(_)) => {
                self.inner.dropped_requests.fetch_add(1, Ordering::Relaxed);
                warn!(
                    event = label,
                    "screenshot request skipped: collector queue is full"
                );
                false
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                self.inner.dropped_requests.fetch_add(1, Ordering::Relaxed);
                warn!(
                    event = label,
                    "screenshot request skipped: collector stopped"
                );
                false
            }
        }
    }

    /// Queue the initial all-tab frame.
    pub fn capture_startup(&self) {
        let _ = self.capture_event("startup", None, &Tab::ALL);
    }

    /// Directory containing the manifest and capture subdirectories.
    #[must_use]
    pub fn run_dir(&self) -> &Path {
        &self.inner.run_dir
    }
}

fn worker_loop(
    config: ScreenshotCollectorConfig,
    run_dir: PathBuf,
    snapshot_rx: watch::Receiver<DashboardSnapshot>,
    receiver: mpsc::Receiver<WorkerMessage>,
    dropped_requests: Arc<AtomicU64>,
    mut manifest: Manifest,
) {
    let started = Instant::now();
    let mut next_periodic = Instant::now() + config.interval;

    loop {
        let timeout = next_periodic.saturating_duration_since(Instant::now());
        match receiver.recv_timeout(timeout) {
            Ok(WorkerMessage::Capture(request)) => capture_one(
                &config,
                &run_dir,
                &dropped_requests,
                &mut manifest,
                started,
                request,
            ),
            Ok(WorkerMessage::Shutdown(snapshot)) => {
                capture_one(
                    &config,
                    &run_dir,
                    &dropped_requests,
                    &mut manifest,
                    started,
                    CaptureRequest {
                        label: "shutdown".to_string(),
                        detail: None,
                        tabs: Tab::ALL.to_vec(),
                        snapshot,
                    },
                );
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                capture_one(
                    &config,
                    &run_dir,
                    &dropped_requests,
                    &mut manifest,
                    started,
                    CaptureRequest {
                        label: "interval".to_string(),
                        detail: None,
                        tabs: Tab::ALL.to_vec(),
                        snapshot: snapshot_rx.borrow().clone(),
                    },
                );
                next_periodic = Instant::now() + config.interval;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        if Instant::now() >= next_periodic {
            capture_one(
                &config,
                &run_dir,
                &dropped_requests,
                &mut manifest,
                started,
                CaptureRequest {
                    label: "interval".to_string(),
                    detail: None,
                    tabs: Tab::ALL.to_vec(),
                    snapshot: snapshot_rx.borrow().clone(),
                },
            );
            next_periodic = Instant::now() + config.interval;
        }
    }

    manifest.dropped_requests = dropped_requests.load(Ordering::Relaxed);
    if let Err(error) = write_manifest(&run_dir, &manifest) {
        warn!(%error, "failed to write final screenshot manifest");
    }
}

fn capture_one(
    config: &ScreenshotCollectorConfig,
    run_dir: &Path,
    dropped_requests: &AtomicU64,
    manifest: &mut Manifest,
    started: Instant,
    request: CaptureRequest,
) {
    let terminal_capture = matches!(request.label.as_str(), "completion" | "shutdown");
    if manifest.captures.len() >= config.max_captures && !terminal_capture {
        dropped_requests.fetch_add(1, Ordering::Relaxed);
        return;
    }

    let index = manifest.captures.len();
    let timestamp = Utc::now().to_rfc3339();
    let tabs = request
        .tabs
        .iter()
        .map(|tab| tab.label_with_key().to_string())
        .collect::<Vec<_>>();
    let mut entry = ManifestCapture {
        index,
        label: request.label.clone(),
        detail: request.detail.clone(),
        timestamp,
        elapsed_secs: started.elapsed().as_secs_f64(),
        tabs,
        path: None,
        files: Vec::new(),
        status: "captured".to_string(),
        reason: None,
    };

    match roko_fs::available_disk_mb(run_dir) {
        Ok(free_mb) if free_mb < config.min_free_disk_mb => {
            entry.status = "skipped".to_string();
            entry.reason = Some(format!(
                "low disk space: {free_mb} MB available; {} MB required",
                config.min_free_disk_mb
            ));
            warn!(
                free_mb,
                minimum_mb = config.min_free_disk_mb,
                event = %entry.label,
                "screenshot skipped because disk space is low"
            );
        }
        _ => {
            let rendered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                render_capture(config, run_dir, index, &request)
            }));
            match rendered {
                Ok(Ok((path, files))) => {
                    entry.path = Some(path);
                    entry.files = files;
                }
                Ok(Err(error)) => {
                    entry.status = "error".to_string();
                    entry.reason = Some(bounded_detail(&format!("{error:#}")));
                    warn!(%error, event = %entry.label, "screenshot capture failed");
                }
                Err(payload) => {
                    let reason = panic_reason(payload.as_ref());
                    entry.status = "error".to_string();
                    entry.reason = Some(bounded_detail(&format!("renderer panicked: {reason}")));
                    warn!(event = %entry.label, %reason, "screenshot renderer panicked");
                }
            }
        }
    }

    manifest.captures.push(entry);
    manifest.dropped_requests = dropped_requests.load(Ordering::Relaxed);
    if let Err(error) = write_manifest(run_dir, manifest) {
        warn!(%error, "failed to durably update screenshot manifest");
    }
}

fn render_capture(
    config: &ScreenshotCollectorConfig,
    run_dir: &Path,
    index: usize,
    request: &CaptureRequest,
) -> Result<(String, Vec<String>)> {
    let capture_dir = create_unique_capture_dir(run_dir, index, &request.label)?;
    let mut app = App::new_with_dashboard_snapshot(&config.workdir, &request.snapshot);
    let rendered = app.render_tabs_to_text(config.width, config.height, &request.tabs);
    if rendered.len() != request.tabs.len() {
        bail!(
            "renderer returned {} tabs for {} requested",
            rendered.len(),
            request.tabs.len()
        );
    }

    let mut files = Vec::with_capacity(rendered.len());
    for (tab, text) in rendered {
        let file_name = format!(
            "{}-{}.txt",
            tab.snapshot_key(),
            tab.label().to_ascii_lowercase()
        );
        atomic_write(&capture_dir.join(&file_name), text.as_bytes())
            .with_context(|| format!("write {file_name}"))?;
        files.push(file_name);
    }

    Ok((
        capture_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        files,
    ))
}

fn new_manifest(
    config: &ScreenshotCollectorConfig,
    run_dir: &Path,
    dropped_requests: u64,
) -> Manifest {
    Manifest {
        schema_version: 2,
        renderer: "app.draw/full-frame/dashboard-snapshot".to_string(),
        run_id: run_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        started_at: Utc::now().to_rfc3339(),
        run_dir: run_dir.display().to_string(),
        width: config.width,
        height: config.height,
        interval_secs: config.interval.as_secs_f64(),
        dropped_requests,
        captures: Vec::new(),
    }
}

fn create_unique_run_dir(config: &ScreenshotCollectorConfig) -> Result<PathBuf> {
    let base = config.workdir.join(".roko").join("screenshots");
    fs::create_dir_all(&base)
        .with_context(|| format!("create screenshot root {}", base.display()))?;
    let requested = config.run_dir.clone().unwrap_or_else(|| {
        let stamp = Utc::now().format("%Y%m%d-%H%M%S-%3f");
        base.join(format!("run-{stamp}-{}", std::process::id()))
    });
    create_unique_directory(&requested)
}

fn create_unique_capture_dir(run_dir: &Path, index: usize, label: &str) -> Result<PathBuf> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    create_unique_directory(&run_dir.join(format!("{index:05}-{millis}-{label}")))
}

fn create_unique_directory(requested: &Path) -> Result<PathBuf> {
    if let Some(parent) = requested.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create parent directory {}", parent.display()))?;
    }
    for suffix in 0..=10_000_u32 {
        let candidate = if suffix == 0 {
            requested.to_path_buf()
        } else {
            PathBuf::from(format!("{}-{suffix:04}", requested.display()))
        };
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("create screenshot directory {}", candidate.display())
                });
            }
        }
    }
    bail!("could not allocate a unique screenshot directory")
}

fn write_manifest(run_dir: &Path, manifest: &Manifest) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(manifest).context("serialize screenshot manifest")?;
    atomic_write(&run_dir.join("manifest.json"), &bytes)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
    let parent = path.parent().context("atomic write path has no parent")?;
    let file_name = path
        .file_name()
        .context("atomic write path has no filename")?;
    let temp = parent.join(format!(
        ".{}.tmp-{}-{}",
        file_name.to_string_lossy(),
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .with_context(|| format!("create temporary file {}", temp.display()))?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)
            .with_context(|| format!("replace {} atomically", path.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn update_latest_symlink(workdir: &Path, run_dir: &Path) {
    let root = workdir.join(".roko").join("screenshots");
    let latest = root.join("latest");
    if let Ok(metadata) = fs::symlink_metadata(&latest)
        && metadata.is_dir()
        && !metadata.file_type().is_symlink()
        && let Err(error) = preserve_legacy_latest_directory(&latest)
    {
        warn!(
            %error,
            path = %latest.display(),
            "failed to preserve legacy latest screenshot directory"
        );
        return;
    }
    let temp = root.join(format!(
        ".latest-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let target = run_dir
        .strip_prefix(&root)
        .map_or_else(|_| run_dir.to_path_buf(), Path::to_path_buf);

    #[cfg(unix)]
    let link_result = std::os::unix::fs::symlink(&target, &temp);
    #[cfg(windows)]
    let link_result = std::os::windows::fs::symlink_dir(&target, &temp);
    #[cfg(not(any(unix, windows)))]
    let link_result: io::Result<()> = Ok(());

    if let Err(error) = link_result.and_then(|()| fs::rename(&temp, &latest)) {
        let _ = fs::remove_file(&temp);
        warn!(%error, path = %latest.display(), "failed to update latest screenshot symlink");
    }
}

fn preserve_legacy_latest_directory(latest: &Path) -> io::Result<()> {
    let parent = latest.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "latest screenshot path has no parent",
        )
    })?;
    let stamp = Utc::now().format("%Y%m%d-%H%M%S-%3f");
    for suffix in 0..=10_000_u32 {
        let name = if suffix == 0 {
            format!("latest.previous-{stamp}")
        } else {
            format!("latest.previous-{stamp}-{suffix:04}")
        };
        let backup = parent.join(name);
        if backup.exists() {
            continue;
        }
        return fs::rename(latest, backup);
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a legacy latest backup path",
    ))
}

fn sanitize_label(label: &str) -> String {
    let value = label
        .chars()
        .take(80)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let value = value.trim_matches('-');
    if value.is_empty() {
        "event".to_string()
    } else {
        value.to_string()
    }
}

fn bounded_detail(detail: &str) -> String {
    detail.chars().take(512).collect()
}

fn panic_reason(payload: &(dyn std::any::Any + Send)) -> String {
    payload.downcast_ref::<String>().map_or_else(
        || {
            payload
                .downcast_ref::<&str>()
                .map_or_else(|| "unknown panic".to_string(), |value| (*value).to_string())
        },
        Clone::clone,
    )
}

fn deduplicate_tabs(tabs: &[Tab]) -> Vec<Tab> {
    let mut result = Vec::with_capacity(tabs.len());
    for tab in tabs {
        if !result.contains(tab) {
            result.push(*tab);
        }
    }
    if result.is_empty() {
        result.push(Tab::Dashboard);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::tui_bridge::TuiBridge;
    use roko_core::dashboard_snapshot::DashboardEvent;
    use roko_runtime::StateHub;
    use tempfile::tempdir;

    fn config(workdir: &Path) -> ScreenshotCollectorConfig {
        let mut config =
            ScreenshotCollectorConfig::for_plan_run(workdir, None, Duration::from_secs(60));
        config.width = 80;
        config.height = 24;
        config.min_free_disk_mb = 0;
        config
    }

    fn read_manifest(path: &Path) -> Manifest {
        serde_json::from_slice(&fs::read(path.join("manifest.json")).unwrap()).unwrap()
    }

    fn wait_for_captures(path: &Path, count: usize) -> Manifest {
        // Full-App renders are intentionally exercised here. When Cargo runs
        // this module in parallel, the renderer workers can contend for CPU
        // and filesystem syncs; keep the assertion bounded without turning
        // ordinary suite parallelism into a false product failure.
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let manifest = read_manifest(path);
            if manifest.captures.len() >= count {
                return manifest;
            }
            assert!(Instant::now() < deadline, "timed out waiting for captures");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn captures_live_statehub_frames_and_durable_manifest() {
        let dir = tempdir().unwrap();
        let hub = StateHub::default_capacity();
        let collector = ScreenshotCollector::start(config(dir.path()), hub.snapshot()).unwrap();
        let run_dir = collector.run_dir().to_path_buf();

        collector.capture_startup();
        hub.sender().publish(DashboardEvent::PlanStarted {
            plan_id: "visible-plan".to_string(),
            tasks_total: 3,
        });
        assert!(collector.capture_event(
            "plan_started",
            Some("visible-plan".to_string()),
            &[Tab::Dashboard, Tab::Plans],
        ));

        let manifest = wait_for_captures(&run_dir, 2);
        assert_eq!(manifest.renderer, "app.draw/full-frame/dashboard-snapshot");
        assert_eq!(manifest.captures[1].status, "captured");
        assert_eq!(manifest.captures[1].files.len(), 2);
        let plan_frame = fs::read_to_string(
            run_dir
                .join(manifest.captures[1].path.as_ref().unwrap())
                .join("f02-plans.txt"),
        )
        .unwrap();
        assert!(plan_frame.contains("visible-plan"));

        drop(collector);
        let manifest = read_manifest(&run_dir);
        assert_eq!(manifest.captures.last().unwrap().label, "shutdown");
        assert_eq!(
            manifest.captures.last().unwrap().files.len(),
            Tab::ALL.len()
        );
    }

    #[test]
    fn periodic_capture_uses_live_snapshot_without_runner_activity() {
        let dir = tempdir().unwrap();
        let hub = StateHub::default_capacity();
        let mut cfg = config(dir.path());
        cfg.interval = Duration::from_millis(50);
        let collector = ScreenshotCollector::start(cfg, hub.snapshot()).unwrap();
        let run_dir = collector.run_dir().to_path_buf();

        let manifest = wait_for_captures(&run_dir, 1);
        assert_eq!(manifest.captures[0].label, "interval");
        assert_eq!(manifest.captures[0].files.len(), Tab::ALL.len());
        drop(collector);
    }

    #[test]
    fn startup_status_capture_proves_cache_warm_visibility() {
        let dir = tempdir().unwrap();
        let hub = StateHub::default_capacity();
        let collector = ScreenshotCollector::start(config(dir.path()), hub.snapshot()).unwrap();
        let run_dir = collector.run_dir().to_path_buf();
        let bridge = TuiBridge::new(hub.sender()).with_screenshot_collector(collector.clone());

        bridge.status(
            "startup.cache_warm.started",
            "warming Cargo workspace cache",
        );
        let manifest = wait_for_captures(&run_dir, 1);
        assert_eq!(manifest.captures[0].label, "startup-cache_warm-started");
        assert_eq!(manifest.captures[0].tabs, vec!["F1 Dashboard", "F5 Logs"]);

        drop(bridge);
        drop(collector);
    }

    #[test]
    fn typed_agent_capture_contains_the_published_agent_state() {
        let dir = tempdir().unwrap();
        let hub = StateHub::default_capacity();
        let collector = ScreenshotCollector::start(config(dir.path()), hub.snapshot()).unwrap();
        let run_dir = collector.run_dir().to_path_buf();
        let bridge = TuiBridge::new(hub.sender()).with_screenshot_collector(collector.clone());

        bridge.agent_spawned(
            "agent-visible",
            "plan-visible",
            "task-visible",
            1,
            "implementer",
            "test-model",
        );
        let manifest = wait_for_captures(&run_dir, 1);
        assert_eq!(manifest.captures[0].label, "agent_spawned");
        let agents_frame = fs::read_to_string(
            run_dir
                .join(manifest.captures[0].path.as_ref().unwrap())
                .join("f03-agents.txt"),
        )
        .unwrap();
        assert!(agents_frame.contains("agent-visible"));

        drop(bridge);
        drop(collector);
    }

    #[test]
    fn low_disk_is_recorded_as_a_skip_not_an_error() {
        let dir = tempdir().unwrap();
        let hub = StateHub::default_capacity();
        let mut cfg = config(dir.path());
        cfg.min_free_disk_mb = u64::MAX;
        let collector = ScreenshotCollector::start(cfg, hub.snapshot()).unwrap();
        let run_dir = collector.run_dir().to_path_buf();
        collector.capture_startup();

        let manifest = wait_for_captures(&run_dir, 1);
        assert_eq!(manifest.captures[0].status, "skipped");
        assert!(
            manifest.captures[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("low disk")
        );
        drop(collector);
    }

    #[test]
    fn dimensions_and_interval_are_validated() {
        let dir = tempdir().unwrap();
        let hub = StateHub::default_capacity();
        let mut cfg = config(dir.path());
        cfg.width = MIN_WIDTH - 1;
        assert!(ScreenshotCollector::start(cfg, hub.snapshot()).is_err());

        let mut cfg = config(dir.path());
        cfg.interval = Duration::ZERO;
        assert!(ScreenshotCollector::start(cfg, hub.snapshot()).is_err());
    }

    #[test]
    fn explicit_run_directory_is_collision_safe() {
        let dir = tempdir().unwrap();
        let requested = dir.path().join("evidence");
        fs::create_dir(&requested).unwrap();
        let hub = StateHub::default_capacity();
        let mut cfg = config(dir.path());
        cfg.run_dir = Some(requested.clone());
        let collector = ScreenshotCollector::start(cfg, hub.snapshot()).unwrap();
        assert_ne!(collector.run_dir(), requested);
        assert!(collector.run_dir().ends_with("evidence-0001"));
        drop(collector);
    }

    #[cfg(unix)]
    #[test]
    fn latest_symlink_is_atomically_replaced() {
        let dir = tempdir().unwrap();
        let hub = StateHub::default_capacity();
        let legacy_latest = dir.path().join(".roko/screenshots/latest");
        fs::create_dir_all(&legacy_latest).unwrap();
        fs::write(legacy_latest.join("legacy-frame.txt"), "evidence").unwrap();
        let first = ScreenshotCollector::start(config(dir.path()), hub.snapshot()).unwrap();
        let first_dir = first.run_dir().to_path_buf();
        let latest = dir.path().join(".roko/screenshots/latest");
        assert_eq!(
            fs::canonicalize(&latest).unwrap(),
            fs::canonicalize(&first_dir).unwrap()
        );
        let preserved = fs::read_dir(dir.path().join(".roko/screenshots"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("latest.previous-"))
            })
            .expect("legacy latest directory should be preserved");
        assert_eq!(
            fs::read_to_string(preserved.join("legacy-frame.txt")).unwrap(),
            "evidence"
        );
        drop(first);

        let second = ScreenshotCollector::start(config(dir.path()), hub.snapshot()).unwrap();
        assert_eq!(
            fs::canonicalize(&latest).unwrap(),
            fs::canonicalize(second.run_dir()).unwrap()
        );
        drop(second);
    }
}
