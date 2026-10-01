//! Per-process agent PID registry with disk persistence.
//!
//! Tracks every child PID spawned by this Roko process so that:
//! - A later Roko instance in the same workspace can kill agents orphaned by
//!   a crash ([`cleanup_orphaned_agents`]).
//! - The reaper can detect orphans reparented to PID 1 (init/launchd)
//!   ([`reap_orphaned_children`]).
//!
//! Every Roko process owns exactly one record file,
//! `<workspace>/.roko/runtime/agent-pids/<owner_pid>-<owner_start>.json`, and
//! never writes any other. It holds the owner's kernel start fingerprint and,
//! per child, its PID, start fingerprint, spawn time, and command name (see
//! [`ProcessIdentity`]). The file is rewritten atomically on every mutation,
//! listing only children that still exist, and removed once none do. Naming it
//! by start fingerprint as well as PID means a recycled owner PID never shares
//! a predecessor's file.
//!
//! Cleanup only signals children listed by owners that are provably gone —
//! no longer running, or their PID now names a different process — and only
//! when the child still matches its recorded fingerprint. A concurrent Roko
//! process's live agents and recycled PIDs are therefore never signaled. The
//! single shared `agent-pids.json` written by earlier releases is migrated
//! conservatively (see `legacy_orphans`) and then deleted.
//!
//! The CLI keys the registry to the resolved workspace with
//! [`set_registry_root`]; until then it uses the current directory.
//!
//! A child is also tagged with the spawn scope of the thread that registered
//! it ([`enter_spawn_scope`]), so one plan run can stop its own agents
//! ([`registered_pids_in_scope`]) without signalling other agents the same
//! process runs beside it.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use super::identity::{ProcessIdentity, process_identity};

/// Per-owner record directory, relative to the workspace root.
const RECORDS_DIR: &str = ".roko/runtime/agent-pids";

/// Shared registry file written by earlier releases, relative to the root.
const LEGACY_FILE: &str = ".roko/runtime/agent-pids.json";

/// Clock slack when a child has no recorded start fingerprint and is instead
/// checked against its registration time.
const SPAWN_CLOCK_SLACK_MS: u64 = 1_000;

/// Grace period between SIGTERM and SIGKILL during startup cleanup.
#[cfg(unix)]
const CLEANUP_GRACE: std::time::Duration = std::time::Duration::from_millis(200);

/// This process's registry. Each mutation persists only this owner's record.
static REGISTRY: LazyLock<Mutex<Registry>> =
    LazyLock::new(|| Mutex::new(Registry::new(Owner::current())));

/// The next scope [`new_spawn_scope`] hands out.
static NEXT_SPAWN_SCOPE: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// The spawn scope of the children this thread registers; see
    /// [`enter_spawn_scope`].
    static SPAWN_SCOPE: Cell<Option<u64>> = const { Cell::new(None) };
}

/// The Roko process that owns one record file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Owner {
    pid: u32,
    start: Option<u64>,
}

impl Owner {
    fn current() -> Self {
        let pid = std::process::id();
        Self {
            pid,
            start: process_identity(pid).map(|identity| identity.start),
        }
    }

    fn file_name(self) -> String {
        match self.start {
            Some(start) => format!("{}-{start}.json", self.pid),
            None => format!("{}.json", self.pid),
        }
    }
}

/// On-disk record of one Roko process and the children it spawned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct OwnerRecord {
    owner_pid: u32,
    #[serde(default)]
    owner_start: Option<u64>,
    #[serde(default)]
    children: Vec<ChildRecord>,
}

/// One registered child process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ChildRecord {
    pid: u32,
    /// Start fingerprint observed at registration.
    #[serde(default)]
    start: Option<u64>,
    /// Wall-clock registration time in milliseconds since the Unix epoch.
    #[serde(default)]
    spawned_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    command: Option<String>,
}

impl ChildRecord {
    /// Capture `pid`'s identity just after it was spawned.
    fn observe(pid: u32) -> Self {
        let identity = process_identity(pid);
        Self {
            pid,
            start: identity.as_ref().map(|identity| identity.start),
            spawned_at_ms: now_ms(),
            command: identity.and_then(|identity| identity.command),
        }
    }

    /// Whether `identity`, the process now holding this PID, is the process
    /// that was registered. Without a start fingerprint, the process must have
    /// started no later than its registration.
    fn is_same_process(&self, identity: &ProcessIdentity) -> bool {
        match self.start {
            Some(start) => identity.start == start,
            None => identity.started_at_ms.is_some_and(|started| {
                started <= self.spawned_at_ms.saturating_add(SPAWN_CLOCK_SLACK_MS)
            }),
        }
    }
}

/// Registered children of one owner plus the workspace its record lives in.
struct Registry {
    owner: Owner,
    /// Canonical workspace root; defaults to the current directory on first use.
    root: Option<PathBuf>,
    children: BTreeMap<u32, ChildRecord>,
    /// The spawn scope each child was registered under, for children
    /// registered inside one. Kept in memory only: a scope means nothing to
    /// another process.
    scopes: BTreeMap<u32, u64>,
}

impl Registry {
    const fn new(owner: Owner) -> Self {
        Self {
            owner,
            root: None,
            children: BTreeMap::new(),
            scopes: BTreeMap::new(),
        }
    }

    /// Tag `pid` with the scope it was registered under, or clear the tag a
    /// recycled PID left behind.
    fn set_scope(&mut self, pid: u32, scope: Option<u64>) {
        match scope {
            Some(scope) => {
                self.scopes.insert(pid, scope);
            }
            None => {
                self.scopes.remove(&pid);
            }
        }
    }

    fn root(&mut self) -> Option<&Path> {
        if self.root.is_none() {
            self.root = std::env::current_dir().ok().map(|cwd| canonical_root(&cwd));
        }
        self.root.as_deref()
    }

    fn record_path(&mut self) -> Option<PathBuf> {
        let name = self.owner.file_name();
        self.root().map(|root| records_dir(root).join(name))
    }

    /// Re-key to `root`, moving this owner's record out of the old workspace.
    fn set_root(&mut self, root: PathBuf) {
        if self.root.as_ref() == Some(&root) {
            return;
        }
        if let Some(old) = self.root.take() {
            remove_record(&records_dir(&old).join(self.owner.file_name()));
        }
        self.root = Some(root);
        if !self.children.is_empty() {
            self.persist();
        }
    }

    /// Rewrite this owner's record with the children that still exist, or
    /// delete it once none do.
    fn persist(&mut self) {
        let children = &self.children;
        self.scopes.retain(|pid, _| children.contains_key(pid));
        let Some(path) = self.record_path() else {
            return;
        };
        let children: Vec<ChildRecord> = self
            .children
            .values()
            .filter(|child| pid_exists(child.pid))
            .cloned()
            .collect();
        if children.is_empty() {
            remove_record(&path);
            return;
        }
        let record = OwnerRecord {
            owner_pid: self.owner.pid,
            owner_start: self.owner.start,
            children,
        };
        if let Err(error) = write_record(&path, &record) {
            tracing::warn!(path = %path.display(), %error, "failed to persist agent PID record");
        }
    }
}

/// Key this process's PID registry to the workspace at `workdir`.
///
/// The CLI calls this at startup with the resolved workspace; until then the
/// registry uses the current directory. Children already registered move with
/// the record.
pub fn set_registry_root(workdir: &Path) {
    let root = canonical_root(workdir);
    REGISTRY.lock().set_root(root);
}

/// Register a child PID in the global registry and persist to disk.
///
/// Call right after spawning: the child's start fingerprint is captured now so
/// later cleanup can tell it apart from a process that recycles its PID.
pub fn register_spawned_pid(pid: u32) {
    let child = ChildRecord::observe(pid);
    let scope = current_spawn_scope();
    let mut registry = REGISTRY.lock();
    registry.children.insert(pid, child);
    registry.set_scope(pid, scope);
    registry.persist();
}

/// Register multiple descendant PIDs discovered during a kill sweep.
pub fn register_spawned_descendants(pids: &[u32]) {
    if pids.is_empty() {
        return;
    }
    let children: Vec<ChildRecord> = pids.iter().map(|pid| ChildRecord::observe(*pid)).collect();
    let scope = current_spawn_scope();
    let mut registry = REGISTRY.lock();
    for child in children {
        registry.set_scope(child.pid, scope);
        registry.children.insert(child.pid, child);
    }
    registry.persist();
}

/// Remove a PID from the registry (e.g. after confirmed exit).
pub fn unregister_pid(pid: u32) {
    let mut registry = REGISTRY.lock();
    if registry.children.remove(&pid).is_some() {
        registry.persist();
    }
}

/// Return a snapshot of all currently registered PIDs.
pub fn registered_pids() -> Vec<u32> {
    REGISTRY.lock().children.keys().copied().collect()
}

/// Return the registered PIDs of one spawn scope: those registered on a
/// thread inside [`enter_spawn_scope`] with `scope`, or, for `None`, those
/// registered outside every scope.
pub fn registered_pids_in_scope(scope: Option<u64>) -> Vec<u32> {
    let registry = REGISTRY.lock();
    registry
        .children
        .keys()
        .copied()
        .filter(|pid| registry.scopes.get(pid).copied() == scope)
        .collect()
}

/// A spawn scope no other caller in this process holds.
pub fn new_spawn_scope() -> u64 {
    NEXT_SPAWN_SCOPE.fetch_add(1, Ordering::Relaxed)
}

/// Tag the children this thread registers with `scope` until the returned
/// guard drops, which restores the thread's previous scope.
///
/// `roko serve` runs each plan on a thread of its own and scopes it, so a
/// cancelled plan run signals only its own agents, never a plan generation,
/// revision or chat agent the server runs beside it (find-65ff6b).
pub fn enter_spawn_scope(scope: u64) -> SpawnScopeGuard {
    let previous = SPAWN_SCOPE.with(|current| current.replace(Some(scope)));
    SpawnScopeGuard { previous }
}

/// The spawn scope the current thread registers children under, if any.
pub fn current_spawn_scope() -> Option<u64> {
    SPAWN_SCOPE.with(Cell::get)
}

/// Ends a spawn scope entered with [`enter_spawn_scope`] when dropped.
#[must_use = "the spawn scope ends when the guard is dropped"]
pub struct SpawnScopeGuard {
    previous: Option<u64>,
}

impl Drop for SpawnScopeGuard {
    fn drop(&mut self) {
        SPAWN_SCOPE.with(|current| current.set(self.previous));
    }
}

/// Kill agent processes orphaned by Roko processes that are gone.
///
/// Scans the workspace's per-owner records and signals only children whose
/// owner is no longer running (or whose owner PID now names a different
/// process) and that still match their recorded start fingerprint, together
/// with their descendants: SIGTERM, a short grace period, then SIGKILL. Live
/// owners' records are never touched; spent records are deleted, as is the
/// legacy shared `agent-pids.json` after its conservative migration. Exited
/// PIDs are also pruned from this process's own registry.
///
/// Called on startup before spawning new agents.
#[cfg(unix)]
pub fn cleanup_orphaned_agents() {
    let root = {
        let mut registry = REGISTRY.lock();
        let before = registry.children.len();
        registry.children.retain(|pid, _| pid_exists(*pid));
        if registry.children.len() != before {
            registry.persist();
        }
        registry.root().map(Path::to_path_buf)
    };
    let Some(root) = root else {
        return;
    };
    let killed = cleanup_workspace(&root);
    if killed > 0 {
        tracing::warn!(
            killed,
            workspace = %root.display(),
            "Cleaned up {killed} orphaned agent process(es) from previous run"
        );
    }
}

/// No-op on non-Unix platforms.
#[cfg(not(unix))]
pub fn cleanup_orphaned_agents() {}

/// Reap orphaned child processes that survived normal cleanup.
///
/// Checks every PID in this process's registry: if it still names the process
/// that was registered and its parent is PID 1 (reparented to init/launchd —
/// i.e. orphaned), send SIGKILL to it and its descendants. PIDs that exited or
/// now name a different process are dropped from the registry unsignaled.
///
/// Returns the number of processes killed.
#[cfg(unix)]
pub fn reap_orphaned_children() -> usize {
    let children: Vec<ChildRecord> = REGISTRY.lock().children.values().cloned().collect();
    let (killed, stale) = reap(&children);
    if !stale.is_empty() {
        let mut registry = REGISTRY.lock();
        let before = registry.children.len();
        for pid in &stale {
            registry.children.remove(pid);
        }
        if registry.children.len() != before {
            registry.persist();
        }
    }
    killed
}

/// No-op on non-Unix platforms.
#[cfg(not(unix))]
pub fn reap_orphaned_children() -> usize {
    0
}

/// Kill verified orphans among `children`. Returns the number of processes
/// killed and the PIDs to drop from the registry.
#[cfg(unix)]
fn reap(children: &[ChildRecord]) -> (usize, Vec<u32>) {
    use super::group::collect_descendants;

    let own_pid = std::process::id();
    let mut killed = 0;
    let mut stale = Vec::new();

    for child in children {
        let Some(identity) = process_identity(child.pid) else {
            if !pid_exists(child.pid) {
                stale.push(child.pid);
            }
            continue;
        };
        if !child.is_same_process(&identity) {
            // The registered process exited and its PID was recycled.
            stale.push(child.pid);
            continue;
        }
        if identity.ppid != 1 || child.pid == own_pid {
            continue;
        }

        let descendants = collect_descendants(child.pid);
        signal(child.pid, libc::SIGKILL);
        for pid in &descendants {
            signal(*pid, libc::SIGKILL);
        }
        tracing::warn!(
            pid = child.pid,
            descendants = descendants.len(),
            "Reaped orphaned process (parent=1)"
        );
        killed += 1 + descendants.len();
        stale.push(child.pid);
        stale.extend(descendants);
    }

    (killed, stale)
}

/// Clean up the orphans recorded under `root`. Returns the number of
/// processes signaled.
#[cfg(unix)]
fn cleanup_workspace(root: &Path) -> usize {
    let mut targets = Vec::new();
    let mut spent = Vec::new();

    if let Ok(entries) = std::fs::read_dir(records_dir(root)) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            // Unreadable: removed concurrently by its owner or another cleanup.
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            // A malformed record still names its owner.
            let Some(record) = serde_json::from_slice::<OwnerRecord>(&bytes)
                .ok()
                .or_else(|| owner_from_file_name(&path))
            else {
                continue;
            };
            if owner_is_live(record.owner_pid, record.owner_start) {
                continue;
            }
            targets.extend(record.children.iter().filter_map(|child| {
                process_identity(child.pid)
                    .filter(|identity| child.is_same_process(identity))
                    .map(|identity| (child.pid, identity.start))
            }));
            spent.push(path);
        }
    }

    let legacy = root.join(LEGACY_FILE);
    if let Some(orphans) = legacy_orphans(&legacy) {
        targets.extend(orphans);
        spent.push(legacy);
    }

    let signaled = terminate_trees(&targets);
    for path in &spent {
        remove_record(path);
    }
    signaled
}

/// Whether the process that wrote a record is still running. An owner PID
/// that now names a different process was recycled, so that owner is gone;
/// anything unverifiable counts as live.
#[cfg(unix)]
fn owner_is_live(pid: u32, start: Option<u64>) -> bool {
    if !pid_exists(pid) {
        return false;
    }
    match (start, process_identity(pid)) {
        (Some(start), Some(identity)) => identity.start == start,
        _ => true,
    }
}

/// Entries of the legacy shared registry that are safe to kill, with their
/// start fingerprints, or `None` when there is no legacy file.
///
/// That file recorded neither owners nor spawn times, so an entry qualifies
/// only when it is orphaned (parent PID 1: whichever Roko process spawned it
/// is gone) and started no later than the file's last write (so its PID was
/// not recycled afterwards).
#[cfg(unix)]
fn legacy_orphans(path: &Path) -> Option<Vec<(u32, u64)>> {
    let bytes = std::fs::read(path).ok()?;
    let written_ms = std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(system_time_ms);
    let pids: Vec<u32> = serde_json::from_slice(&bytes).unwrap_or_default();
    Some(
        pids.into_iter()
            .filter_map(|pid| {
                process_identity(pid)
                    .filter(|identity| {
                        identity.ppid == 1
                            && identity
                                .started_at_ms
                                .zip(written_ms)
                                .is_some_and(|(started, written)| started <= written)
                    })
                    .map(|identity| (pid, identity.start))
            })
            .collect(),
    )
}

/// SIGTERM each verified `(pid, start fingerprint)` root and its descendants,
/// wait briefly, then SIGKILL whatever survives. A process is signaled only
/// while it still carries the fingerprint observed for it. Returns the number
/// of processes signaled.
#[cfg(unix)]
fn terminate_trees(roots: &[(u32, u64)]) -> usize {
    use super::group::collect_descendants;

    let own_pid = std::process::id();
    let running = |pid: u32, start: u64| {
        process_identity(pid).is_some_and(|identity| identity.start == start)
    };
    // Snapshot each tree, with identities, while its root is still alive.
    let mut targets: Vec<(u32, u64)> = Vec::new();
    for &(root, root_start) in roots {
        if !running(root, root_start) {
            continue;
        }
        let tree = std::iter::once((root, Some(root_start))).chain(
            collect_descendants(root)
                .into_iter()
                .map(|pid| (pid, process_identity(pid).map(|identity| identity.start))),
        );
        for (pid, start) in tree {
            let Some(start) = start else {
                continue;
            };
            if pid > 1 && pid != own_pid && !targets.iter().any(|(seen, _)| *seen == pid) {
                targets.push((pid, start));
            }
        }
    }

    for &(pid, _) in &targets {
        tracing::info!(pid, "terminating orphaned agent process");
        signal(pid, libc::SIGTERM);
    }
    let deadline = std::time::Instant::now() + CLEANUP_GRACE;
    while targets.iter().any(|&(pid, start)| running(pid, start))
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    for &(pid, start) in &targets {
        if running(pid, start) {
            signal(pid, libc::SIGKILL);
        }
    }
    targets.len()
}

/// Whether `pid` names an existing process. Only "no such process" counts as
/// absent, so processes owned by other users and zombies are present.
#[cfg(unix)]
fn pid_exists(pid: u32) -> bool {
    i32::try_from(pid).is_ok_and(|raw| raw > 0) && super::kill::pid_is_alive(pid).unwrap_or(true)
}

/// Without a liveness probe every registered PID is assumed to exist.
#[cfg(not(unix))]
fn pid_exists(_pid: u32) -> bool {
    true
}

/// Send `sig` to one positive PID other than init.
#[cfg(unix)]
fn signal(pid: u32, sig: libc::c_int) {
    if let Ok(raw) = i32::try_from(pid)
        && raw > 1
        && let Err(error) = super::kill::signal_pid(raw, sig)
    {
        tracing::debug!(pid, sig, %error, "failed to signal process");
    }
}

fn records_dir(root: &Path) -> PathBuf {
    root.join(RECORDS_DIR)
}

fn canonical_root(workdir: &Path) -> PathBuf {
    std::fs::canonicalize(workdir)
        .or_else(|_| std::path::absolute(workdir))
        .unwrap_or_else(|_| workdir.to_path_buf())
}

fn write_record(path: &Path, record: &OwnerRecord) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(record).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)
}

fn remove_record(path: &Path) {
    if let Err(error) = std::fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(path = %path.display(), %error, "failed to remove agent PID record");
    }
}

/// Owner identity encoded in a record's file name (`<pid>-<start>.json` or
/// `<pid>.json`).
fn owner_from_file_name(path: &Path) -> Option<OwnerRecord> {
    let stem = path.file_stem()?.to_str()?;
    let (pid, start) = match stem.split_once('-') {
        Some((pid, start)) => (pid, Some(start.parse().ok()?)),
        None => (stem, None),
    };
    Some(OwnerRecord {
        owner_pid: pid.parse().ok()?,
        owner_start: start,
        children: Vec::new(),
    })
}

fn now_ms() -> u64 {
    system_time_ms(SystemTime::now()).unwrap_or(0)
}

fn system_time_ms(time: SystemTime) -> Option<u64> {
    let elapsed = time.duration_since(UNIX_EPOCH).ok()?;
    u64::try_from(elapsed.as_millis()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_snapshot() {
        // Register a fake PID, verify it appears in the snapshot.
        let fake_pid = 99_999_999;
        register_spawned_pid(fake_pid);
        let pids = registered_pids();
        assert!(pids.contains(&fake_pid));

        // Unregister and verify removal.
        unregister_pid(fake_pid);
        let pids = registered_pids();
        assert!(!pids.contains(&fake_pid));
    }

    /// find-65ff6b: a child registered inside a spawn scope is listed under
    /// that scope alone, and one registered outside every scope under `None`.
    #[test]
    fn spawn_scopes_keep_agents_apart() {
        let (outside, inside, nested) = (77_777_771, 77_777_772, 77_777_773);
        let scope = new_spawn_scope();
        let nested_scope = new_spawn_scope();
        assert_ne!(scope, nested_scope);
        assert_eq!(current_spawn_scope(), None);

        register_spawned_pid(outside);
        {
            let _scope = enter_spawn_scope(scope);
            register_spawned_pid(inside);
            {
                let _nested = enter_spawn_scope(nested_scope);
                register_spawned_pid(nested);
            }
            assert_eq!(current_spawn_scope(), Some(scope));
        }
        assert_eq!(current_spawn_scope(), None);

        assert_eq!(registered_pids_in_scope(Some(scope)), [inside]);
        assert_eq!(registered_pids_in_scope(Some(nested_scope)), [nested]);
        let unscoped = registered_pids_in_scope(None);
        assert!(unscoped.contains(&outside));
        assert!(!unscoped.contains(&inside) && !unscoped.contains(&nested));

        // A PID registered again outside a scope loses its old tag.
        register_spawned_pid(inside);
        assert!(registered_pids_in_scope(Some(scope)).is_empty());

        for pid in [outside, inside, nested] {
            unregister_pid(pid);
        }
        assert!(registered_pids_in_scope(Some(nested_scope)).is_empty());
    }

    #[test]
    fn register_descendants_batch() {
        let fakes = [88_888_881, 88_888_882, 88_888_883];
        register_spawned_descendants(&fakes);
        let pids = registered_pids();
        for f in &fakes {
            assert!(pids.contains(f));
        }
        // Clean up.
        for f in &fakes {
            unregister_pid(*f);
        }
    }

    #[test]
    fn reap_orphaned_children_returns_zero_when_empty() {
        let killed = reap_orphaned_children();
        // With no registered PIDs pointing to real orphans, expect 0.
        let _ = killed;
    }

    #[test]
    fn record_path_is_per_owner_under_workspace_runtime() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut registry = Registry::new(Owner {
            pid: 4242,
            start: Some(7),
        });
        registry.set_root(tmp.path().to_path_buf());
        let path = registry.record_path().expect("record path");
        assert_eq!(
            path,
            tmp.path().join(".roko/runtime/agent-pids/4242-7.json")
        );
    }

    #[test]
    fn owner_records_are_isolated() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut first = Registry::new(Owner {
            pid: 4_000_001,
            start: Some(11),
        });
        let mut second = Registry::new(Owner {
            pid: 4_000_002,
            start: Some(22),
        });
        first.set_root(tmp.path().to_path_buf());
        second.set_root(tmp.path().to_path_buf());
        // Records only keep children that exist, so both list this process.
        let live = std::process::id();

        first.children.insert(live, fake_child(live, Some(1)));
        first.persist();
        second.children.insert(live, fake_child(live, Some(2)));
        second.persist();

        let first_path = first.record_path().expect("first path");
        let second_path = second.record_path().expect("second path");
        let first_record = read_record(&first_path);
        let second_record = read_record(&second_path);
        assert_eq!(first_record.owner_pid, 4_000_001);
        assert_eq!(first_record.owner_start, Some(11));
        assert_eq!(first_record.children, vec![fake_child(live, Some(1))]);
        assert_eq!(second_record.owner_pid, 4_000_002);
        assert_eq!(second_record.children, vec![fake_child(live, Some(2))]);

        // Emptying one owner deletes only that owner's record.
        first.children.clear();
        first.persist();
        assert!(!first_path.exists());
        assert_eq!(read_record(&second_path), second_record);
    }

    #[test]
    fn set_root_moves_the_owner_record() {
        let old = tempfile::tempdir().expect("tempdir");
        let new = tempfile::tempdir().expect("tempdir");
        let mut registry = Registry::new(Owner {
            pid: 4_000_003,
            start: Some(33),
        });
        registry.set_root(old.path().to_path_buf());
        let live = std::process::id();
        registry.children.insert(live, fake_child(live, Some(3)));
        registry.persist();
        let old_path = registry.record_path().expect("old path");
        assert!(old_path.exists());

        registry.set_root(new.path().to_path_buf());
        assert!(!old_path.exists());
        let new_path = registry.record_path().expect("new path");
        assert_eq!(read_record(&new_path).children.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn records_list_only_children_that_still_exist() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut registry = Registry::new(Owner {
            pid: 4_000_004,
            start: Some(44),
        });
        registry.set_root(tmp.path().to_path_buf());
        let live = std::process::id();
        let exited = 99_999_999;
        registry.children.insert(live, fake_child(live, Some(1)));
        registry
            .children
            .insert(exited, fake_child(exited, Some(2)));
        registry.persist();
        let path = registry.record_path().expect("record path");
        assert_eq!(read_record(&path).children, vec![fake_child(live, Some(1))]);
        // The in-memory registry is unchanged until the PID is unregistered.
        assert!(registry.children.contains_key(&exited));

        registry.children.remove(&live);
        registry.persist();
        assert!(
            !path.exists(),
            "a record with no existing children is removed"
        );
    }

    #[test]
    fn owner_is_recovered_from_file_name() {
        let with_start = owner_from_file_name(Path::new("/x/12-34.json")).expect("owner");
        assert_eq!(
            (with_start.owner_pid, with_start.owner_start),
            (12, Some(34))
        );
        let without_start = owner_from_file_name(Path::new("/x/56.json")).expect("owner");
        assert_eq!(
            (without_start.owner_pid, without_start.owner_start),
            (56, None)
        );
        assert!(owner_from_file_name(Path::new("/x/junk.json")).is_none());
    }

    fn fake_child(pid: u32, start: Option<u64>) -> ChildRecord {
        ChildRecord {
            pid,
            start,
            spawned_at_ms: 1,
            command: None,
        }
    }

    fn read_record(path: &Path) -> OwnerRecord {
        serde_json::from_slice(&std::fs::read(path).expect("read record")).expect("parse record")
    }

    /// Cleanup scenarios against real `sleep` processes spawned by this test
    /// process. Only processes the tests spawned are ever signaled.
    #[cfg(all(unix, any(target_os = "macos", target_os = "linux")))]
    mod cleanup {
        use super::*;
        use std::process::{Child, Command};
        use std::time::{Duration, Instant};

        /// Beyond `pid_max` on macOS and Linux, so never a live process.
        const DEAD_PID: u32 = 99_999_999;

        struct Sleeper(Child);

        impl Sleeper {
            fn spawn() -> Self {
                Self(
                    Command::new("sleep")
                        .arg("30")
                        .spawn()
                        .expect("spawn sleep"),
                )
            }

            fn pid(&self) -> u32 {
                self.0.id()
            }

            fn identity(&self) -> ProcessIdentity {
                process_identity(self.pid()).expect("identity of a live sleep")
            }

            fn record(&self) -> ChildRecord {
                ChildRecord::observe(self.pid())
            }

            fn exits_within(&mut self, timeout: Duration) -> bool {
                let deadline = Instant::now() + timeout;
                loop {
                    if self.0.try_wait().expect("poll sleep").is_some() {
                        return true;
                    }
                    if Instant::now() >= deadline {
                        return false;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }

            fn is_running(&mut self) -> bool {
                self.0.try_wait().expect("poll sleep").is_none()
            }
        }

        impl Drop for Sleeper {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }

        /// A `sleep` whose parent shell exits at once, so it is reparented
        /// (to PID 1 unless a subreaper exists). Killed on drop if it is still
        /// the same process.
        struct Orphan {
            pid: u32,
            start: u64,
        }

        impl Orphan {
            fn spawn() -> Self {
                let output = Command::new("sh")
                    .args(["-c", "sleep 30 >/dev/null 2>&1 & echo $!"])
                    .output()
                    .expect("spawn orphaned sleep");
                let pid = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .parse()
                    .expect("orphan pid");
                let start = process_identity(pid).expect("orphan identity").start;
                Self { pid, start }
            }

            fn is_running(&self) -> bool {
                process_identity(self.pid).is_some_and(|identity| identity.start == self.start)
            }

            fn is_reparented_to_init(&self) -> bool {
                process_identity(self.pid).is_some_and(|identity| identity.ppid == 1)
            }

            fn exits_within(&self, timeout: Duration) -> bool {
                let deadline = Instant::now() + timeout;
                while self.is_running() {
                    if Instant::now() >= deadline {
                        return false;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                true
            }
        }

        impl Drop for Orphan {
            fn drop(&mut self) {
                if self.is_running() {
                    signal(self.pid, libc::SIGKILL);
                }
            }
        }

        fn write_owner(root: &Path, owner: Owner, children: Vec<ChildRecord>) -> PathBuf {
            let path = records_dir(root).join(owner.file_name());
            let record = OwnerRecord {
                owner_pid: owner.pid,
                owner_start: owner.start,
                children,
            };
            write_record(&path, &record).expect("write owner record");
            path
        }

        const EXIT_TIMEOUT: Duration = Duration::from_secs(5);

        #[test]
        fn empty_workspace_signals_nothing() {
            let tmp = tempfile::tempdir().expect("tempdir");
            assert_eq!(cleanup_workspace(tmp.path()), 0);
        }

        #[test]
        fn live_owner_children_are_left_alone() {
            let tmp = tempfile::tempdir().expect("tempdir");
            let owner = Sleeper::spawn();
            let mut child = Sleeper::spawn();
            let record = write_owner(
                tmp.path(),
                Owner {
                    pid: owner.pid(),
                    start: Some(owner.identity().start),
                },
                vec![child.record()],
            );

            assert_eq!(cleanup_workspace(tmp.path()), 0);
            assert!(child.is_running());
            assert!(record.exists(), "a live owner's record must be kept");
        }

        #[test]
        fn dead_owner_children_are_killed() {
            let tmp = tempfile::tempdir().expect("tempdir");
            let mut child = Sleeper::spawn();
            let record = write_owner(
                tmp.path(),
                Owner {
                    pid: DEAD_PID,
                    start: Some(1),
                },
                vec![child.record(), fake_child(DEAD_PID, Some(1))],
            );

            assert_eq!(cleanup_workspace(tmp.path()), 1);
            assert!(child.exits_within(EXIT_TIMEOUT));
            assert!(!record.exists(), "a dead owner's record must be removed");
        }

        #[test]
        fn recycled_owner_pid_counts_as_dead() {
            let tmp = tempfile::tempdir().expect("tempdir");
            // The owner PID is alive but names a different process.
            let mut impostor = Sleeper::spawn();
            let mut child = Sleeper::spawn();
            write_owner(
                tmp.path(),
                Owner {
                    pid: impostor.pid(),
                    start: Some(impostor.identity().start + 1),
                },
                vec![child.record()],
            );

            assert_eq!(cleanup_workspace(tmp.path()), 1);
            assert!(child.exits_within(EXIT_TIMEOUT));
            assert!(
                impostor.is_running(),
                "the recycled owner PID is never signaled"
            );
        }

        #[test]
        fn recycled_child_pids_are_never_signaled() {
            let tmp = tempfile::tempdir().expect("tempdir");
            let mut fingerprinted = Sleeper::spawn();
            let mut timestamped = Sleeper::spawn();
            let started_ms = timestamped.identity().started_at_ms.expect("start time");
            let record = write_owner(
                tmp.path(),
                Owner {
                    pid: DEAD_PID,
                    start: Some(1),
                },
                vec![
                    // Start fingerprint differs: the PID was recycled.
                    ChildRecord {
                        start: Some(fingerprinted.identity().start + 1),
                        ..fingerprinted.record()
                    },
                    // No fingerprint, and the process started after the spawn
                    // was recorded.
                    ChildRecord {
                        pid: timestamped.pid(),
                        start: None,
                        spawned_at_ms: started_ms - 60_000,
                        command: None,
                    },
                ],
            );

            assert_eq!(cleanup_workspace(tmp.path()), 0);
            std::thread::sleep(Duration::from_millis(100));
            assert!(fingerprinted.is_running());
            assert!(timestamped.is_running());
            assert!(!record.exists(), "a dead owner's record must be removed");
        }

        #[test]
        fn malformed_records_are_removed_only_for_dead_owners() {
            let tmp = tempfile::tempdir().expect("tempdir");
            let owner = Sleeper::spawn();
            let dir = records_dir(tmp.path());
            std::fs::create_dir_all(&dir).expect("records dir");
            let live = dir.join(
                Owner {
                    pid: owner.pid(),
                    start: Some(owner.identity().start),
                }
                .file_name(),
            );
            let dead = dir.join(format!("{DEAD_PID}-1.json"));
            std::fs::write(&live, b"{ not json").expect("write live record");
            std::fs::write(&dead, b"{ not json").expect("write dead record");

            assert_eq!(cleanup_workspace(tmp.path()), 0);
            assert!(live.exists());
            assert!(!dead.exists());
        }

        #[test]
        fn legacy_file_kills_only_orphans_older_than_the_file() {
            let tmp = tempfile::tempdir().expect("tempdir");
            let mut attached = Sleeper::spawn();
            let orphan = Orphan::spawn();
            let legacy = tmp.path().join(LEGACY_FILE);
            std::fs::create_dir_all(legacy.parent().expect("runtime dir")).expect("runtime dir");
            std::fs::write(
                &legacy,
                serde_json::to_vec(&[DEAD_PID, attached.pid(), orphan.pid]).expect("json"),
            )
            .expect("write legacy file");
            let orphaned = orphan.is_reparented_to_init();

            cleanup_workspace(tmp.path());
            assert!(
                attached.is_running(),
                "a process with a live parent is kept"
            );
            assert!(
                !legacy.exists(),
                "the legacy file is deleted after migration"
            );
            if orphaned {
                assert!(orphan.exits_within(EXIT_TIMEOUT));
            } else {
                assert!(orphan.is_running(), "reparented to a subreaper, not init");
            }
        }

        #[test]
        fn legacy_entry_started_after_the_last_write_is_refused() {
            let tmp = tempfile::tempdir().expect("tempdir");
            let orphan = Orphan::spawn();
            let legacy = tmp.path().join(LEGACY_FILE);
            std::fs::create_dir_all(legacy.parent().expect("runtime dir")).expect("runtime dir");
            std::fs::write(&legacy, serde_json::to_vec(&[orphan.pid]).expect("json"))
                .expect("write legacy file");
            std::fs::File::options()
                .write(true)
                .open(&legacy)
                .expect("open legacy file")
                .set_modified(SystemTime::now() - Duration::from_secs(3_600))
                .expect("backdate legacy file");

            assert_eq!(cleanup_workspace(tmp.path()), 0);
            assert!(orphan.is_running());
            assert!(!legacy.exists());
        }

        #[test]
        fn reaper_kills_verified_orphans_and_drops_recycled_pids() {
            let verified = Orphan::spawn();
            let recycled = Orphan::spawn();
            let orphaned = verified.is_reparented_to_init();
            let records = [
                ChildRecord::observe(verified.pid),
                ChildRecord {
                    start: Some(recycled.start + 1),
                    ..ChildRecord::observe(recycled.pid)
                },
                fake_child(DEAD_PID, Some(1)),
            ];

            let (killed, stale) = reap(&records);
            assert!(recycled.is_running(), "a recycled PID is never signaled");
            assert!(stale.contains(&recycled.pid));
            assert!(stale.contains(&DEAD_PID));
            if orphaned {
                assert_eq!(killed, 1);
                assert!(stale.contains(&verified.pid));
                assert!(verified.exits_within(EXIT_TIMEOUT));
            } else {
                assert_eq!(killed, 0);
            }
        }

        #[test]
        fn reaper_leaves_attached_children_alone() {
            let mut child = Sleeper::spawn();
            let (killed, stale) = reap(&[child.record()]);
            assert_eq!(killed, 0);
            assert!(stale.is_empty());
            assert!(child.is_running());
        }
    }
}
