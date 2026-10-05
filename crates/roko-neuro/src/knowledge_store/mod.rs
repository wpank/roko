//! Append-only JSONL knowledge store.
//!
//! Knowledge entries live at `.roko/neuro/knowledge.jsonl` by default.
//! Writes append one JSON record per line, while maintenance operations
//! (`decay` and `gc`) rewrite the file atomically through a temporary
//! sibling.

mod anti_pattern;
mod backup;
pub mod commit;
mod crud;
mod gc;
pub mod memory_index;
mod query;
pub(crate) mod scoring;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
pub mod types;

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

use anyhow::{Context, Result};
use parking_lot::Mutex;

use crate::temporal::{AllenRelation, KnowledgeEpoch, TemporalIndex, TemporalInterval};
use crate::{KnowledgeEntry, NeuroStore};

use scoring::normalize_entry_security;

// Re-export all public items from sub-modules.
pub use anti_pattern::{classify_compilation_error, extract_anti_pattern_from_failure};
pub use backup::compute_merkle_root;
#[cfg(feature = "hdc")]
pub use memory_index::{MemoryHit, MemoryIndex};
pub use scoring::is_dead;
pub use types::{
    AntiKnowledgeConflict, BackupHeader, ContextAssemblyWeights, DEATH_THRESHOLD,
    DEFAULT_GC_MIN_CONFIDENCE, ExportBundle, ExportFilter, FalsifierOutcome, ImportOptions,
    ImportResult, KNOWLEDGE_BACKUP_VERSION, KnowledgeConfirmationRecord, KnowledgeQueryBreakdown,
    KnowledgeQueryHit, KnowledgeSimilarityHit, KnowledgeStats, QUERY_SCORE_FLOOR,
    RESURRECTION_CONFIDENCE,
};

/// The batches this process's runs write (P21, 8137): every store in the
/// process sees their uncommitted entries, which other processes' stores
/// skip until the batches commit.
static PROCESS_BATCHES: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// The write gate of each file this process's stores write, by
/// [`gate_key`]: the knowledge store's (bug-c4f0ed), and the admission and
/// heuristic logs' (bug-d81257). Every store of a file takes the same gate,
/// so their read-modify-write cycles never interleave. A gate is dropped
/// once no store holds it.
static WRITE_GATES: Mutex<BTreeMap<PathBuf, Weak<Mutex<()>>>> = Mutex::new(BTreeMap::new());

/// The write gate that every store of this process shares for the file at
/// `path`.
pub(crate) fn write_gate_for(path: &Path) -> Arc<Mutex<()>> {
    let key = gate_key(path);
    let mut gates = WRITE_GATES.lock();
    if let Some(gate) = gates.get(&key).and_then(Weak::upgrade) {
        return gate;
    }
    gates.retain(|_, gate| gate.strong_count() > 0);
    let gate = Arc::new(Mutex::new(()));
    gates.insert(key, Arc::downgrade(&gate));
    gate
}

/// The file at `path` as the write gates know it: its canonical path, so
/// that two spellings of one file share a gate. The part that does not
/// exist yet joins its deepest existing ancestor's canonical path, so the
/// key holds once the file is created.
fn gate_key(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let key = absolute.ancestors().find_map(|ancestor| {
        let canonical = fs::canonicalize(ancestor).ok()?;
        Some(canonical.join(absolute.strip_prefix(ancestor).ok()?))
    });
    key.unwrap_or(absolute)
}

/// A store's hold on its file for one read-modify-write cycle
/// ([`KnowledgeStore::lock_writes`]). Dropping it releases the file's lock
/// among processes, then this process's write gate.
#[must_use = "the file is unlocked as soon as the guard drops"]
pub struct WriteGuard<'a> {
    /// The file's `.lock` sibling, locked exclusively; `None` when the lock
    /// could not be taken, and the gate alone holds.
    _file: Option<File>,
    _gate: parking_lot::MutexGuard<'a, ()>,
}

impl<'a> WriteGuard<'a> {
    /// Take `gate`, the write gate of the file at `path` that this
    /// process's stores share, and then an exclusive lock on the file's
    /// `.lock` sibling, which another process writing the file takes too.
    /// The lock among processes is best-effort: when it cannot be taken, the
    /// write goes on under the gate alone, with a warning.
    pub(crate) fn hold(gate: &'a Mutex<()>, path: &Path) -> Self {
        let gate = gate.lock();
        let locked = path
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| roko_fs::log_rotation::lock_jsonl(path));
        let file = match locked {
            Ok(file) => Some(file),
            Err(error) => {
                tracing::warn!(
                    path = %path.display(),
                    %error,
                    "write without the file's lock among processes"
                );
                None
            }
        };
        Self {
            _file: file,
            _gate: gate,
        }
    }
}

/// Persistent knowledge store backed by an append-only JSONL file.
///
/// The store is cheap to clone: it holds the path and the file's write
/// gate, which every store of this process pointed at the file shares, so
/// that concurrent writes never interleave file rewrites and none loses
/// another's update (bug-c4f0ed). Each write also locks the file's `.lock`
/// sibling, which other processes' stores lock too.
///
/// When new entries overlap with existing entries (by tag and keyword
/// similarity), the store emits [`KnowledgeConfirmationRecord`]s to a
/// sibling JSONL file. These records feed the C-Factor metrics
/// `knowledge_integration_rate` and `convergence_velocity`.
#[derive(Debug, Clone)]
pub struct KnowledgeStore {
    pub(crate) path: PathBuf,
    pub(crate) confirmations_path: PathBuf,
    /// The file's write gate, shared by every store of this process
    /// pointed at it.
    pub(crate) write_gate: Arc<Mutex<()>>,
    temporal_index: Option<Arc<Mutex<TemporalIndex>>>,
    /// The run whose batch this store's ingests join, and whose
    /// uncommitted entries its retrieval sees (P21, 8137).
    commit_batch: Option<String>,
}

impl KnowledgeStore {
    /// Construct a store pointed at an explicit JSONL path.
    ///
    /// Confirmation records are written to a sibling file named
    /// `knowledge-confirmations.jsonl` in the same directory.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let confirmations_path = path
            .parent()
            .map(|parent| parent.join("knowledge-confirmations.jsonl"))
            .unwrap_or_else(|| PathBuf::from("knowledge-confirmations.jsonl"));
        let write_gate = write_gate_for(&path);
        Self {
            path,
            confirmations_path,
            write_gate,
            temporal_index: None,
            commit_batch: None,
        }
    }

    /// This store for the run `batch` (P21, 8137): the entries it ingests
    /// carry `commit_batch = batch` until a guarded commit clears it
    /// ([`commit::propose_batch`]). Every store of this process sees them;
    /// other runs' retrieval skips every uncommitted entry.
    #[must_use]
    pub fn with_commit_batch(mut self, batch: impl Into<String>) -> Self {
        let batch = batch.into();
        PROCESS_BATCHES.lock().insert(batch.clone());
        self.commit_batch = Some(batch);
        self
    }

    /// The run whose batch this store's ingests join.
    #[must_use]
    pub fn commit_batch(&self) -> Option<&str> {
        self.commit_batch.as_deref()
    }

    /// Whether this store's retrieval sees `entry`: a committed entry, or
    /// one of a batch this process writes.
    pub(crate) fn visible(&self, entry: &KnowledgeEntry) -> bool {
        let Some(batch) = entry.commit_batch.as_deref() else {
            return true;
        };
        self.commit_batch.as_deref() == Some(batch) || PROCESS_BATCHES.lock().contains(batch)
    }

    /// Stop seeing the uncommitted entries of `batch` from this process's
    /// other stores: the batch has committed or rolled back.
    pub(crate) fn release_batch(batch: &str) {
        PROCESS_BATCHES.lock().remove(batch);
    }

    /// Construct a store from a `.roko/` root.
    ///
    /// The resulting file is `.roko/neuro/knowledge.jsonl`.
    #[must_use]
    pub fn for_roko_dir(roko_dir: impl AsRef<Path>) -> Self {
        Self::new(roko_dir.as_ref().join("neuro").join("knowledge.jsonl"))
    }

    /// Construct a store from a workspace root.
    ///
    /// The resulting file is `<workdir>/.roko/neuro/knowledge.jsonl`.
    #[must_use]
    pub fn for_workdir(workdir: impl AsRef<Path>) -> Self {
        Self::new(
            workdir
                .as_ref()
                .join(".roko")
                .join("neuro")
                .join("knowledge.jsonl"),
        )
    }

    /// Construct a store from an existing Roko layout.
    #[must_use]
    pub fn for_layout(layout: &roko_fs::RokoLayout) -> Self {
        Self::for_roko_dir(layout.root())
    }

    /// Path of the backing JSONL file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Path of the confirmation records JSONL file.
    #[must_use]
    pub fn confirmations_path(&self) -> &Path {
        &self.confirmations_path
    }

    /// Enable the optional temporal topology and index all durable entries.
    ///
    /// Existing entries receive open-ended intervals starting at their
    /// creation timestamps. New entries and removals are kept synchronized by
    /// the store's write paths.
    pub fn enable_temporal_index(&mut self) -> Result<()> {
        let mut index = TemporalIndex::new();
        for entry in self.read_all()? {
            index.add_entry(
                entry.id,
                TemporalInterval::new(entry.created_at.timestamp_millis(), i64::MAX),
            );
        }
        self.temporal_index = Some(Arc::new(Mutex::new(index)));
        Ok(())
    }

    /// Register an epoch in the optional temporal topology.
    ///
    /// Returns `false` when temporal indexing has not been enabled.
    pub fn add_temporal_epoch(&self, epoch: KnowledgeEpoch) -> bool {
        let Some(index) = &self.temporal_index else {
            return false;
        };
        index.lock().add_epoch(epoch);
        true
    }

    /// Query entries created during a registered temporal epoch.
    pub fn query_temporal(&self, epoch_seq: u64) -> Result<Vec<KnowledgeEntry>> {
        let Some(index) = &self.temporal_index else {
            return Ok(Vec::new());
        };
        let ids = index.lock().entries_in_epoch(epoch_seq);
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids = ids.into_iter().collect::<HashSet<_>>();
        Ok(self
            .read_all()?
            .into_iter()
            .filter(|entry| ids.contains(&entry.id))
            .collect())
    }

    /// Compute the Allen relation between two indexed knowledge entries.
    pub fn query_temporal_relation(
        &self,
        source_id: &str,
        target_id: &str,
    ) -> Result<Option<AllenRelation>> {
        Ok(self
            .temporal_index
            .as_ref()
            .and_then(|index| index.lock().relation(source_id, target_id)))
    }

    // ── I/O helpers ──────────────────────────────────────────────────

    /// Hold this store's file for one read-modify-write cycle (bug-c4f0ed):
    /// the write gate that this process's stores share for the file, and
    /// then an exclusive lock on its `.lock` sibling, which another process
    /// writing the file takes too (`roko serve` beside a run, or `roko
    /// knowledge gc`). Every write method takes it; `roko knowledge restore`
    /// holds it while it swaps the file (bug-d81257).
    ///
    /// The gate is not re-entrant: a caller holding the guard must not call
    /// a write method of any store of the file, or it waits for itself.
    pub fn lock_writes(&self) -> WriteGuard<'_> {
        WriteGuard::hold(&self.write_gate, &self.path)
    }

    /// Read all knowledge entries from the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the store file cannot be read. Malformed nonblank
    /// legacy records are skipped on this compatibility path; imports use a
    /// strict reader before any rewrite.
    pub fn read_all(&self) -> Result<Vec<KnowledgeEntry>> {
        self.read_all_impl(false)
    }

    /// Read every knowledge entry and reject malformed nonblank records.
    ///
    /// This stricter path is used before import rewrites so a damaged existing
    /// store can never be silently shortened by a successful restore.
    pub(crate) fn read_all_strict(&self) -> Result<Vec<KnowledgeEntry>> {
        self.read_all_impl(true)
    }

    fn read_all_impl(&self, strict: bool) -> Result<Vec<KnowledgeEntry>> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("open knowledge store at {}", self.path.display()));
            }
        };

        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for (line_idx, line) in reader.lines().enumerate() {
            let line = line.with_context(|| {
                format!(
                    "read knowledge line {} from {}",
                    line_idx + 1,
                    self.path.display()
                )
            })?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<KnowledgeEntry>(&line) {
                Ok(entry) => entries.push(normalize_entry_security(entry)),
                Err(error) if strict => {
                    return Err(error).with_context(|| {
                        format!(
                            "decode knowledge line {} from {}",
                            line_idx + 1,
                            self.path.display()
                        )
                    });
                }
                Err(_) => {}
            }
        }
        Ok(entries)
    }

    /// Read all confirmation records from the confirmations JSONL file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read.
    pub fn read_confirmations(&self) -> Result<Vec<KnowledgeConfirmationRecord>> {
        let file = match File::open(&self.confirmations_path) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => {
                return Err(err).with_context(|| {
                    format!(
                        "open confirmations file at {}",
                        self.confirmations_path.display()
                    )
                });
            }
        };

        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line.context("read confirmation line")?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<KnowledgeConfirmationRecord>(&line) {
                records.push(record);
            }
        }
        Ok(records)
    }

    pub(crate) fn append_confirmations(
        &self,
        records: &[KnowledgeConfirmationRecord],
    ) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        if let Some(parent) = self.confirmations_path.parent() {
            fs::create_dir_all(parent).context("create confirmations directory")?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.confirmations_path)
            .with_context(|| {
                format!(
                    "open confirmations file at {}",
                    self.confirmations_path.display()
                )
            })?;
        for record in records {
            let mut line =
                serde_json::to_string(record).context("serialize confirmation record")?;
            line.push('\n');
            file.write_all(line.as_bytes())
                .context("append confirmation record")?;
        }
        file.flush().context("flush confirmation records")?;
        file.sync_all().context("sync confirmation records")?;
        Ok(())
    }

    /// Check if an entry's confidence warrants tier promotion or demotion.
    pub(crate) fn maybe_adjust_tier(entry: &mut KnowledgeEntry) {
        use crate::KnowledgeTier;
        if entry.confidence >= 0.9
            && entry.tier.multiplier() < KnowledgeTier::Consolidated.multiplier()
        {
            entry.tier = KnowledgeTier::Consolidated;
        }

        if entry.confidence <= 0.2
            && entry.tier.multiplier() > KnowledgeTier::Transient.multiplier()
        {
            entry.tier = KnowledgeTier::Transient;
        }

        if entry.confidence <= 0.05 {
            entry.half_life_days = 1.0;
        }
    }

    pub(crate) fn register_temporal_entries(&self, entries: &[KnowledgeEntry]) {
        let Some(index) = &self.temporal_index else {
            return;
        };
        let mut index = index.lock();
        for entry in entries {
            index.add_entry(
                entry.id.clone(),
                TemporalInterval::new(entry.created_at.timestamp_millis(), i64::MAX),
            );
        }
    }

    pub(crate) fn synchronize_temporal_entries(&self, entries: &[KnowledgeEntry]) {
        let Some(index) = &self.temporal_index else {
            return;
        };
        index.lock().replace_entries(entries.iter().map(|entry| {
            (
                entry.id.clone(),
                TemporalInterval::new(entry.created_at.timestamp_millis(), i64::MAX),
            )
        }));
    }

    pub(crate) fn rewrite_all(&self, entries: &[KnowledgeEntry]) -> Result<()> {
        let mut text = String::new();
        for entry in entries {
            let entry = normalize_entry_security(entry.clone());
            text.push_str(&serde_json::to_string(&entry).context("serialize knowledge entry")?);
            text.push('\n');
        }
        // Entries distilled from agent output can quote a secret.
        let text = roko_core::obs::scrub_secrets_in_jsonl(&text);
        roko_fs::atomic_write_bytes(&self.path, text.as_bytes()).with_context(|| {
            format!("atomically rewrite knowledge store {}", self.path.display())
        })?;
        Ok(())
    }
}

// ── NeuroStore trait impl ────────────────────────────────────────────

impl NeuroStore for KnowledgeStore {
    fn init(path: &Path) -> Result<Self> {
        Ok(Self::new(path))
    }

    fn query(&self, topic: &str, limit: usize) -> Result<Vec<KnowledgeEntry>> {
        KnowledgeStore::query(self, topic, limit)
    }

    fn query_similar(
        &self,
        fingerprint: &[u8],
        limit: usize,
    ) -> Result<Vec<KnowledgeSimilarityHit>> {
        KnowledgeStore::query_similar(self, fingerprint, limit)
    }

    fn ingest(&mut self, entries: Vec<KnowledgeEntry>) -> Result<()> {
        KnowledgeStore::ingest(self, entries)
    }

    fn decay(&mut self) -> Result<usize> {
        KnowledgeStore::decay(self)
    }

    fn gc(&mut self, min_confidence: f64) -> Result<usize> {
        KnowledgeStore::gc(self, min_confidence)
    }

    fn update_confidence(&mut self, knowledge_id: &str, delta: f64) -> Result<bool> {
        KnowledgeStore::update_confidence(self, knowledge_id, delta)
    }

    fn record_usage(&mut self, knowledge_id: &str, succeeded: bool) -> Result<()> {
        KnowledgeStore::record_usage(self, knowledge_id, succeeded)
    }

    fn batch_record_usage(&mut self, outcomes: &[(String, bool)]) -> Result<usize> {
        KnowledgeStore::batch_record_usage(self, outcomes)
    }
}
