//! Attempt identity and settlement of Graph task dispatch (S01 §4.2, §4.3).
//!
//! Each dispatch that reaches prompt assembly opens one attempt
//! ([`GraphTaskDispatcher::open_attempt`]): it mints the attempt's durable
//! [`AttemptKey`] and writes the `roko.attempt_open/1` line before the prompt
//! is assembled. A dispatch that a T0 reflex rule serves in place of the
//! provider opens one as well. The attempt then settles once
//! ([`AttemptContext::settle`] consumes it): its `roko.verdict/1` record goes
//! to the run's `attempts.jsonl`, and dispatch publishes it through the
//! feedback facade as [`FeedbackEvent::AttemptSettled`]. The attempt's
//! efficiency, cost and episode rows carry the same key.
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
//! learner. That includes the T0 reflex rule that served an attempt
//! (`reflex_credit`).

use roko_core::config::experiments::ExperimentsConfig;
use roko_core::config::schema::ProviderBilling;
use roko_core::pricing_snapshot::{PriceSnapshot, PricedUsage, TokenCounts};
use roko_learn::loop_audit::Registry;
use roko_learn::loop_audit::arm_set::{ArmDraws, ArmMode, ArmSet};
use roko_learn::telemetry::records::b3_digest;
use roko_learn::telemetry::records::{
    AttemptCost, AttemptUsage, CacheWriteClass, PlaceboDecisionRecord, VerifyStepVerdict,
};
use roko_learn::telemetry::{
    AttemptFailureClass, AttemptIdentity, AttemptKey, AttemptLadder, AttemptOpenRecord,
    AttemptOrdinals, AttemptTiming, AttemptVerdictRecord, Blame, ContentDecisionRecord, CostSource,
    ExecutedModel, ExposureCounts, ExposureRecord, GateVerdictTag, HelperCallsUsage, LadderReason,
    TelemetryEvent, TelemetryWriter, TelemetryWriterConfig, TelemetryWriterStats,
};
use sha2::Digest;

use super::audit_select::AuditSelector;
use super::failover::FailoverChain;
use super::served_model::{ServedModel, is_cli_backend};
use super::*;

/// `run_seed` of the arm-set draws (`telemetry::assign`): 0 until runs
/// record an experiment seed (S01 `experiment.seed`), as the route's
/// exploration draws do. The chain key names the run.
const ARM_SEED: u64 = 0;

/// Attempt state of one run: its durable ordinals, its telemetry writer and
/// its chains' arm sets.
struct RunAttempts {
    ordinals: AttemptOrdinals,
    /// `None` when the dispatcher keeps no run files, or the writer did not
    /// start.
    writer: Option<TelemetryWriter>,
    /// The run's audit lottery (DP1), with `[audit] enabled`.
    audit: Option<Arc<AuditSelector>>,
    run_id: String,
    /// The UTC day this process draws the run's arm sets for, fixed when it
    /// opened the run, so a retry after midnight keeps its chain's arms.
    epoch: String,
    /// Each chain's arm set (S02.P1-14), drawn on its first attempt in this
    /// process and inherited by its retries.
    arm_sets: parking_lot::Mutex<HashMap<String, Arc<ArmSet>>>,
}

impl RunAttempts {
    /// Open run `run_id` under `runs_dir` (`.roko/runs`): recover its
    /// ordinals from `attempts.jsonl` and start its writer. Without a
    /// `runs_dir` the ordinals live in memory and nothing is written.
    fn open(runs_dir: Option<&Path>, run_id: &str, audit: Option<Arc<AuditSelector>>) -> Self {
        // DP1: the run commits to its audit key before its first draw.
        if let Some(audit) = &audit {
            audit.open_run(run_id);
        }
        let run_id = run_id.to_string();
        let epoch = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let arm_sets = parking_lot::Mutex::new(HashMap::new());
        let Some(run_dir) = runs_dir.map(|dir| dir.join(&run_id)) else {
            return Self {
                ordinals: AttemptOrdinals::default(),
                writer: None,
                audit,
                run_id,
                epoch,
                arm_sets,
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
        Self {
            ordinals,
            writer,
            audit,
            run_id,
            epoch,
            arm_sets,
        }
    }

    /// The arm set of `key`'s chain: drawn over `loops` in `mode` on the
    /// chain's first attempt, and the same for every later one.
    fn arm_set(&self, key: &AttemptKey, loops: &Registry, mode: &ArmMode) -> Arc<ArmSet> {
        let mut sets = self.arm_sets.lock();
        let set = sets.entry(key.chain_key()).or_insert_with(|| {
            let draws = ArmDraws::new(ARM_SEED, self.epoch.clone());
            Arc::new(ArmSet::assign(key, loops, mode, &draws))
        });
        Arc::clone(set)
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
        // DP1: reveal the run's audit key once the run is over.
        if let Some(audit) = self.audit.take() {
            audit.close_run(&self.run_id);
        }
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
    /// This process's invocation ordinal of each run, from the run's
    /// manifest ([`GraphTaskDispatcher::attach_run_invocation`]). Attempt
    /// records carry it as `inv`.
    invocations: parking_lot::Mutex<HashMap<String, u32>>,
    /// The audit lottery, made when the first attempt opens (`[audit]`).
    audit: std::sync::OnceLock<Option<Arc<AuditSelector>>>,
    /// The loop registry and the `[experiments]` mode the arm sets are drawn
    /// with, loaded when the first attempt opens; `None` without a registry.
    arm_inputs: std::sync::OnceLock<Option<(Registry, ArmMode)>>,
}

impl Default for AttemptBook {
    fn default() -> Self {
        Self {
            fallback_run_id: format!("graph-{}", uuid::Uuid::new_v4().simple()),
            runs: parking_lot::Mutex::new(HashMap::new()),
            invocations: parking_lot::Mutex::new(HashMap::new()),
            audit: std::sync::OnceLock::new(),
            arm_inputs: std::sync::OnceLock::new(),
        }
    }
}

/// The loop registry the arm sets are drawn over: the embedded registry,
/// merged with `workdir`'s override. An unreadable override is logged and
/// the embedded registry used; `None` only when that fails too.
fn load_loop_registry(workdir: &Path) -> Option<Registry> {
    Registry::load(workdir)
        .inspect_err(|error| {
            tracing::warn!(%error, "loop registry override unreadable; using the embedded one");
        })
        .or_else(|_| Registry::embedded())
        .inspect_err(|error| tracing::warn!(%error, "no loop registry; attempts draw no arms"))
        .ok()
}

/// The provider agent id of one attempt: its attempt key
/// (`{run}:{plan}:{task}:{attempt}`). The provider immune boundary keys its
/// isolation controls by this id, so a control covers this attempt and no
/// other, in this run or a later one (decision 1107). An attempt key the
/// boundary would refuse falls back to `{plan}/{task}#{attempt}`.
pub(super) fn attempt_agent_id(key: &AttemptKey, plan_id: &str, task_or_cell: &str) -> String {
    let attempt_key = key.attempt_key();
    if roko_agent::immune_boundary::validate_provider_agent_id(&attempt_key).is_ok() {
        return attempt_key;
    }
    format!("{plan_id}/{task_or_cell}#{}", key.attempt)
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
        let audit = self.audit().cloned();
        let mut runs = self.runs.lock();
        let run = runs
            .entry(run_id.to_string())
            .or_insert_with(|| Arc::new(RunAttempts::open(runs_dir, run_id, audit)));
        Arc::clone(run)
    }

    /// The arm set of `attempt`'s chain in its run (S02.P1-14): drawn over
    /// `workdir`'s loop registry in the mode `experiments` sets on the chain's
    /// first attempt, and inherited by its retries. `None` when no registry
    /// loads.
    fn arm_set(
        &self,
        attempt: &AttemptContext,
        workdir: &Path,
        experiments: &ExperimentsConfig,
    ) -> Option<Arc<ArmSet>> {
        let (loops, mode) = self
            .arm_inputs
            .get_or_init(|| Some((load_loop_registry(workdir)?, ArmMode::for_config(experiments))))
            .as_ref()?;
        Some(attempt.run.arm_set(&attempt.key, loops, mode))
    }

    /// The audit lottery, once the first attempt opened with `[audit]
    /// enabled`.
    pub(super) fn audit(&self) -> Option<&Arc<AuditSelector>> {
        self.audit.get().and_then(Option::as_ref)
    }

    /// Record that this process is invocation `inv` of run `run_id`.
    fn attach_invocation(&self, run_id: &str, inv: u32) {
        self.invocations.lock().insert(run_id.to_string(), inv);
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
        identity.inv = self.invocations.lock().get(run_id).copied();
        let task_spec_hash = b3_digest(spec.task_def_json.as_bytes());
        let started_at = now_ms();
        let mut open = AttemptOpenRecord::new(identity.clone(), started_at);
        open.task_spec_hash = Some(task_spec_hash.clone());
        open.role = Some(task.role.as_deref().unwrap_or("implementer").to_string());
        open.max_retries = Some(spec.max_retries);
        // Learned tier limits group attempts by it (gap-5a6e01).
        open.tier = Some(task.tier_class().label().to_string());
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
            ladder: None,
            reflex_rule: None,
            live_tool_calls: LiveToolCalls::default(),
            verify_steps: Vec::new(),
            exposures: None,
            pricing: None,
            arm_set: None,
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
    /// Where the attempt stands on the model ladder, and whether it is the
    /// task's last chance there (gap-460230).
    ladder: Option<(AttemptLadder, bool)>,
    /// The T0 reflex rule that served the attempt in place of the provider.
    reflex_rule: Option<uuid::Uuid>,
    /// The tool calls the attempt's live output shows (bug-264c41).
    live_tool_calls: LiveToolCalls,
    /// What each verify step did, once verification settled (backlog 2104).
    verify_steps: Vec<VerifyStepVerdict>,
    /// How many content items the attempt's prompt retrieved and included,
    /// once it was planned (S01 P0-9).
    exposures: Option<ExposureCounts>,
    /// The run's price snapshot, which prices the verdict (backlog 2115).
    pricing: Option<Arc<PriceSnapshot>>,
    /// The arms of the attempt's chain (S02.P1-14), which every decision row
    /// of the attempt carries; `None` when no loop registry loaded.
    arm_set: Option<Arc<ArmSet>>,
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

    /// Verification starts: the pre-verify screen, then the verify steps.
    pub(super) fn verify_started(&mut self) {
        self.timing.verify_started_at = Some(now_ms());
    }

    /// Verification ended.
    pub(super) fn verify_ended(&mut self) {
        self.timing.verify_ended_at = Some(now_ms());
    }

    /// Verification found `steps`: what each verify step did, which the
    /// verdict lists (backlog 2104).
    pub(super) fn record_verify_steps(&mut self, steps: Vec<VerifyStepVerdict>) {
        self.verify_steps = steps;
    }

    /// Provider failover passed over `failover`'s models before the one
    /// that ran (bug-35379d). An attempt the ladder routed then records the
    /// rung that ran, as a failover, which never exhausts the ladder
    /// (backlog 1120).
    pub(super) fn record_failover(&mut self, failover: FailoverChain) {
        if let (Some(rung), Some((ladder, last_chance))) = (&failover.rung, &mut self.ladder) {
            ladder.rung = Some(rung.name.clone());
            ladder.index = Some(rung.index);
            ladder.reason = LadderReason::Failover;
            *last_chance = false;
        }
        self.failover = failover;
    }

    /// The attempt's helper model calls settled with `usage` (bug-62e3f4).
    pub(super) fn record_helper_calls(&mut self, usage: HelperCallsUsage) {
        self.helpers = (usage.calls > 0).then_some(usage);
    }

    /// The attempt runs where `ladder` says on the model ladder. With
    /// `last_chance` it is the task's last attempt and no rung is left above
    /// it, so an agent-blamed failure exhausts the ladder (gap-460230).
    pub(super) fn record_ladder(&mut self, ladder: AttemptLadder, last_chance: bool) {
        self.ladder = Some((ladder, last_chance));
    }

    /// Queue the attempt's route decision, with its chain's arms, for the
    /// run's `decisions.jsonl` (S01 P0-8); the writer stamps its sequence
    /// number.
    pub(super) fn record_decision(
        &self,
        mut decision: roko_learn::routing_log::RoutingDecisionLog,
    ) {
        decision.arm_set = self.arm_set.as_deref().cloned();
        self.run.submit(decision);
    }

    /// Queue the attempt's placebo decision (S03 §4.3, S02 L12) for the
    /// run's `decisions.jsonl`: its chain's placebo assignment, whose two
    /// arms propose the same thing, so nothing reads the arm. A retry carries
    /// its chain's arm; an attempt without an arm set writes none.
    fn record_placebo_decision(&self) {
        let Some(assignment) = self.arm_set.as_deref().and_then(ArmSet::placebo) else {
            return;
        };
        let decision = PlaceboDecisionRecord::new(self.identity.clone(), assignment.clone());
        self.run.submit(decision);
    }

    /// The identity every record of the attempt flattens.
    pub(super) fn identity(&self) -> &AttemptIdentity {
        &self.identity
    }

    /// Queue one item the attempt's prompt retrieved for the run's
    /// `exposures.jsonl` (S01 P0-9).
    pub(super) fn record_exposure(&self, exposure: ExposureRecord) {
        self.run.submit(exposure);
    }

    /// Queue the content decision the attempt's prompt made at one decision
    /// point, with its chain's arms, for the run's `decisions.jsonl` (S01
    /// P0-9).
    pub(super) fn record_content_decision(&self, mut decision: ContentDecisionRecord) {
        decision.arm_set = self.arm_set.as_deref().cloned();
        self.run.submit(decision);
    }

    /// The attempt's prompt retrieved and included `counts` content items,
    /// which its verdict records.
    pub(super) fn record_exposure_counts(&mut self, counts: ExposureCounts) {
        self.exposures = Some(counts);
    }

    /// The T0 reflex rule `rule_id` served the attempt in place of the
    /// provider. The attempt's learning label alone credits or demotes the
    /// rule once it settles (gap-4468bd).
    pub(super) fn served_by_reflex(&mut self, rule_id: uuid::Uuid) {
        self.reflex_rule = Some(rule_id);
    }

    /// The record the attempt's live-output tap fills with the tool calls
    /// the provider streams (bug-264c41).
    pub(super) fn live_tool_calls(&self) -> LiveToolCalls {
        self.live_tool_calls.clone()
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
        verdict.steps = self.verify_steps;
        verdict.timing = self.timing;
        // The call's time to first token, measured from its start
        // (gap-7a8474), places the first token. A call that streamed no
        // output records none, never 0 (S01 decision 6).
        let first_token_at = first_token_time(dispatch, verdict.timing.dispatch_started_at);
        let ttft_source = if first_token_at.is_some() {
            "stream"
        } else {
            "unavailable"
        };
        verdict.timing.first_token_at = first_token_at;
        verdict.timing.ttft_source = Some(ttft_source.to_string());
        verdict.timing.settled_at = Some(now_ms());
        verdict.executed = executed_model(model_requested, dispatch, self.failover);
        verdict.usage = dispatch.map(attempt_usage).unwrap_or_default();
        verdict.cost = attempt_cost(dispatch);
        // What the attempt's tokens cost at the run's price snapshot, the
        // figure cost per verified task reads (backlog 2115).
        if let Some(snapshot) = self.pricing.as_deref()
            && let Some(priced) = snapshot_price(snapshot, &verdict.usage, &verdict.executed)
        {
            verdict.cost.api_equiv_usd = Some(priced.api_equiv_usd);
            verdict.cost.without_cache_usd = Some(priced.without_cache_usd);
            verdict.cost.price_snapshot_id = Some(snapshot.id().to_string());
        }
        verdict.helpers = self.helpers;
        let agent_failed = verdict.blame == Blame::Agent;
        verdict.ladder = self.ladder.map(|(mut ladder, last_chance)| {
            ladder.exhausted = last_chance && agent_failed;
            ladder
        });
        verdict.isolation = dispatch.map(agent_isolation).unwrap_or_default();
        verdict.output_sha256 = dispatch
            .and_then(|dispatch| dispatch.result.output.body.as_text().ok())
            .map(sha256_hex);
        verdict.exposures = self.exposures;
        self.run.submit(verdict.clone());
        // DP1: a green attempt draws its audit ticket; the draw is only logged.
        if let Some(audit) = &self.run.audit {
            let output = dispatch.and_then(|dispatch| dispatch.result.output.body.as_text().ok());
            audit.draw(&verdict, output);
        }
        SettledAttempt {
            verdict: Arc::new(verdict),
            failure_reason,
            reflex_rule: self.reflex_rule,
            live_tool_calls: self.live_tool_calls,
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
    /// with the screen's check as its rung. A verify its stopping plan run
    /// cut short is a cancellation, which teaches nothing (bug-82cbef).
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
            Err(RokoError::Cancelled(reason)) => Self {
                outcome: AttemptOutcome::Cancelled,
                gate_verdict: None,
                first_token_seen: true,
                failure_reason: Some(super::turn_policy::attempt_failure_reason(
                    "cancelled",
                    reason,
                )),
                rung: None,
            },
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

    /// An attempt the stall watchdog cancelled, failed with `message`: a
    /// timeout, whatever its message says (bug-4c553b). After its first token
    /// the timeout is the agent's, before it the provider's.
    pub(super) fn stalled(message: &str, first_token_seen: bool) -> Self {
        Self {
            outcome: AttemptOutcome::Timeout,
            gate_verdict: None,
            first_token_seen,
            failure_reason: Some(super::turn_policy::stall_failure_reason(message)),
            rung: None,
        }
    }

    /// A provider call that ended in `error` before returning a result. A
    /// stop the plan run asked for is a cancellation, which teaches nothing
    /// (bug-2b1ddc). An attempt in the shared checkout that only unguarded
    /// CLI agents could take never called a provider, so it is the harness's
    /// failure (decision 1214). Anything else is [`Self::provider_failure`].
    pub(super) fn provider_call_error(error: &RokoError) -> Self {
        match error {
            RokoError::Cancelled(reason) => Self {
                outcome: AttemptOutcome::Cancelled,
                gate_verdict: None,
                first_token_seen: false,
                failure_reason: Some(super::turn_policy::attempt_failure_reason(
                    "cancelled",
                    reason,
                )),
                rung: None,
            },
            RokoError::Gateway { category, .. }
                if *category == super::failover::UNGUARDED_IN_CHECKOUT =>
            {
                Self::harness_failure(error)
            }
            _ => Self::provider_failure(&error.to_string(), false),
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
    /// The T0 reflex rule that served the attempt, which its learning label
    /// credits or demotes ([`GraphTaskDispatcher::credit_reflex_rule`]).
    pub(super) reflex_rule: Option<uuid::Uuid>,
    /// The tool calls the attempt's live output showed, which its efficiency
    /// row lists (bug-264c41).
    pub(super) live_tool_calls: LiveToolCalls,
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
    /// succeeded and no verify step failed. It counts an unverified or
    /// already-satisfied attempt as a success, so no learner reads it;
    /// learners read [`Self::learning_success`].
    pub(super) fn succeeded(&self) -> bool {
        matches!(
            self.verdict.outcome,
            AttemptOutcome::Passed
                | AttemptOutcome::AlreadySatisfied
                | AttemptOutcome::Unverified
                | AttemptOutcome::ForcedAccept
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
        self.attempts.audit.get_or_init(|| {
            AuditSelector::for_config(&self.config.audit, &self.workdir).map(Arc::new)
        });
        let mut attempt = self.attempts.open(
            self.feedback.runs_dir.as_deref(),
            self.attempts.run_id(ctx),
            spec,
            task,
            ctx.cell_id.as_deref(),
        );
        attempt.pricing = self.pricing_snapshot();
        // S02.P1-14: the chain's arms, drawn on its first attempt, and the
        // placebo's decision among them (S02 L12).
        attempt.arm_set = self
            .attempts
            .arm_set(&attempt, &self.workdir, &self.config.experiments);
        attempt.record_placebo_decision();
        attempt
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

    /// Record that this process is invocation `inv` of run `run_id`: the
    /// ordinal the run's manifest gave it. The run's attempt-open lines and
    /// verdicts then carry it as `inv` (S01 §4.2: carried, not part of the
    /// key), so a resumed run's attempts say which invocation, and so which
    /// build and config, they ran under.
    pub fn attach_run_invocation(&self, run_id: &str, inv: u32) {
        self.attempts.attach_invocation(run_id, inv);
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

/// When the attempt's first token arrived (unix ms): the provider call's
/// start plus the time to first token its stream measured. `None` when the
/// call showed no streamed output, or never started.
fn first_token_time(
    dispatch: Option<&crate::dispatch_v2::AgentResultDispatch>,
    dispatch_started_at: Option<i64>,
) -> Option<i64> {
    let ttft_ms = i64::try_from(dispatch?.result.ttft_ms?).ok()?;
    dispatch_started_at?.checked_add(ttft_ms)
}

/// The telemetry mirror of the Graph gate tag.
const fn gate_verdict_tag(verdict: TaskGateVerdict) -> GateVerdictTag {
    match verdict {
        TaskGateVerdict::Passed => GateVerdictTag::Passed,
        TaskGateVerdict::PassedWithPreexistingFailures => {
            GateVerdictTag::PassedWithPreexistingFailures
        }
        TaskGateVerdict::AlreadySatisfied => GateVerdictTag::AlreadySatisfied,
        TaskGateVerdict::Unverified => GateVerdictTag::Unverified,
        TaskGateVerdict::ForcedAccept => GateVerdictTag::ForcedAccept,
    }
}

/// Detail of an outcome other than a pass, an already-satisfied or an
/// unverified attempt. Only the digest of the failure text is kept.
fn failure_class(
    outcome: AttemptOutcome,
    failure_reason: Option<&str>,
    rung: Option<String>,
) -> Option<AttemptFailureClass> {
    if matches!(
        outcome,
        AttemptOutcome::Passed
            | AttemptOutcome::AlreadySatisfied
            | AttemptOutcome::Unverified
            | AttemptOutcome::ForcedAccept
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
        executed.turns = reported_turns(dispatch);
        executed.sampling = request_sampling(&dispatch.target);
        executed.tool_policy = dispatch.tool_policy.clone();
    }
    executed
}

/// The sampling parameters the attempt's requests carried, from the
/// provider and model that ran (gap-13bbbd); empty when the provider's
/// defaults applied, or the target named no provider config or profile.
fn request_sampling(
    target: &crate::dispatch_v2::ProviderDispatchSpec,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    target
        .provider_config
        .as_ref()
        .zip(target.model_profile.as_ref())
        .map(|(provider, model)| {
            roko_agent::provider::openai_compat::request_sampling(provider, model)
        })
        .unwrap_or_default()
}

/// The agent turns `dispatch` reported: the Claude CLI's `num_turns`, or
/// the model calls of roko's tool loop. `None` when the agent reported no
/// count, which records read as unknown (bug-55fd84, bug-ad5487).
pub(super) fn reported_turns(dispatch: &crate::dispatch_v2::AgentResultDispatch) -> Option<u32> {
    dispatch.events.iter().rev().find_map(|event| match event {
        roko_agent::AgentRuntimeEvent::TurnCompleted { num_turns, .. } => *num_turns,
        _ => None,
    })
}

/// How `dispatch`'s agent run was isolated from the invoking user's own
/// configuration: the isolation tags a Claude CLI run puts on its output
/// ([`ClaudeIsolation::tags`](roko_agent::claude_cli_agent::ClaudeIsolation::tags)).
/// Empty for an agent that reports none (gap-751ac9).
fn agent_isolation(
    dispatch: &crate::dispatch_v2::AgentResultDispatch,
) -> std::collections::BTreeMap<String, String> {
    roko_agent::claude_cli_agent::ClaudeIsolation::TAG_KEYS
        .iter()
        .filter_map(|key| {
            let value = dispatch.result.output.tag(key)?;
            Some(((*key).to_string(), value.to_string()))
        })
        .collect()
}

/// The attempt's tokens in the verdict's disjoint classes (S01 §4.4), as its
/// provider reported them; empty when it reported none.
fn attempt_usage(dispatch: &crate::dispatch_v2::AgentResultDispatch) -> AttemptUsage {
    let writes = cache_write_class(dispatch.target.provider_kind);
    dispatch
        .result
        .usage_obs
        .as_ref()
        .map(|observation| AttemptUsage::from_observation(observation, writes))
        .unwrap_or_default()
}

/// Where `kind`'s prompt-cache writes fall among the token classes (S01
/// §4.4): Claude Code sessions write at the 1-hour TTL, the Anthropic API at
/// 5 minutes, and OpenAI-style usage counts writes as input.
const fn cache_write_class(kind: roko_core::ProviderKind) -> CacheWriteClass {
    use roko_core::ProviderKind;
    match kind {
        ProviderKind::ClaudeCli => CacheWriteClass::OneHour,
        ProviderKind::AnthropicApi => CacheWriteClass::FiveMinutes,
        ProviderKind::OpenAiCompat
        | ProviderKind::CerebrasApi
        | ProviderKind::PerplexityApi
        | ProviderKind::Hermes
        | ProviderKind::OpenClaw
        | ProviderKind::CodexCli => CacheWriteClass::InInput,
        _ => CacheWriteClass::Unknown,
    }
}

/// The attempt's cost amounts (S01 §4.4): where its usage came from, the
/// CLI's own figure for a CLI agent, and for an API provider the priced cost
/// the plan budget settled. An unpriced call, or one whose usage is unknown,
/// leaves `billed_usd` unknown. An API provider's usage cost is roko's own
/// price, not a vendor figure, so it never fills `vendor_usd`. A CLI agent
/// bills what its provider's `billing` says (decision 2113): nothing on a
/// subscription, the CLI's own figure when metered, and unknown when unset.
/// The API-equivalent figures come from the run's price snapshot
/// ([`snapshot_price`]).
fn attempt_cost(dispatch: Option<&crate::dispatch_v2::AgentResultDispatch>) -> AttemptCost {
    let mut cost = AttemptCost {
        source: cost_source(dispatch),
        ..AttemptCost::default()
    };
    let Some(dispatch) = dispatch else {
        return cost;
    };
    let usage = &dispatch.result.usage;
    if is_cli_backend(dispatch.target.provider_kind) {
        cost.vendor_usd = dispatch
            .result
            .usage_obs
            .as_ref()
            .and_then(|observation| observation.cost_usd);
        let billing = dispatch
            .target
            .provider_config
            .as_ref()
            .and_then(|provider| provider.billing);
        cost.billed_usd = match billing {
            Some(ProviderBilling::Subscription) => Some(0.0),
            Some(ProviderBilling::Metered) => cost.vendor_usd,
            None => None,
        };
    } else if cost.source != CostSource::Unknown && usage.has_known_cost() {
        cost.billed_usd = Some(f64::from(usage.cost_usd));
    }
    cost
}

/// What the attempt's `usage` costs at the rates of the run's price
/// `snapshot` (S01 §4.4, backlog 2115): `api_equiv_usd` and
/// `without_cache_usd`, for the model that served it, the one the provider
/// reported, else the one the bridge launched. `None`, so all three
/// snapshot figures stay unknown, when the snapshot does not list that
/// model or the provider reported no input or output count. A cache class
/// it did not report holds no tokens.
fn snapshot_price(
    snapshot: &PriceSnapshot,
    usage: &AttemptUsage,
    executed: &ExecutedModel,
) -> Option<PricedUsage> {
    let reported = executed.model_reported.as_deref();
    let model = reported.or(executed.model_dispatched.as_deref())?;
    let tokens = TokenCounts {
        input: usage.tokens_in?,
        cache_read: usage.tokens_cache_read.unwrap_or(0),
        cache_write_5m: usage.tokens_cache_write_5m.unwrap_or(0),
        cache_write_1h: usage.tokens_cache_write_1h.unwrap_or(0),
        output: usage.tokens_out?,
        reasoning: usage.tokens_reasoning.unwrap_or(0),
    };
    snapshot.price(model, &tokens)
}

/// Where the attempt's priced usage came from (S01 §4.4). CLI agents report
/// their own usage.
fn cost_source(dispatch: Option<&crate::dispatch_v2::AgentResultDispatch>) -> CostSource {
    let Some(dispatch) = dispatch else {
        return CostSource::Unknown;
    };
    let Some(usage) = dispatch.result.usage_obs.as_ref() else {
        return CostSource::Unknown;
    };
    CostSource::from_usage_source(&usage.source, is_cli_backend(dispatch.target.provider_kind))
}

pub(super) fn sha256_hex(text: &str) -> String {
    format!("{:x}", sha2::Sha256::digest(text.as_bytes()))
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::{ModelProfile, ProviderConfig};
    use roko_learn::telemetry::Blame;
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        FIXTURE_HANG_GUARD_SECS, FIXTURE_PROVIDER_TIMEOUT_MS, VERIFY_PROVIDER, final_turn,
        make_bare_dispatcher, make_spec, make_task_def, make_test_dispatcher, no_auto_fix,
        spawn_openai_mock, verify_step,
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
        crate::background_writes::settled(path.parent().unwrap_or(path)).await;
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

        // The attempt's one settled row's id is the key (backlog 2107). The
        // provider bridge logs its own `model_call` rows to the same file,
        // under the feedback schema.
        let efficiency = jsonl_rows_where(&roko.join("learn/efficiency.jsonl"), 1, |row| {
            row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA
        })
        .await;
        assert_eq!(field(&efficiency, "attempt_id"), [key.as_str()]);
        assert_eq!(field(&efficiency, "attempt_key"), [key.as_str()]);
        assert_eq!(efficiency[0]["gate_passed"], true);
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

    /// A Claude CLI attempt's verdict states how its run was kept apart from
    /// the invoking user's configuration (gap-751ac9): the isolation tags
    /// this machine gives every Claude run.
    #[tokio::test]
    async fn attempt_records_carry_the_claude_isolation_settings() {
        let temp = tempdir().expect("tempdir");
        let roko = temp.path().join(".roko");
        let feedback = GraphFeedbackContext {
            runs_dir: Some(roko.join("runs")),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the attempt completes");
        drop(dispatcher);

        let attempts = jsonl_rows(&roko.join("runs").join(RUN).join("attempts.jsonl"), 2).await;
        let verdict = &attempts[1];
        let expected: serde_json::Map<String, serde_json::Value> =
            roko_agent::claude_cli_agent::ClaudeIsolation::new(temp.path())
                .tags()
                .into_iter()
                .map(|(key, value)| (key.to_string(), serde_json::Value::String(value)))
                .collect();
        assert_eq!(
            verdict["isolation"],
            serde_json::Value::Object(expected),
            "{verdict}"
        );
        assert_eq!(verdict["isolation"]["setting_sources"], "none");
        assert_eq!(verdict["isolation"]["auto_memory"], "off");
        assert_eq!(verdict["isolation"]["config_dir"], "user");
    }

    /// The attempt's settled efficiency row carries its turns (bug-ad5487):
    /// the reported count when the agent gave one, and 0 marked
    /// `turns_unknown` when it gave none.
    #[tokio::test]
    async fn efficiency_row_carries_the_attempts_turns_or_unknown() {
        const FOUR_TURNS_PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"verify-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-v4","model":"claude-sonnet-4-6","num_turns":4,"total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;
        for (provider, turns) in [(VERIFY_PROVIDER, None), (FOUR_TURNS_PROVIDER, Some(4))] {
            let temp = tempdir().expect("tempdir");
            let efficiency_path = temp.path().join(".roko/learn/efficiency.jsonl");
            let feedback = GraphFeedbackContext {
                efficiency_path: Some(efficiency_path.clone()),
                ..GraphFeedbackContext::default()
            };
            let (dispatcher, mut task) =
                make_test_dispatcher(&temp, provider, no_auto_fix, feedback).await;
            task.verify = vec![verify_step("structural", "true")];
            dispatcher
                .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
                .await
                .expect("the verify step passes");

            let rows = jsonl_rows_where(&efficiency_path, 1, |row| {
                row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA
            })
            .await;
            assert_eq!(rows.len(), 1, "one settled row per attempt");
            for row in &rows {
                let id = row["attempt_id"].as_str().unwrap_or_default();
                assert_eq!(row["turn_number"], turns.unwrap_or(0), "{id}");
                assert_eq!(row["iteration"], turns.unwrap_or(0), "{id}");
                assert_eq!(
                    row.get("turns_unknown")
                        .and_then(serde_json::Value::as_bool),
                    turns.is_none().then_some(true),
                    "{id}: {row}"
                );
            }
        }
    }

    /// gap-2e69b2: an efficiency row's attempt id is the attempt's durable
    /// key, so the same task's first attempt in two runs has two ids.
    #[tokio::test]
    async fn attempt_id_is_unique_across_runs() {
        let temp = tempdir().expect("tempdir");
        let efficiency_path = temp.path().join(".roko/learn/efficiency.jsonl");
        let feedback = GraphFeedbackContext {
            efficiency_path: Some(efficiency_path.clone()),
            runs_dir: Some(temp.path().join(".roko/runs")),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        let spec = make_spec(&task);
        for run in ["run-a", "run-b"] {
            let ctx = CellContext::new().with_run_id(run.to_string());
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect("the attempt completes");
        }

        let rows = jsonl_rows_where(&efficiency_path, 2, |row| {
            row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA
        })
        .await;
        let mut ids = field(&rows, "attempt_id");
        ids.sort_unstable();
        let key = |run: &str| format!("{run}:{}:{}:1", spec.plan_id, task.id);
        assert_eq!(ids, [key("run-a"), key("run-b")]);
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
        // One per-run seq runs across every record kind (S01 §4.7), so the
        // decision records between these rows leave gaps; a resumed writer
        // that restarted the count would repeat a number.
        let seqs: Vec<u64> = rows.iter().filter_map(|row| row["seq"].as_u64()).collect();
        assert_eq!(seqs.len(), 3, "{seqs:?}");
        assert!(
            seqs.windows(2).all(|pair| pair[0] < pair[1]),
            "the resumed writer continues the run's seq: {seqs:?}"
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

    /// A fake Claude CLI that streams its first output after 200 ms.
    const SLOW_FIRST_TOKEN_PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
sleep 0.2
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"verify-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-v1","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// backlog 2102: the verdict places the first token by the stream's time
    /// to first token, and records when verification started and ended. An
    /// attempt with no measured first token records none, never 0.
    #[tokio::test]
    async fn verdict_records_first_token_and_verify_times() {
        let temp = tempdir().expect("tempdir");
        let runs_dir = temp.path().join(".roko/runs");
        let feedback = GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, SLOW_FIRST_TOKEN_PROVIDER, no_auto_fix, feedback).await;
        task.verify = vec![verify_step("structural", "true")];
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the verified attempt passes");
        drop(dispatcher);

        let attempts = jsonl_rows(&runs_dir.join(RUN).join("attempts.jsonl"), 2).await;
        let timing = &attempts[1]["timing"];
        let at = |name: &str| {
            timing[name]
                .as_i64()
                .unwrap_or_else(|| panic!("no {name}: {timing}"))
        };
        assert_eq!(timing["ttft_source"], "stream", "{timing}");
        assert!(at("first_token_at") > at("dispatch_started_at"), "{timing}");
        assert!(
            at("dispatch_ended_at") <= at("verify_started_at"),
            "{timing}"
        );
        assert!(at("verify_started_at") <= at("verify_ended_at"), "{timing}");
        assert!(at("verify_ended_at") <= at("settled_at"), "{timing}");

        let passed = Settlement::verified(&Ok(TaskGateVerdict::Passed));
        let settled = AttemptBook::default()
            .open(None, "run-1", &spec, &task, None)
            .settle(passed, "", None);
        let timing = &settled.verdict.timing;
        assert_eq!(timing.first_token_at, None);
        assert_eq!(timing.ttft_source.as_deref(), Some("unavailable"));
        assert_eq!(timing.verify_started_at, None);
    }

    /// `api-model` on an OpenAI-compatible provider the mock at `base_url`
    /// serves, priced at $1 in and $2 out per million tokens.
    fn priced_api_config(base_url: String) -> RokoConfig {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "api-model".to_string();
        config.agent.bare_mode = false;
        config.gates.cargo_fix_enabled = false;
        // `PATH` is always set, standing in for the provider's key.
        config.providers.insert(
            "mock_api".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: Some(base_url),
                api_key_env: Some("PATH".to_string()),
                timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                ..ProviderConfig::default()
            },
        );
        config.models.insert(
            "api-model".to_string(),
            ModelProfile {
                provider: "mock_api".to_string(),
                slug: "api-model-1".to_string(),
                context_window: 128_000,
                max_output: Some(1_024),
                max_tools: Some(32),
                supports_tools: true,
                tool_format: "openai_json".to_string(),
                cost_input_per_m: Some(1.0),
                cost_output_per_m: Some(2.0),
                ..ModelProfile::default()
            },
        );
        // No stall watchdog: the mock answers whether or not the call
        // streams.
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 0;
        config
    }

    /// backlog 2103: settling fills the verdict's token classes and cost
    /// amounts from what the provider reported. A Claude CLI attempt keeps
    /// the CLI's own figure as `vendor_usd`; an OpenAI-compatible attempt
    /// counts its cached input once, and bills what its cost row records.
    #[tokio::test]
    async fn settle_fills_usage_and_cost_amounts() {
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        let cli = tempdir().expect("tempdir");
        let runs_dir = cli.path().join(".roko/runs");
        let feedback = GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&cli, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        task.verify = vec![verify_step("structural", "true")];
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the verified attempt passes");
        drop(dispatcher);
        let attempts = jsonl_rows(&runs_dir.join(RUN).join("attempts.jsonl"), 2).await;
        let verdict = &attempts[1];
        assert_eq!(verdict["usage"]["tokens_out"], 10, "{verdict}");
        assert_eq!(verdict["cost"]["source"], "cli_usage", "{verdict}");
        assert_eq!(verdict["cost"]["vendor_usd"], 0.01, "{verdict}");
        assert!(verdict["cost"]["billed_usd"].is_null(), "{verdict}");

        let api = tempdir().expect("tempdir");
        let roko = api.path().join(".roko");
        let mut answer = final_turn("done");
        answer["usage"] = serde_json::json!({
            "prompt_tokens": 100,
            "completion_tokens": 10,
            "total_tokens": 110,
            "prompt_tokens_details": { "cached_tokens": 40 }
        });
        let (base_url, _requests) = spawn_openai_mock(vec![answer.clone(), answer]);
        let feedback = GraphFeedbackContext {
            costs_path: Some(roko.join("learn/costs.jsonl")),
            runs_dir: Some(roko.join("runs")),
            ..GraphFeedbackContext::default()
        };
        let dispatcher = make_bare_dispatcher(priced_api_config(base_url), api.path())
            .await
            .with_feedback(feedback);
        let task = TaskDef {
            model_hint: Some("api-model".to_string()),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            verify: vec![verify_step("structural", "true")],
            ..make_task_def("focused")
        };
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the verified attempt passes");
        drop(dispatcher);
        let attempts = jsonl_rows(&roko.join("runs").join(RUN).join("attempts.jsonl"), 2).await;
        let verdict = &attempts[1];
        let usage = &verdict["usage"];
        assert_eq!(usage["tokens_in"], 60, "{verdict}");
        assert_eq!(usage["tokens_cache_read"], 40, "{verdict}");
        assert_eq!(usage["tokens_out"], 10, "{verdict}");
        assert_eq!(usage["tokens_cache_write_5m"], 0, "{verdict}");
        assert_eq!(verdict["cost"]["source"], "provider_usage", "{verdict}");
        assert!(verdict["cost"]["vendor_usd"].is_null(), "{verdict}");
        let costs = jsonl_rows(&roko.join("learn/costs.jsonl"), 1).await;
        let billed = verdict["cost"]["billed_usd"].as_f64().expect("billed_usd");
        let recorded = costs[0]["cost_usd"].as_f64().expect("cost row");
        assert!(billed > 0.0, "{verdict}");
        assert!((billed - recorded).abs() < 1e-12, "{billed} != {recorded}");
    }

    /// The verdict and the cost row of one verified attempt of `slug` on the
    /// OpenAI-compatible mock, which reports `usage`, in a workspace whose
    /// run prices from `prices-2026-09-28` (backlog 2115).
    async fn snapshot_priced_attempt(
        slug: &str,
        usage: serde_json::Value,
    ) -> (serde_json::Value, serde_json::Value) {
        let temp = tempdir().expect("tempdir");
        let roko = temp.path().join(".roko");
        // The workspace's own copy of the snapshot, which a newer built-in
        // one does not replace.
        let prices = temp.path().join("config/prices");
        std::fs::create_dir_all(&prices).expect("prices dir");
        std::fs::write(
            prices.join("2026-09-28.toml"),
            include_str!("../../../../config/prices/2026-09-28.toml"),
        )
        .expect("write the snapshot");
        let mut answer = final_turn("done");
        answer["usage"] = usage;
        let (base_url, _requests) = spawn_openai_mock(vec![answer.clone(), answer]);
        let mut config = priced_api_config(base_url);
        config.pricing.snapshot = "prices-2026-09-28".to_string();
        config.models.get_mut("api-model").expect("api model").slug = slug.to_string();
        let feedback = GraphFeedbackContext {
            costs_path: Some(roko.join("learn/costs.jsonl")),
            runs_dir: Some(roko.join("runs")),
            ..GraphFeedbackContext::default()
        };
        let dispatcher = make_bare_dispatcher(config, temp.path())
            .await
            .with_feedback(feedback);
        let task = TaskDef {
            model_hint: Some("api-model".to_string()),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            verify: vec![verify_step("structural", "true")],
            ..make_task_def("focused")
        };
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the verified attempt passes");
        drop(dispatcher);
        let attempts = jsonl_rows(&roko.join("runs").join(RUN).join("attempts.jsonl"), 2).await;
        let costs = jsonl_rows(&roko.join("learn/costs.jsonl"), 1).await;
        (attempts[1].clone(), costs[0].clone())
    }

    /// backlog 2115: a verified attempt of a model the run's price snapshot
    /// lists carries the snapshot price on its verdict and its cost row: the
    /// API-equivalent cost, the uncached cost and the snapshot's id. Cached
    /// input makes the uncached cost the higher one. An attempt of a model
    /// the snapshot lacks has none of them.
    #[tokio::test]
    async fn verdict_and_cost_rows_carry_the_snapshot_price() {
        // S01 §5.5: 38,211 tokens in at $0.35 and 2,904 out at $0.75 per
        // million.
        let usage = serde_json::json!({
            "prompt_tokens": 38_211,
            "completion_tokens": 2_904,
            "total_tokens": 41_115
        });
        let (verdict, row) = snapshot_priced_attempt("gpt-oss-120b", usage).await;
        let cost = &verdict["cost"];
        let api_equiv = cost["api_equiv_usd"].as_f64().expect("api_equiv_usd");
        assert!((api_equiv - 0.01555185).abs() < 1e-9, "{verdict}");
        assert_eq!(cost["price_snapshot_id"], "prices-2026-09-28", "{verdict}");
        let uncached = cost["without_cache_usd"].as_f64().expect("uncached");
        assert!((uncached - api_equiv).abs() < 1e-12, "{verdict}");
        let recorded = row["api_equiv_usd"].as_f64().expect("cost row");
        assert!((recorded - api_equiv).abs() < 1e-12, "{row}");
        assert_eq!(row["price_snapshot_id"], "prices-2026-09-28", "{row}");

        // kimi-k2.6 reads cached input at $0.16 per million, not at its
        // $0.95 input rate: 60 tokens in, 40 cached and 10 out at $4.00.
        let usage = serde_json::json!({
            "prompt_tokens": 100,
            "completion_tokens": 10,
            "total_tokens": 110,
            "prompt_tokens_details": { "cached_tokens": 40 }
        });
        let (verdict, _) = snapshot_priced_attempt("kimi-k2.6", usage).await;
        let cost = &verdict["cost"];
        let api_equiv = cost["api_equiv_usd"].as_f64().expect("api_equiv_usd");
        let uncached = cost["without_cache_usd"].as_f64().expect("uncached");
        assert!((api_equiv - 103.4e-6).abs() < 1e-12, "{verdict}");
        assert!((uncached - 135e-6).abs() < 1e-12, "{verdict}");
        assert!(uncached > api_equiv, "{verdict}");

        // The snapshot does not list the mock's own model.
        let usage = serde_json::json!({
            "prompt_tokens": 100,
            "completion_tokens": 10,
            "total_tokens": 110
        });
        let (verdict, row) = snapshot_priced_attempt("api-model-1", usage).await;
        for figure in ["api_equiv_usd", "without_cache_usd", "price_snapshot_id"] {
            assert!(verdict["cost"][figure].is_null(), "{figure}: {verdict}");
        }
        assert!(row["api_equiv_usd"].is_null(), "{row}");
        assert!(row["price_snapshot_id"].is_null(), "{row}");
    }

    /// backlog 2115 (decision 2113): a CLI attempt bills what its provider's
    /// `billing` says: nothing on a subscription, the CLI's own figure when
    /// metered. With no `billing` what it bills is unknown
    /// (`settle_fills_usage_and_cost_amounts`).
    #[tokio::test]
    async fn a_cli_attempt_bills_by_its_providers_billing() {
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        for (billing, billed) in [
            (ProviderBilling::Subscription, 0.0),
            (ProviderBilling::Metered, 0.01),
        ] {
            let temp = tempdir().expect("tempdir");
            let runs_dir = temp.path().join(".roko/runs");
            let feedback = GraphFeedbackContext {
                runs_dir: Some(runs_dir.clone()),
                ..GraphFeedbackContext::default()
            };
            let configure = |config: &mut RokoConfig| {
                no_auto_fix(config);
                let provider = config.providers.get_mut("stream-cli").expect("provider");
                provider.billing = Some(billing);
            };
            let (dispatcher, mut task) =
                make_test_dispatcher(&temp, VERIFY_PROVIDER, configure, feedback).await;
            task.verify = vec![verify_step("structural", "true")];
            dispatcher
                .dispatch(&make_spec(&task), Vec::new(), &ctx)
                .await
                .expect("the verified attempt passes");
            drop(dispatcher);
            let attempts = jsonl_rows(&runs_dir.join(RUN).join("attempts.jsonl"), 2).await;
            let cost = &attempts[1]["cost"];
            assert_eq!(cost["source"], "cli_usage", "{billing:?}: {cost}");
            assert_eq!(cost["vendor_usd"], 0.01, "{billing:?}: {cost}");
            assert_eq!(cost["billed_usd"], billed, "{billing:?}: {cost}");
        }
    }

    /// backlog 2109: a cost row says whether roko could price the call. A
    /// model with no price writes `priced: false`, its $0 an unknown cost;
    /// one priced at 0/0 is free, and writes `priced: true`.
    #[tokio::test]
    async fn cost_rows_mark_unpriced_calls() {
        for (prices, priced) in [(None, false), (Some(0.0), true)] {
            let temp = tempdir().expect("tempdir");
            let roko = temp.path().join(".roko");
            let answers = vec![final_turn("done"), final_turn("done")];
            let (base_url, _requests) = spawn_openai_mock(answers);
            let mut config = priced_api_config(base_url);
            let model = config.models.get_mut("api-model").expect("api model");
            model.cost_input_per_m = prices;
            model.cost_output_per_m = prices;
            let feedback = GraphFeedbackContext {
                costs_path: Some(roko.join("learn/costs.jsonl")),
                ..GraphFeedbackContext::default()
            };
            let dispatcher = make_bare_dispatcher(config, temp.path())
                .await
                .with_feedback(feedback);
            let task = TaskDef {
                model_hint: Some("api-model".to_string()),
                timeout_secs: FIXTURE_HANG_GUARD_SECS,
                verify: vec![verify_step("structural", "true")],
                ..make_task_def("focused")
            };
            dispatcher
                .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
                .await
                .expect("the verified attempt passes");
            drop(dispatcher);

            let costs = jsonl_rows(&roko.join("learn/costs.jsonl"), 1).await;
            assert_eq!(costs[0]["priced"], priced, "{prices:?}: {}", costs[0]);
            assert_eq!(costs[0]["cost_usd"], 0.0, "{prices:?}: {}", costs[0]);
        }
    }
}
