//! Durable-memory cross-cut backed by [`roko_neuro::KnowledgeStore`].

#[cfg(feature = "hdc")]
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use roko_core::{Body, Kind, Provenance, Signal};
use roko_neuro::{KnowledgeEntry, KnowledgeStore, KnowledgeTier, ReinforcementSignal};

use crate::cross_cut::{CrossCutContext, CrossCutFunctor, CrossCutResult, LoopStep};
use crate::{AttentionBidder, ComposeError};

const DEFAULT_MAX_ENTRIES: usize = 10;
const GATE_FAILURE_ACCURACY: f64 = 0.0;

#[derive(Clone)]
struct MemoryMatch {
    entry: KnowledgeEntry,
    score: f64,
    retrieval: &'static str,
}

/// Memory enrichment and gate-feedback adapter.
pub struct MemoryFunctor {
    store: Arc<KnowledgeStore>,
    max_entries: usize,
    included: Mutex<HashMap<(String, String), Vec<String>>>,
    last_query_empty: AtomicBool,
}

impl MemoryFunctor {
    /// Wrap an existing durable store without taking ownership of it.
    #[must_use]
    pub fn new(store: Arc<KnowledgeStore>) -> Self {
        Self::with_max_entries(store, DEFAULT_MAX_ENTRIES)
    }

    /// Configure the hard retrieval cap.
    #[must_use]
    pub fn with_max_entries(store: Arc<KnowledgeStore>, max_entries: usize) -> Self {
        Self {
            store,
            max_entries: max_entries.max(1),
            included: Mutex::new(HashMap::new()),
            last_query_empty: AtomicBool::new(false),
        }
    }

    /// Borrow the shared durable store.
    #[must_use]
    pub fn store(&self) -> &Arc<KnowledgeStore> {
        &self.store
    }

    fn query(&self, topic: &str) -> CrossCutResult<Vec<MemoryMatch>> {
        #[cfg(not(feature = "hdc"))]
        {
            self.store
                .query_hits(topic, self.max_entries)
                .map(|hits| {
                    hits.into_iter()
                        .map(|hit| MemoryMatch {
                            entry: hit.entry,
                            score: hit.total_score,
                            retrieval: "keyword",
                        })
                        .collect()
                })
                .map_err(compose_error)
        }

        #[cfg(feature = "hdc")]
        {
            use roko_primitives::hdc::text_fingerprint;

            let mut matches = BTreeMap::<String, MemoryMatch>::new();
            for hit in self
                .store
                .query_hits(topic, self.max_entries)
                .map_err(compose_error)?
            {
                matches.insert(
                    hit.entry.id.clone(),
                    MemoryMatch {
                        entry: hit.entry,
                        score: hit.total_score,
                        retrieval: "keyword",
                    },
                );
            }
            let fingerprint = text_fingerprint(topic).to_bytes();
            for hit in self
                .store
                .query_similar(&fingerprint, self.max_entries)
                .map_err(compose_error)?
            {
                matches
                    .entry(hit.entry.id.clone())
                    .and_modify(|current| {
                        if f64::from(hit.similarity) > current.score {
                            current.score = f64::from(hit.similarity);
                            current.retrieval = "hdc";
                        }
                    })
                    .or_insert(MemoryMatch {
                        entry: hit.entry,
                        score: f64::from(hit.similarity),
                        retrieval: "hdc",
                    });
            }
            let mut matches = matches.into_values().collect::<Vec<_>>();
            matches.sort_by(|left, right| right.score.total_cmp(&left.score));
            matches.truncate(self.max_entries);
            Ok(matches)
        }
    }

    fn remember_included(
        &self,
        ctx: &CrossCutContext,
        matches: &[MemoryMatch],
    ) -> CrossCutResult<()> {
        self.included
            .lock()
            .map_err(|_| ComposeError::Other("memory context lock poisoned".into()))?
            .insert(
                (ctx.plan_id.clone(), ctx.task_id.clone()),
                matches.iter().map(|hit| hit.entry.id.clone()).collect(),
            );
        Ok(())
    }

    fn apply_gate_feedback(&self, ctx: &CrossCutContext, passed: bool) -> CrossCutResult<()> {
        let ids = self
            .included
            .lock()
            .map_err(|_| ComposeError::Other("memory context lock poisoned".into()))?
            .remove(&(ctx.plan_id.clone(), ctx.task_id.clone()))
            .unwrap_or_default();
        let refs = ids.iter().map(String::as_str).collect::<Vec<_>>();
        if passed {
            self.store
                .reinforce_batch(&refs, ReinforcementSignal::Gated, 0.0)
                .map_err(compose_error)?;
        } else {
            self.store
                .score_prediction_utility(&ids, false, GATE_FAILURE_ACCURACY)
                .map_err(compose_error)?;
            let mut store = self.store.as_ref().clone();
            let outcomes = ids.into_iter().map(|id| (id, false)).collect::<Vec<_>>();
            store.batch_record_usage(&outcomes).map_err(compose_error)?;
        }
        Ok(())
    }
}

#[async_trait]
impl CrossCutFunctor for MemoryFunctor {
    fn name(&self) -> &str {
        "memory"
    }

    async fn pre_enrich(
        &self,
        mut input: Vec<Signal>,
        ctx: &CrossCutContext,
    ) -> CrossCutResult<Vec<Signal>> {
        if !matches!(ctx.step, LoopStep::Sense | LoopStep::Compose) {
            return Ok(input);
        }
        let topic = task_topic(&input, ctx);
        let matches = self.query(&topic)?;
        self.last_query_empty
            .store(matches.is_empty(), Ordering::Release);
        self.remember_included(ctx, &matches)?;
        let compose = ctx.step == LoopStep::Compose;
        input.extend(matches.into_iter().map(|hit| memory_signal(hit, compose)));
        Ok(input)
    }

    async fn post_enrich(
        &self,
        output: Vec<Signal>,
        ctx: &CrossCutContext,
    ) -> CrossCutResult<Vec<Signal>> {
        if ctx.step == LoopStep::React
            && let Some(passed) = gate_verdict(&output)
        {
            self.apply_gate_feedback(ctx, passed)?;
        }
        Ok(output)
    }

    fn should_short_circuit(&self) -> bool {
        self.last_query_empty.load(Ordering::Acquire)
            || self
                .store
                .read_all()
                .map(|entries| entries.is_empty())
                .unwrap_or(true)
    }
}

fn memory_signal(hit: MemoryMatch, compose: bool) -> Signal {
    let tier = tier_label(hit.entry.tier);
    let mut builder = Signal::builder(Kind::Insight)
        .body(Body::Json(serde_json::json!({
            "knowledge_id": hit.entry.id,
            "content": hit.entry.content,
            "source": hit.entry.source,
            "tier": tier,
            "retrieval": hit.retrieval,
            "relevance": hit.score,
            "demurrage_balance": hit.entry.balance,
        })))
        .provenance(
            Provenance::agent("memory")
                .with_trust_origin(hit.entry.origin_taint)
                .with_taint_level(hit.entry.classification),
        )
        .tag("cross_cut", "memory")
        .tag("knowledge_id", hit.entry.id.clone())
        .tag("knowledge_tier", tier)
        .tag("retrieval", hit.retrieval);
    if compose {
        builder = builder
            .tag(
                "attention_bidder",
                format!("{:?}", AttentionBidder::Neuro).to_lowercase(),
            )
            .tag("recommendation_source", "memory")
            .tag("decision_kind", "compose")
            .tag("decision_key", "prompt_context")
            .tag("priority_level", "2")
            .tag(
                "recommendation_confidence",
                hit.entry.confidence.to_string(),
            )
            .tag("recommendation_value", hit.entry.id);
    }
    builder.build()
}

fn task_topic(input: &[Signal], ctx: &CrossCutContext) -> String {
    let mut parts = input
        .iter()
        .filter(|signal| signal.kind == Kind::Task)
        .filter_map(|signal| match &signal.body {
            Body::Text(text) => Some(text.clone()),
            Body::Json(value) => Some(value.to_string()),
            Body::Empty | Body::Bytes(_) => None,
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        parts.push(ctx.task_id.clone());
    }
    parts.join(" ")
}

fn gate_verdict(signals: &[Signal]) -> Option<bool> {
    signals.iter().rev().find_map(|signal| {
        if signal.kind != Kind::GateVerdict {
            return None;
        }
        if let Some(verdict) = signal.tag("verdict") {
            return verdict_label(verdict);
        }
        let Body::Json(value) = &signal.body else {
            return None;
        };
        value
            .get("passed")
            .and_then(serde_json::Value::as_bool)
            .or_else(|| {
                value
                    .get("verdict")
                    .or_else(|| value.get("status"))
                    .and_then(serde_json::Value::as_str)
                    .and_then(verdict_label)
            })
    })
}

fn verdict_label(label: &str) -> Option<bool> {
    match label.to_ascii_lowercase().as_str() {
        "pass" | "passed" | "success" => Some(true),
        "fail" | "failed" | "failure" => Some(false),
        _ => None,
    }
}

const fn tier_label(tier: KnowledgeTier) -> &'static str {
    match tier {
        KnowledgeTier::Transient => "transient",
        KnowledgeTier::Working => "working",
        KnowledgeTier::Consolidated => "consolidated",
        KnowledgeTier::Persistent => "persistent",
    }
}

fn compose_error(error: impl std::fmt::Display) -> ComposeError {
    ComposeError::Other(format!("memory cross-cut: {error}"))
}

#[cfg(test)]
mod tests {
    use roko_neuro::KnowledgeKind;

    use super::*;

    fn entry(id: &str, balance: f64) -> KnowledgeEntry {
        KnowledgeEntry {
            id: id.into(),
            kind: KnowledgeKind::Insight,
            source: Some("test-fixture".into()),
            content: "Rust async cancellation requires structured cleanup".into(),
            confidence: 0.9,
            tags: vec!["rust".into(), "async".into(), "cancellation".into()],
            tier: KnowledgeTier::Consolidated,
            balance,
            ..KnowledgeEntry::default()
        }
    }

    fn task() -> Signal {
        Signal::builder(Kind::Task)
            .body(Body::text("implement Rust async cancellation cleanup"))
            .build()
    }

    fn verdict(passed: bool) -> Signal {
        Signal::builder(Kind::GateVerdict)
            .body(Body::Json(serde_json::json!({"passed": passed})))
            .build()
    }

    #[tokio::test]
    async fn sense_and_compose_query_real_store_with_metadata_and_neuro_bid() {
        let temp = tempfile::tempdir().unwrap();
        let store = Arc::new(KnowledgeStore::new(temp.path().join("knowledge.jsonl")));
        store.add(entry("k-memory", 0.3)).unwrap();
        let memory = MemoryFunctor::with_max_entries(store, 1);
        let mut ctx = CrossCutContext {
            step: LoopStep::Sense,
            plan_id: "p".into(),
            task_id: "t".into(),
            agent_role: "implementer".into(),
        };

        let sensed = memory.pre_enrich(vec![task()], &ctx).await.unwrap();
        assert_eq!(sensed.len(), 2);
        assert_eq!(sensed[1].tag("knowledge_id"), Some("k-memory"));
        assert_eq!(sensed[1].tag("knowledge_tier"), Some("consolidated"));

        ctx.step = LoopStep::Compose;
        let composed = memory.pre_enrich(vec![task()], &ctx).await.unwrap();
        assert_eq!(composed.len(), 2);
        assert_eq!(composed[1].tag("attention_bidder"), Some("neuro"));
        assert_eq!(composed[1].tag("decision_kind"), Some("compose"));
        assert!(!memory.should_short_circuit());
    }

    #[tokio::test]
    async fn react_reinforces_passes_and_weakens_failures_in_real_store() {
        let temp = tempfile::tempdir().unwrap();
        let store = Arc::new(KnowledgeStore::new(temp.path().join("knowledge.jsonl")));
        store.add(entry("k-feedback", 0.5)).unwrap();
        let memory = MemoryFunctor::new(store.clone());
        let mut ctx = CrossCutContext {
            step: LoopStep::Sense,
            plan_id: "p".into(),
            task_id: "pass".into(),
            agent_role: "implementer".into(),
        };
        memory.pre_enrich(vec![task()], &ctx).await.unwrap();
        let before_pass = store.read_all().unwrap()[0].balance;
        ctx.step = LoopStep::React;
        memory.post_enrich(vec![verdict(true)], &ctx).await.unwrap();
        let after_pass = store.read_all().unwrap()[0].balance;
        assert!(after_pass > before_pass);

        ctx.step = LoopStep::Sense;
        ctx.task_id = "fail".into();
        memory.pre_enrich(vec![task()], &ctx).await.unwrap();
        let before_fail = store.read_all().unwrap()[0].clone();
        ctx.step = LoopStep::React;
        memory
            .post_enrich(vec![verdict(false)], &ctx)
            .await
            .unwrap();
        let after_fail = store.read_all().unwrap()[0].clone();
        assert!(after_fail.balance < before_fail.balance);
        assert!(after_fail.confidence < before_fail.confidence);
    }

    #[tokio::test]
    async fn empty_query_sets_short_circuit_hint_without_dropping_input() {
        let temp = tempfile::tempdir().unwrap();
        let store = Arc::new(KnowledgeStore::new(temp.path().join("knowledge.jsonl")));
        store.add(entry("k-unrelated", 0.5)).unwrap();
        let memory = MemoryFunctor::new(store);
        let ctx = CrossCutContext {
            step: LoopStep::Sense,
            task_id: "quantum-biology-unrelated".into(),
            ..CrossCutContext::default()
        };
        let input = Signal::builder(Kind::Task)
            .body(Body::text("quantum biology zebrafish"))
            .build();

        let output = memory.pre_enrich(vec![input.clone()], &ctx).await.unwrap();

        assert_eq!(output, vec![input]);
        assert!(memory.should_short_circuit());
    }
}
