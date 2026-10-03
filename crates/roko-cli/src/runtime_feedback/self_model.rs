//! M3's outcome sink (S04 T10; backlog 6129): each settled verdict teaches the self-model the
//! attempt it forecast before routing (6128), and feeds the calibration gate's window (6122).
//!
//! Labels follow S01 §4.1. A learning label of 1 or 0 is a gate label, and `forced_accept`
//! counts as 0. Unverified, provider, infra and harness outcomes teach nothing, and neither does
//! an attempt a provider failover ran on a substitute model, since nobody chose it. The plan
//! runner saves the state when the run ends, and a resumed run loads it again. A frozen run
//! registers no such sink (decision 2218). Late VS labels from S05 come in through
//! [`SelfModelOutcomeSink::observe_label`].

use std::sync::Arc;

use async_trait::async_trait;
use roko_learn::self_model::{ArmKey, Label, LabelSource, Unit};
use roko_learn::telemetry::{AttemptKey, AttemptOutcome, AttemptVerdictRecord};

use super::{FeedbackEvent, FeedbackSink};
use crate::graph_task_dispatch::self_model::{AttemptForecast, SelfModelRuntime};

/// Sink that teaches the self-model each settled verdict it forecast.
#[derive(Debug, Clone)]
pub struct SelfModelOutcomeSink {
    runtime: Arc<SelfModelRuntime>,
}

impl SelfModelOutcomeSink {
    /// A sink teaching `runtime`, the run's self-model.
    #[must_use]
    pub fn new(runtime: Arc<SelfModelRuntime>) -> Self {
        Self { runtime }
    }

    /// A late VS label of a settled attempt (S05's `vs.label`, with weight 1/π): only the
    /// false-green head learns it. `false` when this run settled no such attempt.
    pub fn observe_label(
        &self,
        attempt_key: &str,
        y_vs: bool,
        weight: f64,
        source: LabelSource,
    ) -> bool {
        self.runtime
            .observe_label(attempt_key, y_vs, weight, source)
    }
}

#[async_trait]
impl FeedbackSink for SelfModelOutcomeSink {
    fn name(&self) -> &'static str {
        "self_model"
    }

    /// Settled attempts only: every attempt settles once (S01 §4.3).
    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(event, FeedbackEvent::AttemptSettled(_))
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        let FeedbackEvent::AttemptSettled(verdict) = event else {
            return Ok(());
        };
        // The forecast leaves the cache whatever the verdict teaches.
        let Some(forecast) = self.runtime.take_forecast(&verdict.identity.attempt_key) else {
            return Ok(());
        };
        if let Some(unit) = unit_of(verdict, &forecast) {
            self.runtime.settle(unit, forecast);
        }
        Ok(())
    }
}

/// The gate label of `verdict` (S01 §4.1): its learning label, with `forced_accept` a fail.
fn gate_label(verdict: &AttemptVerdictRecord) -> Option<bool> {
    match verdict.outcome {
        AttemptOutcome::ForcedAccept => Some(false),
        _ => verdict.learning_success(),
    }
}

/// The unit `verdict` teaches, with the features its forecast kept; `None` when it teaches
/// nothing.
fn unit_of(verdict: &AttemptVerdictRecord, forecast: &AttemptForecast) -> Option<Unit> {
    let passed = gate_label(verdict)?;
    let executed = &verdict.executed;
    if !executed.failover_chain.is_empty() {
        return None;
    }
    let model = executed
        .model_dispatched
        .clone()
        .or_else(|| executed.model_requested.clone())?;
    let provider = executed
        .provider
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let identity = &verdict.identity;
    let features = &forecast.features;
    Some(Unit {
        attempt_key: AttemptKey::new(
            identity.run_id.clone(),
            identity.plan_id.clone(),
            identity.task_id.clone(),
            identity.attempt,
        ),
        plan_id: identity.plan_id.clone(),
        task_id: identity.task_id.clone(),
        role: features.role.clone(),
        tier: features.tier.clone(),
        family: features.family.clone(),
        arm: ArmKey::roko(provider, model),
        attempt: identity.attempt,
        prior_failure: features.has_prior_failure,
        failure_class: features.error_class.clone(),
        label: Label {
            y_gate: Some(passed),
            y_vs: None,
            weight: 1.0,
            source: LabelSource::GatePassed,
        },
        api_equiv_usd: verdict.cost.api_equiv_usd,
        cost_source: verdict.cost.source,
        latency_s: latency_s(verdict),
        failover: false,
    })
}

/// The attempt's wall time in seconds, from its start to its settlement.
fn latency_s(verdict: &AttemptVerdictRecord) -> Option<f64> {
    let timing = &verdict.timing;
    let (start, end) = timing
        .attempt_started_at
        .zip(timing.settled_at)
        .or_else(|| timing.dispatch_started_at.zip(timing.dispatch_ended_at))?;
    let millis = end.checked_sub(start).filter(|millis| *millis >= 0)?;
    Some(millis as f64 / 1_000.0)
}

#[cfg(test)]
mod tests {
    use roko_core::config::self_model::{SelfModelConfig, SelfModelMode};
    use roko_core::pricing_snapshot::PriceSnapshot;
    use roko_learn::self_model::features::TaskFeatures;
    use roko_learn::self_model::model::{SelfModel, StateLoad};
    use roko_learn::telemetry::AttemptIdentity;

    use super::*;

    const RUN: &str = "graph-self-model-run";

    fn task() -> TaskFeatures {
        TaskFeatures {
            title: "Fix the parser".to_string(),
            family: "focused".to_string(),
            tier: "focused".to_string(),
            role: "implementer".to_string(),
            attempt: 1,
            ..TaskFeatures::default()
        }
    }

    /// Forecast `task`'s attempt on gpt-oss-120b with `runtime`'s model, as dispatch does.
    fn forecast(runtime: &SelfModelRuntime, snapshot: &PriceSnapshot, task: &str) -> String {
        let arm = ArmKey::roko("cerebras", "gpt-oss-120b");
        let candidates = SelfModel::new(snapshot).forecast(&self::task(), &[arm]);
        let key = AttemptKey::new(RUN, "plan", task, 1).attempt_key();
        let forecast = AttemptForecast {
            version: runtime.version(),
            features: self::task(),
            candidates,
            would_choose: Some(0),
            default: 0,
            pinned: false,
            routed: false,
        };
        runtime.remember(key.clone(), forecast);
        key
    }

    /// `task`'s attempt settled with `outcome` on gpt-oss-120b.
    fn settled(task: &str, outcome: AttemptOutcome) -> FeedbackEvent {
        let identity = AttemptIdentity::new(&AttemptKey::new(RUN, "plan", task, 1));
        let mut verdict = AttemptVerdictRecord::settle(identity, outcome, true);
        verdict.executed.provider = Some("cerebras".to_string());
        verdict.executed.model_dispatched = Some("gpt-oss-120b".to_string());
        verdict.cost.api_equiv_usd = Some(0.015);
        FeedbackEvent::AttemptSettled(Arc::new(verdict))
    }

    /// 6129: a settled pass, an agent failure and a forced accept move the state; an
    /// unverified attempt and a failover substitute do not; every forecast leaves the cache;
    /// the state survives a reload; a late VS label reaches a settled attempt only.
    #[tokio::test]
    async fn settled_verdict_updates_self_model_state() {
        let temp = tempfile::tempdir().expect("tempdir");
        let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");
        let state = temp.path().join(".roko/learn/self-model/state-v1.json");
        let settings = SelfModelConfig {
            mode: SelfModelMode::Shadow,
            ..SelfModelConfig::default()
        };
        let model = SelfModel::new(&snapshot);
        let runtime = Arc::new(SelfModelRuntime::new(settings, state.clone(), model));
        let sink = SelfModelOutcomeSink::new(Arc::clone(&runtime));

        let cases = [
            ("T1", AttemptOutcome::Passed, 1),
            ("T2", AttemptOutcome::GateFailed, 2),
            ("T3", AttemptOutcome::ForcedAccept, 3),
            ("T4", AttemptOutcome::Unverified, 3),
            ("T5", AttemptOutcome::ProviderError, 3),
        ];
        for (task, outcome, learned) in cases {
            let key = forecast(&runtime, &snapshot, task);
            let event = settled(task, outcome);
            assert!(sink.interested(&event));
            sink.on_event(&event).await.expect("the sink takes the verdict");
            assert_eq!(runtime.outcomes(), learned, "{task}: {outcome:?}");
            assert!(
                runtime.take_forecast(&key).is_none(),
                "{task}'s forecast left the cache"
            );
        }
        let key = forecast(&runtime, &snapshot, "T6");
        let FeedbackEvent::AttemptSettled(verdict) = settled("T6", AttemptOutcome::Passed) else {
            unreachable!("settled() builds an AttemptSettled event");
        };
        let mut substitute = (*verdict).clone();
        substitute.executed.failover_chain = vec!["glm-4.7".to_string()];
        let event = FeedbackEvent::AttemptSettled(Arc::new(substitute));
        sink.on_event(&event).await.expect("the sink takes the verdict");
        assert_eq!(
            runtime.outcomes(),
            3,
            "a failover substitute teaches nothing"
        );
        assert!(runtime.take_forecast(&key).is_none());
        // The three labelled attempts reached the calibration gate's window.
        assert_eq!(runtime.gate().n, 3);

        runtime.save().expect("save the state");
        let (reloaded, load) = SelfModel::load_or_new(&state, &snapshot).expect("reload");
        assert_eq!(load, StateLoad::Loaded);
        assert_eq!(reloaded.outcomes, 3);

        let passed = AttemptKey::new(RUN, "plan", "T1", 1).attempt_key();
        assert!(sink.observe_label(&passed, false, 4.0, LabelSource::Vs));
        let unknown = AttemptKey::new(RUN, "plan", "T9", 1).attempt_key();
        assert!(!sink.observe_label(&unknown, true, 4.0, LabelSource::Vs));
        assert_eq!(
            runtime.outcomes(),
            3,
            "a late VS label is not another outcome"
        );
    }
}
