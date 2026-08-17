//! Affect, somatic-marker, and action-valuation cross-cut.

use std::sync::{Arc, RwLock};

use async_trait::async_trait;
#[cfg(test)]
use roko_core::PadVector;
use roko_core::{BehavioralState, Body, Kind, Provenance, Signal};
use roko_daimon::{
    AffectEngine, AffectEvent, BehavioralStateThresholds, DaimonState, SomaticRetrieval,
    SomaticRetrievalConfig, StrategyCoordinates,
};

use crate::ComposeError;
use crate::cross_cut::{CrossCutContext, CrossCutFunctor, CrossCutResult, LoopStep};

/// Kahneman-Tversky loss-aversion multiplier.
pub const PROSPECT_LAMBDA: f64 = 2.25;
/// Kahneman-Tversky diminishing-sensitivity exponent.
pub const PROSPECT_ALPHA: f64 = 0.88;
const HIGH_ANXIETY_AROUSAL: f64 = 0.5;

/// Daimon enrichment backed by the live mutable affect state.
pub struct DaimonFunctor {
    state: Arc<RwLock<DaimonState>>,
    thresholds: BehavioralStateThresholds,
    somatic_config: SomaticRetrievalConfig,
}

impl DaimonFunctor {
    /// Bind the functor to the runtime Daimon state.
    #[must_use]
    pub fn new(state: Arc<RwLock<DaimonState>>) -> Self {
        Self {
            state,
            thresholds: BehavioralStateThresholds::default(),
            somatic_config: SomaticRetrievalConfig::default(),
        }
    }

    /// Borrow the shared live state.
    #[must_use]
    pub fn state(&self) -> &Arc<RwLock<DaimonState>> {
        &self.state
    }

    /// Override the same hysteresis thresholds used by the Daimon tracker.
    #[must_use]
    pub const fn with_thresholds(mut self, thresholds: BehavioralStateThresholds) -> Self {
        self.thresholds = thresholds;
        self
    }

    fn assess_pre(&self, mut input: Vec<Signal>) -> CrossCutResult<Vec<Signal>> {
        let state = self
            .state
            .read()
            .map_err(|_| ComposeError::Other("daimon state lock poisoned".into()))?;
        let affect = state.query();
        let coordinates = strategy_coordinates(&input);
        let somatic =
            SomaticRetrieval::query(&state.somatic_landscape, coordinates, &self.somatic_config);
        input.push(
            metadata_signal(
                "roko.cross_cut.daimon.pad",
                serde_json::json!({
                    "pleasure": affect.pad.pleasure,
                    "arousal": affect.pad.arousal,
                    "dominance": affect.pad.dominance,
                    "behavioral_state": format!("{:?}", affect.behavioral_state).to_lowercase(),
                }),
            )
            .build(),
        );
        input.push(
            metadata_signal(
                "roko.cross_cut.daimon.somatic",
                serde_json::json!({
                    "valence": somatic.blended_valence,
                    "intensity": somatic.blended_intensity,
                    "neighbor_count": somatic.primary_signal.neighbor_count,
                    "contrarian_count": somatic.primary_signal.contrarian_count,
                    "contrarian_applied": somatic.contrarian_applied,
                    "contrarian_fraction_required": self.somatic_config.contrarian_fraction,
                }),
            )
            .tag(
                "contrarian_fraction",
                self.somatic_config.contrarian_fraction.to_string(),
            )
            .build(),
        );
        Ok(input)
    }

    fn assess_post(&self, mut output: Vec<Signal>) -> CrossCutResult<Vec<Signal>> {
        let state = self
            .state
            .read()
            .map_err(|_| ComposeError::Other("daimon state lock poisoned".into()))?;
        let pad = state.query().pad;
        if pad.arousal > HIGH_ANXIETY_AROUSAL
            && pad.dominance < self.thresholds.struggling_entry_dominance
        {
            output.push(
                metadata_signal(
                    "roko.cross_cut.daimon.tier_escalation",
                    serde_json::json!({
                        "minimum_tier": "reflective",
                        "reason": "high anxiety and low dominance",
                        "arousal": pad.arousal,
                        "dominance": pad.dominance,
                    }),
                )
                .tag("tier_escalation", "reflective")
                .build(),
            );
        }
        Ok(output)
    }

    fn act_pre(&self, mut input: Vec<Signal>) -> CrossCutResult<Vec<Signal>> {
        let state = self
            .state
            .read()
            .map_err(|_| ComposeError::Other("daimon state lock poisoned".into()))?;
        let affect = state.query();
        let cautious = affect.behavioral_state == BehavioralState::Struggling
            || affect.pad.dominance < self.thresholds.struggling_entry_dominance;
        if cautious && input.iter().any(is_high_risk) {
            input.push(
                metadata_signal(
                    "roko.cross_cut.daimon.deferral",
                    serde_json::json!({
                        "action": "defer",
                        "reason": "affect state does not support a high-risk action",
                        "behavioral_state": format!("{:?}", affect.behavioral_state).to_lowercase(),
                    }),
                )
                .tag("recommendation_source", "daimon")
                .tag("decision_kind", "act")
                .tag("decision_key", "action_gate")
                .tag("recommendation_value", "defer")
                .tag("recommendation_confidence", "1.0")
                .tag("priority_level", "1")
                .tag("safety_critical", "true")
                .build(),
            );
        }
        Ok(input)
    }

    fn act_post(
        &self,
        mut output: Vec<Signal>,
        ctx: &CrossCutContext,
    ) -> CrossCutResult<Vec<Signal>> {
        let (actual, reference) = reward_pair(&output).unwrap_or((0.5, 0.5));
        let value = prospect_value(actual, reference);
        self.state
            .write()
            .map_err(|_| ComposeError::Other("daimon state lock poisoned".into()))?
            .appraise(AffectEvent::TaskOutcome {
                task_id: ctx.task_id.clone(),
                succeeded: value >= 0.0,
            });
        output.push(
            metadata_signal(
                "roko.cross_cut.daimon.prospect_value",
                serde_json::json!({
                    "actual": actual,
                    "reference": reference,
                    "value": value,
                    "lambda": PROSPECT_LAMBDA,
                    "alpha": PROSPECT_ALPHA,
                }),
            )
            .tag("prospect_value", value.to_string())
            .build(),
        );
        Ok(output)
    }
}

#[async_trait]
impl CrossCutFunctor for DaimonFunctor {
    fn name(&self) -> &str {
        "daimon"
    }

    async fn pre_enrich(
        &self,
        input: Vec<Signal>,
        ctx: &CrossCutContext,
    ) -> CrossCutResult<Vec<Signal>> {
        match ctx.step {
            LoopStep::Assess => self.assess_pre(input),
            LoopStep::Act => self.act_pre(input),
            _ => Ok(input),
        }
    }

    async fn post_enrich(
        &self,
        output: Vec<Signal>,
        ctx: &CrossCutContext,
    ) -> CrossCutResult<Vec<Signal>> {
        match ctx.step {
            LoopStep::Assess => self.assess_post(output),
            LoopStep::Act => self.act_post(output, ctx),
            _ => Ok(output),
        }
    }

    fn should_short_circuit(&self) -> bool {
        self.state.read().map_or(true, |state| {
            let pad = state.query().pad;
            pad.pleasure.abs() < 0.1 && pad.arousal.abs() < 0.1 && pad.dominance.abs() < 0.1
        })
    }
}

/// Prospect-theory value relative to an expected reference point.
#[must_use]
pub fn prospect_value(outcome: f64, reference: f64) -> f64 {
    let delta = outcome - reference;
    if delta >= 0.0 {
        delta.powf(PROSPECT_ALPHA)
    } else {
        -PROSPECT_LAMBDA * (-delta).powf(PROSPECT_ALPHA)
    }
}

fn metadata_signal(kind: &str, value: serde_json::Value) -> roko_core::SignalBuilder {
    Signal::builder(Kind::Custom(kind.to_string()))
        .body(Body::Json(value))
        .provenance(Provenance::trusted("daimon"))
        .tag("cross_cut", "daimon")
}

fn strategy_coordinates(signals: &[Signal]) -> StrategyCoordinates {
    let number = |key: &str, fallback: f64| {
        signals
            .iter()
            .find_map(|signal| signal.tag(key).and_then(|value| value.parse().ok()))
            .or_else(|| {
                signals.iter().find_map(|signal| match &signal.body {
                    Body::Json(value) => value.get(key).and_then(serde_json::Value::as_f64),
                    Body::Empty | Body::Text(_) | Body::Bytes(_) => None,
                })
            })
            .unwrap_or(fallback)
    };
    StrategyCoordinates::new(
        number("complexity", 0.5),
        number("risk", 0.5),
        number("novelty", 0.5),
        number("confidence", 0.5),
        number("time_pressure", 0.5),
        number("scope", 0.5),
        number("reversibility", 0.5),
        number("dependency_depth", 0.5),
    )
}

fn is_high_risk(signal: &Signal) -> bool {
    signal
        .tag("risk_level")
        .is_some_and(|risk| matches!(risk.to_ascii_lowercase().as_str(), "high" | "critical"))
        || match &signal.body {
            Body::Json(value) => value.get("risk").is_some_and(|risk| {
                risk.as_f64().is_some_and(|risk| risk > 0.5)
                    || risk.as_str().is_some_and(|risk| {
                        matches!(risk.to_ascii_lowercase().as_str(), "high" | "critical")
                    })
            }),
            Body::Empty | Body::Text(_) | Body::Bytes(_) => false,
        }
}

fn reward_pair(signals: &[Signal]) -> Option<(f64, f64)> {
    signals.iter().rev().find_map(|signal| {
        let Body::Json(value) = &signal.body else {
            return None;
        };
        let actual = value
            .get("reward")
            .or_else(|| value.get("outcome"))
            .and_then(serde_json::Value::as_f64)?;
        let reference = value
            .get("expected_reward")
            .or_else(|| value.get("reference"))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.5);
        Some((actual, reference))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn losses_are_weighted_more_than_equal_gains() {
        let gain = prospect_value(0.7, 0.5);
        let loss = prospect_value(0.3, 0.5);
        assert!(loss.abs() > gain * 2.2);
    }

    #[tokio::test]
    async fn struggling_state_defers_high_risk_action() {
        let mut state = DaimonState::new();
        state.state.pad = PadVector::new(-0.5, 0.7, -0.5);
        state.state.behavioral_state = BehavioralState::Struggling;
        state.behavioral_tracker.current_state = BehavioralState::Struggling;
        let functor = DaimonFunctor::new(Arc::new(RwLock::new(state)));
        let input = vec![
            Signal::builder(Kind::Task)
                .body(Body::Json(serde_json::json!({"risk": "critical"})))
                .build(),
        ];
        let ctx = CrossCutContext {
            step: LoopStep::Act,
            task_id: "risky".into(),
            ..CrossCutContext::default()
        };

        let enriched = functor.pre_enrich(input, &ctx).await.unwrap();

        assert!(enriched.iter().any(|signal| {
            signal.tag("recommendation_value") == Some("defer")
                && signal.tag("safety_critical") == Some("true")
        }));
    }

    #[tokio::test]
    async fn anxious_assessment_escalates_tier() {
        let mut state = DaimonState::new();
        state.state.pad = PadVector::new(-0.4, 0.8, -0.5);
        let functor = DaimonFunctor::new(Arc::new(RwLock::new(state)));
        let ctx = CrossCutContext {
            step: LoopStep::Assess,
            ..CrossCutContext::default()
        };

        let enriched = functor.post_enrich(Vec::new(), &ctx).await.unwrap();

        assert_eq!(enriched[0].tag("tier_escalation"), Some("reflective"));
    }
}
