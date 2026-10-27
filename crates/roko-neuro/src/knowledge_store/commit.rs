//! Guarded commit for the knowledge store (P21, 8137).
//!
//! What a run adds to the knowledge store waits in a batch. A store made
//! for the run ([`KnowledgeStore::with_commit_batch`]) tags each entry it
//! ingests with `commit_batch = <run id>`; the run's process sees the
//! batch, and other runs' retrieval skips every uncommitted entry. At the run's end (decision 8103)
//! [`propose_batch`] proposes the batch to the `knowledge` [`GuardedStore`]:
//! a commit clears the batch id, so that every run sees the entries, and a
//! rollback deletes them.
//!
//! [`KnowledgeChecks`]' precondition is `delivered` (bug-2ad4fa): a batch
//! commits only when its run delivered every plan it set out to. A run whose
//! plan failed its checks, that was cancelled or interrupted, or that ended
//! before deciding its batch, as a crashed run does, commits none of it, in
//! observe mode too: a check that passed on a try the run then failed to
//! deliver, or a gamed one, is no evidence for a lesson.
//!
//! It has no held-out check: M2 measures what knowledge is worth through
//! S02.P1-7's withhold arm. Its anchor check runs the `[[knowledge]]`
//! queries of the human-edited `.roko/policy/anchors.toml` on the store as
//! other runs will see it once the batch commits, each expected entry
//! staying in its query's top `k`, and checks invariants: no entry from an
//! unverified outcome, a bounded batch, no duplicate content.
//!
//! A person's rollback ([`LaterBatches`]) deletes the batches committed
//! after the version rolled back to.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use roko_learn::guarded_commit::{
    CheckOutcome, CommitCheck, CommitDecision, GuardError, GuardMode, GuardedState, GuardedStore,
    Proposer,
};
use roko_learn::router_commit::ANCHORS_FILE;
use serde::Deserialize;
use serde_json::Value;

use super::KnowledgeStore;
use crate::{KnowledgeEntry, SourceChannel};

/// The knowledge store's store under `.roko/learn/commits/`.
pub const KNOWLEDGE_STORE: &str = "knowledge";
/// The most entries one run's batch may add.
pub const MAX_BATCH_ENTRIES: usize = 200;

/// One `[[knowledge]]` query of the anchors file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeAnchor {
    /// The query.
    pub query: String,
    /// The id of the entry that must stay among its top `k` hits.
    pub expect: String,
    /// How many hits count; 5 when absent.
    #[serde(default = "default_anchor_k")]
    pub k: usize,
}

const fn default_anchor_k() -> usize {
    5
}

/// The anchors file's tables the knowledge store reads; the router reads
/// its own.
#[derive(Debug, Default, Deserialize)]
struct AnchorsFile {
    #[serde(default)]
    knowledge: Vec<KnowledgeAnchor>,
}

/// The `[[knowledge]]` queries of `.roko/policy/anchors.toml` under
/// `roko_dir`; none without the file.
///
/// # Errors
///
/// The read or parse error of a file that is there.
pub fn load_knowledge_anchors(roko_dir: &Path) -> Result<Vec<KnowledgeAnchor>, String> {
    let path = roko_dir.join(ANCHORS_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let file: AnchorsFile =
        toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(file.knowledge)
}

/// A run's batch in a knowledge store: the entries whose `commit_batch` is
/// the run's id.
#[derive(Debug, Clone)]
pub struct KnowledgeBatch {
    store: KnowledgeStore,
    batch: String,
}

impl KnowledgeBatch {
    /// The batch of the run `batch` in `store`.
    #[must_use]
    pub fn new(store: &KnowledgeStore, batch: impl Into<String>) -> Self {
        Self {
            store: store.clone(),
            batch: batch.into(),
        }
    }

    /// The batch's entries, by id.
    ///
    /// # Errors
    ///
    /// The store's read error.
    pub fn entries(&self) -> anyhow::Result<Vec<KnowledgeEntry>> {
        let mut entries: Vec<KnowledgeEntry> = self
            .store
            .read_all()?
            .into_iter()
            .filter(|entry| self.holds(entry))
            .collect();
        entries.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(entries)
    }

    /// Commit the batch: its entries lose the batch id, so that every run's
    /// retrieval sees them, and the confirmations its run held apply, with
    /// their records (gap-1220c5). Returns how many entries.
    ///
    /// # Errors
    ///
    /// The store's, the ledger's or the confirmation log's read or write
    /// error.
    pub fn commit(&self) -> anyhow::Result<usize> {
        let _guard = self.store.lock_writes();
        let mut entries = self.store.read_all()?;
        let mut held = 0;
        for entry in &mut entries {
            if self.holds(entry) {
                entry.commit_batch = None;
                held += 1;
            }
        }
        let (applied, records) = self.store.apply_pending(&self.batch, &mut entries)?;
        if held > 0 || applied > 0 {
            self.store.rewrite_all(&entries)?;
        }
        self.store.append_confirmations(&records)?;
        self.store.drop_pending(&self.batch)?;
        Ok(held)
    }

    /// Roll the batch back: delete its entries, and drop the confirmations
    /// its run held (gap-1220c5). Returns how many entries.
    ///
    /// # Errors
    ///
    /// The store's or the ledger's read or rewrite error.
    pub fn discard(&self) -> anyhow::Result<usize> {
        let _guard = self.store.lock_writes();
        let mut entries = self.store.read_all()?;
        let before = entries.len();
        entries.retain(|entry| !self.holds(entry));
        let removed = before - entries.len();
        if removed > 0 {
            self.store.rewrite_all(&entries)?;
            self.store.synchronize_temporal_entries(&entries);
        }
        self.store.drop_pending(&self.batch)?;
        Ok(removed)
    }

    fn holds(&self, entry: &KnowledgeEntry) -> bool {
        entry.commit_batch.as_deref() == Some(self.batch.as_str())
    }
}

impl GuardedState for KnowledgeBatch {
    /// The batch's entries as they commit, without the batch id: one JSON
    /// line each, by id.
    fn snapshot(&self) -> Vec<u8> {
        let entries = self.entries().unwrap_or_else(|error| {
            tracing::warn!(
                batch = %self.batch,
                error = %format!("{error:#}"),
                "knowledge batch unread"
            );
            Vec::new()
        });
        let mut snapshot = Vec::new();
        for mut entry in entries {
            entry.commit_batch = None;
            if let Ok(line) = serde_json::to_vec(&entry) {
                snapshot.extend(line);
                snapshot.push(b'\n');
            }
        }
        snapshot
    }

    /// A rollback deletes the batch's entries; the last committed batch,
    /// `snapshot`, stays in the store as it is.
    fn restore(&mut self, _snapshot: &[u8]) -> Result<(), GuardError> {
        self.discard()
            .map(|_| ())
            .map_err(|error| store_error(&self.store, &error))
    }
}

/// Whether the run that learned a batch delivered what it set out to
/// (bug-2ad4fa): the batch commits only then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivery {
    /// The run delivered every plan; what it delivered, in words.
    Delivered(String),
    /// It did not; why, in words, such as "plan p5-ship interrupted; a
    /// cancel request stopped the run".
    Undelivered(String),
}

/// The knowledge store's commit checks (decision 8103).
#[derive(Debug, Clone)]
pub struct KnowledgeChecks {
    /// The store as other runs will see it once the batch commits.
    store: KnowledgeStore,
    /// The anchors file's `[[knowledge]]` queries.
    pub anchors: Vec<KnowledgeAnchor>,
    /// The most entries the batch may add.
    pub max_batch: usize,
    /// Whether the batch's run delivered: the precondition.
    pub delivery: Delivery,
}

impl KnowledgeChecks {
    /// The checks of the run `batch`'s batch in `store`, with `anchors`, for
    /// a run that ended as `delivery` says.
    #[must_use]
    pub fn new(
        store: &KnowledgeStore,
        batch: &str,
        anchors: Vec<KnowledgeAnchor>,
        delivery: Delivery,
    ) -> Self {
        Self {
            store: store.clone().with_commit_batch(batch),
            anchors,
            max_batch: MAX_BATCH_ENTRIES,
            delivery,
        }
    }

    /// The first broken invariant of `batch`: more than `max_batch` entries,
    /// an entry from an unverified outcome (one that cites no attempt, or
    /// an agent's own claim or a dream's), or content that a committed entry
    /// or another of the batch has.
    fn invariants(&self, batch: &[KnowledgeEntry]) -> Result<(), String> {
        let (count, limit) = (batch.len(), self.max_batch);
        if count > limit {
            return Err(format!(
                "{count} entries, above the {limit} a batch may add"
            ));
        }
        let unverified = [
            SourceChannel::AgentOutput.as_str(),
            SourceChannel::DreamConsolidation.as_str(),
        ];
        let committed = self
            .store
            .read_all()
            .map_err(|error| format!("the store could not be read: {error:#}"))?;
        let mut contents: HashSet<String> = committed
            .iter()
            .filter(|entry| entry.commit_batch.is_none())
            .map(content_key)
            .collect();
        for entry in batch {
            let claimed = entry
                .source
                .as_deref()
                .is_some_and(|source| unverified.contains(&source));
            if claimed || entry.source_episodes.is_empty() {
                return Err(format!("{} comes from no verified outcome", entry.id));
            }
            if !contents.insert(content_key(entry)) {
                return Err(format!("{} repeats another entry's content", entry.id));
            }
        }
        Ok(())
    }

    /// Whether `anchor`'s expected entry is among its query's top `k` hits.
    /// An anchor whose entry is gone checks nothing.
    fn anchor_holds(&self, anchor: &KnowledgeAnchor) -> anyhow::Result<bool> {
        let hits = self.store.query(&anchor.query, anchor.k.max(1))?;
        if hits.iter().any(|entry| entry.id == anchor.expect) {
            return Ok(true);
        }
        let entries = self.store.read_all()?;
        Ok(!entries.iter().any(|entry| entry.id == anchor.expect))
    }
}

impl CommitCheck for KnowledgeChecks {
    fn held_out(&self, _candidate: &[u8], _lkg: Option<&[u8]>) -> CheckOutcome {
        CheckOutcome::pass(
            "held_out",
            "none at commit: M2 measures knowledge through its withhold arm (decision 8103)",
        )
    }

    fn anchors(&self, candidate: &[u8]) -> CheckOutcome {
        const CHECK: &str = "anchors";
        let text = String::from_utf8_lossy(candidate);
        let parsed: Result<Vec<KnowledgeEntry>, _> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(serde_json::from_str)
            .collect();
        let Ok(batch) = parsed else {
            return CheckOutcome::fail(CHECK, "the candidate is not a batch of entries");
        };
        if let Err(problem) = self.invariants(&batch) {
            return CheckOutcome::fail(CHECK, problem);
        }
        for anchor in &self.anchors {
            let problem = match self.anchor_holds(anchor) {
                Ok(true) => continue,
                Ok(false) => format!(
                    "`{}` no longer finds {} among its top {}",
                    anchor.query, anchor.expect, anchor.k
                ),
                Err(error) => format!("`{}` could not run: {error:#}", anchor.query),
            };
            return CheckOutcome::fail(CHECK, problem);
        }
        // With no anchors the check measures nothing but the invariants, and
        // says so (bug-2ad4fa).
        let reason = if self.anchors.is_empty() {
            format!(
                "no [[knowledge]] anchors in .roko/{ANCHORS_FILE}, so no query was checked; \
                 the invariants hold"
            )
        } else {
            format!(
                "{} knowledge anchors hold, and the invariants",
                self.anchors.len()
            )
        };
        CheckOutcome::pass(CHECK, reason)
            .with("anchors", self.anchors.len() as f64)
            .with("batch_entries", batch.len() as f64)
    }

    /// `delivered`: the batch's run delivered every plan (bug-2ad4fa).
    ///
    /// A batch whose run did not is rolled back whatever the mode, and the
    /// reason says how many lessons went with it.
    fn precondition(&self, candidate: &[u8]) -> Option<CheckOutcome> {
        const CHECK: &str = "delivered";
        let lessons = candidate
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.trim_ascii().is_empty())
            .count();
        let outcome = match &self.delivery {
            Delivery::Delivered(what) => CheckOutcome::pass(CHECK, what.clone()),
            Delivery::Undelivered(why) => {
                let kept = match lessons {
                    1 => "the batch's 1 lesson was not kept".to_string(),
                    count => format!("none of the batch's {count} lessons was kept"),
                };
                CheckOutcome::fail(CHECK, format!("{why}; {kept}"))
            }
        };
        Some(outcome.with("batch_entries", lessons as f64))
    }
}

/// Propose the run `batch`'s entries in `store` to the guarded store.
///
/// The proposal runs under `checks` (P21, 8137) against the guarded store
/// under `learn_dir`, in `mode`, and returns the decision: committed, the
/// entries losing the batch id; rolled back, the entries deleted, which a
/// batch whose run did not deliver always is (bug-2ad4fa); or observed,
/// committed with a row that records the rollback that would have happened.
/// `None` when the batch holds no entry; the confirmations its run held then
/// settle by its delivery alone (gap-1220c5).
///
/// The store's first proposal commits an empty baseline version first, so
/// that a rollback in enforce mode has a version to keep.
///
/// # Errors
///
/// Guarded store errors, and the store's read and rewrite errors.
pub fn propose_batch(
    store: &KnowledgeStore,
    batch: &str,
    learn_dir: &Path,
    mode: GuardMode,
    checks: &KnowledgeChecks,
    proposer: &Proposer,
) -> Result<Option<CommitDecision>, GuardError> {
    let mut state = KnowledgeBatch::new(store, batch);
    let entries = state
        .entries()
        .map_err(|error| store_error(store, &error))?;
    if entries.is_empty() {
        // A run that stored no lesson can still hold confirmations: they
        // settle by its delivery alone, with no version to propose
        // (gap-1220c5).
        let settled = match checks.delivery {
            Delivery::Delivered(_) => state.commit(),
            Delivery::Undelivered(_) => state.discard(),
        };
        settled.map_err(|error| store_error(store, &error))?;
        KnowledgeStore::release_batch(batch);
        return Ok(None);
    }
    let mut guarded = GuardedStore::open(learn_dir, KNOWLEDGE_STORE, mode)?;
    if guarded.current().is_none() {
        let reason = "baseline: the store before its first guarded batch";
        let baseline = Proposer::evolved(proposer.actor.clone(), KNOWLEDGE_STORE, reason);
        guarded.propose(&mut Baseline, &Baseline, &baseline)?;
    }
    let decision = guarded.propose(&mut state, checks, proposer)?;
    if decision != CommitDecision::RolledBack {
        state.commit().map_err(|error| store_error(store, &error))?;
    }
    KnowledgeStore::release_batch(batch);
    Ok(Some(decision))
}

/// The batches `store` holds besides `live`, sorted.
///
/// At a run's start these are the orphans of runs that ended before they
/// proposed theirs, as a run that crashed leaves its batch (gap-6ff99a). A
/// batch that holds only confirmations is one too (gap-1220c5).
///
/// # Errors
///
/// The store's or the ledger's read error.
pub fn orphaned_batches(
    store: &KnowledgeStore,
    live: Option<&str>,
) -> Result<Vec<String>, GuardError> {
    let entries = store
        .read_all()
        .map_err(|error| store_error(store, &error))?;
    let mut batches: BTreeSet<String> = entries
        .into_iter()
        .filter_map(|entry| entry.commit_batch)
        .collect();
    let held = store
        .pending_batches()
        .map_err(|error| store_error(store, &error))?;
    batches.extend(held);
    batches.retain(|batch| Some(batch.as_str()) != live);
    Ok(batches.into_iter().collect())
}

/// Why an orphaned batch commits none of its lessons: its run never reached
/// the end that would have said whether it delivered (bug-2ad4fa).
pub const ORPHAN_UNDELIVERED: &str = "its run ended before deciding its batch: it crashed, was \
                                      killed, or its process stopped, so nothing shows that it \
                                      delivered";

/// Propose every batch `store` holds besides `live` through the guard.
///
/// This works as [`propose_batch`] does for a run's own batch at its end
/// (gap-6ff99a): a batch whose run ended before proposing it, as a crashed
/// run's, goes through the guard under the same checks and mode, instead of
/// staying hidden from every other run. Nothing shows that such a run
/// delivered, so its batch is rolled back (bug-2ad4fa). Returns each orphan
/// with its decision.
///
/// The caller holds the workspace's runner lock, so no other run is writing a
/// batch.
///
/// # Errors
///
/// As [`propose_batch`], at the first orphan whose proposal fails; the ones
/// before it stay proposed.
pub fn recover_orphaned_batches(
    store: &KnowledgeStore,
    live: Option<&str>,
    learn_dir: &Path,
    mode: GuardMode,
    anchors: &[KnowledgeAnchor],
) -> Result<Vec<(String, Option<CommitDecision>)>, GuardError> {
    let orphans = orphaned_batches(store, live)?;
    let mut recovered = Vec::with_capacity(orphans.len());
    for batch in orphans {
        let undelivered = Delivery::Undelivered(ORPHAN_UNDELIVERED.to_string());
        let checks = KnowledgeChecks::new(store, &batch, anchors.to_vec(), undelivered);
        let reason = format!("knowledge:{batch} (orphaned: its run ended before proposing it)");
        let proposer = Proposer::evolved("knowledge", KNOWLEDGE_STORE, reason);
        let decision = propose_batch(store, &batch, learn_dir, mode, &checks, &proposer)?;
        recovered.push((batch, decision));
    }
    Ok(recovered)
}

/// The knowledge store as of a kept version, for a person's rollback (P21, 8139).
///
/// Restoring it deletes the entries of every batch committed after that
/// version, which the store's later versions hold. The version's own batch
/// and the earlier ones stay as they are: a rollback re-adds nothing, so an
/// entry a person retired or removed stays so.
#[derive(Debug)]
pub struct LaterBatches {
    store: KnowledgeStore,
    ids: Vec<String>,
}

impl LaterBatches {
    /// The entries of the batches `guarded` keeps after version `to`, in
    /// `store`.
    ///
    /// # Errors
    ///
    /// I/O errors reading the kept versions.
    pub fn after(
        guarded: &GuardedStore,
        to: u64,
        store: KnowledgeStore,
    ) -> Result<Self, GuardError> {
        let mut ids = Vec::new();
        let versions = guarded.versions()?;
        for version in versions.into_iter().filter(|&version| version > to) {
            let snapshot = guarded.snapshot(version)?;
            for line in String::from_utf8_lossy(&snapshot).lines() {
                if let Ok(entry) = serde_json::from_str::<Value>(line)
                    && let Some(id) = entry["id"].as_str()
                {
                    ids.push(id.to_string());
                }
            }
        }
        Ok(Self { store, ids })
    }
}

impl GuardedState for LaterBatches {
    /// The ids of the entries a rollback deletes, one per line.
    fn snapshot(&self) -> Vec<u8> {
        self.ids.join("\n").into_bytes()
    }

    /// Delete the later batches' entries; the version's own batch stays.
    fn restore(&mut self, _snapshot: &[u8]) -> Result<(), GuardError> {
        let ids: Vec<&str> = self.ids.iter().map(String::as_str).collect();
        self.store
            .remove_entries(&ids)
            .map(|_| ())
            .map_err(|error| store_error(&self.store, &error))
    }
}

/// The empty version a knowledge store's guard starts from: it passes
/// every check.
struct Baseline;

impl GuardedState for Baseline {
    fn snapshot(&self) -> Vec<u8> {
        Vec::new()
    }

    fn restore(&mut self, _snapshot: &[u8]) -> Result<(), GuardError> {
        Ok(())
    }
}

impl CommitCheck for Baseline {
    fn held_out(&self, _candidate: &[u8], _lkg: Option<&[u8]>) -> CheckOutcome {
        CheckOutcome::pass("held_out", "the empty baseline")
    }

    fn anchors(&self, _candidate: &[u8]) -> CheckOutcome {
        CheckOutcome::pass("anchors", "the empty baseline")
    }
}

/// A batch's text: its words, lowercase.
fn content_key(entry: &KnowledgeEntry) -> String {
    let words: Vec<&str> = entry.content.split_whitespace().collect();
    words.join(" ").to_lowercase()
}

/// A store read or rewrite error, as a guarded store error.
fn store_error(store: &KnowledgeStore, error: &anyhow::Error) -> GuardError {
    GuardError::Io {
        path: store.path().display().to_string(),
        source: std::io::Error::other(format!("{error:#}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_learn::guarded_commit::CommitDecision::{Committed, RolledBack};

    /// A verified attempt's entry.
    fn entry(id: &str, content: &str, tags: &[&str], confidence: f64) -> KnowledgeEntry {
        KnowledgeEntry {
            id: id.to_string(),
            content: content.to_string(),
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
            confidence,
            confidence_weight: confidence,
            source: Some(SourceChannel::GateVerdict.as_str().to_string()),
            source_episodes: vec![format!("attempt-{id}")],
            ..KnowledgeEntry::default()
        }
    }

    /// A run that delivered.
    fn delivered() -> Delivery {
        Delivery::Delivered("every plan of the run was delivered".to_string())
    }

    /// The ids `store`'s retrieval finds for `query`, best first.
    fn found(store: &KnowledgeStore, query: &str) -> Vec<String> {
        let hits = store.query(query, 5).expect("query");
        hits.into_iter().map(|entry| entry.id).collect()
    }

    /// P21 (8137): a batch that pushes an anchor query's expected entry out
    /// of its top k is rolled back, its entries deleted, and a neutral batch
    /// commits; no other run sees a batch before it commits.
    #[test]
    fn knowledge_batch_rolled_back_when_anchor_query_regresses() {
        let temp = tempfile::tempdir().expect("tempdir");
        let roko_dir = temp.path().join(".roko");
        let learn_dir = roko_dir.join("learn");
        let store = KnowledgeStore::for_roko_dir(&roko_dir);
        let policy = roko_dir.join(ANCHORS_FILE);
        std::fs::create_dir_all(policy.parent().expect("policy dir")).expect("policy dir");
        std::fs::write(
            &policy,
            "[[knowledge]]\nquery = \"retry backoff\"\nexpect = \"kn-anchor\"\nk = 1\n",
        )
        .expect("write the anchors");
        let anchors = load_knowledge_anchors(&roko_dir).expect("the anchors parse");
        let proposer = Proposer::evolved("knowledge", KNOWLEDGE_STORE, "knowledge:test");

        // The anchored entry, committed, and run 7's batch, which outranks it.
        let anchor = entry("kn-anchor", "Retry backoff doubles the wait.", &[], 0.5);
        let poisoned = KnowledgeEntry {
            commit_batch: Some("run-7".to_string()),
            ..entry(
                "kn-poison",
                "retry backoff retry backoff: never wait",
                &["retry backoff"],
                0.95,
            )
        };
        store
            .rewrite_all(&[anchor, poisoned])
            .expect("seed the store");
        assert_eq!(found(&store, "retry backoff"), ["kn-anchor"]);
        let run7 = store.clone().with_commit_batch("run-7");
        assert_eq!(found(&run7, "retry backoff")[0], "kn-poison");

        let checks = KnowledgeChecks::new(&store, "run-7", anchors.clone(), delivered());
        let decision = propose_batch(
            &store,
            "run-7",
            &learn_dir,
            GuardMode::Enforce,
            &checks,
            &proposer,
        )
        .expect("proposed");
        assert_eq!(decision, Some(RolledBack));
        let ids: Vec<String> = store
            .read_all()
            .expect("read")
            .into_iter()
            .map(|entry| entry.id)
            .collect();
        assert_eq!(ids, ["kn-anchor"]);

        // Run 8's batch leaves the anchor alone and commits.
        let neutral = KnowledgeEntry {
            commit_batch: Some("run-8".to_string()),
            ..entry(
                "kn-theme",
                "The dashboard takes its colours from the theme.",
                &[],
                0.8,
            )
        };
        let mut entries = store.read_all().expect("read");
        entries.push(neutral);
        store.rewrite_all(&entries).expect("add run 8's entry");
        assert!(found(&store, "dashboard colours").is_empty());
        let checks = KnowledgeChecks::new(&store, "run-8", anchors, delivered());
        let decision = propose_batch(
            &store,
            "run-8",
            &learn_dir,
            GuardMode::Enforce,
            &checks,
            &proposer,
        )
        .expect("proposed");
        assert_eq!(decision, Some(Committed));
        assert_eq!(found(&store, "dashboard colours"), ["kn-theme"]);
        let entries = store.read_all().expect("read");
        assert!(entries.iter().all(|entry| entry.commit_batch.is_none()));

        // The baseline, the rollback and the commit.
        let guarded = GuardedStore::open(&learn_dir, KNOWLEDGE_STORE, GuardMode::Enforce)
            .expect("the knowledge store's guard");
        let rows = guarded.rows().expect("its rows");
        let decisions: Vec<CommitDecision> = rows.iter().map(|row| row.decision).collect();
        assert_eq!(decisions, [Committed, RolledBack, Committed]);
        let anchors_check = &rows[1].checks[2];
        assert!(!anchors_check.passed, "{anchors_check:?}");
        assert!(anchors_check.reason.contains("kn-anchor"));
    }

    /// gap-6ff99a: at the next run's start, the batches of runs that crashed
    /// before proposing them go through the guard, and the running run's own
    /// batch waits for its end. Nothing shows that a crashed run delivered,
    /// so even a sound orphan is rolled back, in observe mode too, and its
    /// row says why (bug-2ad4fa).
    #[test]
    fn orphaned_knowledge_batch_is_recovered_at_next_run_start() {
        let temp = tempfile::tempdir().expect("tempdir");
        let roko_dir = temp.path().join(".roko");
        let learn_dir = roko_dir.join("learn");
        let store = KnowledgeStore::for_roko_dir(&roko_dir);
        let tagged = |entry: KnowledgeEntry, batch: &str| KnowledgeEntry {
            commit_batch: Some(batch.to_string()),
            ..entry
        };
        let sound = entry("kn-sound", "Pin the toolchain before a release.", &[], 0.8);
        let claim = KnowledgeEntry {
            source_episodes: Vec::new(),
            ..entry("kn-claim", "Every test is flaky.", &[], 0.8)
        };
        let live = entry("kn-live", "The run in progress learned this.", &[], 0.8);
        let seeded = [
            tagged(sound, "run-crashed"),
            tagged(claim, "run-claimed"),
            tagged(live, "run-live"),
        ];
        store.rewrite_all(&seeded).expect("seed the store");
        assert!(found(&store, "toolchain release").is_empty());

        let recovered = recover_orphaned_batches(
            &store,
            Some("run-live"),
            &learn_dir,
            GuardMode::Observe,
            &[],
        )
        .expect("recovered");
        let expected = [
            ("run-claimed".to_string(), Some(RolledBack)),
            ("run-crashed".to_string(), Some(RolledBack)),
        ];
        assert_eq!(recovered, expected);

        assert!(found(&store, "toolchain release").is_empty());
        let left: Vec<(String, Option<String>)> = store
            .read_all()
            .expect("read")
            .into_iter()
            .map(|entry| (entry.id, entry.commit_batch))
            .collect();
        let live = ("kn-live".to_string(), Some("run-live".to_string()));
        assert_eq!(left, [live]);

        // The store's baseline, then one row per orphan, each saying why.
        let guarded = GuardedStore::open(&learn_dir, KNOWLEDGE_STORE, GuardMode::Observe)
            .expect("the knowledge store's guard");
        let rows = guarded.rows().expect("its rows");
        let decisions: Vec<CommitDecision> = rows.iter().map(|row| row.decision).collect();
        assert_eq!(decisions, [Committed, RolledBack, RolledBack]);
        let reason = rows[2].reason.as_deref().unwrap_or_default();
        assert!(reason.contains("orphaned"), "{reason}");
        let delivered = &rows[2].checks[0];
        assert_eq!(delivered.check, "delivered");
        assert!(!delivered.passed, "{delivered:?}");
        let expected = format!("{ORPHAN_UNDELIVERED}; the batch's 1 lesson was not kept");
        assert_eq!(delivered.reason, expected);
    }
}
