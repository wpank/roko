//! Production canary writers for L-know and L-play (S03 §4.7; backlog 5127).
//!
//! The canary driver (`roko_learn::loop_audit::canary`) writes a
//! `CANARY-<nonce>` artifact through a loop's own writer and traces it, but
//! roko-learn cannot reach the real writers: roko-neuro depends on it, and
//! the prompt readers live here. These [`CanaryWriter`]s close that gap.
//!
//! - [`KnowledgeCanary`] (L-know) adds a knowledge entry through
//!   `KnowledgeStore::add`, the writer runtime knowledge ingestion uses.
//! - [`PlaybookCanary`] (L-play) saves a playbook through roko-learn's
//!   `PlaybookStore`, the writer playbook extraction uses.
//!
//! Each artifact holds only the words of its synthetic category,
//! `canary-<nonce>` and `canary-<nonce>-probe`, which no real task holds, so
//! only [`canary_task`] retrieves it. P2 loads the snapshot a plan run builds
//! its prompts from (`PromptCache::load`), whose item count is the state
//! version decision rows record; P3 runs the cached readers on it. `credit`
//! settles a synthetic pass the way the runtime does, and `cleanup` removes
//! the canary's own artifact and nothing else: M2 deletes no other learned
//! state (S03 §2).
//!
//! L-route has no writer here. The cascade router keys what it learns by
//! closed enums (task category, role) and context features, so no router
//! preference can be scoped to a synthetic category; its static role table,
//! the one preference a canary could set and restore exactly, applies to
//! every task of a role and is read only at cold start.

use std::path::{Path, PathBuf};

use roko_fs::RokoLayout;
use roko_learn::loop_audit::canary::CanaryWriter;
use roko_learn::playbook::{Playbook, PlaybookStore};
use roko_neuro::{KnowledgeEntry, KnowledgeKind, KnowledgeStore, ReinforcementSignal};

use crate::dispatch::prompt_builder::cached_reader_ids;
use crate::dispatch::prompt_cache::PromptCache;
use crate::task_parser::TaskDef;

/// The knowledge canary entry's `source`.
const CANARY_SOURCE: &str = "loop-canary";

/// The knowledge canary entry's confidence, well above the reader's floor.
const CANARY_CONFIDENCE: f64 = 0.9;

/// The novelty of the gated reinforcement `credit` settles: the knowledge
/// lifecycle's default.
const CREDIT_NOVELTY: f64 = 0.5;

/// The id of `nonce`'s canary artifact: `CANARY-<nonce>`.
#[must_use]
pub fn canary_id(nonce: &str) -> String {
    format!("CANARY-{nonce}")
}

/// The synthetic category of `nonce`'s canary, `canary-<nonce>`, as one of
/// the words the prompt readers match on: lowercase, with `-` for anything
/// but an ASCII letter, a digit or `_`.
#[must_use]
pub fn canary_category(nonce: &str) -> String {
    let nonce: String = nonce.chars().map(category_char).collect();
    format!("canary-{nonce}")
}

/// `c` as a category word keeps it.
fn category_char(c: char) -> char {
    if c.is_ascii_alphanumeric() || c == '_' {
        c.to_ascii_lowercase()
    } else {
        '-'
    }
}

/// The text of `nonce`'s canary artifacts: its id and the two words of its
/// category, so that only the canary task shares two topic words with it.
fn canary_text(nonce: &str) -> String {
    let category = canary_category(nonce);
    format!("{} {category} {category}-probe", canary_id(nonce))
}

/// The synthetic task a canary of `nonce` traces: its title holds the two
/// words of the canary's category.
#[must_use]
pub fn canary_task(nonce: &str) -> TaskDef {
    let category = canary_category(nonce);
    let task = serde_json::json!({
        "id": category,
        "title": format!("{category} {category}-probe"),
        "role": "implementer",
    });
    serde_json::from_value(task).expect("a canary task is a valid task")
}

/// `n` as a state version.
fn version(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Run `future`, a call of the async playbook store, to completion on a
/// runtime and a thread of its own, so a canary probe can make it from any
/// context, inside a Tokio runtime or outside one.
fn run_store<T: Send>(
    future: impl Future<Output = std::io::Result<T>> + Send,
) -> std::io::Result<T> {
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(future)
        });
        worker
            .join()
            .map_err(|_| std::io::Error::other("the store's call panicked"))?
    })
}

/// The success count of `playbook`, if there is one.
fn successes(playbook: Option<Playbook>) -> Option<u64> {
    playbook.map(|playbook| playbook.success_count)
}

/// L-know's canary writer (S03 §4.7): a `CANARY-<nonce>` knowledge entry on
/// the canary's category, which the cached knowledge reader selects for the
/// canary task.
#[derive(Debug)]
pub struct KnowledgeCanary {
    workdir: PathBuf,
    store: KnowledgeStore,
    snapshot: Option<PromptCache>,
}

impl KnowledgeCanary {
    /// L-know's canary writer for the workspace `workdir`.
    #[must_use]
    pub fn new(workdir: &Path) -> Self {
        Self {
            workdir: workdir.to_path_buf(),
            store: KnowledgeStore::for_workdir(workdir),
            snapshot: None,
        }
    }

    /// The balance of the entry `id`, if the store holds it.
    fn balance(&self, id: &str) -> Option<f64> {
        let entries = self.store.read_all().ok()?;
        entries
            .into_iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.balance)
    }
}

impl CanaryWriter for KnowledgeCanary {
    /// Add the entry through `KnowledgeStore::add`, and read it back among
    /// the hot entries a run snapshot loads; their count is the version.
    fn write(&mut self, nonce: &str) -> Result<u64, String> {
        let id = canary_id(nonce);
        let entry = KnowledgeEntry {
            id: id.clone(),
            kind: KnowledgeKind::Insight,
            source: Some(CANARY_SOURCE.to_string()),
            content: canary_text(nonce),
            confidence: CANARY_CONFIDENCE,
            tags: vec![canary_category(nonce)],
            ..KnowledgeEntry::default()
        };
        self.store
            .add(entry)
            .map_err(|error| format!("the knowledge store refused the entry: {error}"))?;
        let entries = self
            .store
            .hot_entries()
            .map_err(|error| format!("the knowledge store is unreadable: {error}"))?;
        if entries.iter().any(|entry| entry.id == id) {
            Ok(version(entries.len()))
        } else {
            Err("the knowledge store did not admit the entry".to_string())
        }
    }

    /// The number of entries the run snapshot loaded.
    fn loaded_version(&mut self) -> Option<u64> {
        let snapshot = PromptCache::load(&self.workdir);
        let loaded = snapshot.digest().knowledge.count;
        self.snapshot = Some(snapshot);
        (loaded > 0).then(|| version(loaded))
    }

    /// Whether the cached knowledge reader selects the entry for the canary
    /// task.
    fn read_selected(&mut self, nonce: &str) -> bool {
        let snapshot = self
            .snapshot
            .get_or_insert_with(|| PromptCache::load(&self.workdir));
        let (knowledge, _) = cached_reader_ids(&canary_task(nonce), snapshot);
        knowledge.contains(&canary_id(nonce))
    }

    /// Settle a synthetic pass as the knowledge lifecycle does, with the
    /// gated reinforcement; whether the entry's balance moved.
    fn credit(&mut self, nonce: &str) -> bool {
        let id = canary_id(nonce);
        let before = self.balance(&id);
        let ids = [id.as_str()];
        let reinforced = self
            .store
            .reinforce_batch(&ids, ReinforcementSignal::Gated, CREDIT_NOVELTY);
        reinforced.is_ok_and(|count| count == 1) && self.balance(&id) > before
    }

    /// Remove the entry, and only it.
    fn cleanup(&mut self, nonce: &str) {
        let id = canary_id(nonce);
        if let Err(error) = self.store.remove_entries(&[id.as_str()]) {
            tracing::warn!(%id, %error, "the knowledge canary's entry was not removed");
        }
        self.snapshot = None;
    }
}

/// L-play's canary writer (S03 §4.7): a `CANARY-<nonce>` playbook whose goal
/// holds the canary's category, which the cached playbook reader selects
/// for the canary task.
#[derive(Debug)]
pub struct PlaybookCanary {
    workdir: PathBuf,
    store: PlaybookStore,
    snapshot: Option<PromptCache>,
}

impl PlaybookCanary {
    /// L-play's canary writer for the workspace `workdir`.
    #[must_use]
    pub fn new(workdir: &Path) -> Self {
        Self {
            workdir: workdir.to_path_buf(),
            store: PlaybookStore::new(RokoLayout::for_project(workdir).playbooks_dir()),
            snapshot: None,
        }
    }
}

impl CanaryWriter for PlaybookCanary {
    /// Save the playbook through `PlaybookStore::save`, and load it back;
    /// the number of playbooks the store lists is the version.
    fn write(&mut self, nonce: &str) -> Result<u64, String> {
        let playbook = Playbook::new(canary_id(nonce), canary_text(nonce));
        let store = &self.store;
        let saved = run_store(async {
            store.save(&playbook).await?;
            let held = store.load(&playbook.id).await?.is_some();
            Ok::<_, std::io::Error>((held, store.list().await?.len()))
        });
        match saved {
            Ok((true, playbooks)) => Ok(version(playbooks)),
            Ok((false, _)) => Err("the playbook store does not hold the playbook".to_string()),
            Err(error) => Err(format!("the playbook store refused the playbook: {error}")),
        }
    }

    /// The number of playbooks the run snapshot loaded.
    fn loaded_version(&mut self) -> Option<u64> {
        let snapshot = PromptCache::load(&self.workdir);
        let loaded = snapshot.digest().playbooks.count;
        self.snapshot = Some(snapshot);
        (loaded > 0).then(|| version(loaded))
    }

    /// Whether the cached playbook reader selects the playbook for the
    /// canary task.
    fn read_selected(&mut self, nonce: &str) -> bool {
        let snapshot = self
            .snapshot
            .get_or_insert_with(|| PromptCache::load(&self.workdir));
        let (_, playbooks) = cached_reader_ids(&canary_task(nonce), snapshot);
        playbooks.contains(&canary_id(nonce))
    }

    /// Settle a synthetic pass through `PlaybookStore::record_outcome`;
    /// whether the playbook's success count moved.
    fn credit(&mut self, nonce: &str) -> bool {
        let id = canary_id(nonce);
        let store = &self.store;
        let credited = run_store(async {
            let before = successes(store.load(&id).await?);
            let recorded = store.record_outcome(&id, true).await?;
            let after = successes(store.load(&id).await?);
            Ok::<_, std::io::Error>(recorded && after > before)
        });
        credited.unwrap_or(false)
    }

    /// Delete the playbook, and only it.
    fn cleanup(&mut self, nonce: &str) {
        let id = canary_id(nonce);
        if let Err(error) = run_store(self.store.delete(&id)) {
            tracing::warn!(%id, %error, "the playbook canary's playbook was not removed");
        }
        self.snapshot = None;
    }
}

#[cfg(test)]
mod tests {
    use roko_learn::loop_audit::canary::{CanaryTarget, CanaryTask, DryRunPlanner, PlanProbe, trace};

    use super::*;

    /// The canaries' nonce.
    const NONCE: &str = "c-5d1e";

    /// A planner without a dispatcher: P4 waits for backlog 5128's.
    struct NoPlanner;

    impl DryRunPlanner for NoPlanner {
        fn plan(&mut self, _task: &CanaryTask, _dry_run: bool) -> Result<PlanProbe, String> {
            Err("no dry-run planner yet (5128)".to_string())
        }
    }

    /// The real task the seeded learned state is about.
    fn real_task() -> TaskDef {
        let task = serde_json::json!({
            "id": "fmt",
            "title": "Run cargo fmt before committing the workspace",
            "role": "implementer",
        });
        serde_json::from_value(task).expect("a task")
    }

    /// Seed workspace `dir` with a knowledge entry and a playbook about
    /// [`real_task`].
    fn seed(dir: &Path) {
        let entry = KnowledgeEntry {
            id: "kn-real".to_string(),
            kind: KnowledgeKind::Insight,
            content: "Run cargo fmt before committing the workspace".to_string(),
            confidence: 0.8,
            ..KnowledgeEntry::default()
        };
        KnowledgeStore::for_workdir(dir)
            .add(entry)
            .expect("seed a knowledge entry");
        let playbook = Playbook::new("pb-real", "Format the workspace before committing");
        let store = PlaybookStore::new(RokoLayout::for_project(dir).playbooks_dir());
        run_store(store.save(&playbook)).expect("seed a playbook");
    }

    /// What workspace `dir`'s stores hold: each knowledge entry's id and
    /// balance, and each playbook's id and success count, by id.
    fn stores(dir: &Path) -> (Vec<(String, f64)>, Vec<(String, u64)>) {
        let mut knowledge: Vec<(String, f64)> = KnowledgeStore::for_workdir(dir)
            .read_all()
            .expect("read the knowledge store")
            .into_iter()
            .map(|entry| (entry.id, entry.balance))
            .collect();
        knowledge.sort_by(|a, b| a.0.cmp(&b.0));
        let store = PlaybookStore::new(RokoLayout::for_project(dir).playbooks_dir());
        let mut playbooks: Vec<(String, u64)> = run_store(store.list())
            .expect("list the playbooks")
            .into_iter()
            .map(|playbook| (playbook.id, playbook.success_count))
            .collect();
        playbooks.sort();
        (knowledge, playbooks)
    }

    /// S03 §4.7 (backlog 5127): each canary writes through its loop's own
    /// writer, and the real readers return it for the canary task, so the
    /// driver's P1-P3 pass and the trace stops at P4, which waits for a
    /// dry-run planner. A real task never retrieves a canary, a synthetic
    /// pass moves the canary's counters (P7), and `cleanup` leaves the
    /// stores as they were.
    #[test]
    fn canary_writers_reach_real_readers() {
        let dir = tempfile::tempdir().expect("temp dir");
        seed(dir.path());
        let before = stores(dir.path());
        let run_dir = dir.path().join(".roko/runs/gr-canary");
        let writers: [(&str, Box<dyn CanaryWriter>); 2] = [
            ("L-know", Box::new(KnowledgeCanary::new(dir.path()))),
            ("L-play", Box::new(PlaybookCanary::new(dir.path()))),
        ];
        let real = (vec!["kn-real".to_string()], vec!["pb-real".to_string()]);
        for (loop_id, mut writer) in writers {
            let task = CanaryTask {
                loop_id: loop_id.to_string(),
                nonce: NONCE.to_string(),
                category: canary_category(NONCE),
                target: CanaryTarget::Prompt,
            };
            let row = trace(&mut *writer, &mut NoPlanner, &run_dir, &task, true);
            let probes: Vec<(&str, bool)> = row
                .probes
                .iter()
                .map(|probe| (probe.p.as_str(), probe.ok))
                .collect();
            let expected = [("P1", true), ("P2", true), ("P3", true), ("P4", false)];
            assert_eq!(probes, expected, "{loop_id}: {row:?}");
            assert_eq!(stores(dir.path()), before, "{loop_id}: the trace cleaned up");

            // While the canary is written, a real task's readers pass it by
            // and still find the real state.
            writer.write(NONCE).expect("write the canary again");
            let snapshot = PromptCache::load(dir.path());
            let found = cached_reader_ids(&real_task(), &snapshot);
            assert_eq!(found, real, "{loop_id}");
            assert!(writer.credit(NONCE), "{loop_id}: P7 moves the counters");
            writer.cleanup(NONCE);
            assert_eq!(stores(dir.path()), before, "{loop_id}: cleanup");
        }
    }
}
