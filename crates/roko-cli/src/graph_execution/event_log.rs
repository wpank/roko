//! `--log-file` JSONL recorder for Graph plan runs.
//!
//! A Graph run publishes its plan-set, lifecycle, agent, and gate events to
//! its StateHub. [`run_recorded`] subscribes to that hub before the run
//! starts and writes one JSON line per event until the run returns:
//!
//! ```text
//! {"type":"run.started","run_id":R,"timestamp_ms":..,"plan_ids":[..],"total_tasks":N,..}
//! {"type":"dashboard.task_started","run_id":R,"seq":N,"ts_millis":..,"event":{"type":"task_started",..}}
//! {"type":"log.lagged","run_id":R,"skipped":N}
//! {"type":"run.completed","run_id":R,"timestamp_ms":..,"outcome":"failed","exit_code":1,..}
//! ```
//!
//! Exactly one `run.started` and one `run.completed` line bracket the events,
//! and every line carries the same `run_id` (`ROKO_EVIDENCE_RUN_ID` when set).
//! That is the contract `scripts/run_evidence.py --require-events` checks for
//! `./dev.sh fast`. A `log.lagged` line counts events the recorder missed
//! because it fell behind the hub's broadcast buffer.

use std::collections::BTreeMap;
use std::fs::File;
use std::future::Future;
use std::io::{BufWriter, Write as _};
use std::path::Path;
use std::pin::Pin;

use anyhow::{Context as _, anyhow};
use chrono::{DateTime, Utc};
use roko_core::DashboardEvent;
use roko_core::dashboard_snapshot::PlanSetEntry;
use roko_runtime::event_bus::Envelope;
use serde::Serialize;
use tokio::sync::broadcast::error::{RecvError, TryRecvError};
use tokio::sync::oneshot;

use super::plan_runner::{GraphPlanRunParams, PlanRunInterrupt, run_graph_plan};
use crate::exit_codes::EXIT_SUCCESS;
use crate::runner::types::{RunOutcome, RunnerEvent};
use crate::state_hub::StateHub;

// ── Event tap ────────────────────────────────────────────────────────────

/// What an [`EventTap`] receives from its hub.
pub enum Tapped<'a> {
    /// One published event.
    Event(&'a Envelope<DashboardEvent>),
    /// The tap fell behind the hub's broadcast buffer and missed this many
    /// events.
    Lagged(u64),
}

/// Folds every event a hub publishes, from the moment the tap is spawned
/// until [`EventTap::finish`], into a state value.
///
/// The tap reads on a task of its own, so a long run cannot overflow the
/// hub's broadcast buffer between reads. Dropping an unfinished tap stops it.
pub struct EventTap<S> {
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<S>>,
}

impl<S: Send + 'static> EventTap<S> {
    /// Subscribe to `hub` now and fold its events into `state`.
    pub fn spawn<F>(hub: &StateHub, state: S, mut on_event: F) -> Self
    where
        F: FnMut(&mut S, Tapped<'_>) + Send + 'static,
    {
        let mut events = hub.subscribe_events();
        let (stop, mut stopped) = oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let mut state = state;
            loop {
                tokio::select! {
                    biased;
                    received = events.recv() => match received {
                        Ok(envelope) => on_event(&mut state, Tapped::Event(&envelope)),
                        Err(RecvError::Lagged(skipped)) => {
                            on_event(&mut state, Tapped::Lagged(skipped));
                        }
                        Err(RecvError::Closed) => return state,
                    },
                    _ = &mut stopped => break,
                }
            }
            // Take whatever was published before the stop request.
            loop {
                match events.try_recv() {
                    Ok(envelope) => on_event(&mut state, Tapped::Event(&envelope)),
                    Err(TryRecvError::Lagged(skipped)) => {
                        on_event(&mut state, Tapped::Lagged(skipped));
                    }
                    Err(TryRecvError::Empty | TryRecvError::Closed) => return state,
                }
            }
        });
        Self {
            stop: Some(stop),
            task: Some(task),
        }
    }

    /// Consume every event published so far, stop, and return the state.
    pub async fn finish(mut self) -> anyhow::Result<S> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        let task = self
            .task
            .take()
            .ok_or_else(|| anyhow!("event tap already finished"))?;
        task.await.context("event tap task failed")
    }
}

impl<S> Drop for EventTap<S> {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

// ── JSONL run log ────────────────────────────────────────────────────────

/// Run `params` with its `--log-file` recorder attached (see the module
/// docs). The inner run gets the recorder's hub and no `log_file`.
///
/// [`run_graph_plan`] hands every run with a `log_file` to this function.
pub(crate) fn run_recorded(
    mut params: GraphPlanRunParams,
) -> Pin<Box<dyn Future<Output = anyhow::Result<i32>> + Send>> {
    Box::pin(async move {
        let Some(path) = params.log_file.take() else {
            return run_graph_plan(params).await;
        };
        let path = params.workdir.join(path);
        let hub = params
            .state_hub
            .get_or_insert_with(crate::state_hub::shared_state_hub)
            .clone();
        let log = RunEventLog::open(&path, &hub, params.resume_plan.is_some())
            .with_context(|| format!("open --log-file {}", path.display()))?;
        let result = run_graph_plan(params).await;
        if let Err(error) = log.finish(&result).await {
            tracing::warn!(
                path = %path.display(),
                error = %format!("{error:#}"),
                "--log-file is incomplete"
            );
        }
        result
    })
}

/// A run's `--log-file` recorder: an [`EventTap`] that writes JSONL.
pub struct RunEventLog {
    tap: EventTap<EventLogWriter>,
}

impl RunEventLog {
    /// Create (or truncate) `path` and start recording `hub`'s events.
    pub fn open(path: &Path, hub: &StateHub, resumed: bool) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let out = BufWriter::new(File::create(path)?);
        let run_id = std::env::var("ROKO_EVIDENCE_RUN_ID")
            .ok()
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| format!("graph-{}", uuid::Uuid::new_v4()));
        let writer = EventLogWriter {
            out,
            run_id,
            started_at: Utc::now(),
            resumed,
            wrote_start: false,
            task_outcomes: BTreeMap::new(),
            agent_calls: 0,
            events: 0,
            skipped: 0,
            write_error: None,
        };
        Ok(Self {
            tap: EventTap::spawn(hub, writer, EventLogWriter::record),
        })
    }

    /// Write the events published so far, then the `run.completed` line for
    /// the run's `result`.
    pub async fn finish(self, result: &anyhow::Result<i32>) -> anyhow::Result<()> {
        let mut writer = self.tap.finish().await?;
        writer.write_end(result);
        match writer.write_error.take() {
            Some(error) => Err(error.into()),
            None => Ok(()),
        }
    }
}

/// One hub event.
#[derive(Serialize)]
struct EventLine<'a> {
    /// `dashboard.<event type>`.
    #[serde(rename = "type")]
    kind: String,
    run_id: &'a str,
    seq: u64,
    ts_millis: u64,
    event: serde_json::Value,
}

/// Events the recorder missed.
#[derive(Serialize)]
struct LaggedLine<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    run_id: &'a str,
    skipped: u64,
}

/// The `run.completed` line.
#[derive(Serialize)]
struct RunEndLine<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    timestamp: String,
    timestamp_ms: u64,
    run_id: &'a str,
    outcome: RunOutcome,
    /// The run's exit status; absent when it ended with an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    duration_ms: u64,
    /// `TaskCompleted` events by outcome (`passed`, `failed`, ...).
    task_outcomes: &'a BTreeMap<String, usize>,
    /// `AgentSpawned` events.
    total_agent_calls: usize,
    events_recorded: u64,
    events_skipped: u64,
}

/// JSONL writer state folded by a [`RunEventLog`]'s tap.
struct EventLogWriter {
    out: BufWriter<File>,
    run_id: String,
    started_at: DateTime<Utc>,
    resumed: bool,
    wrote_start: bool,
    task_outcomes: BTreeMap<String, usize>,
    agent_calls: usize,
    events: u64,
    skipped: u64,
    /// First write failure; later lines are dropped.
    write_error: Option<std::io::Error>,
}

impl EventLogWriter {
    fn record(&mut self, tapped: Tapped<'_>) {
        match tapped {
            Tapped::Event(envelope) => {
                let event = &envelope.payload;
                let plan_set = match event {
                    DashboardEvent::PlanSetLoaded { plans } => Some(plans.as_slice()),
                    _ => None,
                };
                self.write_start(plan_set);
                match event {
                    DashboardEvent::TaskCompleted { outcome, .. } => {
                        *self.task_outcomes.entry(outcome.clone()).or_default() += 1;
                    }
                    DashboardEvent::AgentSpawned { .. } => self.agent_calls += 1,
                    _ => {}
                }
                self.events += 1;
                let event = serde_json::to_value(event).unwrap_or_default();
                let kind = event
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                let line = EventLine {
                    kind: format!("dashboard.{kind}"),
                    run_id: &self.run_id,
                    seq: envelope.seq,
                    ts_millis: envelope.ts_millis,
                    event,
                };
                write_line(&mut self.out, &mut self.write_error, &line);
            }
            Tapped::Lagged(skipped) => {
                self.write_start(None);
                self.skipped += skipped;
                let line = LaggedLine {
                    kind: "log.lagged",
                    run_id: &self.run_id,
                    skipped,
                };
                write_line(&mut self.out, &mut self.write_error, &line);
            }
        }
    }

    /// Write `run.started` once, before the first other line. The plan set
    /// comes from the run's `PlanSetLoaded` event, its first.
    fn write_start(&mut self, plan_set: Option<&[PlanSetEntry]>) {
        if self.wrote_start {
            return;
        }
        self.wrote_start = true;
        let plans = plan_set.unwrap_or_default();
        let start = RunnerEvent::RunStarted {
            timestamp: self.started_at.to_rfc3339(),
            timestamp_ms: unix_millis(self.started_at),
            run_id: self.run_id.clone(),
            plan_ids: plans.iter().map(|plan| plan.plan_id.clone()).collect(),
            total_tasks: plans.iter().map(|plan| plan.tasks_total).sum(),
            resumed: self.resumed,
            resume_session: None,
        };
        write_line(&mut self.out, &mut self.write_error, &start);
    }

    fn write_end(&mut self, result: &anyhow::Result<i32>) {
        self.write_start(None);
        let (outcome, exit_code, error) = match result {
            Ok(code) if *code == EXIT_SUCCESS => (RunOutcome::Succeeded, Some(*code), None),
            Ok(code)
                if [PlanRunInterrupt::Interrupt, PlanRunInterrupt::Terminate]
                    .iter()
                    .any(|interrupt| interrupt.exit_code() == *code) =>
            {
                (RunOutcome::Cancelled, Some(*code), None)
            }
            Ok(code) => (RunOutcome::Failed, Some(*code), None),
            Err(error) => (RunOutcome::Failed, None, Some(format!("{error:#}"))),
        };
        let now = Utc::now();
        let end = RunEndLine {
            kind: "run.completed",
            timestamp: now.to_rfc3339(),
            timestamp_ms: unix_millis(now),
            run_id: &self.run_id,
            outcome,
            exit_code,
            error,
            duration_ms: u64::try_from((now - self.started_at).num_milliseconds()).unwrap_or(0),
            task_outcomes: &self.task_outcomes,
            total_agent_calls: self.agent_calls,
            events_recorded: self.events,
            events_skipped: self.skipped,
        };
        write_line(&mut self.out, &mut self.write_error, &end);
    }
}

/// Append `line` as one flushed JSON line; after the first failure, record
/// it in `write_error` and drop later lines.
fn write_line<T: Serialize>(
    out: &mut BufWriter<File>,
    write_error: &mut Option<std::io::Error>,
    line: &T,
) {
    if write_error.is_some() {
        return;
    }
    let result = serde_json::to_writer(&mut *out, line)
        .map_err(std::io::Error::from)
        .and_then(|()| out.write_all(b"\n"))
        .and_then(|()| out.flush());
    if let Err(error) = result {
        *write_error = Some(error);
    }
}

fn unix_millis(at: DateTime<Utc>) -> u64 {
    u64::try_from(at.timestamp_millis()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_lines(path: &Path) -> Vec<serde_json::Value> {
        std::fs::read_to_string(path)
            .expect("read log")
            .lines()
            .map(|line| serde_json::from_str(line).expect("valid JSON line"))
            .collect()
    }

    #[tokio::test]
    async fn log_brackets_hub_events_with_one_start_and_one_terminal() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("events.jsonl");
        let hub = crate::state_hub::shared_state_hub();
        let log = RunEventLog::open(&path, &hub, false).expect("open log");

        let sender = hub.sender();
        sender.publish(DashboardEvent::PlanSetLoaded {
            plans: vec![PlanSetEntry {
                plan_id: "p1".to_string(),
                title: "Plan one".to_string(),
                tasks_total: 2,
                ..PlanSetEntry::default()
            }],
        });
        sender.publish(DashboardEvent::AgentSpawned {
            agent_id: "a1".to_string(),
            plan_id: "p1".to_string(),
            task_id: "T1".to_string(),
            attempt: 1,
            role: "implementer".to_string(),
            model: String::new(),
            provider: String::new(),
        });
        sender.publish(DashboardEvent::TaskCompleted {
            plan_id: "p1".to_string(),
            task_id: "T1".to_string(),
            outcome: "passed".to_string(),
        });
        log.finish(&Ok(1)).await.expect("finish log");

        let lines = read_lines(&path);
        let types: Vec<&str> = lines
            .iter()
            .map(|line| line["type"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(
            types,
            [
                "run.started",
                "dashboard.plan_set_loaded",
                "dashboard.agent_spawned",
                "dashboard.task_completed",
                "run.completed",
            ]
        );
        let run_id = lines[0]["run_id"].as_str().expect("run_id");
        assert!(lines.iter().all(|line| line["run_id"] == run_id));
        assert_eq!(lines[0]["plan_ids"], serde_json::json!(["p1"]));
        assert_eq!(lines[0]["total_tasks"], 2);
        let end = &lines[4];
        assert_eq!(end["outcome"], "failed");
        assert_eq!(end["exit_code"], 1);
        assert_eq!(end["total_agent_calls"], 1);
        assert_eq!(end["task_outcomes"]["passed"], 1);
        assert!(end["timestamp_ms"].as_u64() >= lines[0]["timestamp_ms"].as_u64());
    }

    #[tokio::test]
    async fn run_error_still_writes_start_and_terminal() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        let hub = crate::state_hub::shared_state_hub();
        let log = RunEventLog::open(&path, &hub, true).expect("open log");
        log.finish(&Err(anyhow!("plan set has a cycle")))
            .await
            .expect("finish log");

        let lines = read_lines(&path);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["type"], "run.started");
        assert_eq!(lines[0]["resumed"], true);
        assert_eq!(lines[1]["type"], "run.completed");
        assert_eq!(lines[1]["outcome"], "failed");
        assert!(lines[1].get("exit_code").is_none());
        assert_eq!(lines[1]["error"], "plan set has a cycle");
    }

    #[tokio::test]
    async fn interrupted_run_is_cancelled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        let hub = crate::state_hub::shared_state_hub();
        let log = RunEventLog::open(&path, &hub, false).expect("open log");
        log.finish(&Ok(PlanRunInterrupt::Terminate.exit_code()))
            .await
            .expect("finish log");

        let lines = read_lines(&path);
        assert_eq!(lines[1]["outcome"], "cancelled");
        assert_eq!(lines[1]["exit_code"], 143);
    }
}
