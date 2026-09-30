//! Attempt identity and settlement of Graph task dispatch (S01 §4.2, §4.3).
//!
//! Each dispatch that reaches prompt assembly opens one attempt
//! ([`GraphTaskDispatcher::open_attempt`]): it mints the attempt's durable
//! [`AttemptKey`] and writes the `roko.attempt_open/1` line before the prompt
//! is assembled. The attempt then settles once ([`AttemptContext::settle`]
//! consumes it): its `roko.verdict/1` record goes to the run's
//! `attempts.jsonl`, and dispatch publishes it through the feedback facade as
//! [`FeedbackEvent::AttemptSettled`]. The attempt's efficiency, cost and
//! episode rows carry the same key.
//!
//! Ordinals are 1-based and durable. A run's attempts of a task continue from
//! the highest ordinal its `attempts.jsonl` holds, so a resumed run never
//! reuses a key; an attempt whose process died after its open line has no
//! verdict, counts as abandoned, and keeps its ordinal. An attempt the
//! harness fails after its open line, in prompt assembly or in its
//! cost-ledger write, settles as `harness_error`
//! ([`GraphTaskDispatcher::fail_attempt`]).
//!
//! Learners read only the verdict's learning label
//! ([`SettledAttempt::learning_success`]): an attempt without one updates no
//! learner.

use roko_learn::telemetry::records::b3_digest;
use roko_learn::telemetry::{
    AttemptFailureClass, AttemptIdentity, AttemptKey, AttemptOpenRecord, AttemptOrdinals,
    AttemptTiming, AttemptVerdictRecord, CostSource, ExecutedModel, GateVerdictTag,
    HelperCallsUsage, TelemetryEvent, TelemetryWriter, TelemetryWriterConfig, TelemetryWriterStats,
};
use sha2::Digest;

use super::failover::FailoverChain;
use super::served_model::{ServedModel, is_cli_backend};
use super::*;

/// Attempt state of one run: its durable ordinals and its telemetry writer.
struct RunAttempts {
    ordinals: AttemptOrdinals,
    /// `None` when the dispatcher keeps no run files, or the writer did not
    /// start.
    writer: Option<TelemetryWriter>,
}

impl RunAttempts {
    /// Open run `run_id` under `runs_dir` (`.roko/runs`): recover its
    /// ordinals from `attempts.jsonl` and start its writer. Without a
    /// `runs_dir` the ordinals live in memory and nothing is written.
    fn open(runs_dir: Option<&Path>, run_id: &str) -> Self {
        let Some(run_dir) = runs_dir.map(|dir| dir.join(run_id)) else {
            return Self {
                ordinals: AttemptOrdinals::default(),
                writer: None,
            };
        };
        let ordinals = AttemptOrdinals::load(&run_dir).unwrap_or_else(|error| {
            tracing::warn!(
                run_dir = %run_dir.display(),
                %error,
                "attempt log unreadable; this run's attempt ordinals restart at 1"
            );
            AttemptOrdinals::default()
        });
        let writer = match TelemetryWriter::spawn(&run_dir, TelemetryWriterConfig::default()) {
            Ok(writer) => Some(writer),
            Err(error) => {
                tracing::warn!(
                    run_dir = %run_dir.display(),
                    %error,
                    "attempt telemetry writer did not start; this run's attempts are not recorded"
                );
                None
            }
        };
        Self { ordinals, writer }
    }

    /// Queue `record` without waiting; the writer counts what it drops.
    fn submit(&self, record: impl Into<TelemetryEvent>) {
        if let Some(writer) = &self.writer
            && !writer.submit(record)
        {
            tracing::debug!("attempt telemetry record dropped: the writer's channel is full");
        }
    }

    /// Close the writer, wait for its queued lines, and return its final
    /// counters; `None` when it never started or is already closed.
    fn close(&mut self) -> Option<TelemetryWriterStats> {
        let stats = self.writer.take()?.close();
        if stats.dropped > 0 || stats.write_errors > 0 {
            tracing::warn!(
                dropped = stats.dropped,
                write_errors = stats.write_errors,
                "attempt telemetry lost records"
            );
        }
        Some(stats)
    }
}

impl Drop for RunAttempts {
    /// Close the writer and wait for its queued lines, so a run's last
    /// verdicts reach disk before the process exits.
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// Every run's attempt state, opened on the run's first attempt in this
/// process.
pub(super) struct AttemptBook {
    /// Run id of attempts whose cell context names no run: one per
    /// dispatcher, so keys stay unique across processes.
    fallback_run_id: String,
    runs: parking_lot::Mutex<HashMap<String, Arc<RunAttempts>>>,
}

impl Default for AttemptBook {
    fn default() -> Self {
        Self {
            fallback_run_id: format!("graph-{}", uuid::Uuid::new_v4().simple()),
            runs: parking_lot::Mutex::new(HashMap::new()),
        }
    }
}

impl AttemptBook {
    /// The run an attempt dispatched with `ctx` belongs to: the Graph
    /// checkpoint's run the engine names, else this dispatcher's own.
    pub(super) fn run_id<'a>(&'a self, ctx: &'a CellContext) -> &'a str {
        ctx.run_id
            .as_deref()
            .filter(|run_id| !run_id.is_empty())
            .unwrap_or(&self.fallback_run_id)
    }

    /// This dispatcher's own run id, for attempts outside a Graph run.
    #[cfg(test)]
    pub(super) fn fallback_run_id(&self) -> &str {
        &self.fallback_run_id
    }

    fn run(&self, runs_dir: Option<&Path>, run_id: &str) -> Arc<RunAttempts> {
        let mut runs = self.runs.lock();
        let run = runs
            .entry(run_id.to_string())
            .or_insert_with(|| Arc::new(RunAttempts::open(runs_dir, run_id)));
        Arc::clone(run)
    }

    /// Close run `run_id`'s writer once no attempt of the run is open, and
    /// return its final counters. `None` when this process opened no attempt
    /// of the run, or one is still open: that attempt keeps the writer until
    /// it settles, and dropping the last of them closes it.
    fn close(&self, run_id: &str) -> Option<TelemetryWriterStats> {
        let mut run = {
            let mut runs = self.runs.lock();
            let run = runs.remove(run_id)?;
            match Arc::try_unwrap(run) {
                Ok(run) => run,
                Err(run) => {
                    runs.insert(run_id.to_string(), run);
                    return None;
                }
            }
        };
        run.close()
    }

    /// Open the next attempt of `task` in run `run_id`: mint its key and
    /// write its `attempt_open` line under `runs_dir`, when there is one.
    pub(super) fn open(
        &self,
        runs_dir: Option<&Path>,
        run_id: &str,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        node_id: Option<&str>,
    ) -> AttemptContext {
        let run = self.run(runs_dir, run_id);
        let plan_id = if spec.plan_id.is_empty() {
            "-"
        } else {
            spec.plan_id.as_str()
        };
        let key = run.ordinals.mint(run_id, plan_id, &task.id);
        let mut identity = AttemptIdentity::new(&key);
        identity.node_id = node_id.map(str::to_string);
        let task_spec_hash = b3_digest(spec.task_def_json.as_bytes());
        let started_at = now_ms();
        let mut open = AttemptOpenRecord::new(identity.clone(), started_at);
        open.task_spec_hash = Some(task_spec_hash.clone());
        open.role = Some(task.role.as_deref().unwrap_or("implementer").to_string());
        open.max_retries = Some(spec.max_retries);
        run.submit(open);
        AttemptContext {
            key,
            identity,
            task_spec_hash,
            timing: AttemptTiming {
                attempt_started_at: Some(started_at),
                ..AttemptTiming::default()
            },
            failover: FailoverChain::default(),
            helpers: None,
            run,
        }
    }
}

/// One open attempt: its key, and what its verdict records besides the
/// outcome.
pub(super) struct AttemptContext {
    /// The attempt's durable key.
    pub(super) key: AttemptKey,
    identity: AttemptIdentity,
    task_spec_hash: String,
    timing: AttemptTiming,
    /// The models provider failover passed over.
    failover: FailoverChain,
    /// The attempt's helper model calls, once they settled.
    helpers: Option<HelperCallsUsage>,
    run: Arc<RunAttempts>,
}

impl AttemptContext {
    /// Prompt assembly finished.
    pub(super) fn prompt_assembled(&mut self) {
        self.timing.prompt_assembled_at = Some(now_ms());
    }

    /// The provider call starts.
    pub(super) fn dispatch_started(&mut self) {
        self.timing.dispatch_started_at = Some(now_ms());
    }

    /// The provider call returned.
    pub(super) fn dispatch_ended(&mut self) {
        self.timing.dispatch_ended_at = Some(now_ms());
    }

    /// Provider failover passed over `failover`'s models before the one
    /// that ran (bug-35379d).
    pub(super) fn record_failover(&mut self, failover: FailoverChain) {
        self.failover = failover;
    }

    /// The attempt's helper model calls settled with `usage` (bug-62e3f4).
    pub(super) fn record_helper_calls(&mut self, usage: HelperCallsUsage) {
        self.helpers = (usage.calls > 0).then_some(usage);
    }

    /// Settle the attempt: build its verdict record, queue it for the run's
    /// `attempts.jsonl`, and return it with the inputs the feedback sinks
    /// read. `model_requested` is the model dispatch asked for, empty when
    /// the attempt failed before routing; `dispatch` is the provider call's
    /// result, when there was one. Settling consumes the attempt, so it
    /// settles once.
    pub(super) fn settle(
        self,
        settlement: Settlement,
        model_requested: &str,
        dispatch: Option<&crate::dispatch_v2::AgentResultDispatch>,
    ) -> SettledAttempt {
        let Settlement {
            outcome,
            gate_verdict,
            first_token_seen,
            failure_reason,
            rung,
        } = settlement;
        let mut verdict = AttemptVerdictRecord::settle(self.identity, outcome, first_token_seen);
        verdict.task_spec_hash = Some(self.task_spec_hash);
        verdict.gate_verdict = gate_verdict;
        verdict.failure_class = failure_class(outcome, failure_reason.as_deref(), rung);
        verdict.timing = self.timing;
        // Neither path sees the first token's time yet (S01 P0-5).
        verdict.timing.ttft_source = Some("unavailable".to_string());
        verdict.timing.settled_at = Some(now_ms());
        verdict.executed = executed_model(model_requested, dispatch, self.failover);
        verdict.cost.source = cost_source(dispatch);
        verdict.helpers = self.helpers;
        verdict.output_sha256 = dispatch
            .and_then(|dispatch| dispatch.result.output.body.as_text().ok())
            .map(sha256_hex);
        self.run.submit(verdict.clone());
        SettledAttempt {
            verdict: Arc::new(verdict),
            failure_reason,
        }
    }
}

/// What settled an attempt, before it becomes a verdict record (S01 §4.3).
pub(super) struct Settlement {
    outcome: AttemptOutcome,
    gate_verdict: Option<GateVerdictTag>,
    /// Whether the agent produced output: after its first token a timeout is
    /// the agent's, before it the provider's.
    first_token_seen: bool,
    failure_reason: Option<String>,
    /// The check that failed the attempt, when it names one:
    /// `pre_verify:<check>` for the pre-verify screen (`red_flags`).
    rung: Option<String>,
}

impl Settlement {
    /// The verify steps' verdict on a successful provider call. An attempt
    /// the pre-verify screen rejected is a verify failure too: the agent's,
    /// with the screen's check as its rung.
    pub(super) fn verified(verification: &Result<TaskGateVerdict>) -> Self {
        match verification {
            Ok(verdict) => {
                let tag = gate_verdict_tag(*verdict);
                Self {
                    outcome: tag.into(),
                    gate_verdict: Some(tag),
                    first_token_seen: true,
                    failure_reason: None,
                    rung: None,
                }
            }
            Err(error) => Self {
                outcome: AttemptOutcome::GateFailed,
                gate_verdict: None,
                first_token_seen: true,
                failure_reason: Some(verify_failure_reason(error)),
                rung: match error {
                    RokoError::Verify { gate, .. }
                        if gate.starts_with(red_flags::PRE_VERIFY_GATE_PREFIX) =>
                    {
                        Some(gate.clone())
                    }
                    _ => None,
                },
            },
        }
    }

    /// An unsuccessful provider result, or a provider call that errored,
    /// with its message.
    pub(super) fn provider_failure(message: &str, first_token_seen: bool) -> Self {
        Self {
            outcome: provider_failure_outcome(message),
            gate_verdict: None,
            first_token_seen,
            failure_reason: Some(provider_failure_reason(message)),
            rung: None,
        }
    }

    /// The harness failed the attempt outside its provider call and verify
    /// steps: prompt assembly, or recording its spend in the cost ledger.
    pub(super) fn harness_failure(error: &RokoError) -> Self {
        Self {
            outcome: AttemptOutcome::HarnessError,
            gate_verdict: None,
            first_token_seen: false,
            failure_reason: Some(format!("harness: {error}")),
            rung: None,
        }
    }
}

/// A settled attempt (S01 §4.3): its verdict record, and the inputs the
/// analysis rows (episodes, efficiency, costs) still read.
pub(super) struct SettledAttempt {
    /// The `roko.verdict/1` record, shared with the feedback events.
    pub(super) verdict: Arc<AttemptVerdictRecord>,
    /// Class-prefixed failure reason (`"verify: …"`) for episodes and
    /// prompt-experiment settlement.
    pub(super) failure_reason: Option<String>,
}

impl SettledAttempt {
    /// The attempt's key (`"{run}:{plan}:{task}:{attempt}"`).
    pub(super) fn attempt_key(&self) -> &str {
        &self.verdict.identity.attempt_key
    }

    /// The attempt's key, parsed.
    pub(super) fn key(&self) -> AttemptKey {
        self.verdict.identity.key()
    }

    /// The success flag the analysis rows record: the provider call
    /// succeeded and no verify step failed. It counts an unverified attempt
    /// as a success, so no learner reads it; learners read
    /// [`Self::learning_success`].
    pub(super) fn succeeded(&self) -> bool {
        matches!(
            self.verdict.outcome,
            AttemptOutcome::Passed | AttemptOutcome::Unverified | AttemptOutcome::ForcedAccept
        )
    }

    /// What learners record (S01 §4.1): a pass (`Some(true)`), a failure of
    /// the agent's work (`Some(false)`), or nothing (`None`: unverified,
    /// provider and harness outcomes).
    pub(super) fn learning_success(&self) -> Option<bool> {
        self.verdict.learning_success()
    }
}

impl GraphTaskDispatcher {
    /// Open a new dispatch attempt of `task` (S01 §4.2): count it among this
    /// process's attempts of the task, mint its durable key in the run the
    /// cell context names (see [`AttemptBook::run_id`]), and write its
    /// `attempt_open` line. Dispatch calls it once per attempt, before
    /// prompt assembly.
    pub(super) fn open_attempt(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        ctx: &CellContext,
    ) -> AttemptContext {
        {
            let mut attempts = self.task_attempts.lock();
            let started = attempts
                .entry(format!("{}/{}", spec.plan_id, task.id))
                .or_default();
            *started = started.saturating_add(1);
        }
        self.attempts.open(
            self.feedback.runs_dir.as_deref(),
            self.attempts.run_id(ctx),
            spec,
            task,
            ctx.cell_id.as_deref(),
        )
    }

    /// Close run `run_id`'s attempt log once its plan has finished: wait
    /// until every queued line is on disk, so the run's `attempts.jsonl` is
    /// complete, and return the writer's final counters (the run manifest
    /// records what it dropped). `None` when this process opened no attempt
    /// of the run, or an attempt of it is still in flight. A later attempt
    /// of the run reopens the log where it left off.
    pub fn close_run_attempts(&self, run_id: &str) -> Option<TelemetryWriterStats> {
        self.attempts.close(run_id)
    }

    /// Settle `attempt`, which the harness failed after its open line with
    /// `error` (S01 §4.3), and publish its verdict, so the attempt does not
    /// read as abandoned. `routed` is the model dispatch asked for and the
    /// provider call's result, when the failure came after the call. Returns
    /// `error`, for the caller to return.
    pub(super) async fn fail_attempt(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt: AttemptContext,
        routed: Option<(&str, &crate::dispatch_v2::AgentResultDispatch)>,
        error: RokoError,
    ) -> RokoError {
        let (model_requested, dispatch) =
            routed.map_or(("", None), |(model, dispatch)| (model, Some(dispatch)));
        let settlement = Settlement::harness_failure(&error);
        let settled = attempt.settle(settlement, model_requested, dispatch);
        self.publish_settlement(spec, task, &settled).await;
        error
    }
}

/// Whether the agent produced any output before the call ended.
pub(super) fn first_token_seen(dispatch: &crate::dispatch_v2::AgentResultDispatch) -> bool {
    dispatch.events.iter().any(|event| {
        matches!(
            event,
            roko_agent::AgentRuntimeEvent::MessageDelta { .. }
                | roko_agent::AgentRuntimeEvent::ToolCall { .. }
        )
    })
}

/// The telemetry mirror of the Graph gate tag.
const fn gate_verdict_tag(verdict: TaskGateVerdict) -> GateVerdictTag {
    match verdict {
        TaskGateVerdict::Passed => GateVerdictTag::Passed,
        TaskGateVerdict::Unverified => GateVerdictTag::Unverified,
        TaskGateVerdict::ForcedAccept => GateVerdictTag::ForcedAccept,
    }
}

/// Detail of an outcome other than a pass or an unverified attempt. Only the
/// digest of the failure text is kept.
fn failure_class(
    outcome: AttemptOutcome,
    failure_reason: Option<&str>,
    rung: Option<String>,
) -> Option<AttemptFailureClass> {
    if matches!(
        outcome,
        AttemptOutcome::Passed | AttemptOutcome::Unverified | AttemptOutcome::ForcedAccept
    ) {
        return None;
    }
    let mut class = AttemptFailureClass::new(outcome);
    class.rung = rung;
    class.detail_sha256 = failure_reason.map(sha256_hex);
    Some(class)
}

/// The model that ran: the one the bridge launched, which after failover
/// is not the requested one, and the one the provider reported serving,
/// which is `None` when it named none (bug-31438d). `failover` lists the
/// models passed over first (bug-35379d).
fn executed_model(
    model_requested: &str,
    dispatch: Option<&crate::dispatch_v2::AgentResultDispatch>,
    failover: FailoverChain,
) -> ExecutedModel {
    let mut executed = ExecutedModel {
        model_requested: (!model_requested.is_empty()).then(|| model_requested.to_string()),
        failover_chain: failover.models,
        failover_reason: failover.reason,
        failover_refusals: failover.refusals,
        ..ExecutedModel::default()
    };
    if let Some(dispatch) = dispatch {
        let served = ServedModel::of(dispatch);
        executed.provider = Some(dispatch.target.provider_id.clone());
        executed.model_dispatched = Some(dispatch.target.model_slug.clone());
        executed.model_reported = served.reported;
        executed.models_reported = served.all_reported;
        executed.model_mismatch = served.mismatch;
        executed.turns = dispatch.events.iter().rev().find_map(|event| match event {
            roko_agent::AgentRuntimeEvent::TurnCompleted { num_turns, .. } => *num_turns,
            _ => None,
        });
    }
    executed
}

/// Where the attempt's priced usage came from (S01 §4.4); gap-ad0d39 prices
/// it. CLI agents report their own usage.
fn cost_source(dispatch: Option<&crate::dispatch_v2::AgentResultDispatch>) -> CostSource {
    let Some(dispatch) = dispatch else {
        return CostSource::Unknown;
    };
    let Some(usage) = dispatch.result.usage_obs.as_ref() else {
        return CostSource::Unknown;
    };
    CostSource::from_usage_source(&usage.source, is_cli_backend(dispatch.target.provider_kind))
}

fn sha256_hex(text: &str) -> String {
    format!("{:x}", sha2::Sha256::digest(text.as_bytes()))
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use roko_learn::telemetry::Blame;
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, make_spec, make_task_def, make_test_dispatcher, no_auto_fix, verify_step,
    };
    use crate::runtime_feedback::EpisodeSink;

    const RUN: &str = "graph-stream-plan-run";

    /// The rows of the JSONL file at `path` that `keep` accepts, once at least
    /// `expected` of them have landed from the background writers.
    async fn jsonl_rows_where(
        path: &Path,
        expected: usize,
        keep: impl Fn(&serde_json::Value) -> bool,
    ) -> Vec<serde_json::Value> {
        for _ in 0..600 {
            let rows: Vec<serde_json::Value> = std::fs::read_to_string(path)
                .unwrap_or_default()
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .filter(|row| keep(row))
                .collect();
            if rows.len() >= expected {
                return rows;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        panic!("{expected} rows were not written to {}", path.display());
    }

    /// Every row of the JSONL file at `path`, once at least `expected` have
    /// landed.
    async fn jsonl_rows(path: &Path, expected: usize) -> Vec<serde_json::Value> {
        jsonl_rows_where(path, expected, |_| true).await
    }

    fn field<'a>(rows: &'a [serde_json::Value], name: &str) -> Vec<&'a str> {
        rows.iter()
            .map(|row| row[name].as_str().unwrap_or_default())
            .collect()
    }

    /// One attempt's verdict, efficiency, cost and episode rows all name it
    /// by the same key, minted in the run the Graph engine names.
    #[tokio::test]
    async fn graph_feedback_records_share_attempt_key() {
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

        let key = format!("{RUN}:{}:{}:1", spec.plan_id, task.id);
        let attempts = jsonl_rows(&roko.join("runs").join(RUN).join("attempts.jsonl"), 2).await;
        assert_eq!(
            field(&attempts, "schema_version"),
            ["roko.attempt_open/1", "roko.verdict/1"]
        );
        assert_eq!(
            field(&attempts, "attempt_key"),
            [key.as_str(), key.as_str()]
        );
        let verdict = &attempts[1];
        assert_eq!(verdict["settlement_id"], format!("{key}#settle"));
        assert_eq!(verdict["outcome"], "passed");
        assert_eq!(verdict["gate_verdict"], "passed");
        assert_eq!(verdict["learning_label"], 1);
        assert_eq!(verdict["executed"]["provider"], "stream-cli");
        assert!(verdict["cost"]["source"].is_string(), "{verdict}");

        // The dispatch row's id is the key; the gate-pass row extends it. The
        // provider bridge logs its own `model_call` rows to the same file,
        // under the feedback schema.
        let efficiency = jsonl_rows_where(&roko.join("learn/efficiency.jsonl"), 2, |row| {
            row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA
        })
        .await;
        let mut ids = field(&efficiency, "attempt_id");
        ids.sort_unstable();
        assert_eq!(ids, [key.clone(), format!("{key}/gate-pass")]);
        assert_eq!(
            field(&efficiency, "attempt_key"),
            [key.as_str(), key.as_str()]
        );
        // The bridge's own `model_call` row names the attempt and the model
        // the provider reported (bug-92f655).
        let model_calls = jsonl_rows_where(&roko.join("learn/efficiency.jsonl"), 1, |row| {
            row["kind"] == "model_call"
        })
        .await;
        assert_eq!(field(&model_calls, "attempt_key"), [key.as_str()]);
        assert_eq!(model_calls[0]["model_reported"], "claude-sonnet-4-6");
        let costs = jsonl_rows(&roko.join("learn/costs.jsonl"), 1).await;
        assert_eq!(field(&costs, "attempt_key"), [key.as_str()]);
        let episodes = roko_learn::episode_logger::EpisodeLogger::read_all(&episodes_path)
            .await
            .expect("episodes");
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].extra["attempt_key"], key.as_str());
    }

    /// Provider whose first call hangs until the attempt is killed; later
    /// calls answer at once.
    const HANGS_ONCE_PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
dir=$(dirname -- "$0")
if [ ! -f "$dir/hung-once" ]; then
  : > "$dir/hung-once"
  sleep 30
fi
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"resumed-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-r","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// A run killed after an attempt's open line resumes with a new, higher
    /// ordinal; the killed attempt has no verdict, so it counts as abandoned.
    #[tokio::test]
    async fn graph_attempt_ordinal_survives_resume() {
        let temp = tempdir().expect("tempdir");
        let runs_dir = temp.path().join(".roko/runs");
        let attempts_path = runs_dir.join(RUN).join("attempts.jsonl");
        let feedback = || GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            ..GraphFeedbackContext::default()
        };
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        // The first process opens attempt 1 and dies while its provider runs.
        let (first, task) =
            make_test_dispatcher(&temp, HANGS_ONCE_PROVIDER, no_auto_fix, feedback()).await;
        let spec = make_spec(&task);
        let provider_hung = temp.path().join("hung-once");
        tokio::select! {
            result = first.dispatch(&spec, Vec::new(), &ctx) => {
                panic!("the hung attempt must not settle: {result:?}");
            }
            rows = async {
                while !provider_hung.exists() {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
                jsonl_rows(&attempts_path, 1).await
            } => {
                assert_eq!(field(&rows, "schema_version"), ["roko.attempt_open/1"]);
            }
        }
        drop(first);

        // The resumed process continues the run from the attempts log.
        let (resumed, _) =
            make_test_dispatcher(&temp, HANGS_ONCE_PROVIDER, no_auto_fix, feedback()).await;
        resumed
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the resumed attempt completes");
        drop(resumed);

        let chain = format!("{RUN}:{}:{}", spec.plan_id, task.id);
        let rows = jsonl_rows(&attempts_path, 3).await;
        let lines: Vec<(&str, &str)> = rows
            .iter()
            .map(|row| {
                let schema = row["schema_version"].as_str().unwrap_or_default();
                (schema, row["attempt_key"].as_str().unwrap_or_default())
            })
            .collect();
        let (killed, resumed_key) = (format!("{chain}:1"), format!("{chain}:2"));
        assert_eq!(
            lines,
            [
                ("roko.attempt_open/1", killed.as_str()),
                ("roko.attempt_open/1", resumed_key.as_str()),
                ("roko.verdict/1", resumed_key.as_str()),
            ]
        );
        let settled: Vec<&str> = lines
            .iter()
            .filter(|(schema, _)| *schema == "roko.verdict/1")
            .map(|(_, key)| *key)
            .collect();
        let abandoned: Vec<&str> = lines
            .iter()
            .filter(|(schema, key)| *schema == "roko.attempt_open/1" && !settled.contains(key))
            .map(|(_, key)| *key)
            .collect();
        assert_eq!(abandoned, [killed.as_str()]);
        let seqs: Vec<u64> = rows.iter().filter_map(|row| row["seq"].as_u64()).collect();
        assert_eq!(
            seqs,
            [1, 2, 3],
            "the resumed writer continues the run's seq"
        );
    }

    /// Verify and provider results settle into the outcomes, blame and
    /// learning labels of S01 §4.3, and the pre-S01 success flag still
    /// counts an unverified attempt.
    #[test]
    fn settlements_follow_the_outcome_table() {
        let book = AttemptBook::default();
        let task = make_task_def("focused");
        let spec = make_spec(&task);
        let settle = |settlement: Settlement| {
            book.open(None, "run-1", &spec, &task, None)
                .settle(settlement, "model-a", None)
        };
        let verify_error = RokoError::Verify {
            gate: "graph-verify".to_string(),
            message: "1/1 verify step(s) failed".to_string(),
        };
        let cases = [
            (
                Settlement::verified(&Ok(TaskGateVerdict::Passed)),
                "passed",
                Blame::None,
                Some(1),
                true,
            ),
            (
                Settlement::verified(&Ok(TaskGateVerdict::Unverified)),
                "unverified",
                Blame::None,
                None,
                true,
            ),
            (
                Settlement::verified(&Err(verify_error)),
                "gate_failed",
                Blame::Agent,
                Some(0),
                false,
            ),
            (
                Settlement::provider_failure("error: provider unavailable", false),
                "provider_error",
                Blame::Infra,
                None,
                false,
            ),
            (
                Settlement::harness_failure(&RokoError::Planning(
                    "prompt assembly failed: invalid context file".to_string(),
                )),
                "harness_error",
                Blame::Harness,
                None,
                false,
            ),
        ];
        for (settlement, outcome, blame, label, succeeded) in cases {
            let settled = settle(settlement);
            let verdict = &settled.verdict;
            let wire = serde_json::to_value(verdict.outcome).expect("outcome");
            assert_eq!(wire, outcome);
            assert_eq!(
                (verdict.blame, verdict.learning_label, settled.succeeded()),
                (blame, label, succeeded),
                "{outcome}"
            );
            let learned = label.map(|label| label == 1);
            assert_eq!(settled.learning_success(), learned, "{outcome}");
            assert_eq!(verdict.failure_class.is_some(), !succeeded, "{outcome}");
            assert_eq!(settled.failure_reason.is_some(), !succeeded, "{outcome}");
            assert_eq!(verdict.executed.model_requested.as_deref(), Some("model-a"));
        }
        let keys: Vec<u32> = (0..2)
            .map(|_| {
                settle(Settlement::verified(&Ok(TaskGateVerdict::Passed)))
                    .key()
                    .attempt
            })
            .collect();
        assert_eq!(keys, [6, 7], "each open mints the chain's next ordinal");
    }

    /// An attempt whose prompt cannot be assembled settles as a harness
    /// error with no learning label, so it neither reads as abandoned nor
    /// teaches a learner, and names no model: it never reached routing.
    #[tokio::test]
    async fn a_prompt_assembly_failure_settles_as_a_harness_error() {
        let temp = tempdir().expect("tempdir");
        let runs_dir = temp.path().join(".roko/runs");
        let feedback = GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        // A declared context file that does not exist fails prompt assembly.
        task.context = Some(crate::task_parser::TaskContext {
            read_files: vec![crate::task_parser::ReadFile {
                path: "src/missing.rs".to_string(),
                lines: None,
                why: "context".to_string(),
            }],
            ..crate::task_parser::TaskContext::default()
        });
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        let error = dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("prompt assembly fails");
        assert!(matches!(error, RokoError::Planning(_)), "{error:?}");
        drop(dispatcher);

        let rows = jsonl_rows(&runs_dir.join(RUN).join("attempts.jsonl"), 2).await;
        assert_eq!(
            field(&rows, "schema_version"),
            ["roko.attempt_open/1", "roko.verdict/1"]
        );
        let verdict = &rows[1];
        assert_eq!(verdict["attempt_key"], rows[0]["attempt_key"]);
        assert_eq!(verdict["outcome"], "harness_error");
        assert_eq!(verdict["blame"], "harness");
        assert!(verdict["learning_label"].is_null(), "{verdict}");
        assert_eq!(verdict["failure_class"]["kind"], "harness_error");
        assert!(
            verdict["executed"]["model_requested"].is_null(),
            "{verdict}"
        );
        assert!(
            verdict["timing"]["prompt_assembled_at"].is_null(),
            "{verdict}"
        );
    }

    /// An attempt the pre-verify screen rejected settles as the agent's
    /// failed gate, so no learner credits it, and its rung names the check.
    #[test]
    fn a_pre_verify_rejection_names_its_check_as_the_rung() {
        let book = AttemptBook::default();
        let task = make_task_def("focused");
        let spec = make_spec(&task);
        let rung = |gate: &str| {
            let error = RokoError::Verify {
                gate: gate.to_string(),
                message: "rejected".to_string(),
            };
            let settled = book.open(None, "run-1", &spec, &task, None).settle(
                Settlement::verified(&Err(error)),
                "model-a",
                None,
            );
            let verdict = &settled.verdict;
            assert_eq!(
                (verdict.outcome, verdict.blame, verdict.learning_label),
                (AttemptOutcome::GateFailed, Blame::Agent, Some(0)),
                "{gate}"
            );
            assert!(!settled.succeeded());
            verdict
                .failure_class
                .as_ref()
                .and_then(|class| class.rung.clone())
        };
        assert_eq!(
            rung("pre_verify:no_changes").as_deref(),
            Some("pre_verify:no_changes")
        );
        assert_eq!(rung("graph-verify"), None, "a failed verify step");
    }
}
