//! A rung that runs slow, said once per run (gap-3217a0): one of its tries
//! reached the rung's time bound (bug-f0007b), or its settled tries spent
//! more than `[routing.ladder] slow_call_secs` on each model call over at
//! least [`MIN_CALLS`] calls. This is the live, absolute signal; the run's
//! summary compares the rungs with each other afterwards
//! (`roko_learn::telemetry::rung_stats`).

use std::collections::{HashMap, HashSet};

use roko_learn::telemetry::AttemptVerdictRecord;

use super::*;

/// The model calls a rung's settled tries need before their time per call
/// counts: one quick try's few calls say little.
pub(super) const MIN_CALLS: u32 = 5;

/// What a run has seen of its rungs' speed.
#[derive(Debug, Default)]
pub(super) struct RungSpeed {
    /// The model time, in ms, and the model calls of each rung and model's
    /// settled tries.
    calls: HashMap<(String, String), (u64, u32)>,
    /// The rungs and models already said to be slow.
    flagged: HashSet<(String, String)>,
}

/// A slow rung, as its event says it.
struct SlowRung<'a> {
    plan_id: &'a str,
    task_id: &'a str,
    attempt: u32,
    rung: &'a str,
    model: &'a str,
    reason: &'static str,
    bound_secs: Option<u64>,
    secs_per_call: Option<f64>,
    model_calls: Option<u32>,
    detail: String,
}

impl GraphTaskDispatcher {
    /// A try of `plan_id/task_id`, its `attempt`-th, reached the time bound
    /// of `rung` (`model`), `bound_secs` seconds.
    pub(super) fn rung_reached_time_bound(
        &self,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
        rung: &str,
        model: &str,
        bound_secs: u64,
    ) {
        self.flag_slow_rung(SlowRung {
            plan_id,
            task_id,
            attempt,
            rung,
            model,
            reason: "time_bound",
            bound_secs: Some(bound_secs),
            secs_per_call: None,
            model_calls: None,
            detail: format!(
                "rung {rung} ({model}) runs slow: {task_id}'s try {attempt} reached its \
                 {bound_secs} s time bound ([routing.ladder] max_secs)"
            ),
        });
    }

    /// Count `verdict`, a settled try of `plan_id/task_id`, toward its rung's
    /// time per model call, and say once that the rung runs slow when its
    /// settled tries spend more than `[routing.ladder] slow_call_secs` on
    /// each call over at least [`MIN_CALLS`] calls.
    pub(super) fn note_rung_speed(
        &self,
        plan_id: &str,
        task_id: &str,
        verdict: &AttemptVerdictRecord,
    ) {
        let Some(limit) = self.config.routing.ladder.slow_call_secs() else {
            return;
        };
        let Some(rung) = verdict
            .ladder
            .as_ref()
            .and_then(|ladder| ladder.rung.clone())
        else {
            return;
        };
        let Some((model_ms, calls)) = verdict
            .timing
            .phases
            .as_ref()
            .and_then(|phases| phases.model_ms.zip(phases.model_calls))
        else {
            return;
        };
        let executed = &verdict.executed;
        let model = executed
            .model_dispatched
            .clone()
            .or_else(|| executed.model_requested.clone())
            .unwrap_or_default();
        let (total_ms, total_calls) = {
            let mut speed = self.rung_speed.lock();
            let seen = speed
                .calls
                .entry((rung.clone(), model.clone()))
                .or_default();
            seen.0 += model_ms;
            seen.1 += calls;
            *seen
        };
        if total_calls < MIN_CALLS {
            return;
        }
        let secs_per_call = total_ms as f64 / 1_000.0 / f64::from(total_calls);
        if secs_per_call <= limit as f64 {
            return;
        }
        self.flag_slow_rung(SlowRung {
            plan_id,
            task_id,
            attempt: verdict.identity.attempt,
            rung: &rung,
            model: &model,
            reason: "slow_calls",
            bound_secs: None,
            secs_per_call: Some(secs_per_call),
            model_calls: Some(total_calls),
            detail: format!(
                "rung {rung} ({model}) runs slow: {secs_per_call:.0} s per model call over \
                 {total_calls} calls, above [routing.ladder] slow_call_secs = {limit}"
            ),
        });
    }

    /// Say `slow` once per run per rung and model.
    fn flag_slow_rung(&self, slow: SlowRung<'_>) {
        let first = self
            .rung_speed
            .lock()
            .flagged
            .insert((slow.rung.to_string(), slow.model.to_string()));
        if !first {
            return;
        }
        tracing::warn!(
            plan_id = slow.plan_id,
            task_id = slow.task_id,
            rung = slow.rung,
            model = slow.model,
            reason = slow.reason,
            "{}",
            slow.detail
        );
        if let Some(tui) = &self.tui_bridge {
            tui.publish_event(roko_core::DashboardEvent::RungSlow {
                plan_id: slow.plan_id.to_string(),
                task_id: slow.task_id.to_string(),
                attempt: slow.attempt,
                rung: slow.rung.to_string(),
                model: slow.model.to_string(),
                reason: slow.reason.to_string(),
                bound_secs: slow.bound_secs,
                secs_per_call: slow.secs_per_call,
                model_calls: slow.model_calls,
                detail: slow.detail,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use roko_learn::telemetry::AttemptIdentity;
    use roko_learn::telemetry::records::{AttemptKey, AttemptLadder, AttemptOutcome, LadderReason};

    use super::*;
    use crate::graph_task_dispatch::tests::{fixture_config, make_bare_dispatcher};
    use crate::runner::tui_bridge::TuiBridge;
    use crate::state_hub::StateHub;

    /// A settled try of `task` on mid (glm-4.7) that made `calls` model calls
    /// over `model_ms`.
    fn try_on_mid(task: &str, model_ms: u64, calls: u32) -> AttemptVerdictRecord {
        let identity = AttemptIdentity::new(&AttemptKey::new("run", "p1", task, 1));
        let mut verdict = AttemptVerdictRecord::settle(identity, AttemptOutcome::Passed, true);
        verdict.ladder = Some(AttemptLadder {
            rung: Some("mid".to_string()),
            index: Some(1),
            step: 0,
            reason: LadderReason::Start,
            exhausted: false,
            router_pick: None,
        });
        verdict.executed.model_dispatched = Some("glm-4.7".to_string());
        verdict.timing.phases = Some(roko_core::dashboard_snapshot::AttemptPhases {
            model_ms: Some(model_ms),
            model_calls: Some(calls),
            ..roko_core::dashboard_snapshot::AttemptPhases::default()
        });
        verdict
    }

    /// The `rung_slow` events `events` received so far: rung, reason and the
    /// seconds per call.
    fn slow_rungs(
        events: &mut tokio::sync::broadcast::Receiver<
            roko_runtime::event_bus::Envelope<roko_core::DashboardEvent>,
        >,
    ) -> Vec<(String, String, Option<f64>)> {
        let mut found = Vec::new();
        while let Ok(envelope) = events.try_recv() {
            if let roko_core::DashboardEvent::RungSlow {
                rung,
                reason,
                secs_per_call,
                ..
            } = envelope.payload
            {
                found.push((rung, reason, secs_per_call));
            }
        }
        found
    }

    /// gap-3217a0: a rung whose settled tries spend more than
    /// `slow_call_secs` on each model call, over at least five calls, is said
    /// to run slow once per run; so is a rung one of whose tries reached its
    /// time bound. A rung already said to run slow is not said again.
    #[tokio::test]
    async fn a_slow_rung_is_said_once_per_run() {
        let temp = tempfile::tempdir().expect("tempdir");
        let hub = StateHub::new(64);
        let mut events = hub.subscribe_events();
        let dispatcher = make_bare_dispatcher(fixture_config(), temp.path())
            .await
            .with_tui_bridge(TuiBridge::new(hub.sender()));

        // Two tries of two 90 s calls each: four calls are too few.
        dispatcher.note_rung_speed("p1", "T1", &try_on_mid("T1", 180_000, 2));
        dispatcher.note_rung_speed("p1", "T2", &try_on_mid("T2", 180_000, 2));
        assert!(slow_rungs(&mut events).is_empty());

        dispatcher.note_rung_speed("p1", "T3", &try_on_mid("T3", 180_000, 2));
        assert_eq!(
            slow_rungs(&mut events),
            [("mid".to_string(), "slow_calls".to_string(), Some(90.0))]
        );

        dispatcher.note_rung_speed("p1", "T4", &try_on_mid("T4", 180_000, 2));
        dispatcher.rung_reached_time_bound("p1", "T5", 1, "mid", "glm-4.7", 900);
        assert!(
            slow_rungs(&mut events).is_empty(),
            "mid was said to run slow"
        );

        dispatcher.rung_reached_time_bound("p1", "T6", 2, "cheap", "gpt-oss-120b", 450);
        assert_eq!(
            slow_rungs(&mut events),
            [("cheap".to_string(), "time_bound".to_string(), None)]
        );

        // Quick calls are not slow.
        let quick = make_bare_dispatcher(fixture_config(), temp.path())
            .await
            .with_tui_bridge(TuiBridge::new(hub.sender()));
        quick.note_rung_speed("p1", "T7", &try_on_mid("T7", 60_000, 12));
        assert!(slow_rungs(&mut events).is_empty());
    }
}
