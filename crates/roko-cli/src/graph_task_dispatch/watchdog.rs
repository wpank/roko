//! The per-attempt stall watchdog (spec-a0403b, part A).
//!
//! Without it the hard `timeout_secs` is the only bound on an agent run, so a
//! provider that hangs, or a model that stops answering, holds its task slot
//! until that deadline. While a provider runs, the attempt's live-output tap
//! ([`GraphTaskDispatcher::live_output_tap`]) stamps every event it receives
//! into an [`AttemptProgress`], and [`GraphTaskDispatcher::run_watched`] checks
//! a [`StallWatch`] over it every [`STALL_CHECK_INTERVAL`]:
//!
//! - silent for `[conductor] silence_timeout_secs`: it publishes a warning
//!   [`DiagnosisSummary`], once per silence;
//! - silent for `[conductor] task_stall_secs`: it drops the dispatch future,
//!   which cancels the provider call, publishes a diagnosis saying so, and the
//!   attempt fails as stalled ([`AttemptInterrupted::error`]) and settles as a
//!   timeout ([`failed_call_settlement`]), so the Graph engine retries it under
//!   the task's `max_retries`.
//!
//! `0` turns a threshold off, and with both off nothing is watched. The hard
//! `timeout_secs` stays the outer bound either way. The attempt's progress is
//! kept either way, so a call that is stopped or cancelled settles the usage
//! it streamed (bug-3a3b0f).
//!
//! A plan run that outlives its interrupt's drain asks its attempts to stop
//! ([`WatchedAttempt::stop`]): `run_watched` drops the call within
//! [`STOP_CHECK_INTERVAL`], and the attempt settles as cancelled with the usage
//! its progress saw stream (bug-2b1ddc). The operator can stop one task's
//! attempt the same way ([`OperatorStops`], gap-c002bb); it is not retried.
//!
//! Silence counts only while the agent waits on its model:
//!
//! - every event the agent streams is progress, text and reasoning deltas
//!   included;
//! - a provider whose adapter streams as it goes (the Claude CLI) is silent
//!   from the start of its call once [`FIRST_OUTPUT_GRACE`] has passed, so one
//!   that never reports anything is cancelled too (bug-2aa55f);
//! - other providers may report nothing until they finish (the Codex CLI hands
//!   over its whole answer at the end), so their attempts are left to the hard
//!   timeout until they first report something;
//! - a tool call the agent made (a long `cargo test`, say) is bounded by the
//!   provider's own tool timeout, so silence while one runs does not count.

use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use roko_agent::StreamEventKind;
use roko_agent::live_output::{LiveAgentEvent, LiveOutput};
use roko_core::config::schema::ConductorConfig;
use roko_core::{DiagnosisSeverity, DiagnosisSummary};

use super::failover::FailoverChain;
use super::supervision::{AttemptFeed, ConductorRestart, SupervisedAttempt};
use super::*;

/// How often a running attempt's [`StallWatch`] is checked. The thresholds
/// are whole seconds, so this is precise enough.
const STALL_CHECK_INTERVAL: Duration = Duration::from_secs(1);

/// How often a running attempt checks whether its plan run asked it to stop
/// ([`WatchedAttempt::stop`]).
const STOP_CHECK_INTERVAL: Duration = Duration::from_millis(250);

/// How long a call whose provider streams as it goes may report nothing from
/// its start before that counts as silence. Starting the CLI and the model's
/// first token take longer than the gap between two events, and much longer
/// under load, so a short `task_stall_secs` would otherwise cancel a call that
/// is only starting.
pub(super) const FIRST_OUTPUT_GRACE: Duration = Duration::from_secs(30);

/// Capacity of the channel between the provider boundary and the live-output
/// tap. The boundary never waits on it: events that do not fit are dropped.
const LIVE_OUTPUT_CHANNEL_CAPACITY: usize = 64;

/// What the dashboard names the watchdog's cancel.
const CANCELLED_STALLED_ATTEMPT: &str = "cancelled stalled attempt";

/// The watchdog's thresholds, from `[conductor]`; `None` is off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StallThresholds {
    /// Silence after which a warning diagnosis goes out
    /// (`silence_timeout_secs`).
    warn_after: Option<Duration>,
    /// Silence after which the attempt is cancelled (`task_stall_secs`).
    cancel_after: Option<Duration>,
}

impl StallThresholds {
    pub(super) fn from_config(config: &ConductorConfig) -> Self {
        let threshold = |secs: u64| (secs > 0).then(|| Duration::from_secs(secs));
        Self {
            warn_after: threshold(config.silence_timeout_secs),
            cancel_after: threshold(config.task_stall_secs),
        }
    }

    /// Whether anything is watched at all.
    pub(super) const fn is_enabled(self) -> bool {
        self.warn_after.is_some() || self.cancel_after.is_some()
    }
}

/// What one attempt's live output says about its progress: written by its
/// live-output tap, read by its [`StallWatch`].
#[derive(Debug, Clone, Default)]
pub(super) struct AttemptProgress {
    inner: Arc<parking_lot::Mutex<ProgressState>>,
}

#[derive(Debug, Default)]
struct ProgressState {
    /// When the attempt last reported anything; `None` until it first does.
    last_event: Option<Instant>,
    /// When the call in flight started, if its provider streams as it goes:
    /// the start of its silence until it reports anything.
    quiet_since: Option<Instant>,
    /// The call waits for its provider's concurrency permit, which is no
    /// silence (bug-eba31d).
    queued: bool,
    /// The first-output grace of this attempt's calls; `None` is
    /// [`FIRST_OUTPUT_GRACE`].
    first_output_grace: Option<Duration>,
    /// Tool calls the agent made whose results have not arrived yet.
    open_tool_calls: HashSet<String>,
    /// The provider call the attempt waits on, once one started.
    call: Option<CallInFlight>,
    /// What that call streamed of its token usage.
    usage: StreamedUsage,
    /// The watchdog, the conductor or a stopping plan run cancelled the
    /// call.
    interrupted: bool,
}

/// A provider call in flight: its target, and the models failover passed
/// over before it.
#[derive(Debug, Clone)]
struct CallInFlight {
    target: crate::dispatch_v2::ProviderDispatchSpec,
    failover: FailoverChain,
}

/// A provider call's usage as it streams. Within one model call each
/// `Usage` event is its usage so far, and `Done` ends the model call, so the
/// total is the sum of each model call's last `Usage`. A CLI agent's run is
/// one model call to the stream.
#[derive(Debug, Default)]
struct StreamedUsage {
    /// The model calls that ended.
    ended: Option<roko_core::Usage>,
    /// The model call still streaming.
    open: Option<roko_core::Usage>,
}

impl StreamedUsage {
    fn total(&self) -> Option<roko_core::Usage> {
        match (self.ended, self.open) {
            (Some(mut ended), Some(open)) => {
                ended.add(&open);
                Some(ended)
            }
            (ended, open) => ended.or(open),
        }
    }
}

/// A provider call the watchdog, the conductor or a stopping plan run
/// cancelled, with what it streamed of its usage (bug-aa2044, bug-2b1ddc).
#[derive(Debug)]
pub(super) struct InterruptedCall {
    call: CallInFlight,
    usage: Option<roko_core::Usage>,
}

impl InterruptedCall {
    /// The call as an unsuccessful dispatch that failed with `message`, so
    /// the attempt's records account it like any failed call: its streamed
    /// usage, estimated since the provider never reported a total, or
    /// unknown when it streamed none.
    pub(super) fn into_dispatch(
        self,
        message: &str,
        wall_ms: u64,
        snapshot: Option<&roko_core::pricing_snapshot::PriceSnapshot>,
    ) -> (crate::dispatch_v2::AgentResultDispatch, FailoverChain) {
        let target = self.call.target;
        let usage_obs = match self.usage {
            Some(mut usage) => {
                crate::dispatch_v2::fill_usage_cost_from_pricing(
                    &mut usage,
                    snapshot,
                    target.model_profile.as_ref(),
                    &target.model_slug,
                );
                roko_core::UsageObservation {
                    source: roko_core::UsageSource::Estimated,
                    wall_ms,
                    ..usage.into()
                }
            }
            None => roko_core::UsageObservation {
                wall_ms,
                ..roko_core::UsageObservation::default()
            },
        };
        let output = Signal::builder(roko_core::Kind::AgentOutput)
            .body(roko_core::Body::text(message))
            .tag("failed", "true")
            .build();
        let dispatch = crate::dispatch_v2::AgentResultDispatch {
            target,
            result: roko_agent::AgentResult::fail(output).with_usage_obs(usage_obs),
            events: Vec::new(),
            tool_calls: Vec::new(),
            tool_policy: None,
        };
        (dispatch, self.call.failover)
    }
}

impl AttemptProgress {
    /// Record `event`, received now.
    pub(super) fn observe(&self, event: &LiveAgentEvent) {
        self.observe_at(event, Instant::now());
    }

    fn observe_at(&self, event: &LiveAgentEvent, at: Instant) {
        let mut state = self.inner.lock();
        if let LiveAgentEvent::Queued { waiting } = event {
            // A call waiting for its provider's permit is queued, not silent.
            // Once it has the permit it starts: its silence, and its
            // first-output grace, count from then (bug-eba31d). Neither is
            // progress the agent reported.
            state.queued = *waiting;
            if !*waiting {
                state.last_event = state.last_event.map(|_| at);
                state.quiet_since = state.quiet_since.map(|_| at);
            }
            return;
        }
        state.last_event = Some(at);
        state.queued = false;
        match event {
            LiveAgentEvent::ToolStep { id, .. }
            | LiveAgentEvent::Unscreened(StreamEventKind::ToolCallEnd { id, .. }) => {
                state.open_tool_calls.insert(id.clone());
            }
            LiveAgentEvent::Unscreened(StreamEventKind::ToolResult { id, .. }) => {
                state.open_tool_calls.remove(id);
            }
            LiveAgentEvent::Unscreened(StreamEventKind::Usage(usage)) => {
                state.usage.open = Some(*usage);
            }
            LiveAgentEvent::Unscreened(StreamEventKind::Done { .. }) => {
                if let Some(open) = state.usage.open.take() {
                    let ended = state.usage.ended.get_or_insert_with(Default::default);
                    ended.add(&open);
                }
            }
            LiveAgentEvent::Unscreened(_) | LiveAgentEvent::Queued { .. } => {}
        }
    }

    /// The attempt's provider call to `target` starts, after failover passed
    /// over `failover`'s models: the usage that streams from now on is its.
    pub(super) fn call_started(
        &self,
        target: crate::dispatch_v2::ProviderDispatchSpec,
        failover: FailoverChain,
    ) {
        self.call_started_at(target, failover, Instant::now());
    }

    fn call_started_at(
        &self,
        target: crate::dispatch_v2::ProviderDispatchSpec,
        failover: FailoverChain,
        at: Instant,
    ) {
        let mut state = self.inner.lock();
        state.quiet_since = streams_as_it_goes(target.provider_kind).then_some(at);
        state.queued = false;
        state.call = Some(CallInFlight { target, failover });
        state.usage = StreamedUsage::default();
    }

    /// Whether the attempt reported anything yet: after it did, a timeout is
    /// the agent's, before it the provider's.
    pub(super) fn reported_progress(&self) -> bool {
        self.inner.lock().last_event.is_some()
    }

    /// The watchdog, the conductor or a stopping plan run cancelled the call
    /// in flight.
    fn interrupted(&self) {
        self.inner.lock().interrupted = true;
    }

    /// The call the watchdog, the conductor or a stopping plan run
    /// cancelled, with the usage it streamed; `None` when none was
    /// cancelled.
    pub(super) fn interrupted_call(&self) -> Option<InterruptedCall> {
        let state = self.inner.lock();
        let call = state.call.clone().filter(|_| state.interrupted)?;
        Some(InterruptedCall {
            call,
            usage: state.usage.total(),
        })
    }

    /// How long, at `now`, the attempt has been waiting on its model without
    /// reporting progress: since its last event or, when its provider streams
    /// as it goes, since its call started, once its first-output grace
    /// ([`FIRST_OUTPUT_GRACE`]) has passed. `None` within that grace, while
    /// the call waits for its provider's permit, before a provider that may
    /// report only at the end first reports anything, and while a tool call
    /// the agent made is still running.
    fn silence(&self, now: Instant) -> Option<Duration> {
        let state = self.inner.lock();
        if state.queued {
            return None;
        }
        let grace = state.first_output_grace.unwrap_or(FIRST_OUTPUT_GRACE);
        let quiet_since = match state.last_event {
            Some(last_event) => last_event,
            None => state
                .quiet_since
                .filter(|&started| now.saturating_duration_since(started) >= grace)?,
        };
        state
            .open_tool_calls
            .is_empty()
            .then(|| now.saturating_duration_since(quiet_since))
    }
}

/// Whether the adapter of a `kind` provider streams the agent's events as it
/// goes, so that a call reporting nothing for a while is silent. The Claude
/// CLI does; providers that may report only at the end (the Codex CLI, the
/// Cursor CLI) do not.
const fn streams_as_it_goes(kind: roko_core::agent::ProviderKind) -> bool {
    matches!(kind, roko_core::agent::ProviderKind::ClaudeCli)
}

/// One attempt's stall check.
#[derive(Debug)]
pub(super) struct StallWatch {
    thresholds: StallThresholds,
    progress: AttemptProgress,
    /// A warning went out for the current silence.
    warned: bool,
}

/// What a [`StallWatch`] check found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StallCheck {
    /// Recent progress, a running tool call, no progress reported yet, or a
    /// silence that was already warned about.
    Healthy,
    /// Silent past `silence_timeout_secs`, for this long: warn.
    Silent(Duration),
    /// Silent past `task_stall_secs`, for this long: cancel the attempt.
    Stalled(Duration),
}

impl StallWatch {
    pub(super) fn new(thresholds: StallThresholds) -> Self {
        Self {
            thresholds,
            progress: AttemptProgress::default(),
            warned: false,
        }
    }

    /// This watch with `grace` as its calls' first-output grace.
    pub(super) fn with_first_output_grace(self, grace: Duration) -> Self {
        self.progress.inner.lock().first_output_grace = Some(grace);
        self
    }

    /// The progress this watch reads, for the attempt's live-output tap.
    pub(super) fn progress(&self) -> AttemptProgress {
        self.progress.clone()
    }

    pub(super) fn check(&mut self, now: Instant) -> StallCheck {
        let Some(silent_for) = self.progress.silence(now) else {
            self.warned = false;
            return StallCheck::Healthy;
        };
        if self
            .thresholds
            .cancel_after
            .is_some_and(|cancel_after| silent_for >= cancel_after)
        {
            return StallCheck::Stalled(silent_for);
        }
        if !self
            .thresholds
            .warn_after
            .is_some_and(|warn_after| silent_for >= warn_after)
        {
            self.warned = false;
            return StallCheck::Healthy;
        }
        if std::mem::replace(&mut self.warned, true) {
            StallCheck::Healthy
        } else {
            StallCheck::Silent(silent_for)
        }
    }
}

/// The attempt a watchdog watches.
#[derive(Debug, Clone, Copy)]
pub(super) struct WatchedAttempt<'a> {
    /// The dashboard's agent id (`plan/cell`).
    pub(super) agent_id: &'a str,
    pub(super) plan_id: &'a str,
    pub(super) task_id: &'a str,
    /// Durable attempt key, which names the attempt's diagnoses.
    pub(super) attempt_key: &'a str,
    /// The plan run's request that its attempts stop (the cell context's
    /// cancel flag). Once set, the attempt's provider call is dropped and the
    /// attempt settles with the usage its progress saw stream (bug-2b1ddc,
    /// bug-3a3b0f).
    pub(super) stop: Option<&'a AtomicBool>,
}

/// Why [`GraphTaskDispatcher::run_watched`] ended an attempt before its
/// provider call did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AttemptInterrupted {
    /// The stall watchdog cancelled it after this long without progress.
    Stalled(Duration),
    /// The conductor restarted it.
    Restarted(ConductorRestart),
    /// Its plan run is stopping (an interrupt the attempt outlived).
    Stopped,
    /// The operator stopped its task ([`OperatorStops::stop`]).
    StoppedByOperator,
}

impl AttemptInterrupted {
    /// The error the attempt fails with. The Graph engine retries a stall's
    /// timeout under the task's `max_retries`, but not the cancellation of a
    /// stopping run.
    pub(super) fn error(&self, attempt: &WatchedAttempt<'_>) -> RokoError {
        match self {
            Self::Stalled(silent_for) => RokoError::Timeout {
                operation: format!(
                    "agent for {}/{} stalled: no progress for {}s, so the stall watchdog \
                     cancelled the attempt ([conductor] task_stall_secs)",
                    attempt.plan_id,
                    attempt.task_id,
                    silent_for.as_secs()
                ),
                timeout_ms: u64::try_from(silent_for.as_millis()).unwrap_or(u64::MAX),
            },
            Self::Restarted(restart) => restart.error(),
            Self::Stopped => RokoError::cancelled(format!(
                "agent for {}/{} stopped: its plan run is stopping",
                attempt.plan_id, attempt.task_id
            )),
            Self::StoppedByOperator => RokoError::cancelled(format!(
                "agent for {}/{} stopped by the operator",
                attempt.plan_id, attempt.task_id
            )),
        }
    }

    /// How the streaming path reports the attempt's end.
    pub(super) const fn outcome(&self) -> TaskDispatchOutcomeKind {
        match self {
            Self::Stalled(_) => TaskDispatchOutcomeKind::TimedOut,
            Self::Restarted(_) | Self::Stopped | Self::StoppedByOperator => {
                TaskDispatchOutcomeKind::Cancelled
            }
        }
    }
}

/// How an attempt whose provider call failed with `error` settles, the one
/// path for every such call. `interrupted` is what ended the call, when
/// [`GraphTaskDispatcher::run_watched`] did; `error` is then its
/// [`AttemptInterrupted::error`].
///
/// - A stall is a timeout, whatever the error's text says (bug-4c553b): the
///   agent's once `progress` shows it reported anything, the provider's
///   before.
/// - A cancellation, a stopping plan run's or the operator's included,
///   teaches nothing (bug-2b1ddc).
/// - Anything else, a conductor restart included, is a provider failure.
pub(super) fn failed_call_settlement(
    interrupted: Option<&AttemptInterrupted>,
    error: &RokoError,
    progress: Option<&AttemptProgress>,
) -> Settlement {
    match interrupted {
        Some(AttemptInterrupted::Stalled(_)) => Settlement::stalled(
            &error.to_string(),
            progress.is_some_and(AttemptProgress::reported_progress),
        ),
        Some(
            AttemptInterrupted::Restarted(_)
            | AttemptInterrupted::Stopped
            | AttemptInterrupted::StoppedByOperator,
        )
        | None => Settlement::provider_call_error(error),
    }
}

/// The dashboard diagnosis of a silent attempt, or of one the watchdog
/// cancelled.
fn stall_diagnosis(
    attempt: &WatchedAttempt<'_>,
    thresholds: StallThresholds,
    silent_for: Duration,
    cancelled: bool,
) -> DiagnosisSummary {
    let secs = |threshold: Option<Duration>| threshold.map_or(0, |threshold| threshold.as_secs());
    let silent_secs = silent_for.as_secs();
    let subject = format!("{}/{}", attempt.plan_id, attempt.task_id);
    if cancelled {
        return DiagnosisSummary {
            id: format!("stall-watchdog:{}:cancelled", attempt.attempt_key),
            ts: chrono::Utc::now(),
            severity: DiagnosisSeverity::Alert,
            subject,
            detail: format!(
                "agent stalled: no progress for {silent_secs}s ([conductor] task_stall_secs = {}); \
                 the task retries under its max_retries",
                secs(thresholds.cancel_after)
            ),
            suggested_action: None,
            intervention_taken: Some(CANCELLED_STALLED_ATTEMPT.to_string()),
        };
    }
    let suggested_action = thresholds.cancel_after.map_or_else(
        || {
            "check the provider: with [conductor] task_stall_secs = 0 the attempt runs until its \
             timeout_secs"
                .to_string()
        },
        |cancel_after| {
            format!(
                "the attempt is cancelled and retried after {}s of silence",
                cancel_after.as_secs()
            )
        },
    );
    DiagnosisSummary {
        id: format!("stall-watchdog:{}:silent", attempt.attempt_key),
        ts: chrono::Utc::now(),
        severity: DiagnosisSeverity::Warn,
        subject,
        detail: format!(
            "agent silent: no progress for {silent_secs}s ([conductor] silence_timeout_secs = {})",
            secs(thresholds.warn_after)
        ),
        suggested_action: Some(suggested_action),
        intervention_taken: None,
    }
}

impl GraphTaskDispatcher {
    /// A stall watch for an attempt about to start; `None` when
    /// `[conductor] silence_timeout_secs` and `task_stall_secs` are both 0.
    pub(super) fn stall_watch(&self) -> Option<StallWatch> {
        let thresholds = StallThresholds::from_config(&self.config.conductor);
        thresholds
            .is_enabled()
            .then(|| StallWatch::new(thresholds).with_first_output_grace(self.first_output_grace))
    }

    /// Give each call `grace` before its silence counts, instead of
    /// [`FIRST_OUTPUT_GRACE`].
    #[cfg(test)]
    pub(super) fn with_first_output_grace(mut self, grace: Duration) -> Self {
        self.first_output_grace = grace;
        self
    }

    /// The live-output channel for an attempt's dispatch request, when
    /// anything reads it: the TUI, when both [`Self::with_tui_bridge`] and
    /// [`Self::with_live_agent_output`] are set, the attempt's stall watch,
    /// through `progress`, or the run's conductor, through `feed`. Spawns the
    /// tap that reads it, which ends when the dispatch drops its end.
    ///
    /// The watchdog and the conductor ask the provider boundary for
    /// unscreened text, reasoning and tool results as well: the watchdog to
    /// tell a model that is thinking from a tool that is running, the
    /// conductor to see the agent's messages. They stay in the tap: the TUI
    /// still gets them only under [`LiveAgentOutput::Trusted`].
    ///
    /// A tap also records the tool calls the provider streams in
    /// `tool_calls` (bug-264c41), with the outcomes its tool results report
    /// once the tap is trusted. Recording alone opens no tap, and widens no
    /// trust: a tap makes the provider stream its call.
    pub(super) fn live_output_tap(
        &self,
        attempt: &WatchedAttempt<'_>,
        progress: Option<AttemptProgress>,
        feed: Option<AttemptFeed>,
        tool_calls: Option<LiveToolCalls>,
    ) -> Option<LiveOutput> {
        let tui = self.tui_bridge.clone().zip(self.live_agent_output);
        if tui.is_none() && progress.is_none() && feed.is_none() {
            return None;
        }
        let forward_unscreened = matches!(tui, Some((_, LiveAgentOutput::Trusted)));
        let trusted = forward_unscreened || progress.is_some() || feed.is_some();
        let (sink, mut events) =
            tokio::sync::mpsc::channel::<LiveAgentEvent>(LIVE_OUTPUT_CHANNEL_CAPACITY);
        let agent_id = attempt.agent_id.to_string();
        let plan_id = attempt.plan_id.to_string();
        let task_id = attempt.task_id.to_string();
        let mut feed = feed;
        let record = tool_calls.clone();
        let tap = tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                if let Some(progress) = &progress {
                    progress.observe(&event);
                }
                if let Some(tool_calls) = &tool_calls {
                    tool_calls.observe(&event);
                }
                if let Some(feed) = feed.as_mut() {
                    feed.push_live(&event);
                }
                if let Some((tui, _)) = &tui
                    && (forward_unscreened || matches!(event, LiveAgentEvent::ToolStep { .. }))
                {
                    forward_live_event_to_tui(tui, &agent_id, &plan_id, &task_id, event);
                }
            }
        });
        if let Some(record) = record {
            record.attach(tap);
        }
        Some(LiveOutput { sink, trusted })
    }

    /// Drive an attempt's provider `dispatch` to its end, publishing a TUI
    /// heartbeat every [`AGENT_HEARTBEAT_INTERVAL`] so the dashboard's
    /// elapsed-time counter stays live, and checking `watch` every
    /// [`STALL_CHECK_INTERVAL`]. An attempt that stalls, that the conductor
    /// restarts through `supervised`, whose plan run asks it to stop
    /// ([`WatchedAttempt::stop`]), or whose task the operator stops
    /// ([`OperatorStops`]) returns [`AttemptInterrupted`]; `dispatch`
    /// is dropped with it, which cancels the provider call, and `progress`
    /// then gives the call and what it streamed
    /// ([`AttemptProgress::interrupted_call`]), with or without a `watch`
    /// (bug-3a3b0f).
    pub(super) async fn run_watched<T>(
        &self,
        dispatch: impl Future<Output = T>,
        progress: &AttemptProgress,
        mut watch: Option<StallWatch>,
        supervised: Option<&SupervisedAttempt>,
        attempt: &WatchedAttempt<'_>,
    ) -> std::result::Result<T, AttemptInterrupted> {
        let started_at = Instant::now();
        let mut heartbeat = tokio::time::interval(AGENT_HEARTBEAT_INTERVAL);
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // Consume the immediate first ticks so neither fires at t=0.
        heartbeat.tick().await;
        let mut stall_check = tokio::time::interval(STALL_CHECK_INTERVAL);
        stall_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        stall_check.tick().await;
        let mut stop_check = tokio::time::interval(STOP_CHECK_INTERVAL);
        stop_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        stop_check.tick().await;
        let restarted = async {
            match supervised {
                Some(supervised) => supervised.restarted().await,
                None => std::future::pending().await,
            }
        };
        let operator_stop = self
            .operator_stops
            .register(attempt.plan_id, attempt.task_id);
        let stopped_by_operator = operator_stop.stopped();
        tokio::pin!(dispatch, restarted, stopped_by_operator);
        loop {
            tokio::select! {
                result = &mut dispatch => return Ok(result),
                restart = &mut restarted => {
                    progress.interrupted();
                    return Err(AttemptInterrupted::Restarted(restart));
                }
                () = &mut stopped_by_operator => {
                    progress.interrupted();
                    return Err(AttemptInterrupted::StoppedByOperator);
                }
                _ = heartbeat.tick() => {
                    if let Some(tui) = &self.tui_bridge {
                        tui.agent_heartbeat(
                            attempt.agent_id,
                            attempt.plan_id,
                            attempt.task_id,
                            started_at.elapsed().as_millis() as u64,
                        );
                    }
                }
                _ = stop_check.tick(), if attempt.stop.is_some() => {
                    if attempt.stop.is_some_and(|stop| stop.load(Ordering::Acquire)) {
                        progress.interrupted();
                        return Err(AttemptInterrupted::Stopped);
                    }
                }
                _ = stall_check.tick(), if watch.is_some() => {
                    if let Some(watch) = watch.as_mut()
                        && let Some(stalled) = self.check_stall(watch, attempt)
                    {
                        progress.interrupted();
                        return Err(stalled);
                    }
                }
            }
        }
    }

    /// Check `watch` now and publish what it found: a warning for a silent
    /// attempt; for a stalled one, the cancel, which the caller carries out.
    fn check_stall(
        &self,
        watch: &mut StallWatch,
        attempt: &WatchedAttempt<'_>,
    ) -> Option<AttemptInterrupted> {
        let (silent_for, cancelled) = match watch.check(Instant::now()) {
            StallCheck::Healthy => return None,
            StallCheck::Silent(silent_for) => (silent_for, false),
            StallCheck::Stalled(silent_for) => (silent_for, true),
        };
        let diagnosis = stall_diagnosis(attempt, watch.thresholds, silent_for, cancelled);
        tracing::warn!(
            plan_id = attempt.plan_id,
            task_id = attempt.task_id,
            attempt = attempt.attempt_key,
            silent_secs = silent_for.as_secs(),
            cancelled,
            "stall watchdog: {}",
            diagnosis.detail
        );
        if let Some(tui) = &self.tui_bridge {
            tui.diagnosis(diagnosis);
        }
        cancelled.then_some(AttemptInterrupted::Stalled(silent_for))
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use roko_core::DashboardEvent;
    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::{ModelProfile, ProviderConfig};
    use roko_graph::Cell;
    use tempfile::tempdir;

    use super::*;

    fn thresholds(warn_secs: u64, cancel_secs: u64) -> StallThresholds {
        StallThresholds::from_config(&ConductorConfig {
            silence_timeout_secs: warn_secs,
            task_stall_secs: cancel_secs,
            ..ConductorConfig::default()
        })
    }

    fn text(text: &str) -> LiveAgentEvent {
        LiveAgentEvent::Unscreened(StreamEventKind::TextDelta(text.to_string()))
    }

    fn tool_step(id: &str) -> LiveAgentEvent {
        LiveAgentEvent::ToolStep {
            id: id.to_string(),
            name: "Bash".to_string(),
            target: "cargo test".to_string(),
        }
    }

    fn tool_result(id: &str) -> LiveAgentEvent {
        LiveAgentEvent::Unscreened(StreamEventKind::ToolResult {
            id: id.to_string(),
            output: "ok".to_string(),
            is_error: false,
        })
    }

    fn secs(secs: u64) -> Duration {
        Duration::from_secs(secs)
    }

    #[test]
    fn stall_thresholds_follow_the_conductor_config() {
        let defaults = StallThresholds::from_config(&ConductorConfig::default());
        assert_eq!(defaults.warn_after, Some(secs(180)));
        assert_eq!(defaults.cancel_after, Some(secs(300)));
        assert!(defaults.is_enabled());

        assert!(!thresholds(0, 0).is_enabled(), "0 and 0 watch nothing");
        let warn_only = thresholds(60, 0);
        assert!(warn_only.is_enabled());
        assert_eq!(warn_only.cancel_after, None);
    }

    #[test]
    fn silence_counts_only_while_the_agent_waits_on_its_model() {
        let progress = AttemptProgress::default();
        let t0 = Instant::now();
        assert_eq!(
            progress.silence(t0 + secs(600)),
            None,
            "no progress reported yet: left to the hard timeout"
        );

        progress.observe_at(&text("reading the task"), t0);
        assert_eq!(progress.silence(t0 + secs(5)), Some(secs(5)));

        progress.observe_at(&tool_step("toolu_1"), t0 + secs(10));
        assert_eq!(
            progress.silence(t0 + secs(900)),
            None,
            "a running tool call is not silence"
        );

        progress.observe_at(&tool_result("toolu_1"), t0 + secs(920));
        assert_eq!(progress.silence(t0 + secs(923)), Some(secs(3)));
    }

    #[test]
    fn stall_watch_warns_once_then_cancels() {
        let mut watch = StallWatch::new(thresholds(10, 20));
        let t0 = Instant::now();
        watch.progress().observe_at(&text("working"), t0);

        assert_eq!(watch.check(t0 + secs(5)), StallCheck::Healthy);
        assert_eq!(watch.check(t0 + secs(10)), StallCheck::Silent(secs(10)));
        assert_eq!(
            watch.check(t0 + secs(15)),
            StallCheck::Healthy,
            "one warning per silence"
        );
        assert_eq!(watch.check(t0 + secs(20)), StallCheck::Stalled(secs(20)));
    }

    #[test]
    fn progress_ends_a_silence_and_rearms_its_warning() {
        let mut watch = StallWatch::new(thresholds(10, 0));
        let t0 = Instant::now();
        watch.progress().observe_at(&text("working"), t0);
        assert_eq!(watch.check(t0 + secs(11)), StallCheck::Silent(secs(11)));

        watch
            .progress()
            .observe_at(&text("still working"), t0 + secs(12));
        assert_eq!(watch.check(t0 + secs(13)), StallCheck::Healthy);
        assert_eq!(watch.check(t0 + secs(22)), StallCheck::Silent(secs(10)));
        assert_eq!(
            watch.check(t0 + secs(3_600)),
            StallCheck::Healthy,
            "task_stall_secs = 0 never cancels"
        );
    }

    #[test]
    fn a_cancel_threshold_below_the_warning_cancels_without_warning() {
        let mut watch = StallWatch::new(thresholds(30, 10));
        let t0 = Instant::now();
        watch.progress().observe_at(&text("working"), t0);
        assert_eq!(watch.check(t0 + secs(9)), StallCheck::Healthy);
        assert_eq!(watch.check(t0 + secs(10)), StallCheck::Stalled(secs(10)));
    }

    #[test]
    fn stall_diagnoses_name_the_task_and_the_intervention() {
        let attempt = WatchedAttempt {
            agent_id: "p1/T01",
            plan_id: "p1",
            task_id: "T01",
            attempt_key: "run-1/p1/T01/1",
            stop: None,
        };
        let silent = stall_diagnosis(&attempt, thresholds(180, 300), secs(181), false);
        assert_eq!(silent.severity, DiagnosisSeverity::Warn);
        assert_eq!(silent.subject, "p1/T01");
        assert_eq!(silent.intervention_taken, None);
        assert!(silent.detail.contains("181s"), "{}", silent.detail);
        assert!(
            silent
                .suggested_action
                .as_deref()
                .is_some_and(|action| action.contains("300s")),
            "{:?}",
            silent.suggested_action
        );

        let cancelled = stall_diagnosis(&attempt, thresholds(180, 300), secs(300), true);
        assert_eq!(cancelled.severity, DiagnosisSeverity::Alert);
        assert_eq!(
            cancelled.intervention_taken.as_deref(),
            Some(CANCELLED_STALLED_ATTEMPT)
        );
        assert_ne!(silent.id, cancelled.id);

        let error = AttemptInterrupted::Stalled(secs(300)).error(&attempt);
        assert!(matches!(error, RokoError::Timeout { .. }));
        assert!(error.to_string().contains("p1/T01 stalled"), "{error}");
    }

    /// A Claude CLI stand-in that reports progress once and then goes silent
    /// until `sleep` ends. Each launch appends a line to `launches`.
    fn silent_provider_script(dir: &Path, launches: &Path) -> PathBuf {
        provider_script(dir, launches, 1)
    }

    /// [`silent_provider_script`] sending the same message `messages` times
    /// before it goes silent, as an agent in a loop might.
    fn provider_script(dir: &Path, launches: &Path, messages: usize) -> PathBuf {
        let script = dir.join("fake-claude.sh");
        std::fs::write(
            &script,
            format!(
                r#"#!/bin/sh
set -eu
cat >/dev/null
echo launched >> '{}'
i=0
while [ "$i" -lt {messages} ]; do
  printf '%s\n' '{{"type":"assistant","message":{{"id":"msg-1","content":[{{"type":"text","text":"reading the task"}}]}}}}'
  i=$((i + 1))
done
exec sleep 60
"#,
                launches.display()
            ),
        )
        .expect("write provider script");
        let mut permissions = std::fs::metadata(&script)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).expect("make script executable");
        script
    }

    fn watched_config(script: &Path) -> RokoConfig {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "graph-model".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "graph-cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(script.display().to_string()),
                args: None,
                timeout_ms: Some(120_000),
                ttft_timeout_ms: Some(120_000),
                connect_timeout_ms: Some(5_000),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
                stream_usage: None,
            },
        );
        config.models.insert(
            "graph-model".to_string(),
            ModelProfile {
                provider: "graph-cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..ModelProfile::default()
            },
        );
        config.conductor.silence_timeout_secs = 1;
        config.conductor.task_stall_secs = 2;
        config
    }

    const STALLED_TASK_TITLE: &str = "Stall after the first message";

    /// A task with a 60 s timeout for [`silent_provider_script`].
    fn stalled_task_json(max_retries: u32) -> String {
        serde_json::json!({
            "id": "T01",
            "title": STALLED_TASK_TITLE,
            "role": "implementer",
            "model_hint": "graph-model",
            "timeout_secs": 60,
            "max_retries": max_retries,
        })
        .to_string()
    }

    /// The diagnoses published on the hub so far.
    fn drain_diagnoses(
        events: &mut tokio::sync::broadcast::Receiver<
            roko_runtime::event_bus::Envelope<DashboardEvent>,
        >,
    ) -> Vec<DiagnosisSummary> {
        let mut diagnoses = Vec::new();
        loop {
            match events.try_recv() {
                Ok(envelope) => {
                    if let DashboardEvent::Diagnosis { summary } = envelope.payload {
                        diagnoses.push(summary);
                    }
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {}
                Err(_) => return diagnoses,
            }
        }
    }

    /// A live cell for the task of [`stalled_task_json`].
    fn stalled_task_cell(
        dispatcher: Arc<GraphTaskDispatcher>,
        max_retries: u32,
    ) -> roko_graph::cells::TaskExecutorCell {
        let cell_config = toml::Value::Table(toml::map::Map::from_iter([
            ("plan_id".to_string(), toml::Value::String("p1".to_string())),
            (
                "title".to_string(),
                toml::Value::String(STALLED_TASK_TITLE.to_string()),
            ),
            ("timeout_secs".to_string(), toml::Value::Integer(60)),
            (
                "max_retries".to_string(),
                toml::Value::Integer(i64::from(max_retries)),
            ),
            (
                "task_def_json".to_string(),
                toml::Value::String(stalled_task_json(max_retries)),
            ),
        ]));
        roko_graph::cells::TaskExecutorCell::live(cell_config, dispatcher)
    }

    /// The run's conductor restarts an attempt whose agent repeats itself
    /// (the ghost-turn watcher of the default watchers), the task retries
    /// under its `max_retries`, and each restart reaches the dashboard.
    #[tokio::test]
    async fn conductor_restart_retries_a_looping_task() {
        let temp = tempdir().expect("tempdir");
        let launches = temp.path().join("launches.log");
        let script = provider_script(temp.path(), &launches, 4);
        let mut config = watched_config(&script);
        // Supervised, with the stall watchdog far behind the conductor.
        config.conductor.silence_timeout_secs = 30;
        config.conductor.task_stall_secs = 60;
        let config = Arc::new(config);
        let hub = crate::state_hub::shared_state_hub();
        let mut hub_events = hub.subscribe_events();
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf())
                .with_tui_bridge(TuiBridge::new(hub.sender()))
                .with_conductor(
                    Arc::new(roko_conductor::Conductor::from_config(&config.conductor)),
                    crate::runner::conductor_adapter::ConductorRing::new(),
                ),
        );
        let failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let _ticker = dispatcher
            .spawn_conductor_ticker(Duration::from_millis(200), {
                let failed = Arc::clone(&failed);
                move |_| failed.store(true, std::sync::atomic::Ordering::SeqCst)
            })
            .expect("the dispatcher is supervised");

        let started = Instant::now();
        let error = stalled_task_cell(dispatcher, 1)
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T01".to_string()),
            )
            .await
            .expect_err("a task that loops on every attempt fails");

        assert!(error.to_string().contains("ghost-turn"), "{error}");
        assert!(started.elapsed() < secs(30), "{:?}", started.elapsed());
        let launched = std::fs::read_to_string(&launches).expect("launch log");
        assert_eq!(
            launched.lines().count(),
            2,
            "one retry under max_retries = 1"
        );
        assert!(
            !failed.load(std::sync::atomic::Ordering::SeqCst),
            "a restart does not stop the run"
        );
        let restarts = drain_diagnoses(&mut hub_events)
            .iter()
            .filter(|d| {
                d.subject == "p1/T01"
                    && d.intervention_taken.as_deref() == Some("restarted attempt")
            })
            .count();
        assert_eq!(restarts, 2);
    }

    /// gap-c002bb: the operator's stop of a running task ends its attempt at
    /// once, and the task fails as stopped by the operator without a retry.
    #[tokio::test]
    async fn an_operator_stop_ends_the_running_attempt_without_a_retry() {
        let temp = tempdir().expect("tempdir");
        let launches = temp.path().join("launches.log");
        let script = silent_provider_script(temp.path(), &launches);
        let mut config = watched_config(&script);
        // Nothing but the operator ends the attempt.
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 0;
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            temp.path().to_path_buf(),
        ));
        let stops = dispatcher.operator_stops();
        // Stop the task once its agent has launched.
        let operator = tokio::spawn({
            let launches = launches.clone();
            async move {
                for _ in 0..400 {
                    let launched =
                        std::fs::read_to_string(&launches).is_ok_and(|log| !log.is_empty());
                    if launched && stops.stop("p1", "T01") {
                        return true;
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                false
            }
        });

        let started = Instant::now();
        let error = stalled_task_cell(dispatcher, 1)
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T01".to_string()),
            )
            .await
            .expect_err("a stopped task fails");

        assert!(
            operator.await.expect("operator task"),
            "the attempt never ran"
        );
        assert!(matches!(error, RokoError::Cancelled(_)), "{error}");
        assert!(
            error.to_string().contains("stopped by the operator"),
            "{error}"
        );
        assert!(started.elapsed() < secs(30), "{:?}", started.elapsed());
        let launched = std::fs::read_to_string(&launches).expect("launch log");
        assert_eq!(launched.lines().count(), 1, "no retry after the stop");
    }

    /// The streaming path cancels a stalled attempt the same way and ends it
    /// `TimedOut`.
    #[tokio::test]
    async fn streaming_dispatch_ends_a_stalled_attempt_timed_out() {
        let temp = tempdir().expect("tempdir");
        let script = silent_provider_script(temp.path(), &temp.path().join("launches.log"));
        let config = Arc::new(watched_config(&script));
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher =
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf());
        let spec = TaskExecutionSpec {
            plan_id: "p1".to_string(),
            title: STALLED_TASK_TITLE.to_string(),
            timeout_secs: 60,
            task_def_json: stalled_task_json(0),
            ..TaskExecutionSpec::default()
        };
        let lease = TaskLease {
            path: temp.path().to_path_buf(),
            fingerprint: "test-fingerprint".to_string(),
        };
        let (event_tx, mut event_rx) =
            tokio::sync::mpsc::channel(streaming_event_channel_capacity());

        let started = Instant::now();
        let error = dispatcher
            .dispatch_streaming(
                &spec,
                Vec::new(),
                &CellContext::new().with_cell_id("T01".to_string()),
                &lease,
                event_tx,
                &roko_graph::cells::NoopAttemptRecorder,
            )
            .await
            .expect_err("a stalled attempt fails");

        assert!(matches!(error, RokoError::Timeout { .. }), "{error}");
        assert!(started.elapsed() < secs(30), "{:?}", started.elapsed());
        let mut terminal = None;
        while let Ok(event) = event_rx.try_recv() {
            if let GraphTaskEvent::AttemptTerminal { outcome, .. } = event {
                terminal = Some(outcome);
            }
        }
        assert_eq!(terminal, Some(TaskDispatchOutcomeKind::TimedOut));
    }

    /// A task whose agent reports progress and then goes silent is cancelled
    /// after `task_stall_secs`, long before its `timeout_secs`, and retried
    /// under its `max_retries`; the dashboard (and so `--log-file`) gets a
    /// warning and a cancel diagnosis for each attempt.
    #[tokio::test]
    async fn graph_watchdog_intervenes_on_stalled_task() {
        let temp = tempdir().expect("tempdir");
        let launches = temp.path().join("launches.log");
        let script = silent_provider_script(temp.path(), &launches);
        let config = Arc::new(watched_config(&script));
        let hub = crate::state_hub::shared_state_hub();
        let mut hub_events = hub.subscribe_events();
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf())
                .with_tui_bridge(TuiBridge::new(hub.sender())),
        );
        let started = Instant::now();
        let error = stalled_task_cell(dispatcher, 1)
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T01".to_string()),
            )
            .await
            .expect_err("a task that stalls on every attempt fails");
        let elapsed = started.elapsed();

        assert!(matches!(error, RokoError::Timeout { .. }), "{error}");
        assert!(error.to_string().contains("stalled"), "{error}");
        assert!(
            elapsed < secs(30),
            "two attempts cancelled after {elapsed:?}: the watchdog, not the 60 s timeout, \
             ended them"
        );
        let launched = std::fs::read_to_string(&launches).expect("launch log");
        assert_eq!(
            launched.lines().count(),
            2,
            "the stalled attempt is retried once under max_retries = 1"
        );

        let diagnoses = drain_diagnoses(&mut hub_events);
        let warnings = diagnoses
            .iter()
            .filter(|d| d.severity == DiagnosisSeverity::Warn && d.subject == "p1/T01")
            .count();
        let cancels = diagnoses
            .iter()
            .filter(|d| {
                d.subject == "p1/T01"
                    && d.intervention_taken.as_deref() == Some(CANCELLED_STALLED_ATTEMPT)
            })
            .count();
        assert_eq!(warnings, 2, "{diagnoses:#?}");
        assert_eq!(cancels, 2, "{diagnoses:#?}");
    }

    /// Under the `ToolSteps` live output a watched attempt still asks the
    /// provider boundary for unscreened events, stamps its progress with
    /// them, and keeps them away from the TUI.
    #[tokio::test]
    async fn the_live_output_tap_feeds_the_watchdog_without_widening_the_tui_feed() {
        let temp = tempdir().expect("tempdir");
        let script = silent_provider_script(temp.path(), &temp.path().join("launches.log"));
        let config = Arc::new(watched_config(&script));
        let hub = crate::state_hub::shared_state_hub();
        let mut hub_events = hub.subscribe_events();
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher =
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf())
                .with_tui_bridge(TuiBridge::new(hub.sender()))
                .with_live_agent_output(LiveAgentOutput::ToolSteps);
        let attempt = WatchedAttempt {
            agent_id: "p1/T01",
            plan_id: "p1",
            task_id: "T01",
            attempt_key: "run-1/p1/T01/1",
            stop: None,
        };

        let unwatched = dispatcher
            .live_output_tap(&attempt, None, None, None)
            .expect("the TUI reads live output");
        assert!(!unwatched.trusted, "without the watchdog nothing changes");

        let watch = StallWatch::new(thresholds(1, 2));
        let live = dispatcher
            .live_output_tap(&attempt, Some(watch.progress()), None, None)
            .expect("the watchdog reads live output");
        assert!(live.trusted);
        live.sink
            .send(text("private reasoning"))
            .await
            .expect("send text");
        live.sink
            .send(tool_step("toolu_1"))
            .await
            .expect("send tool step");
        drop(live);

        // The tap handles events in order, so once the tool step reaches the
        // TUI the text before it has been handled too.
        let outputs = tokio::time::timeout(secs(5), async {
            let mut outputs = Vec::new();
            loop {
                match hub_events.recv().await {
                    Ok(envelope) => {
                        if let DashboardEvent::AgentOutput { content, .. } = envelope.payload {
                            let tool_step = content.contains("toolu_1");
                            outputs.push(content);
                            if tool_step {
                                return outputs;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(error) => panic!("state hub closed: {error}"),
                }
            }
        })
        .await
        .expect("the tool step reaches the TUI");
        assert_eq!(outputs.len(), 1, "only the tool step: {outputs:?}");

        let progress = watch.progress.inner.lock();
        assert!(progress.last_event.is_some(), "the tap stamps progress");
        assert!(
            progress.open_tool_calls.contains("toolu_1"),
            "the tool call is still running"
        );
    }

    fn usage(input_tokens: u32, output_tokens: u32) -> LiveAgentEvent {
        LiveAgentEvent::Unscreened(StreamEventKind::Usage(roko_core::Usage {
            input_tokens,
            output_tokens,
            ..roko_core::Usage::default()
        }))
    }

    /// Within a model call each `Usage` is its usage so far and `Done` ends
    /// the call, so a cancelled call's usage is the sum of each model call's
    /// last `Usage`.
    #[test]
    fn streamed_usage_sums_the_last_usage_of_each_model_call() {
        let progress = AttemptProgress::default();
        let done = LiveAgentEvent::Unscreened(StreamEventKind::Done {
            finish_reason: "tool_use".to_string(),
        });
        progress.observe(&usage(100, 0));
        progress.observe(&usage(100, 40));
        progress.observe(&done);
        progress.observe(&usage(300, 0));
        progress.observe(&text("still thinking"));

        let total = progress.inner.lock().usage.total().expect("usage streamed");
        assert_eq!((total.input_tokens, total.output_tokens), (400, 40));
        assert!(
            progress.interrupted_call().is_none(),
            "nothing was cancelled"
        );
    }

    /// A stalled attempt keeps what its call streamed (bug-aa2044): the
    /// usage of the message the agent sent before it went silent is priced,
    /// counts against the task's spend, and its verdict and cost row mark it
    /// estimated.
    #[tokio::test]
    async fn a_stalled_attempt_records_the_usage_it_streamed() {
        let temp = tempdir().expect("tempdir");
        let script = temp.path().join("fake-claude.sh");
        std::fs::write(
            &script,
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"assistant","message":{"id":"msg-1","model":"claude-sonnet-4-6","content":[{"type":"text","text":"reading the task"}],"usage":{"input_tokens":1000,"output_tokens":200,"cache_creation_input_tokens":3000,"cache_read_input_tokens":4000}}}'
exec sleep 60
"#,
        )
        .expect("write provider script");
        let mut permissions = std::fs::metadata(&script)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).expect("make script executable");
        let config = Arc::new(watched_config(&script));
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher =
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf())
                .with_feedback(crate::graph_task_dispatch::tests::recording_feedback(
                    temp.path(),
                ));
        let spec = TaskExecutionSpec {
            plan_id: "p1".to_string(),
            title: STALLED_TASK_TITLE.to_string(),
            timeout_secs: 60,
            task_def_json: stalled_task_json(0),
            ..TaskExecutionSpec::default()
        };
        let run = "stall-run";

        let error = dispatcher
            .dispatch(
                &spec,
                Vec::new(),
                &CellContext::new()
                    .with_cell_id("T01".to_string())
                    .with_run_id(run.to_string()),
            )
            .await
            .expect_err("the attempt stalls");

        assert!(matches!(error, RokoError::Timeout { .. }), "{error}");
        // Sonnet per million: $3 in, $15 out, $0.30 cache read, $3.75 cache write.
        let expected = (1_000.0 * 3.0 + 200.0 * 15.0 + 4_000.0 * 0.30 + 3_000.0 * 3.75) / 1e6;
        assert!(
            dispatcher.task_spend.admit("p1/T01", 0.01).is_err(),
            "the streamed ${expected} counts against the task"
        );
        let roko = temp.path().join(".roko");
        let verdicts = crate::graph_task_dispatch::tests::jsonl_rows_where(
            &roko.join("runs").join(run).join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let verdict = &verdicts[0];
        assert_eq!(verdict["cost"]["source"], "estimated", "{verdict}");
        assert_eq!(verdict["executed"]["provider"], "graph-cli");
        assert_eq!(verdict["executed"]["model_dispatched"], "claude-sonnet-4-6");
        let costs = crate::graph_task_dispatch::tests::jsonl_rows_where(
            &roko.join("learn/costs.jsonl"),
            1,
            |_| true,
        )
        .await;
        let cost = &costs[0];
        assert_eq!(cost["attempt_key"], verdict["attempt_key"]);
        assert_eq!(cost["cost_source"], "estimated");
        assert_eq!(cost["input_tokens"], 1_000);
        assert_eq!(cost["output_tokens"], 200);
        assert_eq!(cost["cached_tokens"], 4_000);
        let cost_usd = cost["cost_usd"].as_f64().expect("cost");
        assert!((cost_usd - expected).abs() < 1e-6, "{cost}");
    }

    /// bug-3a3b0f: with both stall thresholds at 0 nothing is watched, but
    /// the attempt's progress still is: a call its plan run stops settles the
    /// usage it streamed, not an unknown one.
    #[tokio::test]
    async fn usage_is_tracked_with_the_watchdog_off() {
        let temp = tempdir().expect("tempdir");
        let script = temp.path().join("fake-claude.sh");
        write_provider(
            &script,
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"assistant","message":{"id":"msg-1","model":"claude-sonnet-4-6","content":[{"type":"text","text":"reading the task"}],"usage":{"input_tokens":1000,"output_tokens":200}}}'
echo streamed > "$(dirname -- "$0")/streamed"
exec sleep 60
"#,
        );
        let mut config = watched_config(&script);
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 0;
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher =
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf())
                .with_feedback(crate::graph_task_dispatch::tests::recording_feedback(
                    temp.path(),
                ));
        assert!(dispatcher.stall_watch().is_none(), "nothing is watched");
        let spec = TaskExecutionSpec {
            plan_id: "p1".to_string(),
            title: STALLED_TASK_TITLE.to_string(),
            timeout_secs: 60,
            task_def_json: stalled_task_json(0),
            ..TaskExecutionSpec::default()
        };
        let run = "stopped-run";
        let stop = Arc::new(AtomicBool::new(false));
        let ctx = CellContext::new()
            .with_cell_id("T01".to_string())
            .with_run_id(run.to_string())
            .with_cancel_flag(Arc::clone(&stop));
        let streamed = temp.path().join("streamed");
        let stop_once_streamed = async {
            for _ in 0..400 {
                if streamed.exists() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            // Time for the live-output tap to take in the message's usage.
            for _ in 0..5 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            stop.store(true, Ordering::Release);
        };

        let (result, ()) = tokio::join!(
            dispatcher.dispatch(&spec, Vec::new(), &ctx),
            stop_once_streamed
        );
        let error = result.expect_err("the plan run stopped the attempt");
        assert!(matches!(error, RokoError::Cancelled(_)), "{error}");
        let roko = temp.path().join(".roko");
        let verdicts = crate::graph_task_dispatch::tests::jsonl_rows_where(
            &roko.join("runs").join(run).join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let verdict = &verdicts[0];
        assert_eq!(verdict["outcome"], "cancelled", "{verdict}");
        assert_eq!(verdict["cost"]["source"], "estimated", "{verdict}");
        let costs = crate::graph_task_dispatch::tests::jsonl_rows_where(
            &roko.join("learn/costs.jsonl"),
            1,
            |_| true,
        )
        .await;
        assert_eq!(costs[0]["input_tokens"], 1_000, "{}", costs[0]);
        assert_eq!(costs[0]["output_tokens"], 200, "{}", costs[0]);
    }

    /// Write `body` as an executable provider script at `path`.
    fn write_provider(path: &Path, body: &str) {
        std::fs::write(path, body).expect("write provider script");
        let mut permissions = std::fs::metadata(path)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("make script executable");
    }

    /// Dispatch the stalled task of [`stalled_task_json`] once on `config`,
    /// recording to `workdir/.roko`, in run `run`: the attempt's error and
    /// its verdict line.
    async fn dispatch_once(
        workdir: &Path,
        config: RokoConfig,
        run: &str,
    ) -> (RokoError, serde_json::Value) {
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = GraphTaskDispatcher::new(factory, config, workdir.to_path_buf())
            .with_feedback(crate::graph_task_dispatch::tests::recording_feedback(
                workdir,
            ));
        let spec = TaskExecutionSpec {
            plan_id: "p1".to_string(),
            title: STALLED_TASK_TITLE.to_string(),
            timeout_secs: 60,
            task_def_json: stalled_task_json(0),
            ..TaskExecutionSpec::default()
        };
        let error = dispatcher
            .dispatch(
                &spec,
                Vec::new(),
                &CellContext::new()
                    .with_cell_id("T01".to_string())
                    .with_run_id(run.to_string()),
            )
            .await
            .expect_err("the attempt stalls");
        let verdicts = crate::graph_task_dispatch::tests::jsonl_rows_where(
            &workdir.join(".roko/runs").join(run).join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        (error, verdicts[0].clone())
    }

    /// bug-4c553b: an attempt the watchdog cancelled settles as a timeout,
    /// not a provider error. Once it reported progress the timeout is the
    /// agent's; a Claude CLI call that never reported anything is silent
    /// from its start (after its first-output grace), and its timeout is the
    /// provider's.
    #[tokio::test]
    async fn a_stalled_attempt_settles_as_a_timeout() {
        let temp = tempdir().expect("tempdir");
        let script = silent_provider_script(temp.path(), &temp.path().join("launches.log"));
        let (error, verdict) = dispatch_once(temp.path(), watched_config(&script), "talked").await;
        assert!(matches!(error, RokoError::Timeout { .. }), "{error}");
        assert_eq!(verdict["outcome"], "timeout", "{verdict}");
        assert_eq!(verdict["failure_class"]["kind"], "timeout", "{verdict}");
        assert_eq!(verdict["blame"], "agent", "{verdict}");
        assert_eq!(verdict["learning_label"], 0, "{verdict}");

        let mute = temp.path().join("mute-claude.sh");
        write_provider(&mute, "#!/bin/sh\nset -eu\ncat >/dev/null\nexec sleep 60\n");
        let (error, verdict) = dispatch_once(temp.path(), watched_config(&mute), "mute").await;
        assert!(error.to_string().contains("stalled"), "{error}");
        assert_eq!(verdict["outcome"], "timeout", "{verdict}");
        assert_eq!(verdict["blame"], "infra", "{verdict}");
        assert!(verdict["learning_label"].is_null(), "{verdict}");
    }

    /// bug-2aa55f: deltas are progress. A stream of deltas holds the
    /// watchdog off, and once it goes silent the attempt is cancelled.
    #[tokio::test]
    async fn a_delta_only_stream_still_trips_the_watchdog() {
        let temp = tempdir().expect("tempdir");
        let deltas = temp.path().join("deltas.log");
        let script = temp.path().join("delta-claude.sh");
        write_provider(
            &script,
            &format!(
                r#"#!/bin/sh
set -eu
cat >/dev/null
for i in 1 2 3 4 5; do
  printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"thinking "}}}}'
  echo "$i" >> '{deltas}'
  sleep 1
done
exec sleep 60
"#,
                deltas = deltas.display()
            ),
        );
        let mut config = watched_config(&script);
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 5;
        let started = Instant::now();
        let (error, verdict) = dispatch_once(temp.path(), config, "deltas").await;
        let elapsed = started.elapsed();

        assert!(error.to_string().contains("stalled"), "{error}");
        assert_eq!(verdict["outcome"], "timeout", "{verdict}");
        let streamed = std::fs::read_to_string(&deltas).expect("delta log");
        assert_eq!(
            streamed.lines().count(),
            5,
            "the deltas held the watchdog off until they stopped"
        );
        assert!(
            elapsed < secs(55),
            "the watchdog, not the 60 s timeout, ended the attempt after {elapsed:?}"
        );
    }

    /// bug-2aa55f: a call whose provider streams as it goes is silent from
    /// its start, once its first-output grace has passed; one whose provider
    /// may report only at the end is not until it reports something.
    #[test]
    fn a_streaming_call_is_silent_from_its_start() {
        let config = Arc::new(watched_config(Path::new("/bin/true")));
        let claude =
            crate::dispatch_v2::ProviderDispatchResolver::new(config).resolve("graph-model");
        assert_eq!(claude.provider_kind, ProviderKind::ClaudeCli);
        let mut codex = claude.clone();
        codex.provider_kind = ProviderKind::CodexCli;
        let t0 = Instant::now();

        let streaming = AttemptProgress::default();
        streaming.call_started_at(claude, FailoverChain::default(), t0);
        assert_eq!(
            streaming.silence(t0 + secs(5)),
            None,
            "a call that is only starting has its grace"
        );
        assert_eq!(
            streaming.silence(t0 + FIRST_OUTPUT_GRACE),
            Some(FIRST_OUTPUT_GRACE)
        );
        assert!(!streaming.reported_progress());
        streaming.observe_at(&text("working"), t0 + secs(4));
        assert_eq!(streaming.silence(t0 + secs(5)), Some(secs(1)));
        assert!(streaming.reported_progress());

        let at_the_end = AttemptProgress::default();
        at_the_end.call_started_at(codex, FailoverChain::default(), t0);
        assert_eq!(
            at_the_end.silence(t0 + secs(600)),
            None,
            "a provider that may report only at the end is left to its timeout"
        );
    }

    /// bug-eba31d: a call waiting for its provider's permit is queued, not
    /// silent. Once it has the permit, its silence and its first-output grace
    /// count from then, and the wait is no progress the agent reported.
    #[test]
    fn a_queued_call_is_not_silent_until_it_starts() {
        let config = Arc::new(watched_config(Path::new("/bin/true")));
        let claude =
            crate::dispatch_v2::ProviderDispatchResolver::new(config).resolve("graph-model");
        let t0 = Instant::now();
        let progress = AttemptProgress::default();
        progress.call_started_at(claude, FailoverChain::default(), t0);

        progress.observe_at(&LiveAgentEvent::Queued { waiting: true }, t0 + secs(1));
        assert_eq!(
            progress.silence(t0 + secs(600)),
            None,
            "a queued call is not silent"
        );
        progress.observe_at(&LiveAgentEvent::Queued { waiting: false }, t0 + secs(600));
        assert_eq!(
            progress.silence(t0 + secs(601)),
            None,
            "its first-output grace starts with the permit"
        );
        assert_eq!(
            progress.silence(t0 + secs(600) + FIRST_OUTPUT_GRACE),
            Some(FIRST_OUTPUT_GRACE)
        );
        assert!(!progress.reported_progress(), "waiting is no progress");
    }

    /// bug-eba31d: with `max_concurrent = 1`, an attempt queued behind a long
    /// first attempt waits for the permit without being cancelled as stalled,
    /// although the wait outlasts its first-output grace and `task_stall_secs`.
    #[tokio::test]
    async fn a_queued_attempt_is_not_cancelled_while_it_waits() {
        let temp = tempdir().expect("tempdir");
        let script = temp.path().join("one-slot-claude.sh");
        // The call that claims `first` works for 8 s, reporting every second;
        // the other one, queued meanwhile, answers at once.
        write_provider(
            &script,
            r#"#!/bin/sh
set -eu
cat >/dev/null
if mkdir "$(dirname "$0")/first" 2>/dev/null; then
  i=0
  while [ "$i" -lt 8 ]; do
    printf '%s\n' '{"type":"assistant","message":{"id":"msg-1","content":[{"type":"text","text":"working"}]}}'
    i=$((i + 1))
    sleep 1
  done
fi
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"done"}}'
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0}'
"#,
        );
        let mut config = watched_config(&script);
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 3;
        config
            .providers
            .get_mut("graph-cli")
            .expect("the fixture provider")
            .max_concurrent = Some(1);
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf())
                .with_first_output_grace(secs(5)),
        );
        let attempt = |id: &'static str| {
            let dispatcher = Arc::clone(&dispatcher);
            async move {
                let spec = TaskExecutionSpec {
                    plan_id: "p1".to_string(),
                    title: format!("Task {id}"),
                    timeout_secs: 60,
                    task_def_json: serde_json::json!({
                        "id": id,
                        "title": format!("Task {id}"),
                        "role": "implementer",
                        "model_hint": "graph-model",
                        "timeout_secs": 60,
                    })
                    .to_string(),
                    ..TaskExecutionSpec::default()
                };
                dispatcher
                    .dispatch(
                        &spec,
                        Vec::new(),
                        &CellContext::new().with_cell_id(id.to_string()),
                    )
                    .await
            }
        };

        let (first, second) = tokio::join!(attempt("T01"), attempt("T02"));
        first.expect("the attempt holding the permit finishes");
        second.expect("the queued attempt waits for the permit, then finishes");
    }

    /// A run the stall watchdog drops leaves none of its tasks behind
    /// (bug-2a2f63). The watchdog's heartbeat is an interval inside
    /// `run_watched` and ends with it; the agent run's own heartbeat task is
    /// aborted with the run, and its output readers end once its process is
    /// killed.
    #[tokio::test]
    async fn a_dropped_run_stops_its_heartbeat() {
        let temp = tempdir().expect("tempdir");
        let launches = temp.path().join("launches.log");
        let script = silent_provider_script(temp.path(), &launches);
        let config = Arc::new(watched_config(&script));
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            temp.path().to_path_buf(),
        ));
        let spec = TaskExecutionSpec {
            plan_id: "p1".to_string(),
            title: STALLED_TASK_TITLE.to_string(),
            timeout_secs: 60,
            task_def_json: stalled_task_json(0),
            ..TaskExecutionSpec::default()
        };
        let metrics = tokio::runtime::Handle::current().metrics();
        let before = metrics.num_alive_tasks();

        let dispatch = tokio::spawn(async move {
            dispatcher
                .dispatch(
                    &spec,
                    Vec::new(),
                    &CellContext::new().with_cell_id("T01".to_string()),
                )
                .await
        });
        for _ in 0..400 {
            if launches.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
        // The dispatch and, on this runtime, the run's readers and heartbeat.
        assert!(
            metrics.num_alive_tasks() >= before + 3,
            "the run's tasks are counted"
        );
        let error = dispatch
            .await
            .expect("the dispatch task")
            .expect_err("the stalled attempt is dropped");
        assert!(matches!(error, RokoError::Timeout { .. }), "{error}");

        let mut alive = metrics.num_alive_tasks();
        for _ in 0..200 {
            if alive <= before {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            alive = metrics.num_alive_tasks();
        }
        assert!(
            alive <= before,
            "{} task(s) of the dropped run outlived it",
            alive - before
        );
    }
}
