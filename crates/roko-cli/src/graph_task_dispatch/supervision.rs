//! Conductor supervision of running Graph task attempts (spec-a0403b,
//! part B).
//!
//! With [`GraphTaskDispatcher::with_conductor`], each attempt feeds the run's
//! [`ConductorRing`] while its provider runs: its start, and the messages and
//! tool calls of its live output, each tagged with its attempt key
//! ([`ATTEMPT_TAG`]). The plan host runs [`GraphConductor::tick`] every
//! [`SUPERVISION_INTERVAL`] ([`GraphTaskDispatcher::spawn_conductor_ticker`]),
//! which evaluates the conductor over each running attempt's own signals and
//! acts on the decision:
//!
//! - `Restart` cancels that attempt the way the stall watchdog does: it fails
//!   and retries under its task's `max_retries`. When the task's next attempt
//!   ends, [`Conductor::record_intervention_outcome`] learns whether the
//!   restart helped;
//! - `Fail` stops the run: `tick` returns a [`ConductorStop`] naming the
//!   watcher and the reason, which the host makes the run's error;
//! - `Nudge` and `ForceAdvance` are not carried out yet (gap-ebd656).
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
use roko_core::{ConductorDecision, DiagnosisSeverity, DiagnosisSummary};
use tokio_util::sync::CancellationToken;

use crate::runner::conductor_adapter::{
    ATTEMPT_TAG, ConductorRing, graph_task_event_to_signal, graph_text_turn_signal,
};

use super::watchdog::StallThresholds;
use super::*;

/// How often the plan host runs [`GraphConductor::tick`].
pub const SUPERVISION_INTERVAL: Duration = Duration::from_secs(5);

/// A Graph run's conductor, the ring its attempts feed, and the attempts
/// whose providers are running.
#[derive(Clone)]
pub struct GraphConductor {
    conductor: Arc<Conductor>,
    ring: ConductorRing,
    running: Arc<parking_lot::Mutex<HashMap<String, RunningAttempt>>>,
    /// Restarts whose outcome is not known yet, by task (`plan/task`).
    restarts: Arc<parking_lot::Mutex<HashMap<String, PendingRestart>>>,
}

impl std::fmt::Debug for GraphConductor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphConductor")
            .field("conductor", &self.conductor)
            .field("ring_len", &self.ring.len())
            .field("running", &self.running.lock().len())
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
    /// its decisions, publishing each intervention through `tui`. Returns the
    /// `Fail` that must stop the run, if any.
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
                ConductorDecision::Continue => {}
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
    use roko_core::{Body, Context, React};

    use super::*;
    use crate::graph_task_dispatch::tests::make_bare_dispatcher;

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
