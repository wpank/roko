//! T0 reflex credit from settled attempts (gap-4468bd).
//!
//! A reflex rule that serves an attempt in place of the provider
//! ([`AttemptContext::served_by_reflex`]) learns only from that attempt's
//! settled record, after its verify steps:
//!
//! - a pass credits the rule with a gate pass;
//! - a failure of the agent's work demotes it, and a demotion below the
//!   store's threshold deletes it;
//! - an attempt without a learning label (unverified, provider and harness
//!   outcomes) leaves it as it was.
//!
//! Firing earns a rule nothing.
//!
//! [`AttemptContext::served_by_reflex`]: super::attempt::AttemptContext::served_by_reflex

use super::*;

impl GraphTaskDispatcher {
    /// Credit or demote the T0 reflex rule that served `settled`, if one
    /// did, from the attempt's learning label alone. The store writes its
    /// rules to disk off the reactor.
    pub(super) async fn credit_reflex_rule(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        settled: &SettledAttempt,
    ) {
        let Some(rule_id) = settled.reflex_rule else {
            return;
        };
        let Some(reflexes) = self.reflex_store.clone() else {
            return;
        };
        let Some(passed) = settled.learning_success() else {
            return;
        };
        let deleted = tokio::task::spawn_blocking(move || {
            if passed {
                reflexes.record_gate_pass_for(rule_id);
                false
            } else {
                reflexes.record_gate_fail_for(rule_id)
            }
        })
        .await
        .unwrap_or(false);
        tracing::info!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            attempt_key = settled.attempt_key(),
            %rule_id,
            passed,
            deleted,
            "T0 reflex rule credited from its settled attempt"
        );
    }
}

#[cfg(test)]
mod tests {
    use roko_learn::reflex_store::{PromotionCandidate, ReflexAction, ReflexCondition};
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        batch_ctx, jsonl_rows_where, make_batch_dispatcher, make_spec, no_auto_fix,
        recording_feedback,
    };

    /// A failed verify step, as the verify steps report it.
    fn verify_failure() -> Result<TaskGateVerdict> {
        Err(RokoError::Verify {
            gate: "structural".to_string(),
            message: "exit 1".to_string(),
        })
    }

    /// A T0 reflex rule learns only from the settled record of an attempt
    /// it served. A served task without verify steps settles unverified and
    /// leaves the rule as it was; a verify failure demotes the rule and
    /// earns it no pass; a verified pass credits it with one; the pass of
    /// an attempt it did not serve credits it nothing; and a second failure
    /// deletes the demoted rule.
    #[tokio::test]
    async fn reflex_rule_is_credited_from_the_settled_attempt() {
        let store_dir = tempdir().expect("tempdir");
        let reflexes = ReflexStore::open(store_dir.path().join("reflexes.jsonl"));
        // A wildcard rule matches every task; promotion credits it with three
        // gate passes out of three hits.
        let promoted = reflexes.try_promote(
            &PromotionCandidate {
                episode_id: "episode-reflex".to_string(),
                condition: ReflexCondition::default(),
                action: ReflexAction {
                    tool: "respond".to_string(),
                    args: "cached reflex output".to_string(),
                },
            },
            3,
        );
        assert!(promoted);
        let rule = || reflexes.snapshot().pop().expect("the promoted rule");

        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_batch_dispatcher(&temp, 0.01, |config| {
            no_auto_fix(config);
            config.learning.t0_reflexes = true;
        })
        .await;
        let dispatcher = dispatcher
            .with_feedback(recording_feedback(temp.path()))
            .with_reflex_store(reflexes.clone());
        let spec = make_spec(&task);

        // The rule serves a task without verify steps. Its attempt settles
        // unverified, with no learning label, so the rule keeps its passes.
        let outputs = dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect("the reflex serves the task");
        let text = outputs[0].body.as_text().expect("output text");
        assert_eq!(text, "cached reflex output");
        let attempts = temp
            .path()
            .join(".roko/runs")
            .join(dispatcher.attempts.fallback_run_id())
            .join("attempts.jsonl");
        let is_verdict = |row: &serde_json::Value| row["schema_version"] == "roko.verdict/1";
        let verdicts = jsonl_rows_where(&attempts, 1, is_verdict).await;
        assert_eq!(verdicts[0]["outcome"], "unverified");
        assert!(verdicts[0]["learning_label"].is_null(), "{}", verdicts[0]);
        assert_eq!((rule().hit_count, rule().success_count), (4, 3));

        // An attempt the rule serves, settled with `verification`.
        let served = |verification: Result<TaskGateVerdict>| {
            let matched = reflexes
                .match_observation_with_id(&ReflexObservation::default())
                .expect("the rule matches");
            let mut attempt = dispatcher.open_attempt(&spec, &task, &batch_ctx());
            attempt.served_by_reflex(matched.rule_id);
            attempt.settle(Settlement::verified(&verification), "", None)
        };

        // A verify failure halves the rule's confidence and earns no pass.
        let failed = served(verify_failure());
        assert_eq!(failed.learning_success(), Some(false));
        dispatcher.publish_settlement(&spec, &task, &failed).await;
        let demoted = rule();
        assert_eq!((demoted.hit_count, demoted.success_count), (5, 3));
        assert!((demoted.confidence - 0.5).abs() < 1e-9, "{demoted:?}");

        // A verified pass credits it with one gate pass.
        let passed = served(Ok(TaskGateVerdict::Passed));
        assert_eq!(passed.learning_success(), Some(true));
        dispatcher.publish_settlement(&spec, &task, &passed).await;
        assert_eq!((rule().hit_count, rule().success_count), (6, 4));

        // A pass the rule did not serve credits it nothing.
        let pass = Ok(TaskGateVerdict::Passed);
        let other = dispatcher.open_attempt(&spec, &task, &batch_ctx());
        let other = other.settle(Settlement::verified(&pass), "", None);
        dispatcher.publish_settlement(&spec, &task, &other).await;
        assert_eq!((rule().hit_count, rule().success_count), (6, 4));

        // A second failure takes its confidence below one half, and the
        // store deletes the rule.
        let again = served(verify_failure());
        dispatcher.publish_settlement(&spec, &task, &again).await;
        assert!(reflexes.is_empty(), "{:?}", reflexes.snapshot());
    }
}
