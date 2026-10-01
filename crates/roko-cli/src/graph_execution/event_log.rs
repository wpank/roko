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
//!
//! The log holds no agent or gate output text, because evidence bundles keep
//! derived data only. Each text field is replaced by its size and hash:
//! `agent_output` gives `content_bytes`, `content_lines` and `content_sha256`
//! (plus `stream_kind` for a TUI stream record), and streamed gate and task
//! output lines are replaced the same way. A `gate_result` also keeps
//! `output_text_excerpt`, the redacted last 240 bytes of its output, which
//! end with how a failed step ended (`✗ timed out after 600000 ms`).
//!
//! Every run, with or without `--log-file`, also records its events in the
//! workspace event log, `.roko/events.jsonl` ([`WorkspaceEventLog`]).

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
use roko_core::obs::LogScrubber;
use roko_runtime::event_bus::Envelope;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::sync::broadcast::error::{RecvError, TryRecvError};
use tokio::sync::oneshot;

use super::plan_runner::{
    GraphPlanRunParams, PlanRunInterrupt, run_graph_plan, run_graph_plan_observed,
};
use crate::exit_codes::EXIT_SUCCESS;
use crate::runner::tui_bridge::STREAM_RECORD_PREFIX;
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
        // The log, `status.json` and the workspace event log name the run
        // alike.
        let run_id = graph_run_id(None);
        let log =
            RunEventLog::open_for_run(&path, &hub, params.resume_plan.is_some(), run_id.clone())
                .with_context(|| format!("open --log-file {}", path.display()))?;
        let result = run_graph_plan_observed(params, None, run_id).await;
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
    /// Create (or truncate) `path` and start recording `hub`'s events under a
    /// new run id ([`graph_run_id`]).
    pub fn open(path: &Path, hub: &StateHub, resumed: bool) -> std::io::Result<Self> {
        Self::open_for_run(path, hub, resumed, graph_run_id(None))
    }

    /// [`Self::open`] for the run `run_id`.
    pub fn open_for_run(
        path: &Path,
        hub: &StateHub,
        resumed: bool,
        run_id: String,
    ) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let out = BufWriter::new(File::create(path)?);
        let writer = EventLogWriter {
            out,
            run_id,
            scrubber: output_scrubber(),
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

/// The evidence collector's run id (`ROKO_EVIDENCE_RUN_ID`) when a run is
/// recorded under `scripts/run_evidence.py`. The run's `--log-file` lines and
/// its `status.json` carry it, so the collector can tell them from other
/// runs' records.
pub(crate) fn evidence_run_id() -> Option<String> {
    std::env::var("ROKO_EVIDENCE_RUN_ID")
        .ok()
        .filter(|id| !id.trim().is_empty())
}

/// The id a Graph run's records name it by: the evidence collector's
/// ([`evidence_run_id`]), else the caller's `run_id`, else a new
/// `graph-<uuid>`.
pub(crate) fn graph_run_id(run_id: Option<&str>) -> String {
    evidence_run_id()
        .or_else(|| run_id.map(str::to_string))
        .unwrap_or_else(|| format!("graph-{}", uuid::Uuid::new_v4()))
}

// ── Workspace event log ──────────────────────────────────────────────────

/// Records a Graph run's hub events in the workspace event log,
/// `.roko/events.jsonl`, and in the run's derived index,
/// `.roko/events-by-run/<sha256(run id)>.jsonl`, whether or not it has a
/// `--log-file` (bug-230de6). A dashboard in another terminal, serve's gate
/// evidence, the runs route and `roko doctor` read them.
///
/// Each line is the [`DashboardEvent`] as the hub publishes it, the format
/// the TUI replays, with the run's `run_id` added, by which the index is
/// derived and repaired. A hub that writes the workspace log itself (serve's,
/// see [`StateHub::persists_events`]) gets the index only. Plan, task, gate
/// and run lifecycle events are synced and flush the index; the rest, agent
/// output above all, are appended without a sync per line.
pub(crate) struct WorkspaceEventLog {
    tap: EventTap<WorkspaceLogWriter>,
}

impl WorkspaceEventLog {
    /// Start recording `hub`'s events for the run `run_id` in `workdir`;
    /// `None` when its `.roko` directories cannot be created.
    pub(crate) fn spawn(hub: &StateHub, workdir: &Path, run_id: String) -> Option<Self> {
        let paths = crate::runner::persist::PersistPaths::from_workdir(workdir)
            .inspect_err(|error| {
                tracing::warn!(error = %format!("{error:#}"), "the run's events are not recorded");
            })
            .ok()?;
        let writer = WorkspaceLogWriter {
            paths,
            run_id,
            index_only: hub.persists_events(),
            failed: false,
        };
        Some(Self {
            tap: EventTap::spawn(hub, writer, WorkspaceLogWriter::record),
        })
    }

    /// Record the events published so far, flush the run's index, and stop.
    pub(crate) async fn finish(self) {
        let flushed = self.tap.finish().await.and_then(|writer| {
            crate::runner::persist::flush_run_index(&writer.paths, &writer.run_id)
        });
        if let Err(error) = flushed {
            tracing::warn!(
                error = %format!("{error:#}"),
                "the run's event log is incomplete"
            );
        }
    }
}

/// Writer state folded by a [`WorkspaceEventLog`]'s tap.
struct WorkspaceLogWriter {
    paths: crate::runner::persist::PersistPaths,
    run_id: String,
    /// The hub writes `.roko/events.jsonl` itself.
    index_only: bool,
    /// A write failed, and was reported.
    failed: bool,
}

impl WorkspaceLogWriter {
    fn record(&mut self, tapped: Tapped<'_>) {
        let event = match tapped {
            Tapped::Event(envelope) => &envelope.payload,
            Tapped::Lagged(skipped) => {
                tracing::warn!(
                    run_id = %self.run_id,
                    skipped,
                    "the run's event log fell behind and missed events"
                );
                return;
            }
        };
        if !crate::state_hub::should_persist(event) {
            return;
        }
        let mut line = serde_json::to_value(event).unwrap_or_default();
        if let Some(fields) = line.as_object_mut() {
            fields
                .entry("run_id")
                .or_insert_with(|| serde_json::json!(self.run_id));
        }
        let lifecycle = matches!(
            event,
            DashboardEvent::PlanSetLoaded { .. }
                | DashboardEvent::PlanStarted { .. }
                | DashboardEvent::PlanCompleted { .. }
                | DashboardEvent::TaskStarted { .. }
                | DashboardEvent::TaskCompleted { .. }
                | DashboardEvent::GateResult { .. }
                | DashboardEvent::RunCompleted { .. }
        );
        let result = if self.index_only {
            crate::runner::persist::append_run_index_event(
                &self.paths,
                &self.run_id,
                &line,
                lifecycle,
            )
        } else {
            let durability = if lifecycle {
                crate::runner::persist::EventDurability::Durable
            } else {
                crate::runner::persist::EventDurability::Relaxed
            };
            crate::runner::persist::append_run_scoped_event(
                &self.paths,
                &self.run_id,
                &line,
                durability,
                lifecycle,
            )
        };
        if let Err(error) = result
            && !std::mem::replace(&mut self.failed, true)
        {
            tracing::warn!(
                run_id = %self.run_id,
                error = %format!("{error:#}"),
                "the run's event log could not be written"
            );
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
    /// Redacts gate output excerpts.
    scrubber: LogScrubber,
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
                let event = logged_event(event, &self.scrubber);
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

// ── Output scrubbing ─────────────────────────────────────────────────────

/// How much of a gate's output the log keeps, in bytes: its redacted tail,
/// which holds the closing `✗` line the evidence collector reads timeouts
/// from.
const GATE_OUTPUT_EXCERPT_BYTES: usize = 240;

/// `event` as it is logged: its JSON, with every agent, gate and task output
/// text replaced by derived fields (see the module docs).
fn logged_event(event: &DashboardEvent, scrubber: &LogScrubber) -> serde_json::Value {
    let mut value = serde_json::to_value(event).unwrap_or_default();
    if let Some(fields) = value.as_object_mut() {
        match event {
            DashboardEvent::AgentOutput { content, .. } => {
                replace_with_digest(fields, "content", content);
                if let Some(kind) = stream_record_kind(content) {
                    fields.insert("stream_kind".to_string(), serde_json::json!(kind));
                }
            }
            DashboardEvent::GateResult {
                output_text: Some(text),
                ..
            } => {
                replace_with_digest(fields, "output_text", text);
                fields.insert(
                    "output_text_excerpt".to_string(),
                    serde_json::json!(gate_output_excerpt(text, scrubber)),
                );
            }
            DashboardEvent::GateOutputLine { line, .. } => {
                replace_with_digest(fields, "line", line);
            }
            DashboardEvent::TaskOutputAppended { lines, .. } => {
                replace_with_digest(fields, "lines", &lines.join("\n"));
            }
            _ => {}
        }
    }
    value
}

/// Replace the text field `name` by `<name>_bytes`, `<name>_lines` and
/// `<name>_sha256`.
fn replace_with_digest(
    fields: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
    text: &str,
) {
    fields.remove(name);
    fields.insert(format!("{name}_bytes"), serde_json::json!(text.len()));
    fields.insert(
        format!("{name}_lines"),
        serde_json::json!(text.lines().count()),
    );
    fields.insert(
        format!("{name}_sha256"),
        serde_json::json!(sha256_hex(text)),
    );
}

fn sha256_hex(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// The `kind` (`text`, `tool_start`, `tool_result`, ...) of a TUI stream
/// record, when `content` is one.
fn stream_record_kind(content: &str) -> Option<String> {
    let record = content.strip_prefix(STREAM_RECORD_PREFIX)?;
    let record: serde_json::Value = serde_json::from_str(record).ok()?;
    record.get("kind")?.as_str().map(str::to_string)
}

/// The redacted last [`GATE_OUTPUT_EXCERPT_BYTES`] of a gate's output,
/// marked with a leading `…` when cut. Redaction runs first, so a cut never
/// leaves part of a secret the scrubber would have matched whole.
fn gate_output_excerpt(text: &str, scrubber: &LogScrubber) -> String {
    let redacted = scrubber.scrub(text.trim_end());
    let mut start = redacted.len().saturating_sub(GATE_OUTPUT_EXCERPT_BYTES);
    while !redacted.is_char_boundary(start) {
        start += 1;
    }
    if start == 0 {
        redacted
    } else {
        format!("…{}", &redacted[start..])
    }
}

/// The scrubber for gate output excerpts: the built-in secret patterns,
/// `<secret name>=<value>` assignments, and the value of every
/// secret-named environment variable, which is what the evidence collector's
/// own redaction covers.
fn output_scrubber() -> LogScrubber {
    let scrubber = LogScrubber::new();
    let _ = scrubber.add_pattern_with_replacement(
        r"(?i)((?:api[_-]?key|password|private[_-]?key|secret|token)\s*[=:]\s*)[^\s,;]+",
        "${1}[REDACTED]",
    );
    for (name, value) in std::env::vars_os() {
        if let (Ok(name), Ok(value)) = (name.into_string(), value.into_string())
            && value.len() >= 8
            && roko_core::child_env::is_secret_env_name(&name)
        {
            let _ = scrubber.add_literal_value(&value, &name);
        }
    }
    scrubber
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

    /// bug-230de6: a run's hub events land in `.roko/events.jsonl` as the
    /// dashboard events the TUI replays, stamped with the run's id, and in
    /// the run's derived index. A hub that writes the workspace log itself
    /// gets the index only, so no event is written twice.
    #[tokio::test]
    async fn workspace_event_log_records_replayable_events_and_the_runs_index() {
        let completed = || DashboardEvent::TaskCompleted {
            plan_id: "p1".to_string(),
            task_id: "T1".to_string(),
            outcome: "passed".to_string(),
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let hub = crate::state_hub::shared_state_hub();
        let log = WorkspaceEventLog::spawn(&hub, dir.path(), "graph-test-1".to_string())
            .expect("record the run");
        let sender = hub.sender();
        sender.publish(completed());
        sender.publish(DashboardEvent::GateResult {
            plan_id: "p1".to_string(),
            task_id: "T1".to_string(),
            gate: "verify[0:compile]".to_string(),
            passed: true,
            output_text: Some("$ cargo check".to_string()),
        });
        log.finish().await;

        let events_path = dir.path().join(".roko/events.jsonl");
        let lines = read_lines(&events_path);
        let types: Vec<&str> = lines
            .iter()
            .map(|line| line["type"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(types, ["task_completed", "gate_result"]);
        assert!(lines.iter().all(|line| line["run_id"] == "graph-test-1"));
        let replay = crate::state_hub::shared_state_hub();
        let mut reader = std::io::BufReader::new(File::open(&events_path).expect("open log"));
        assert_eq!(replay.replay_events_from_reader(&mut reader), 2);
        let index =
            roko_fs::run_index::run_index_path(&events_path, "graph-test-1").expect("index path");
        assert_eq!(read_lines(&index), lines);

        let durable_dir = tempfile::tempdir().expect("tempdir");
        let durable_events = durable_dir.path().join(".roko/events.jsonl");
        let durable = crate::state_hub::SharedStateHub::new(
            crate::state_hub::StateHub::with_event_log(64, &durable_events),
        );
        let log =
            WorkspaceEventLog::spawn(&durable, durable_dir.path(), "graph-test-2".to_string())
                .expect("record the run");
        durable.sender().publish(completed());
        log.finish().await;
        let lines = read_lines(&durable_events);
        assert_eq!(lines.len(), 1, "the hub wrote it, once");
        assert!(lines[0].get("run_id").is_none());
        let index = roko_fs::run_index::run_index_path(&durable_events, "graph-test-2")
            .expect("index path");
        let indexed = read_lines(&index);
        assert_eq!(indexed.len(), 1);
        assert_eq!(indexed[0]["type"], "task_completed");
        assert_eq!(indexed[0]["run_id"], "graph-test-2");
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

    /// The `event` objects of the log's `dashboard.<kind>` lines.
    fn logged_events<'a>(lines: &'a [serde_json::Value], kind: &str) -> Vec<&'a serde_json::Value> {
        let line_type = format!("dashboard.{kind}");
        lines
            .iter()
            .filter(|line| line["type"] == line_type)
            .map(|line| &line["event"])
            .collect()
    }

    #[tokio::test]
    async fn log_file_never_copies_agent_output_text() {
        const CANARY: &str = "canary-4c4eea: text only the agent printed";
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        let hub = crate::state_hub::shared_state_hub();
        let log = RunEventLog::open(&path, &hub, false).expect("open log");

        let sender = hub.sender();
        sender.publish(DashboardEvent::AgentOutput {
            agent_id: "a1".to_string(),
            plan_id: "p1".to_string(),
            task_id: "T1".to_string(),
            attempt: 2,
            content: CANARY.to_string(),
        });
        let record = serde_json::json!({
            "kind": "tool_result",
            "payload": {"tool_id": "call-1", "output": CANARY},
        });
        let record = format!("{STREAM_RECORD_PREFIX}{record}");
        sender.publish(DashboardEvent::AgentOutput {
            agent_id: "a1".to_string(),
            plan_id: "p1".to_string(),
            task_id: "T1".to_string(),
            attempt: 2,
            content: record.clone(),
        });
        sender.publish(DashboardEvent::TaskOutputAppended {
            task_id: "T1".to_string(),
            lines: vec![CANARY.to_string(), CANARY.to_string()],
        });
        sender.publish(DashboardEvent::GateOutputLine {
            plan_id: "p1".to_string(),
            task_id: "T1".to_string(),
            gate: "verify[0]".to_string(),
            line: CANARY.to_string(),
        });
        log.finish(&Ok(0)).await.expect("finish log");

        let raw = std::fs::read_to_string(&path).expect("read log");
        assert!(
            !raw.contains("canary-4c4eea"),
            "the log copied output text:\n{raw}"
        );
        let lines = read_lines(&path);

        let outputs = logged_events(&lines, "agent_output");
        assert_eq!(outputs.len(), 2);
        let plain = outputs[0];
        assert!(plain.get("content").is_none());
        assert_eq!(plain["content_sha256"], sha256_hex(CANARY));
        assert_eq!(plain["content_bytes"], CANARY.len());
        assert_eq!(plain["content_lines"], 1);
        assert_eq!(plain["agent_id"], "a1");
        assert_eq!(plain["plan_id"], "p1");
        assert_eq!(plain["task_id"], "T1");
        assert_eq!(plain["attempt"], 2);
        assert!(plain.get("stream_kind").is_none());
        let streamed = outputs[1];
        assert!(streamed.get("content").is_none());
        assert_eq!(streamed["content_sha256"], sha256_hex(&record));
        assert_eq!(streamed["stream_kind"], "tool_result");

        let [appended] = logged_events(&lines, "task_output_appended")[..] else {
            panic!("one task_output_appended line");
        };
        assert!(appended.get("lines").is_none());
        assert_eq!(appended["lines_lines"], 2);
        assert_eq!(
            appended["lines_sha256"],
            sha256_hex(&format!("{CANARY}\n{CANARY}"))
        );
        let [gate_line] = logged_events(&lines, "gate_output_line")[..] else {
            panic!("one gate_output_line line");
        };
        assert!(gate_line.get("line").is_none());
        assert_eq!(gate_line["line_sha256"], sha256_hex(CANARY));
        assert_eq!(gate_line["gate"], "verify[0]");
    }

    #[tokio::test]
    async fn gate_output_is_logged_as_a_redacted_tail() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        let hub = crate::state_hub::shared_state_hub();
        let log = RunEventLog::open(&path, &hub, false).expect("open log");

        let early: String = (1..=50).map(|n| format!("early output {n}\n")).collect();
        let output = format!(
            "$ ./check.sh\n{early}export OPENAI_API_KEY=sk-proj-abcdefghijklmnopqrstuvwxyz0123\n\
             password=hunter2hunter2\n✗ timed out after 600000 ms"
        );
        let sender = hub.sender();
        sender.publish(DashboardEvent::GateResult {
            plan_id: "p1".to_string(),
            task_id: "T1".to_string(),
            gate: "verify[0:test]".to_string(),
            passed: false,
            output_text: Some(output.clone()),
        });
        sender.publish(DashboardEvent::GateResult {
            plan_id: "p1".to_string(),
            task_id: "T2".to_string(),
            gate: "verify[0]".to_string(),
            passed: true,
            output_text: None,
        });
        log.finish(&Ok(1)).await.expect("finish log");

        let lines = read_lines(&path);
        let [failed, passed] = logged_events(&lines, "gate_result")[..] else {
            panic!("two gate_result lines");
        };
        assert!(failed.get("output_text").is_none());
        assert_eq!(failed["output_text_sha256"], sha256_hex(&output));
        assert_eq!(failed["output_text_bytes"], output.len());
        assert_eq!(failed["output_text_lines"], output.lines().count());
        let excerpt = failed["output_text_excerpt"].as_str().expect("excerpt");
        let tail = excerpt.strip_prefix('…').expect("leading …");
        assert_eq!(tail.len(), GATE_OUTPUT_EXCERPT_BYTES);
        assert!(tail.ends_with("timed out after 600000 ms"), "{excerpt}");
        assert!(!excerpt.contains("early output 1\n"), "{excerpt}");
        for secret in ["sk-proj-", "hunter2"] {
            assert!(!excerpt.contains(secret), "{secret} in {excerpt}");
        }
        assert!(excerpt.contains("[REDACTED"), "{excerpt}");
        assert!(passed["output_text"].is_null());
        assert!(passed.get("output_text_excerpt").is_none());

        let short = gate_output_excerpt("$ true\nok\n", &output_scrubber());
        assert_eq!(short, "$ true\nok");
    }
}
