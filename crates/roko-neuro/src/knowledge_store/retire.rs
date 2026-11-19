//! Retiring a knowledge entry (bug-2ad4fa).
//!
//! A person retires an entry they judge wrong, such as a lesson a gamed pass
//! taught. [`KnowledgeStore::retire`] sets its `deprecated` flag, which
//! retrieval honours, and appends the retirement to the ledger
//! [`RETIRED_FILE`] beside the store. Every read of the store applies the
//! ledger, so an entry that a restore, an import, a sync or a rollback writes
//! back stays retired.
//!
//! A retired entry stays the dedup target of its lesson: a run that learns
//! the lesson again adds no copy and confirms nothing, and the ledger counts
//! the relearning ([`RetirementAction::Relearned`]), which `roko knowledge
//! stats` shows. A lesson that keeps coming back after a person retired it
//! is a signal for that person.

use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use chrono::{DateTime, Utc};
use roko_learn::guarded_commit::{COMMITS_DIR, GuardMode, GuardedStore};
use serde::{Deserialize, Serialize};

use super::KnowledgeStore;
use super::commit::KNOWLEDGE_STORE;
use crate::KnowledgeEntry;

/// The retirement ledger, beside the knowledge store's file.
pub const RETIRED_FILE: &str = "knowledge-retired.jsonl";
/// `schema` of a ledger row.
pub const RETIREMENT_SCHEMA: &str = "roko.knowledge_retirement/1";
/// The longest content or lesson a ledger row keeps, in characters.
const LEDGER_TEXT_CHARS: usize = 300;

/// What a ledger row records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetirementAction {
    /// A person retired the entry.
    Retired,
    /// A run learned the retired entry's lesson again, and the retired entry
    /// absorbed it.
    Relearned,
}

/// One row of the retirement ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetirementRow {
    /// [`RETIREMENT_SCHEMA`].
    pub schema: String,
    /// The entry.
    pub id: String,
    /// What happened.
    pub action: RetirementAction,
    /// When.
    pub at: DateTime<Utc>,
    /// Who: `human` for `roko knowledge retire`; for a relearning, the
    /// episode that learned the lesson again.
    pub actor: String,
    /// Why, in the person's words; for a relearning, the lesson as the run
    /// learned it again.
    pub reason: String,
    /// Where the retirement found the entry: `store`, or the guard's kept
    /// version that holds it, such as `v14`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub found_in: Option<String>,
    /// The entry's content when it was retired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// What [`KnowledgeStore::retire`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retired {
    /// The entry.
    pub id: String,
    /// `store`, or the guard's kept version that holds the entry the store
    /// no longer has, such as `v14`.
    pub found_in: String,
    /// The entry's content.
    pub content: String,
    /// Whether the ledger had retired it already, so that no row was added.
    pub already: bool,
}

/// A retired entry, as the ledger tells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RetiredEntry {
    /// The entry.
    pub id: String,
    /// Why a person retired it.
    pub reason: String,
    /// When.
    pub at: DateTime<Utc>,
    /// How many times a run learned its lesson again since.
    pub relearned: usize,
}

impl KnowledgeStore {
    /// The retirement ledger's path, beside the store's file.
    #[must_use]
    pub fn retired_path(&self) -> PathBuf {
        self.path.parent().map_or_else(
            || PathBuf::from(RETIRED_FILE),
            |parent| parent.join(RETIRED_FILE),
        )
    }

    /// Retire the entry `id` for `reason`: set `deprecated`, which retrieval
    /// honours, and record it in the ledger.
    ///
    /// An id the store no longer holds, as when a person removed it by hand,
    /// is retired when one of the guard's kept versions under `learn_dir`
    /// holds it, so that nothing can bring it back. `actor` is who retires
    /// it, such as `human`.
    ///
    /// # Errors
    ///
    /// An empty id or reason; an id found neither in the store nor in a kept
    /// version; the store's and the ledger's read and write errors.
    pub fn retire(&self, id: &str, reason: &str, actor: &str, learn_dir: &Path) -> Result<Retired> {
        let (id, reason) = (id.trim(), reason.trim());
        ensure!(!id.is_empty(), "name the knowledge entry to retire");
        ensure!(!reason.is_empty(), "say why the entry is retired");
        let _guard = self.lock_writes();
        let already = self.retired_ids().contains(id);
        let mut entries = self.read_all()?;
        let index = entries.iter().position(|entry| entry.id == id);
        let (found_in, content) = match index {
            Some(index) => ("store".to_string(), entries[index].content.clone()),
            None => match kept_version_holding(learn_dir, id)? {
                Some((version, content)) => (format!("v{version}"), content),
                None => bail!(
                    "no knowledge entry `{id}` in {} or in the knowledge guard's kept versions",
                    self.path.display()
                ),
            },
        };
        // The ledger first: once it holds the id, every read retires the
        // entry, even if the rewrite below never happens.
        if !already {
            let mut retirement = row(id, RetirementAction::Retired, actor, reason);
            retirement.found_in = Some(found_in.clone());
            retirement.content = Some(cut(&content));
            self.append_retirement(&retirement)?;
        }
        if let Some(index) = index {
            entries[index].deprecated = true;
            self.rewrite_all(&entries)?;
        }
        Ok(Retired {
            id: id.to_string(),
            found_in,
            content,
            already,
        })
    }

    /// The entries the ledger retires, in the order they were retired, each
    /// with how many times a run learned its lesson again since.
    ///
    /// # Errors
    ///
    /// The ledger's read error.
    pub fn retirements(&self) -> Result<Vec<RetiredEntry>> {
        let mut retired: Vec<RetiredEntry> = Vec::new();
        for row in self.retirement_rows()? {
            match row.action {
                RetirementAction::Retired => {
                    if !retired.iter().any(|entry| entry.id == row.id) {
                        retired.push(RetiredEntry {
                            id: row.id,
                            reason: row.reason,
                            at: row.at,
                            relearned: 0,
                        });
                    }
                }
                RetirementAction::Relearned => {
                    if let Some(entry) = retired.iter_mut().find(|entry| entry.id == row.id) {
                        entry.relearned += 1;
                    }
                }
            }
        }
        Ok(retired)
    }

    /// The ledger's rows, oldest first; none without the file. A row that
    /// does not parse is skipped.
    ///
    /// # Errors
    ///
    /// The ledger's read error.
    pub fn retirement_rows(&self) -> Result<Vec<RetirementRow>> {
        let path = self.retired_path();
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(error).with_context(|| format!("open {}", path.display()));
            }
        };
        let mut rows = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line.with_context(|| format!("read {}", path.display()))?;
            if let Ok(row) = serde_json::from_str::<RetirementRow>(&line) {
                rows.push(row);
            }
        }
        Ok(rows)
    }

    /// The ids the ledger retires; none when it is missing or unreadable,
    /// with a warning. Every read of the store marks them `deprecated`.
    pub(crate) fn retired_ids(&self) -> BTreeSet<String> {
        match self.retirement_rows() {
            Ok(rows) => rows
                .into_iter()
                .filter(|row| row.action == RetirementAction::Retired)
                .map(|row| row.id)
                .collect(),
            Err(error) => {
                let error = format!("{error:#}");
                tracing::warn!(%error, "the knowledge retirement ledger is unreadable");
                BTreeSet::new()
            }
        }
    }

    /// Count a relearning of the retired entry `id`: the episode `episode`
    /// learned its lesson again, as `lesson`.
    ///
    /// # Errors
    ///
    /// The ledger's write error.
    pub(crate) fn record_relearning(&self, id: &str, episode: &str, lesson: &str) -> Result<()> {
        self.append_retirement(&row(id, RetirementAction::Relearned, episode, lesson))
    }

    /// Append `row` to the ledger.
    fn append_retirement(&self, row: &RetirementRow) -> Result<()> {
        let path = self.retired_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("open {}", path.display()))?;
        let mut line = serde_json::to_string(row).context("serialize a retirement")?;
        line.push('\n');
        file.write_all(line.as_bytes())
            .and_then(|()| file.sync_all())
            .with_context(|| format!("append to {}", path.display()))
    }
}

/// The newest of the guard's kept versions under `learn_dir` that holds the
/// entry `id`, with the entry's content.
fn kept_version_holding(learn_dir: &Path, id: &str) -> Result<Option<(u64, String)>> {
    if !learn_dir.join(COMMITS_DIR).join(KNOWLEDGE_STORE).is_dir() {
        return Ok(None);
    }
    let guarded = GuardedStore::open(learn_dir, KNOWLEDGE_STORE, GuardMode::Observe)?;
    for version in guarded.versions()?.into_iter().rev() {
        let snapshot = guarded.snapshot(version)?;
        for line in String::from_utf8_lossy(&snapshot).lines() {
            if let Ok(entry) = serde_json::from_str::<KnowledgeEntry>(line)
                && entry.id == id
            {
                return Ok(Some((version, entry.content)));
            }
        }
    }
    Ok(None)
}

/// The first [`LEDGER_TEXT_CHARS`] characters of `text`.
fn cut(text: &str) -> String {
    text.chars().take(LEDGER_TEXT_CHARS).collect()
}

/// A row `actor` writes now.
fn row(id: &str, action: RetirementAction, actor: &str, reason: &str) -> RetirementRow {
    RetirementRow {
        schema: RETIREMENT_SCHEMA.to_string(),
        id: id.to_string(),
        action,
        at: Utc::now(),
        actor: actor.to_string(),
        reason: cut(reason),
        found_in: None,
        content: None,
    }
}

#[cfg(test)]
mod tests {
    use roko_learn::guarded_commit::{CommitDecision, Proposer};

    use super::*;
    use crate::SourceChannel;
    use crate::knowledge_store::commit::{Delivery, KnowledgeChecks, LaterBatches, propose_batch};

    /// A lesson of the run `batch`.
    fn lesson(id: &str, content: &str, batch: &str) -> KnowledgeEntry {
        KnowledgeEntry {
            id: id.to_string(),
            content: content.to_string(),
            tags: vec!["lesson".to_string()],
            confidence: 0.87,
            confidence_weight: 0.87,
            source: Some(SourceChannel::GateVerdict.as_str().to_string()),
            source_episodes: vec![format!("{batch}:p5-ship:{id}")],
            commit_batch: Some(batch.to_string()),
            ..KnowledgeEntry::default()
        }
    }

    /// The ids `store`'s retrieval finds for `topic`.
    fn found(store: &KnowledgeStore, topic: &str) -> Vec<String> {
        let hits = store.query(topic, 10).expect("query");
        hits.into_iter().map(|entry| entry.id).collect()
    }

    /// The store's entry `id`, as a read gives it.
    fn read(store: &KnowledgeStore, id: &str) -> Option<KnowledgeEntry> {
        let entries = store.read_all().expect("read");
        entries.into_iter().find(|entry| entry.id == id)
    }

    /// bug-2ad4fa: a person retires the lesson a gamed pass taught. It stays
    /// retired through the guard's rollback and through a restore that
    /// writes it back as its kept version holds it, as an older backup
    /// would. An id the store lost but a kept version holds is retired too,
    /// and one found nowhere is refused.
    #[test]
    fn a_retired_entry_stays_retired_after_a_rollback() {
        let temp = tempfile::tempdir().expect("tempdir");
        let roko_dir = temp.path().join(".roko");
        let learn_dir = roko_dir.join("learn");
        let store = KnowledgeStore::for_roko_dir(&roko_dir);
        let delivered = || Delivery::Delivered("the run delivered plan p5-ship".to_string());
        let commit = |batch: &str| {
            let checks = KnowledgeChecks::new(&store, batch, Vec::new(), delivered());
            let proposer = Proposer::evolved("knowledge", KNOWLEDGE_STORE, batch);
            let decision = propose_batch(
                &store,
                batch,
                &learn_dir,
                GuardMode::Observe,
                &checks,
                &proposer,
            )
            .expect("proposed");
            assert_eq!(decision, Some(CommitDecision::Committed), "{batch}");
        };

        // v2 holds the gamed lesson and a sound one, v3 a later run's lesson.
        let gamed = lesson(
            "kn-gamed",
            "Showcase renders come out quiet, so the demo script must compensate locally.",
            "run-2ad4fa-retire-a",
        );
        let sound = lesson(
            "kn-sound",
            "Preset loudness comes from each preset's master gain.",
            "run-2ad4fa-retire-a",
        );
        store
            .rewrite_all(&[gamed, sound])
            .expect("seed the first batch");
        commit("run-2ad4fa-retire-a");
        let later = lesson(
            "kn-later",
            "Install paths follow the bundle layout.",
            "run-2ad4fa-retire-b",
        );
        let mut entries = store.read_all().expect("read");
        entries.push(later);
        store.rewrite_all(&entries).expect("seed the second batch");
        commit("run-2ad4fa-retire-b");
        let topic = "showcase renders quiet demo compensate";
        assert_eq!(found(&store, topic), ["kn-gamed"]);

        // A person retires it.
        let reason = "a gamed pass taught it (bug-3b3bb6)";
        let retired = store
            .retire("kn-gamed", reason, "human", &learn_dir)
            .expect("retired");
        assert_eq!(retired.found_in, "store");
        assert!(!retired.already);
        assert!(found(&store, topic).is_empty());
        assert!(read(&store, "kn-gamed").expect("kept").deprecated);
        assert_eq!(found(&store, "preset loudness master gain"), ["kn-sound"]);
        let rows = store.retirement_rows().expect("the ledger");
        let [row] = rows.as_slice() else {
            panic!("one row: {rows:?}");
        };
        assert_eq!(row.action, RetirementAction::Retired);
        assert_eq!((row.id.as_str(), row.reason.as_str()), ("kn-gamed", reason));
        assert_eq!(row.found_in.as_deref(), Some("store"));
        let again = store
            .retire("kn-gamed", reason, "human", &learn_dir)
            .expect("retired again");
        assert!(again.already);
        assert_eq!(store.retirement_rows().expect("the ledger").len(), 1);

        // A person's rollback to v2 deletes v3's lesson and leaves it retired.
        let mut guarded =
            GuardedStore::open(&learn_dir, KNOWLEDGE_STORE, GuardMode::Observe).expect("the guard");
        let mut batches = LaterBatches::after(&guarded, 2, store.clone()).expect("v3's ids");
        let human = Proposer::human(KNOWLEDGE_STORE, "roko learn rollback knowledge --to 2");
        guarded
            .rollback(&mut batches, 2, &human)
            .expect("rolled back");
        assert!(read(&store, "kn-later").is_none());
        assert!(read(&store, "kn-gamed").expect("kept").deprecated);
        assert!(found(&store, topic).is_empty());

        // A restore writes v2's lessons back as they were committed, the
        // gamed one unflagged: it stays retired.
        let snapshot = guarded.snapshot(2).expect("v2");
        let restored: Vec<KnowledgeEntry> = String::from_utf8_lossy(&snapshot)
            .lines()
            .map(|line| serde_json::from_str(line).expect("an entry"))
            .collect();
        assert!(restored.iter().all(|entry| !entry.deprecated));
        store.rewrite_all(&restored).expect("restore v2");
        assert!(found(&store, topic).is_empty());
        assert!(read(&store, "kn-gamed").expect("restored").deprecated);
        let hot: Vec<String> = store
            .hot_entries()
            .expect("hot entries")
            .into_iter()
            .map(|entry| entry.id)
            .collect();
        assert_eq!(hot, ["kn-sound"]);

        // An entry removed by hand that a kept version still holds, as
        // Undertow's was, is retired from that version, and stays retired
        // when a restore brings it back.
        store
            .rewrite_all(&restored[..1])
            .expect("remove the sound lesson by hand");
        let by_hand = store
            .retire("kn-sound", "removed by hand", "human", &learn_dir)
            .expect("retired from v2");
        assert_eq!(by_hand.found_in, "v2");
        assert!(by_hand.content.contains("master gain"));
        store.rewrite_all(&restored).expect("restore v2 again");
        assert!(found(&store, "preset loudness master gain").is_empty());

        // An id found nowhere is refused, and adds no row.
        assert!(
            store
                .retire("kn-typo", "a typo", "human", &learn_dir)
                .is_err()
        );
        assert_eq!(store.retirement_rows().expect("the ledger").len(), 2);
        let summary = store.retirements().expect("the summary");
        let ids: Vec<&str> = summary.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, ["kn-gamed", "kn-sound"]);
    }
}
