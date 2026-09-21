use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, UNIX_EPOCH};

use anyhow::{Context, Result};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{
    DebounceEventHandler, DebounceEventResult, Debouncer, FileIdMap, new_debouncer_opt,
};

const DEBOUNCE_WINDOW: Duration = Duration::from_millis(200);
/// Debounce window for source-file changes before triggering an index rebuild.
/// 2 seconds gives a burst of saves (e.g. fmt-on-save) time to settle.
const SOURCE_INDEX_DEBOUNCE: Duration = Duration::from_secs(2);
const FALLBACK_POLL_INTERVAL: Duration = Duration::from_secs(1);
const CHANNEL_BOUND: usize = 4;
const RECURSIVE_WATCH_DIRS: &[&str] = &["state", "plans", "gates"];
const SHALLOW_WATCH_DIRS: &[&str] = &["learn"];
const EXCLUDED_TREE_NAMES: &[&str] =
    &["worktrees", "reflex-replays", "target", "cache", "archives"];

type NotifyDebouncer = Debouncer<notify::RecommendedWatcher, FileIdMap>;

/// Coalesced filesystem refresh signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsRefresh {
    /// One or more filesystem events landed in a debounce window.
    Coalesced,
}

/// Keeps the filesystem watcher alive for the app lifetime.
pub struct FsWatchHandle {
    /// Receiver side for refresh notifications.
    pub rx: Receiver<FsRefresh>,
    _backend: FsWatchBackend,
}

// RAII guard: variants hold resources (debouncer, poll thread) that are
// cleaned up on Drop. Stored in `_backend` and never matched on.
#[allow(dead_code)]
enum FsWatchBackend {
    Notify(NotifyDebouncer),
    Poll(PollerHandle),
}

struct PollerHandle {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl Drop for PollerHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            if let Err(err) = join.join() {
                tracing::warn!(?err, "failed to join .roko poll fallback thread");
            }
        }
    }
}

struct RefreshHandler {
    tx: SyncSender<FsRefresh>,
}

impl DebounceEventHandler for RefreshHandler {
    fn handle_event(&mut self, event: DebounceEventResult) {
        match event {
            Ok(events) => {
                if !events.is_empty() {
                    let _ = self.tx.try_send(FsRefresh::Coalesced);
                }
            }
            Err(errors) => {
                tracing::warn!(count = errors.len(), "filesystem watcher debounce error");
                let _ = self.tx.try_send(FsRefresh::Coalesced);
            }
        }
    }
}

impl FsWatchHandle {
    /// Try to read the next refresh signal without blocking.
    #[must_use]
    pub fn try_recv(&self) -> std::result::Result<FsRefresh, TryRecvError> {
        self.rx.try_recv()
    }
}

/// Watch `.roko/` recursively and coalesce bursts into one refresh event.
pub fn watch_roko_dir(workdir: &Path) -> Result<FsWatchHandle> {
    let roko_dir = prepare_roko_dir(workdir)?;
    let (tx, rx) = mpsc::sync_channel(CHANNEL_BOUND);
    let handler = RefreshHandler { tx };

    let mut debouncer: NotifyDebouncer = new_debouncer_opt(
        DEBOUNCE_WINDOW,
        None,
        handler,
        FileIdMap::new(),
        notify::Config::default(),
    )
    .context("failed to create debounced filesystem watcher")?;
    // Root-level lifecycle files are enough for connected/disk snapshots.
    // Explicitly avoid recursively subscribing to attempt worktrees and build
    // outputs, which can contain tens of thousands of high-churn files.
    debouncer
        .watcher()
        .watch(&roko_dir, RecursiveMode::NonRecursive)
        .with_context(|| format!("failed to watch {}", roko_dir.display()))?;
    debouncer
        .cache()
        .add_root(&roko_dir, RecursiveMode::NonRecursive);
    for child in RECURSIVE_WATCH_DIRS {
        let path = roko_dir.join(child);
        if !path.is_dir() {
            continue;
        }
        debouncer
            .watcher()
            .watch(&path, RecursiveMode::Recursive)
            .with_context(|| format!("failed to watch {}", path.display()))?;
        debouncer.cache().add_root(&path, RecursiveMode::Recursive);
    }
    for child in SHALLOW_WATCH_DIRS {
        let path = roko_dir.join(child);
        if !path.is_dir() {
            continue;
        }
        debouncer
            .watcher()
            .watch(&path, RecursiveMode::NonRecursive)
            .with_context(|| format!("failed to watch {}", path.display()))?;
        debouncer
            .cache()
            .add_root(&path, RecursiveMode::NonRecursive);
    }

    Ok(FsWatchHandle {
        rx,
        _backend: FsWatchBackend::Notify(debouncer),
    })
}

/// Watch `.roko/` and fall back to a 1 s poll if notify cannot initialize.
pub fn watch_roko_dir_with_fallback(workdir: &Path) -> FsWatchHandle {
    match watch_roko_dir(workdir) {
        Ok(handle) => handle,
        Err(error) => {
            tracing::warn!(
                error = %error,
                cadence = ?FALLBACK_POLL_INTERVAL,
                "notify unavailable; falling back to poll watcher"
            );
            spawn_poll_fallback_with_interval(workdir.to_path_buf(), FALLBACK_POLL_INTERVAL)
        }
    }
}

/// Watch `<workdir>/crates/*/src/` for `.rs` file changes and rebuild the
/// workspace code index automatically.
///
/// Fires a 2-second debounce window on every `.rs` change to coalesce burst
/// saves. The rebuild runs in a background thread so it never blocks the
/// caller. Failures are logged as warnings but do not stop the watcher.
///
/// Returns a [`FsWatchHandle`] whose receiver delivers a [`FsRefresh::Coalesced`]
/// notification *after* each completed (or failed) rebuild attempt. The caller
/// can use this to refresh any in-memory index caches.
///
/// If `notify` is unavailable the watcher silently skips source watching and
/// returns a handle that never delivers events.
pub fn watch_source_dirs_with_index_rebuild(workdir: &Path) -> FsWatchHandle {
    let crates_dir = workdir.join("crates");
    if !crates_dir.is_dir() {
        tracing::debug!(
            path = %crates_dir.display(),
            "crates/ directory not found; skipping source-dir index watcher"
        );
        // Return a no-op handle: channel that is never sent to.
        let (_, rx) = mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        return FsWatchHandle {
            rx,
            _backend: FsWatchBackend::Poll(PollerHandle { stop, join: None }),
        };
    }

    let workdir_owned = workdir.to_path_buf();
    let (tx, rx) = mpsc::sync_channel::<FsRefresh>(CHANNEL_BOUND);

    // Build the debounced handler that fires the rebuild.
    let rebuild_tx = tx.clone();
    let rebuild_workdir = workdir_owned.clone();

    struct SourceChangeHandler {
        tx: SyncSender<FsRefresh>,
        workdir: PathBuf,
    }

    impl DebounceEventHandler for SourceChangeHandler {
        fn handle_event(&mut self, result: DebounceEventResult) {
            // Only act on events that involve at least one .rs file.
            let has_rs = match &result {
                Ok(events) => events.iter().any(|ev| {
                    ev.paths
                        .iter()
                        .any(|p| p.extension().is_some_and(|ext| ext == "rs"))
                }),
                Err(_) => true, // errors are treated conservatively
            };
            if !has_rs {
                return;
            }

            if let Err(e) = &result {
                tracing::warn!(
                    count = e.len(),
                    "source-dir watcher debounce error; triggering rebuild anyway"
                );
            }

            // Run the rebuild in a dedicated thread to avoid blocking the
            // debouncer's internal timer thread.
            let workdir = self.workdir.clone();
            let tx = self.tx.clone();
            let _ = thread::Builder::new()
                .name("index-rebuild-on-src-change".into())
                .spawn(move || {
                    rebuild_index_from_workdir(&workdir);
                    let _ = tx.try_send(FsRefresh::Coalesced);
                });
        }
    }

    let handler = SourceChangeHandler {
        tx: rebuild_tx,
        workdir: rebuild_workdir,
    };

    let debouncer_result: Result<NotifyDebouncer> = new_debouncer_opt(
        SOURCE_INDEX_DEBOUNCE,
        None,
        handler,
        FileIdMap::new(),
        notify::Config::default(),
    )
    .context("failed to create source-dir debounced watcher");

    match debouncer_result {
        Err(e) => {
            tracing::warn!(
                error = %e,
                "notify unavailable for source-dir index watcher; index will not auto-rebuild"
            );
            // Return a no-op handle.
            let stop = Arc::new(AtomicBool::new(false));
            FsWatchHandle {
                rx,
                _backend: FsWatchBackend::Poll(PollerHandle { stop, join: None }),
            }
        }
        Ok(mut debouncer) => {
            // Enumerate `crates/*/src/` directories and watch each recursively.
            let mut watched = 0usize;
            if let Ok(entries) = fs::read_dir(&crates_dir) {
                for entry in entries.flatten() {
                    let src_dir = entry.path().join("src");
                    if !src_dir.is_dir() {
                        continue;
                    }
                    match debouncer
                        .watcher()
                        .watch(&src_dir, RecursiveMode::Recursive)
                    {
                        Ok(()) => {
                            debouncer
                                .cache()
                                .add_root(&src_dir, RecursiveMode::Recursive);
                            watched += 1;
                        }
                        Err(e) => {
                            tracing::warn!(
                                path = %src_dir.display(),
                                error = %e,
                                "failed to watch crate src/ directory"
                            );
                        }
                    }
                }
            }
            tracing::debug!(
                crates_watched = watched,
                "source-dir index watcher started"
            );
            FsWatchHandle {
                rx,
                _backend: FsWatchBackend::Notify(debouncer),
            }
        }
    }
}

/// Rebuild the persistent code index for `workdir`.
///
/// Called from the background rebuild thread spawned by the source-dir
/// watcher.  Errors are logged as warnings; a failed rebuild leaves the
/// previous (stale) DB in place.
fn rebuild_index_from_workdir(workdir: &Path) {
    tracing::info!(
        workdir = %workdir.display(),
        "rebuilding code index after source change"
    );

    // Load in-memory workspace index.
    let idx = match roko_index::WorkspaceIndex::load(workdir) {
        Ok(idx) => idx,
        Err(e) => {
            tracing::warn!(error = %e, "index rebuild: WorkspaceIndex::load failed");
            return;
        }
    };

    // Materialise file/ranking records.
    let file_records: Vec<roko_index::FileRecord> = idx
        .all_source_files()
        .iter()
        .map(|sf| roko_index::FileRecord {
            path: sf.path.clone(),
            content: sf.content.clone(),
        })
        .collect();
    let rankings: Vec<roko_index::RankingRecord> = idx
        .all_pagerank_scores()
        .iter()
        .map(|(id, &score)| roko_index::RankingRecord {
            id: id.clone(),
            score,
        })
        .collect();

    // Atomically replace the SQLite DB.
    match roko_index::IndexStore::build_with_rankings(
        idx.root(),
        &idx.all_symbols(),
        &idx.all_edges(),
        &file_records,
        &rankings,
    ) {
        Ok(store) => {
            let stats = idx.stats();
            tracing::info!(
                db = %store.db_path().display(),
                files = stats.indexed_files,
                symbols = stats.total_symbols,
                "code index rebuilt successfully"
            );
        }
        Err(e) => {
            tracing::warn!(error = %e, "index rebuild: IndexStore::build_with_rankings failed");
        }
    }
}

fn spawn_poll_fallback_with_interval(workdir: PathBuf, interval: Duration) -> FsWatchHandle {
    let roko_dir = workdir.join(".roko");
    if let Err(error) = fs::create_dir_all(&roko_dir) {
        tracing::warn!(
            error = %error,
            path = %roko_dir.display(),
            "could not create .roko before starting poll fallback"
        );
    }

    let (tx, rx) = mpsc::sync_channel(CHANNEL_BOUND);
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = Arc::clone(&stop);
    let poll_root = roko_dir.clone();
    let mut last_fingerprint = roko_dir_fingerprint(&poll_root);

    let join = thread::Builder::new()
        .name("tui-fs-poll-fallback".into())
        .spawn(move || {
            loop {
                if stop_thread.load(Ordering::Relaxed) {
                    break;
                }

                thread::sleep(interval);

                if stop_thread.load(Ordering::Relaxed) {
                    break;
                }

                let fingerprint = roko_dir_fingerprint(&poll_root);
                if fingerprint != last_fingerprint {
                    last_fingerprint = fingerprint;
                    match tx.try_send(FsRefresh::Coalesced) {
                        Ok(()) | Err(mpsc::TrySendError::Full(_)) => {}
                        Err(mpsc::TrySendError::Disconnected(_)) => break,
                    }
                }
            }
        })
        .unwrap_or_else(|error| {
            tracing::warn!(
                error = %error,
                thread = "tui-fs-poll-fallback",
                "failed to spawn fallback filesystem poller"
            );
            thread::spawn(|| {})
        });

    FsWatchHandle {
        rx,
        _backend: FsWatchBackend::Poll(PollerHandle {
            stop,
            join: Some(join),
        }),
    }
}

fn prepare_roko_dir(workdir: &Path) -> Result<PathBuf> {
    let roko_dir = workdir.join(".roko");
    fs::create_dir_all(&roko_dir)
        .with_context(|| format!("failed to create {}", roko_dir.display()))?;
    Ok(roko_dir)
}

fn roko_dir_fingerprint(path: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_path(path, &mut hasher);
    hasher.finish()
}

fn hash_path(path: &Path, hasher: &mut DefaultHasher) {
    path.hash(hasher);

    let Ok(metadata) = fs::metadata(path) else {
        false.hash(hasher);
        return;
    };

    metadata.is_dir().hash(hasher);
    metadata.len().hash(hasher);
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .hash(hasher);

    if !metadata.is_dir() {
        return;
    }

    let Ok(entries) = fs::read_dir(path) else {
        false.hash(hasher);
        return;
    };

    let mut children: Vec<_> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    children.sort();
    for child in children {
        if child
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| EXCLUDED_TREE_NAMES.contains(&name))
        {
            continue;
        }
        hash_path(&child, hasher);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_roko_dir_emits_refresh_within_500ms() {
        let tempdir = tempfile::tempdir().unwrap();
        let handle = spawn_poll_fallback_with_interval(
            tempdir.path().to_path_buf(),
            Duration::from_millis(50),
        );
        let target = tempdir.path().join(".roko").join("watch-trigger.json");
        std::fs::write(&target, br#"{"ok":true}"#).unwrap();

        let refresh = handle.rx.recv_timeout(Duration::from_millis(500));
        assert_eq!(refresh.unwrap(), FsRefresh::Coalesced);
    }
}
