#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use chrono::{DateTime, Duration, Utc};
    use roko_core::extension::CamelTaintLevel;
    use roko_core::{PadVector, TaintLevel};
    use tempfile::TempDir;

    use crate::{KnowledgeEntry, KnowledgeKind, KnowledgeTier, SourceChannel};

    use crate::knowledge_store::KnowledgeStore;
    #[cfg(feature = "hdc")]
    use crate::knowledge_store::MemoryIndex;
    use crate::knowledge_store::anti_pattern::{
        classify_compilation_error, extract_anti_pattern_from_failure,
    };
    #[cfg(feature = "hdc")]
    use crate::knowledge_store::scoring::{
        check_against_anti_knowledge, prepare_entries_for_ingest,
    };
    use crate::knowledge_store::scoring::{
        detect_confirmations, effective_confidence, entries_are_similar,
    };
    use crate::knowledge_store::types::*;

    fn entry(
        kind: KnowledgeKind,
        id: &str,
        content: &str,
        tags: &[&str],
        confidence: f64,
        source_episodes: &[&str],
        created_at: DateTime<Utc>,
    ) -> KnowledgeEntry {
        KnowledgeEntry {
            id: id.to_owned(),
            kind,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: content.to_owned(),
            confidence,
            confidence_weight: confidence,
            refuted_insight_id: None,
            refutation_evidence: None,
            source_episodes: source_episodes
                .iter()
                .map(|source| (*source).to_owned())
                .collect(),
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            source_model: None,
            model_generality: 1.0,
            created_at,
            half_life_days: kind.default_half_life_days(),
            tier: KnowledgeTier::Consolidated,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,

            confirmation_count: 0,

            distinct_contexts: Vec::new(),

            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
            hdc_encoder_version: 0,
            access_count: 0,
            last_accessed: None,
            contradiction_count: 0,
            activation_conditions: Vec::new(),
            commit_batch: None,
        }
    }

    #[test]
    fn external_ingress_labels_survive_persistence_round_trip() {
        let temp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(temp.path().join("knowledge.jsonl"));
        let candidate = entry(
            KnowledgeKind::AntiKnowledge,
            "external-round-trip",
            "untrusted external observation",
            &["security"],
            0.9,
            &["episode-external"],
            Utc::now(),
        );

        store
            .ingest_with_source(vec![candidate], SourceChannel::ExternalApi)
            .expect("ingest external entry");

        let persisted = store.read_all().expect("read persisted entry");
        assert_eq!(persisted.len(), 1);
        assert_eq!(persisted[0].source.as_deref(), Some("external-api"));
        assert_eq!(persisted[0].origin_taint, CamelTaintLevel::External);
        assert_eq!(persisted[0].classification, TaintLevel::Confidential);

        let raw = std::fs::read_to_string(store.path()).expect("read raw knowledge JSONL");
        assert!(raw.contains("\"origin_taint\":\"external\""));
        assert!(raw.contains("\"classification\":\"Confidential\""));
    }

    #[test]
    fn same_id_replay_can_raise_but_never_lower_security_labels() {
        let temp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(temp.path().join("knowledge.jsonl"));
        let original = entry(
            KnowledgeKind::AntiKnowledge,
            "monotonic-replay",
            "stable content",
            &["security"],
            0.9,
            &["episode-1"],
            Utc::now(),
        );
        store
            .ingest_with_source(vec![original], SourceChannel::GateVerdict)
            .expect("ingest trusted entry");

        let mut hostile_replay = entry(
            KnowledgeKind::AntiKnowledge,
            "monotonic-replay",
            "stable content",
            &["security"],
            0.9,
            &["episode-2"],
            Utc::now(),
        );
        hostile_replay.source = Some("user-external-import".to_string());
        hostile_replay.origin_taint = CamelTaintLevel::Trusted;
        hostile_replay.classification = TaintLevel::Public;
        store.add(hostile_replay).expect("replay external entry");

        let raised = store.read_all().expect("read raised label");
        assert_eq!(raised.len(), 1, "same-ID replay remains deduplicated");
        assert_eq!(raised[0].origin_taint, CamelTaintLevel::External);
        assert_eq!(raised[0].classification, TaintLevel::Confidential);

        let mut downgrade = raised[0].clone();
        downgrade.source = Some("manual-user".to_string());
        downgrade.origin_taint = CamelTaintLevel::Trusted;
        downgrade.classification = TaintLevel::Public;
        store.add(downgrade).expect("attempt lower-label replay");

        let retained = store.read_all().expect("read retained label");
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].origin_taint, CamelTaintLevel::External);
        assert_eq!(retained[0].classification, TaintLevel::Confidential);
    }

    #[test]
    fn extract_anti_pattern_from_failure_builds_anti_knowledge_entry() {
        let entry = extract_anti_pattern_from_failure(
            "task-42",
            "repair the compile failure in the Rust workspace",
            "compile",
            "error[E0425]: cannot find value `foo` in this scope\nerror[E0308]: mismatched types",
            Some("let bar = foo();"),
        );

        assert_eq!(entry.kind, KnowledgeKind::AntiKnowledge);
        assert_eq!(entry.tier, KnowledgeTier::Transient);
        assert!((entry.confidence - 0.6).abs() < f64::EPSILON);
        assert!((entry.confidence_weight + 0.6).abs() < f64::EPSILON);
        assert_eq!(entry.source.as_deref(), Some("bench-gate-failure"));
        assert!(entry.content.contains("Anti-pattern for task type"));
        assert!(entry.content.contains("Gate 'compile' failed with:"));
        assert!(entry.content.contains("Agent output snippet:"));
        assert!(entry.tags.contains(&"bench".to_string()));
        assert!(entry.tags.contains(&"gate:compile".to_string()));
        assert!(entry.tags.contains(&"task:task-42".to_string()));
        assert!(entry.tags.contains(&"error:unresolved-name".to_string()));
        assert!(entry.tags.contains(&"error:type-mismatch".to_string()));
        assert!(entry.tags.contains(&"error-code:E0425".to_string()));
        assert!(entry.tags.contains(&"error-code:E0308".to_string()));
    }

    #[test]
    fn classify_compilation_error_labels_common_codes() {
        let tags = classify_compilation_error(
            "error[E0425]: cannot find value `foo` in this scope\nerror[E0277]: trait bound not satisfied",
        );

        assert!(tags.contains(&"error:unresolved-name".to_string()));
        assert!(tags.contains(&"error:trait-not-satisfied".to_string()));
    }

    #[test]
    fn record_anti_pattern_from_failure_reinforces_existing_entry() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::for_roko_dir(tmp.path());
        let task_id = "task-42";
        let task_prompt = "repair the compile failure in the Rust workspace";
        let gate_name = "compile";
        let gate_error = "error[E0425]: cannot find value `foo` in this scope";
        let agent_output = Some("let bar = foo();");

        let initial = extract_anti_pattern_from_failure(
            task_id,
            task_prompt,
            gate_name,
            gate_error,
            agent_output,
        );

        store.add(initial.clone()).expect("seed anti knowledge");
        store
            .record_anti_pattern_from_failure(
                task_id,
                task_prompt,
                gate_name,
                gate_error,
                agent_output,
            )
            .expect("record repeated failure");

        let entries = store.read_all().expect("read entries");
        assert_eq!(entries.len(), 1);
        let updated = &entries[0];
        assert_eq!(updated.kind, KnowledgeKind::AntiKnowledge);
        assert!(updated.confidence > initial.confidence);
        assert!(updated.confidence_weight.is_sign_negative());
        assert_eq!(updated.confirmation_count, 1);
        assert!(updated.tags.contains(&"gate:compile".to_string()));
        assert!(updated.tags.contains(&"task:task-42".to_string()));
    }

    #[test]
    fn add_query_and_gc_roundtrip() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "Rust async actors and memory stores",
                &["rust", "async"],
                1.0,
                &["ep-a"],
                now,
            ))
            .expect("add first");
        store
            .add(entry(
                KnowledgeKind::Insight,
                "k2",
                "Rust data pipelines",
                &["rust"],
                0.8,
                &["ep-b"],
                now - Duration::days(10),
            ))
            .expect("add second");
        store
            .add(entry(
                KnowledgeKind::Insight,
                "k3",
                "Completely unrelated note",
                &["misc"],
                0.01,
                &[],
                now,
            ))
            .expect("add third");

        let results = store.query("rust async", 2).expect("query");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "k1");
        assert_eq!(results[1].id, "k2");

        store.gc(DEFAULT_GC_MIN_CONFIDENCE).expect("gc");
        let all = store.read_all().expect("read after gc");
        assert_eq!(all.len(), 2);
        assert!(all.iter().all(|entry| entry.id != "k3"));
    }

    #[test]
    fn update_confidence_clamps_and_promotes_tier() {
        let tmp = TempDir::new().expect("tempdir");
        let mut store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let mut knowledge = entry(
            KnowledgeKind::Insight,
            "confidence-up",
            "Confidence reinforcement should promote validated knowledge.",
            &["confidence"],
            0.89,
            &["ep1"],
            Utc::now(),
        );
        knowledge.tier = KnowledgeTier::Working;
        store.add(knowledge).expect("add knowledge");

        assert!(
            store
                .update_confidence("confidence-up", 0.20)
                .expect("update confidence")
        );
        assert!(
            !store
                .update_confidence("missing", 0.20)
                .expect("missing update")
        );

        let all = store.read_all().expect("read all");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].confidence, 1.0);
        assert_eq!(all[0].tier, KnowledgeTier::Consolidated);
    }

    /// A prompt's access is counted (S01 P0-9) without the spaced half-life
    /// extension, which is knowledge decay, held for now; `record_access`
    /// still applies it.
    #[test]
    fn record_access_counts_without_changing_half_life() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let (kind, now) = (KnowledgeKind::Insight, Utc::now());
        for (id, content) in [
            ("counted", "Prompt inclusions count as accesses"),
            ("spaced", "Spaced retrieval stretches the half-life"),
            ("untouched", "Unrelated deployment checklist for releases"),
        ] {
            let knowledge = entry(kind, id, content, &[], 0.9, &["ep1"], now);
            store.add(knowledge).expect("add knowledge");
        }
        let half_life = kind.default_half_life_days();
        let state = |id: &str| {
            let entries = store.read_all().expect("read the store");
            let entry = entries
                .iter()
                .find(|entry| entry.id == id)
                .unwrap_or_else(|| panic!("no entry {id}: {entries:?}"));
            let accessed = entry.last_accessed.is_some();
            (entry.access_count, entry.half_life_days, accessed)
        };

        assert_eq!(
            store.count_access(&["counted", "missing"]).expect("count"),
            1
        );
        assert_eq!(store.count_access(&["counted"]).expect("count again"), 1);
        assert_eq!(state("counted"), (2, half_life, true));
        assert_eq!(store.count_access(&[]).expect("count nothing"), 0);

        store.record_access(&["spaced"]).expect("first access");
        store.record_access(&["spaced"]).expect("second access");
        let (accesses, spaced, accessed) = state("spaced");
        assert_eq!((accesses, accessed), (2, true));
        assert!(spaced > half_life, "{spaced} <= {half_life}");
        assert_eq!(state("untouched"), (0, half_life, false));
    }

    /// bug-c4f0ed: every store of one file shares its write gate, whatever
    /// the path's spelling, and so does a store built before the file's
    /// directory existed; another file has a gate of its own.
    #[test]
    fn stores_of_one_file_share_one_write_gate() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("neuro").join("knowledge.jsonl");
        let early = KnowledgeStore::new(&path);
        std::fs::create_dir_all(tmp.path().join("neuro")).expect("mkdir");
        let late = KnowledgeStore::new(&path);
        let spelled = KnowledgeStore::new(tmp.path().join("neuro/../neuro/knowledge.jsonl"));
        let other = KnowledgeStore::new(tmp.path().join("neuro").join("other.jsonl"));

        assert!(Arc::ptr_eq(&early.write_gate, &late.write_gate));
        assert!(Arc::ptr_eq(&early.write_gate, &spelled.write_gate));
        assert!(!Arc::ptr_eq(&early.write_gate, &other.write_gate));
    }

    /// bug-c4f0ed: a write waits for the file's lock among processes, which
    /// another process's store holds while it rewrites the file.
    #[test]
    fn a_write_waits_for_the_lock_another_process_holds() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let (kind, now) = (KnowledgeKind::Insight, Utc::now());
        let content = "Prompt inclusions count as accesses";
        let knowledge = entry(kind, "counted", content, &[], 0.9, &["ep1"], now);
        store.add(knowledge).expect("add knowledge");
        let accesses = |store: &KnowledgeStore| store.read_all().expect("read")[0].access_count;

        // Another process's write holds the lock.
        let held = roko_fs::log_rotation::lock_jsonl(store.path()).expect("hold the lock");
        let writer = {
            let store = store.clone();
            std::thread::spawn(move || store.count_access(&["counted"]))
        };
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert_eq!(accesses(&store), 0, "the write waits for the lock");
        drop(held);
        let counted = writer.join().expect("the writer").expect("count");
        assert_eq!(counted, 1);
        assert_eq!(accesses(&store), 1);
    }

    #[test]
    fn record_usage_demotes_low_confidence_entry() {
        let tmp = TempDir::new().expect("tempdir");
        let mut store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let mut knowledge = entry(
            KnowledgeKind::Heuristic,
            "confidence-down",
            "Failed usage should reduce confidence.",
            &["confidence"],
            0.23,
            &["ep1"],
            Utc::now(),
        );
        knowledge.tier = KnowledgeTier::Persistent;
        store.add(knowledge).expect("add knowledge");

        store
            .record_usage("confidence-down", false)
            .expect("record usage");

        let all = store.read_all().expect("read all");
        assert!((all[0].confidence - 0.18).abs() < 1e-9);
        assert_eq!(all[0].tier, KnowledgeTier::Transient);
    }

    #[test]
    fn batch_record_usage_updates_once_and_shortens_weak_entries() {
        let tmp = TempDir::new().expect("tempdir");
        let mut store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let mut weak = entry(
            KnowledgeKind::Warning,
            "weak",
            "Repeated misses should make this decay quickly.",
            &["confidence"],
            0.08,
            &["ep1"],
            Utc::now(),
        );
        weak.tier = KnowledgeTier::Working;
        weak.half_life_days = 30.0;
        let stable = entry(
            KnowledgeKind::Insight,
            "stable",
            "Successful usage should reinforce this entry.",
            &["confidence"],
            0.50,
            &["ep2"],
            Utc::now(),
        );
        store.add(weak).expect("add weak");
        store.add(stable).expect("add stable");

        let updated = store
            .batch_record_usage(&[
                ("weak".to_owned(), false),
                ("stable".to_owned(), true),
                ("missing".to_owned(), true),
            ])
            .expect("batch record usage");

        assert_eq!(updated, 2);
        let all = store.read_all().expect("read all");
        let weak = all.iter().find(|entry| entry.id == "weak").expect("weak");
        let stable = all
            .iter()
            .find(|entry| entry.id == "stable")
            .expect("stable");
        assert!((weak.confidence - 0.03).abs() < 1e-9);
        assert_eq!(weak.tier, KnowledgeTier::Transient);
        assert_eq!(weak.half_life_days, 1.0);
        assert!((stable.confidence - 0.52).abs() < 1e-9);
    }

    #[test]
    fn decay_reduces_old_entries() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let created_at = Utc::now() - Duration::days(30);

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "A durable heuristic",
                &["heuristic"],
                1.0,
                &["ep-a", "ep-b"],
                created_at,
            ))
            .expect("add");

        store.decay().expect("decay");
        let all = store.read_all().expect("read");
        assert_eq!(all.len(), 1);
        assert!((all[0].confidence - 0.5).abs() < 0.05);
    }

    #[test]
    fn query_prefers_entries_validated_across_diverse_emotional_states() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        let mut high_diversity = entry(
            KnowledgeKind::Warning,
            "k-diverse",
            "Check rollback health before retrying a failed rollout",
            &["deploy", "rollback"],
            0.8,
            &["ep-a", "ep-b"],
            now,
        );
        high_diversity.emotional_provenance = Some(crate::EmotionalProvenance {
            average_pad: PadVector::new(-0.2, 0.3, 0.0),
            discovery_emotion: "negative_high_arousal".to_string(),
            validation_arc: Some(crate::ValidationArc::Redemptive),
            emotional_diversity: 1.0,
        });

        let mut low_diversity = entry(
            KnowledgeKind::Warning,
            "k-narrow",
            "Check rollback health before retrying a failed rollout after a database migration",
            &["deploy", "rollback"],
            0.8,
            &["ep-c", "ep-d"],
            now,
        );
        low_diversity.emotional_provenance = Some(crate::EmotionalProvenance {
            average_pad: PadVector::new(-0.2, 0.3, 0.0),
            discovery_emotion: "negative_high_arousal".to_string(),
            validation_arc: Some(crate::ValidationArc::Stable),
            emotional_diversity: 0.0,
        });

        store.add(low_diversity).expect("add narrow");
        store.add(high_diversity).expect("add diverse");

        let results = store
            .query("retry failed rollout rollback health", 2)
            .expect("query");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "k-diverse");
        assert_eq!(results[1].id, "k-narrow");
    }

    #[test]
    fn query_hits_expose_scoring_breakdown() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "Rust async actors and memory stores",
                &["rust", "async"],
                0.9,
                &["ep-a", "ep-b"],
                now,
            ))
            .expect("add first");
        store
            .add(entry(
                KnowledgeKind::Warning,
                "k2",
                "Retry loops can amplify flaky async tests",
                &["testing"],
                0.8,
                &["ep-c"],
                now - Duration::days(10),
            ))
            .expect("add second");

        let hits = store.query_hits("rust async", 5).expect("query hits");
        assert!(!hits.is_empty());
        assert_eq!(hits[0].entry.id, "k1");
        assert!(hits[0].total_score > QUERY_SCORE_FLOOR);
        assert!(hits[0].breakdown.keyword_score >= 2.0);
        assert!(hits[0].breakdown.effective_confidence > hits[0].entry.confidence);
        assert!(hits[0].breakdown.recency_factor > 0.9);
    }

    #[test]
    fn query_kind_hits_filter_by_kind() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "Prefer small async state machines",
                &["async"],
                0.9,
                &["ep-a"],
                now,
            ))
            .expect("add insight");
        store
            .add(entry(
                KnowledgeKind::StrategyFragment,
                "k2",
                "Break async migrations into small compileable steps",
                &["async", "migration"],
                0.95,
                &["ep-b", "ep-c", "ep-d"],
                now,
            ))
            .expect("add strategy fragment");

        let hits = store
            .query_kind_hits("async migration", KnowledgeKind::StrategyFragment, 5)
            .expect("query kind hits");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry.id, "k2");
        assert_eq!(hits[0].entry.kind, KnowledgeKind::StrategyFragment);
    }

    #[test]
    fn query_similar_ranks_by_hamming_similarity() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        let mut exact = entry(
            KnowledgeKind::Insight,
            "k-exact",
            "Exact fingerprint match",
            &["fingerprint"],
            0.9,
            &["ep-a"],
            now,
        );
        exact.hdc_vector = Some(vec![0; HDC_VECTOR_BYTES]);

        let mut close = entry(
            KnowledgeKind::Insight,
            "k-close",
            "Close fingerprint match",
            &["fingerprint"],
            0.8,
            &["ep-b"],
            now,
        );
        let mut close_fp = vec![0; HDC_VECTOR_BYTES];
        close_fp[0] = 0b0000_0011;
        close.hdc_vector = Some(close_fp);

        let mut far = entry(
            KnowledgeKind::Insight,
            "k-far",
            "Far fingerprint match",
            &["fingerprint"],
            0.7,
            &["ep-c"],
            now,
        );
        far.hdc_vector = Some(vec![0xFF; HDC_VECTOR_BYTES]);

        store.ingest(vec![far, close, exact]).expect("ingest");

        let query = vec![0; HDC_VECTOR_BYTES];
        let hits = store.query_similar(&query, 3).expect("query similar");

        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].entry.id, "k-exact");
        assert_eq!(hits[1].entry.id, "k-close");
        assert_eq!(hits[2].entry.id, "k-far");
        assert!(hits[0].similarity > hits[1].similarity);
        assert!(hits[1].similarity > hits[2].similarity);
    }

    #[test]
    fn query_similar_rejects_invalid_fingerprint_length() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));

        let error = store
            .query_similar(&[0_u8; 16], 1)
            .expect_err("invalid fingerprint length should fail");

        assert!(
            error
                .to_string()
                .contains("knowledge fingerprints must be 1280 bytes")
        );
    }

    #[test]
    fn ingest_skips_duplicate_ids() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();
        let duplicate = entry(
            KnowledgeKind::Insight,
            "dup",
            "Keep one durable copy",
            &["dup"],
            0.8,
            &["ep-a"],
            now,
        );

        store
            .ingest(vec![duplicate.clone(), duplicate.clone()])
            .expect("ingest duplicates");
        store.add(duplicate).expect("add duplicate again");

        let all = store.read_all().expect("read all");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "dup");
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn hdc_only_unrelated_entries_do_not_clear_query_floor() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "Completely unrelated note about shell prompts",
                &["misc"],
                0.0,
                &["ep-a"],
                now,
            ))
            .expect("add unrelated");

        let hits = store
            .query_hits("database migrations", 5)
            .expect("query hits");
        assert!(hits.is_empty());
    }

    #[test]
    fn query_prefers_emotionally_reinforced_entries() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        let mut neutral = entry(
            KnowledgeKind::Warning,
            "k-neutral",
            "Prefer the rollback path when rollout validation fails during a routine canary release",
            &["deploy", "rollback"],
            0.8,
            &["ep-a"],
            now,
        );
        neutral.emotional_provenance = Some(crate::EmotionalProvenance {
            average_pad: PadVector::new(-0.1, 0.2, 0.0),
            discovery_emotion: "neutral_mid_arousal".to_string(),
            validation_arc: Some(crate::ValidationArc::Stable),
            emotional_diversity: 0.0,
        });

        let mut reinforced = entry(
            KnowledgeKind::Warning,
            "k-reinforced",
            "Prefer the rollback path when rollout validation fails",
            &["deploy", "rollback"],
            0.8,
            &["ep-b"],
            now,
        );
        reinforced.emotional_tag = Some(roko_core::EmotionalTag::new(
            PadVector::new(-0.8, 0.4, 0.0),
            0.95,
            "rollback_failure",
            PadVector::new(-0.7, 0.3, 0.0),
        ));
        reinforced.emotional_provenance = Some(crate::EmotionalProvenance {
            average_pad: PadVector::new(-0.8, 0.4, 0.0),
            discovery_emotion: "negative_high_arousal".to_string(),
            validation_arc: Some(crate::ValidationArc::Redemptive),
            emotional_diversity: 1.0,
        });

        store.add(neutral).expect("add neutral");
        store.add(reinforced).expect("add reinforced");

        let results = store
            .query("rollback rollout validation failure", 2)
            .expect("query");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "k-reinforced");
    }

    #[test]
    fn decay_preserves_antiknowledge_confidence_floor() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let created_at = Utc::now() - Duration::days(365);

        store
            .add(KnowledgeEntry {
                id: "anti-floor".to_owned(),
                kind: KnowledgeKind::AntiKnowledge,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "This previously successful pattern regressed badly.".to_owned(),
                confidence: 0.8,
                confidence_weight: -0.8,
                refuted_insight_id: Some("insight-1".to_owned()),
                refutation_evidence: Some("repeated gate failures".to_owned()),
                source_episodes: vec!["ep-a".to_owned()],
                tags: vec!["anti_knowledge".to_owned(), "regression".to_owned()],
                source_model: None,
                model_generality: 1.0,
                created_at,
                half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
                tier: KnowledgeTier::Working,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
                hdc_encoder_version: 0,
                access_count: 0,
                last_accessed: None,
                contradiction_count: 0,
                activation_conditions: Vec::new(),
                commit_batch: None,
            })
            .expect("add anti knowledge");

        store.decay().expect("decay");
        let all = store.read_all().expect("read");
        assert_eq!(all.len(), 1);
        assert!((all[0].confidence - ANTI_KNOWLEDGE_CONFIDENCE_FLOOR).abs() < f64::EPSILON);
    }

    #[test]
    fn decay_uses_kind_specific_half_lives() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let created_at = Utc::now() - Duration::days(30);

        store
            .add(entry(
                KnowledgeKind::StrategyFragment,
                "strategy",
                "Reusable long-lived strategy fragment",
                &["strategy_fragment"],
                1.0,
                &[],
                created_at,
            ))
            .expect("add strategy fragment");
        store
            .add(entry(
                KnowledgeKind::Insight,
                "insight",
                "Short-lived insight",
                &["insight"],
                1.0,
                &["ep-a", "ep-b"],
                created_at,
            ))
            .expect("add insight");
        store
            .add(entry(
                KnowledgeKind::Heuristic,
                "heuristic",
                "Mid-lived heuristic",
                &["heuristic"],
                1.0,
                &[],
                created_at,
            ))
            .expect("add heuristic");

        store.decay().expect("decay");
        let all = store.read_all().expect("read");
        let strategy = all
            .iter()
            .find(|entry| entry.id == "strategy")
            .expect("strategy");
        let insight = all
            .iter()
            .find(|entry| entry.id == "insight")
            .expect("insight");
        let heuristic = all
            .iter()
            .find(|entry| entry.id == "heuristic")
            .expect("heuristic");

        assert!(heuristic.confidence > insight.confidence);
        assert!(insight.confidence > strategy.confidence);
        assert!((insight.confidence - 0.5).abs() < 0.05);
        assert!((strategy.confidence - 0.22).abs() < 0.05);
        assert!((heuristic.confidence - 0.79).abs() < 0.05);
    }

    #[test]
    fn decay_drops_below_half_after_two_half_lives() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let created_at = Utc::now() - Duration::days(60);

        store
            .add(entry(
                KnowledgeKind::Insight,
                "old-insight",
                "A stale but valid insight",
                &["insight"],
                1.0,
                &["ep-a", "ep-b"],
                created_at,
            ))
            .expect("add");

        store.decay().expect("decay");
        let all = store.read_all().expect("read");
        let confidence = all
            .iter()
            .find(|entry| entry.id == "old-insight")
            .expect("old insight")
            .confidence;
        assert!(confidence < 0.5);
        assert!((confidence - 0.25).abs() < 0.05);
    }

    #[test]
    fn confirmation_boost_retains_validated_entries_through_gc() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let created_at = Utc::now() - Duration::days(30);

        store
            .add(entry(
                KnowledgeKind::Insight,
                "single",
                "Single-source insight",
                &["insight"],
                0.4,
                &["ep-a"],
                created_at,
            ))
            .expect("add single");
        store
            .add(entry(
                KnowledgeKind::Insight,
                "validated",
                "Validated insight",
                &["insight"],
                0.4,
                &["ep-a", "ep-b"],
                created_at,
            ))
            .expect("add validated");

        store.gc(0.5).expect("gc");
        let all = store.read_all().expect("read");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "validated");
    }

    #[test]
    fn gc_preserves_antiknowledge_even_below_threshold() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(KnowledgeEntry {
                id: "anti-gc".to_owned(),
                kind: KnowledgeKind::AntiKnowledge,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "This optimization path is deceptively harmful.".to_owned(),
                confidence: 0.01,
                confidence_weight: -0.4,
                refuted_insight_id: Some("insight-2".to_owned()),
                refutation_evidence: Some("caused repeated failures".to_owned()),
                source_episodes: vec!["ep-a".to_owned()],
                tags: vec!["anti_knowledge".to_owned(), "optimization".to_owned()],
                source_model: None,
                model_generality: 1.0,
                created_at: now,
                half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
                tier: KnowledgeTier::Working,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
                hdc_encoder_version: 0,
                access_count: 0,
                last_accessed: None,
                contradiction_count: 0,
                activation_conditions: Vec::new(),
                commit_batch: None,
            })
            .expect("add anti knowledge");

        store.gc(0.95).expect("gc");
        let all = store.read_all().expect("read");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "anti-gc");
        assert!(
            (effective_confidence(&all[0]) - ANTI_KNOWLEDGE_CONFIDENCE_FLOOR).abs() < f64::EPSILON
        );
    }

    #[test]
    fn antiknowledge_halves_refuted_entry_confidence() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "insight-1",
                "A reusable insight",
                &["insight"],
                1.0,
                &["ep-a"],
                now,
            ))
            .expect("add original");
        store
            .add(KnowledgeEntry {
                id: "anti-1".to_owned(),
                kind: KnowledgeKind::AntiKnowledge,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "Previous insight insight-1 was wrong because it failed in practice."
                    .to_owned(),
                confidence: 0.9,
                confidence_weight: -0.9,
                refuted_insight_id: Some("insight-1".to_owned()),
                refutation_evidence: Some("it failed in practice".to_owned()),
                source_episodes: vec!["ep-b".to_owned()],
                tags: vec!["anti_knowledge".to_owned(), "insight".to_owned()],
                source_model: None,
                model_generality: 1.0,
                created_at: now,
                half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
                tier: KnowledgeTier::Working,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
                hdc_encoder_version: 0,
                access_count: 0,
                last_accessed: None,
                contradiction_count: 0,
                activation_conditions: Vec::new(),
                commit_batch: None,
            })
            .expect("add anti knowledge");

        let all = store.read_all().expect("read");
        let original = all
            .iter()
            .find(|entry| entry.id == "insight-1")
            .expect("original");
        let anti = all.iter().find(|entry| entry.id == "anti-1").expect("anti");

        assert!((original.confidence - 0.5).abs() < f64::EPSILON);
        assert_eq!(anti.kind, KnowledgeKind::AntiKnowledge);
    }

    #[test]
    fn stats_aggregate_by_kind_and_age() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(KnowledgeEntry {
                id: "oldest".to_owned(),
                kind: KnowledgeKind::Insight,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "first".to_owned(),
                confidence: 0.8,
                confidence_weight: 0.8,
                refuted_insight_id: None,
                refutation_evidence: None,
                source_episodes: Vec::new(),
                tags: Vec::new(),
                source_model: None,
                model_generality: 1.0,
                created_at: now - Duration::days(3),
                half_life_days: KnowledgeKind::Insight.default_half_life_days(),
                tier: KnowledgeTier::Consolidated,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
                hdc_encoder_version: 0,
                access_count: 0,
                last_accessed: None,
                contradiction_count: 0,
                activation_conditions: Vec::new(),
                commit_batch: None,
            })
            .expect("add oldest");
        store
            .add(KnowledgeEntry {
                id: "middle".to_owned(),
                kind: KnowledgeKind::StrategyFragment,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "second".to_owned(),
                confidence: 0.6,
                confidence_weight: 0.6,
                refuted_insight_id: None,
                refutation_evidence: None,
                source_episodes: Vec::new(),
                tags: Vec::new(),
                source_model: None,
                model_generality: 1.0,
                created_at: now - Duration::days(1),
                half_life_days: KnowledgeKind::StrategyFragment.default_half_life_days(),
                tier: KnowledgeTier::Consolidated,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
                hdc_encoder_version: 0,
                access_count: 0,
                last_accessed: None,
                contradiction_count: 0,
                activation_conditions: Vec::new(),
                commit_batch: None,
            })
            .expect("add middle");
        store
            .add(KnowledgeEntry {
                id: "newest".to_owned(),
                kind: KnowledgeKind::Insight,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "third".to_owned(),
                confidence: 1.0,
                confidence_weight: 1.0,
                refuted_insight_id: None,
                refutation_evidence: None,
                source_episodes: Vec::new(),
                tags: Vec::new(),
                source_model: None,
                model_generality: 1.0,
                created_at: now,
                half_life_days: KnowledgeKind::Insight.default_half_life_days(),
                tier: KnowledgeTier::Consolidated,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
                hdc_encoder_version: 0,
                access_count: 0,
                last_accessed: None,
                contradiction_count: 0,
                activation_conditions: Vec::new(),
                commit_batch: None,
            })
            .expect("add newest");

        let stats = store.stats().expect("stats");
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.kind_counts.get("insight"), Some(&2));
        assert_eq!(stats.kind_counts.get("strategy_fragment"), Some(&1));
        assert!((stats.average_confidence.expect("average") - 0.8).abs() < f64::EPSILON);
        assert_eq!(
            stats.oldest_entry.as_ref().map(|entry| entry.id.as_str()),
            Some("oldest")
        );
        assert_eq!(
            stats.newest_entry.as_ref().map(|entry| entry.id.as_str()),
            Some("newest")
        );
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn memory_index_search_prefers_matching_content() {
        let now = Utc::now();
        let index = MemoryIndex::from_entries(vec![
            entry(
                KnowledgeKind::Insight,
                "k1",
                "rust async memory retrieval",
                &["rust", "memory"],
                1.0,
                &["ep-a"],
                now,
            ),
            entry(
                KnowledgeKind::Insight,
                "k2",
                "postgres maintenance routine",
                &["db"],
                0.9,
                &[],
                now,
            ),
        ]);

        assert_eq!(index.len(), 2);
        assert!(!index.is_empty());

        let hits = index.search("rust async memory retrieval", 1);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry.id, "k1");
        assert!(hits[0].similarity >= 0.99);
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn knowledge_store_builds_memory_index() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "semantic retrieval over durable knowledge",
                &["memory"],
                1.0,
                &["ep-a"],
                now,
            ))
            .expect("add first");
        store
            .add(entry(
                KnowledgeKind::Insight,
                "k2",
                "completely unrelated topic",
                &["misc"],
                0.8,
                &[],
                now,
            ))
            .expect("add second");

        let index = store.memory_index().expect("index");
        assert_eq!(index.entries().len(), 2);
        let hits = index.search("semantic retrieval over durable knowledge", 1);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry.id, "k1");
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn causal_links_match_queries_by_cause_and_effect() {
        let now = Utc::now();
        let index = MemoryIndex::from_entries(vec![
            entry(
                KnowledgeKind::CausalLink,
                "k1",
                "high complexity -> more review",
                &[
                    "cause:high complexity",
                    "effect:more review",
                    "domain:coding",
                ],
                0.9,
                &["ep-a"],
                now,
            ),
            entry(
                KnowledgeKind::Insight,
                "k2",
                "postgres vacuum keeps tables healthy",
                &["postgres"],
                0.9,
                &["ep-b"],
                now,
            ),
        ]);

        let cause_hits = index.search("high complexity", 1);
        assert_eq!(cause_hits.len(), 1);
        assert_eq!(cause_hits[0].entry.id, "k1");

        let effect_hits = index.search("more review", 1);
        assert_eq!(effect_hits.len(), 1);
        assert_eq!(effect_hits[0].entry.id, "k1");
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn ingest_populates_hdc_vector_when_feature_is_enabled() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "semantic retrieval over durable knowledge",
                &["memory"],
                1.0,
                &["ep-a"],
                now,
            ))
            .expect("add entry");

        let all = store.read_all().expect("read");
        let vector = all[0].hdc_vector.as_ref().expect("persisted hdc vector");
        assert_eq!(vector.len(), HDC_VECTOR_BYTES);
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn backfill_hdc_vectors_populates_missing_and_is_idempotent() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        // Write two entries directly without HDC vectors (bypassing ingest normalization)
        // by using rewrite_all on raw entries.
        let mut e1 = entry(
            KnowledgeKind::Insight,
            "k1",
            "distributed tracing improves observability",
            &["tracing", "observability"],
            0.9,
            &["ep-a"],
            now,
        );
        let mut e2 = entry(
            KnowledgeKind::Heuristic,
            "k2",
            "prefer idempotent operations in distributed systems",
            &["distributed", "idempotent"],
            0.8,
            &["ep-b"],
            now,
        );
        // Ensure hdc_vector is absent so backfill has work to do.
        e1.hdc_vector = None;
        e2.hdc_vector = None;
        store.rewrite_all(&[e1, e2]).expect("write raw entries");

        // First backfill: both entries lack vectors, so 2 must be updated.
        let changed = store
            .backfill_hdc_vectors()
            .expect("backfill_hdc_vectors first pass");
        assert_eq!(changed, 2, "both entries should receive HDC vectors");

        // Verify vectors were persisted with the correct byte length.
        let all = store.read_all().expect("read after backfill");
        for e in &all {
            let vec = e.hdc_vector.as_ref().expect("hdc_vector must be set");
            assert_eq!(
                vec.len(),
                HDC_VECTOR_BYTES,
                "entry {} must have {HDC_VECTOR_BYTES}-byte HDC vector",
                e.id
            );
        }

        // Second backfill: all entries already have valid vectors — 0 changes.
        let changed_again = store
            .backfill_hdc_vectors()
            .expect("backfill_hdc_vectors second pass");
        assert_eq!(changed_again, 0, "backfill must be idempotent");
    }

    // ── Confirmation detection tests ─────────────────────────────────

    #[test]
    fn entries_are_similar_detects_tag_and_keyword_overlap() {
        let now = Utc::now();
        let existing = entry(
            KnowledgeKind::Insight,
            "k1",
            "Rust async actors are useful for concurrent pipelines",
            &["rust", "async", "concurrency"],
            1.0,
            &["ep-a"],
            now,
        );
        let similar = entry(
            KnowledgeKind::Insight,
            "k2",
            "Rust async runtime handles concurrent execution well",
            &["rust", "async"],
            0.9,
            &["ep-b"],
            now,
        );
        let unrelated = entry(
            KnowledgeKind::Insight,
            "k3",
            "PostgreSQL requires VACUUM for dead tuple cleanup",
            &["postgres", "maintenance"],
            0.8,
            &["ep-c"],
            now,
        );

        assert!(entries_are_similar(&existing, &similar));
        assert!(!entries_are_similar(&existing, &unrelated));
    }

    #[test]
    fn entries_are_similar_requires_minimum_keyword_overlap() {
        let now = Utc::now();
        let existing = entry(
            KnowledgeKind::Insight,
            "k1",
            "Rust async actors are useful",
            &["rust"],
            1.0,
            &["ep-a"],
            now,
        );
        // Shares the tag "rust" but only one keyword overlap ("rust").
        let one_keyword = entry(
            KnowledgeKind::Insight,
            "k2",
            "Rust borrow checker prevents data races",
            &["rust"],
            0.9,
            &["ep-b"],
            now,
        );

        // Meets MIN_TAG_OVERLAP but not MIN_KEYWORD_OVERLAP.
        assert!(!entries_are_similar(&existing, &one_keyword));
    }

    #[test]
    fn entries_are_similar_skips_antiknowledge() {
        let now = Utc::now();
        let existing = entry(
            KnowledgeKind::Insight,
            "k1",
            "Rust async actors are useful for concurrent pipelines",
            &["rust", "async"],
            1.0,
            &["ep-a"],
            now,
        );
        let anti = KnowledgeEntry {
            id: "anti-1".to_owned(),
            kind: KnowledgeKind::AntiKnowledge,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: "Rust async actors are not suitable for all concurrent pipelines".to_owned(),
            confidence: 0.9,
            confidence_weight: -0.9,
            refuted_insight_id: Some("k1".to_owned()),
            refutation_evidence: Some("test".to_owned()),
            source_episodes: vec!["ep-b".to_owned()],
            tags: vec!["rust".to_owned(), "async".to_owned()],
            source_model: None,
            model_generality: 1.0,
            created_at: now,
            half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
            tier: KnowledgeTier::Working,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,

            confirmation_count: 0,

            distinct_contexts: Vec::new(),

            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
            hdc_encoder_version: 0,
            access_count: 0,
            last_accessed: None,
            contradiction_count: 0,
            activation_conditions: Vec::new(),
            commit_batch: None,
        };

        assert!(!entries_are_similar(&existing, &anti));
    }

    #[test]
    fn detect_confirmations_finds_similar_entries() {
        let now = Utc::now();
        let existing = vec![entry(
            KnowledgeKind::Insight,
            "k1",
            "Rust async actors are useful for concurrent pipelines",
            &["rust", "async"],
            1.0,
            &["ep-a"],
            now,
        )];
        let new_entries = vec![entry(
            KnowledgeKind::Insight,
            "k2",
            "Rust async runtime handles concurrent execution well",
            &["rust", "async"],
            0.9,
            &["ep-b"],
            now,
        )];

        let confirmations = detect_confirmations(&existing, &new_entries);
        assert_eq!(confirmations.len(), 1);
        assert_eq!(confirmations[0].confirmed_entry_id, "k1");
        assert_eq!(confirmations[0].confirming_entry_id, "k2");
        assert!(
            confirmations[0]
                .source_episodes
                .contains(&"ep-a".to_owned())
        );
        assert!(
            confirmations[0]
                .source_episodes
                .contains(&"ep-b".to_owned())
        );
    }

    #[test]
    fn detect_confirmations_skips_unrelated_entries() {
        let now = Utc::now();
        let existing = vec![entry(
            KnowledgeKind::Insight,
            "k1",
            "Rust async actors are useful for concurrent pipelines",
            &["rust", "async"],
            1.0,
            &["ep-a"],
            now,
        )];
        let new_entries = vec![entry(
            KnowledgeKind::Insight,
            "k3",
            "PostgreSQL requires VACUUM for dead tuple cleanup",
            &["postgres", "maintenance"],
            0.8,
            &["ep-c"],
            now,
        )];

        let confirmations = detect_confirmations(&existing, &new_entries);
        assert!(confirmations.is_empty());
    }

    #[test]
    fn ingest_writes_confirmation_records_for_similar_entries() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        // Add first entry.
        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "Rust async actors are useful for concurrent pipelines",
                &["rust", "async"],
                1.0,
                &["ep-a"],
                now,
            ))
            .expect("add first");

        // No confirmations after first entry.
        let records = store.read_confirmations().expect("read confirmations");
        assert!(records.is_empty());

        // Add a similar entry.
        store
            .add(entry(
                KnowledgeKind::Insight,
                "k2",
                "Rust async runtime handles concurrent execution well",
                &["rust", "async"],
                0.9,
                &["ep-b"],
                now,
            ))
            .expect("add similar");

        // Now there should be a confirmation record.
        let records = store.read_confirmations().expect("read confirmations");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].confirmed_entry_id, "k1");
        assert_eq!(records[0].confirming_entry_id, "k2");
        assert!(records[0].source_episodes.contains(&"ep-a".to_owned()));
        assert!(records[0].source_episodes.contains(&"ep-b".to_owned()));
    }

    #[test]
    fn ingest_does_not_write_confirmations_for_unrelated_entries() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "Rust async actors are useful for concurrent pipelines",
                &["rust", "async"],
                1.0,
                &["ep-a"],
                now,
            ))
            .expect("add first");

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k3",
                "PostgreSQL requires VACUUM for dead tuple cleanup",
                &["postgres", "maintenance"],
                0.8,
                &["ep-c"],
                now,
            ))
            .expect("add unrelated");

        let records = store.read_confirmations().expect("read confirmations");
        assert!(records.is_empty());
    }

    #[test]
    fn confirmations_path_is_sibling_of_knowledge_path() {
        let store = KnowledgeStore::new("/some/path/neuro/knowledge.jsonl");
        assert_eq!(
            store.confirmations_path(),
            Path::new("/some/path/neuro/knowledge-confirmations.jsonl")
        );
    }

    #[test]
    fn ingest_promotes_high_support_entries_to_longer_tiers() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(KnowledgeEntry {
                id: "tiered".to_owned(),
                kind: KnowledgeKind::Insight,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "Repeatedly validated insight".to_owned(),
                confidence: 0.92,
                confidence_weight: 0.92,
                refuted_insight_id: None,
                refutation_evidence: None,
                source_episodes: vec!["ep-a".to_owned(), "ep-b".to_owned(), "ep-c".to_owned()],
                tags: vec!["tier".to_owned()],
                source_model: None,
                model_generality: 1.0,
                created_at: now,
                half_life_days: KnowledgeKind::Insight.default_half_life_days(),
                tier: KnowledgeTier::Transient,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
                hdc_encoder_version: 0,
                access_count: 0,
                last_accessed: None,
                contradiction_count: 0,
                activation_conditions: Vec::new(),
                commit_batch: None,
            })
            .expect("add tiered");

        let all = store.read_all().expect("read");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].tier, KnowledgeTier::Consolidated);
    }

    #[test]
    fn ingest_keeps_stronger_explicit_tiers() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(KnowledgeEntry {
                id: "persistent".to_owned(),
                kind: KnowledgeKind::StrategyFragment,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "A durable playbook fragment".to_owned(),
                confidence: 0.6,
                confidence_weight: 0.6,
                refuted_insight_id: None,
                refutation_evidence: None,
                source_episodes: vec!["ep-a".to_owned()],
                tags: vec!["strategy".to_owned()],
                source_model: None,
                model_generality: 1.0,
                created_at: now,
                half_life_days: KnowledgeKind::StrategyFragment.default_half_life_days(),
                tier: KnowledgeTier::Persistent,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
                hdc_encoder_version: 0,
                access_count: 0,
                last_accessed: None,
                contradiction_count: 0,
                activation_conditions: Vec::new(),
                commit_batch: None,
            })
            .expect("add persistent");

        let all = store.read_all().expect("read");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].tier, KnowledgeTier::Persistent);
    }

    #[test]
    fn stats_includes_tier_and_source_counts() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        // Use low confidence so normalize_entry_tier does not auto-promote.
        let mut e1 = entry(
            KnowledgeKind::Insight,
            "k1",
            "something useful",
            &["rust"],
            0.5,
            &["ep-a"],
            now,
        );
        e1.tier = KnowledgeTier::Working;
        e1.source = Some("local".to_owned());

        let mut e2 = entry(
            KnowledgeKind::AntiKnowledge,
            "k2",
            "do not retry on 5xx",
            &["http"],
            0.5,
            &["ep-b"],
            now,
        );
        e2.tier = KnowledgeTier::Working;

        store.add(e1).expect("add");
        store.add(e2).expect("add anti");

        let stats = store.stats().expect("stats");
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.anti_knowledge_count, 1);
        assert_eq!(stats.tier_counts.get("working"), Some(&2));
        assert_eq!(stats.source_counts.get("local"), Some(&1));
    }

    #[test]
    fn export_import_roundtrip_with_confidence_discount() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        let mut e = entry(
            KnowledgeKind::Insight,
            "k1",
            "important heuristic",
            &["rust"],
            0.5,
            &["ep-a"],
            now,
        );
        e.tier = KnowledgeTier::Consolidated;
        store.add(e).expect("add");

        // Export.
        let backup_path = tmp.path().join("backup.jsonl");
        let filter = ExportFilter::default();
        let count = store.export(&backup_path, &filter).expect("export");
        assert_eq!(count, 1);
        assert!(backup_path.exists());

        // Import into a fresh store.
        let store2 = KnowledgeStore::new(tmp.path().join("neuro2").join("knowledge.jsonl"));
        let options = ImportOptions {
            confidence_discount: 0.85,
            reset_tier: true,
            source_label: "backup-test".to_owned(),
            ..Default::default()
        };
        let imported = store2.import(&backup_path, &options).expect("import");
        assert_eq!(imported.imported, 1);

        let all = store2.read_all().expect("read");
        assert_eq!(all.len(), 1);
        // Confidence should be discounted: 0.5 * 0.85 = 0.425.
        assert!((all[0].confidence - 0.425).abs() < 0.01);
        // Tier should be reset to Transient (low confidence won't trigger promotion).
        assert_eq!(all[0].tier, KnowledgeTier::Transient);
        // Source label should be recorded.
        assert_eq!(all[0].source.as_deref(), Some("backup-test"));
    }

    #[test]
    fn export_filter_by_kind_and_confidence() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "high confidence insight",
                &["rust"],
                0.9,
                &["ep-a"],
                now,
            ))
            .expect("add");
        store
            .add(entry(
                KnowledgeKind::Warning,
                "k2",
                "low confidence warning",
                &["rust"],
                0.2,
                &["ep-b"],
                now,
            ))
            .expect("add");

        let backup_path = tmp.path().join("filtered.jsonl");
        let filter = ExportFilter {
            kinds: Some(vec![KnowledgeKind::Insight]),
            min_confidence: Some(0.5),
            ..Default::default()
        };
        let count = store.export(&backup_path, &filter).expect("export");
        assert_eq!(count, 1);
    }

    #[test]
    fn import_rejects_unsupported_version() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));

        let bad_backup = tmp.path().join("bad.jsonl");
        let header = BackupHeader {
            version: 99,
            created_at: Utc::now(),
            entry_count: 0,
            source_path: "test".to_owned(),
            merkle_root: String::new(),
        };
        std::fs::write(&bad_backup, serde_json::to_string(&header).unwrap() + "\n").unwrap();

        let result = store.import(&bad_backup, &ImportOptions::default());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unsupported backup version")
        );
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn anti_knowledge_check_rejects_near_duplicate() {
        // When a new entry has very high HDC similarity to an existing
        // AntiKnowledge entry, it should be filtered out.
        let anti = KnowledgeEntry {
            id: "anti-1".to_owned(),
            kind: KnowledgeKind::AntiKnowledge,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: "Never retry failed HTTP 5xx requests without backoff".to_owned(),
            confidence: 0.9,
            confidence_weight: 1.0,
            refuted_insight_id: Some("old-insight".to_owned()),
            refutation_evidence: Some("caused cascading failures".to_owned()),
            source_episodes: vec!["ep-a".to_owned()],
            tags: vec!["http".to_owned(), "retry".to_owned()],
            source_model: None,
            model_generality: 1.0,
            created_at: Utc::now(),
            half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
            tier: KnowledgeTier::Working,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
            hdc_encoder_version: 0,
            access_count: 0,
            last_accessed: None,
            contradiction_count: 0,
            activation_conditions: Vec::new(),
            commit_batch: None,
        };

        // A near-identical entry that should be rejected.
        let duplicate = KnowledgeEntry {
            id: "new-1".to_owned(),
            kind: KnowledgeKind::Insight,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: "Never retry failed HTTP 5xx requests without backoff".to_owned(),
            confidence: 0.8,
            confidence_weight: 1.0,
            refuted_insight_id: None,
            refutation_evidence: None,
            source_episodes: vec!["ep-b".to_owned()],
            tags: vec!["http".to_owned(), "retry".to_owned()],
            source_model: None,
            model_generality: 1.0,
            created_at: Utc::now(),
            half_life_days: KnowledgeKind::Insight.default_half_life_days(),
            tier: KnowledgeTier::Transient,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
            hdc_encoder_version: 0,
            access_count: 0,
            last_accessed: None,
            contradiction_count: 0,
            activation_conditions: Vec::new(),
            commit_batch: None,
        };

        // An unrelated entry that should pass through.
        let unrelated = KnowledgeEntry {
            id: "new-2".to_owned(),
            kind: KnowledgeKind::Insight,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: "PostgreSQL requires regular VACUUM for performance".to_owned(),
            confidence: 0.9,
            confidence_weight: 1.0,
            refuted_insight_id: None,
            refutation_evidence: None,
            source_episodes: vec!["ep-c".to_owned()],
            tags: vec!["postgres".to_owned(), "maintenance".to_owned()],
            source_model: None,
            model_generality: 1.0,
            created_at: Utc::now(),
            half_life_days: KnowledgeKind::Insight.default_half_life_days(),
            tier: KnowledgeTier::Transient,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
            hdc_encoder_version: 0,
            access_count: 0,
            last_accessed: None,
            contradiction_count: 0,
            activation_conditions: Vec::new(),
            commit_batch: None,
        };

        let existing = vec![anti];
        let new_entries = prepare_entries_for_ingest(vec![duplicate, unrelated]);

        let result = check_against_anti_knowledge(new_entries, &existing);
        // The near-duplicate should be rejected, leaving only the unrelated entry.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "new-2");
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn anti_knowledge_check_passes_antiknowledge_entries_through() {
        // AntiKnowledge entries themselves should not be blocked by existing
        // AntiKnowledge.
        let existing_anti = KnowledgeEntry {
            id: "anti-1".to_owned(),
            kind: KnowledgeKind::AntiKnowledge,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: "Never retry failed HTTP 5xx requests without backoff".to_owned(),
            confidence: 0.9,
            confidence_weight: 1.0,
            refuted_insight_id: Some("old".to_owned()),
            refutation_evidence: None,
            source_episodes: vec!["ep-a".to_owned()],
            tags: vec!["http".to_owned()],
            source_model: None,
            model_generality: 1.0,
            created_at: Utc::now(),
            half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
            tier: KnowledgeTier::Working,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
            hdc_encoder_version: 0,
            access_count: 0,
            last_accessed: None,
            contradiction_count: 0,
            activation_conditions: Vec::new(),
            commit_batch: None,
        };

        let new_anti = KnowledgeEntry {
            id: "anti-2".to_owned(),
            kind: KnowledgeKind::AntiKnowledge,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: "Never retry failed HTTP 5xx requests without backoff -- updated".to_owned(),
            confidence: 0.95,
            confidence_weight: 1.0,
            refuted_insight_id: Some("other".to_owned()),
            refutation_evidence: None,
            source_episodes: vec!["ep-b".to_owned()],
            tags: vec!["http".to_owned()],
            source_model: None,
            model_generality: 1.0,
            created_at: Utc::now(),
            half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
            tier: KnowledgeTier::Working,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
            hdc_encoder_version: 0,
            access_count: 0,
            last_accessed: None,
            contradiction_count: 0,
            activation_conditions: Vec::new(),
            commit_batch: None,
        };

        let existing = vec![existing_anti];
        let new_entries = prepare_entries_for_ingest(vec![new_anti]);
        let result = check_against_anti_knowledge(new_entries, &existing);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "anti-2");
    }

    // -----------------------------------------------------------------------
    // NEURO-10: Demurrage balance model and reinforcement signals
    // -----------------------------------------------------------------------

    #[test]
    fn reinforcement_bumps_balance() {
        let mut e = KnowledgeEntry {
            balance: 1.0,
            ..KnowledgeEntry::default()
        };
        let before = e.balance;
        e.reinforce(crate::ReinforcementSignal::Retrieved, 0.0);
        assert!(
            e.balance > before,
            "balance should increase after reinforcement"
        );

        // Novelty amplifies the bump.
        let mid = e.balance;
        e.reinforce(crate::ReinforcementSignal::Retrieved, 1.0);
        let bump_with_novelty = e.balance - mid;
        // Reset and do without novelty.
        e.balance = mid;
        e.reinforce(crate::ReinforcementSignal::Retrieved, 0.0);
        let bump_without_novelty = e.balance - mid;
        assert!(bump_with_novelty > bump_without_novelty);
    }

    #[test]
    fn balance_capped_at_five() {
        let mut e = KnowledgeEntry {
            balance: 4.95,
            ..KnowledgeEntry::default()
        };
        e.reinforce(crate::ReinforcementSignal::Surprised, 1.0);
        assert!(e.balance <= 5.0, "balance must not exceed 5.0");
    }

    #[test]
    fn demurrage_reduces_balance() {
        let mut e = KnowledgeEntry {
            balance: 1.0,
            ..KnowledgeEntry::default()
        };
        e.apply_demurrage(100.0);
        assert!(e.balance < 1.0, "demurrage should reduce balance");
        assert!(e.balance >= 0.0, "balance must not go negative");
    }

    #[test]
    fn demurrage_does_not_go_negative() {
        let mut e = KnowledgeEntry {
            balance: 0.01,
            ..KnowledgeEntry::default()
        };
        e.apply_demurrage(1_000_000.0);
        assert_eq!(e.balance, 0.0);
    }

    #[test]
    fn freshness_combines_balance_and_decay() {
        let now = Utc::now();
        let old = now - Duration::hours(24 * 30); // 30 days
        let mut e = KnowledgeEntry {
            balance: 1.0,
            half_life_days: 30.0,
            created_at: old,
            ..KnowledgeEntry::default()
        };
        let fresh_high = e.freshness(now);
        e.balance = 0.1;
        let fresh_low = e.freshness(now);
        assert!(fresh_high > fresh_low, "higher balance => higher freshness");
    }

    #[test]
    fn reinforcement_signal_base_values_positive() {
        for signal in &[
            crate::ReinforcementSignal::Retrieved,
            crate::ReinforcementSignal::Cited,
            crate::ReinforcementSignal::Gated,
            crate::ReinforcementSignal::Surprised,
            crate::ReinforcementSignal::AgentQuoted,
        ] {
            assert!(
                signal.base_value() > 0.0,
                "{:?} must have positive base_value",
                signal
            );
        }
    }

    // -----------------------------------------------------------------------
    // NEURO-10: Balance/freshness influence on query scoring
    // -----------------------------------------------------------------------

    /// Two entries with equal topic relevance, confidence, and recency: the one
    /// with higher balance should rank first because of the balance/freshness boost.
    #[test]
    fn query_prefers_balance_reinforced_entries() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());
        let now = Utc::now();

        // Use distinct tags and unique content to prevent the confirmation-detection
        // path from running (entries_are_similar fires on shared tags + keywords).
        // Both entries match the query but are distinct enough to not confirm each other.
        let low_balance = {
            let mut e = entry(
                KnowledgeKind::Insight,
                "low-balance",
                "Run deploy jobs inside the integration gating pipeline",
                &["deploy-gate", "integration"],
                0.8,
                &["ep-x"],
                now,
            );
            e.balance = 0.0; // zero: no reinforcement history
            e
        };

        let high_balance = {
            let mut e = entry(
                KnowledgeKind::Insight,
                "high-balance",
                "Always validate deploy artifacts in the gating pipeline",
                &["deploy-validate", "pipeline"],
                0.8,
                &["ep-y"],
                now,
            );
            e.balance = 3.0; // reinforced: should get the balance/freshness boost
            e
        };

        store.add(low_balance).unwrap();
        store.add(high_balance).unwrap();

        let hits = store
            .query_hits("deploy gating pipeline", 2)
            .expect("query_hits");
        assert_eq!(hits.len(), 2, "both entries should score above the floor");

        assert_eq!(
            hits[0].entry.id, "high-balance",
            "reinforced (high-balance) entry must rank first"
        );
        assert!(
            hits[0].breakdown.balance_freshness_boost > hits[1].breakdown.balance_freshness_boost,
            "high-balance entry must have a larger balance_freshness_boost in the breakdown"
        );
    }

    #[test]
    fn store_reinforce_entry() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());
        let mut e = entry(
            KnowledgeKind::Insight,
            "reinforce-me",
            "test entry",
            &["test"],
            0.8,
            &["ep1"],
            Utc::now(),
        );
        e.balance = 0.5;
        store.add(e).unwrap();

        let found = store
            .reinforce_entry("reinforce-me", crate::ReinforcementSignal::Gated, 0.2)
            .unwrap();
        assert!(found);

        let entries = store.read_all().unwrap();
        let updated = entries.iter().find(|e| e.id == "reinforce-me").unwrap();
        assert!(updated.balance > 0.5, "balance should have been bumped");
    }

    #[test]
    fn store_apply_demurrage() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());
        let mut e = entry(
            KnowledgeKind::Insight,
            "demurrage-test",
            "test entry",
            &["test"],
            0.8,
            &["ep1"],
            Utc::now() - Duration::hours(100),
        );
        e.balance = 1.0;
        store.add(e).unwrap();

        let taxed = store.apply_demurrage().unwrap();
        assert_eq!(taxed, 1);

        let entries = store.read_all().unwrap();
        let updated = entries.iter().find(|e| e.id == "demurrage-test").unwrap();
        assert!(updated.balance < 1.0);
    }

    // -----------------------------------------------------------------------
    // NEURO-11: Cold-tier freeze/thaw
    // -----------------------------------------------------------------------

    #[test]
    fn freeze_and_thaw_entry() {
        let mut e = KnowledgeEntry {
            balance: 0.01,
            ..KnowledgeEntry::default()
        };
        assert!(!e.frozen);
        e.freeze();
        assert!(e.frozen);
        e.thaw(0.3);
        assert!(!e.frozen);
        assert!((e.balance - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn frozen_entries_excluded_from_hot_queries() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());
        let mut e = entry(
            KnowledgeKind::Insight,
            "frozen-entry",
            "important knowledge about testing",
            &["testing"],
            0.8,
            &["ep1"],
            Utc::now(),
        );
        e.frozen = true;
        store.add(e).unwrap();

        let results = store.query("testing", 10).unwrap();
        assert!(
            results.is_empty(),
            "frozen entries should not appear in hot queries"
        );

        let cold = store.query_cold(10).unwrap();
        assert_eq!(
            cold.len(),
            1,
            "frozen entries should appear in cold queries"
        );
        assert_eq!(cold[0].id, "frozen-entry");
    }

    #[test]
    fn gc_with_freeze_freezes_low_confidence_entries() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());

        let low = entry(
            KnowledgeKind::Insight,
            "low-conf",
            "fading knowledge",
            &["test"],
            0.01,
            &["ep1"],
            Utc::now(),
        );
        store.add(low).unwrap();

        let removed = store.gc_with_freeze(0.05).unwrap();
        assert_eq!(removed, 0, "entry should be frozen, not removed");

        let entries = store.read_all().unwrap();
        let frozen_entry = entries.iter().find(|e| e.id == "low-conf").unwrap();
        assert!(frozen_entry.frozen, "entry should have been frozen");
    }

    #[test]
    fn gc_with_freeze_removes_already_frozen_below_threshold() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());

        // Already frozen entry below confidence threshold.
        let mut frozen = entry(
            KnowledgeKind::Insight,
            "already-frozen",
            "old frozen knowledge",
            &["test"],
            0.01,
            &["ep1"],
            Utc::now(),
        );
        frozen.frozen = true;
        store.add(frozen).unwrap();

        let removed = store.gc_with_freeze(0.05).unwrap();
        assert_eq!(
            removed, 1,
            "already-frozen entry below threshold should be permanently removed"
        );
    }

    #[test]
    fn store_thaw_entry() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());
        let mut e = entry(
            KnowledgeKind::Insight,
            "thaw-me",
            "frozen knowledge",
            &["test"],
            0.8,
            &["ep1"],
            Utc::now(),
        );
        e.frozen = true;
        e.balance = 0.0;
        store.add(e).unwrap();

        let thawed = store.thaw_entry("thaw-me", 0.3).unwrap();
        assert!(thawed);

        let entries = store.read_all().unwrap();
        let updated = entries.iter().find(|e| e.id == "thaw-me").unwrap();
        assert!(!updated.frozen);
        assert!((updated.balance - 0.3).abs() < f64::EPSILON);
    }
}

#[cfg(test)]
mod anti_pattern_tests {
    use chrono::{DateTime, Duration, Utc};
    use tempfile::TempDir;

    use std::path::Path;

    use crate::knowledge_store::KnowledgeStore;
    use crate::knowledge_store::anti_pattern::extract_anti_pattern_from_failure;
    use crate::knowledge_store::backup::{
        compute_entry_merkle_root, compute_merkle_root, read_import_entries,
    };
    use crate::knowledge_store::types::*;
    use crate::temporal::KnowledgeEpoch;
    use crate::{Falsifier, KnowledgeEntry, KnowledgeKind, KnowledgeTier};
    #[cfg(feature = "hdc")]
    use roko_primitives::hdc::HdcVector;

    #[test]
    fn test_extract_creates_anti_knowledge() {
        let entry = extract_anti_pattern_from_failure(
            "task-1",
            "Implement add function",
            "compile",
            "error[E0425]: cannot find value `x` in this scope",
            Some("fn add(a: i32, b: i32) -> i32 { x + y }"),
        );

        assert_eq!(entry.kind, KnowledgeKind::AntiKnowledge);
        assert_eq!(entry.tier, KnowledgeTier::Transient);
        assert!(entry.content.contains("compile"));
        assert!(entry.content.contains("E0425"));
        assert!(entry.tags.contains(&"gate:compile".to_string()));
        assert!(entry.tags.contains(&"task:task-1".to_string()));
        assert!(entry.confidence > 0.0 && entry.confidence <= 1.0);
    }

    #[test]
    fn test_extract_without_agent_output() {
        let entry = extract_anti_pattern_from_failure(
            "task-2",
            "Fix imports",
            "test",
            "test failed: expected true, got false",
            None,
        );

        assert_eq!(entry.kind, KnowledgeKind::AntiKnowledge);
        assert!(entry.content.contains("test"));
        assert!(!entry.content.contains("Agent output"));
    }

    #[test]
    fn test_extract_tags_include_error_codes() {
        let entry = extract_anti_pattern_from_failure(
            "task-3",
            "Type error task",
            "compile",
            "error[E0308]: mismatched types",
            None,
        );

        assert!(entry.tags.iter().any(|tag| tag.starts_with("error:")));
    }

    #[test]
    fn test_anti_pattern_is_queryable() {
        let dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(dir.path().join("knowledge.jsonl"));

        let entry = extract_anti_pattern_from_failure(
            "task-4",
            "Implement iterator",
            "compile",
            "error[E0277]: trait bound not satisfied",
            None,
        );

        store.add(entry).unwrap();

        let results = store
            .query_kind("Implement iterator", KnowledgeKind::AntiKnowledge, 5)
            .unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].kind, KnowledgeKind::AntiKnowledge);
        assert!(results[0].content.contains("E0277"));
    }

    // ── Merkle root tests (E43-T01) ───────────────────────────────────

    fn entry(
        kind: KnowledgeKind,
        id: &str,
        content: &str,
        tags: &[&str],
        confidence: f64,
        source_episodes: &[&str],
        created_at: DateTime<Utc>,
    ) -> KnowledgeEntry {
        KnowledgeEntry {
            id: id.to_owned(),
            kind,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: content.to_owned(),
            confidence,
            confidence_weight: confidence,
            refuted_insight_id: None,
            refutation_evidence: None,
            source_episodes: source_episodes
                .iter()
                .map(|source| (*source).to_owned())
                .collect(),
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            source_model: None,
            model_generality: 1.0,
            created_at,
            half_life_days: kind.default_half_life_days(),
            tier: KnowledgeTier::Consolidated,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
            hdc_encoder_version: 0,
            access_count: 0,
            last_accessed: None,
            contradiction_count: 0,
            activation_conditions: Vec::new(),
            commit_batch: None,
        }
    }

    #[test]
    fn export_includes_merkle_root_in_header() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "first entry",
                &["rust"],
                0.8,
                &["ep-a"],
                now,
            ))
            .expect("add");
        store
            .add(entry(
                KnowledgeKind::Heuristic,
                "k2",
                "second entry",
                &["tooling"],
                0.6,
                &["ep-b"],
                now,
            ))
            .expect("add");

        let backup_path = tmp.path().join("merkle.jsonl");
        let count = store
            .export(&backup_path, &ExportFilter::default())
            .expect("export");
        assert_eq!(count, 2);

        // Parse the header line and verify merkle_root is non-empty.
        let content = std::fs::read_to_string(&backup_path).expect("read");
        let header_line = content.lines().next().expect("header line");
        let header: BackupHeader = serde_json::from_str(header_line).expect("parse header");
        assert!(
            !header.merkle_root.is_empty(),
            "merkle_root must be non-empty for non-empty export"
        );
        // Must be a 64-char lowercase hex SHA-256.
        assert_eq!(
            header.merkle_root.len(),
            64,
            "merkle_root must be 64 hex chars (SHA-256)"
        );
        assert!(
            header.merkle_root.chars().all(|c| c.is_ascii_hexdigit()),
            "merkle_root must be hex"
        );
    }

    #[test]
    fn export_merkle_root_is_deterministic() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let now = Utc::now();

        let store_a = KnowledgeStore::new(tmp.path().join("a").join("knowledge.jsonl"));
        store_a
            .add(entry(
                KnowledgeKind::Insight,
                "id-1",
                "alpha",
                &["x"],
                0.9,
                &[],
                now,
            ))
            .expect("add");
        store_a
            .add(entry(
                KnowledgeKind::Insight,
                "id-2",
                "beta",
                &["y"],
                0.7,
                &[],
                now,
            ))
            .expect("add");

        let store_b = KnowledgeStore::new(tmp.path().join("b").join("knowledge.jsonl"));
        // Insert in reverse order.
        store_b
            .add(entry(
                KnowledgeKind::Insight,
                "id-2",
                "beta",
                &["y"],
                0.7,
                &[],
                now,
            ))
            .expect("add");
        store_b
            .add(entry(
                KnowledgeKind::Insight,
                "id-1",
                "alpha",
                &["x"],
                0.9,
                &[],
                now,
            ))
            .expect("add");

        let path_a = tmp.path().join("out_a.jsonl");
        let path_b = tmp.path().join("out_b.jsonl");
        store_a
            .export(&path_a, &ExportFilter::default())
            .expect("export a");
        store_b
            .export(&path_b, &ExportFilter::default())
            .expect("export b");

        let header_a: BackupHeader = serde_json::from_str(
            std::fs::read_to_string(&path_a)
                .expect("read a")
                .lines()
                .next()
                .expect("line"),
        )
        .expect("parse a");
        let header_b: BackupHeader = serde_json::from_str(
            std::fs::read_to_string(&path_b)
                .expect("read b")
                .lines()
                .next()
                .expect("line"),
        )
        .expect("parse b");

        assert_eq!(
            header_a.merkle_root, header_b.merkle_root,
            "merkle root must be deterministic regardless of insertion order"
        );
    }

    #[test]
    fn compute_merkle_root_empty_returns_empty_string() {
        assert_eq!(compute_merkle_root(&[]), "");
    }

    #[test]
    fn compute_merkle_root_single_entry() {
        let root = compute_merkle_root(&["only-entry".to_owned()]);
        // Single leaf: root = SHA-256("only-entry") as hex.
        assert_eq!(root.len(), 64);
        assert!(root.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn compute_merkle_root_order_independent() {
        let ids_a = vec!["alpha".to_owned(), "beta".to_owned(), "gamma".to_owned()];
        let mut ids_b = ids_a.clone();
        ids_b.reverse();
        assert_eq!(
            compute_merkle_root(&ids_a),
            compute_merkle_root(&ids_b),
            "Merkle root must be order-independent (IDs are sorted internally)"
        );
    }

    fn write_canonical_test_backup(path: &Path, entries: &[KnowledgeEntry]) {
        let header = BackupHeader {
            version: KNOWLEDGE_BACKUP_VERSION,
            created_at: Utc::now(),
            entry_count: entries.len(),
            source_path: "test".to_owned(),
            merkle_root: compute_entry_merkle_root(entries).expect("compute root"),
        };
        let mut contents = serde_json::to_string(&header).expect("serialize header");
        contents.push('\n');
        for entry in entries {
            contents.push_str(&serde_json::to_string(entry).expect("serialize entry"));
            contents.push('\n');
        }
        std::fs::write(path, contents).expect("write canonical backup");
    }

    fn assert_failed_import_preserves_store(
        destination: &KnowledgeStore,
        backup: &Path,
        expected_error: &str,
    ) {
        let before = destination.read_all().expect("read before failed import");
        let error = destination
            .import(backup, &ImportOptions::default())
            .expect_err("import must fail");
        assert!(
            format!("{error:#}").contains(expected_error),
            "expected `{expected_error}` in `{error:#}`"
        );
        assert_eq!(
            destination.read_all().expect("read after failed import"),
            before,
            "validation failure must not partially write"
        );
    }

    #[test]
    fn export_default_filters_secrets_before_top_n() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("knowledge.jsonl"));
        let now = Utc::now();
        store
            .add(entry(
                KnowledgeKind::Insight,
                "secret",
                "ANTHROPIC_API_KEY=private-value",
                &["api_key"],
                0.99,
                &[],
                now,
            ))
            .expect("add secret");
        store
            .add(entry(
                KnowledgeKind::Insight,
                "safe",
                "bounded retries improve reliability",
                &["reliability"],
                0.75,
                &[],
                now,
            ))
            .expect("add safe");

        let backup = tmp.path().join("export.jsonl");
        let count = store
            .export(
                &backup,
                &ExportFilter {
                    max_entries: Some(1),
                    ..Default::default()
                },
            )
            .expect("export");
        assert_eq!(count, 1);
        let (entries, legacy) = read_import_entries(&backup, false).expect("validate export");
        assert!(!legacy);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "safe");
    }

    #[test]
    fn import_rejects_content_and_id_tampering_without_partial_writes() {
        let tmp = TempDir::new().expect("tempdir");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        destination
            .add(entry(
                KnowledgeKind::Insight,
                "existing",
                "existing durable knowledge",
                &["existing"],
                0.9,
                &[],
                Utc::now(),
            ))
            .expect("seed destination");
        let source = entry(
            KnowledgeKind::Insight,
            "source-id",
            "original content",
            &["source"],
            0.9,
            &[],
            Utc::now(),
        );

        for (name, needle, replacement) in [
            ("content", "original content", "tampered content"),
            ("id", "source-id", "tampered-id"),
        ] {
            let backup = tmp.path().join(format!("tampered-{name}.jsonl"));
            write_canonical_test_backup(&backup, std::slice::from_ref(&source));
            let contents = std::fs::read_to_string(&backup)
                .expect("read backup")
                .replace(needle, replacement);
            std::fs::write(&backup, contents).expect("tamper backup");
            assert_failed_import_preserves_store(
                &destination,
                &backup,
                "backup Merkle verification failed",
            );
        }
    }

    #[test]
    fn import_rejects_count_mismatch_malformed_and_truncated_inputs_without_writes() {
        let tmp = TempDir::new().expect("tempdir");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        let source = entry(
            KnowledgeKind::Insight,
            "source",
            "valid import source",
            &["source"],
            0.9,
            &[],
            Utc::now(),
        );

        let count_mismatch = tmp.path().join("count-mismatch.jsonl");
        write_canonical_test_backup(&count_mismatch, std::slice::from_ref(&source));
        let contents = std::fs::read_to_string(&count_mismatch)
            .expect("read")
            .replacen("\"entry_count\":1", "\"entry_count\":2", 1);
        std::fs::write(&count_mismatch, contents).expect("write mismatch");
        assert_failed_import_preserves_store(
            &destination,
            &count_mismatch,
            "backup entry_count mismatch",
        );

        let malformed = tmp.path().join("malformed.jsonl");
        write_canonical_test_backup(&malformed, std::slice::from_ref(&source));
        let header = std::fs::read_to_string(&malformed)
            .expect("read")
            .lines()
            .next()
            .expect("header")
            .to_owned();
        std::fs::write(&malformed, format!("{header}\n{{\n")).expect("write malformed");
        assert_failed_import_preserves_store(&destination, &malformed, "malformed_entries=1");

        let truncated = tmp.path().join("truncated.jsonl");
        write_canonical_test_backup(&truncated, &[source]);
        let header = std::fs::read_to_string(&truncated)
            .expect("read")
            .lines()
            .next()
            .expect("header")
            .to_owned();
        std::fs::write(&truncated, format!("{header}\n")).expect("write truncated");
        assert_failed_import_preserves_store(
            &destination,
            &truncated,
            "backup entry_count mismatch",
        );
    }

    #[test]
    fn import_reports_exact_id_and_semantic_dedup_counts() {
        let tmp = TempDir::new().expect("tempdir");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        destination
            .add(entry(
                KnowledgeKind::Insight,
                "existing",
                "semantic duplicate content",
                &["dedup"],
                0.9,
                &[],
                Utc::now(),
            ))
            .expect("seed destination");
        let duplicate_semantic = entry(
            KnowledgeKind::Insight,
            "semantic-copy",
            "semantic duplicate content",
            &["dedup"],
            0.8,
            &[],
            Utc::now(),
        );
        let first = entry(
            KnowledgeKind::Heuristic,
            "repeated-id",
            "first repeated ID entry",
            &["first"],
            0.8,
            &[],
            Utc::now(),
        );
        let second = entry(
            KnowledgeKind::Warning,
            "repeated-id",
            "second repeated ID entry",
            &["second"],
            0.8,
            &[],
            Utc::now(),
        );
        let backup = tmp.path().join("duplicates.jsonl");
        write_canonical_test_backup(&backup, &[duplicate_semantic, first, second]);

        let result = destination
            .import(&backup, &ImportOptions::default())
            .expect("import");
        assert_eq!(result.source_entries, 3);
        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped_dedup, 2);
        assert_eq!(result.skipped_contradiction, 0);
        assert_eq!(destination.read_all().expect("read").len(), 2);
    }

    #[test]
    fn import_unconditionally_skips_high_confidence_contradictions() {
        let tmp = TempDir::new().expect("tempdir");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        destination
            .add(entry(
                KnowledgeKind::AntiKnowledge,
                "refutation",
                "never retry an irreversible payment",
                &["payments", "retry"],
                0.95,
                &[],
                Utc::now(),
            ))
            .expect("seed AntiKnowledge");
        let candidate = entry(
            KnowledgeKind::Insight,
            "contradiction",
            "never retry an irreversible payment",
            &["payments", "retry"],
            0.95,
            &[],
            Utc::now(),
        );
        let backup = tmp.path().join("contradiction.jsonl");
        write_canonical_test_backup(&backup, &[candidate]);

        let result = destination
            .import(&backup, &ImportOptions::default())
            .expect("import");
        assert_eq!(result.imported, 0);
        assert_eq!(result.skipped_contradiction, 1);
        assert_eq!(result.skipped_dedup, 0);
        assert_eq!(destination.read_all().expect("read").len(), 1);
    }

    #[test]
    fn import_default_discount_is_point_eight_and_legacy_requires_opt_in() {
        assert_eq!(ImportOptions::default().confidence_discount, 0.8);

        let tmp = TempDir::new().expect("tempdir");
        let raw = tmp.path().join("legacy.jsonl");
        let source = entry(
            KnowledgeKind::Insight,
            "legacy",
            "trusted legacy entry",
            &["legacy"],
            0.5,
            &[],
            Utc::now(),
        );
        std::fs::write(
            &raw,
            format!("{}\n", serde_json::to_string(&source).unwrap()),
        )
        .expect("write legacy");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        assert!(destination.import(&raw, &ImportOptions::default()).is_err());
        let result = destination
            .import(
                &raw,
                &ImportOptions {
                    allow_legacy: true,
                    ..Default::default()
                },
            )
            .expect("explicit legacy import");
        assert!(result.legacy_input);
        assert_eq!(result.imported, 1);
        let imported = destination.read_all().expect("read");
        assert!((imported[0].confidence - 0.4).abs() < f64::EPSILON);
    }

    #[test]
    fn export_and_import_reject_the_live_store_as_their_transfer_path() {
        let tmp = TempDir::new().expect("tempdir");
        let empty_store = KnowledgeStore::new(tmp.path().join("empty").join("knowledge.jsonl"));
        let empty_error = empty_store
            .export(empty_store.path(), &ExportFilter::default())
            .expect_err("an absent self-export path must still fail");
        assert!(format!("{empty_error:#}").contains("live store"));
        assert!(
            !empty_store.path().exists(),
            "failed empty-store self-export must not create a backup header"
        );

        let store = KnowledgeStore::new(tmp.path().join("knowledge.jsonl"));
        store
            .add(entry(
                KnowledgeKind::Insight,
                "live",
                "live knowledge must remain a store",
                &["safety"],
                0.9,
                &[],
                Utc::now(),
            ))
            .expect("seed live store");
        let before = std::fs::read(store.path()).expect("read live bytes");

        let export_error = store
            .export(store.path(), &ExportFilter::default())
            .expect_err("self-export must fail");
        assert!(format!("{export_error:#}").contains("live store"));
        let import_error = store
            .import(
                store.path(),
                &ImportOptions {
                    allow_legacy: true,
                    ..Default::default()
                },
            )
            .expect_err("self-import must fail");
        assert!(format!("{import_error:#}").contains("live store"));
        assert_eq!(std::fs::read(store.path()).expect("read after"), before);
    }

    #[test]
    fn export_failure_preserves_an_existing_destination() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("knowledge.jsonl"));
        std::fs::write(store.path(), b"not-json\n").expect("write corrupt source store");
        let output = tmp.path().join("existing-export.jsonl");
        std::fs::write(&output, b"previous valid artifact\n").expect("write prior export");

        let error = store
            .export(&output, &ExportFilter::default())
            .expect_err("corrupt source must fail export");
        assert!(format!("{error:#}").contains("decode knowledge line 1"));
        assert_eq!(
            std::fs::read(&output).expect("read preserved export"),
            b"previous valid artifact\n"
        );
    }

    #[test]
    fn import_rejects_a_corrupt_existing_store_without_losing_raw_records() {
        let tmp = TempDir::new().expect("tempdir");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        let existing = entry(
            KnowledgeKind::Insight,
            "existing",
            "valid existing record",
            &["existing"],
            0.9,
            &[],
            Utc::now(),
        );
        let mut live_bytes = serde_json::to_vec(&existing).expect("serialize existing");
        live_bytes.extend_from_slice(b"\nnot-json\n");
        std::fs::write(destination.path(), &live_bytes).expect("write corrupt live store");

        let backup = tmp.path().join("source.jsonl");
        write_canonical_test_backup(
            &backup,
            &[entry(
                KnowledgeKind::Heuristic,
                "source",
                "valid source record",
                &["source"],
                0.8,
                &[],
                Utc::now(),
            )],
        );
        let error = destination
            .import(&backup, &ImportOptions::default())
            .expect_err("corrupt destination must fail closed");
        assert!(format!("{error:#}").contains("decode knowledge line 2"));
        assert_eq!(
            std::fs::read(destination.path()).expect("read preserved live store"),
            live_bytes
        );
    }

    #[test]
    fn imported_high_confidence_antiknowledge_blocks_regardless_of_record_order_or_decay() {
        let tmp = TempDir::new().expect("tempdir");
        let anti = entry(
            KnowledgeKind::AntiKnowledge,
            "anti",
            "never retry an irreversible payment",
            &["payments", "retry"],
            0.95,
            &[],
            Utc::now(),
        );
        let claim = entry(
            KnowledgeKind::Insight,
            "claim",
            "never retry an irreversible payment",
            &["payments", "retry"],
            0.95,
            &[],
            Utc::now(),
        );

        for (label, source) in [
            ("anti-first", vec![anti.clone(), claim.clone()]),
            ("anti-last", vec![claim.clone(), anti.clone()]),
        ] {
            let backup = tmp.path().join(format!("{label}.jsonl"));
            write_canonical_test_backup(&backup, &source);
            let destination = KnowledgeStore::new(tmp.path().join(format!("{label}-dest.jsonl")));
            let result = destination
                .import(&backup, &ImportOptions::default())
                .expect("import guarded bundle");
            assert_eq!(result.imported, 1, "{label}");
            assert_eq!(result.skipped_contradiction, 1, "{label}");
            let restored = destination.read_all().expect("read destination");
            assert_eq!(restored.len(), 1, "{label}");
            assert_eq!(restored[0].kind, KnowledgeKind::AntiKnowledge, "{label}");
            assert!(restored[0].confidence < 0.8, "default decay must apply");
        }
    }

    #[test]
    fn imported_antiknowledge_is_not_deduplicated_against_ordinary_knowledge() {
        let tmp = TempDir::new().expect("tempdir");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        destination
            .add(entry(
                KnowledgeKind::Insight,
                "claim",
                "retrying an irreversible payment is unsafe",
                &["payments"],
                0.9,
                &[],
                Utc::now(),
            ))
            .expect("seed ordinary knowledge");
        let backup = tmp.path().join("anti.jsonl");
        write_canonical_test_backup(
            &backup,
            &[entry(
                KnowledgeKind::AntiKnowledge,
                "anti",
                "retrying an irreversible payment is unsafe",
                &["payments"],
                0.95,
                &[],
                Utc::now(),
            )],
        );

        let result = destination
            .import(&backup, &ImportOptions::default())
            .expect("import AntiKnowledge");
        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped_dedup, 0);
        assert!(
            destination
                .read_all()
                .expect("read destination")
                .iter()
                .any(|entry| entry.kind == KnowledgeKind::AntiKnowledge)
        );
    }

    fn e24_store() -> (TempDir, KnowledgeStore) {
        let temp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(temp.path().join("knowledge.jsonl"));
        (temp, store)
    }

    fn e24_entry(id: &str) -> KnowledgeEntry {
        let mut value = entry(
            KnowledgeKind::AntiKnowledge,
            id,
            "Avoid repeating an observed verification failure",
            &["memory", "verification"],
            0.8,
            &["episode-1"],
            Utc::now(),
        );
        value.tier = KnowledgeTier::Transient;
        value
    }

    #[test]
    fn e24_demurrage_reduces_balance_and_halves_half_life_once() {
        let (_temp, store) = e24_store();
        let mut value = e24_entry("demurrage");
        value.balance = 0.004;
        value.half_life_days = 20.0;
        store.add(value).expect("add");

        assert_eq!(store.demurrage(0.005).expect("demurrage"), 1);
        let after = store.read_all().expect("read").remove(0);
        assert!((after.balance + 0.001).abs() < 1e-9);
        assert_eq!(after.half_life_days, 10.0);
        assert!(after.balance_depleted_at.is_some());

        store.demurrage(0.005).expect("second demurrage");
        let after = store.read_all().expect("read").remove(0);
        assert_eq!(
            after.half_life_days, 10.0,
            "half-life changes only at crossing"
        );
    }

    #[test]
    fn e24_demurrage_freezes_after_seven_depleted_days() {
        let (_temp, store) = e24_store();
        let mut value = e24_entry("freeze-after-grace");
        value.balance = 0.0;
        value.balance_depleted_at = Some(Utc::now() - Duration::days(8));
        store.add(value).expect("add");

        store.demurrage(0.005).expect("demurrage");
        let after = store.read_all().expect("read").remove(0);
        assert!(after.frozen);
        assert!(after.frozen_at.is_some());
    }

    #[test]
    fn e24_reinforce_uses_exact_signal_amounts() {
        let (_temp, store) = e24_store();
        let mut value = e24_entry("reinforcement");
        value.balance = 0.0;
        value.balance_depleted_at = Some(Utc::now());
        store.add(value).expect("add");
        let signals = [
            (crate::ReinforcementSignal::Retrieved, 0.05),
            (crate::ReinforcementSignal::Cited, 0.10),
            (crate::ReinforcementSignal::Gated, 0.15),
            (crate::ReinforcementSignal::Surprised, 0.08),
            (crate::ReinforcementSignal::AgentQuoted, 0.12),
        ];
        let mut expected = 0.0;
        for (signal, amount) in signals {
            store.reinforce("reinforcement", signal).expect("reinforce");
            expected += amount;
            let actual = store.read_all().expect("read")[0].balance;
            assert!((actual - expected).abs() < 1e-9);
        }
        assert!(
            store.read_all().expect("read")[0]
                .balance_depleted_at
                .is_none()
        );
    }

    #[test]
    fn e24_falsifier_survives_immunizes_and_discredits() {
        let (_temp, store) = e24_store();
        let mut survivor = e24_entry("survivor");
        survivor.falsifier = Some(Falsifier {
            predicate: "retries remain bounded".to_string(),
            observations: 0,
            violations: 0,
            last_checked: Utc::now(),
            active: true,
        });
        let mut discredited = e24_entry("discredited");
        discredited.content = "Avoid a separately falsified retry pattern".to_string();
        discredited.tags.push("separate".to_string());
        discredited.falsifier = survivor.falsifier.clone();
        store.ingest(vec![survivor, discredited]).expect("ingest");

        assert_eq!(
            store.check_falsifier("survivor", false).expect("check"),
            FalsifierOutcome::Survived
        );
        store.check_falsifier("survivor", false).expect("check");
        assert_eq!(
            store.check_falsifier("survivor", false).expect("check"),
            FalsifierOutcome::Immunized
        );
        assert_eq!(
            store.check_falsifier("discredited", true).expect("check"),
            FalsifierOutcome::Discredited
        );
        let entries = store.read_all().expect("read");
        let survivor = entries.iter().find(|entry| entry.id == "survivor").unwrap();
        assert_eq!(survivor.tier, KnowledgeTier::Consolidated);
        assert!(survivor.confidence >= 0.9);
        let discredited = entries
            .iter()
            .find(|entry| entry.id == "discredited")
            .unwrap();
        assert_eq!(discredited.confidence, 0.4);
        assert!(!discredited.falsifier.as_ref().unwrap().active);
    }

    #[test]
    fn e24_tier_progression_promotes_and_demotes() {
        let (_temp, store) = e24_store();
        let mut promote = e24_entry("promote");
        promote.kind = KnowledgeKind::Insight;
        promote.content = "Independent confirmations support bounded retries".to_string();
        promote.tags.push("promotion".to_string());
        promote.confirmation_count = 2;
        promote.confidence = 0.6;
        let mut demote = e24_entry("demote");
        demote.content = "A fragile working rule should leave working memory".to_string();
        demote.tags.push("demotion".to_string());
        demote.tier = KnowledgeTier::Working;
        demote.confidence = 0.1;
        store.ingest(vec![promote, demote]).expect("ingest");

        let report = store
            .apply_tier_progression(&crate::TierProgressionConfig::default())
            .expect("progression");
        assert!(
            report
                .promoted
                .contains(&("promote".to_string(), KnowledgeTier::Working))
        );
        assert!(
            report
                .demoted
                .contains(&("demote".to_string(), KnowledgeTier::Transient))
        );
    }

    #[test]
    fn e24_temporal_index_tracks_add_query_relation_and_gc() {
        let (temp, mut store) = e24_store();
        store.enable_temporal_index().expect("enable");
        let now = Utc::now();
        let mut epoch = KnowledgeEpoch::at(7, "test", now - Duration::seconds(1));
        epoch.close(now + Duration::seconds(1));
        assert!(store.add_temporal_epoch(epoch));
        let mut first = e24_entry("temporal-a");
        first.created_at = now - Duration::milliseconds(20);
        first.kind = KnowledgeKind::Insight;
        first.content = "Temporal entry alpha has unique context".to_string();
        first.tags.push("alpha".to_string());
        let mut second = e24_entry("temporal-b");
        second.kind = KnowledgeKind::Insight;
        second.content = "Temporal entry beta has distinct evidence".to_string();
        second.tags.push("beta".to_string());
        second.created_at = now;
        store.ingest(vec![first, second]).expect("ingest");
        assert_eq!(store.query_temporal(7).expect("query").len(), 2);
        assert!(
            store
                .query_temporal_relation("temporal-a", "temporal-b")
                .expect("relation")
                .is_some()
        );
        store
            .update_confidence("temporal-a", -1.0)
            .expect("confidence update");
        store.gc(0.5).expect("gc");
        assert!(
            store
                .query_temporal_relation("temporal-a", "temporal-b")
                .expect("relation")
                .is_none()
        );
        drop(temp);
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn e24_query_hdc_is_similarity_sorted() {
        let (_temp, store) = e24_store();
        let query = HdcVector::from_seed(b"e24-exact");
        let mut exact = e24_entry("hdc-exact");
        exact.content = "Exact HDC query target".to_string();
        exact.tags.push("exact".to_string());
        exact.hdc_vector = Some(query.to_bytes().to_vec());
        let mut different = e24_entry("hdc-different");
        different.content = "Different HDC comparison target".to_string();
        different.tags.push("different".to_string());
        different.hdc_vector = Some(HdcVector::from_seed(b"e24-different").to_bytes().to_vec());
        store.ingest(vec![different, exact]).expect("ingest");

        let hits = store.query_hdc(&query, 2).expect("query hdc");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].entry.id, "hdc-exact");
        assert!(hits[0].total_score >= hits[1].total_score);
    }
}
