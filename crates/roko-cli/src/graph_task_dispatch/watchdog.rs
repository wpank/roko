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
//!   attempt fails as stalled ([`AttemptStalled::error`]), so the Graph engine
//!   retries it under the task's `max_retries`.
//!
//! `0` turns a threshold off, and with both off nothing is watched. The hard
//! `timeout_secs` stays the outer bound either way.
//!
//! Silence counts only while the agent waits on its model:
//!
//! - some providers report nothing until they finish (the Codex CLI hands over
//!   its whole answer at the end), so an attempt that has not reported
//!   anything yet is left to its hard timeout;
//! - a tool call the agent made (a long `cargo test`, say) is bounded by the
//!   provider's own tool timeout, so silence while one runs does not count.

use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use roko_agent::StreamEventKind;
use roko_agent::live_output::{LiveAgentEvent, LiveOutput};
use roko_core::config::schema::ConductorConfig;
use roko_core::{DiagnosisSeverity, DiagnosisSummary};

use super::*;

/// How often a running attempt's [`StallWatch`] is checked. The thresholds
/// are whole seconds, so this is precise enough.
const STALL_CHECK_INTERVAL: Duration = Duration::from_secs(1);

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
    /// Tool calls the agent made whose results have not arrived yet.
    open_tool_calls: HashSet<String>,
}

impl AttemptProgress {
    /// Record `event`, received now.
    pub(super) fn observe(&self, event: &LiveAgentEvent) {
        self.observe_at(event, Instant::now());
    }

    fn observe_at(&self, event: &LiveAgentEvent, at: Instant) {
        let mut state = self.inner.lock();
        state.last_event = Some(at);
        match event {
            LiveAgentEvent::ToolStep { id, .. }
            | LiveAgentEvent::Unscreened(StreamEventKind::ToolCallEnd { id, .. }) => {
                state.open_tool_calls.insert(id.clone());
            }
            LiveAgentEvent::Unscreened(StreamEventKind::ToolResult { id, .. }) => {
                state.open_tool_calls.remove(id);
            }
            LiveAgentEvent::Unscreened(_) => {}
        }
    }

    /// How long, at `now`, the attempt has been waiting on its model without
    /// reporting progress: `None` before it first reported anything and while
    /// a tool call it made is still running.
    fn silence(&self, now: Instant) -> Option<Duration> {
        let state = self.inner.lock();
        let last_event = state.last_event?;
        state
            .open_tool_calls
            .is_empty()
            .then(|| now.saturating_duration_since(last_event))
    }
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
}

/// An attempt [`GraphTaskDispatcher::run_watched`] cancelled because it
/// stalled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AttemptStalled {
    /// How long it had been silent.
    pub(super) silent_for: Duration,
}

impl AttemptStalled {
    /// The error the cancelled attempt fails with: a timeout, which the Graph
    /// engine retries under the task's `max_retries`.
    pub(super) fn error(self, attempt: &WatchedAttempt<'_>) -> RokoError {
        RokoError::Timeout {
            operation: format!(
                "agent for {}/{} stalled: no progress for {}s, so the stall watchdog cancelled \
                 the attempt ([conductor] task_stall_secs)",
                attempt.plan_id,
                attempt.task_id,
                self.silent_for.as_secs()
            ),
            timeout_ms: u64::try_from(self.silent_for.as_millis()).unwrap_or(u64::MAX),
        }
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
        thresholds.is_enabled().then(|| StallWatch::new(thresholds))
    }

    /// The live-output channel for an attempt's dispatch request, when
    /// anything reads it: the TUI, when both [`Self::with_tui_bridge`] and
    /// [`Self::with_live_agent_output`] are set, or the attempt's stall watch,
    /// through `progress`. Spawns the tap that reads it, which ends when the
    /// dispatch drops its end.
    ///
    /// The watchdog asks the provider boundary for unscreened text, reasoning
    /// and tool results as well, to tell a model that is thinking from a tool
    /// that is running. They stay in the tap: the TUI still gets them only
    /// under [`LiveAgentOutput::Trusted`].
    pub(super) fn live_output_tap(
        &self,
        attempt: &WatchedAttempt<'_>,
        progress: Option<AttemptProgress>,
    ) -> Option<LiveOutput> {
        let tui = self.tui_bridge.clone().zip(self.live_agent_output);
        if tui.is_none() && progress.is_none() {
            return None;
        }
        let forward_unscreened = matches!(tui, Some((_, LiveAgentOutput::Trusted)));
        let trusted = forward_unscreened || progress.is_some();
        let (sink, mut events) =
            tokio::sync::mpsc::channel::<LiveAgentEvent>(LIVE_OUTPUT_CHANNEL_CAPACITY);
        let agent_id = attempt.agent_id.to_string();
        let plan_id = attempt.plan_id.to_string();
        let task_id = attempt.task_id.to_string();
        tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                if let Some(progress) = &progress {
                    progress.observe(&event);
                }
                if let Some((tui, _)) = &tui
                    && (forward_unscreened || matches!(event, LiveAgentEvent::ToolStep { .. }))
                {
                    forward_live_event_to_tui(tui, &agent_id, &plan_id, &task_id, event);
                }
            }
        });
        Some(LiveOutput { sink, trusted })
    }

    /// Drive an attempt's provider `dispatch` to its end, publishing a TUI
    /// heartbeat every [`AGENT_HEARTBEAT_INTERVAL`] so the dashboard's
    /// elapsed-time counter stays live, and checking `watch` every
    /// [`STALL_CHECK_INTERVAL`]. A stalled attempt returns
    /// [`AttemptStalled`]; `dispatch` is dropped with it, which cancels the
    /// provider call.
    pub(super) async fn run_watched<T>(
        &self,
        dispatch: impl Future<Output = T>,
        mut watch: Option<StallWatch>,
        attempt: &WatchedAttempt<'_>,
    ) -> std::result::Result<T, AttemptStalled> {
        let started_at = Instant::now();
        let mut heartbeat = tokio::time::interval(AGENT_HEARTBEAT_INTERVAL);
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // Consume the immediate first ticks so neither fires at t=0.
        heartbeat.tick().await;
        let mut stall_check = tokio::time::interval(STALL_CHECK_INTERVAL);
        stall_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        stall_check.tick().await;
        tokio::pin!(dispatch);
        loop {
            tokio::select! {
                result = &mut dispatch => return Ok(result),
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
                _ = stall_check.tick(), if watch.is_some() => {
                    if let Some(watch) = watch.as_mut()
                        && let Some(stalled) = self.check_stall(watch, attempt)
                    {
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
    ) -> Option<AttemptStalled> {
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
        cancelled.then_some(AttemptStalled { silent_for })
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

        watch.progress().observe_at(&text("still working"), t0 + secs(12));
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

        let error = AttemptStalled {
            silent_for: secs(300),
        }
        .error(&attempt);
        assert!(matches!(error, RokoError::Timeout { .. }));
        assert!(error.to_string().contains("p1/T01 stalled"), "{error}");
    }

    /// A Claude CLI stand-in that reports progress once and then goes silent
    /// until `sleep` ends. Each launch appends a line to `launches`.
    fn silent_provider_script(dir: &Path, launches: &Path) -> PathBuf {
        let script = dir.join("fake-claude.sh");
        std::fs::write(
            &script,
            format!(
                r#"#!/bin/sh
set -eu
cat >/dev/null
echo launched >> '{}'
printf '%s\n' '{{"type":"assistant","message":{{"id":"msg-1","content":[{{"type":"text","text":"reading the task"}}]}}}}'
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
        let task_def_json = serde_json::json!({
            "id": "T01",
            "title": "Stall after the first message",
            "role": "implementer",
            "model_hint": "graph-model",
            "timeout_secs": 60,
            "max_retries": 1,
        })
        .to_string();
        let cell_config = toml::Value::Table(toml::map::Map::from_iter([
            ("plan_id".to_string(), toml::Value::String("p1".to_string())),
            (
                "title".to_string(),
                toml::Value::String("Stall after the first message".to_string()),
            ),
            ("timeout_secs".to_string(), toml::Value::Integer(60)),
            ("max_retries".to_string(), toml::Value::Integer(1)),
            ("task_def_json".to_string(), toml::Value::String(task_def_json)),
        ]));
        let cell = roko_graph::cells::TaskExecutorCell::live(cell_config, dispatcher);

        let started = Instant::now();
        let error = cell
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

        let mut diagnoses = Vec::new();
        loop {
            match hub_events.try_recv() {
                Ok(envelope) => {
                    if let DashboardEvent::Diagnosis { summary } = envelope.payload {
                        diagnoses.push(summary);
                    }
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {}
                Err(_) => break,
            }
        }
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
        };

        let unwatched = dispatcher
            .live_output_tap(&attempt, None)
            .expect("the TUI reads live output");
        assert!(!unwatched.trusted, "without the watchdog nothing changes");

        let watch = StallWatch::new(thresholds(1, 2));
        let live = dispatcher
            .live_output_tap(&attempt, Some(watch.progress()))
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
}
