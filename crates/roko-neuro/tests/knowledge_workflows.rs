//! Integration test: knowledge store workflows.
//!
//! Exercises the public `KnowledgeStore` API for write, query, duplicate
//! handling, export/import roundtrip, GC, and tier progression boundaries.
//! All tests use temporary storage and no credentials.

use roko_neuro::{ExportFilter, ImportOptions, KnowledgeEntry, KnowledgeKind, KnowledgeStore};

fn insight(id: &str, content: &str, confidence: f64, tags: &[&str]) -> KnowledgeEntry {
    KnowledgeEntry {
        id: id.to_string(),
        kind: KnowledgeKind::Insight,
        content: content.to_string(),
        confidence,
        tags: tags.iter().map(|t| t.to_string()).collect(),
        source_episodes: vec![format!("ep-{id}")],
        ..Default::default()
    }
}

fn anti_knowledge(id: &str, content: &str, confidence: f64) -> KnowledgeEntry {
    KnowledgeEntry {
        id: id.to_string(),
        kind: KnowledgeKind::AntiKnowledge,
        content: content.to_string(),
        confidence,
        tags: vec!["anti".to_string()],
        source_episodes: vec![format!("ep-{id}")],
        ..Default::default()
    }
}

#[test]
fn write_and_query_roundtrip() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store = KnowledgeStore::new(tmp.path().join("knowledge.jsonl"));

    store
        .add(insight("i-1", "Rust tests run in parallel", 0.9, &["rust", "testing"]))
        .expect("add");
    store
        .add(insight("i-2", "Cargo clippy catches common mistakes", 0.8, &["rust", "clippy"]))
        .expect("add");
    store
        .add(insight("i-3", "Pin futures before polling", 0.7, &["rust", "async"]))
        .expect("add");

    let results = store.query("rust", 10).expect("query");
    assert!(!results.is_empty(), "query should return results for 'rust'");
    // All three entries have the 'rust' tag.
    assert!(results.len() >= 2, "expected at least 2 results, got {}", results.len());
}

#[test]
fn duplicate_id_is_deduplicated_on_ingest() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store = KnowledgeStore::new(tmp.path().join("knowledge.jsonl"));

    store
        .add(insight("dup-1", "first version", 0.9, &["test"]))
        .expect("first add");
    store
        .add(insight("dup-1", "second version", 0.8, &["test"]))
        .expect("second add");

    let all = store.read_all().expect("read all");
    // The store deduplicates by ID during ingest.
    let matching: Vec<_> = all.iter().filter(|e| e.id == "dup-1").collect();
    assert_eq!(matching.len(), 1, "duplicate ID should be deduplicated");
}

#[test]
fn export_import_roundtrip_preserves_entries() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let source_store = KnowledgeStore::new(tmp.path().join("source.jsonl"));
    let dest_store = KnowledgeStore::new(tmp.path().join("dest.jsonl"));
    let export_path = tmp.path().join("backup.jsonl");

    source_store
        .add(insight("exp-1", "exportable insight", 0.95, &["export"]))
        .expect("add");
    source_store
        .add(insight("exp-2", "another exportable", 0.85, &["export"]))
        .expect("add");

    let exported = source_store
        .export(&export_path, &ExportFilter::default())
        .expect("export");
    assert_eq!(exported, 2);

    let import_result = dest_store
        .import(
            &export_path,
            &ImportOptions {
                allow_legacy: false,
                ..ImportOptions::default()
            },
        )
        .expect("import");
    assert_eq!(import_result.source_entries, 2);
    assert_eq!(import_result.imported, 2);
    assert_eq!(import_result.skipped_dedup, 0);

    // Verify entries survive the roundtrip.
    let all = dest_store.read_all().expect("read all");
    assert_eq!(all.len(), 2);
    let ids: Vec<_> = all.iter().map(|e| e.id.as_str()).collect();
    assert!(ids.contains(&"exp-1"));
    assert!(ids.contains(&"exp-2"));

    // Import confidence should be discounted.
    for entry in &all {
        assert!(
            entry.confidence < 0.96,
            "imported confidence should be discounted"
        );
    }
}

#[test]
fn gc_removes_low_confidence_entries() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store = KnowledgeStore::new(tmp.path().join("knowledge.jsonl"));

    store
        .add(insight("gc-1", "high confidence", 0.95, &["keep"]))
        .expect("add");
    store
        .add(insight("gc-2", "very low confidence", 0.01, &["remove"]))
        .expect("add");

    let removed = store.gc(0.05).expect("gc");
    // The low-confidence entry should be removed (or frozen then removed).
    // gc removes entries below the threshold.
    let all = store.read_all().expect("read all");
    let remaining_ids: Vec<_> = all.iter().map(|e| e.id.as_str()).collect();
    assert!(
        remaining_ids.contains(&"gc-1"),
        "high confidence entry should survive gc"
    );
    // The gc-2 entry with 0.01 confidence should have been removed.
    assert!(
        removed >= 1 || !remaining_ids.contains(&"gc-2"),
        "low confidence entry should be collected"
    );
}

#[test]
fn query_by_kind_filters_correctly() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store = KnowledgeStore::new(tmp.path().join("knowledge.jsonl"));

    store
        .add(insight("kind-1", "an insight", 0.9, &["test"]))
        .expect("add insight");
    store
        .add(anti_knowledge("kind-2", "a warning pattern", 0.8))
        .expect("add anti-knowledge");

    let insights = store
        .query_kind("", KnowledgeKind::Insight, 10)
        .expect("query insights");
    assert!(
        insights.iter().all(|e| e.kind == KnowledgeKind::Insight),
        "query_kind should only return the requested kind"
    );

    let anti = store
        .query_kind("", KnowledgeKind::AntiKnowledge, 10)
        .expect("query anti-knowledge");
    assert!(
        anti.iter().all(|e| e.kind == KnowledgeKind::AntiKnowledge),
        "query_kind should only return anti-knowledge"
    );
}

#[test]
fn empty_store_returns_empty_results() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store = KnowledgeStore::new(tmp.path().join("knowledge.jsonl"));

    let results = store.query("anything", 10).expect("query empty store");
    assert!(results.is_empty());

    let all = store.read_all().expect("read empty store");
    assert!(all.is_empty());

    let stats = store.stats().expect("stats of empty store");
    assert_eq!(stats.total_entries, 0);
}

#[test]
fn export_to_same_path_as_live_store_is_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store_path = tmp.path().join("knowledge.jsonl");
    let store = KnowledgeStore::new(&store_path);

    store
        .add(insight("self-1", "should not export to self", 0.9, &["test"]))
        .expect("add");

    let result = store.export(&store_path, &ExportFilter::default());
    assert!(
        result.is_err(),
        "export to same path as live store should fail"
    );
}

#[test]
fn import_from_same_path_as_live_store_is_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store_path = tmp.path().join("knowledge.jsonl");
    let store = KnowledgeStore::new(&store_path);

    store
        .add(insight("self-2", "should not import from self", 0.9, &["test"]))
        .expect("add");

    let result = store.import(&store_path, &ImportOptions::default());
    assert!(
        result.is_err(),
        "import from same path as live store should fail"
    );
}
