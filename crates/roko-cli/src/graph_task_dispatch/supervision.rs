//! Conductor supervision of running Graph task attempts (spec-a0403b,
//! part B).
//!
//! With [`GraphTaskDispatcher::with_conductor`], each attempt feeds the run's
//! [`ConductorRing`] while its provider runs: its start, and the messages and
//! tool calls of its live output, each tagged with its attempt key
//! ([`ATTEMPT_TAG`]). Its verify run settles after the provider call, so its
//! gate verdict and compile diagnostic are kept as its task's evidence
//! instead ([`GraphConductor::settle_verify`], gap-1a7f9c). The plan host
//! runs [`GraphConductor::tick`] every [`SUPERVISION_INTERVAL`]
//! ([`GraphTaskDispatcher::spawn_conductor_ticker`]), which evaluates the
//! conductor over each running attempt's own signals and acts on the
//! decision:
//!
//! - `Restart` cancels that attempt the way the stall watchdog does: it fails
//!   and retries under its task's `max_retries`. When the task's next attempt
//!   ends, [`Conductor::record_intervention_outcome`] learns whether the
//!   restart helped;
//! - `Fail` stops the run: `tick` returns a [`ConductorStop`] naming the
//!   watcher and the reason, which the host makes the run's error;
//! - `Nudge` and `ForceAdvance` are not carried out yet (gap-ebd656).
//!
//! When an attempt's own signals call for nothing, the conductor evaluates
//! them again after its task's evidence, so `compile-fail-repeat` and
//! `test-failure-budget` compare the task's verify runs across attempts.
//! What it then calls for is only advice: until gap-ebd656 gives it a
//! reaction to failing gates, such as a nudge or an escalation, a restart
//! would cancel the task's fresh attempt, perhaps the one the model ladder
//! just escalated. Advice is logged and published as an advisory diagnosis,
//! cancels nothing, and spends the task's evidence, so it is given once.
//!
//! Each intervention is published as a dashboard diagnosis, and the attempt's
//! signals leave the ring, so the same evidence never acts twice; an attempt
//! that ends leaves the ring too. Evaluating each attempt on its own keeps
//! one task's signals from breaking or extending another's patterns, and lets
//! a restart name its attempt, which the decision itself does not.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use roko_agent::StreamEventKind;
use roko_agent::live_output::LiveAgentEvent;
use roko_conductor::{Conductor, InterventionOutcome};
use roko_core::{ConductorDecision, DiagnosisSeverity, DiagnosisSummary, TestCount, Verdict};
use roko_gate::{BuildSystem, GateFailureClassification};
use tokio_util::sync::CancellationToken;

use crate::runner::conductor_adapter::{
    ATTEMPT_TAG, ConductorRing, GateRun, compile_diagnostic_signal, gate_verdict_signal,
    graph_task_event_to_signal, graph_text_turn_signal,
};

use super::verification::verify_step_rung;
use super::watchdog::StallThresholds;
use super::*;

/// How often the plan host runs [`GraphConductor::tick`].
pub const SUPERVISION_INTERVAL: Duration = Duration::from_secs(5);

/// How many settled-verify signals the conductor keeps for a task, the
/// oldest dropped first: plenty for the watchers, which compare a few runs.
const TASK_EVIDENCE_CAPACITY: usize = 32;

/// The gate a verify run settles as when every step passed, as the Graph
/// path names it elsewhere.
const VERIFY_GATE: &str = "graph-verify";

/// A Graph run's conductor, the ring its attempts feed, the attempts whose
/// providers are running, and each task's settled verify runs.
#[derive(Clone)]
pub struct GraphConductor {
    conductor: Arc<Conductor>,
    ring: ConductorRing,
    running: Arc<parking_lot::Mutex<HashMap<String, RunningAttempt>>>,
    /// Restarts whose outcome is not known yet, by task (`plan/task`).
    restarts: Arc<parking_lot::Mutex<HashMap<String, PendingRestart>>>,
    /// The signals of each task's settled verify runs, oldest first, by task
    /// (`plan/task`): the evidence its next attempt is advised on.
    settled: Arc<parking_lot::Mutex<HashMap<String, Vec<Signal>>>>,
}

impl std::fmt::Debug for GraphConductor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphConductor")
            .field("conductor", &self.conductor)
            .field("ring_len", &self.ring.len())
            .field("running", &self.running.lock().len())
            .field("settled_tasks", &self.settled.lock().len())
            .finish_non_exhaustive()
    }
}

struct RunningAttempt {
    plan_id: String,
    task_id: String,
    cancel: CancellationToken,
    restart: Arc<OnceLock<ConductorRestart>>,
}

struct PendingRestart {
    /// The attempt the conductor restarted.
    attempt_key: String,
    watcher: String,
}

/// Why the conductor restarted an attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConductorRestart {
    pub(super) watcher: String,
    pub(super) reason: String,
}

impl ConductorRestart {
    /// The error the restarted attempt fails with. The Graph engine retries
    /// it under the task's `max_retries`.
    pub(super) fn error(&self) -> RokoError {
        RokoError::Agent {
            backend: "conductor".to_string(),
            message: format!(
                "conductor watcher `{}` restarted the attempt: {}",
                self.watcher, self.reason
            ),
        }
    }
}

/// A conductor `Fail`: the run must stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConductorStop {
    pub plan_id: String,
    pub task_id: String,
    /// The watcher that failed the run.
    pub watcher: String,
    pub reason: String,
}

impl std::fmt::Display for ConductorStop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "conductor watcher `{}` stopped the run at {}/{}: {}",
            self.watcher, self.plan_id, self.task_id, self.reason
        )
    }
}

impl GraphConductor {
    #[must_use]
    pub fn new(conductor: Arc<Conductor>, ring: ConductorRing) -> Self {
        Self {
            conductor,
            ring,
            running: Arc::default(),
            restarts: Arc::default(),
            settled: Arc::default(),
        }
    }

    /// Register an attempt whose provider is about to run; it stays
    /// supervised until the returned handle drops.
    pub(super) fn supervise(
        &self,
        plan_id: &str,
        task_id: &str,
        attempt_key: &str,
    ) -> SupervisedAttempt {
        let cancel = CancellationToken::new();
        let restart = Arc::new(OnceLock::new());
        self.running.lock().insert(
            attempt_key.to_string(),
            RunningAttempt {
                plan_id: plan_id.to_string(),
                task_id: task_id.to_string(),
                cancel: cancel.clone(),
                restart: Arc::clone(&restart),
            },
        );
        let mut feed = AttemptFeed {
            ring: self.ring.clone(),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            attempt_key: attempt_key.to_string(),
            last_text: None,
        };
        feed.push(&GraphTaskEvent::AttemptStarted {
            attempt_id: attempt_key.to_string(),
        });
        SupervisedAttempt {
            conductor: self.clone(),
            feed,
            cancel,
            restart,
        }
    }

    /// Evaluate the conductor over each running attempt's signals and act on
    /// its decisions, publishing each intervention through `tui`; an attempt
    /// whose signals call for nothing gets the advice of its task's settled
    /// verify runs (`advise`). Returns the `Fail` that must stop the run, if
    /// any.
    pub fn tick(&self, tui: Option<&TuiBridge>) -> Option<ConductorStop> {
        let running: Vec<(String, String, String)> = self
            .running
            .lock()
            .iter()
            .map(|(key, attempt)| {
                (
                    key.clone(),
                    attempt.plan_id.clone(),
                    attempt.task_id.clone(),
                )
            })
            .collect();
        if running.is_empty() {
            return None;
        }
        let signals = self.ring.snapshot();
        let ctx = roko_core::Context::now();
        for (attempt_key, plan_id, task_id) in running {
            let stream: Vec<Signal> = signals
                .iter()
                .filter(|signal| signal.tag(ATTEMPT_TAG) == Some(attempt_key.as_str()))
                .cloned()
                .collect();
            if stream.is_empty() {
                continue;
            }
            match self.conductor.evaluate_full(&stream, &ctx).decision {
                ConductorDecision::Continue => {
                    self.advise(tui, &attempt_key, &plan_id, &task_id, &stream, &ctx);
                }
                ConductorDecision::Restart { watcher, reason } => {
                    self.forget(&attempt_key);
                    publish_intervention(
                        tui,
                        &attempt_key,
                        &plan_id,
                        &task_id,
                        &watcher,
                        &reason,
                        false,
                    );
                    self.restarts.lock().insert(
                        format!("{plan_id}/{task_id}"),
                        PendingRestart {
                            attempt_key: attempt_key.clone(),
                            watcher: watcher.clone(),
                        },
                    );
                    if let Some(attempt) = self.running.lock().get(&attempt_key) {
                        let _ = attempt.restart.set(ConductorRestart { watcher, reason });
                        attempt.cancel.cancel();
                    }
                }
                ConductorDecision::Fail { watcher, reason } => {
                    self.forget(&attempt_key);
                    let reason = reason.to_string();
                    publish_intervention(
                        tui,
                        &attempt_key,
                        &plan_id,
                        &task_id,
                        &watcher,
                        &reason,
                        true,
                    );
                    return Some(ConductorStop {
                        plan_id,
                        task_id,
                        watcher,
                        reason,
                    });
                }
                decision => tracing::info!(
                    plan_id = %plan_id,
                    task_id = %task_id,
                    decision = decision.label(),
                    "conductor decision is not carried out on the Graph path yet (gap-ebd656)"
                ),
            }
        }
        None
    }

    /// Drop an attempt's signals from the ring.
    fn forget(&self, attempt_key: &str) {
        self.ring
            .retain(|signal| signal.tag(ATTEMPT_TAG) != Some(attempt_key));
    }

    /// Evaluate a running attempt at `plan_id/task_id`, whose `own` signals
    /// call for nothing, after its task's evidence, and publish what that
    /// calls for as advice ([`publish_advice`]). Until gap-ebd656 gives the
    /// conductor a reaction to failing gates, the advice acts on nothing: it
    /// cancels no attempt, and no restart outcome is learned from it. It
    /// spends the evidence, so it is given once.
    fn advise(
        &self,
        tui: Option<&TuiBridge>,
        attempt_key: &str,
        plan_id: &str,
        task_id: &str,
        own: &[Signal],
        ctx: &Context,
    ) {
        let task_key = format!("{plan_id}/{task_id}");
        let Some(evidence) = self.settled.lock().get(&task_key).cloned() else {
            return;
        };
        let stream: Vec<Signal> = evidence.into_iter().chain(own.iter().cloned()).collect();
        let decision = self.conductor.evaluate_full(&stream, ctx).decision;
        if decision.is_continue() {
            return;
        }
        self.settled.lock().remove(&task_key);
        publish_advice(tui, attempt_key, plan_id, task_id, &decision);
    }

    /// Keep the settled verify run of an attempt at `plan_id/task_id`
    /// ([`verify_run_signals`]) as its task's evidence, which the task's
    /// later attempts are advised on: `compile-fail-repeat` and
    /// `test-failure-budget` compare the task's runs across its attempts.
    pub(super) fn settle_verify(
        &self,
        plan_id: &str,
        task_id: &str,
        steps: &[(String, Verdict)],
        build: BuildSystem,
    ) {
        let signals = verify_run_signals(plan_id, task_id, steps, build);
        if signals.is_empty() {
            return;
        }
        let mut settled = self.settled.lock();
        let evidence = settled.entry(format!("{plan_id}/{task_id}")).or_default();
        evidence.extend(signals);
        let dropped = evidence.len().saturating_sub(TASK_EVIDENCE_CAPACITY);
        evidence.drain(..dropped);
    }

    /// An attempt of `plan_id/task_id` other than the one the conductor
    /// restarted ran to `succeeded`: that is how the restart turned out.
    fn learn_restart_outcome(
        &self,
        plan_id: &str,
        task_id: &str,
        attempt_key: &str,
        succeeded: bool,
    ) {
        let task_key = format!("{plan_id}/{task_id}");
        let restart = {
            let mut restarts = self.restarts.lock();
            match restarts.get(&task_key) {
                Some(pending) if pending.attempt_key != attempt_key => restarts.remove(&task_key),
                _ => None,
            }
        };
        if let Some(restart) = restart {
            self.conductor
                .record_intervention_outcome(InterventionOutcome {
                    watcher_name: restart.watcher,
                    severity_at_fire: 1.0,
                    decision_label: "restart".to_string(),
                    task_improved: succeeded,
                    recorded_at_ms: chrono::Utc::now().timestamp_millis(),
                });
        }
    }
}

/// Stops the conductor ticker when dropped.
#[derive(Debug)]
pub struct ConductorTicker {
    task: tokio::task::JoinHandle<()>,
}

impl Drop for ConductorTicker {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl GraphTaskDispatcher {
    /// Supervise running attempts with the run's `conductor`, which reads the
    /// signals they feed into `ring`, unless `[conductor]
    /// silence_timeout_secs` and `task_stall_secs` are both 0, which turns
    /// Graph supervision off. The plan host ticks it
    /// ([`Self::spawn_conductor_ticker`]).
    #[must_use]
    pub fn with_conductor(mut self, conductor: Arc<Conductor>, ring: ConductorRing) -> Self {
        if StallThresholds::from_config(&self.config.conductor).is_enabled() {
            self.conductor = Some(GraphConductor::new(conductor, ring));
        }
        self
    }

    /// Register an attempt whose provider is about to run with the conductor,
    /// if there is one.
    pub(super) fn supervise_attempt(
        &self,
        attempt: &WatchedAttempt<'_>,
    ) -> Option<SupervisedAttempt> {
        self.conductor.as_ref().map(|conductor| {
            conductor.supervise(attempt.plan_id, attempt.task_id, attempt.attempt_key)
        })
    }

    /// Hand the conductor, if there is one, the settled verify run of an
    /// attempt at `task` in `workdir`: the steps that ran, in order, each
    /// with its phase ([`GraphConductor::settle_verify`]).
    pub(super) fn publish_verify_run(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        workdir: &Path,
        steps: &[(String, Verdict)],
    ) {
        if let Some(conductor) = &self.conductor {
            conductor.settle_verify(&spec.plan_id, &task.id, steps, BuildSystem::detect(workdir));
        }
    }

    /// Tick the conductor every `interval` until the returned handle drops.
    /// A `Fail` goes to `on_stop`, once, and ends the ticking. `None` without
    /// [`Self::with_conductor`].
    pub fn spawn_conductor_ticker(
        &self,
        interval: Duration,
        on_stop: impl FnOnce(ConductorStop) + Send + 'static,
    ) -> Option<ConductorTicker> {
        let conductor = self.conductor.clone()?;
        let tui = self.tui_bridge.clone();
        let task = tokio::spawn(async move {
            let mut ticks = tokio::time::interval(interval);
            ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticks.tick().await;
                if let Some(stop) = conductor.tick(tui.as_ref()) {
                    on_stop(stop);
                    return;
                }
            }
        });
        Some(ConductorTicker { task })
    }
}

/// The signals of a settled verify run at `plan_id/task_id`. `steps` are the
/// steps that ran, in order, each with its phase, and `build` says how their
/// test summaries read. When no step ran there is no signal.
///
/// - A `GateVerdict` ([`gate_verdict_signal`]). Its gate is the failed step,
///   with that step's rung, failure kind and failure class; when every step
///   passed, it is [`VERIFY_GATE`] at the last step's rung. Its test count
///   sums the steps' test summaries, which `test-failure-budget` compares
///   across runs.
/// - A `CompileDiagnostic` ([`compile_diagnostic_signal`]) when a compiler
///   error failed the step, naming the first one by code, message and file,
///   which `compile-fail-repeat` compares.
fn verify_run_signals(
    plan_id: &str,
    task_id: &str,
    steps: &[(String, Verdict)],
    build: BuildSystem,
) -> Vec<Signal> {
    let failed = steps.iter().find(|(_, verdict)| !verdict.passed);
    let Some((phase, settled_by)) = failed.or_else(|| steps.last()) else {
        return Vec::new();
    };
    let classification = failed.map(|(phase, verdict)| failure_classification(phase, verdict));
    let failure_kind = classification
        .as_ref()
        .map(|classification| format!("{:?}", classification.failure_kind));
    let failure_class = classification
        .as_ref()
        .and_then(|classification| serde_json::to_value(&classification.primary).ok())
        .and_then(|class| class.as_str().map(str::to_owned));
    let test_count = steps
        .iter()
        .filter_map(|(_, verdict)| roko_gate::parse_test_counts(verdict.detail.as_deref()?, build))
        .reduce(|total, count| {
            TestCount::new(
                total.passed + count.passed,
                total.failed + count.failed,
                total.ignored + count.ignored,
            )
        });
    let gate_verdict = gate_verdict_signal(&GateRun {
        plan_id,
        task_id,
        gate: if failed.is_some() {
            settled_by.gate.as_str()
        } else {
            VERIFY_GATE
        },
        rung: verify_step_rung(phase),
        passed: failed.is_none(),
        failure_kind: failure_kind.as_deref(),
        failure_class: failure_class.as_deref(),
        duration_ms: steps.iter().map(|(_, verdict)| verdict.duration_ms).sum(),
        test_count,
    });
    let compile_diagnostic = classification
        .as_ref()
        .and_then(first_compiler_error)
        .and_then(|message| compile_diagnostic_signal(plan_id, task_id, &message));
    gate_verdict.into_iter().chain(compile_diagnostic).collect()
}

/// A failed step's classification: the one its gate recorded as the
/// verdict's error digest, else one made from its output and its `phase`.
fn failure_classification(phase: &str, verdict: &Verdict) -> GateFailureClassification {
    verdict
        .error_digest
        .as_deref()
        .and_then(|digest| serde_json::from_str(digest).ok())
        .unwrap_or_else(|| {
            roko_gate::classify_step_failure(
                &verdict.gate,
                Some(phase),
                verdict.detail.as_deref().unwrap_or(&verdict.reason),
            )
        })
}

/// The first compiler error a failed step reported, by code, message and
/// file; cargo's own `could not compile` and `test failed` lines are none.
fn first_compiler_error(classification: &GateFailureClassification) -> Option<String> {
    classification
        .compile_errors
        .iter()
        .zip(roko_gate::records_from_classification(classification))
        .find(|(error, _)| error.is_compiler_error())
        .map(|(_, record)| record.digest)
}

/// Log and publish a decision a task's evidence drove as an advisory
/// diagnosis: what the conductor would do to the attempt, and that it did
/// not.
fn publish_advice(
    tui: Option<&TuiBridge>,
    attempt_key: &str,
    plan_id: &str,
    task_id: &str,
    decision: &ConductorDecision,
) {
    let (watcher, reason) = match decision {
        ConductorDecision::Restart { watcher, reason }
        | ConductorDecision::ForceAdvance {
            watcher, reason, ..
        } => (watcher.as_str(), reason.clone()),
        ConductorDecision::Nudge {
            watcher, message, ..
        } => (watcher.as_str(), message.clone()),
        ConductorDecision::Fail { watcher, reason } => (watcher.as_str(), reason.to_string()),
        _ => ("conductor", String::new()),
    };
    let label = decision.label();
    tracing::warn!(
        plan_id,
        task_id,
        attempt = attempt_key,
        watcher,
        decision = label,
        advisory = true,
        "conductor advice from the task's verify runs, not carried out: {reason}"
    );
    if let Some(tui) = tui {
        tui.diagnosis(DiagnosisSummary {
            id: format!("conductor:{attempt_key}:advisory"),
            ts: chrono::Utc::now(),
            severity: DiagnosisSeverity::Warn,
            subject: format!("{plan_id}/{task_id}"),
            detail: format!("conductor watcher `{watcher}` (advisory): {reason}"),
            suggested_action: Some(format!(
                "{label} the attempt; decisions the task's verify runs drive are advisory \
                 until gap-ebd656"
            )),
            intervention_taken: None,
        });
    }
}

/// Publish a conductor intervention as a dashboard diagnosis.
fn publish_intervention(
    tui: Option<&TuiBridge>,
    attempt_key: &str,
    plan_id: &str,
    task_id: &str,
    watcher: &str,
    reason: &str,
    stopped_run: bool,
) {
    let (label, severity, intervention) = if stopped_run {
        ("fail", DiagnosisSeverity::Alert, "stopped the run")
    } else {
        ("restart", DiagnosisSeverity::Warn, "restarted attempt")
    };
    tracing::warn!(
        plan_id,
        task_id,
        attempt = attempt_key,
        watcher,
        decision = label,
        "conductor intervention: {reason}"
    );
    if let Some(tui) = tui {
        tui.diagnosis(DiagnosisSummary {
            id: format!("conductor:{attempt_key}:{label}"),
            ts: chrono::Utc::now(),
            severity,
            subject: format!("{plan_id}/{task_id}"),
            detail: format!("conductor watcher `{watcher}`: {reason}"),
            suggested_action: None,
            intervention_taken: Some(intervention.to_string()),
        });
    }
}

/// One running attempt's link to the conductor. It leaves the running set,
/// and its signals the ring, when dropped.
pub(super) struct SupervisedAttempt {
    conductor: GraphConductor,
    feed: AttemptFeed,
    cancel: CancellationToken,
    restart: Arc<OnceLock<ConductorRestart>>,
}

impl SupervisedAttempt {
    /// A feed of this attempt's live output into the ring, for its tap.
    pub(super) fn feed(&self) -> AttemptFeed {
        AttemptFeed {
            last_text: None,
            ..self.feed.clone()
        }
    }

    /// Resolves once the conductor restarts this attempt, with the reason.
    pub(super) async fn restarted(&self) -> ConductorRestart {
        self.cancel.cancelled().await;
        self.restart
            .get()
            .cloned()
            .unwrap_or_else(|| ConductorRestart {
                watcher: "conductor".to_string(),
                reason: "restart requested".to_string(),
            })
    }

    /// The provider call ended, successfully when `succeeded`. If the
    /// conductor restarted an earlier attempt of the task, this is how the
    /// restart turned out.
    pub(super) fn end(self, succeeded: bool) {
        self.conductor.learn_restart_outcome(
            &self.feed.plan_id,
            &self.feed.task_id,
            &self.feed.attempt_key,
            succeeded,
        );
    }
}

impl Drop for SupervisedAttempt {
    fn drop(&mut self) {
        self.conductor.running.lock().remove(&self.feed.attempt_key);
        self.conductor.forget(&self.feed.attempt_key);
    }
}

/// Feeds one attempt's events into the conductor ring, tagged with its
/// attempt key.
#[derive(Clone)]
pub(super) struct AttemptFeed {
    ring: ConductorRing,
    plan_id: String,
    task_id: String,
    attempt_key: String,
    /// The attempt's previous message, to tell a repeat.
    last_text: Option<String>,
}

impl AttemptFeed {
    pub(super) fn push(&mut self, event: &GraphTaskEvent) {
        let signal = match event {
            GraphTaskEvent::Text { text } => {
                let repeated = self.last_text.as_deref() == Some(text.as_str());
                self.last_text = Some(text.clone());
                graph_text_turn_signal(
                    &self.plan_id,
                    &self.task_id,
                    !repeated && !text.trim().is_empty(),
                )
            }
            event => graph_task_event_to_signal(&self.plan_id, &self.task_id, event),
        };
        if let Some(mut signal) = signal {
            signal
                .tags
                .insert(ATTEMPT_TAG.to_string(), self.attempt_key.clone());
            signal.id = signal.content_hash();
            self.ring.push(signal);
        }
    }

    /// Feed one live-output event: a message or a tool call. Reasoning is
    /// no message, a trusted `ToolCallEnd` repeats its `ToolStep`, and tool
    /// results mean nothing to the watchers.
    pub(super) fn push_live(&mut self, event: &LiveAgentEvent) {
        let event = match event {
            LiveAgentEvent::ToolStep { id, name, .. } => GraphTaskEvent::ToolCall {
                id: id.clone(),
                name: name.clone(),
            },
            LiveAgentEvent::Unscreened(StreamEventKind::TextDelta(text)) => {
                GraphTaskEvent::Text { text: text.clone() }
            }
            LiveAgentEvent::Unscreened(_) => return,
        };
        self.push(&event);
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use roko_core::{Body, Context, React};

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, cli_provider, make_bare_dispatcher, make_scripted_batch_dispatcher,
        make_spec, make_test_dispatcher_with, model, no_auto_fix, verify_step,
    };
    use crate::state_hub::StateHub;

    /// A watcher that fires at `severity` on any stream that has a message.
    struct FiresOnMessages {
        severity: &'static str,
    }

    impl roko_core::Cell for FiresOnMessages {
        fn cell_id(&self) -> &str {
            "fires-on-messages"
        }
        fn cell_name(&self) -> &str {
            "FiresOnMessages"
        }
    }

    impl React for FiresOnMessages {
        fn decide(&self, stream: &[Signal], _ctx: &Context) -> Vec<Signal> {
            if !stream
                .iter()
                .any(|signal| matches!(signal.kind, Kind::Custom(_)))
            {
                return Vec::new();
            }
            vec![
                Signal::builder(Kind::Custom("conductor.intervention".into()))
                    .body(Body::text("the agent said something"))
                    .tag("watcher", "fires-on-messages")
                    .tag("severity", self.severity)
                    .build(),
            ]
        }

        fn name(&self) -> &str {
            "fires-on-messages"
        }
    }

    fn graph_conductor(watchers: Vec<Box<dyn React>>) -> GraphConductor {
        GraphConductor::new(
            Arc::new(Conductor::with_watchers(watchers)),
            ConductorRing::with_capacity(64),
        )
    }

    fn text(text: &str) -> LiveAgentEvent {
        LiveAgentEvent::Unscreened(StreamEventKind::TextDelta(text.to_string()))
    }

    fn attempt_signals(conductor: &GraphConductor, attempt_key: &str) -> usize {
        conductor
            .ring
            .snapshot()
            .iter()
            .filter(|signal| signal.tag(ATTEMPT_TAG) == Some(attempt_key))
            .count()
    }

    #[tokio::test]
    async fn conductor_restart_cancels_the_attempt_it_names() {
        let conductor = graph_conductor(vec![Box::new(
            roko_conductor::watchers::GhostTurnWatcher::default(),
        )]);
        let looping = conductor.supervise("p1", "T01", "run-1/p1/T01/1");
        let working = conductor.supervise("p1", "T02", "run-1/p1/T02/1");
        let mut looping_feed = looping.feed();
        let mut working_feed = working.feed();
        for _ in 0..4 {
            // The same message again and again, with no tool call between:
            // three repeats after the first.
            looping_feed.push_live(&text("I will look at the file."));
            working_feed.push_live(&text("Editing src/lib.rs."));
            working_feed.push_live(&LiveAgentEvent::ToolStep {
                id: "toolu_1".to_string(),
                name: "Edit".to_string(),
                target: "src/lib.rs".to_string(),
            });
        }

        assert_eq!(
            conductor.tick(None),
            None,
            "a restart does not stop the run"
        );
        let restart = tokio::time::timeout(Duration::from_secs(1), looping.restarted())
            .await
            .expect("the looping attempt is restarted");
        assert_eq!(restart.watcher, "ghost-turn");
        assert!(
            restart.error().to_string().contains("ghost-turn"),
            "{}",
            restart.error()
        );
        assert!(
            !working.cancel.is_cancelled(),
            "the other task keeps running"
        );
        assert_eq!(
            attempt_signals(&conductor, "run-1/p1/T01/1"),
            0,
            "the evidence is spent"
        );
        assert!(attempt_signals(&conductor, "run-1/p1/T02/1") > 0);
        assert_eq!(conductor.tick(None), None);

        // The task's next attempt runs to the end: the restart helped.
        drop(looping);
        let retry = conductor.supervise("p1", "T01", "run-1/p1/T01/2");
        retry.end(true);
        let outcomes = conductor
            .conductor
            .with_learner(|learner| learner.intervention_history.clone());
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].watcher_name, "ghost-turn");
        assert!(outcomes[0].task_improved);
    }

    #[test]
    fn conductor_fail_stops_the_run_naming_the_watcher() {
        let conductor = graph_conductor(vec![Box::new(FiresOnMessages {
            severity: "critical",
        })]);
        let attempt = conductor.supervise("p1", "T01", "run-1/p1/T01/1");
        assert_eq!(
            conductor.tick(None),
            None,
            "an attempt that said nothing yet is healthy"
        );

        attempt.feed().push_live(&text("hello"));
        let stop = conductor
            .tick(None)
            .expect("a critical finding stops the run");
        assert_eq!(stop.watcher, "fires-on-messages");
        assert_eq!(
            (stop.plan_id.as_str(), stop.task_id.as_str()),
            ("p1", "T01")
        );
        let message = stop.to_string();
        assert!(message.contains("fires-on-messages"), "{message}");
        assert!(message.contains("the agent said something"), "{message}");
    }

    fn watched() -> WatchedAttempt<'static> {
        WatchedAttempt {
            agent_id: "p1/T01",
            plan_id: "p1",
            task_id: "T01",
            attempt_key: "run-1/p1/T01/1",
            stop: None,
        }
    }

    #[tokio::test]
    async fn conductor_ticker_stops_the_run_naming_the_watcher() {
        let temp = tempfile::tempdir().expect("tempdir");
        let dispatcher = make_bare_dispatcher(RokoConfig::default(), temp.path())
            .await
            .with_conductor(
                Arc::new(Conductor::with_watchers(vec![Box::new(FiresOnMessages {
                    severity: "critical",
                })])),
                ConductorRing::with_capacity(64),
            );
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        let _ticker = dispatcher
            .spawn_conductor_ticker(Duration::from_millis(50), move |stop| {
                let _ = stop_tx.send(stop);
            })
            .expect("the dispatcher is supervised");
        let attempt = dispatcher
            .supervise_attempt(&watched())
            .expect("the dispatcher is supervised");
        attempt.feed().push_live(&text("hello"));

        let stop = tokio::time::timeout(Duration::from_secs(5), stop_rx)
            .await
            .expect("the ticker stops the run")
            .expect("the stop is handed over");
        assert_eq!(stop.watcher, "fires-on-messages");
        assert!(
            stop.to_string().contains("the agent said something"),
            "{stop}"
        );
    }

    #[tokio::test]
    async fn with_both_thresholds_off_nothing_is_supervised() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = RokoConfig::default();
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 0;
        let dispatcher = make_bare_dispatcher(config, temp.path())
            .await
            .with_conductor(Arc::new(Conductor::default()), ConductorRing::new());

        assert!(dispatcher.stall_watch().is_none());
        assert!(dispatcher.supervise_attempt(&watched()).is_none());
        assert!(
            dispatcher
                .spawn_conductor_ticker(SUPERVISION_INTERVAL, |_| {})
                .is_none()
        );
        assert!(
            dispatcher.live_output_tap(&watched(), None, None).is_none(),
            "no TUI, no watchdog and no conductor: no live output"
        );
    }

    /// A step of phase `phase` that ran as gate `label` and printed `output`,
    /// with the failure classification `ShellGate` records when it failed.
    fn ran_step(phase: &str, label: &str, passed: bool, output: &str) -> (String, Verdict) {
        let verdict = if passed {
            Verdict::pass(label)
        } else {
            Verdict::fail(label, "exit code: 101").with_error_digest(
                roko_gate::render_failure_classification(&roko_gate::classify_step_failure(
                    label,
                    Some(phase),
                    output,
                )),
            )
        };
        (
            phase.to_string(),
            verdict.with_detail(output).with_duration(10),
        )
    }

    fn signal_body(signal: &Signal) -> serde_json::Value {
        signal.body.as_json().expect("json body")
    }

    #[test]
    fn settled_verify_runs_carry_what_the_gate_watchers_read() {
        let signals = |steps: &[(String, Verdict)]| {
            verify_run_signals("p1", "T01", steps, BuildSystem::Cargo)
        };
        let compiled = ran_step("compile", "verify[0:compile]", true, "Finished `dev`");

        // A compiler error: the failed step is the gate, with its rung and
        // class, and its first compiler error the diagnostic.
        let compile_error = ran_step(
            "compile",
            "verify[0:compile]",
            false,
            "error[E0308]: mismatched types\n --> src/lib.rs:3:5\n\
             error: could not compile `demo` (lib) due to 1 previous error",
        );
        let run = signals(std::slice::from_ref(&compile_error));
        assert_eq!(run.len(), 2, "{run:?}");
        assert_eq!(run[0].kind, Kind::GateVerdict);
        assert_eq!(run[0].tag("severity"), Some("error"));
        let verdict = signal_body(&run[0]);
        assert_eq!(verdict["plan_id"], "p1");
        assert_eq!(verdict["task"], "T01");
        assert_eq!(verdict["gate"], "verify[0:compile]");
        assert_eq!(verdict["rung"], 0);
        assert_eq!(verdict["passed"], false);
        assert_eq!(verdict["failure_class"], "type_error");
        assert!(verdict["failure_kind"].is_string(), "{verdict}");
        assert!(verdict["test_count"].is_null(), "no test ran: {verdict}");
        assert_eq!(run[1].kind, Kind::CompileDiagnostic);
        assert_eq!(signal_body(&run[1])["message"], "E0308: mismatched types");

        // Failing tests: counted, and cargo's `test failed` line is no
        // compiler error.
        let failing_tests = ran_step(
            "test",
            "verify[1:test]",
            false,
            "test result: FAILED. 3 passed; 2 failed; 0 ignored\n\
             error: test failed, to rerun pass `--lib`",
        );
        let run = signals(&[compiled.clone(), failing_tests]);
        assert_eq!(run.len(), 1, "{run:?}");
        let verdict = signal_body(&run[0]);
        assert_eq!(verdict["gate"], "verify[1:test]");
        assert_eq!(verdict["rung"], 2);
        assert_eq!(verdict["failure_class"], "test_expectation_failure");
        assert_eq!(verdict["duration_ms"], 20);
        assert_eq!(verdict["test_count"]["passed"], 3);
        assert_eq!(verdict["test_count"]["failed"], 2);

        // Every step passed.
        let passing_tests = ran_step(
            "test",
            "verify[1:test]",
            true,
            "test result: ok. 5 passed; 0 failed; 1 ignored",
        );
        let run = signals(&[compiled, passing_tests]);
        assert_eq!(run.len(), 1, "{run:?}");
        assert_eq!(run[0].tag("severity"), Some("info"));
        let verdict = signal_body(&run[0]);
        assert_eq!(verdict["gate"], VERIFY_GATE);
        assert_eq!(verdict["passed"], true);
        assert!(verdict["failure_class"].is_null(), "{verdict}");
        assert_eq!(verdict["test_count"]["failed"], 0);

        assert!(signals(&[]).is_empty(), "no step ran, no verdict");
    }

    /// A Claude CLI stand-in that records the model of each call in
    /// `provider-models` beside it and, while `hold` exists there, waits for
    /// `go` before it answers.
    const HELD_PROVIDER: &str = r#"#!/bin/sh
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
if [ -f "$dir/hold" ]; then
  while [ ! -f "$dir/go" ]; do sleep 0.05; done
fi
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"held-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-h","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// A verify step whose failing tests grow by two on each run, from one.
    const GROWING_TEST_FAILURES: &str = "n=$(cat failing 2>/dev/null || echo 1); \
         echo $((n + 2)) > failing; \
         echo \"test result: FAILED. 4 passed; $n failed; 0 ignored\"; exit 101";

    /// Dispatches `attempts` attempts at `spec`, each failing its verify.
    async fn fail_attempts(
        dispatcher: &GraphTaskDispatcher,
        spec: &TaskExecutionSpec,
        attempts: usize,
    ) {
        for _ in 0..attempts {
            let error = dispatcher
                .dispatch(spec, Vec::new(), &CellContext::new())
                .await
                .expect_err("the verify step fails");
            assert!(matches!(error, RokoError::Verify { .. }), "{error}");
        }
    }

    /// Dispatches the next attempt at `spec` and ticks the conductor while
    /// it runs, its [`HELD_PROVIDER`] call held in `dir` until the tick is
    /// done; returns how the attempt ended.
    async fn attempt_under_a_tick(
        dispatcher: &Arc<GraphTaskDispatcher>,
        spec: &TaskExecutionSpec,
        dir: &Path,
    ) -> RokoError {
        std::fs::write(dir.join("hold"), "").expect("hold the provider");
        let attempt = tokio::spawn({
            let dispatcher = Arc::clone(dispatcher);
            let spec = spec.clone();
            async move {
                dispatcher
                    .dispatch(&spec, Vec::new(), &CellContext::new())
                    .await
            }
        });
        let conductor = dispatcher
            .conductor
            .as_ref()
            .expect("the dispatcher is supervised");
        tokio::time::timeout(Duration::from_secs(60), async {
            while conductor.running.lock().is_empty() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the attempt starts");
        assert_eq!(
            conductor.tick(dispatcher.tui_bridge.as_ref()),
            None,
            "advice never stops the run"
        );
        std::fs::write(dir.join("go"), "").expect("release the provider");
        let ended = attempt
            .await
            .expect("the attempt's task")
            .expect_err("the attempt's verify step fails");
        for file in ["hold", "go"] {
            std::fs::remove_file(dir.join(file)).expect("reset the provider");
        }
        ended
    }

    /// The advisory diagnoses published for `plan/task`, by detail.
    fn advice(hub: &StateHub, task: &str) -> Vec<String> {
        hub.current_snapshot()
            .diagnoses
            .iter()
            .filter(|diagnosis| {
                diagnosis.subject == format!("stream-plan/{task}")
                    && diagnosis.id.ends_with(":advisory")
            })
            .map(|diagnosis| diagnosis.detail.clone())
            .collect()
    }

    /// Settled Graph verify runs reach the conductor's watchers for compile
    /// and test failures (gap-1a7f9c), and what they find is advice: the
    /// next attempt of a task whose verify failed three times on the same
    /// compiler error, or whose failing tests grew, is advised on by
    /// `compile-fail-repeat` or `test-failure-budget` and runs to its end.
    /// Advice spends the evidence it was given on, and teaches nothing.
    #[tokio::test]
    async fn conductor_watchers_receive_graph_gate_verdicts() {
        let temp = tempfile::tempdir().expect("tempdir");
        let hub = StateHub::new(64);
        let (dispatcher, task) = make_test_dispatcher_with(
            &temp,
            HELD_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
            |dispatcher| {
                let conductor = Conductor::from_config(&dispatcher.config.conductor);
                dispatcher
                    .with_tui_bridge(TuiBridge::new(hub.sender()))
                    .with_conductor(Arc::new(conductor), ConductorRing::new())
            },
        )
        .await;

        let mut compile = task.clone();
        compile.id = "T-COMPILE".to_string();
        compile.verify = vec![verify_step(
            "compile",
            "printf 'error[E0308]: mismatched types\\n' >&2; exit 101",
        )];
        let spec = make_spec(&compile);
        fail_attempts(&dispatcher, &spec, 2).await;
        let ended = attempt_under_a_tick(&dispatcher, &spec, temp.path()).await;
        assert!(matches!(ended, RokoError::Verify { .. }), "{ended}");
        assert!(
            advice(&hub, "T-COMPILE").is_empty(),
            "two identical compile failures are not a repeat yet"
        );
        let ended = attempt_under_a_tick(&dispatcher, &spec, temp.path()).await;
        assert!(
            matches!(ended, RokoError::Verify { .. }),
            "the advised attempt runs to its verify: {ended}"
        );
        let compile_advice = advice(&hub, "T-COMPILE");
        assert_eq!(compile_advice.len(), 1, "{compile_advice:?}");
        assert!(
            compile_advice[0].contains("compile-fail-repeat")
                && compile_advice[0].contains("E0308: mismatched types"),
            "{compile_advice:?}"
        );

        let mut tests = task;
        tests.id = "T-TESTS".to_string();
        tests.verify = vec![verify_step("test", GROWING_TEST_FAILURES)];
        let spec = make_spec(&tests);
        fail_attempts(&dispatcher, &spec, 2).await;
        let ended = attempt_under_a_tick(&dispatcher, &spec, temp.path()).await;
        assert!(matches!(ended, RokoError::Verify { .. }), "{ended}");
        let test_advice = advice(&hub, "T-TESTS");
        assert_eq!(test_advice.len(), 1, "{test_advice:?}");
        assert!(
            test_advice[0].contains("test-failure-budget") && test_advice[0].contains("1 -> 3"),
            "{test_advice:?}"
        );
        // The advice spent the runs before it: the advised attempt's own run
        // is the next attempt's only evidence, and repeats no growth.
        attempt_under_a_tick(&dispatcher, &spec, temp.path()).await;
        assert_eq!(advice(&hub, "T-TESTS").len(), 1);

        let diagnoses = hub.current_snapshot().diagnoses;
        assert!(
            diagnoses
                .iter()
                .all(|diagnosis| diagnosis.intervention_taken.is_none()),
            "{diagnoses:#?}"
        );
        let conductor = dispatcher.conductor.as_ref().expect("supervised");
        assert!(
            conductor
                .conductor
                .with_learner(|learner| learner.intervention_history.is_empty()),
            "advice teaches the conductor nothing"
        );
    }

    /// The conductor's advice never cancels an attempt, here the escalation
    /// of a task whose failing tests grew on the cheap rung of a two-rung
    /// model ladder.
    #[tokio::test]
    async fn gate_advice_never_cancels_a_ladder_escalation() {
        const CHEAP: &str = "claude-haiku-4-5";
        const TOP: &str = "claude-sonnet-4-6";
        let rung = |name: &str, model: &str| roko_core::config::routing::LadderRung {
            name: name.to_string(),
            model: model.to_string(),
        };
        let temp = tempfile::tempdir().expect("tempdir");
        let helper = temp.path().join("helper.sh");
        std::fs::write(&helper, VERIFY_PROVIDER).expect("write the helper provider");
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755))
            .expect("make the helper executable");
        let (dispatcher, mut task) =
            make_scripted_batch_dispatcher(&temp, HELD_PROVIDER, |config| {
                no_auto_fix(config);
                config
                    .models
                    .insert("cheap-model".to_string(), model("batch-cli", CHEAP, None));
                config.routing.ladder.rungs =
                    vec![rung("cheap", "cheap-model"), rung("top", "batch-model")];
                // Helper model calls go to their own provider, so the held
                // one records only the attempts.
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
        let hub = StateHub::new(64);
        let conductor = Conductor::from_config(&dispatcher.config.conductor);
        let dispatcher = Arc::new(
            dispatcher
                .with_tui_bridge(TuiBridge::new(hub.sender()))
                .with_conductor(Arc::new(conductor), ConductorRing::new()),
        );
        task.model_hint = None;
        task.tier = "mechanical".to_string();
        task.verify = vec![verify_step("test", GROWING_TEST_FAILURES)];
        let mut spec = make_spec(&task);
        spec.max_retries = 4;

        fail_attempts(&dispatcher, &spec, 2).await;
        let ended = attempt_under_a_tick(&dispatcher, &spec, temp.path()).await;

        let models = std::fs::read_to_string(temp.path().join("provider-models"))
            .expect("the provider recorded its calls");
        assert_eq!(
            models.lines().collect::<Vec<_>>(),
            [CHEAP, CHEAP, TOP],
            "the third attempt is the ladder's escalation"
        );
        assert!(
            matches!(ended, RokoError::Verify { .. }),
            "the escalated attempt runs to its verify: {ended}"
        );
        let advised = advice(&hub, &task.id);
        assert_eq!(advised.len(), 1, "{advised:?}");
        assert!(advised[0].contains("test-failure-budget"), "{advised:?}");
    }

    #[test]
    fn ended_attempts_leave_the_ring_and_are_not_evaluated() {
        let conductor = graph_conductor(vec![Box::new(FiresOnMessages {
            severity: "critical",
        })]);
        let attempt = conductor.supervise("p1", "T01", "run-1/p1/T01/1");
        attempt.feed().push_live(&text("hello"));
        attempt.end(true);
        assert!(conductor.ring.is_empty());
        assert_eq!(conductor.tick(None), None);
        assert!(
            conductor
                .conductor
                .with_learner(|learner| learner.intervention_history.is_empty()),
            "no restart, nothing to learn"
        );
    }
}
