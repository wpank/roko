//! Routing observation sink — feeds task / turn outcomes back into the
//! [`CascadeRouter`] so model selection learns from real performance.
//!
//! ## Why this exists
//!
//! The router exposes `record_confidence_outcome(model_slug, success)` for
//! confidence-only updates and `record_override_outcome(...)` for contextual
//! override learning. Until this sink existed those methods were called from
//! ad-hoc helpers in the runner, leading to double-counting and missed
//! observations. Now there is one path:
//! `FeedbackEvent::TaskCompleted -> RoutingObservationSink::record(...)`.
//!
//! ## Quality evidence only
//!
//! The router learns only from the settled attempt's learning label
//! ([`FeedbackEvent::learning_success`], S01 §4.1): a gate pass is a
//! success, and a failure of the agent's work (a gate failure, a turn-cap
//! stop, a timeout after output) is a failure. Unverified, force-accepted,
//! provider-failed and harness-failed attempts are not quality evidence and
//! leave the router alone; the failover health registry tracks provider
//! health.
//!
//! ## Override handling
//!
//! When [`ModelChoiceSource::Override`] tagged a task, the sink records
//! it via `record_override_outcome` so manual operator overrides do not
//! pollute the bandit signal that drives router decisions on
//! non-overridden tasks. A `[routing.ladder]` rung
//! ([`ModelChoiceSource::Ladder`]) is recorded like the router's own pick,
//! so the learner sees every rung.

use std::sync::Arc;

use async_trait::async_trait;
use roko_core::agent::AgentRole;
use roko_core::config::RewardWeights;
use roko_core::task::{TaskCategory, TaskComplexityBand};
use roko_core::{BehavioralState, DaimonPolicy};
use roko_learn::cascade_router::CascadeRouter;
use roko_learn::model_router::RoutingContext;

use super::{FeedbackEvent, FeedbackSink};
use crate::dispatch::ModelChoiceSource;
use crate::runner::conductor_adapter::compute_conductor_load;

/// Sink that records a routing observation per `task_completed` event.
#[derive(Clone)]
pub struct RoutingObservationSink {
    router: Arc<CascadeRouter>,
}

impl RoutingObservationSink {
    /// Construct a routing sink wrapping a shared router.
    #[must_use]
    pub fn new(router: Arc<CascadeRouter>) -> Self {
        Self { router }
    }
}

impl std::fmt::Debug for RoutingObservationSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutingObservationSink")
            .field("router", &"..")
            .finish()
    }
}

#[async_trait]
impl FeedbackSink for RoutingObservationSink {
    fn name(&self) -> &'static str {
        "routing"
    }

    /// Only a completed attempt with a learning label; the facade counts
    /// the others as skipped.
    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(event, FeedbackEvent::TaskCompleted { .. }) && event.learning_success().is_some()
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        let FeedbackEvent::TaskCompleted {
            outcome,
            model_source,
            routing_context,
            ..
        } = event
        else {
            return Ok(());
        };
        // Quality evidence only: an attempt without a learning label
        // updates no counter, override or bandit.
        let Some(succeeded) = event.learning_success() else {
            return Ok(());
        };

        let ctx = match routing_context {
            Some(ctx) => ctx.clone(),
            None => build_fallback_routing_context(&outcome.model),
        };

        // Audit #84: always record category-level stats (even for
        // overrides) so confidence_scores can adjust per-category.
        self.router
            .record_category_outcome(&outcome.model, ctx.task_category, succeeded);

        // Audit #90: manual overrides must not pollute the bandit signal.
        // Route them through the dampened `record_override_outcome` path
        // instead of the full router-outcome path.
        if *model_source == ModelChoiceSource::Override {
            self.router
                .record_override_outcome(&outcome.model, &ctx, succeeded, None);
            return Ok(());
        }

        observe_router_outcome(
            &self.router,
            &outcome.model,
            &ctx,
            succeeded,
            outcome.cost_usd,
            outcome.duration_ms,
        );
        Ok(())
    }
}

/// Record the outcome of a router-chosen model.
///
/// Successes and failures update the same learners (bug-8da8ba): one
/// confidence trial, a success only when `succeeded`, and one `LinUCB`
/// observation. A success earns the multi-objective reward; a failure earns
/// 0. A slug the router does not track falls back to the binary confidence
/// path, which logs and drops it.
pub(crate) fn observe_router_outcome(
    router: &CascadeRouter,
    model: &str,
    ctx: &RoutingContext,
    succeeded: bool,
    cost_usd: f64,
    duration_ms: u64,
) {
    let Some(model_idx) = router.model_index_for_slug(model) else {
        router.record_confidence_outcome(model, succeeded);
        return;
    };

    // P0-05: Feed real cost from the agent outcome into the bandit.
    // Normalize against a $1.00 per-task ceiling and latency against a
    // 5-minute ceiling so both signals stay in [0, 1] for the LinUCB
    // reward computation.
    let normalized_cost = (cost_usd / 1.0).clamp(0.0, 1.0);
    let normalized_latency = (duration_ms as f64 / 300_000.0).clamp(0.0, 1.0);
    let weights = RewardWeights::default();
    router.observe_multi_objective_outcome(
        ctx.to_features(),
        model_idx,
        /* quality */ 1.0,
        normalized_cost,
        normalized_latency,
        &weights,
        succeeded,
    );
}

/// Build a fallback [`RoutingContext`] for observations that lack
/// dispatch-time context.
///
/// Used when `routing_context` is `None` (backward compat with older
/// code paths that don't carry context through `FeedbackEvent`), and by
/// the Graph settlement routing sink, whose receipts carry no context.
pub(crate) fn build_fallback_routing_context(model: &str) -> RoutingContext {
    RoutingContext {
        task_category: TaskCategory::Implementation,
        complexity: TaskComplexityBand::Standard,
        iteration: 0,
        role: AgentRole::Implementer,
        crate_familiarity: 0.5,
        has_prior_failure: false,
        conductor_load: compute_conductor_load(0, 0, 0.0),
        active_agents: 0,
        ready_queue_depth: 0,
        max_queue_wait_hours: 0.0,
        daimon_policy: DaimonPolicy::new(0.5, BehavioralState::Engaged),
        thinking_level: None,
        temperament: None,
        previous_model: Some(model.to_string()),
        plan_context_tokens: None,
        tier_thresholds: None,
        cfactor: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::AgentOutcome;
    use crate::runtime_feedback::settled_as;
    use roko_learn::cascade_router::CascadeRouter;
    use roko_learn::telemetry::{AttemptOutcome, AttemptVerdictRecord};

    /// A completed attempt whose provider call succeeded, settled as
    /// `settled`.
    fn completed(
        verdict: Option<Arc<AttemptVerdictRecord>>,
        model_source: ModelChoiceSource,
    ) -> FeedbackEvent {
        FeedbackEvent::TaskCompleted {
            turns: 0,
            failure_reason: None,
            settled: verdict,
            plan_id: "p".into(),
            task_id: "t".into(),
            outcome: outcome(true),
            model_source,
            succeeded: true,
            routing_context: Some(test_routing_context()),
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: String::new(),
        }
    }

    /// bug-c34782 (S01 §4.1): only a gate verdict or a failure of the
    /// agent's work moves the router. Unverified, force-accepted,
    /// provider-failed and harness-failed attempts, and events without a
    /// settled record, update no counter, override or bandit, although the
    /// provider call succeeded and `succeeded` is set.
    #[tokio::test]
    async fn routing_sink_skips_attempts_without_a_learning_label() {
        use AttemptOutcome as O;
        let r = router();
        let sink = RoutingObservationSink::new(r.clone());
        let unlabelled = [
            settled_as(O::Unverified, false),
            settled_as(O::ForcedAccept, false),
            settled_as(O::ProviderError, true),
            settled_as(O::ProviderExhausted, false),
            settled_as(O::Timeout, false),
            settled_as(O::HarnessError, false),
            None,
        ];
        for source in [ModelChoiceSource::Router, ModelChoiceSource::Override] {
            for verdict in unlabelled.clone() {
                let event = completed(verdict, source);
                assert!(!sink.interested(&event));
                sink.on_event(&event).await.unwrap();
            }
        }
        assert!(r.confidence_snapshot().is_empty(), "no confidence trial");
        assert_eq!(r.total_observations(), 0, "no bandit observation");

        for (outcome, first_token_seen) in [
            (O::Passed, false),
            (O::GateFailed, false),
            (O::TurnCap, false),
            (O::Timeout, true),
        ] {
            let event = completed(
                settled_as(outcome, first_token_seen),
                ModelChoiceSource::Router,
            );
            assert!(sink.interested(&event), "{outcome:?}");
            sink.on_event(&event).await.unwrap();
        }
        assert_eq!(
            r.confidence_snapshot().get("claude-sonnet-4-6").copied(),
            Some((4, 1)),
            "one pass and three failures of the agent's work"
        );
        assert_eq!(r.total_observations(), 4);
    }

    fn outcome(success: bool) -> AgentOutcome {
        AgentOutcome {
            task_id: "t".into(),
            plan_id: "p".into(),
            model: "claude-sonnet-4-6".into(),
            provider: "claude_cli".into(),
            output: "".into(),
            tokens_in: 0,
            tokens_out: 0,
            cost_usd: 0.0,
            duration_ms: 0,
            exit_code: if success { Some(0) } else { Some(1) },
            is_error: !success,
        }
    }

    fn router() -> Arc<CascadeRouter> {
        Arc::new(CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "gpt-5".into(),
        ]))
    }

    fn test_routing_context() -> RoutingContext {
        RoutingContext {
            task_category: TaskCategory::Implementation,
            complexity: TaskComplexityBand::Complex,
            iteration: 2,
            role: AgentRole::Implementer,
            crate_familiarity: 0.8,
            has_prior_failure: true,
            conductor_load: 0.3,
            active_agents: 2,
            ready_queue_depth: 5,
            max_queue_wait_hours: 0.1,
            daimon_policy: DaimonPolicy::new(0.7, BehavioralState::Engaged),
            thinking_level: None,
            temperament: None,
            previous_model: Some("claude-sonnet-4-6".into()),
            plan_context_tokens: None,
            tier_thresholds: None,
            cfactor: None,
        }
    }

    #[tokio::test]
    async fn success_drives_observe_multi_objective_for_known_model() {
        let r = router();
        let sink = RoutingObservationSink::new(r.clone());
        let event = FeedbackEvent::TaskCompleted {
            turns: 0,
            failure_reason: None,
            settled: settled_as(AttemptOutcome::Passed, false),
            plan_id: "p".into(),
            task_id: "t".into(),
            outcome: outcome(true),
            model_source: ModelChoiceSource::Router,
            succeeded: true,
            routing_context: None,
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: String::new(),
        };
        sink.on_event(&event).await.unwrap();
        let snap = r.confidence_snapshot();
        let (trials, successes) = snap
            .get("claude-sonnet-4-6")
            .copied()
            .expect("snapshot for the observed slug");
        assert_eq!(trials, 1);
        assert_eq!(successes, 1);
        assert!(
            r.total_observations() >= 1,
            "observe_multi_objective should advance the LinUCB observation counter",
        );
    }

    /// gap-9cbf35: a ladder rung's settled outcome teaches the router like
    /// its own pick would, so the learner sees every rung.
    #[tokio::test]
    async fn ladder_outcomes_are_recorded_like_router_outcomes() {
        let r = router();
        let sink = RoutingObservationSink::new(r.clone());
        let event = FeedbackEvent::TaskCompleted {
            turns: 0,
            failure_reason: None,
            settled: settled_as(AttemptOutcome::Passed, false),
            plan_id: "p".into(),
            task_id: "t".into(),
            outcome: outcome(true),
            model_source: ModelChoiceSource::Ladder { rung: 3 },
            succeeded: true,
            routing_context: Some(test_routing_context()),
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: String::new(),
        };
        sink.on_event(&event).await.unwrap();
        assert_eq!(
            r.confidence_snapshot().get("claude-sonnet-4-6").copied(),
            Some((1, 1)),
            "a ladder outcome is a full confidence trial, not a dampened override"
        );
        assert_eq!(r.total_observations(), 1);
    }

    #[tokio::test]
    async fn routing_sink_updates_linucb_on_failure() {
        // bug-8da8ba: a router-chosen failure reaches the same learners as a
        // success, the confidence counters and LinUCB, with reward 0.
        let r = router();
        let sink = RoutingObservationSink::new(r.clone());
        let event = FeedbackEvent::TaskCompleted {
            turns: 0,
            failure_reason: None,
            settled: settled_as(AttemptOutcome::GateFailed, false),
            plan_id: "p".into(),
            task_id: "t".into(),
            outcome: outcome(false),
            model_source: ModelChoiceSource::Router,
            succeeded: false,
            routing_context: Some(test_routing_context()),
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: String::new(),
        };
        sink.on_event(&event).await.unwrap();
        let (trials, successes) = r
            .confidence_snapshot()
            .get("claude-sonnet-4-6")
            .copied()
            .expect("snapshot for the observed slug");
        assert_eq!(trials, 1, "failure must increment trials");
        assert_eq!(successes, 0, "failure must not increment successes");
        assert_eq!(
            r.total_observations(),
            1,
            "a failure must reach LinUCB like a success does",
        );
        let arms = r.linucb().arm_stats();
        let arm = arms
            .iter()
            .find(|arm| arm.slug == "claude-sonnet-4-6")
            .expect("arm for the observed slug");
        assert_eq!(arm.observations, 1);
        assert!(
            arm.b_vector.iter().all(|b| *b == 0.0),
            "a failure earns reward 0, so the arm's b vector stays 0",
        );
    }

    #[tokio::test]
    async fn override_source_routes_through_dampened_path() {
        // Audit #90: ModelChoiceSource::Override must use
        // record_override_outcome (dampened) instead of the normal
        // observe_multi_objective / confidence path.
        let r = router();
        let sink = RoutingObservationSink::new(r.clone());
        let event = FeedbackEvent::TaskCompleted {
            turns: 0,
            failure_reason: None,
            settled: settled_as(AttemptOutcome::Passed, false),
            plan_id: "p".into(),
            task_id: "t".into(),
            outcome: outcome(true),
            model_source: ModelChoiceSource::Override,
            succeeded: true,
            routing_context: None,
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: String::new(),
        };
        sink.on_event(&event).await.unwrap();
        // record_override_outcome uses observe_multi_objective with
        // dampened quality (0.5), so LinUCB observations advance but
        // the confidence_stats snapshot should NOT be touched by the
        // override path (it only goes through the LinUCB bandit).
        assert!(
            r.total_observations() >= 1,
            "override must still advance LinUCB observations via dampened path",
        );
    }

    #[tokio::test]
    async fn unknown_model_falls_back_to_record_outcome() {
        let r = router();
        let sink = RoutingObservationSink::new(r.clone());
        let mut bad_outcome = outcome(true);
        bad_outcome.model = "no-such-slug".into();
        let event = FeedbackEvent::TaskCompleted {
            turns: 0,
            failure_reason: None,
            settled: settled_as(AttemptOutcome::Passed, false),
            plan_id: "p".into(),
            task_id: "t".into(),
            outcome: bad_outcome,
            model_source: ModelChoiceSource::Router,
            succeeded: true,
            routing_context: None,
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: String::new(),
        };
        sink.on_event(&event).await.unwrap();
        assert!(
            r.confidence_snapshot().get("no-such-slug").is_none(),
            "unknown slug must not be silently registered",
        );
    }

    #[tokio::test]
    async fn sink_ignores_non_task_events() {
        let r = router();
        let sink = RoutingObservationSink::new(r);
        let event = FeedbackEvent::IdleTick {
            ticks_since_last_work: 1,
        };
        assert!(!sink.interested(&event));
        sink.on_event(&event).await.unwrap();
    }

    #[tokio::test]
    async fn real_routing_context_feeds_bandit() {
        let r = router();
        let sink = RoutingObservationSink::new(r.clone());
        let ctx = test_routing_context();
        let event = FeedbackEvent::TaskCompleted {
            turns: 0,
            failure_reason: None,
            settled: settled_as(AttemptOutcome::Passed, false),
            plan_id: "p".into(),
            task_id: "t".into(),
            outcome: outcome(true),
            model_source: ModelChoiceSource::Router,
            succeeded: true,
            routing_context: Some(ctx),
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: String::new(),
        };
        sink.on_event(&event).await.unwrap();
        assert!(
            r.total_observations() >= 1,
            "observe_multi_objective must use the real RoutingContext features",
        );
        let snap = r.confidence_snapshot();
        let (trials, successes) = snap
            .get("claude-sonnet-4-6")
            .copied()
            .expect("snapshot for the observed slug");
        assert_eq!(trials, 1);
        assert_eq!(successes, 1);
    }

    #[tokio::test]
    async fn none_context_falls_back_to_defaults() {
        // When routing_context is None, the sink should still work using
        // the hardcoded default context (backward compat).
        let r = router();
        let sink = RoutingObservationSink::new(r.clone());
        let event = FeedbackEvent::TaskCompleted {
            turns: 0,
            failure_reason: None,
            settled: settled_as(AttemptOutcome::Passed, false),
            plan_id: "p".into(),
            task_id: "t".into(),
            outcome: outcome(true),
            model_source: ModelChoiceSource::Router,
            succeeded: true,
            routing_context: None,
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: String::new(),
        };
        sink.on_event(&event).await.unwrap();
        assert!(
            r.total_observations() >= 1,
            "fallback context must still drive observe_multi_objective",
        );
    }
}
