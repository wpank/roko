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
//! ## Credit for the router's own picks
//!
//! The sink credits an outcome by the source routing returned for its model
//! (`router_credit`). The router learns only from the choices it made
//! (decision 4111 (A), S02 §4.1): its own pick
//! ([`ModelChoiceSource::Router`]) updates its category stats, confidence
//! and bandit. When [`ModelChoiceSource::Override`] tagged a task, the sink
//! records it via `record_override_outcome` so manual operator overrides do
//! not pollute the bandit signal that drives router decisions on
//! non-overridden tasks. A `[routing.ladder]` rung
//! ([`ModelChoiceSource::Ladder`]), a task hint, a guard's fallback and the
//! default were not the router's choice, so they update none of its state:
//! while the ladder routes, the router is a shadow learner. The attempt's
//! verdict still records its rung and the router's shadow pick
//! (`AttemptLadder`).
//!
//! ## Durability
//!
//! A Graph run saves its router when it ends. Its sink journals each outcome
//! in the learning WAL first ([`RoutingObservationSink::with_journal`]), so
//! a run that dies before that save keeps them: the router the next run
//! loads replays the journal (bug-dfb28f).

use std::sync::Arc;

use async_trait::async_trait;
use roko_core::agent::AgentRole;
use roko_core::config::RewardWeights;
use roko_core::task::{TaskCategory, TaskComplexityBand};
use roko_core::{BehavioralState, DaimonPolicy};
use roko_learn::cascade_router::{CascadeRouter, normalized_cost_and_latency};
use roko_learn::model_call_feedback::ModelCallJournal;
use roko_learn::model_router::RoutingContext;

use super::{FeedbackEvent, FeedbackSink};
use crate::dispatch::ModelChoiceSource;
use crate::runner::conductor_adapter::compute_conductor_load;

/// Sink that records a routing observation per `task_completed` event.
#[derive(Clone)]
pub struct RoutingObservationSink {
    router: Arc<CascadeRouter>,
    /// The journal the router's owner saves the router through, when it has
    /// one: each outcome is journaled before it is applied.
    journal: Option<Arc<ModelCallJournal>>,
}

impl RoutingObservationSink {
    /// Construct a routing sink wrapping a shared router.
    #[must_use]
    pub fn new(router: Arc<CascadeRouter>) -> Self {
        Self {
            router,
            journal: None,
        }
    }

    /// Journal each outcome in `journal` before applying it, so the outcomes
    /// survive a crash before the router's owner saves the router through
    /// `journal` (bug-dfb28f).
    #[must_use]
    pub fn with_journal(mut self, journal: Arc<ModelCallJournal>) -> Self {
        self.journal = Some(journal);
        self
    }
}

impl std::fmt::Debug for RoutingObservationSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutingObservationSink")
            .field("router", &"..")
            .field("journal", &self.journal.is_some())
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
            settled,
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
        // Provider failover ran a model the router did not pick: crediting
        // it would teach the router a choice it never made (bug-35379d).
        if settled
            .as_ref()
            .is_some_and(|settled| !settled.executed.failover_chain.is_empty())
        {
            return Ok(());
        }

        // Decision 4111 (A): a model the router did not choose teaches it
        // nothing.
        let Some(credit) = router_credit(*model_source) else {
            return Ok(());
        };
        let ctx = match routing_context {
            Some(ctx) => ctx.clone(),
            None => build_fallback_routing_context(
                &outcome.model,
                settled.as_ref().map(|settled| settled.identity.attempt),
            ),
        };

        if let Some(journal) = &self.journal {
            // Journaled, then applied, with the reward and weight the paths
            // below give it (an override's weight is dampened), so a replay
            // repeats it exactly. The journal moves the category counts under
            // the same lock, so a save cannot split them from the journaled
            // observation (bug-a83a6e). The WAL write syncs to disk, so it
            // runs off the reactor.
            let journal = Arc::clone(journal);
            let router = Arc::clone(&self.router);
            let model = outcome.model.clone();
            let (cost_usd, duration_ms) = (outcome.cost_usd, outcome.duration_ms);
            let overridden = credit == RouterCredit::Override;
            tokio::task::spawn_blocking(move || {
                if overridden {
                    journal.observe_override_outcome(
                        &router,
                        &model,
                        &ctx,
                        succeeded,
                        cost_usd,
                        duration_ms,
                    );
                } else {
                    journal.observe_task_outcome(
                        &router,
                        &model,
                        &ctx,
                        succeeded,
                        cost_usd,
                        duration_ms,
                    );
                }
            })
            .await?;
            return Ok(());
        }

        // Audit #84: always record category-level stats (even for
        // overrides) so confidence_scores can adjust per-category.
        self.router
            .record_category_outcome(&outcome.model, ctx.task_category, succeeded);

        // Audit #90: manual overrides must not pollute the bandit signal.
        // Route them through the dampened `record_override_outcome` path
        // instead of the full router-outcome path.
        if credit == RouterCredit::Override {
            self.router.record_override_outcome(
                &outcome.model,
                &ctx,
                succeeded,
                outcome.cost_usd,
                outcome.duration_ms,
                None,
            );
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

/// How the routing sink credits an outcome to the router.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouterCredit {
    /// The router's learners: category stats, confidence and the bandit.
    Full,
    /// An operator override: dampened, so it does not pollute the bandit
    /// signal (audit #90).
    Override,
}

/// The credit an outcome earns from the source routing returned for its
/// model (decision 4111 (A)): full for the router's own pick, dampened for an
/// operator override, and none for a ladder rung, a task hint, a guard's
/// fallback or the default, which the router did not choose.
const fn router_credit(source: ModelChoiceSource) -> Option<RouterCredit> {
    match source {
        ModelChoiceSource::Router => Some(RouterCredit::Full),
        ModelChoiceSource::Override => Some(RouterCredit::Override),
        ModelChoiceSource::TaskHint
        | ModelChoiceSource::Ladder { .. }
        | ModelChoiceSource::Fallback { .. }
        | ModelChoiceSource::Default => None,
    }
}

/// Whether the routing sink credits the router with an outcome whose model
/// came from `source`. The episode records it, so a hindsight relabel
/// retracts only the credit the router was given.
pub(crate) const fn credits_router(source: ModelChoiceSource) -> bool {
    router_credit(source).is_some()
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
    let (normalized_cost, normalized_latency) = normalized_cost_and_latency(cost_usd, duration_ms);
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
/// `attempt` is the attempt's 1-based ordinal, when known: a later attempt
/// is a retry after a failed one (gap-b62e95).
pub(crate) fn build_fallback_routing_context(model: &str, attempt: Option<u32>) -> RoutingContext {
    let iteration = attempt.map_or(0, |attempt| attempt.saturating_sub(1));
    RoutingContext {
        task_category: TaskCategory::Implementation,
        complexity: TaskComplexityBand::Standard,
        iteration,
        role: AgentRole::Implementer,
        crate_familiarity: 0.5,
        has_prior_failure: iteration > 0,
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

    /// gap-b62e95: a fallback context for a later attempt is a retry after a
    /// failure; one for a first or unknown attempt is not.
    #[test]
    fn fallback_routing_context_marks_retries() {
        let marks = |attempt| {
            let ctx = build_fallback_routing_context("claude-sonnet-4-6", attempt);
            (ctx.iteration, ctx.has_prior_failure)
        };
        assert_eq!(marks(None), (0, false));
        assert_eq!(marks(Some(1)), (0, false));
        assert_eq!(marks(Some(3)), (2, true));
    }

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

    /// An attempt that provider failover ran on a substitute is not the
    /// router's pick, so it earns the substitute no credit (bug-35379d).
    #[tokio::test]
    async fn a_failover_substitute_earns_no_router_credit() {
        use roko_learn::telemetry::{AttemptIdentity, AttemptKey};

        let r = router();
        let sink = RoutingObservationSink::new(r.clone());
        let mut verdict = AttemptVerdictRecord::settle(
            AttemptIdentity::new(&AttemptKey::new("run", "p", "t", 1)),
            AttemptOutcome::Passed,
            true,
        );
        verdict.executed.failover_chain = vec!["gpt-5".into()];
        let event = FeedbackEvent::TaskCompleted {
            turns: 0,
            failure_reason: None,
            settled: Some(Arc::new(verdict)),
            plan_id: "p".into(),
            task_id: "t".into(),
            outcome: outcome(true),
            model_source: ModelChoiceSource::Router,
            succeeded: true,
            routing_context: Some(test_routing_context()),
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: String::new(),
        };
        sink.on_event(&event).await.unwrap();

        let trials = r
            .confidence_snapshot()
            .get("claude-sonnet-4-6")
            .map_or(0, |(trials, _)| *trials);
        assert_eq!(trials, 0, "the substitute is not credited");
        assert_eq!(r.total_observations(), 0, "LinUCB is not updated");
    }

    /// Decision 4111 (A): the router is credited only for its own picks. A
    /// ladder rung's labelled outcome updates none of its state, on either
    /// sink path, and the attempt's verdict still carries its rung. A task
    /// hint, a guard's fallback and the default teach it nothing either; the
    /// router's own pick of the same model does.
    #[tokio::test]
    async fn ladder_outcome_recorded_apart_from_router_picks() {
        use roko_learn::telemetry::{AttemptLadder, LadderReason};

        let mut verdict = settled_as(AttemptOutcome::Passed, false).expect("a settled verdict");
        Arc::make_mut(&mut verdict).ladder = Some(AttemptLadder {
            rung: Some("mid".into()),
            index: Some(1),
            step: 0,
            reason: LadderReason::Start,
            exhausted: false,
            router_pick: Some("gpt-5".into()),
        });
        let temp = tempfile::tempdir().expect("tempdir");
        let snapshot = temp.path().join("learn").join("cascade-router.json");
        let journal = Arc::new(ModelCallJournal::for_snapshot(&snapshot));
        let (plain, journaled) = (router(), router());
        let sinks = [
            RoutingObservationSink::new(plain.clone()),
            RoutingObservationSink::new(journaled.clone()).with_journal(journal),
        ];
        let not_chosen = [
            ModelChoiceSource::Ladder { rung: 1 },
            ModelChoiceSource::TaskHint,
            ModelChoiceSource::Fallback {
                reason: crate::dispatch::FallbackReason::NoToolSupport,
            },
            ModelChoiceSource::Default,
        ];
        for source in not_chosen {
            let event = completed(Some(Arc::clone(&verdict)), source);
            for sink in &sinks {
                assert!(sink.interested(&event), "{source:?}");
                sink.on_event(&event).await.unwrap();
            }
        }
        for r in [&plain, &journaled] {
            assert!(r.confidence_snapshot().is_empty(), "no confidence trial");
            assert!(r.category_stats_snapshot().is_empty(), "no category stats");
            assert_eq!(r.total_observations(), 0, "no bandit observation");
        }
        let ladder = verdict.ladder.as_ref().expect("the verdict keeps its rung");
        assert_eq!(ladder.rung.as_deref(), Some("mid"));

        let picked = completed(Some(verdict), ModelChoiceSource::Router);
        sinks[0].on_event(&picked).await.unwrap();
        let confidence = plain.confidence_snapshot();
        assert_eq!(
            confidence.get("claude-sonnet-4-6").copied(),
            Some((1, 1)),
            "the router's own pick is a full confidence trial"
        );
        assert_eq!(plain.total_observations(), 1);
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
        // record_override_outcome counts the override as a confidence trial
        // and gives LinUCB a dampened (half-weight) update, so the bandit's
        // observation count still advances (bug-f68404).
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

    /// bug-dfb28f: a Graph run journals each routing outcome before it
    /// applies it. A run that dies before it saves its router keeps them:
    /// the router the next run loads replays the journal, an override at
    /// its dampened weight, and a later load does not replay it again.
    #[tokio::test]
    async fn graph_run_routing_observations_survive_a_crash() {
        use roko_learn::model_call_feedback::load_recovered_router;

        let temp = tempfile::tempdir().expect("tempdir");
        let snapshot = temp.path().join("learn").join("cascade-router.json");
        let models = || vec!["claude-sonnet-4-6".to_string(), "gpt-5".to_string()];
        let live = Arc::new(CascadeRouter::new(models()));
        let journal = Arc::new(ModelCallJournal::for_snapshot(&snapshot));
        let sink = RoutingObservationSink::new(live.clone()).with_journal(Arc::clone(&journal));
        let passed = completed(
            settled_as(AttemptOutcome::Passed, false),
            ModelChoiceSource::Router,
        );
        let overridden = completed(
            settled_as(AttemptOutcome::GateFailed, false),
            ModelChoiceSource::Override,
        );
        sink.on_event(&passed).await.unwrap();
        sink.on_event(&overridden).await.unwrap();
        let sonnet = |router: &CascadeRouter| {
            router
                .linucb()
                .arm_stats()
                .into_iter()
                .find(|arm| arm.slug == "claude-sonnet-4-6")
                .expect("the sonnet arm")
        };
        let applied = sonnet(live.as_ref());
        assert_eq!(applied.observations, 2);

        // The run dies before it saves the router. Its journal goes with it,
        // and dropping the journal releases its segment: no handle is left
        // that would make recovery take the segment for a live writer's.
        drop(sink);
        let journal =
            Arc::try_unwrap(journal).expect("the sink held the journal's only other handle");
        drop(journal);
        drop(live);
        assert!(!snapshot.exists(), "nothing saved the router");

        let recovered = load_recovered_router(&snapshot, models());
        assert_eq!(
            recovered
                .confidence_snapshot()
                .get("claude-sonnet-4-6")
                .copied(),
            Some((2, 1)),
            "a pass and an overridden failure"
        );
        assert_eq!(recovered.total_observations(), 2);
        // A snapshot keeps an arm's `A` and `b`, not its own counter.
        let replayed = sonnet(&recovered);
        let rows = replayed.a_matrix.iter().zip(&applied.a_matrix);
        for (replayed_row, applied_row) in rows {
            for (r, a) in replayed_row.iter().zip(applied_row) {
                assert!((r - a).abs() < 1e-9, "A replayed {r}, applied {a}");
            }
        }
        for (r, a) in replayed.b_vector.iter().zip(&applied.b_vector) {
            assert!((r - a).abs() < 1e-9, "b replayed {r}, applied {a}");
        }

        // The snapshot holds them now, and the journal is gone.
        let reloaded = load_recovered_router(&snapshot, models());
        assert_eq!(
            reloaded
                .confidence_snapshot()
                .get("claude-sonnet-4-6")
                .copied(),
            Some((2, 1))
        );
    }
}
