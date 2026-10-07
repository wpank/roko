//! The run-end guarded commit (P21, 8138).
//!
//! `[learning] guarded_commit` decides what the end of a Graph run does with
//! what the run taught the cascade router and the knowledge store. In
//! `observe` (decision 8103's default) and `enforce` the router's merge goes
//! to its guard ([`ModelCallJournal::save_guarded`], 8136) and the run's
//! knowledge batch to the knowledge store's ([`propose_batch`], 8137),
//! whatever the run's outcome: a failed or cancelled run's learning is the
//! one that most needs the check. Each proposal writes a `commits.jsonl`
//! row, and a rollback, or in `observe` a would-be rollback, a notice on the
//! run's StateHub. `off` saves the router unchecked, as before, and the run
//! tags no knowledge batch. A frozen run saves and proposes nothing.
//!
//! At a run's start, [`recover_orphaned_knowledge`] proposes the knowledge
//! batches earlier runs left unproposed, as a run that crashed before its end
//! leaves its batch (gap-6ff99a), through the same guard.

use std::collections::BTreeSet;
use std::path::Path;

use roko_core::DashboardEvent;
use roko_core::config::learning::GuardedCommitMode;
use roko_core::config::schema::RokoConfig;
use roko_fs::RokoLayout;
use roko_learn::cascade_router::CascadeRouter;
use roko_learn::guarded_commit::{CommitDecision, GuardMode, Proposer};
use roko_learn::model_call_feedback::ModelCallJournal;
use roko_learn::router_commit::{self, ROUTER_STORE, RouterChecks, RouterGuard, SettledOutcome};
use roko_learn::telemetry::report::RunRecords;
use roko_neuro::KnowledgeStore;
use roko_neuro::knowledge_store::commit::{
    KNOWLEDGE_STORE, KnowledgeBatch, KnowledgeChecks, load_knowledge_anchors, orphaned_batches,
    propose_batch, recover_orphaned_batches,
};

use crate::state_hub::StateHub;

/// The StateHub event type of a guard's notice.
pub const GUARD_NOTICE: &str = "learning.guarded_commit";

/// What the run-end commit needs from the run.
pub struct RunLearning<'a> {
    /// The workspace.
    pub workdir: &'a Path,
    /// The run's config.
    pub config: &'a RokoConfig,
    /// The run's id, which names its knowledge batch; `None` when the run
    /// tags none (`off`, or a frozen run).
    pub batch: Option<&'a str>,
    /// The run's router and the journal it saves through.
    pub router: Option<(&'a CascadeRouter, &'a ModelCallJournal)>,
    /// The Graph checkpoint runs of the run's plans, whose settled attempts
    /// the router's held-out check scores.
    pub run_ids: &'a [String],
    /// The run's StateHub, for notices.
    pub notices: Option<&'a StateHub>,
}

/// What the run-end commit decided for each store; `None` where it proposed
/// nothing or the proposal failed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RunCommits {
    /// The cascade router.
    pub router: Option<CommitDecision>,
    /// The run's knowledge batch.
    pub knowledge: Option<CommitDecision>,
}

/// Save or propose what the run taught the router and the knowledge store,
/// as `[learning] guarded_commit` says.
pub fn commit_run_learning(run: &RunLearning<'_>) -> RunCommits {
    let mut commits = RunCommits::default();
    if run.config.learning.frozen {
        return commits;
    }
    let mode = guard_mode(run.config);
    if let Some((router, journal)) = run.router {
        commits.router = save_router(run, router, journal, mode);
    }
    if let (Some(mode), Some(batch)) = (mode, run.batch) {
        commits.knowledge = propose_knowledge(run, batch, mode);
    }
    commits
}

/// Save the router through its journal: under guard in `mode`, or unchecked
/// without one.
fn save_router(
    run: &RunLearning<'_>,
    router: &CascadeRouter,
    journal: &ModelCallJournal,
    mode: Option<GuardMode>,
) -> Option<CommitDecision> {
    let warn = |error: &dyn std::fmt::Display| {
        tracing::warn!(
            path = %journal.snapshot_path().display(),
            error = %error,
            "failed to persist cascade router state (non-fatal)"
        );
    };
    let Some(mode) = mode else {
        if let Err(error) = journal.save(router) {
            warn(&error);
        }
        return None;
    };
    let layout = RokoLayout::for_project(run.workdir);
    let anchors = router_commit::load_router_anchors(layout.root()).unwrap_or_else(|problem| {
        tracing::warn!(%problem, "no router anchors: the file is unreadable");
        Vec::new()
    });
    let settled = settled_outcomes(&layout, run.run_ids);
    let reason = format!("router:{}", run.batch.unwrap_or("run"));
    let guard = RouterGuard {
        learn_dir: layout.learn_dir(),
        mode,
        checks: RouterChecks {
            held_out: router_commit::held_out_attempts(&settled),
            anchors,
            configured: configured_models(run.config),
        },
        proposer: Proposer::evolved("router", ROUTER_STORE, reason),
    };
    match journal.save_guarded(router, &guard) {
        Ok(decision) => {
            notice(run.notices, "router", decision);
            Some(decision)
        }
        Err(error) => {
            warn(&error);
            None
        }
    }
}

/// Propose the run's knowledge batch to the knowledge store's guard.
fn propose_knowledge(
    run: &RunLearning<'_>,
    batch: &str,
    mode: GuardMode,
) -> Option<CommitDecision> {
    let layout = RokoLayout::for_project(run.workdir);
    let store = KnowledgeStore::for_layout(&layout);
    let anchors = load_knowledge_anchors(layout.root()).unwrap_or_else(|problem| {
        tracing::warn!(%problem, "no knowledge anchors: the file is unreadable");
        Vec::new()
    });
    let checks = KnowledgeChecks::new(&store, batch, anchors);
    let proposer = Proposer::evolved("knowledge", KNOWLEDGE_STORE, format!("knowledge:{batch}"));
    let learn_dir = layout.learn_dir();
    match propose_batch(&store, batch, &learn_dir, mode, &checks, &proposer) {
        Ok(decision) => {
            if let Some(decision) = decision {
                notice(run.notices, "knowledge", decision);
            }
            decision
        }
        Err(error) => {
            tracing::warn!(%error, batch, "the run's knowledge batch was not proposed");
            None
        }
    }
}

/// At a run's start, propose every knowledge batch the store holds besides
/// `live`, the run's own: the orphans of runs that ended before they proposed
/// theirs, as a run that crashed leaves its batch (gap-6ff99a). Each goes
/// through the knowledge store's guard as a run-end proposal does, with the
/// same checks, mode, `commits.jsonl` row and notice, so its entries are
/// committed or rolled back instead of hidden from every other run for good.
/// With `off` they are committed unchecked, as `off` saves learning; a frozen
/// run touches nothing. Returns each orphan with its decision.
///
/// The caller holds the workspace's runner lock, so no other run owns a
/// batch.
pub fn recover_orphaned_knowledge(
    workdir: &Path,
    config: &RokoConfig,
    live: Option<&str>,
    notices: Option<&StateHub>,
) -> Vec<(String, Option<CommitDecision>)> {
    if config.learning.frozen {
        return Vec::new();
    }
    let layout = RokoLayout::for_project(workdir);
    let store = KnowledgeStore::for_layout(&layout);
    let Some(mode) = guard_mode(config) else {
        let orphans = orphaned_batches(&store, live).unwrap_or_default();
        for batch in &orphans {
            if let Err(error) = KnowledgeBatch::new(&store, batch).commit() {
                let error = format!("{error:#}");
                tracing::warn!(%error, %batch, "orphaned batch kept");
            }
        }
        return orphans.into_iter().map(|batch| (batch, None)).collect();
    };
    let anchors = load_knowledge_anchors(layout.root()).unwrap_or_else(|problem| {
        tracing::warn!(%problem, "no knowledge anchors: the file is unreadable");
        Vec::new()
    });
    let learn_dir = layout.learn_dir();
    match recover_orphaned_batches(&store, live, &learn_dir, mode, &anchors) {
        Ok(recovered) => {
            for (batch, decision) in &recovered {
                tracing::info!(%batch, ?decision, "proposed an orphaned knowledge batch");
                if let Some(decision) = *decision {
                    notice(notices, "knowledge", decision);
                }
            }
            recovered
        }
        Err(error) => {
            tracing::warn!(%error, "orphaned knowledge batches were not proposed");
            Vec::new()
        }
    }
}

/// What a failed guard check does under `[learning] guarded_commit`; `None`
/// for `off`, which proposes nothing.
fn guard_mode(config: &RokoConfig) -> Option<GuardMode> {
    match config.learning.guarded_commit {
        GuardedCommitMode::Observe => Some(GuardMode::Observe),
        GuardedCommitMode::Enforce => Some(GuardMode::Enforce),
        GuardedCommitMode::Off => None,
    }
}

/// The settled attempts of the runs `run_ids`, in order, each marked where it
/// ran on M2's holdout arm of the router loop (gap-a47d07).
fn settled_outcomes(layout: &RokoLayout, run_ids: &[String]) -> Vec<SettledOutcome> {
    let mut settled = Vec::new();
    for run_id in run_ids {
        if let Ok(records) = RunRecords::load(&layout.run_dir(run_id)) {
            settled.extend(router_commit::settled_outcomes(&records));
        }
    }
    settled
}

/// The models the router may route to: the slugs of the configured models
/// whose provider `[routing] disabled_providers` does not name.
fn configured_models(config: &RokoConfig) -> BTreeSet<String> {
    let disabled = &config.routing.disabled_providers;
    config
        .effective_models()
        .into_values()
        .filter(|model| !disabled.contains(&model.provider) && !model.slug.trim().is_empty())
        .map(|model| model.slug)
        .collect()
}

/// Tell the run's StateHub about a rollback, or about the rollback observe
/// mode did not make.
fn notice(notices: Option<&StateHub>, store: &str, decision: CommitDecision) {
    let message = match decision {
        CommitDecision::RolledBack => format!(
            "{store}: a guard check failed, and the run's {store} learning was rolled back to \
             the last-known-good version"
        ),
        CommitDecision::Observed => format!(
            "{store}: a guard check failed; observe mode kept the run's {store} learning, \
             which enforce mode would have rolled back"
        ),
        CommitDecision::Committed | CommitDecision::Restored => return,
    };
    tracing::warn!(store, ?decision, "{message}");
    if let Some(hub) = notices {
        let now = chrono::Utc::now().timestamp_millis();
        hub.publish(DashboardEvent::EventLogEntry {
            timestamp_ms: u64::try_from(now).unwrap_or_default(),
            event_type: GUARD_NOTICE.to_string(),
            plan_id: String::new(),
            task_id: String::new(),
            message,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_learn::guarded_commit::GuardedStore;
    use roko_learn::model_router::RoutingContext;
    use roko_neuro::KnowledgeEntry;

    /// A verified attempt's knowledge entry.
    fn entry(id: &str, content: &str, tags: &[&str], confidence: f64) -> KnowledgeEntry {
        KnowledgeEntry {
            id: id.to_string(),
            content: content.to_string(),
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
            confidence,
            confidence_weight: confidence,
            source: Some("gate-verdict".to_string()),
            source_episodes: vec![format!("attempt-{id}")],
            ..KnowledgeEntry::default()
        }
    }

    /// P21 (8138): a run in enforce mode whose knowledge batch pushes an
    /// anchored entry out of its query's top hit ends with the batch rolled
    /// back, with a notice, and its router committed.
    #[test]
    fn run_end_commits_router_and_knowledge_through_guard() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let layout = RokoLayout::for_project(workdir);
        let mut config = RokoConfig::default();
        config.learning.guarded_commit = GuardedCommitMode::Enforce;

        // What the run taught the router.
        let router = CascadeRouter::new(vec!["model-a".to_string(), "model-b".to_string()]);
        let ctx = RoutingContext::default();
        for _ in 0..4 {
            router.record_observation(&ctx, "model-a", 1.0, true);
        }
        let journal = ModelCallJournal::for_snapshot(&layout.cascade_router_path());

        // The run's poisoned batch outranks the anchored entry.
        let anchor = entry("kn-anchor", "Retry backoff doubles the wait.", &[], 0.5);
        let poisoned = KnowledgeEntry {
            commit_batch: Some("run-1".to_string()),
            ..entry(
                "kn-poison",
                "retry backoff retry backoff: never wait",
                &["retry backoff"],
                0.95,
            )
        };
        let knowledge = KnowledgeStore::for_layout(&layout);
        let neuro = knowledge.path().parent().expect("the neuro directory");
        std::fs::create_dir_all(neuro).expect("create the neuro directory");
        let lines: String = [anchor, poisoned]
            .iter()
            .map(|entry| serde_json::to_string(entry).expect("an entry") + "\n")
            .collect();
        std::fs::write(knowledge.path(), lines).expect("seed the knowledge store");
        let anchors = layout.root().join("policy/anchors.toml");
        std::fs::create_dir_all(anchors.parent().expect("policy dir")).expect("policy dir");
        std::fs::write(
            &anchors,
            "[[knowledge]]\nquery = \"retry backoff\"\nexpect = \"kn-anchor\"\nk = 1\n",
        )
        .expect("write the anchors");

        let hub = crate::state_hub::shared_state_hub();
        let mut events = hub.subscribe_events();
        let commits = commit_run_learning(&RunLearning {
            workdir,
            config: &config,
            batch: Some("run-1"),
            router: Some((&router, &journal)),
            run_ids: &[],
            notices: Some(&hub),
        });

        assert_eq!(
            commits,
            RunCommits {
                router: Some(CommitDecision::Committed),
                knowledge: Some(CommitDecision::RolledBack),
            }
        );
        let ids: Vec<String> = knowledge
            .read_all()
            .expect("read the knowledge store")
            .into_iter()
            .map(|entry| entry.id)
            .collect();
        assert_eq!(ids, ["kn-anchor"]);
        // The router's file holds what the run taught it.
        let saved = std::fs::read(journal.snapshot_path()).expect("the router's snapshot");
        let saved: serde_json::Value = serde_json::from_slice(&saved).expect("JSON");
        assert_eq!(saved["confidence_stats"]["model-a"]["trials"], 4);

        // One row per proposal: the router's commit, and the knowledge
        // store's baseline and rollback.
        let rows = |store: &str| -> Vec<CommitDecision> {
            let guarded = GuardedStore::open(&layout.learn_dir(), store, GuardMode::Enforce)
                .expect("the store's guard");
            let rows = guarded.rows().expect("its rows");
            rows.iter().map(|row| row.decision).collect()
        };
        assert_eq!(rows(ROUTER_STORE), [CommitDecision::Committed]);
        assert_eq!(
            rows(KNOWLEDGE_STORE),
            [CommitDecision::Committed, CommitDecision::RolledBack]
        );

        // The rollback has its notice on the run's StateHub.
        let mut notices = Vec::new();
        while let Ok(envelope) = events.try_recv() {
            if let DashboardEvent::EventLogEntry {
                event_type,
                message,
                ..
            } = envelope.payload
                && event_type == GUARD_NOTICE
            {
                notices.push(message);
            }
        }
        assert_eq!(notices.len(), 1, "{notices:?}");
        assert!(notices[0].starts_with("knowledge: "), "{notices:?}");
    }
}
