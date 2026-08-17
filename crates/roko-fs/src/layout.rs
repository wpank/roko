//! Low-level `.roko/` directory layout definition and path helpers.
//!
//! The canonical layout under a project root is:
//!
//! ```text
//! .roko/
//!   runtime/        # pid files, sockets, locks
//!   memory/         # legacy compatibility artifacts
//!   episodes.jsonl  # canonical episode log
//!   plans/          # enrichment artifacts per plan
//!     {plan_id}/
//!   runs/           # per-run metrics, traces, snapshots
//!     {run_id}/
//!   state/          # orchestrator snapshots, event logs, session state
//!   config/         # config.toml, presets
//!   cache/          # cargo-target, context-pack-cache
//! ```
//!
//! [`RokoLayout`] exposes typed path helpers for filesystem-internal code.
//! Runtime and CLI code should prefer `roko_core::Workspace` as the
//! workspace boundary, with this module retained as a path catalog for
//! crates that need direct layout versioning or migration helpers.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// On-disk format version for the `.roko/` directory.
///
/// When the directory layout changes in a backwards-incompatible way, a
/// new variant is added here, and the migration code maps old versions to
/// the current one. The version is persisted in `.roko/VERSION`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayoutVersion {
    /// Initial layout: `runtime/`, `memory/`, `plans/`, `runs/`,
    /// `state/`, `config/`, `cache/`.
    V1 = 1,

    /// Converged storage layout (E02):
    /// - gate verdicts written to `gate-verdicts.jsonl` (not `signals.jsonl`)
    /// - episodes canonical at root `episodes.jsonl` (not `memory/episodes.jsonl`)
    /// - `state/executor.json` superseded by `state/state-snapshot.json`
    /// - gate thresholds materialized to `learn/gate-thresholds.json`
    /// - `signals.jsonl` retained only as a legacy migration input
    V2 = 2,

    /// Canonical episode storage:
    /// - root `episodes.jsonl` is the only active episode sink
    /// - `learn/episodes.jsonl` and `memory/episodes.jsonl` are migrated inputs
    /// - malformed records are quarantined without discarding their bytes
    V3 = 3,
}

impl LayoutVersion {
    /// The most recent version. New directories are initialized to this.
    pub const CURRENT: Self = Self::V3;

    /// Parse from the numeric value stored in `.roko/VERSION`.
    ///
    /// Returns `None` for unrecognized values.
    #[must_use]
    pub const fn from_u32(n: u32) -> Option<Self> {
        match n {
            1 => Some(Self::V1),
            2 => Some(Self::V2),
            3 => Some(Self::V3),
            _ => None,
        }
    }

    /// The numeric value written to `.roko/VERSION`.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}

/// Path helpers for the `.roko/` directory tree.
///
/// All methods are pure path arithmetic — no I/O is performed. Call
/// [`RokoLayout::ensure_dirs`] to create the directory structure on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RokoLayout {
    /// Root of the `.roko/` directory (e.g. `/project/.roko`).
    root: PathBuf,
}

impl RokoLayout {
    /// Construct a layout rooted at `root` (the `.roko/` directory itself).
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Construct a layout for the given project directory.
    ///
    /// Equivalent to `RokoLayout::new(project_root.join(".roko"))`.
    #[must_use]
    pub fn for_project(project_root: impl AsRef<Path>) -> Self {
        Self::new(project_root.as_ref().join(".roko"))
    }

    /// Construct a layout for a named repo under the project's `.roko/repos/{name}/`.
    ///
    /// Repo-specific signals, episodes, state, etc. are stored in their own
    /// subdirectory so they don't intermingle with the global data.
    #[must_use]
    pub fn for_repo(project_root: impl AsRef<Path>, repo_name: &str) -> Self {
        Self::new(
            project_root
                .as_ref()
                .join(".roko")
                .join("repos")
                .join(repo_name),
        )
    }

    // ── root accessors ────────────────────────────────────────────────────

    /// The `.roko/` root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path to the VERSION file: `.roko/VERSION`.
    #[must_use]
    pub fn version_file(&self) -> PathBuf {
        self.root.join("VERSION")
    }

    // ── top-level subdirectories ──────────────────────────────────────────

    /// `.roko/runtime/` — pid files, sockets, locks.
    #[must_use]
    pub fn runtime_dir(&self) -> PathBuf {
        self.root.join("runtime")
    }

    /// `.roko/memory/` — episodes, playbook, skills.
    #[must_use]
    pub fn memory_dir(&self) -> PathBuf {
        self.root.join("memory")
    }

    /// `.roko/plans/` — enrichment artifacts per plan.
    #[must_use]
    pub fn plans_dir(&self) -> PathBuf {
        self.root.join("plans")
    }

    /// `.roko/runs/` — per-run metrics, traces, snapshots.
    #[must_use]
    pub fn runs_dir(&self) -> PathBuf {
        self.root.join("runs")
    }

    /// `.roko/state/` — orchestrator snapshots, event logs, session state.
    #[must_use]
    pub fn state_dir(&self) -> PathBuf {
        self.root.join("state")
    }

    /// `.roko/config/` — config.toml, presets.
    #[must_use]
    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    /// `.roko/cache/` — cargo-target, context-pack-cache.
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    /// `.roko/learn/` — efficiency events, cascade router, experiments,
    /// gate thresholds, and other learning artifacts.
    #[must_use]
    pub fn learn_dir(&self) -> PathBuf {
        self.root.join("learn")
    }

    /// `.roko/prd/` — PRD storage.
    #[must_use]
    pub fn prd_dir(&self) -> PathBuf {
        self.root.join("prd")
    }

    /// `.roko/research/` — research artifacts.
    #[must_use]
    pub fn research_dir(&self) -> PathBuf {
        self.root.join("research")
    }

    /// `.roko/jobs/` — job execution artifacts.
    #[must_use]
    pub fn jobs_dir(&self) -> PathBuf {
        self.root.join("jobs")
    }

    /// `.roko/jobs/plan-runs/` — copied plan roots for local job execution.
    #[must_use]
    pub fn plan_runs_dir(&self) -> PathBuf {
        self.jobs_dir().join("plan-runs")
    }

    /// `.roko/extensions/` — local plugin extension manifests.
    #[must_use]
    pub fn extensions_dir(&self) -> PathBuf {
        self.root.join("extensions")
    }

    /// `.roko/triggers/` — persisted trigger bindings (one `.toml` per binding).
    #[must_use]
    pub fn triggers_dir(&self) -> PathBuf {
        self.root.join("triggers")
    }

    /// `.roko/neuro/` — knowledge ingestion scratch space.
    #[must_use]
    pub fn neuro_dir(&self) -> PathBuf {
        self.root.join("neuro")
    }

    // ── per-entity paths ─────────────────────────────────────────────────

    /// `.roko/engrams.jsonl` — the main signal log.
    #[must_use]
    pub fn engrams_path(&self) -> PathBuf {
        self.root.join("engrams.jsonl")
    }

    /// Legacy path for the signal log (pre-rename).
    ///
    /// Use [`Self::engrams_path`] for new code. This helper exists so
    /// callers can check for the old file and migrate it.
    #[must_use]
    pub fn engrams_path_legacy(&self) -> PathBuf {
        self.root.join("signals.jsonl")
    }

    /// `.roko/signals.jsonl` — legacy signal log.
    #[must_use]
    pub fn signals_path(&self) -> PathBuf {
        self.root.join("signals.jsonl")
    }

    /// `.roko/gate-verdicts.jsonl` — typed gate verdict log.
    ///
    /// Gate verdicts are written here by the runner and read by
    /// serve dashboard routes. This replaces the legacy practice
    /// of appending flat verdict rows to `signals.jsonl`.
    #[must_use]
    pub fn gate_verdicts_path(&self) -> PathBuf {
        self.root.join("gate-verdicts.jsonl")
    }

    /// `.roko/episodes.jsonl` — canonical root episode log.
    #[must_use]
    pub fn root_episodes_path(&self) -> PathBuf {
        self.root.join("episodes.jsonl")
    }

    /// `.roko/events.jsonl` — append-only runner event log.
    #[must_use]
    pub fn events_jsonl_path(&self) -> PathBuf {
        self.root.join("events.jsonl")
    }

    /// `.roko/metrics/telemetry-observations.jsonl` — periodic Lens snapshots.
    #[must_use]
    pub fn telemetry_observations_path(&self) -> PathBuf {
        self.root
            .join("metrics")
            .join("telemetry-observations.jsonl")
    }

    /// `.roko/roko.log` — main log file.
    #[must_use]
    pub fn log_path(&self) -> PathBuf {
        self.root.join("roko.log")
    }

    /// `.roko/roko.toml` — legacy workspace-local configuration file.
    #[must_use]
    pub fn roko_toml_path(&self) -> PathBuf {
        self.root.join("roko.toml")
    }

    /// `.roko/mcp.json` — workspace-local MCP configuration file.
    #[must_use]
    pub fn mcp_config_path(&self) -> PathBuf {
        self.root.join("mcp.json")
    }

    /// `.roko/runner-stderr.log` — approval-TUI stderr redirect.
    #[must_use]
    pub fn runner_stderr_log(&self) -> PathBuf {
        self.root.join("runner-stderr.log")
    }

    /// `.roko/learn/efficiency.jsonl` — per-turn efficiency events.
    #[must_use]
    pub fn efficiency_path(&self) -> PathBuf {
        self.learn_dir().join("efficiency.jsonl")
    }

    /// `.roko/learn/efficiency.jsonl` — per-turn efficiency events.
    #[must_use]
    pub fn efficiency_log_path(&self) -> PathBuf {
        self.efficiency_path()
    }

    /// `.roko/learn/cascade-router.json` — model routing bandit state.
    #[must_use]
    pub fn cascade_router_path(&self) -> PathBuf {
        self.learn_dir().join("cascade-router.json")
    }

    /// `.roko/learn/gate-thresholds.json` — adaptive gate thresholds.
    #[must_use]
    pub fn gate_thresholds_path(&self) -> PathBuf {
        self.learn_dir().join("gate-thresholds.json")
    }

    /// `.roko/learn/experiments.json` — prompt experiment store.
    #[must_use]
    pub fn experiments_path(&self) -> PathBuf {
        self.learn_dir().join("experiments.json")
    }

    /// `.roko/learn/playbooks/` — learned playbook store.
    #[must_use]
    pub fn playbooks_dir(&self) -> PathBuf {
        self.learn_dir().join("playbooks")
    }

    /// `.roko/plans/{plan_id}/` — enrichment artifacts for one plan.
    #[must_use]
    pub fn plan_dir(&self, plan_id: &str) -> PathBuf {
        self.plans_dir().join(plan_id)
    }

    /// `.roko/runs/{run_id}/` — data directory for one run.
    #[must_use]
    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.runs_dir().join(run_id)
    }

    /// `.roko/runs/{run_id}/metrics.jsonl` — metrics log for one run.
    #[must_use]
    pub fn run_metrics(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("metrics.jsonl")
    }

    /// `.roko/runs/{run_id}/traces/` — traces directory for one run.
    #[must_use]
    pub fn run_traces_dir(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("traces")
    }

    /// `.roko/episodes.jsonl` — the canonical episodes log.
    ///
    /// Previously pointed at `.roko/memory/episodes.jsonl`. Converged to the
    /// root log so that `FeedbackService`, the runner, and serve all agree on
    /// a single episode sink. Legacy `memory/episodes.jsonl` is treated as
    /// migration-only input.
    #[must_use]
    pub fn episodes_path(&self) -> PathBuf {
        self.root_episodes_path()
    }

    /// `.roko/custody.jsonl` — append-only custody audit chain.
    ///
    /// Each line is a JSON-serialized [`Custody`] record capturing who
    /// did what, with what authorization, under what taint, and which
    /// gates passed.
    #[must_use]
    pub fn custody_log(&self) -> PathBuf {
        self.root.join("custody.jsonl")
    }

    /// `.roko/witness.jsonl` — append-only witness DAG log.
    ///
    /// Each line is a JSON-serialized `WitnessVertex` recording the
    /// agent reasoning chain (observation, prediction, decision,
    /// resolution, neuro entry).
    #[must_use]
    pub fn witness_log(&self) -> PathBuf {
        self.root.join("witness.jsonl")
    }

    /// `.roko/memory/playbook.toml` — the active playbook.
    #[must_use]
    pub fn playbook_path(&self) -> PathBuf {
        self.memory_dir().join("playbook.toml")
    }

    /// `.roko/memory/skills/` — learned skills directory.
    #[must_use]
    pub fn skills_dir(&self) -> PathBuf {
        self.memory_dir().join("skills")
    }

    /// `.roko/config/config.toml` — main configuration file.
    #[must_use]
    pub fn config_file(&self) -> PathBuf {
        self.config_dir().join("config.toml")
    }

    /// `.roko/cache/cargo-target/` — shared cargo target directory.
    #[must_use]
    pub fn cargo_target_dir(&self) -> PathBuf {
        self.cache_dir().join("cargo-target")
    }

    /// `.roko/cache/context-pack-cache/` — cached context packs.
    #[must_use]
    pub fn context_pack_cache_dir(&self) -> PathBuf {
        self.cache_dir().join("context-pack-cache")
    }

    /// `.roko/state/executor.json` — executor snapshot for crash recovery.
    #[must_use]
    pub fn executor_snapshot(&self) -> PathBuf {
        self.state_dir().join("executor.json")
    }

    /// `.roko/state/executor.json` — executor snapshot for crash recovery.
    #[must_use]
    pub fn executor_snapshot_path(&self) -> PathBuf {
        self.executor_snapshot()
    }

    /// `.roko/state/orchestrator.json` — aggregate orchestrator snapshot.
    #[must_use]
    pub fn orchestrator_snapshot(&self) -> PathBuf {
        self.state_dir().join("orchestrator.json")
    }

    /// `.roko/state/run-state.json` — runner-owned resume state.
    #[must_use]
    pub fn run_state_path(&self) -> PathBuf {
        self.state_dir().join("run-state.json")
    }

    /// `.roko/state/run-ledger.jsonl` — typed run ledger (task starts, completions, gate outcomes).
    #[must_use]
    pub fn run_ledger_path(&self) -> PathBuf {
        self.state_dir().join("run-ledger.jsonl")
    }

    /// `.roko/state/events.json` — event log snapshot for crash recovery.
    #[must_use]
    pub fn event_log_snapshot(&self) -> PathBuf {
        self.state_dir().join("events.json")
    }

    /// `.roko/state/sessions/` — per-session directories.
    #[must_use]
    pub fn sessions_dir(&self) -> PathBuf {
        self.state_dir().join("sessions")
    }

    /// `.roko/state/sessions/{session_id}/` — one session's state.
    #[must_use]
    pub fn session_dir(&self, session_id: &str) -> PathBuf {
        self.sessions_dir().join(session_id)
    }

    /// `.roko/runtime/roko.pid` — the PID file for the running process.
    #[must_use]
    pub fn pid_file(&self) -> PathBuf {
        self.runtime_dir().join("roko.pid")
    }

    /// `.roko/runtime/roko.lock` — the advisory lock file.
    #[must_use]
    pub fn lock_file(&self) -> PathBuf {
        self.runtime_dir().join("roko.lock")
    }

    /// `.roko/runtime/agent-pids.json` — live agent PID registry.
    #[must_use]
    pub fn agent_pids_path(&self) -> PathBuf {
        self.runtime_dir().join("agent-pids.json")
    }

    /// `.roko/runtime/gate-attempts/` — per-gate attempt sentinels.
    #[must_use]
    pub fn gate_attempts_dir(&self) -> PathBuf {
        self.runtime_dir().join("gate-attempts")
    }

    // ── I/O helpers ──────────────────────────────────────────────────────

    /// All top-level subdirectories that should exist for a working layout.
    #[must_use]
    pub fn top_level_dirs(&self) -> Vec<PathBuf> {
        vec![
            self.runtime_dir(),
            self.memory_dir(),
            self.plans_dir(),
            self.runs_dir(),
            self.state_dir(),
            self.config_dir(),
            self.cache_dir(),
            self.learn_dir(),
        ]
    }

    /// Create all top-level subdirectories and write the VERSION file.
    ///
    /// Idempotent: re-running on an existing layout is a no-op.
    ///
    /// # Errors
    ///
    /// Returns an error if any directory cannot be created or the VERSION
    /// file cannot be written.
    pub async fn ensure_dirs(&self) -> std::io::Result<()> {
        for dir in &self.top_level_dirs() {
            tokio::fs::create_dir_all(dir).await?;
        }
        let version_path = self.version_file();
        if !version_path.exists() {
            let has_historical_episodes = [
                self.episodes_path(),
                self.learn_dir().join("episodes.jsonl"),
                self.memory_dir().join("episodes.jsonl"),
            ]
            .iter()
            .any(|path| path.exists());
            if has_historical_episodes {
                self.migrate_v2_to_v3().await?;
            } else {
                // Fresh workspace: publish CURRENT directly.
                self.write_version_atomic(LayoutVersion::CURRENT).await?;
            }
        } else {
            // Existing workspace: check if migration is needed.
            let on_disk = self.read_version().await?;
            match on_disk {
                Some(LayoutVersion::V1) => {
                    self.migrate_v1_to_v2().await?;
                    self.migrate_v2_to_v3().await?;
                }
                Some(LayoutVersion::V2) => self.migrate_v2_to_v3().await?,
                Some(LayoutVersion::V3) | None => {}
            }
        }
        Ok(())
    }

    /// Read the layout version from the VERSION file.
    ///
    /// Returns `None` if the file does not exist or contains an
    /// unrecognized version number.
    ///
    /// # Errors
    ///
    /// Returns an error on I/O failure (other than not-found).
    pub async fn read_version(&self) -> std::io::Result<Option<LayoutVersion>> {
        let path = self.version_file();
        match tokio::fs::read_to_string(&path).await {
            Ok(contents) => {
                let parsed = contents.trim().parse::<u32>().ok();
                Ok(parsed.and_then(LayoutVersion::from_u32))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Migrate a V1 workspace to V2 in-place.
    ///
    /// 1. Renames `signals.jsonl` to `signals.jsonl.v1-legacy` (no data loss).
    /// 2. Writes VERSION = 2.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if renaming or writing the VERSION file fails.
    pub async fn migrate_v1_to_v2(&self) -> std::io::Result<()> {
        let signals = self.signals_path();
        if signals.exists() {
            let legacy = self.root.join("signals.jsonl.v1-legacy");
            tokio::fs::rename(&signals, &legacy).await?;
        }
        self.write_version_atomic(LayoutVersion::V2).await?;
        Ok(())
    }

    /// Migrate all historical episode roots into the canonical root log.
    ///
    /// Valid JSON objects are merged in root, learn, then memory order and
    /// de-duplicated by `episode_id`, then `id`, then the legacy identity
    /// fields. Exact source bytes are retained in sibling `.v2-legacy`
    /// archives. Non-JSON records are encoded with their original bytes in
    /// `episodes.jsonl.v3-malformed.jsonl` for operator recovery.
    ///
    /// The canonical replacement and version bump happen only after every
    /// source has been archived. Re-running a completed migration is a no-op.
    pub async fn migrate_v2_to_v3(&self) -> std::io::Result<()> {
        if self.read_version().await? == Some(LayoutVersion::V3) {
            return Ok(());
        }

        let canonical = self.episodes_path();
        let learn = self.learn_dir().join("episodes.jsonl");
        let memory = self.memory_dir().join("episodes.jsonl");
        let sources = [
            ("episodes.jsonl", canonical.clone()),
            ("learn/episodes.jsonl", learn.clone()),
            ("memory/episodes.jsonl", memory.clone()),
        ];
        let mut source_bytes = Vec::new();
        for (label, path) in &sources {
            match tokio::fs::read(path).await {
                Ok(bytes) => source_bytes.push(((*label).to_string(), path.clone(), bytes)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }

        for (_, path, bytes) in &source_bytes {
            preserve_episode_source(path, bytes).await?;
        }

        let mut seen = HashSet::new();
        let mut canonical_bytes = Vec::new();
        let mut malformed = Vec::new();
        for (label, _, bytes) in &source_bytes {
            collect_episode_records(
                label,
                bytes,
                &mut seen,
                &mut canonical_bytes,
                &mut malformed,
            );
        }

        if !malformed.is_empty() {
            write_malformed_quarantine(
                &self.root.join("episodes.jsonl.v3-malformed.jsonl"),
                &malformed,
            )
            .await?;
        }
        if !canonical_bytes.is_empty() || canonical.exists() {
            let temporary = self.root.join("episodes.jsonl.v3.tmp");
            tokio::fs::write(&temporary, &canonical_bytes).await?;
            tokio::fs::rename(&temporary, &canonical).await?;
        }

        for legacy in [&learn, &memory] {
            match tokio::fs::remove_file(legacy).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        self.write_version_atomic(LayoutVersion::V3).await?;
        Ok(())
    }

    async fn write_version_atomic(&self, version: LayoutVersion) -> std::io::Result<()> {
        let temporary = self.root.join("VERSION.tmp");
        tokio::fs::write(&temporary, version.as_u32().to_string()).await?;
        tokio::fs::rename(temporary, self.version_file()).await
    }
}

fn episode_record_key(value: &serde_json::Value) -> Vec<u8> {
    for field in ["episode_id", "id"] {
        if let Some(id) = value.get(field).and_then(serde_json::Value::as_str)
            && !id.is_empty()
        {
            return format!("id:{id}").into_bytes();
        }
    }
    let identity_fields = ["agent_id", "task_id", "timestamp", "model"];
    if identity_fields
        .iter()
        .any(|field| value.get(field).is_some())
    {
        let mut key = b"legacy:".to_vec();
        for field in identity_fields {
            if let Some(text) = value.get(field).and_then(serde_json::Value::as_str) {
                key.extend_from_slice(text.as_bytes());
            }
            key.push(0);
        }
        return key;
    }
    let mut key = b"json:".to_vec();
    key.extend(serde_json::to_vec(value).unwrap_or_default());
    key
}

fn collect_episode_records(
    source: &str,
    bytes: &[u8],
    seen: &mut HashSet<Vec<u8>>,
    canonical: &mut Vec<u8>,
    malformed: &mut Vec<(String, Vec<u8>)>,
) {
    for raw in bytes.split_inclusive(|byte| *byte == b'\n') {
        let without_newline = raw.strip_suffix(b"\n").unwrap_or(raw);
        let record = without_newline
            .strip_suffix(b"\r")
            .unwrap_or(without_newline);
        if record.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        match serde_json::from_slice::<serde_json::Value>(record) {
            Ok(value) if value.is_object() => {
                if seen.insert(episode_record_key(&value)) {
                    canonical.extend_from_slice(record);
                    canonical.push(b'\n');
                }
            }
            Ok(_) | Err(_) => malformed.push((source.to_string(), raw.to_vec())),
        }
    }
}

async fn preserve_episode_source(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut suffix = 0_u64;
    loop {
        let ending = if suffix == 0 {
            ".v2-legacy".to_string()
        } else {
            format!(".v2-legacy.{suffix}")
        };
        let mut archived = path.as_os_str().to_os_string();
        archived.push(ending);
        let archived = PathBuf::from(archived);
        match tokio::fs::read(&archived).await {
            Ok(existing) if existing == bytes => return Ok(()),
            Ok(_) => suffix = suffix.saturating_add(1),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return tokio::fs::write(archived, bytes).await;
            }
            Err(error) => return Err(error),
        }
    }
}

async fn write_malformed_quarantine(
    path: &Path,
    records: &[(String, Vec<u8>)],
) -> std::io::Result<()> {
    let mut existing = match tokio::fs::read(path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error),
    };
    let mut seen = existing
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .filter_map(|value| {
            let source = value.get("source")?.as_str()?.to_string();
            let bytes = value
                .get("bytes")?
                .as_array()?
                .iter()
                .map(|byte| u8::try_from(byte.as_u64()?).ok())
                .collect::<Option<Vec<_>>>()?;
            Some((source, bytes))
        })
        .collect::<HashSet<_>>();
    for (source, bytes) in records {
        if !seen.insert((source.clone(), bytes.clone())) {
            continue;
        }
        let mut line = serde_json::to_vec(&serde_json::json!({
            "source": source,
            "bytes": bytes,
        }))
        .map_err(std::io::Error::other)?;
        line.push(b'\n');
        existing.extend(line);
    }
    let mut temporary = path.as_os_str().to_os_string();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    tokio::fs::write(&temporary, existing).await?;
    tokio::fs::rename(temporary, path).await
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn layout_root_is_what_was_passed() {
        let layout = RokoLayout::new("/tmp/.roko");
        assert_eq!(layout.root(), Path::new("/tmp/.roko"));
    }

    #[test]
    fn for_project_appends_dot_roko() {
        let layout = RokoLayout::for_project("/home/user/project");
        assert_eq!(layout.root(), Path::new("/home/user/project/.roko"));
    }

    #[test]
    fn top_level_dirs_returns_eight() {
        let layout = RokoLayout::new("/tmp/.roko");
        let dirs = layout.top_level_dirs();
        assert_eq!(dirs.len(), 8);
        assert!(dirs.contains(&PathBuf::from("/tmp/.roko/runtime")));
        assert!(dirs.contains(&PathBuf::from("/tmp/.roko/memory")));
        assert!(dirs.contains(&PathBuf::from("/tmp/.roko/plans")));
        assert!(dirs.contains(&PathBuf::from("/tmp/.roko/runs")));
        assert!(dirs.contains(&PathBuf::from("/tmp/.roko/state")));
        assert!(dirs.contains(&PathBuf::from("/tmp/.roko/config")));
        assert!(dirs.contains(&PathBuf::from("/tmp/.roko/cache")));
        assert!(dirs.contains(&PathBuf::from("/tmp/.roko/learn")));
    }

    #[test]
    fn plan_dir_is_under_plans() {
        let layout = RokoLayout::new("/p/.roko");
        assert_eq!(
            layout.plan_dir("plan-42"),
            PathBuf::from("/p/.roko/plans/plan-42")
        );
    }

    #[test]
    fn run_dir_and_children() {
        let layout = RokoLayout::new("/p/.roko");
        assert_eq!(layout.run_dir("r1"), PathBuf::from("/p/.roko/runs/r1"));
        assert_eq!(
            layout.run_metrics("r1"),
            PathBuf::from("/p/.roko/runs/r1/metrics.jsonl")
        );
        assert_eq!(
            layout.run_traces_dir("r1"),
            PathBuf::from("/p/.roko/runs/r1/traces")
        );
    }

    #[test]
    fn memory_paths() {
        let layout = RokoLayout::new("/x/.roko");
        assert_eq!(
            layout.episodes_path(),
            PathBuf::from("/x/.roko/episodes.jsonl")
        );
        assert_eq!(
            layout.playbook_path(),
            PathBuf::from("/x/.roko/memory/playbook.toml")
        );
        assert_eq!(layout.skills_dir(), PathBuf::from("/x/.roko/memory/skills"));
    }

    #[test]
    fn config_and_cache_paths() {
        let layout = RokoLayout::new("/c/.roko");
        assert_eq!(
            layout.config_file(),
            PathBuf::from("/c/.roko/config/config.toml")
        );
        assert_eq!(
            layout.cargo_target_dir(),
            PathBuf::from("/c/.roko/cache/cargo-target")
        );
        assert_eq!(
            layout.context_pack_cache_dir(),
            PathBuf::from("/c/.roko/cache/context-pack-cache")
        );
        assert_eq!(
            layout.telemetry_observations_path(),
            PathBuf::from("/c/.roko/metrics/telemetry-observations.jsonl")
        );
    }

    #[test]
    fn witness_log_path() {
        let layout = RokoLayout::new("/w/.roko");
        assert_eq!(
            layout.witness_log(),
            PathBuf::from("/w/.roko/witness.jsonl")
        );
    }

    #[test]
    fn runtime_paths() {
        let layout = RokoLayout::new("/r/.roko");
        assert_eq!(
            layout.pid_file(),
            PathBuf::from("/r/.roko/runtime/roko.pid")
        );
        assert_eq!(
            layout.lock_file(),
            PathBuf::from("/r/.roko/runtime/roko.lock")
        );
    }

    #[test]
    fn version_file_path() {
        let layout = RokoLayout::new("/v/.roko");
        assert_eq!(layout.version_file(), PathBuf::from("/v/.roko/VERSION"));
    }

    #[tokio::test]
    async fn ensure_dirs_creates_all_directories() {
        let tmp = TempDir::new().expect("tempdir");
        let layout = RokoLayout::for_project(tmp.path());
        layout.ensure_dirs().await.expect("ensure_dirs");

        for dir in &layout.top_level_dirs() {
            assert!(dir.is_dir(), "directory should exist: {dir:?}");
        }
        assert!(layout.version_file().exists(), "VERSION file should exist");
    }

    #[tokio::test]
    async fn ensure_dirs_is_idempotent() {
        let tmp = TempDir::new().expect("tempdir");
        let layout = RokoLayout::for_project(tmp.path());
        layout.ensure_dirs().await.expect("first call");
        layout.ensure_dirs().await.expect("second call");

        let version = layout.read_version().await.expect("read version");
        assert_eq!(version, Some(LayoutVersion::CURRENT));
    }

    #[tokio::test]
    async fn read_version_returns_none_for_missing() {
        let tmp = TempDir::new().expect("tempdir");
        let layout = RokoLayout::for_project(tmp.path());
        let version = layout.read_version().await.expect("read version");
        assert_eq!(version, None);
    }

    #[tokio::test]
    async fn read_version_round_trips() {
        let tmp = TempDir::new().expect("tempdir");
        let layout = RokoLayout::for_project(tmp.path());
        layout.ensure_dirs().await.expect("ensure dirs");

        let version = layout.read_version().await.expect("read version");
        assert_eq!(version, Some(LayoutVersion::V3));
    }

    #[test]
    fn layout_version_from_u32() {
        assert_eq!(LayoutVersion::from_u32(1), Some(LayoutVersion::V1));
        assert_eq!(LayoutVersion::from_u32(2), Some(LayoutVersion::V2));
        assert_eq!(LayoutVersion::from_u32(3), Some(LayoutVersion::V3));
        assert_eq!(LayoutVersion::from_u32(0), None);
        assert_eq!(LayoutVersion::from_u32(999), None);
    }

    #[test]
    fn layout_version_as_u32() {
        assert_eq!(LayoutVersion::V1.as_u32(), 1);
        assert_eq!(LayoutVersion::V2.as_u32(), 2);
        assert_eq!(LayoutVersion::V3.as_u32(), 3);
    }

    #[test]
    fn layout_version_current_is_v3() {
        assert_eq!(LayoutVersion::CURRENT, LayoutVersion::V3);
    }

    #[tokio::test]
    async fn ensure_dirs_migrates_v1_through_v3() {
        let tmp = TempDir::new().expect("tempdir");
        let layout = RokoLayout::for_project(tmp.path());

        // Bootstrap a V1 workspace manually.
        for dir in &layout.top_level_dirs() {
            std::fs::create_dir_all(dir).expect("create dir");
        }
        std::fs::write(layout.version_file(), "1").expect("write VERSION");
        std::fs::write(layout.signals_path(), "{}\n").expect("write signals.jsonl");

        layout.ensure_dirs().await.expect("ensure_dirs migration");

        let version = layout.read_version().await.expect("read version");
        assert_eq!(version, Some(LayoutVersion::V3));

        assert!(
            !layout.signals_path().exists(),
            "signals.jsonl must be renamed away during migration"
        );
        assert!(
            layout.root().join("signals.jsonl.v1-legacy").exists(),
            "legacy signals file must be preserved as signals.jsonl.v1-legacy"
        );
    }

    #[tokio::test]
    async fn migrate_v1_to_v2_is_idempotent_when_no_signals_file() {
        let tmp = TempDir::new().expect("tempdir");
        let layout = RokoLayout::for_project(tmp.path());
        for dir in &layout.top_level_dirs() {
            std::fs::create_dir_all(dir).expect("create dir");
        }
        std::fs::write(layout.version_file(), "1").expect("write VERSION");

        layout.migrate_v1_to_v2().await.expect("migrate idempotent");
        layout.migrate_v1_to_v2().await.expect("repeat migration");

        let version = layout.read_version().await.expect("read version");
        assert_eq!(version, Some(LayoutVersion::V2));
    }

    #[tokio::test]
    async fn ensure_dirs_migrates_unversioned_legacy_episode_files() {
        let tmp = TempDir::new().expect("tempdir");
        let layout = RokoLayout::for_project(tmp.path());
        std::fs::create_dir_all(layout.learn_dir()).expect("create learn dir");
        std::fs::create_dir_all(layout.memory_dir()).expect("create memory dir");
        std::fs::write(
            layout.learn_dir().join("episodes.jsonl"),
            b"{\"episode_id\":\"learn-unversioned\"}\n",
        )
        .expect("write learn episodes");
        std::fs::write(
            layout.memory_dir().join("episodes.jsonl"),
            b"{\"episode_id\":\"memory-unversioned\"}\n",
        )
        .expect("write memory episodes");

        layout.ensure_dirs().await.expect("ensure and migrate");

        assert_eq!(
            layout.read_version().await.expect("read version"),
            Some(LayoutVersion::V3)
        );
        let canonical = std::fs::read_to_string(layout.episodes_path()).expect("read canonical");
        assert!(canonical.contains("learn-unversioned"));
        assert!(canonical.contains("memory-unversioned"));
        assert!(!layout.learn_dir().join("episodes.jsonl").exists());
        assert!(!layout.memory_dir().join("episodes.jsonl").exists());
        assert!(!layout.root().join("VERSION.tmp").exists());
    }

    #[tokio::test]
    async fn migrate_v2_to_v3_merges_deduplicates_quarantines_and_is_idempotent() {
        let tmp = TempDir::new().expect("tempdir");
        let layout = RokoLayout::for_project(tmp.path());
        for dir in &layout.top_level_dirs() {
            std::fs::create_dir_all(dir).expect("create dir");
        }
        std::fs::write(layout.version_file(), "2").expect("write VERSION");
        std::fs::write(
            layout.episodes_path(),
            b"{\"episode_id\":\"root\"}\n{\"episode_id\":\"duplicate\",\"source\":\"root\"}\nnot-json\n",
        )
        .expect("write root episodes");
        let learn = layout.learn_dir().join("episodes.jsonl");
        std::fs::write(
            &learn,
            b"{\"episode_id\":\"learn\"}\n{\"id\":\"duplicate\",\"source\":\"learn\"}\n{broken\n",
        )
        .expect("write learn episodes");
        let memory = layout.memory_dir().join("episodes.jsonl");
        std::fs::write(&memory, b"{\"episode_id\":\"memory\"}\n").expect("write memory episodes");

        layout.migrate_v2_to_v3().await.expect("migrate episodes");

        assert_eq!(
            layout.read_version().await.expect("read version"),
            Some(LayoutVersion::V3)
        );
        let canonical = std::fs::read_to_string(layout.episodes_path()).expect("read canonical");
        let rows = canonical.lines().collect::<Vec<_>>();
        assert_eq!(rows.len(), 4);
        assert!(rows.iter().any(|row| row.contains("\"root\"")));
        assert!(rows.iter().any(|row| row.contains("\"learn\"")));
        assert!(rows.iter().any(|row| row.contains("\"memory\"")));
        assert_eq!(
            rows.iter()
                .filter(|row| row.contains("\"duplicate\""))
                .count(),
            1
        );
        assert!(
            rows.iter().any(|row| row.contains("\"source\":\"root\"")),
            "canonical root must win duplicate IDs"
        );
        assert!(!learn.exists());
        assert!(!memory.exists());
        assert_eq!(
            std::fs::read(layout.root().join("episodes.jsonl.v2-legacy"))
                .expect("read root archive"),
            b"{\"episode_id\":\"root\"}\n{\"episode_id\":\"duplicate\",\"source\":\"root\"}\nnot-json\n"
        );
        assert_eq!(
            std::fs::read(layout.learn_dir().join("episodes.jsonl.v2-legacy"))
                .expect("read learn archive"),
            b"{\"episode_id\":\"learn\"}\n{\"id\":\"duplicate\",\"source\":\"learn\"}\n{broken\n"
        );
        assert_eq!(
            std::fs::read(layout.memory_dir().join("episodes.jsonl.v2-legacy"))
                .expect("read memory archive"),
            b"{\"episode_id\":\"memory\"}\n"
        );
        let quarantine_path = layout.root().join("episodes.jsonl.v3-malformed.jsonl");
        let quarantine = std::fs::read_to_string(&quarantine_path).expect("read quarantine");
        let quarantined_bytes = quarantine
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("quarantine row"))
            .map(|value| {
                value["bytes"]
                    .as_array()
                    .expect("byte array")
                    .iter()
                    .map(|byte| byte.as_u64().expect("byte") as u8)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert!(quarantined_bytes.contains(&b"not-json\n".to_vec()));
        assert!(quarantined_bytes.contains(&b"{broken\n".to_vec()));
        assert!(!layout.root().join("VERSION.tmp").exists());

        let canonical_before = std::fs::read(layout.episodes_path()).expect("read canonical");
        let quarantine_before = std::fs::read(&quarantine_path).expect("read quarantine");
        layout.migrate_v2_to_v3().await.expect("repeat migration");
        assert_eq!(
            std::fs::read(layout.episodes_path()).expect("read canonical"),
            canonical_before
        );
        assert_eq!(
            std::fs::read(quarantine_path).expect("read quarantine"),
            quarantine_before
        );
    }
}
