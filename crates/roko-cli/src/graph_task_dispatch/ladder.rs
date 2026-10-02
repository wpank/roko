//! Verify-then-escalate on the model ladder (gap-460230).
//!
//! A task that `[routing.ladder]` routes climbs one runnable rung after
//! [`FAILURES_PER_RUNG`] agent-blamed failures on the rung it runs on (a
//! failed verify step, a turn-cap stop, or a timeout after the first token),
//! at most [`MAX_ESCALATIONS`] times and never past its top rung. Provider and
//! harness failures never count; they stay with provider failover. A pinned
//! model (`--model`, a task's `model_hint`, express routing) never moves, and
//! the dispatch log says so.
//!
//! Every attempt's verdict records its rung, the rungs the task climbed and
//! why it ran there ([`AttemptLadder`]). The task's standing is kept beside
//! its retry feedback ([`LadderStanding`]), so a resumed run climbs from where
//! the last one stopped. A task whose last attempt fails on its top rung has
//! exhausted the ladder: dispatch logs `ladder_exhausted` and tells the
//! dashboard to split or replan the task. Nothing splits it automatically.

use roko_core::dashboard_snapshot::{DiagnosisSeverity, DiagnosisSummary};
use roko_learn::telemetry::{AttemptLadder, AttemptVerdictRecord, Blame, LadderReason};

use super::attempt::AttemptContext;
use super::retry_feedback::LadderStanding;
use super::*;
use crate::dispatch::{RoutingLadder, RunnerDispatchPlan};

/// Agent-blamed failures on one rung before the task climbs to the next.
const FAILURES_PER_RUNG: u32 = 2;

/// Most rungs a task climbs above its start rung (`K_max`).
const MAX_ESCALATIONS: u32 = 2;

/// Retry budget of a task that does not author `max_retries` while the
/// ladder routes it: [`FAILURES_PER_RUNG`] attempts on its start rung and on
/// each rung it may climb, less the first attempt.
const LADDER_MIN_RETRIES: u32 = FAILURES_PER_RUNG * (MAX_ESCALATIONS + 1) - 1;

impl GraphTaskDispatcher {
    /// `[routing.ladder]` as dispatch binds it: `None` when it is off or no
    /// rung can run here.
    fn routing_ladder(&self) -> Option<&RoutingLadder> {
        self.factory.dispatcher().routing_ladder()
    }

    /// Least retry budget of a task that does not author `max_retries`:
    /// enough to climb the ladder while it routes tasks (`--model` stops
    /// that), else none.
    pub(super) fn ladder_min_retries(&self) -> u32 {
        if self.cli_model_override.is_none() && self.routing_ladder().is_some() {
            LADDER_MIN_RETRIES
        } else {
            0
        }
    }

    /// Rungs `task` climbed above its start rung, where its next attempt
    /// routes.
    pub(super) fn ladder_step(&self, spec: &TaskExecutionSpec, task: &TaskDef) -> u32 {
        self.gate_retry_context
            .ladder_standing(&spec.plan_id, &task.id)
            .escalations
    }

    /// Record on `attempt` where `plan` put it on the ladder, `step` rungs
    /// above the task's start rung, and log it. An attempt the ladder did not
    /// route (it is off, or no rung of the task can run) records nothing.
    pub(super) fn record_attempt_ladder(
        &self,
        attempt: &mut AttemptContext,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        plan: &RunnerDispatchPlan,
        step: u32,
    ) {
        let Some(ladder) = self.routing_ladder() else {
            return;
        };
        let role = task.role.as_deref().unwrap_or("implementer");
        let (record, last_chance) = match plan.source {
            ModelChoiceSource::Ladder { rung } => {
                // A `rung` hint that names one of the task's rungs replaced
                // its start rung (gap-dbf2a6).
                let hinted = task
                    .hints
                    .rung
                    .as_deref()
                    .is_some_and(|hint| ladder.has_rung(role, hint));
                let reason = if step > 0 {
                    LadderReason::Escalated
                } else if hinted {
                    LadderReason::Hint
                } else {
                    LadderReason::Start
                };
                let record = AttemptLadder {
                    rung: ladder.rung_name(role, rung).map(str::to_string),
                    index: u32::try_from(rung).ok(),
                    step,
                    reason,
                    exhausted: false,
                    // The cascade router's shadow pick beside the rung (G56).
                    router_pick: plan
                        .route_decision
                        .as_ref()
                        .and_then(|decision| decision.proposals.learned.clone()),
                };
                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    model = %plan.model.slug,
                    rung = record.rung.as_deref().unwrap_or("-"),
                    step,
                    reason = ?reason,
                    "attempt routed on the model ladder"
                );
                // The task's last attempt, with no rung left to climb.
                let top = step >= MAX_ESCALATIONS || ladder.runnable_above(role, rung) == 0;
                let task_key = format!("{}/{}", spec.plan_id, task.id);
                (
                    record,
                    top && self.attempt_in_run(&task_key) >= spec.max_retries,
                )
            }
            ModelChoiceSource::Override | ModelChoiceSource::TaskHint => {
                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    model = %plan.model.slug,
                    source = ?plan.source,
                    "the model is pinned; the ladder never escalates it"
                );
                let pinned = AttemptLadder {
                    rung: None,
                    index: None,
                    step: 0,
                    reason: LadderReason::Pinned,
                    exhausted: false,
                    router_pick: None,
                };
                (pinned, false)
            }
            ModelChoiceSource::Router
            | ModelChoiceSource::Explore
            | ModelChoiceSource::Fallback { .. }
            | ModelChoiceSource::Default => return,
        };
        attempt.record_ladder(record, last_chance);
    }

    /// Count `settled`, an attempt of `task`, toward the task's standing on
    /// the ladder. A pass clears the standing. The agent's failure on the rung
    /// the ladder put it on counts, and the second such failure moves the
    /// task one runnable rung up, unless it already climbed
    /// [`MAX_ESCALATIONS`] rungs or stands on its top rung. Other failures,
    /// pinned attempts, and attempts a failover substitute ran in place of
    /// the rung's model (backlog 1118), change nothing.
    pub(super) fn note_ladder_outcome(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        settled: &SettledAttempt,
    ) {
        if settled.succeeded() {
            self.gate_retry_context
                .clear_ladder(&spec.plan_id, &task.id);
            return;
        }
        let verdict = &settled.verdict;
        let Some(ladder) = verdict.ladder.as_ref() else {
            return;
        };
        if verdict.blame != Blame::Agent || ladder.reason == LadderReason::Pinned {
            return;
        }
        if !verdict.executed.failover_chain.is_empty() {
            tracing::debug!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                rung = ladder.rung.as_deref().unwrap_or("-"),
                substitute = verdict.executed.model_dispatched.as_deref().unwrap_or("-"),
                "a substitute ran; the rung's standing is unchanged"
            );
            return;
        }
        let Some(routing) = self.routing_ladder() else {
            return;
        };
        let role = task.role.as_deref().unwrap_or("implementer");
        let can_climb = ladder
            .index
            .and_then(|index| usize::try_from(index).ok())
            .is_some_and(|index| routing.runnable_above(role, index) > 0);
        let mut standing = self
            .gate_retry_context
            .ladder_standing(&spec.plan_id, &task.id);
        standing.failures_on_rung = standing.failures_on_rung.saturating_add(1);
        if standing.failures_on_rung >= FAILURES_PER_RUNG
            && standing.escalations < MAX_ESCALATIONS
            && can_climb
        {
            standing = LadderStanding {
                escalations: standing.escalations.saturating_add(1),
                failures_on_rung: 0,
            };
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                rung = ladder.rung.as_deref().unwrap_or("-"),
                escalations = standing.escalations,
                "the task failed twice on its rung; its next attempt runs one rung up the ladder"
            );
        }
        self.gate_retry_context
            .set_ladder_standing(&spec.plan_id, &task.id, standing);
        if ladder.exhausted {
            self.report_ladder_exhausted(spec, task, verdict, ladder);
        }
    }

    /// Log that `task` exhausted the ladder on `ladder`'s rung, and tell the
    /// dashboard to split or replan it.
    fn report_ladder_exhausted(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        verdict: &AttemptVerdictRecord,
        ladder: &AttemptLadder,
    ) {
        let rung = ladder.rung.as_deref().unwrap_or("-");
        let model = verdict.executed.model_requested.as_deref().unwrap_or("-");
        tracing::warn!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            rung,
            model,
            escalations = ladder.step,
            reason = "ladder_exhausted",
            "the task failed on its top ladder rung with no retries left; split or replan it"
        );
        if let Some(tui) = &self.tui_bridge {
            tui.diagnosis(DiagnosisSummary {
                id: format!("ladder_exhausted:{}/{}", spec.plan_id, task.id),
                severity: DiagnosisSeverity::Warn,
                subject: format!("{} exhausted the model ladder", task.id),
                detail: format!(
                    "Task `{}` of plan `{}` failed its checks on rung `{rung}` ({model}), the top \
                     rung it can climb, and has no retries left (ladder_exhausted).",
                    task.id, spec.plan_id
                ),
                suggested_action: Some("Split the task into smaller tasks, or replan it.".into()),
                ..DiagnosisSummary::default()
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use roko_core::config::routing::LadderRung;
    use roko_graph::cells::NoopAttemptRecorder;
    use roko_learn::cascade_router::CascadeRouter;
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, cli_provider, make_scripted_batch_dispatcher, make_spec, model,
        no_auto_fix, verify_step,
    };
    use crate::state_hub::StateHub;

    const RUN: &str = "ladder-run";
    const CHEAP: &str = "claude-haiku-4-5";
    const TOP: &str = "claude-sonnet-4-6";

    /// Fake Claude CLI that records the model of each call. While
    /// `fail-once` exists beside it, a call removes it and fails as the
    /// provider, before any output.
    const PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
dir=$(dirname -- "$0")
previous=
for arg in "$@"; do
  if [ "$previous" = "--model" ]; then
    printf '%s\n' "$arg" >> "$dir/provider-models"
  fi
  previous=$arg
done
if [ -f "$dir/fail-once" ]; then
  rm "$dir/fail-once"
  exit 1
fi
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"ladder-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-l","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    fn rung(name: &str, model: &str) -> LadderRung {
        LadderRung {
            name: name.to_string(),
            model: model.to_string(),
        }
    }

    /// Batch fixture with a two-rung ladder that starts mechanical tasks on
    /// [`CHEAP`], and an unhinted mechanical task whose verify step always
    /// fails. Helper model calls go to their own provider, so [`PROVIDER`]
    /// records only the attempts.
    async fn ladder_fixture(temp: &tempfile::TempDir) -> (GraphTaskDispatcher, TaskDef) {
        let helper = temp.path().join("helper.sh");
        std::fs::write(&helper, VERIFY_PROVIDER).expect("write helper provider");
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755))
            .expect("make helper executable");
        let (dispatcher, mut task) = make_scripted_batch_dispatcher(temp, PROVIDER, |config| {
            no_auto_fix(config);
            config
                .models
                .insert("cheap-model".to_string(), model("batch-cli", CHEAP, None));
            config.routing.ladder.rungs =
                vec![rung("cheap", "cheap-model"), rung("top", "batch-model")];
            config.providers.insert(
                "helper-cli".to_string(),
                cli_provider(&helper.display().to_string()),
            );
            config.models.insert(
                "helper-model".to_string(),
                model("helper-cli", "helper-slug", None),
            );
            config.routing.fast_task_model = "helper-model".to_string();
        })
        .await;
        task.model_hint = None;
        task.tier = "mechanical".to_string();
        task.verify = vec![verify_step("test", "false")];
        (dispatcher, task)
    }

    /// The model of each call [`PROVIDER`] recorded, in order.
    fn called_models(temp: &tempfile::TempDir) -> Vec<String> {
        std::fs::read_to_string(temp.path().join("provider-models"))
            .expect("the provider recorded its calls")
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// Blame, rung, ladder reason and exhaustion of a verdict line.
    fn place(verdict: &serde_json::Value) -> (&str, &str, &str, bool) {
        let ladder = &verdict["ladder"];
        (
            verdict["blame"].as_str().unwrap_or_default(),
            ladder["rung"].as_str().unwrap_or_default(),
            ladder["reason"].as_str().unwrap_or_default(),
            ladder["exhausted"].as_bool().unwrap_or_default(),
        )
    }

    /// Two agent-blamed failures on a rung move the task one rung up, and a
    /// provider failure does not count. On its top rung the task stays, and
    /// its last failed attempt there exhausts the ladder once. A pinned task
    /// never moves, and a `rung` hint picks the start rung.
    #[tokio::test]
    async fn two_failed_attempts_escalate_one_rung() {
        let temp = tempdir().expect("tempdir");
        let runs_dir = temp.path().join(".roko/runs");
        let hub = StateHub::new(64);
        let (dispatcher, task) = ladder_fixture(&temp).await;
        let dispatcher = dispatcher
            .with_feedback(GraphFeedbackContext {
                runs_dir: Some(runs_dir.clone()),
                ..GraphFeedbackContext::default()
            })
            .with_tui_bridge(TuiBridge::new(hub.sender()));
        assert_eq!(dispatcher.ladder_min_retries(), 5);
        let mut spec = make_spec(&task);
        spec.max_retries = 4;
        let mut pinned = task.clone();
        pinned.id = "T-PIN".to_string();
        pinned.model_hint = Some("cheap-model".to_string());
        let pinned_spec = make_spec(&pinned);
        let mut hinted = task.clone();
        hinted.id = "T-HINT".to_string();
        hinted.hints.rung = Some("top".to_string());
        let mut hinted_spec = make_spec(&hinted);
        hinted_spec.max_retries = 1;
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        std::fs::write(temp.path().join("fail-once"), "").expect("fail the first call");
        for _ in 0..=spec.max_retries {
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("every attempt fails");
        }
        for _ in 0..3 {
            dispatcher
                .dispatch(&pinned_spec, Vec::new(), &ctx)
                .await
                .expect_err("every pinned attempt fails");
        }
        dispatcher
            .dispatch(&hinted_spec, Vec::new(), &ctx)
            .await
            .expect_err("the hinted attempt fails");
        assert_eq!(
            called_models(&temp),
            [CHEAP, CHEAP, CHEAP, TOP, TOP, CHEAP, CHEAP, CHEAP, TOP]
        );
        assert_eq!(dispatcher.ladder_step(&spec, &task), 1);
        assert_eq!(dispatcher.ladder_step(&pinned_spec, &pinned), 0);

        dispatcher.close_run_attempts(RUN);
        let verdicts: Vec<serde_json::Value> =
            std::fs::read_to_string(runs_dir.join(RUN).join("attempts.jsonl"))
                .expect("the run's attempt log")
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .filter(|line| line["schema_version"] == "roko.verdict/1")
                .collect();
        let places: Vec<(&str, &str, &str, bool)> = verdicts.iter().map(place).collect();
        assert_eq!(
            places,
            [
                ("infra", "cheap", "start", false),
                ("agent", "cheap", "start", false),
                ("agent", "cheap", "start", false),
                ("agent", "top", "escalated", false),
                ("agent", "top", "escalated", true),
                ("agent", "", "pinned", false),
                ("agent", "", "pinned", false),
                ("agent", "", "pinned", false),
                ("agent", "top", "hint", false),
            ]
        );
        let diagnoses: Vec<String> = hub
            .current_snapshot()
            .diagnoses
            .iter()
            .map(|diagnosis| diagnosis.id.clone())
            .filter(|id| id.starts_with("ladder_exhausted"))
            .collect();
        assert_eq!(diagnoses, ["ladder_exhausted:stream-plan/T-EXP"]);
    }

    /// A settled turn-cap stop of `task` on the cheap rung. `substitute`
    /// names the routed model failover replaced, when a substitute ran.
    fn turn_cap_on_cheap_rung(
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        substitute: Option<&str>,
    ) -> SettledAttempt {
        use roko_learn::telemetry::{AttemptIdentity, AttemptKey, AttemptOutcome};

        let key = AttemptKey::new(RUN, &spec.plan_id, &task.id, 1);
        let identity = AttemptIdentity::new(&key);
        let mut verdict = AttemptVerdictRecord::settle(identity, AttemptOutcome::TurnCap, true);
        verdict.ladder = Some(AttemptLadder {
            rung: Some("cheap".to_string()),
            index: Some(0),
            step: 0,
            reason: LadderReason::Start,
            exhausted: false,
            router_pick: None,
        });
        verdict.executed.failover_chain = substitute.map(str::to_string).into_iter().collect();
        SettledAttempt {
            verdict: Arc::new(verdict),
            failure_reason: None,
            reflex_rule: None,
            live_tool_calls: LiveToolCalls::default(),
        }
    }

    /// backlog 1118: an agent-blamed failure of a failover substitute says
    /// nothing about the rung whose model it replaced, so it leaves the
    /// task's standing alone; the same failures on the rung's own model
    /// climb it.
    #[tokio::test]
    async fn substitute_failure_does_not_count_against_routed_rung() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = ladder_fixture(&temp).await;
        let spec = make_spec(&task);
        let substituted = turn_cap_on_cheap_rung(&spec, &task, Some("cheap-model"));
        assert_eq!(substituted.verdict.blame, Blame::Agent);

        for _ in 0..FAILURES_PER_RUNG {
            dispatcher.note_ladder_outcome(&spec, &task, &substituted);
        }
        let standing = dispatcher
            .gate_retry_context
            .ladder_standing(&spec.plan_id, &task.id);
        assert_eq!(standing.failures_on_rung, 0);
        assert_eq!(dispatcher.ladder_step(&spec, &task), 0);

        let own = turn_cap_on_cheap_rung(&spec, &task, None);
        for _ in 0..FAILURES_PER_RUNG {
            dispatcher.note_ladder_outcome(&spec, &task, &own);
        }
        assert_eq!(dispatcher.ladder_step(&spec, &task), 1);
    }

    /// The streaming path climbs the same ladder.
    #[tokio::test]
    async fn two_failed_attempts_escalate_one_rung_when_streaming() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = ladder_fixture(&temp).await;
        let spec = make_spec(&task);
        let lease = TaskLease {
            path: temp.path().to_path_buf(),
            fingerprint: "ladder".to_string(),
        };
        for _ in 0..3 {
            let (events, _received) =
                tokio::sync::mpsc::channel(streaming_event_channel_capacity());
            dispatcher
                .dispatch_streaming(
                    &spec,
                    Vec::new(),
                    &CellContext::new(),
                    &lease,
                    events,
                    &NoopAttemptRecorder,
                )
                .await
                .expect_err("every attempt fails verification");
        }
        assert_eq!(called_models(&temp), [CHEAP, CHEAP, TOP]);
    }

    /// While the ladder routes, an attempt's verdict names the cascade
    /// router's shadow pick beside its rung (G56): the router's own pick its
    /// route row records. Without a cascade router the verdict names none.
    #[tokio::test]
    async fn ladder_attempt_records_router_pick() {
        let temp = tempdir().expect("tempdir");
        let runs_dir = temp.path().join(".roko/runs");
        let feedback = || GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            ..GraphFeedbackContext::default()
        };
        let (plain, task) = ladder_fixture(&temp).await;
        let config = Arc::clone(&plain.config);
        let models = vec![CHEAP.to_string(), TOP.to_string()];
        let router = Arc::new(CascadeRouter::new(models));
        let factory = SharedAgentFactory::new(Arc::clone(&config), None, Some(router), None).await;
        let routed = GraphTaskDispatcher::new(Arc::new(factory), config, temp.path().to_path_buf())
            .with_feedback(feedback());
        let plain = plain.with_feedback(feedback());
        let spec = make_spec(&task);
        for (dispatcher, run) in [(&routed, "routed"), (&plain, "plain")] {
            let ctx = CellContext::new().with_run_id(run.to_string());
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("the attempt fails its verify step");
            dispatcher.close_run_attempts(run);
        }
        let rows = |run: &str, file: &str| -> Vec<serde_json::Value> {
            std::fs::read_to_string(runs_dir.join(run).join(file))
                .expect("the run's log")
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect()
        };
        let verdict = |run: &str| {
            rows(run, "attempts.jsonl")
                .into_iter()
                .find(|row| row["schema_version"] == "roko.verdict/1")
                .expect("the attempt's verdict")
        };

        let routed_verdict = verdict("routed");
        assert_eq!(routed_verdict["ladder"]["reason"], "start");
        let pick = routed_verdict["ladder"]["router_pick"]
            .as_str()
            .expect("the cascade router's shadow pick");
        let decisions = rows("routed", "decisions.jsonl");
        let route = decisions
            .iter()
            .find(|row| row["decision_point"] == "route")
            .expect("the attempt's route row");
        assert_eq!(route["source"], "ladder");
        assert_eq!(route["proposals"]["learned"], pick);

        let plain_verdict = verdict("plain");
        assert_eq!(plain_verdict["ladder"]["reason"], "start");
        assert!(
            plain_verdict["ladder"].get("router_pick").is_none(),
            "{plain_verdict}"
        );
    }
}
