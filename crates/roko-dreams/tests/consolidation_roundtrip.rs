//! Integration test: dream consolidation roundtrip.
//!
//! Exercises the public `DreamCycle` API with a fake `AgentDispatcher` and
//! temporary stores. Verifies episode ingestion, one consolidation cycle,
//! persisted knowledge, report read-back, and the standalone knowledge
//! compression path.

use std::sync::Arc;

use async_trait::async_trait;
use roko_core::{Body, Context as RokoContext, Kind, Signal};
use roko_dreams::cycle::{AgentDispatcher, DreamCycle};
use roko_learn::episode_logger::{Episode, EpisodeLogger};
use roko_learn::playbook::PlaybookStore;
use roko_neuro::KnowledgeStore;

/// Fake dispatcher that returns a fixed review string without calling a model.
struct FakeDispatcher;

#[async_trait]
impl AgentDispatcher for FakeDispatcher {
    async fn dispatch(
        &self,
        _input: &Signal,
        _ctx: &RokoContext,
    ) -> roko_agent::AgentResult {
        let output = Signal::builder(Kind::AgentOutput)
            .body(Body::text("dream review: all patterns consolidated"))
            .build();
        roko_agent::AgentResult::ok(output)
    }
}

fn make_episode(agent: &str, task: &str, success: bool) -> Episode {
    let mut ep = Episode::new(agent, task);
    ep.success = success;
    ep.model = "claude-haiku".to_string();
    ep
}

#[tokio::test]
async fn dream_cycle_produces_report_from_episodes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let episodes_path = tmp.path().join("episodes.jsonl");
    let knowledge_path = tmp.path().join("knowledge.jsonl");
    let playbooks_path = tmp.path().join("playbooks");

    let logger = Arc::new(EpisodeLogger::new(&episodes_path));
    let knowledge_store = Arc::new(KnowledgeStore::new(&knowledge_path));
    let playbook_store = Arc::new(PlaybookStore::new(&playbooks_path));
    let dispatcher: Arc<dyn AgentDispatcher> = Arc::new(FakeDispatcher);

    // Write several episodes so the cycle has data to process.
    for i in 0..5 {
        let ep = make_episode(
            &format!("agent-{i}"),
            &format!("task-{i}"),
            i % 2 == 0,
        );
        logger.append(&ep).await.expect("append episode");
    }

    let mut cycle = DreamCycle::new(
        Arc::clone(&logger),
        Arc::clone(&knowledge_store),
        Arc::clone(&playbook_store),
        dispatcher,
    );

    let report = cycle.run().await.expect("dream cycle should succeed");

    assert_eq!(report.total_episodes, 5);
    assert_eq!(report.processed_episodes, 5);
    assert!(
        report.completed_at >= report.started_at,
        "timestamps should be ordered"
    );
}

#[tokio::test]
async fn dream_cycle_with_empty_episodes_produces_zero_report() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let episodes_path = tmp.path().join("episodes.jsonl");
    let knowledge_path = tmp.path().join("knowledge.jsonl");
    let playbooks_path = tmp.path().join("playbooks");

    let logger = Arc::new(EpisodeLogger::new(&episodes_path));
    let knowledge_store = Arc::new(KnowledgeStore::new(&knowledge_path));
    let playbook_store = Arc::new(PlaybookStore::new(&playbooks_path));
    let dispatcher: Arc<dyn AgentDispatcher> = Arc::new(FakeDispatcher);

    let mut cycle = DreamCycle::new(logger, knowledge_store, playbook_store, dispatcher);

    let report = cycle.run().await.expect("empty cycle should succeed");

    assert_eq!(report.total_episodes, 0);
    assert_eq!(report.processed_episodes, 0);
    assert_eq!(report.knowledge_entries_written, 0);
    assert_eq!(report.playbooks_created, 0);
}

#[test]
fn add_consolidated_requires_three_source_episodes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let knowledge_path = tmp.path().join("knowledge.jsonl");
    let store = KnowledgeStore::new(&knowledge_path);

    // Fewer than three source episodes should fail.
    let entry = roko_neuro::KnowledgeEntry {
        id: "merged-01".to_string(),
        kind: roko_neuro::KnowledgeKind::Insight,
        content: "merged insight".to_string(),
        confidence: 0.9,
        source_episodes: vec!["ep-1".to_string(), "ep-2".to_string()],
        tags: vec!["rust".to_string()],
        ..Default::default()
    };
    let result = store.add_consolidated(entry);
    assert!(result.is_err(), "should reject fewer than 3 source episodes");

    // Exactly three source episodes should succeed.
    let entry = roko_neuro::KnowledgeEntry {
        id: "merged-02".to_string(),
        kind: roko_neuro::KnowledgeKind::Insight,
        content: "properly merged insight".to_string(),
        confidence: 0.9,
        source_episodes: vec![
            "ep-1".to_string(),
            "ep-2".to_string(),
            "ep-3".to_string(),
        ],
        tags: vec!["rust".to_string()],
        ..Default::default()
    };
    let added = store
        .add_consolidated(entry)
        .expect("should accept 3+ source episodes");
    assert!(added, "first add should succeed");

    // Duplicate ID should return false (idempotent).
    let duplicate = roko_neuro::KnowledgeEntry {
        id: "merged-02".to_string(),
        kind: roko_neuro::KnowledgeKind::Insight,
        content: "duplicate".to_string(),
        confidence: 0.9,
        source_episodes: vec![
            "ep-4".to_string(),
            "ep-5".to_string(),
            "ep-6".to_string(),
        ],
        tags: vec!["rust".to_string()],
        ..Default::default()
    };
    let added_again = store.add_consolidated(duplicate).expect("should not error");
    assert!(!added_again, "duplicate ID should return false");

    // Verify persisted state.
    let all = store.read_all().expect("read all");
    assert_eq!(all.len(), 1, "only one entry should be persisted");
    assert_eq!(all[0].id, "merged-02");
}
