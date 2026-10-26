//! The run-end guarded commit (P21, 8138).
//!
//! `[learning] guarded_commit` decides what the end of a Graph run does with
//! what the run taught the cascade router and the knowledge store. In
//! `observe` (decision 8103's default) and `enforce` the router's merge goes
//! to its guard ([`ModelCallJournal::save_guarded`], 8136) and the run's
//! knowledge batch to the knowledge store's ([`propose_batch`], 8137),
//! whatever the run's outcome: a failed or cancelled run's learning is the
//! one that most needs the check. The knowledge batch commits only when the
//! run delivered every plan ([`run_delivery`]): a run whose plan failed its
//! checks, or that was cancelled, interrupted or ended with an error, keeps
//! none of its lessons, in `observe` too (bug-2ad4fa). A run that taught the
//! router nothing proposes no router version and saves it unchecked
//! (gap-51d2ca), which merges nothing into its file. Each proposal writes a
//! `commits.jsonl` row, and a rollback, or in `observe` a would-be rollback,
//! a notice on the run's StateHub. `off` saves the router unchecked, as
//! before, and the run tags no knowledge batch. A frozen run saves and
//! proposes nothing.
//!
//! At a run's start, [`recover_orphaned_knowledge`] proposes the knowledge
//! batches earlier runs left unproposed, as a run that crashed before its end
//! leaves its batch (gap-6ff99a), through the same guard, which rolls them
//! back: nothing shows that their runs delivered.

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
    Delivery, KNOWLEDGE_STORE, KnowledgeBatch, KnowledgeChecks, load_knowledge_anchors,
    orphaned_batches, propose_batch, recover_orphaned_batches,
};

use super::plan_runner::PlanRunInterrupt;
use super::plan_set::PlanOutcome;
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
    /// Whether the run delivered every plan ([`run_delivery`]): its
    /// knowledge batch commits only then (bug-2ad4fa).
    pub delivery: Delivery,
}

/// Whether a run delivered every plan it set out to (bug-2ad4fa), from each
/// plan with its outcome, `None` for one that never settled, and what
/// stopped the run, if anything. Only a run whose every plan succeeded, and
/// that nothing stopped, delivered; the reason names every plan that did not
/// and the stop, such as "plan p5-ship interrupted; a cancel request stopped
/// the run".
pub fn run_delivery<'a>(
    plans: impl IntoIterator<Item = (&'a str, Option<PlanOutcome>)>,
    stopped_by: Option<PlanRunInterrupt>,
) -> Delivery {
    let mut delivered = Vec::new();
    let mut missed = Vec::new();
    for (plan, outcome) in plans {
        match outcome {
            Some(PlanOutcome::Succeeded) => delivered.push(plan),
            Some(outcome) => missed.push(format!("plan {plan} {}", outcome.label())),
            None => missed.push(format!("plan {plan} not started")),
        }
    }
    if let Some(stop) = stopped_by {
        missed.push(format!("{} stopped the run", stop.cause()));
    }
    match (missed.is_empty(), delivered.as_slice()) {
        (false, _) => Delivery::Undelivered(missed.join("; ")),
        (true, []) => Delivery::Undelivered("the run had no plan to deliver".to_string()),
        (true, [plan]) => Delivery::Delivered(format!("the run delivered plan {plan}")),
        (true, plans) => {
            Delivery::Delivered(format!("the run delivered plans {}", plans.join(", ")))
        }
    }
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

/// Settle the knowledge batch of a run that ends with `error` before its
/// learning is saved (bug-2ad4fa): the run did not deliver, so the batch is
/// rolled back, with the error as the reason, instead of being left for the
/// next run's start. The router keeps what it journaled for its next load,
/// as before.
pub fn settle_errored_run_knowledge(
    workdir: &Path,
    config: &RokoConfig,
    batch: Option<&str>,
    error: &dyn std::fmt::Display,
    notices: Option<&StateHub>,
) -> RunCommits {
    commit_run_learning(&RunLearning {
        workdir,
        config,
        batch,
        router: None,
        run_ids: &[],
        notices,
        delivery: Delivery::Undelivered(format!("the run ended with an error: {error}")),
    })
}

/// Save the router through its journal: under guard in `mode`, or unchecked
/// without one or when the run taught it nothing (gap-51d2ca).
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
    // `off` saves unchecked. So does a run that taught the router nothing, as
    // a run whose rungs each run one model with certainty does: it proposes
    // no version, so the commit log records only real changes (gap-51d2ca).
    // Its save merges nothing into the file, and still writes the file when
    // it is missing. What other processes merged into the file waits for the
    // next run that proposes a version.
    let learned = router.learned_since_save();
    let Some(mode) = mode.filter(|_| learned) else {
        if mode.is_some() {
            tracing::info!(
                "the run taught the cascade router nothing; no router version is proposed"
            );
        }
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

/// Propose the run's knowledge batch to the knowledge store's guard, which
/// commits it only when the run delivered (bug-2ad4fa).
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
    let checks = KnowledgeChecks::new(&store, batch, anchors, run.delivery.clone());
    let proposer = Proposer::evolved("knowledge", KNOWLEDGE_STORE, format!("knowledge:{batch}"));
    let learn_dir = layout.learn_dir();
    match propose_batch(&store, batch, &learn_dir, mode, &checks, &proposer) {
        Ok(decision) => {
            match (decision, &run.delivery) {
                (Some(CommitDecision::RolledBack), Delivery::Undelivered(why)) => publish_notice(
                    run.notices,
                    "knowledge",
                    format!(
                        "knowledge: the run did not deliver ({why}), so its knowledge batch was \
                         rolled back and none of its lessons was kept"
                    ),
                ),
                (Some(decision), _) => notice(run.notices, "knowledge", decision),
                (None, _) => {}
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
    publish_notice(notices, store, message);
}

/// Log `message`, a guard's notice about `store`, and publish it on the
/// run's StateHub.
fn publish_notice(notices: Option<&StateHub>, store: &str, message: String) {
    tracing::warn!(store, "{message}");
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
            delivery: run_delivery([("p1", Some(PlanOutcome::Succeeded))], None),
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

    /// gap-51d2ca: a run that taught the router nothing, as every
    /// ladder-routed run did while each rung ran one model, proposes no
    /// router version, so the commit log records only real changes. A run
    /// that taught it something proposes one; the next run, and one that
    /// loads the saved router, propose none until the router learns again.
    #[test]
    fn a_run_that_taught_the_router_nothing_proposes_no_version() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let layout = RokoLayout::for_project(workdir);
        let config = RokoConfig::default();
        assert_eq!(
            guard_mode(&config),
            Some(GuardMode::Observe),
            "decision 8103's default"
        );
        let models = || vec!["model-a".to_string(), "model-b".to_string()];
        let journal = ModelCallJournal::for_snapshot(&layout.cascade_router_path());
        let commit = |router: &CascadeRouter| {
            let run = RunLearning {
                workdir,
                config: &config,
                batch: None,
                router: Some((router, &journal)),
                run_ids: &[],
                notices: None,
                delivery: run_delivery([("p1", Some(PlanOutcome::Succeeded))], None),
            };
            commit_run_learning(&run).router
        };
        let versions = || {
            let guarded = GuardedStore::open(&layout.learn_dir(), ROUTER_STORE, GuardMode::Observe)
                .expect("the router's guard");
            guarded.rows().expect("its rows").len()
        };

        let router = CascadeRouter::new(models());
        for _ in 0..3 {
            assert_eq!(commit(&router), None, "nothing learned");
        }
        assert_eq!(versions(), 0);
        // The run still writes the router's file, as every run did.
        assert!(journal.snapshot_path().is_file(), "the router's snapshot");

        router.record_observation(&RoutingContext::default(), "model-a", 1.0, true);
        assert_eq!(commit(&router), Some(CommitDecision::Committed));
        assert_eq!(versions(), 1);
        assert_eq!(commit(&router), None, "nothing learned since");
        let loaded = CascadeRouter::load_or_new(journal.snapshot_path(), models());
        assert_eq!(commit(&loaded), None, "a loaded router learned nothing yet");
        assert_eq!(versions(), 1);
    }

    /// Write `entries` as the workspace's knowledge store.
    fn seed(knowledge: &KnowledgeStore, entries: &[KnowledgeEntry]) {
        let neuro = knowledge.path().parent().expect("the neuro directory");
        std::fs::create_dir_all(neuro).expect("create the neuro directory");
        let lines: String = entries
            .iter()
            .map(|entry| serde_json::to_string(entry).expect("an entry") + "\n")
            .collect();
        std::fs::write(knowledge.path(), lines).expect("seed the knowledge store");
    }

    /// bug-2ad4fa: phase 5's run 1f677933 lost its plan check and was
    /// cancelled, yet observe mode committed its batch, a gamed try's lesson
    /// among them. A run's batch commits only when the run delivered: this
    /// one's is rolled back in observe mode, its row says why and how many
    /// lessons went, the run's hub hears it, and its router still saves what
    /// the run taught it. A run that delivered commits its batch.
    #[test]
    fn a_cancelled_runs_lessons_are_not_committed() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let layout = RokoLayout::for_project(workdir);
        let config = RokoConfig::default();
        assert_eq!(guard_mode(&config), Some(GuardMode::Observe));
        let router = CascadeRouter::new(vec!["model-a".to_string(), "model-b".to_string()]);
        router.record_observation(&RoutingContext::default(), "model-a", 1.0, true);
        let journal = ModelCallJournal::for_snapshot(&layout.cascade_router_path());

        // A committed entry, and the run's batch: a gamed try's lesson and a
        // sound one.
        let in_batch = |batch: &str, entry: KnowledgeEntry| KnowledgeEntry {
            commit_batch: Some(batch.to_string()),
            ..entry
        };
        let committed = entry(
            "kn-toolchain",
            "Pin the toolchain before a release.",
            &[],
            0.8,
        );
        let gamed = entry(
            "kn-gamed",
            "tools/render-demos.sh must compensate locally for quiet showcase renders.",
            &["tools/render-demos.sh"],
            0.874,
        );
        let sound = entry(
            "kn-gain",
            "Preset loudness is matched by each preset's master gain.",
            &["presets"],
            0.8,
        );
        let knowledge = KnowledgeStore::for_layout(&layout);
        let seeded = [
            committed,
            in_batch("run-1", gamed),
            in_batch("run-1", sound),
        ];
        seed(&knowledge, &seeded);

        let delivery = run_delivery(
            [("p5-ship", Some(PlanOutcome::Interrupted))],
            Some(PlanRunInterrupt::Cancel),
        );
        let why = "plan p5-ship interrupted; a cancel request stopped the run";
        assert_eq!(delivery, Delivery::Undelivered(why.to_string()));
        let hub = crate::state_hub::shared_state_hub();
        let mut events = hub.subscribe_events();
        let commits = commit_run_learning(&RunLearning {
            workdir,
            config: &config,
            batch: Some("run-1"),
            router: Some((&router, &journal)),
            run_ids: &[],
            notices: Some(&hub),
            delivery,
        });
        assert_eq!(
            commits,
            RunCommits {
                router: Some(CommitDecision::Committed),
                knowledge: Some(CommitDecision::RolledBack),
            }
        );

        // Neither lesson is kept, and no run retrieves them.
        let ids = |store: &KnowledgeStore| -> Vec<String> {
            let entries = store.read_all().expect("read the knowledge store");
            entries.into_iter().map(|entry| entry.id).collect()
        };
        assert_eq!(ids(&knowledge), ["kn-toolchain"]);
        let hits = knowledge
            .query("render-demos showcase compensate", 5)
            .expect("query");
        assert!(hits.is_empty(), "{hits:?}");

        // The row: rolled back in observe mode, `delivered` first, saying why.
        let guarded = GuardedStore::open(&layout.learn_dir(), KNOWLEDGE_STORE, GuardMode::Observe)
            .expect("the knowledge store's guard");
        let rows = guarded.rows().expect("its rows");
        let row = rows.last().expect("the run's row");
        assert_eq!(row.decision, CommitDecision::RolledBack);
        assert_eq!(row.mode, GuardMode::Observe);
        let delivered = &row.checks[0];
        assert_eq!(delivered.check, "delivered");
        assert!(!delivered.passed, "{delivered:?}");
        let expected = format!("{why}; none of the batch's 2 lessons was kept");
        assert_eq!(delivered.reason, expected);

        // The run's hub hears why.
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
        assert!(notices[0].contains("did not deliver"), "{notices:?}");
        assert!(notices[0].contains(why), "{notices:?}");

        // A run that delivered commits its batch.
        let next = entry(
            "kn-renderer",
            "The renderer sets the showcase level, not the demo script.",
            &["renderer"],
            0.8,
        );
        let mut entries = knowledge.read_all().expect("read the knowledge store");
        entries.push(in_batch("run-2", next));
        seed(&knowledge, &entries);
        let delivery = run_delivery([("p5-1-ship", Some(PlanOutcome::Succeeded))], None);
        assert_eq!(
            delivery,
            Delivery::Delivered("the run delivered plan p5-1-ship".to_string())
        );
        let commits = commit_run_learning(&RunLearning {
            workdir,
            config: &config,
            batch: Some("run-2"),
            router: None,
            run_ids: &[],
            notices: None,
            delivery,
        });
        assert_eq!(commits.knowledge, Some(CommitDecision::Committed));
        assert_eq!(ids(&knowledge), ["kn-toolchain", "kn-renderer"]);
        let hits = knowledge
            .query("renderer showcase level", 5)
            .expect("query");
        assert_eq!(hits.len(), 1, "{hits:?}");
    }
}
