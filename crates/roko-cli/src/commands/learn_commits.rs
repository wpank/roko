//! `roko learn commits` and `roko learn rollback` (P21, backlog 8139): what
//! the guarded stores (the cascade router, the knowledge store's batches and
//! M1's θ) committed, rolled back and restored, newest first, and a person's
//! rollback of a store to a version it keeps. A rollback writes a `restored`
//! row with actor `human`, and is refused while a plan run holds the
//! workspace's runner lock.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use clap::ValueEnum;
use roko_cli::exit_codes::EXIT_SUCCESS;
use roko_core::config::harness_params::HarnessParams;
use roko_core::config::learning::GuardedCommitMode;
use roko_fs::RokoLayout;
use roko_learn::guarded_commit::{
    COMMITS_DIR, CommitRow, GuardError, GuardMode, GuardedState, GuardedStore, Proposer,
};
use roko_learn::homeostasis::lkg;
use roko_learn::model_call_feedback::discard_unsaved_router_observations;
use roko_learn::router_commit::ROUTER_STORE;
use roko_neuro::KnowledgeStore;
use roko_neuro::knowledge_store::commit::KNOWLEDGE_STORE;
use serde::Serialize;
use serde_json::Value;

/// A guarded store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum StoreName {
    /// The cascade router, `.roko/learn/cascade-router.json`.
    Router,
    /// The knowledge store's batches, `.roko/neuro/knowledge.jsonl`.
    Knowledge,
    /// M1's θ.
    Harness,
}

impl StoreName {
    /// Every store, in the order `roko learn commits` reads them.
    const ALL: [Self; 3] = [Self::Router, Self::Knowledge, Self::Harness];

    /// The store's directory under `.roko/learn/commits/`.
    const fn dir(self) -> &'static str {
        match self {
            Self::Router => ROUTER_STORE,
            Self::Knowledge => KNOWLEDGE_STORE,
            Self::Harness => lkg::STORE,
        }
    }
}

/// One row of `roko learn commits`.
#[derive(Debug, Serialize)]
struct Listed {
    /// The store.
    store: &'static str,
    /// Its commit row.
    #[serde(flatten)]
    row: CommitRow,
}

/// Run `roko learn commits`: every row of `store`, or of every store,
/// newest first.
pub(crate) fn cmd_commits(workdir: &Path, store: Option<StoreName>, json: bool) -> Result<i32> {
    let learn_dir = RokoLayout::for_project(workdir).learn_dir();
    let stores: &[StoreName] = match &store {
        Some(store) => std::slice::from_ref(store),
        None => &StoreName::ALL,
    };
    let rows = listed(&learn_dir, stores)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        print!("{}", render(&rows));
    }
    Ok(EXIT_SUCCESS)
}

/// Run `roko learn rollback`.
pub(crate) fn cmd_rollback(workdir: &Path, store: StoreName, to: u64, json: bool) -> Result<i32> {
    let previous = rollback(workdir, store, to)?;
    if json {
        let report = serde_json::json!({
            "store": store.dir(),
            "restored": to,
            "previous": previous,
        });
        println!("{report}");
    } else {
        let was = previous.map_or_else(|| "none".to_string(), |version| format!("v{version}"));
        println!("{}: restored v{to} (was {was})", store.dir());
    }
    Ok(EXIT_SUCCESS)
}

/// The rows of `stores` under `learn_dir`, newest first. A store without a
/// directory has none: listing creates nothing.
fn listed(learn_dir: &Path, stores: &[StoreName]) -> Result<Vec<Listed>> {
    let mut listed = Vec::new();
    for &store in stores {
        if !learn_dir.join(COMMITS_DIR).join(store.dir()).is_dir() {
            continue;
        }
        let guarded = GuardedStore::open(learn_dir, store.dir(), GuardMode::Observe)?;
        let rows = guarded.rows()?;
        listed.extend(rows.into_iter().map(|row| Listed {
            store: store.dir(),
            row,
        }));
    }
    // By time, newest first; a store's later rows first within one time.
    listed.reverse();
    listed.sort_by(|left, right| right.row.ts.cmp(&left.row.ts));
    Ok(listed)
}

/// `rows` for a person.
fn render(rows: &[Listed]) -> String {
    if rows.is_empty() {
        return "no commits\n".to_string();
    }
    let mut text = String::new();
    for Listed { store, row } in rows {
        let parent = row
            .parent
            .map_or_else(|| "none".to_string(), |parent| format!("v{parent}"));
        let (decision, mode) = (label(row.decision), label(row.mode));
        let (actor, ts) = (&row.actor, &row.ts);
        let head = format!("{store} v{} (parent {parent})", row.version);
        let _ = writeln!(text, "{head}: {decision} in {mode} mode by {actor} at {ts}");
        let _ = writeln!(text, "  digest {}", row.digest);
        for check in &row.checks {
            let verdict = if check.passed { "pass" } else { "fail" };
            let _ = writeln!(text, "  {} {verdict}: {}", check.check, check.reason);
        }
        if let Some(reason) = &row.reason {
            let _ = writeln!(text, "  reason: {reason}");
        }
    }
    text
}

/// The serialized name of a decision or a mode, such as `rolled_back`.
fn label(value: impl Serialize) -> String {
    match serde_json::to_value(value) {
        Ok(Value::String(name)) => name,
        _ => String::new(),
    }
}

/// Restore `store` to its kept version `to`, as a person's rollback: a
/// `restored` row with actor `human`, through the guarded store (8110).
/// Returns the version the store held before.
///
/// # Errors
///
/// Refused while a plan run holds the workspace's runner lock; a store
/// without commits, a version it does not keep, and I/O errors.
pub(crate) fn rollback(workdir: &Path, store: StoreName, to: u64) -> Result<Option<u64>> {
    let layout = RokoLayout::for_project(workdir);
    let _runner = roko_cli::workspace_lock::acquire_runner_lock(layout.root())
        .context("a plan run holds this workspace's learned state; roll back once it ends")?;
    let learn_dir = layout.learn_dir();
    if !learn_dir.join(COMMITS_DIR).join(store.dir()).is_dir() {
        bail!("`{}` has no commits to roll back", store.dir());
    }
    let config = roko_core::config::loader::load_config_unified(workdir).unwrap_or_default();
    let mode = match (store, config.learning.guarded_commit) {
        (StoreName::Harness, _) | (_, GuardedCommitMode::Enforce) => GuardMode::Enforce,
        _ => GuardMode::Observe,
    };
    let mut guarded = GuardedStore::open(&learn_dir, store.dir(), mode)?;
    let previous = guarded.current();
    let reason = format!("roko learn rollback {} --to {to}", store.dir());
    let proposer = Proposer::human(store.dir(), reason);
    match store {
        StoreName::Router => {
            let mut file = RouterFile {
                path: layout.cascade_router_path(),
            };
            guarded.rollback(&mut file, to, &proposer)?;
        }
        StoreName::Knowledge => {
            let knowledge = KnowledgeStore::for_layout(&layout);
            let mut batches = LaterBatches::after(&guarded, to, knowledge)?;
            guarded.rollback(&mut batches, to, &proposer)?;
        }
        StoreName::Harness => {
            // At its next start, M1 adopts the store's top version.
            let mut theta = HarnessParams::baseline(&config);
            guarded.rollback(&mut theta, to, &proposer)?;
        }
    }
    Ok(previous)
}

/// The router's live snapshot, `cascade-router.json`: a rollback writes the
/// version's JSON back under the file's lock, as a save does, once the
/// observations a gone writer journaled and never saved are dropped from the
/// WAL: replayed at the next load, they would undo the rollback (gap-775aa6).
struct RouterFile {
    path: PathBuf,
}

impl GuardedState for RouterFile {
    fn snapshot(&self) -> Vec<u8> {
        std::fs::read(&self.path).unwrap_or_default()
    }

    fn restore(&mut self, snapshot: &[u8]) -> Result<(), GuardError> {
        let version: Value =
            serde_json::from_slice(snapshot).map_err(|error| GuardError::BadSnapshot {
                store: ROUTER_STORE.to_string(),
                reason: error.to_string(),
            })?;
        discard_unsaved_router_observations(&self.path);
        roko_fs::with_locked_json_transaction(&self.path, |file: &mut Value| {
            *file = version;
            Ok::<(), std::io::Error>(())
        })
        .map_err(|source| GuardError::Io {
            path: self.path.display().to_string(),
            source,
        })
    }
}

/// The knowledge store as of a version: a rollback deletes the entries of
/// every batch committed after it, which the store's later versions hold.
struct LaterBatches {
    store: KnowledgeStore,
    ids: Vec<String>,
}

impl LaterBatches {
    /// The entries of the batches `guarded` keeps after version `to`, in
    /// `store`.
    fn after(guarded: &GuardedStore, to: u64, store: KnowledgeStore) -> Result<Self, GuardError> {
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
            .map_err(|error| GuardError::Io {
                path: self.store.path().display().to_string(),
                source: std::io::Error::other(format!("{error:#}")),
            })
    }
}

#[cfg(test)]
mod tests {
    use roko_learn::cascade_router::CascadeRouter;
    use roko_learn::guarded_commit::CommitDecision;
    use roko_learn::model_call_feedback::{ModelCallJournal, load_recovered_router};
    use roko_learn::model_router::RoutingContext;
    use roko_learn::router_commit::{RouterChecks, RouterGuard};

    use super::*;

    /// The trials of `model` in the router's file at `path`.
    fn trials(path: &Path, model: &str) -> u64 {
        let text = std::fs::read_to_string(path).expect("the router's file");
        let value: Value = serde_json::from_str(&text).expect("JSON");
        value["confidence_stats"][model]["trials"]
            .as_u64()
            .expect("a count")
    }

    /// The router models of these tests.
    fn slugs() -> Vec<String> {
        vec!["model-a".to_string(), "model-b".to_string()]
    }

    /// Three committed versions of the workspace's router, after 2, 6 and 12
    /// observations of model-a, and the router that learned them.
    fn three_versions(layout: &RokoLayout) -> CascadeRouter {
        let journal = ModelCallJournal::for_snapshot(&layout.cascade_router_path());
        let guard = RouterGuard {
            learn_dir: layout.learn_dir(),
            mode: GuardMode::Enforce,
            checks: RouterChecks::default(),
            proposer: Proposer::evolved("router", ROUTER_STORE, "router:test-run"),
        };
        let router = CascadeRouter::new(slugs());
        let ctx = RoutingContext::default();
        for observations in [2, 4, 6] {
            for _ in 0..observations {
                router.record_observation(&ctx, "model-a", 1.0, true);
            }
            let decision = journal.save_guarded(&router, &guard).expect("saved");
            assert_eq!(decision, CommitDecision::Committed);
        }
        router
    }

    /// P21 (8139): with three router versions, a rollback to the first puts
    /// its snapshot back in the router's file under a `restored` row by
    /// `human`, once no plan run holds the runner lock, and `roko learn
    /// commits` lists that row first.
    #[test]
    fn learn_rollback_restores_router_version() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let layout = RokoLayout::for_project(workdir);
        let path = layout.cascade_router_path();
        three_versions(&layout);
        assert_eq!(trials(&path, "model-a"), 12);

        // A plan run holds the runner lock: the rollback is refused.
        let run = roko_cli::workspace_lock::acquire_runner_lock(layout.root()).expect("lock");
        assert!(rollback(workdir, StoreName::Router, 1).is_err());
        drop(run);
        assert_eq!(trials(&path, "model-a"), 12);

        let previous = rollback(workdir, StoreName::Router, 1).expect("rolled back");
        assert_eq!(previous, Some(3));
        assert_eq!(trials(&path, "model-a"), 2);
        let store = GuardedStore::open(&layout.learn_dir(), ROUTER_STORE, GuardMode::Enforce)
            .expect("the router's store");
        assert_eq!(store.current(), Some(1));
        let rows = store.rows().expect("its rows");
        let restored = rows.last().expect("the rollback's row");
        assert_eq!((restored.version, restored.parent), (1, Some(3)));
        assert_eq!(restored.decision, CommitDecision::Restored);
        assert_eq!(restored.actor, "human");

        let all = listed(&layout.learn_dir(), &StoreName::ALL).expect("listed");
        assert_eq!(all.len(), 4);
        assert_eq!((all[0].store, all[0].row.version), (ROUTER_STORE, 1));
        assert!(render(&all).starts_with("router v1 (parent v3): restored"));
        assert_eq!(render(&[]), "no commits\n");
        // A version the store does not keep is refused.
        assert!(rollback(workdir, StoreName::Router, 9).is_err());
    }

    /// gap-775aa6: observations a crashed run journaled and never saved, still
    /// in its WAL segment at the rollback, do not replay over the restored
    /// version at the next load.
    #[test]
    fn router_rollback_survives_a_stale_wal_segment() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let layout = RokoLayout::for_project(workdir);
        let path = layout.cascade_router_path();
        let router = three_versions(&layout);
        // A run journals five more observations, then dies before it saves them.
        let crashed = ModelCallJournal::for_snapshot(&path);
        for _ in 0..5 {
            crashed.observe_model_call(&router, "model-a", "implementer", true, 1_000);
        }
        drop(crashed);

        rollback(workdir, StoreName::Router, 1).expect("rolled back");
        let loaded = load_recovered_router(&path, slugs());
        let snapshot: Value = serde_json::from_str(&loaded.snapshot_json()).expect("JSON");
        assert_eq!(snapshot["confidence_stats"]["model-a"]["trials"], 2);
        assert_eq!(trials(&path, "model-a"), 2);
    }
}
