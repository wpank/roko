//! Decision records of Graph task dispatch (S01 §4.5, §5.3).
//!
//! An attempt that reaches routing writes one route decision row to its
//! run's `decisions.jsonl`: the row [`ModelRouter::decide`] made when
//! dispatch planned the attempt, keyed to the attempt. A T0 reflex attempt
//! and a harness failure before planning write none.
//!
//! [`ModelRouter::decide`]: crate::dispatch::ModelRouter::decide

use super::attempt::AttemptContext;
use super::*;
use crate::dispatch::RunnerDispatchPlan;

impl GraphTaskDispatcher {
    /// Record what planning decided for `attempt` (S01 P0-8): `plan`'s route
    /// decision, keyed to the attempt (its trace id too) and stamped with
    /// `task`'s id and the time it is written.
    pub(super) fn record_planned_attempt(
        &self,
        attempt: &AttemptContext,
        task: &TaskDef,
        plan: &RunnerDispatchPlan,
    ) {
        let Some(mut decision) = plan.route_decision.clone() else {
            return;
        };
        let attempt_key = attempt.key.attempt_key();
        decision.trace_id.clone_from(&attempt_key);
        decision.attempt_key = Some(attempt_key);
        decision.task_id.clone_from(&task.id);
        decision.timestamp = chrono::Utc::now().to_rfc3339();
        attempt.record_decision(decision);
    }
}

#[cfg(test)]
mod tests {
    use roko_fs::layout::RokoLayout;
    use roko_learn::telemetry::DecisionSource;
    use roko_learn::telemetry::report::{LegacyRows, RunRecords, check};
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, make_spec, make_test_dispatcher, no_auto_fix, verify_step,
    };
    use crate::runtime_feedback::EpisodeSink;

    const RUN: &str = "graph-decision-run";

    /// One dispatched attempt writes one route decision row to its run's
    /// `decisions.jsonl`, keyed to the attempt and written before its
    /// verdict, and the run passes `roko learn telemetry check`.
    #[tokio::test]
    async fn graph_route_writes_decision_row() {
        let temp = tempdir().expect("tempdir");
        let roko = temp.path().join(".roko");
        let episodes_path = roko.join("episodes.jsonl");
        let facade = FeedbackFacade::new().with_sink(Arc::new(EpisodeSink::at(&episodes_path)));
        let feedback = GraphFeedbackContext {
            feedback_facade: Some(Arc::new(facade)),
            efficiency_path: Some(roko.join("learn/efficiency.jsonl")),
            costs_path: Some(roko.join("learn/costs.jsonl")),
            runs_dir: Some(roko.join("runs")),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        task.verify = vec![verify_step("structural", "true")];
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the verified attempt passes");
        // Closing the run's writer flushes its lines.
        drop(dispatcher);
        crate::background_writes::settled(&roko).await;

        let run = RunRecords::load(&roko.join("runs").join(RUN)).expect("load the run");
        assert_eq!(run.decisions.len(), 1, "one route row per attempt");
        assert_eq!(run.verdicts.len(), 1);
        let (decision, verdict) = (&run.decisions[0], &run.verdicts[0]);
        let key = verdict.record.identity.attempt_key.as_str();
        assert_eq!(decision.record.attempt_key.as_deref(), Some(key));
        assert_eq!(decision.record.trace_id, key);
        assert_eq!(decision.record.task_id, task.id);
        assert!(decision.seq < verdict.seq, "decided before settled");
        // The task's model hint picked its model.
        assert_eq!(decision.record.source, Some(DecisionSource::TaskHint));
        assert_eq!(decision.record.selected_model, "stream-model");
        let total: f64 = decision.record.candidates.iter().filter_map(|c| c.p).sum();
        assert!((total - 1.0).abs() < 1e-9, "candidate p sums to {total}");

        let legacy = LegacyRows::load(&RokoLayout::new(roko), RUN).expect("legacy rows");
        let report = check(&run, &legacy);
        assert_eq!(report.failures(), Vec::<String>::new());
        assert_eq!(report.decisions, 1);
    }
}
