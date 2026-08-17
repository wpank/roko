//! Delta-speed Dreams cross-cut and dream-cycle output publication.

use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use roko_daimon::{AffectEngine, DaimonState};
use roko_dreams::{DreamCycleReport, DreamRoutingAdvice, dream_advice_to_routing_bias};
use roko_learn::cascade_router::{CascadeModel, CascadeRouter};
use roko_learn::model_router::RoutingContext;
use roko_neuro::KnowledgeStore;

use crate::ComposeError;
use crate::cross_cut::{CrossCutContext, CrossCutFunctor, CrossCutResult};
use crate::natural_transforms::{eta_DM, eta_DN};

/// Per-tick Dreams functor. Dream work itself runs only at delta speed.
#[derive(Debug, Default, Clone, Copy)]
pub struct DreamsFunctor;

#[async_trait]
impl CrossCutFunctor for DreamsFunctor {
    fn name(&self) -> &str {
        "dreams"
    }

    async fn pre_enrich(
        &self,
        input: Vec<roko_core::Signal>,
        _ctx: &CrossCutContext,
    ) -> CrossCutResult<Vec<roko_core::Signal>> {
        Ok(input)
    }

    async fn post_enrich(
        &self,
        output: Vec<roko_core::Signal>,
        _ctx: &CrossCutContext,
    ) -> CrossCutResult<Vec<roko_core::Signal>> {
        Ok(output)
    }

    fn should_short_circuit(&self) -> bool {
        true
    }
}

/// Observable result of publishing one completed dream cycle.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DreamConsumptionReport {
    /// Entries offered to the real durable knowledge store.
    pub knowledge_entries: usize,
    /// Whether the live Daimon received dream affect and depotentiation.
    pub affect_updated: bool,
    /// Whether routing advice was published for future CascadeRouter calls.
    pub routing_advice_published: bool,
}

/// Publishes existing [`DreamCycleReport`] output into live cross-cut stores.
pub struct DreamOutputConsumer {
    knowledge_store: Arc<KnowledgeStore>,
    daimon: Arc<RwLock<DaimonState>>,
    cascade_router: Arc<CascadeRouter>,
    latest_routing_advice: RwLock<Option<DreamRoutingAdvice>>,
}

impl DreamOutputConsumer {
    /// Bind dream output to the live KnowledgeStore, Daimon, and CascadeRouter.
    #[must_use]
    pub fn new(
        knowledge_store: Arc<KnowledgeStore>,
        daimon: Arc<RwLock<DaimonState>>,
        cascade_router: Arc<CascadeRouter>,
    ) -> Self {
        Self {
            knowledge_store,
            daimon,
            cascade_router,
            latest_routing_advice: RwLock::new(None),
        }
    }

    /// Consume a report returned by `DreamRunner`/`DreamEngine`.
    pub fn consume(
        &self,
        report: &DreamCycleReport,
        routing_advice: Option<&DreamRoutingAdvice>,
    ) -> CrossCutResult<DreamConsumptionReport> {
        let knowledge = eta_DM(report);
        self.knowledge_store
            .ingest(knowledge.clone())
            .map_err(|error| ComposeError::Other(format!("dream -> memory: {error}")))?;

        let affect = eta_DN(report);
        let mut daimon = self
            .daimon
            .write()
            .map_err(|_| ComposeError::Other("daimon state lock poisoned".into()))?;
        daimon.appraise(affect.event);
        if affect.depotentiate {
            daimon.apply_dream_depotentiation();
        }
        drop(daimon);

        if let Some(advice) = routing_advice {
            *self
                .latest_routing_advice
                .write()
                .map_err(|_| ComposeError::Other("dream routing lock poisoned".into()))? =
                Some(advice.clone());
        }
        Ok(DreamConsumptionReport {
            knowledge_entries: knowledge.len(),
            affect_updated: true,
            routing_advice_published: routing_advice.is_some(),
        })
    }

    /// Route using the latest dream advice through the real CascadeRouter.
    pub fn route_with_published_advice(
        &self,
        context: &RoutingContext,
        task_category: &str,
        complexity_band: &str,
    ) -> CrossCutResult<CascadeModel> {
        let advice = self
            .latest_routing_advice
            .read()
            .map_err(|_| ComposeError::Other("dream routing lock poisoned".into()))?;
        Ok(advice.as_ref().map_or_else(
            || self.cascade_router.route(context),
            |advice| {
                let bias = dream_advice_to_routing_bias(advice, task_category, complexity_band);
                self.cascade_router.route_with_bias(context, &bias)
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use roko_dreams::RoutingRecommendation;
    use roko_neuro::tier_progression::{PlaybookCompilation, TierProgressionReport};
    use roko_neuro::{KnowledgeEntry, KnowledgeKind};

    use super::*;

    fn report(entry: KnowledgeEntry) -> DreamCycleReport {
        DreamCycleReport {
            started_at: Utc::now(),
            completed_at: Utc::now(),
            total_episodes: 1,
            processed_episodes: 1,
            processed_through: Some(Utc::now()),
            analysis: TierProgressionReport {
                insights: Vec::new(),
                heuristics: Vec::new(),
                playbook: PlaybookCompilation {
                    markdown: String::new(),
                    rules: Vec::new(),
                },
                falsifiers: Vec::new(),
            },
            cfactor_regression: None,
            clusters: Vec::new(),
            cross_episode_report: None,
            routing_recommendations: 1,
            knowledge_entries_written: 1,
            playbooks_created: 0,
            regressions_detected: Vec::new(),
            strategy_hypotheses: vec![entry],
            performance_notes: Vec::new(),
            hypnagogia_entries_count: 0,
            staging_buffer_stats: None,
            intensive_mode_active: false,
            phase_budget_summary: None,
        }
    }

    #[tokio::test]
    async fn dreams_is_a_strict_per_tick_passthrough() {
        let signal = roko_core::Signal::builder(roko_core::Kind::Task).build();
        let result = DreamsFunctor
            .pre_enrich(vec![signal.clone()], &CrossCutContext::default())
            .await
            .unwrap();
        assert!(DreamsFunctor.should_short_circuit());
        assert_eq!(result, vec![signal]);
    }

    #[test]
    fn output_consumer_publishes_to_all_three_live_targets() {
        let temp = tempfile::tempdir().unwrap();
        let store = Arc::new(KnowledgeStore::new(temp.path().join("knowledge.jsonl")));
        let mut state = DaimonState::new();
        state.state.pad.arousal = 0.8;
        let daimon = Arc::new(RwLock::new(state));
        let router = Arc::new(CascadeRouter::new(vec!["model-a".into(), "model-b".into()]));
        let consumer = DreamOutputConsumer::new(store.clone(), daimon.clone(), router);
        let entry = KnowledgeEntry {
            id: "dream-heuristic".into(),
            kind: KnowledgeKind::AntiKnowledge,
            content: "this failed approach should not be repeated".into(),
            confidence: 0.9,
            ..KnowledgeEntry::default()
        };
        let advice = DreamRoutingAdvice {
            source_dream_report: "dream-report.json".into(),
            recommendations: vec![RoutingRecommendation {
                task_category: "implementation".into(),
                complexity_band: "standard".into(),
                recommended_model: "model-a".into(),
                deprioritize: vec!["model-b".into()],
                confidence: 0.9,
                supporting_episodes: 5,
                recommended_model_success_rate: 0.9,
                pattern_signature: 7,
            }],
            ..DreamRoutingAdvice::default()
        };

        let consumed = consumer.consume(&report(entry), Some(&advice)).unwrap();
        let routed = consumer
            .route_with_published_advice(&RoutingContext::default(), "implementation", "standard")
            .unwrap();

        assert_eq!(consumed.knowledge_entries, 1);
        assert!(consumed.affect_updated);
        assert!(consumed.routing_advice_published);
        assert_eq!(store.read_all().unwrap()[0].id, "dream-heuristic");
        assert!(daimon.read().unwrap().state.pad.arousal.abs() < 0.8);
        assert_eq!(routed.primary.slug, "model-a");
    }
}
