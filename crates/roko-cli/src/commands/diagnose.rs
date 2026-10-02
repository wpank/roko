//! `roko diagnose <plan-id>` — diagnostic report for plan failures, as readable
//! text or, with `--json`, as structured JSON.
//!
//! A Graph run is reported from the plan's checkpoint under
//! `.roko/state/graph/<plan>/` (status, recorded task outputs, spend), the
//! plan's `tasks.toml`, and the logs Graph task dispatch appends:
//! `.roko/learn/costs.jsonl` (one row per provider attempt),
//! `.roko/learn/gate-failures.jsonl` (failed verify steps),
//! `.roko/episodes.jsonl` (failure reasons) and the run's
//! `.roko/runs/<run>/attempts.jsonl` (tool policies). The Runner-v2 snapshot at
//! `.roko/state/state-snapshot.json` is read only for a plan without a Graph
//! checkpoint.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write as _;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, SecondsFormat, Utc};
use regex::Regex;
use roko_fs::RokoLayout;
use roko_gate::{FailureClass, GateFailureAction, GateFailureKind, GateFailureRecord};
use roko_graph::cells::task_executor::TaskGateVerdict;
use roko_learn::costs_db::CostRecord;
use roko_learn::episode_logger::Episode;
use roko_learn::telemetry::{CostSource, ToolPolicyRecord};
use roko_runtime::{
    DurableRunnerProjection, STATE_SNAPSHOT_RELATIVE_PATH, load_durable_runner_projection,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::graph_checkpoint::{
    CheckpointInspection, GraphCheckpointStatus, InvalidatedActivity, ResumeAction, ResumeOptions,
    ResumePreview, inspect_canonical_checkpoint, preview_plan_resume,
};
use crate::runner::plan_loader::Plan;
use crate::task_parser::{TaskDef, TasksFile};

/// Canonical Graph checkpoint root, relative to the workspace.
const GRAPH_STATE_DIR: &str = ".roko/state/graph";

/// Largest gap between an attempt's row in `costs.jsonl` and its episode.
/// Graph task dispatch writes both when the attempt ends.
const EPISODE_MATCH_WINDOW_MS: i64 = 5_000;

/// Episode ids kept on [`FailedTaskInfo`].
const FAILED_TASK_EPISODE_IDS: usize = 5;

/// Run the diagnose command, printing the report to stdout as text, or as JSON
/// with `json`.
pub fn cmd_diagnose(workdir: &Path, plan_id: &str, verbose: bool, json: bool) -> Result<i32> {
    let report = build_report(workdir, plan_id, verbose)?;
    if json {
        let json = serde_json::to_string_pretty(&report).context("serializing diagnose report")?;
        println!("{json}");
    } else {
        print!("{}", render_text(&report, verbose));
    }
    Ok(0)
}

// ---------------------------------------------------------------------------
// Report types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct DiagnoseReport {
    pub plan_id: String,
    /// `completed`, `failed`, `running`, `cancelled` or `interrupted` for a
    /// Graph run; derived from the plan phase and its paused flag for a
    /// Runner-v2 snapshot.
    pub status: String,
    /// Where the run state came from.
    pub source: ReportSource,
    pub phase: Option<String>,
    pub iteration: Option<u32>,
    /// The run the plan's Graph checkpoint records.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_run: Option<GraphRunInfo>,
    /// Directory holding the plan's `tasks.toml`, relative to the workspace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_dir: Option<String>,
    pub failed_task: Option<FailedTaskInfo>,
    /// Every plan task as the Graph run left it, in plan order, followed by
    /// tasks the run recorded that the plan no longer defines.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tasks: Vec<TaskDiagnosis>,
    pub gate_results: Vec<GateResultInfo>,
    pub run_state: Option<RunStateSummary>,
    /// What re-running the plan would do with its Graph checkpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<ResumeInfo>,
    pub git_state: Option<GitStateInfo>,
    pub suggested_recovery: Vec<String>,
    /// What the report could not read or left out.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    /// Top-level plan cost derived from `.roko/learn/efficiency.jsonl`,
    /// summed over every recorded run of the plan. `None` when no efficiency
    /// data exists for this plan. A Graph run's own spend is
    /// `graph_run.spent_usd`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
}

/// Where a report's run state came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportSource {
    /// The plan's Graph checkpoint under `.roko/state/graph/<plan>/`.
    GraphCheckpoint,
    /// The Runner-v2 state snapshot, for a plan without a Graph checkpoint.
    RunnerSnapshot,
}

#[derive(Debug, Serialize)]
pub struct FailedTaskInfo {
    pub task_id: String,
    pub last_error: Option<String>,
    pub files_changed: Vec<String>,
    /// Episode IDs from `.roko/episodes.jsonl` linked to this failed task.
    /// At most 5, ordered most-recent-last.
    #[serde(default)]
    pub episode_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct GateResultInfo {
    /// Task whose verify step failed. `None` for a Runner-v2 snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub gate_name: String,
    pub rung: u32,
    pub passed: bool,
    pub summary: String,
    pub duration_ms: u64,
    /// Classified errors extracted from the gate summary / output.
    #[serde(default)]
    pub classified_errors: Vec<ClassifiedError>,
}

/// One classified error extracted from gate output via regex pattern matching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClassifiedError {
    /// Category of the error.
    pub error_class: ErrorClass,
    /// Source file, if identifiable from the error output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Line number in the source file, if identifiable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// One-line human-readable error summary.
    pub error_summary: String,
    /// Suggested remediation action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Error classification categories for gate failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorClass {
    /// Rust compiler error (e.g. `error[E0308]: mismatched types`).
    CompileError,
    /// Test failure (e.g. `test result: FAILED`).
    TestFailure,
    /// Clippy or other lint warning/error.
    LintWarning,
    /// Linker error (e.g. `ld: symbol(s) not found`).
    LinkError,
    /// Runtime panic (e.g. `thread 'main' panicked at`).
    RuntimePanic,
    /// Timeout or process killed.
    Timeout,
    /// Unable to classify the error.
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct RunStateSummary {
    pub tasks_total: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub total_cost_usd: f64,
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub total_agent_calls: usize,
    pub completed_tasks: Vec<String>,
    pub failed_tasks: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct GitStateInfo {
    pub current_branch: Option<String>,
    pub has_uncommitted_changes: bool,
    pub plan_branch_exists: bool,
}

/// The run a plan's Graph checkpoint records.
#[derive(Debug, Serialize)]
pub struct GraphRunInfo {
    /// Run id stamped on every Activity record.
    pub run_id: String,
    /// Fingerprint of the plan graph the run executed.
    pub graph_fingerprint: String,
    /// Status as the checkpoint records it.
    pub checkpoint_status: GraphCheckpointStatus,
    /// Last checkpoint write, RFC 3339 UTC.
    pub updated_at: Option<String>,
    /// Checkpoint manifest, relative to the workspace.
    pub checkpoint: String,
    /// Actual provider spend in the run's cost ledger, in USD.
    pub spent_usd: Option<f64>,
    /// Spend reserved for a provider call that never settled, in USD.
    pub reserved_usd: Option<f64>,
    /// When the checkpoint replaced an earlier run's. Attempts, gate failures
    /// and episodes from before this time belong to that run and are left out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub earlier_runs_before: Option<String>,
    /// Task outputs the last resume discarded because their gate verdict was
    /// not a pass; those tasks ran again.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub invalidated_on_resume: Vec<InvalidatedActivity>,
}

/// How a plan task ended in the diagnosed Graph run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// The run recorded the task's output.
    Completed,
    /// The task ran without completing, and the run failed.
    Failed,
    /// The task ran without completing, and the run has not failed: it is
    /// still running, or was interrupted or cancelled.
    Incomplete,
    /// Nothing was recorded for the task in this run.
    NeverRan,
}

/// One plan task as the diagnosed Graph run left it.
#[derive(Debug, Serialize)]
pub struct TaskDiagnosis {
    pub task_id: String,
    /// Title from `tasks.toml`; `None` for a task the plan no longer defines.
    pub title: Option<String>,
    pub state: TaskState,
    /// One line saying why the task is in `state`.
    pub reason: String,
    pub depends_on: Vec<String>,
    /// For a task that never ran: the tasks that did not complete and that it
    /// waits on, directly or through dependencies that never ran either.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<String>,
    /// Gate verdict recorded with the task's output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate_verdict: Option<TaskGateVerdict>,
    pub attempt_count: usize,
    pub failed_attempts: usize,
    pub timed_out_attempts: usize,
    /// Most recent failure: the last failed verify step, or the failure
    /// reason of the last failed attempt when it is more recent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    /// Provider attempts, oldest first. Listed for tasks that did not
    /// complete, and for every task with `--verbose`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attempts: Vec<AttemptInfo>,
    /// Failed verify steps, oldest first. Listed like `attempts`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gate_failures: Vec<GateFailureInfo>,
    /// Episodes from `.roko/episodes.jsonl`, oldest first. Listed like
    /// `attempts`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub episode_ids: Vec<String>,
    /// The tool policy of each attempt that recorded one, oldest first.
    /// Listed like `attempts`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tool_policies: Vec<AttemptToolPolicy>,
}

/// One provider attempt at a task, from `.roko/learn/costs.jsonl`.
#[derive(Debug, Serialize)]
pub struct AttemptInfo {
    /// When the attempt ended.
    pub timestamp: String,
    pub model: String,
    pub provider: String,
    /// Whether the attempt succeeded, verify steps included.
    pub success: bool,
    pub duration_ms: u64,
    pub cost_usd: f64,
    /// Where the usage behind `cost_usd` came from: `estimated` for usage the
    /// attempt streamed before it was cut off, which no provider reported
    /// (gap-288e38); `unknown` for a row written before the field.
    pub cost_source: CostSource,
    /// The attempt timed out: its failure reason says so, or one of its
    /// verify steps is recorded as a timeout.
    pub timed_out: bool,
    /// Failure reason recorded on the attempt's episode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_id: Option<String>,
}

/// One attempt's tool policy, from its verdict in the run's `attempts.jsonl`
/// (`executed.tool_policy`): what its contract asked for, what its provider
/// enforced, and the operation it was stopped at. Only an attempt on a
/// provider that runs its own tools, such as Codex, records one (gap-baab0a).
#[derive(Debug, Clone, Serialize)]
pub struct AttemptToolPolicy {
    /// The attempt's number in the run.
    pub attempt: u32,
    /// The model the attempt was launched on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The recorded policy, its fields beside `attempt` and `model`.
    #[serde(flatten)]
    pub policy: ToolPolicyRecord,
}

/// One failed verify step, from `.roko/learn/gate-failures.jsonl`.
#[derive(Debug, Serialize)]
pub struct GateFailureInfo {
    pub timestamp: String,
    pub gate_name: String,
    /// The verify step the summary names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify_step: Option<VerifyStepInfo>,
    pub failure_kind: GateFailureKind,
    pub primary_class: FailureClass,
    pub recommended_action: GateFailureAction,
    /// Summary as recorded: on Graph runs the step's label, its failure
    /// message and its output, at most 2 KiB (bug-6f7f72). Records written
    /// before then hold the first 200 characters of the failure text.
    pub summary: String,
}

/// A task's verify step.
#[derive(Debug, Serialize)]
pub struct VerifyStepInfo {
    /// Zero-based position among the task's `[[task.verify]]` steps.
    pub index: usize,
    /// Phase label, such as `test`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    /// The step's full command in the current `tasks.toml`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

impl VerifyStepInfo {
    /// `1 (test)`, or `1` without a phase.
    fn label(&self) -> String {
        match &self.phase {
            Some(phase) => format!("{} ({phase})", self.index),
            None => self.index.to_string(),
        }
    }
}

/// What re-running the plan with default options would do with its Graph
/// checkpoint.
#[derive(Debug, Serialize)]
pub struct ResumeInfo {
    /// The command that re-runs the plan.
    pub command: String,
    /// The checkpoint preview `roko plan run --dry-run` prints.
    #[serde(flatten)]
    pub preview: ResumePreview,
}

// ---------------------------------------------------------------------------
// Report builder
// ---------------------------------------------------------------------------

pub(crate) fn build_report(workdir: &Path, plan_id: &str, verbose: bool) -> Result<DiagnoseReport> {
    let checkpoint = inspect_canonical_checkpoint(workdir, plan_id)
        .with_context(|| format!("reading the Graph checkpoint of plan '{plan_id}'"))?;
    if let Some(checkpoint) = checkpoint {
        return Ok(build_graph_report(workdir, plan_id, &checkpoint, verbose));
    }

    // Only a plan that last ran under the removed Runner-v2 engine has no
    // Graph checkpoint; its state is in the unified snapshot.
    let projection = load_durable_runner_projection(workdir).context("reading state snapshot")?;
    let Some(projection) = projection else {
        bail!(
            "No run state for plan '{plan_id}': it has no Graph checkpoint under {} and there is no Runner-v2 snapshot at {}.{} Run `roko plan run` first.",
            workdir.join(GRAPH_STATE_DIR).display(),
            workdir.join(STATE_SNAPSHOT_RELATIVE_PATH).display(),
            checkpointed_plans_hint(workdir),
        );
    };
    build_legacy_report(workdir, plan_id, &projection)
}

/// ` Plans with a Graph checkpoint: a, b.`, or empty when there are none.
fn checkpointed_plans_hint(workdir: &Path) -> String {
    let mut plans: Vec<String> = std::fs::read_dir(workdir.join(GRAPH_STATE_DIR))
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("checkpoint.json").is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    if plans.is_empty() {
        return String::new();
    }
    plans.sort();
    format!(" Plans with a Graph checkpoint: {}.", plans.join(", "))
}

// ---------------------------------------------------------------------------
// Graph runs
// ---------------------------------------------------------------------------

/// A plan's directory and parsed `tasks.toml`.
struct PlanDefinition {
    dir: PathBuf,
    tasks: TasksFile,
}

/// What Graph task dispatch logged for one plan during one run.
struct RunRecords {
    /// Provider attempts from `.roko/learn/costs.jsonl`.
    attempts: Vec<CostRecord>,
    /// Failed verify steps from `.roko/learn/gate-failures.jsonl`.
    gate_failures: Vec<GateFailureRecord>,
    /// Task episodes from `.roko/episodes.jsonl`.
    episodes: Vec<Episode>,
    /// Attempts' tool policies from the run's `attempts.jsonl`, with their
    /// task ids, oldest first.
    tool_policies: Vec<(String, AttemptToolPolicy)>,
}

impl RunRecords {
    /// `plan_id`'s records written at or after `since_ms` (all of them when
    /// `None`), and its attempts' tool policies in run `run_id`. A record
    /// whose time cannot be read is kept.
    fn load(workdir: &Path, plan_id: &str, run_id: &str, since_ms: Option<i64>) -> Self {
        let layout = RokoLayout::for_project(workdir);
        let in_run = |at_ms: Option<i64>| since_ms.zip(at_ms).is_none_or(|(since, at)| at >= since);
        Self {
            attempts: read_jsonl_lossy::<CostRecord>(&layout.learn_dir().join("costs.jsonl"))
                .into_iter()
                .filter(|record| record.plan_id == plan_id && in_run(rfc3339_ms(&record.timestamp)))
                .collect(),
            gate_failures: read_jsonl_lossy::<GateFailureRecord>(&layout.gate_failures_path())
                .into_iter()
                .filter(|record| {
                    record.plan_id == plan_id && in_run(Some(record.timestamp.timestamp_millis()))
                })
                .collect(),
            episodes: read_jsonl_lossy::<Episode>(&layout.episodes_path())
                .into_iter()
                .filter(|episode| {
                    episode_plan_id(episode) == Some(plan_id)
                        && in_run(Some(episode.timestamp.timestamp_millis()))
                })
                .collect(),
            tool_policies: recorded_tool_policies(&layout.run_dir(run_id), plan_id),
        }
    }

    /// Every task id the records mention.
    fn task_ids(&self) -> impl Iterator<Item = &str> {
        self.attempts
            .iter()
            .map(|record| record.task_id.as_str())
            .chain(
                self.gate_failures
                    .iter()
                    .map(|record| record.task_id.as_str()),
            )
            .chain(self.episodes.iter().map(|episode| episode.task_id.as_str()))
            .filter(|task_id| !task_id.is_empty())
    }
}

/// The tool policies `plan_id`'s attempts recorded in their verdicts in
/// `run_dir`'s `attempts.jsonl`, with their task ids, oldest first. An
/// unreadable log has none.
fn recorded_tool_policies(run_dir: &Path, plan_id: &str) -> Vec<(String, AttemptToolPolicy)> {
    let Ok(run) = roko_learn::telemetry::report::RunRecords::load(run_dir) else {
        return Vec::new();
    };
    run.verdicts
        .into_iter()
        .map(|verdict| verdict.record)
        .filter(|record| record.identity.plan_id == plan_id)
        .filter_map(|record| {
            let policy = record.executed.tool_policy?;
            let attempt = AttemptToolPolicy {
                attempt: record.identity.attempt,
                model: record.executed.model_dispatched,
                policy,
            };
            Some((record.identity.task_id, attempt))
        })
        .collect()
}

/// Report a plan's Graph run from its checkpoint, its `tasks.toml` and the
/// logs Graph task dispatch appended during the run.
fn build_graph_report(
    workdir: &Path,
    plan_id: &str,
    checkpoint: &CheckpointInspection,
    verbose: bool,
) -> DiagnoseReport {
    let mut notes = Vec::new();
    let status = checkpoint.manifest.status;
    let since_ms = checkpoint
        .replaced_at_ms
        .and_then(|ms| i64::try_from(ms).ok());
    let records = RunRecords::load(workdir, plan_id, &checkpoint.manifest.run_id, since_ms);
    let definition = load_plan_definition(workdir, plan_id, &mut notes);
    let tasks = diagnose_tasks(checkpoint, definition.as_ref(), &records, verbose);
    let resume = definition
        .as_ref()
        .and_then(|definition| resume_info(workdir, plan_id, definition, &mut notes));

    let failed_task = tasks
        .iter()
        .find(|task| task.state == TaskState::Failed)
        .map(|task| {
            let recent = task
                .episode_ids
                .len()
                .saturating_sub(FAILED_TASK_EPISODE_IDS);
            FailedTaskInfo {
                task_id: task.task_id.clone(),
                last_error: task.last_error.clone(),
                files_changed: Vec::new(),
                episode_ids: task.episode_ids[recent..].to_vec(),
            }
        });
    let unfinished: HashSet<&str> = tasks
        .iter()
        .filter(|task| matches!(task.state, TaskState::Failed | TaskState::Incomplete))
        .map(|task| task.task_id.as_str())
        .collect();
    let gate_results: Vec<GateResultInfo> = records
        .gate_failures
        .iter()
        .filter(|record| unfinished.contains(record.task_id.as_str()))
        .map(|record| GateResultInfo {
            task_id: Some(record.task_id.clone()),
            gate_name: record.gate_name.clone(),
            rung: record.rung,
            passed: false,
            summary: record.summary.clone(),
            duration_ms: 0,
            classified_errors: classify_recorded_failure(record),
        })
        .collect();

    let ids_in = |state: TaskState| -> Vec<String> {
        tasks
            .iter()
            .filter(|task| task.state == state)
            .map(|task| task.task_id.clone())
            .collect()
    };
    let completed_tasks = ids_in(TaskState::Completed);
    let failed_tasks = ids_in(TaskState::Failed);
    let run_state = RunStateSummary {
        tasks_total: definition
            .as_ref()
            .map_or(tasks.len(), |definition| definition.tasks.tasks.len()),
        tasks_completed: completed_tasks.len(),
        tasks_failed: failed_tasks.len(),
        total_cost_usd: checkpoint.spent_micro_usd.map_or_else(
            || records.attempts.iter().map(|record| record.cost_usd).sum(),
            micro_to_usd,
        ),
        total_tokens_in: records
            .attempts
            .iter()
            .map(|record| record.input_tokens)
            .sum(),
        total_tokens_out: records
            .attempts
            .iter()
            .map(|record| record.output_tokens)
            .sum(),
        total_agent_calls: records.attempts.len(),
        completed_tasks,
        failed_tasks,
    };

    let git_state = collect_git_state(workdir, plan_id);
    let suggested_recovery =
        graph_recovery_suggestions(status, &tasks, resume.as_ref(), git_state.as_ref());

    DiagnoseReport {
        plan_id: plan_id.to_string(),
        status: match status {
            GraphCheckpointStatus::Succeeded => "completed".to_string(),
            other => other.as_str().to_string(),
        },
        source: ReportSource::GraphCheckpoint,
        phase: None,
        iteration: None,
        graph_run: Some(GraphRunInfo {
            run_id: checkpoint.manifest.run_id.clone(),
            graph_fingerprint: checkpoint.manifest.graph_fingerprint.clone(),
            checkpoint_status: status,
            updated_at: ms_to_rfc3339(checkpoint.manifest.updated_at_ms),
            checkpoint: relative_display(workdir, &checkpoint.paths.manifest),
            spent_usd: checkpoint.spent_micro_usd.map(micro_to_usd),
            reserved_usd: checkpoint.reserved_micro_usd.map(micro_to_usd),
            earlier_runs_before: checkpoint.replaced_at_ms.and_then(ms_to_rfc3339),
            invalidated_on_resume: checkpoint.invalidated_on_resume.clone(),
        }),
        plan_dir: definition
            .as_ref()
            .map(|definition| relative_display(workdir, &definition.dir)),
        failed_task,
        tasks,
        gate_results,
        run_state: Some(run_state),
        resume,
        git_state,
        suggested_recovery,
        notes,
        total_cost_usd: collect_total_cost_usd(workdir, plan_id),
    }
}

/// Find `plan_id`'s definition the way checkpoints name plans: by leaf
/// directory name under the workspace plans root, inside plan sets too.
fn load_plan_definition(
    workdir: &Path,
    plan_id: &str,
    notes: &mut Vec<String>,
) -> Option<PlanDefinition> {
    let Some(plan_dir) = crate::plan::plan_dirs_by_id(workdir).remove(plan_id) else {
        notes.push(format!(
            "plan '{plan_id}' was not found under {}: tasks are limited to those the run recorded, and there is no resume preview",
            relative_display(workdir, &crate::plan::plans_dir(workdir)),
        ));
        return None;
    };
    let tasks_path = plan_dir.dir.join("tasks.toml");
    match TasksFile::parse(&tasks_path) {
        Ok(tasks) => Some(PlanDefinition {
            dir: plan_dir.dir,
            tasks,
        }),
        Err(error) => {
            notes.push(format!(
                "cannot read {}: {error:#}; tasks are limited to those the run recorded",
                relative_display(workdir, &tasks_path)
            ));
            None
        }
    }
}

/// What `roko plan run <plan dir>` would do with the checkpoint.
fn resume_info(
    workdir: &Path,
    plan_id: &str,
    definition: &PlanDefinition,
    notes: &mut Vec<String>,
) -> Option<ResumeInfo> {
    let plan = Plan {
        id: plan_id.to_string(),
        dir: definition.dir.clone(),
        tasks: definition.tasks.clone(),
    };
    match preview_plan_resume(workdir, &plan, 1, &ResumeOptions::default()) {
        Ok(preview) => Some(ResumeInfo {
            command: format!(
                "roko plan run {}",
                relative_display(workdir, &definition.dir)
            ),
            preview,
        }),
        Err(error) => {
            notes.push(format!("no resume preview: {error:#}"));
            None
        }
    }
}

/// Every task of the plan, then the tasks the run recorded that the plan no
/// longer defines, as the run left them.
fn diagnose_tasks(
    checkpoint: &CheckpointInspection,
    definition: Option<&PlanDefinition>,
    records: &RunRecords,
    verbose: bool,
) -> Vec<TaskDiagnosis> {
    let plan_tasks: &[TaskDef] =
        definition.map_or(&[], |definition| definition.tasks.tasks.as_slice());
    let defined: BTreeMap<&str, &TaskDef> = plan_tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect();
    let mut ids: Vec<&str> = plan_tasks.iter().map(|task| task.id.as_str()).collect();
    let undefined: BTreeSet<&str> = checkpoint
        .recorded
        .keys()
        .map(String::as_str)
        .chain(records.task_ids())
        .filter(|task_id| !defined.contains_key(task_id))
        .collect();
    ids.extend(undefined);

    let attempted: HashSet<&str> = records.task_ids().collect();
    let states: BTreeMap<&str, TaskState> = ids
        .iter()
        .map(|&task_id| {
            let state = if checkpoint.recorded.contains_key(task_id) {
                TaskState::Completed
            } else if !attempted.contains(task_id) {
                TaskState::NeverRan
            } else if checkpoint.manifest.status == GraphCheckpointStatus::Failed {
                TaskState::Failed
            } else {
                TaskState::Incomplete
            };
            (task_id, state)
        })
        .collect();

    ids.iter()
        .map(|&task_id| {
            let task = defined.get(task_id).copied();
            let state = states[task_id];
            let attempts = task_attempts(records, task_id);
            let gate_failures: Vec<GateFailureInfo> = records
                .gate_failures
                .iter()
                .filter(|record| record.task_id == task_id)
                .map(|record| GateFailureInfo {
                    timestamp: record
                        .timestamp
                        .to_rfc3339_opts(SecondsFormat::Millis, true),
                    gate_name: record.gate_name.clone(),
                    verify_step: verify_step(&record.summary, task),
                    failure_kind: record.failure_kind.clone(),
                    primary_class: record.primary_class.clone(),
                    recommended_action: record.recommended_action.clone(),
                    summary: record.summary.clone(),
                })
                .collect();
            let episode_ids: Vec<String> = records
                .episodes
                .iter()
                .filter(|episode| episode.task_id == task_id)
                .map(episode_id)
                .collect();
            let tool_policies: Vec<AttemptToolPolicy> = records
                .tool_policies
                .iter()
                .filter(|(policy_task, _)| policy_task == task_id)
                .map(|(_, policy)| policy.clone())
                .collect();
            // Listed for every task that did not complete; a completed task's
            // history only with `--verbose`.
            let listed = verbose || state != TaskState::Completed;
            let mut diagnosis = TaskDiagnosis {
                task_id: task_id.to_string(),
                title: task.map(|task| task.title.clone()),
                state,
                reason: String::new(),
                depends_on: task.map(|task| task.depends_on.clone()).unwrap_or_default(),
                blocked_by: if state == TaskState::NeverRan {
                    blockers(task_id, &defined, &states)
                } else {
                    Vec::new()
                },
                gate_verdict: checkpoint.recorded.get(task_id).copied().flatten(),
                attempt_count: attempts.len(),
                failed_attempts: attempts.iter().filter(|attempt| !attempt.success).count(),
                timed_out_attempts: attempts.iter().filter(|attempt| attempt.timed_out).count(),
                last_error: last_error(records, task_id),
                attempts: if listed { attempts } else { Vec::new() },
                gate_failures: if listed { gate_failures } else { Vec::new() },
                episode_ids: if listed { episode_ids } else { Vec::new() },
                tool_policies: if listed { tool_policies } else { Vec::new() },
            };
            diagnosis.reason = describe_task(&diagnosis, checkpoint.manifest.status, &states);
            diagnosis
        })
        .collect()
}

/// `task_id`'s provider attempts, oldest first, each with the failure reason
/// of its episode. An attempt timed out when that reason says so, or when one
/// of its verify steps is recorded as a timeout: a step's authored `fail_msg`
/// keeps its timeout out of the reason.
fn task_attempts(records: &RunRecords, task_id: &str) -> Vec<AttemptInfo> {
    let mut episodes: Vec<&Episode> = records
        .episodes
        .iter()
        .filter(|episode| episode.task_id == task_id)
        .collect();
    let mut step_timeouts: Vec<i64> = records
        .gate_failures
        .iter()
        .filter(|record| {
            record.task_id == task_id && record.failure_kind == GateFailureKind::Timeout
        })
        .map(|record| record.timestamp.timestamp_millis())
        .collect();
    records
        .attempts
        .iter()
        .filter(|record| record.task_id == task_id)
        .map(|record| {
            let ended_ms = rfc3339_ms(&record.timestamp);
            let episode = ended_ms.and_then(|ended_ms| {
                let index = episodes.iter().position(|episode| {
                    episode.success == record.success
                        && (episode.timestamp.timestamp_millis() - ended_ms).abs()
                            <= EPISODE_MATCH_WINDOW_MS
                })?;
                Some(episodes.remove(index))
            });
            let step_timeout = ended_ms
                .filter(|_| !record.success)
                .and_then(|ended_ms| take_step_timeout(&mut step_timeouts, ended_ms));
            let failure_reason = episode
                .and_then(|episode| episode.failure_reason.clone())
                .filter(|reason| !reason.trim().is_empty());
            AttemptInfo {
                timestamp: record.timestamp.clone(),
                model: record.model.clone(),
                provider: record.provider.clone(),
                success: record.success,
                duration_ms: record.duration_ms,
                cost_usd: record.cost_usd,
                cost_source: record.cost_source,
                timed_out: step_timeout.is_some()
                    || failure_reason.as_deref().is_some_and(mentions_timeout),
                failure_reason,
                episode_id: episode.map(episode_id),
            }
        })
        .collect()
}

/// Take from `step_timeouts` the verify step timeout of the failed attempt
/// that ended at `ended_ms`: the first recorded within
/// [`EPISODE_MATCH_WINDOW_MS`] before it. Graph task dispatch records a
/// timed-out step just before its attempt ends.
fn take_step_timeout(step_timeouts: &mut Vec<i64>, ended_ms: i64) -> Option<i64> {
    let index = step_timeouts
        .iter()
        .position(|&at_ms| at_ms <= ended_ms && ended_ms - at_ms <= EPISODE_MATCH_WINDOW_MS)?;
    Some(step_timeouts.remove(index))
}

/// `task_id`'s most recent failure. A verify step failure is preferred over
/// the attempt's failure reason from the same moment: the reason only says
/// how many steps failed, the gate failure names the step.
fn last_error(records: &RunRecords, task_id: &str) -> Option<String> {
    let episode = records
        .episodes
        .iter()
        .filter(|episode| episode.task_id == task_id && !episode.success)
        .filter_map(|episode| {
            let reason = episode.failure_reason.as_deref()?.trim();
            (!reason.is_empty()).then(|| (episode.timestamp.timestamp_millis(), reason))
        })
        .next_back();
    let gate_failure = records
        .gate_failures
        .iter()
        .filter(|record| record.task_id == task_id)
        .map(|record| (record.timestamp.timestamp_millis(), record.summary.as_str()))
        .next_back();
    let (_, error) = match (episode, gate_failure) {
        (Some(episode), Some(gate_failure))
            if episode.0 > gate_failure.0 + EPISODE_MATCH_WINDOW_MS =>
        {
            episode
        }
        (_, Some(gate_failure)) => gate_failure,
        (Some(episode), None) => episode,
        (None, None) => return None,
    };
    Some(error.to_string())
}

/// The tasks that did not complete and that never-ran `task_id` waits on,
/// directly or through dependencies that never ran either.
fn blockers(
    task_id: &str,
    tasks: &BTreeMap<&str, &TaskDef>,
    states: &BTreeMap<&str, TaskState>,
) -> Vec<String> {
    let mut found = BTreeSet::new();
    let mut seen = HashSet::new();
    let mut pending = vec![task_id];
    while let Some(current) = pending.pop() {
        let Some(task) = tasks.get(current) else {
            continue;
        };
        for dependency in &task.depends_on {
            if !seen.insert(dependency.as_str()) {
                continue;
            }
            match states.get(dependency.as_str()) {
                Some(TaskState::Failed | TaskState::Incomplete) => {
                    found.insert(dependency.clone());
                }
                Some(TaskState::NeverRan) => pending.push(dependency),
                Some(TaskState::Completed) | None => {}
            }
        }
    }
    found.into_iter().collect()
}

/// One line saying why `task` is in its state.
fn describe_task(
    task: &TaskDiagnosis,
    status: GraphCheckpointStatus,
    states: &BTreeMap<&str, TaskState>,
) -> String {
    let timed_out = if task.timed_out_attempts > 0 {
        format!(" ({} timed out)", task.timed_out_attempts)
    } else {
        String::new()
    };
    let attempts = if task.attempt_count > 0 {
        format!(
            " after {} attempt{}{timed_out}",
            task.attempt_count,
            plural(task.attempt_count)
        )
    } else {
        String::new()
    };
    let last_error = task.last_error.as_deref().map_or_else(
        || "; no failure reason was recorded".to_string(),
        |error| format!("; last error: {error}"),
    );
    match task.state {
        TaskState::Completed => {
            let retries = if task.failed_attempts > 0 {
                format!(
                    " after {} failed attempt{}{timed_out}",
                    task.failed_attempts,
                    plural(task.failed_attempts)
                )
            } else {
                String::new()
            };
            let verdict = match task.gate_verdict {
                Some(TaskGateVerdict::Passed) => "; its verify steps passed",
                Some(TaskGateVerdict::PassedWithPreexistingFailures) => {
                    "; its verify steps passed, apart from tests that failed before the run"
                }
                Some(TaskGateVerdict::AlreadySatisfied) => {
                    "; it changed nothing, and its verify steps passed on the tree as it was"
                }
                Some(TaskGateVerdict::Unverified) => "; it has no verify steps",
                Some(TaskGateVerdict::ForcedAccept) => {
                    "; its verify steps failed and it was force-accepted, so a resume runs it again"
                }
                None => "",
            };
            format!("completed{retries}{verdict}")
        }
        TaskState::Failed => format!("failed{attempts}{last_error}"),
        TaskState::Incomplete => format!(
            "did not complete{attempts}, and the run is {}{last_error}",
            status.as_str()
        ),
        TaskState::NeverRan if !task.blocked_by.is_empty() => {
            let all_failed = task
                .blocked_by
                .iter()
                .all(|blocker| states.get(blocker.as_str()) == Some(&TaskState::Failed));
            format!(
                "never ran: {} {} {}",
                if task.blocked_by.len() == 1 {
                    "dependency"
                } else {
                    "dependencies"
                },
                task.blocked_by.join(", "),
                if all_failed {
                    "failed"
                } else {
                    "did not complete"
                }
            )
        }
        TaskState::NeverRan => match status {
            GraphCheckpointStatus::Running => "has not started".to_string(),
            GraphCheckpointStatus::Succeeded => {
                "never ran: the recorded run did not include it".to_string()
            }
            _ => "never ran: the run stopped before it started".to_string(),
        },
    }
}

/// The verify step named by a Graph verify failure summary, which starts
/// with `verify[<index>]` or `verify[<index>:<phase>]`.
fn verify_step(summary: &str, task: Option<&TaskDef>) -> Option<VerifyStepInfo> {
    static LABEL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^verify\[(\d+)(?::([^\]]*))?\]").expect("valid regex"));
    let captures = LABEL.captures(summary.trim_start())?;
    let index: usize = captures.get(1)?.as_str().parse().ok()?;
    Some(VerifyStepInfo {
        index,
        phase: captures
            .get(2)
            .map(|phase| phase.as_str().to_string())
            .filter(|phase| !phase.is_empty()),
        command: task
            .and_then(|task| task.verify.get(index))
            .map(|step| step.command.clone()),
    })
}

/// Classify a recorded Graph gate failure by the class the gate recorded.
fn classify_recorded_failure(record: &GateFailureRecord) -> Vec<ClassifiedError> {
    // The first line names the step and its failure message; the output that
    // follows may mention anything.
    let headline = record.summary.lines().next().unwrap_or_default();
    let class = match record.primary_class {
        FailureClass::SyntaxError
        | FailureClass::ImportError
        | FailureClass::TypeError
        | FailureClass::MissingDependencyOrFeature
        | FailureClass::BorrowOrLifetime => ErrorClass::CompileError,
        FailureClass::TestExpectationFailure => ErrorClass::TestFailure,
        _ if record.failure_kind == GateFailureKind::Timeout => ErrorClass::Timeout,
        _ if mentions_timeout(headline) => ErrorClass::Timeout,
        _ => ErrorClass::Unknown,
    };
    vec![ClassifiedError {
        error_class: class,
        file: None,
        line: None,
        error_summary: truncate(headline, 200),
        suggestion: suggestion_for(class),
    }]
}

/// Recovery steps for a Graph run.
fn graph_recovery_suggestions(
    status: GraphCheckpointStatus,
    tasks: &[TaskDiagnosis],
    resume: Option<&ResumeInfo>,
    git_state: Option<&GitStateInfo>,
) -> Vec<String> {
    let mut suggestions = Vec::new();
    let unfinished: Vec<&TaskDiagnosis> = tasks
        .iter()
        .filter(|task| matches!(task.state, TaskState::Failed | TaskState::Incomplete))
        .collect();
    for task in &unfinished {
        let task_id = &task.task_id;
        if task.timed_out_attempts > 0 {
            // A verify step that ran out of time has a limit of its own.
            let verify_step_timed_out = task
                .gate_failures
                .iter()
                .any(|failure| failure.failure_kind == GateFailureKind::Timeout);
            let limit = if verify_step_timed_out {
                "the task's `timeout_secs` or its verify step's `timeout_ms`,"
            } else {
                "the task's `timeout_secs`"
            };
            suggestions.push(format!(
                "{task_id}: {} attempt{} timed out; raise {limit} or split the task.",
                task.timed_out_attempts,
                plural(task.timed_out_attempts)
            ));
        }
        if let Some(failure) = task.gate_failures.last() {
            let step = failure.verify_step.as_ref().map_or_else(
                || "a verify step".to_string(),
                |step| format!("verify step {}", step.label()),
            );
            let class = enum_label(&failure.primary_class);
            match failure.recommended_action {
                GateFailureAction::Blocked => suggestions.push(format!(
                    "{task_id}: {step} is blocked by its environment ({class}); fix that before running the task again."
                )),
                GateFailureAction::NeedsReplan => suggestions.push(format!(
                    "{task_id}: {step} failed in a way the gate says needs a revised plan ({class})."
                )),
                GateFailureAction::NeedsHuman => suggestions.push(format!(
                    "{task_id}: {step} needs human input before the task runs again ({class})."
                )),
                GateFailureAction::Retry => {}
            }
            if let Some(command) = failure
                .verify_step
                .as_ref()
                .and_then(|step| step.command.as_deref())
            {
                suggestions.push(format!(
                    "{task_id}: reproduce the failing {step} with `{command}`"
                ));
            }
        } else if task.failed_attempts > 0 && task.last_error.is_none() {
            suggestions.push(format!(
                "{task_id}: no failure reason was recorded for its failed attempts; check the `roko plan run` output of this run."
            ));
        }
    }
    if status == GraphCheckpointStatus::Failed && unfinished.is_empty() {
        suggestions.push(
            "No task failure was recorded for this run, so the plan failed outside its tasks; check the `roko plan run` output."
                .to_string(),
        );
    }
    if status == GraphCheckpointStatus::Running {
        suggestions.push(
            "The checkpoint still says `running`. If no `roko plan run` is active for this plan, the run stopped without recording an outcome."
                .to_string(),
        );
    }
    if status != GraphCheckpointStatus::Succeeded
        && let Some(resume) = resume
    {
        suggestions.push(resume_suggestion(resume));
    }
    if git_state.is_some_and(|git| git.has_uncommitted_changes) {
        suggestions.push(
            "Uncommitted changes detected. Consider committing or stashing before retry."
                .to_string(),
        );
    }
    if suggestions.is_empty() && status == GraphCheckpointStatus::Succeeded {
        suggestions.push("Plan completed successfully. No recovery needed.".to_string());
    }
    suggestions
}

/// How re-running the plan continues from its checkpoint.
fn resume_suggestion(resume: &ResumeInfo) -> String {
    let command = &resume.command;
    let preview = &resume.preview;
    let tasks = |tasks: &[String]| {
        if tasks.is_empty() {
            "nothing".to_string()
        } else {
            tasks.join(", ")
        }
    };
    let reason = preview.reason.as_deref().unwrap_or("see `--dry-run`");
    match preview.action {
        ResumeAction::Resume => format!(
            "Resume with `{command}`: it restores {} and runs {}.",
            tasks(&preview.restored_tasks),
            tasks(&preview.tasks_to_run)
        ),
        ResumeAction::Refuse => format!("`{command}` cannot resume this checkpoint: {reason}."),
        ResumeAction::Archive => {
            format!("`{command}` archives this checkpoint and runs every task: {reason}.")
        }
        ResumeAction::Start => format!("Run the plan with `{command}`."),
    }
}

// ---------------------------------------------------------------------------
// Runner-v2 snapshots
// ---------------------------------------------------------------------------

/// Report from the Runner-v2 snapshot, for a plan without a Graph checkpoint.
fn build_legacy_report(
    workdir: &Path,
    plan_id: &str,
    projection: &DurableRunnerProjection,
) -> Result<DiagnoseReport> {
    // ── Extract executor plan states ────────────────────────────────────
    let plan_states = projection
        .executor
        .get("plan_states")
        .and_then(Value::as_object);

    let Some(plan_state) = plan_states.and_then(|states| states.get(plan_id)) else {
        let available: Vec<String> = plan_states
            .map(|states| states.keys().cloned().collect())
            .unwrap_or_default();
        bail!(
            "Plan '{}' not found in state snapshot. Available plans: [{}].{}",
            plan_id,
            available.join(", "),
            checkpointed_plans_hint(workdir)
        );
    };

    // ── Phase / status ──────────────────────────────────────────────────
    let phase = plan_state.get("current_phase").and_then(phase_name);
    // Pausing is a flag on the plan state, not a phase.
    let paused = plan_state
        .get("paused")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let iteration = plan_state
        .get("iteration")
        .and_then(Value::as_u64)
        .map(|v| v as u32);

    let last_error = plan_state
        .get("last_error")
        .and_then(Value::as_str)
        .map(String::from);

    let files_changed: Vec<String> = plan_state
        .get("files_changed")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let mut status = derive_status(phase.as_ref(), last_error.as_ref());
    if paused && matches!(status.as_str(), "running" | "gating" | "pending") {
        status = "paused".to_string();
    }

    // ── Gate results ────────────────────────────────────────────────────
    let gate_results: Vec<GateResultInfo> = plan_state
        .get("gate_results")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let gate_name = v.get("gate_name")?.as_str()?.to_string();
                    let passed = v.get("passed")?.as_bool()?;
                    let summary = v
                        .get("summary")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();

                    // Classify errors from gate output when the gate failed.
                    let output = v.get("output").and_then(Value::as_str).unwrap_or("");
                    let classified_errors = if !passed {
                        classify_gate_errors(&gate_name, &summary, output)
                    } else {
                        Vec::new()
                    };

                    Some(GateResultInfo {
                        task_id: None,
                        gate_name,
                        rung: v.get("rung")?.as_u64()? as u32,
                        passed,
                        summary,
                        duration_ms: v.get("duration_ms").and_then(Value::as_u64).unwrap_or(0),
                        classified_errors,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // ── Failed task info ────────────────────────────────────────────────
    let failed_task = if status == "failed" || last_error.is_some() {
        // Try to identify the failed task from run_state
        let failed_task_id = extract_failed_task_id(projection.run_state.as_ref(), plan_id);
        let task_id_str = failed_task_id.unwrap_or_else(|| "unknown".to_string());

        // Look up episode IDs from .roko/episodes.jsonl for the failed task.
        let episode_ids = collect_episode_ids(workdir, &task_id_str);

        Some(FailedTaskInfo {
            task_id: task_id_str,
            last_error,
            files_changed,
            episode_ids,
        })
    } else {
        None
    };

    // ── Run state summary ───────────────────────────────────────────────
    let run_state = projection.run_state.as_ref().map(|rs| {
        let completed: Vec<String> = rs
            .get("completed_tasks")
            .and_then(Value::as_object)
            .and_then(|m| m.get(plan_id))
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let failed: Vec<String> = rs
            .get("failed_tasks")
            .and_then(Value::as_object)
            .and_then(|m| m.get(plan_id))
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        RunStateSummary {
            tasks_total: rs.get("tasks_total").and_then(Value::as_u64).unwrap_or(0) as usize,
            tasks_completed: rs
                .get("tasks_completed")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            tasks_failed: rs.get("tasks_failed").and_then(Value::as_u64).unwrap_or(0) as usize,
            total_cost_usd: rs
                .get("total_cost_usd")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            total_tokens_in: rs
                .get("total_tokens_in")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            total_tokens_out: rs
                .get("total_tokens_out")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            total_agent_calls: rs
                .get("total_agent_calls")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            completed_tasks: completed,
            failed_tasks: failed,
        }
    });

    // ── Git state ───────────────────────────────────────────────────────
    let git_state = collect_git_state(workdir, plan_id);

    // ── Total cost from efficiency events ────────────────────────────────
    let total_cost_usd = collect_total_cost_usd(workdir, plan_id);

    // ── Recovery suggestions ────────────────────────────────────────────
    let suggested_recovery = build_recovery_suggestions(&status, &gate_results, git_state.as_ref());

    Ok(DiagnoseReport {
        plan_id: plan_id.to_string(),
        status,
        source: ReportSource::RunnerSnapshot,
        phase,
        iteration,
        graph_run: None,
        plan_dir: None,
        failed_task,
        tasks: Vec::new(),
        gate_results,
        run_state,
        resume: None,
        git_state,
        suggested_recovery,
        notes: Vec::new(),
        total_cost_usd,
    })
}

/// A Runner-v2 phase's name. `PlanState.current_phase` is a `PlanPhase`,
/// which serializes as an object tagged by `kind` (`{"kind": "implementing"}`);
/// older snapshots wrote a plain string.
fn phase_name(phase: &Value) -> Option<String> {
    phase
        .as_str()
        .or_else(|| phase.get("kind").and_then(Value::as_str))
        .map(String::from)
}

/// A Runner-v2 plan's status from its phase name (a kebab-case `PlanPhase`
/// kind, or an older snapshot's name), else from whether it recorded an error.
fn derive_status(phase: Option<&String>, last_error: Option<&String>) -> String {
    if let Some(phase) = phase {
        match phase.as_str() {
            "done" | "complete" | "merged" | "accepted" => "completed".to_string(),
            "failed" | "error" => "failed".to_string(),
            "gating" | "gate" | "verifying" | "regenerating-verify" => "gating".to_string(),
            "implementing" | "implement" | "agent" | "coding" => "running".to_string(),
            "enriching" | "reviewing" | "doc-revision" | "auto-fixing" => "running".to_string(),
            "merging" => "running".to_string(),
            "queued" | "pending" | "ready" => "pending".to_string(),
            "paused" => "paused".to_string(),
            "skipped" => "skipped".to_string(),
            _ => {
                if last_error.is_some() {
                    "failed".to_string()
                } else {
                    phase.clone()
                }
            }
        }
    } else if last_error.is_some() {
        "failed".to_string()
    } else {
        "unknown".to_string()
    }
}

fn extract_failed_task_id(run_state: Option<&Value>, plan_id: &str) -> Option<String> {
    let rs = run_state?;
    let failed = rs
        .get("failed_tasks")?
        .as_object()?
        .get(plan_id)?
        .as_array()?;
    failed.last()?.as_str().map(String::from)
}

fn collect_git_state(workdir: &Path, plan_id: &str) -> Option<GitStateInfo> {
    let current_branch = run_git(workdir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let has_uncommitted = run_git(workdir, &["status", "--porcelain"])
        .map(|out| !out.trim().is_empty())
        .unwrap_or(false);

    // Check if a branch matching the plan id exists
    let plan_branch_exists = run_git(workdir, &["branch", "--list", &format!("*{plan_id}*")])
        .map(|out| !out.trim().is_empty())
        .unwrap_or(false);

    Some(GitStateInfo {
        current_branch,
        has_uncommitted_changes: has_uncommitted,
        plan_branch_exists,
    })
}

fn run_git(workdir: &Path, args: &[&str]) -> Option<String> {
    ProcessCommand::new("git")
        .args(args)
        .current_dir(workdir)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

// ---------------------------------------------------------------------------
// Error classification
// ---------------------------------------------------------------------------

/// Classify errors from gate output using regex-based pattern matching.
///
/// Inspects the gate name, summary, and raw output to produce structured
/// `ClassifiedError` entries. When no raw output is available, falls back
/// to classifying the summary line itself.
fn classify_gate_errors(gate_name: &str, summary: &str, output: &str) -> Vec<ClassifiedError> {
    let mut errors = Vec::new();

    // If there is raw output, parse it line-by-line.
    if !output.is_empty() {
        errors.extend(classify_output_lines(output));
    }

    // If the raw output produced nothing, fall back to classifying the
    // summary + gate name combination.
    if errors.is_empty() && !summary.is_empty() {
        let class = infer_class_from_gate_name(gate_name, summary);
        errors.push(ClassifiedError {
            error_class: class,
            file: None,
            line: None,
            error_summary: truncate(summary, 200),
            suggestion: suggestion_for(class),
        });
    }

    errors
}

/// Walk raw gate output and extract classified errors from compiler/test
/// diagnostics.
fn classify_output_lines(output: &str) -> Vec<ClassifiedError> {
    // Regex: `error[E0308]: mismatched types` or `error: ...`
    let re_compile = Regex::new(r"^error(\[E\d+\])?: (.+)").expect("valid regex");
    // Regex: `warning: ...` (clippy / lint)
    let re_lint = Regex::new(r"^warning: (.+)").expect("valid regex");
    // Regex: `  --> path/to/file.rs:42:10`
    let re_location = Regex::new(r"^\s*--> (.+):(\d+):\d+").expect("valid regex");
    // Regex: `test foo::bar ... FAILED`
    let re_test = Regex::new(r"^test .+ \.\.\. FAILED").expect("valid regex");
    // Regex: `thread '...' panicked at '...'`
    let re_panic = Regex::new(r"thread '.+' panicked at").expect("valid regex");
    // Regex: linker error patterns. Whole words only, so `build` or `world`
    // is not read as `ld`.
    let re_link =
        Regex::new(r"(?i)\b(linker|ld|undefined (reference|symbol)|symbol\(s\) not found)\b")
            .expect("valid regex");

    let mut errors: Vec<ClassifiedError> = Vec::new();

    // Track the most recent file/line from `-->` markers so that
    // subsequent error lines can inherit the location.
    let mut pending_file: Option<String> = None;
    let mut pending_line: Option<u32> = None;

    for raw_line in output.lines() {
        let line = raw_line.trim();

        // Update pending location from `-->` lines.
        if let Some(caps) = re_location.captures(line) {
            pending_file = caps.get(1).map(|m| m.as_str().to_string());
            pending_line = caps.get(2).and_then(|m| m.as_str().parse().ok());
            continue;
        }

        if let Some(caps) = re_compile.captures(line) {
            let msg = caps.get(2).map_or("", |m| m.as_str());
            errors.push(ClassifiedError {
                error_class: ErrorClass::CompileError,
                file: pending_file.take(),
                line: pending_line.take(),
                error_summary: truncate(msg, 200),
                suggestion: suggestion_for(ErrorClass::CompileError),
            });
        } else if re_test.is_match(line) {
            errors.push(ClassifiedError {
                error_class: ErrorClass::TestFailure,
                file: None,
                line: None,
                error_summary: truncate(line, 200),
                suggestion: suggestion_for(ErrorClass::TestFailure),
            });
        } else if re_panic.is_match(line) {
            errors.push(ClassifiedError {
                error_class: ErrorClass::RuntimePanic,
                file: pending_file.take(),
                line: pending_line.take(),
                error_summary: truncate(line, 200),
                suggestion: suggestion_for(ErrorClass::RuntimePanic),
            });
        } else if re_link.is_match(line) {
            errors.push(ClassifiedError {
                error_class: ErrorClass::LinkError,
                file: None,
                line: None,
                error_summary: truncate(line, 200),
                suggestion: suggestion_for(ErrorClass::LinkError),
            });
        } else if let Some(caps) = re_lint.captures(line) {
            let msg = caps.get(1).map_or("", |m| m.as_str());
            errors.push(ClassifiedError {
                error_class: ErrorClass::LintWarning,
                file: pending_file.take(),
                line: pending_line.take(),
                error_summary: truncate(msg, 200),
                suggestion: suggestion_for(ErrorClass::LintWarning),
            });
        }
    }

    errors
}

/// Infer an error class from the gate name when no raw output is available.
fn infer_class_from_gate_name(gate_name: &str, summary: &str) -> ErrorClass {
    let combined = format!("{gate_name} {summary}").to_lowercase();
    if combined.contains("compile") || combined.contains("build") {
        ErrorClass::CompileError
    } else if combined.contains("test") {
        ErrorClass::TestFailure
    } else if combined.contains("clippy") || combined.contains("lint") {
        ErrorClass::LintWarning
    } else if combined.contains("link") {
        ErrorClass::LinkError
    } else if combined.contains("panic") {
        ErrorClass::RuntimePanic
    } else if combined.contains("timeout") || combined.contains("timed out") {
        ErrorClass::Timeout
    } else {
        ErrorClass::Unknown
    }
}

/// Return a human-friendly suggestion for the given error class.
fn suggestion_for(class: ErrorClass) -> Option<String> {
    Some(
        match class {
            ErrorClass::CompileError => {
                "Fix the type/syntax error and rerun `cargo build --workspace`."
            }
            ErrorClass::TestFailure => {
                "Inspect the failing test assertion and rerun `cargo test --workspace`."
            }
            ErrorClass::LintWarning => {
                "Address the clippy/lint issue and rerun `cargo clippy --workspace --no-deps -- -D warnings`."
            }
            ErrorClass::LinkError => {
                "Check for missing native libraries or duplicate symbol definitions."
            }
            ErrorClass::RuntimePanic => {
                "Examine the panic message and add appropriate error handling."
            }
            ErrorClass::Timeout => "Consider increasing timeout or optimizing the operation.",
            ErrorClass::Unknown => return None,
        }
        .to_string(),
    )
}

/// The first `max` characters of `s`, marked with `...` when cut.
fn truncate(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((cut, _)) => format!("{}...", &s[..cut]),
        None => s.to_string(),
    }
}

/// Whether a failure reason says the attempt or step ran out of time.
fn mentions_timeout(text: &str) -> bool {
    text.to_ascii_lowercase().contains("timed out")
}

/// The snake_case name a gate failure enum serializes to.
fn enum_label(value: &impl Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Log helpers
// ---------------------------------------------------------------------------

/// Every line of a JSONL file that parses as `T`. Blank and malformed lines
/// are skipped; a missing file has no records.
fn read_jsonl_lossy<T: DeserializeOwned>(path: &Path) -> Vec<T> {
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    std::io::BufReader::new(file)
        .split(b'\n')
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_slice(&line).ok())
        .collect()
}

/// Plan an episode belongs to; Graph task dispatch records it in `extra`.
fn episode_plan_id(episode: &Episode) -> Option<&str> {
    episode.extra.get("plan_id").and_then(Value::as_str)
}

/// An episode's id: `id`, else the deprecated `episode_id`.
fn episode_id(episode: &Episode) -> String {
    if episode.id.is_empty() {
        episode.episode_id.clone()
    } else {
        episode.id.clone()
    }
}

/// Unix milliseconds of an RFC 3339 timestamp.
fn rfc3339_ms(timestamp: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|at| at.timestamp_millis())
}

/// RFC 3339 UTC form of Unix milliseconds.
fn ms_to_rfc3339(ms: u128) -> Option<String> {
    DateTime::<Utc>::from_timestamp_millis(i64::try_from(ms).ok()?)
        .map(|at| at.to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn micro_to_usd(micro_usd: u64) -> f64 {
    micro_usd as f64 / 1_000_000.0
}

/// `path` relative to `workdir` when it lies inside it.
fn relative_display(workdir: &Path, path: &Path) -> String {
    path.strip_prefix(workdir)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

// ---------------------------------------------------------------------------
// Episode ID collection
// ---------------------------------------------------------------------------

/// Read `.roko/episodes.jsonl` and return up to 5 episode IDs whose
/// `task_id` matches the given failed task. Returns most-recent-last.
fn collect_episode_ids(workdir: &Path, task_id: &str) -> Vec<String> {
    let mut ids: Vec<String> =
        read_jsonl_lossy::<Value>(&RokoLayout::for_project(workdir).episodes_path())
            .iter()
            .filter(|episode| episode.get("task_id").and_then(Value::as_str) == Some(task_id))
            // Prefer `id`; fall back to deprecated `episode_id`.
            .filter_map(|episode| {
                episode
                    .get("id")
                    .or_else(|| episode.get("episode_id"))
                    .and_then(Value::as_str)
            })
            .filter(|id| !id.is_empty())
            .map(String::from)
            .collect();

    // Keep only the last 5 (most recent).
    ids.split_off(ids.len().saturating_sub(FAILED_TASK_EPISODE_IDS))
}

// ---------------------------------------------------------------------------
// Cost collection from efficiency events
// ---------------------------------------------------------------------------

/// Sum `cost_usd` from `.roko/learn/efficiency.jsonl` for entries whose
/// `plan_id` matches. Returns `None` when no matching records exist or
/// the file is absent.
fn collect_total_cost_usd(workdir: &Path, plan_id: &str) -> Option<f64> {
    let costs: Vec<f64> =
        read_jsonl_lossy::<Value>(&RokoLayout::for_project(workdir).efficiency_path())
            .iter()
            .filter(|event| event.get("plan_id").and_then(Value::as_str) == Some(plan_id))
            .filter_map(|event| event.get("cost_usd").and_then(Value::as_f64))
            .collect();
    if costs.is_empty() {
        None
    } else {
        Some(costs.iter().sum())
    }
}

// ---------------------------------------------------------------------------
// Text output
// ---------------------------------------------------------------------------

/// The report for a person: the status, then each task that did not complete
/// (every task with `verbose`) with why, the verify step it failed and its
/// attempts, then what to do next. `--json` prints the report itself.
pub fn render_text(report: &DiagnoseReport, verbose: bool) -> String {
    let mut out = String::new();
    let (plan_id, status) = (&report.plan_id, &report.status);
    let _ = writeln!(out, "Plan {plan_id}: {status}");
    if let Some(run) = &report.graph_run {
        let (run_id, checkpoint) = (&run.run_id, &run.checkpoint);
        let updated = run.updated_at.as_deref().unwrap_or("unknown");
        let _ = writeln!(out, "  run {run_id}");
        let _ = writeln!(out, "  checkpoint {checkpoint}, updated {updated}");
        if let Some(spent) = run.spent_usd {
            let _ = writeln!(out, "  spent ${spent:.2}");
        }
    } else {
        let phase = report.phase.as_deref().unwrap_or("unknown");
        let _ = writeln!(out, "  from the Runner-v2 snapshot, phase {phase}");
    }
    if let Some(plan_dir) = &report.plan_dir {
        let _ = writeln!(out, "  plan {plan_dir}");
    }
    if let Some(run_state) = &report.run_state {
        let (completed, total) = (run_state.tasks_completed, run_state.tasks_total);
        let failed = run_state.tasks_failed;
        let _ = writeln!(
            out,
            "  {completed} of {total} tasks completed, {failed} failed"
        );
    }

    let shown: Vec<&TaskDiagnosis> = report
        .tasks
        .iter()
        .filter(|task| verbose || task.state != TaskState::Completed)
        .collect();
    for task in &shown {
        out.push('\n');
        render_task(&mut out, task);
    }
    let hidden = report.tasks.len() - shown.len();
    if hidden > 0 {
        let suffix = plural(hidden);
        let _ = writeln!(
            out,
            "\n{hidden} completed task{suffix} not shown; --verbose lists them."
        );
    }

    // A Runner-v2 snapshot names the failed task and the plan's gate results.
    if report.tasks.is_empty() {
        if let Some(failed) = &report.failed_task {
            let task_id = &failed.task_id;
            let _ = writeln!(out, "\nfailed task {task_id}");
            if let Some(error) = &failed.last_error {
                let _ = writeln!(out, "    last error: {error}");
            }
        }
        for gate in report.gate_results.iter().filter(|gate| !gate.passed) {
            let (name, summary) = (&gate.gate_name, &gate.summary);
            let _ = writeln!(out, "    gate {name} failed: {summary}");
        }
    }

    for (heading, lines) in [
        ("Next steps", &report.suggested_recovery),
        ("Notes", &report.notes),
    ] {
        if !lines.is_empty() {
            let _ = writeln!(out, "\n{heading}:");
            for line in lines {
                let _ = writeln!(out, "  - {line}");
            }
        }
    }
    out
}

/// One task: its state and title, why it is in that state, the verify step it
/// failed last, and its attempts.
fn render_task(out: &mut String, task: &TaskDiagnosis) {
    let task_id = &task.task_id;
    let state = if task.blocked_by.is_empty() {
        enum_label(&task.state).replace('_', " ")
    } else {
        "blocked".to_string()
    };
    let title = task.title.as_deref().unwrap_or("(not in tasks.toml)");
    let reason = &task.reason;
    let _ = writeln!(out, "{task_id} [{state}] {title}");
    let _ = writeln!(out, "    {reason}");
    let step = task
        .gate_failures
        .last()
        .and_then(|failure| failure.verify_step.as_ref());
    if let Some(step) = step {
        let label = step.label();
        let command = step.command.as_deref().unwrap_or("(not in tasks.toml)");
        let _ = writeln!(out, "    failing verify step {label}: {command}");
    }
    for (index, attempt) in task.attempts.iter().enumerate() {
        let number = index + 1;
        let outcome = match (attempt.success, attempt.timed_out) {
            (true, _) => "succeeded",
            (false, true) => "timed out",
            (false, false) => "failed",
        };
        let seconds = attempt.duration_ms as f64 / 1000.0;
        let (cost, model) = (attempt.cost_usd, &attempt.model);
        let _ = write!(
            out,
            "    attempt {number}: {outcome} after {seconds:.1} s, ${cost:.2}, {model}"
        );
        if let Some(reason) = &attempt.failure_reason {
            let _ = write!(out, ": {reason}");
        }
        out.push('\n');
    }
    for policy in &task.tool_policies {
        render_tool_policy(out, policy);
    }
}

/// One attempt's tool policy: what its contract asked for, what its provider
/// enforced, and the operation it was stopped at.
fn render_tool_policy(out: &mut String, policy: &AttemptToolPolicy) {
    let (attempt, record) = (policy.attempt, &policy.policy);
    let model = policy.model.as_deref().unwrap_or("an unrecorded model");
    let enforcement = &record.enforcement;
    let _ = writeln!(
        out,
        "    attempt {attempt} on {model}: tool policy enforced by {enforcement}"
    );
    let mut asked = Vec::new();
    if let Some(allowed) = &record.allowed_tools {
        asked.push(format!("allow only [{}]", allowed.join(", ")));
    }
    if !record.forbidden_tools.is_empty() {
        asked.push(format!("forbid {}", record.forbidden_tools.join(", ")));
    }
    let mut enforced = Vec::new();
    if !record.denied_operations.is_empty() {
        enforced.push(format!("deny {}", record.denied_operations.join(", ")));
    }
    if record.network_off {
        enforced.push("network off".to_string());
    }
    for (label, parts) in [("asked", asked), ("enforced", enforced)] {
        if !parts.is_empty() {
            let _ = writeln!(out, "      {label}: {}", parts.join("; "));
        }
    }
    if let Some(denial) = &record.denial {
        let _ = writeln!(out, "      stopped at: {denial}");
    }
}

// ---------------------------------------------------------------------------
// Recovery suggestions
// ---------------------------------------------------------------------------

fn build_recovery_suggestions(
    status: &str,
    gate_results: &[GateResultInfo],
    git_state: Option<&GitStateInfo>,
) -> Vec<String> {
    let mut suggestions = Vec::new();

    if status == "failed" {
        let failed_gates: Vec<&GateResultInfo> =
            gate_results.iter().filter(|g| !g.passed).collect();

        if failed_gates.iter().any(|g| g.gate_name.contains("compile")) {
            suggestions.push(
                "Compile errors detected. Run `cargo build --workspace` to see full errors."
                    .to_string(),
            );
        }
        if failed_gates.iter().any(|g| g.gate_name.contains("test")) {
            suggestions.push(
                "Test failures detected. Run `cargo test --workspace` to see full output."
                    .to_string(),
            );
        }
        if failed_gates.iter().any(|g| g.gate_name.contains("clippy")) {
            suggestions.push(
                "Clippy warnings detected. Run `cargo clippy --workspace --no-deps -- -D warnings`."
                    .to_string(),
            );
        }

        if failed_gates.is_empty() {
            suggestions.push(
                "No gate failures recorded. Check the last_error field for agent-level failures."
                    .to_string(),
            );
        }

        suggestions.push("To retry: `roko plan run plans/ --resume-plan`".to_string());
    }

    if let Some(git) = git_state
        && git.has_uncommitted_changes
    {
        suggestions.push(
            "Uncommitted changes detected. Consider committing or stashing before retry."
                .to_string(),
        );
    }

    if status == "paused" {
        suggestions.push("Plan is paused. Resume with `roko plan run`.".to_string());
    }

    if suggestions.is_empty() && status == "completed" {
        suggestions.push("Plan completed successfully. No recovery needed.".to_string());
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_status_from_phases() {
        let done: String = "done".into();
        let failed: String = "failed".into();
        let implementing: String = "implementing".into();
        let queued: String = "queued".into();
        let paused: String = "paused".into();
        let err: String = "err".into();
        assert_eq!(derive_status(Some(&done), None), "completed");
        assert_eq!(derive_status(Some(&failed), None), "failed");
        assert_eq!(derive_status(Some(&implementing), None), "running");
        assert_eq!(derive_status(Some(&queued), None), "pending");
        assert_eq!(derive_status(Some(&paused), None), "paused");
        assert_eq!(derive_status(None, Some(&err)), "failed");
        assert_eq!(derive_status(None, None), "unknown");
    }

    #[test]
    fn derive_status_unknown_phase_with_error() {
        let custom: String = "custom".into();
        let oops: String = "oops".into();
        assert_eq!(derive_status(Some(&custom), Some(&oops)), "failed");
    }

    #[test]
    fn recovery_suggestions_compile_failure() {
        let gates = vec![GateResultInfo {
            task_id: None,
            gate_name: "compile:cargo".into(),
            rung: 1,
            passed: false,
            summary: "3 errors".into(),
            duration_ms: 5000,
            classified_errors: Vec::new(),
        }];
        let suggestions = build_recovery_suggestions("failed", &gates, None);
        assert!(suggestions.iter().any(|s| s.contains("cargo build")));
        assert!(suggestions.iter().any(|s| s.contains("resume-plan")));
    }

    #[test]
    fn recovery_suggestions_completed() {
        let suggestions = build_recovery_suggestions("completed", &[], None);
        assert!(suggestions.iter().any(|s| s.contains("successfully")));
    }

    #[test]
    fn recovery_suggestions_uncommitted_changes() {
        let git = GitStateInfo {
            current_branch: Some("main".into()),
            has_uncommitted_changes: true,
            plan_branch_exists: false,
        };
        let suggestions = build_recovery_suggestions("failed", &[], Some(&git));
        assert!(suggestions.iter().any(|s| s.contains("Uncommitted")));
    }

    // ── ClassifiedError tests ────────────────────────────────────────

    #[test]
    fn classify_compile_error_from_output() {
        let output = "\
  --> crates/roko-cli/src/main.rs:42:10
error[E0308]: mismatched types
   |
42 |     foo(bar)
   |         ^^^ expected `u32`, found `&str`";
        let errors = classify_gate_errors("compile:cargo", "", output);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].error_class, ErrorClass::CompileError);
        assert!(errors[0].error_summary.contains("mismatched types"));
        assert_eq!(
            errors[0].file.as_deref(),
            Some("crates/roko-cli/src/main.rs")
        );
        assert_eq!(errors[0].line, Some(42));
        assert!(errors[0].suggestion.is_some());
    }

    #[test]
    fn classify_test_failure_from_output() {
        let output = "test commands::diagnose::tests::my_test ... FAILED";
        let errors = classify_gate_errors("test:cargo", "", output);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_class, ErrorClass::TestFailure);
    }

    #[test]
    fn classify_panic_from_output() {
        let output = "thread 'main' panicked at 'index out of bounds'";
        let errors = classify_gate_errors("test:cargo", "", output);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_class, ErrorClass::RuntimePanic);
    }

    #[test]
    fn classify_lint_from_output() {
        let output = "\
  --> crates/roko-cli/src/foo.rs:10:9
warning: unused variable: `x`";
        let errors = classify_gate_errors("clippy:cargo", "", output);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].error_class, ErrorClass::LintWarning);
        assert_eq!(
            errors[0].file.as_deref(),
            Some("crates/roko-cli/src/foo.rs")
        );
        assert_eq!(errors[0].line, Some(10));
    }

    #[test]
    fn classify_link_error_from_output() {
        let output = "ld: symbol(s) not found for architecture arm64";
        let errors = classify_gate_errors("compile:cargo", "", output);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_class, ErrorClass::LinkError);
    }

    #[test]
    fn classify_words_containing_ld_are_not_link_errors() {
        let output = "cargo build of hello-world held the old build lock";
        assert!(classify_output_lines(output).is_empty());
    }

    #[test]
    fn classify_fallback_from_summary_when_no_output() {
        let errors = classify_gate_errors("compile:cargo", "build failed with 3 errors", "");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_class, ErrorClass::CompileError);
        assert!(errors[0].error_summary.contains("build failed"));
    }

    #[test]
    fn classify_unknown_gate() {
        let errors = classify_gate_errors("custom:gate", "something went wrong", "");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_class, ErrorClass::Unknown);
        assert!(errors[0].suggestion.is_none());
    }

    #[test]
    fn classify_empty_summary_and_output_produces_nothing() {
        let errors = classify_gate_errors("test:cargo", "", "");
        assert!(errors.is_empty());
    }

    #[test]
    fn classify_passed_gate_produces_nothing() {
        // classify_output_lines returns nothing for benign output.
        let errors = classify_output_lines("Compiling roko-cli v0.1.0\nFinished dev [unoptimized]");
        assert!(errors.is_empty());
    }

    #[test]
    fn truncate_cuts_on_character_boundaries() {
        assert_eq!(truncate("short", 200), "short");
        assert_eq!(truncate("✗✗✗✗", 2), "✗✗...");
        assert_eq!(truncate("abcd", 4), "abcd");
    }

    // ── Episode ID tests ─────────────────────────────────────────────

    #[test]
    fn collect_episode_ids_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let ids = collect_episode_ids(tmp.path(), "task-1");
        assert!(ids.is_empty());
    }

    #[test]
    fn collect_episode_ids_filters_by_task() {
        let tmp = tempfile::tempdir().unwrap();
        let roko = tmp.path().join(".roko");
        std::fs::create_dir_all(&roko).unwrap();
        let content = [
            r#"{"id":"ep-1","task_id":"task-1","agent_id":"a1"}"#,
            r#"{"id":"ep-2","task_id":"task-2","agent_id":"a2"}"#,
            r#"{"id":"ep-3","task_id":"task-1","agent_id":"a1"}"#,
        ]
        .join("\n");
        std::fs::write(roko.join("episodes.jsonl"), content).unwrap();

        let ids = collect_episode_ids(tmp.path(), "task-1");
        assert_eq!(ids, vec!["ep-1", "ep-3"]);
    }

    #[test]
    fn collect_episode_ids_caps_at_five() {
        let tmp = tempfile::tempdir().unwrap();
        let roko = tmp.path().join(".roko");
        std::fs::create_dir_all(&roko).unwrap();
        let lines: Vec<String> = (0..8)
            .map(|i| format!(r#"{{"id":"ep-{i}","task_id":"task-1","agent_id":"a"}}"#))
            .collect();
        std::fs::write(roko.join("episodes.jsonl"), lines.join("\n")).unwrap();

        let ids = collect_episode_ids(tmp.path(), "task-1");
        assert_eq!(ids.len(), 5);
        assert_eq!(ids, vec!["ep-3", "ep-4", "ep-5", "ep-6", "ep-7"]);
    }

    #[test]
    fn collect_episode_ids_falls_back_to_episode_id_field() {
        let tmp = tempfile::tempdir().unwrap();
        let roko = tmp.path().join(".roko");
        std::fs::create_dir_all(&roko).unwrap();
        let content = r#"{"episode_id":"legacy-1","task_id":"task-1","agent_id":"a1"}"#;
        std::fs::write(roko.join("episodes.jsonl"), content).unwrap();

        let ids = collect_episode_ids(tmp.path(), "task-1");
        assert_eq!(ids, vec!["legacy-1"]);
    }

    // ── Cost collection tests ────────────────────────────────────────

    #[test]
    fn collect_total_cost_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let cost = collect_total_cost_usd(tmp.path(), "plan-1");
        assert!(cost.is_none());
    }

    #[test]
    fn collect_total_cost_sums_matching_plan() {
        let tmp = tempfile::tempdir().unwrap();
        let learn = tmp.path().join(".roko").join("learn");
        std::fs::create_dir_all(&learn).unwrap();
        let content = [
            r#"{"plan_id":"plan-1","cost_usd":0.05,"task_id":"t1"}"#,
            r#"{"plan_id":"plan-2","cost_usd":0.10,"task_id":"t2"}"#,
            r#"{"plan_id":"plan-1","cost_usd":0.03,"task_id":"t3"}"#,
        ]
        .join("\n");
        std::fs::write(learn.join("efficiency.jsonl"), content).unwrap();

        let cost = collect_total_cost_usd(tmp.path(), "plan-1");
        assert!(cost.is_some());
        let total = cost.unwrap();
        assert!((total - 0.08).abs() < 1e-9);
    }

    #[test]
    fn collect_total_cost_none_when_no_matching_plan() {
        let tmp = tempfile::tempdir().unwrap();
        let learn = tmp.path().join(".roko").join("learn");
        std::fs::create_dir_all(&learn).unwrap();
        let content = r#"{"plan_id":"other","cost_usd":0.10,"task_id":"t1"}"#;
        std::fs::write(learn.join("efficiency.jsonl"), content).unwrap();

        let cost = collect_total_cost_usd(tmp.path(), "plan-1");
        assert!(cost.is_none());
    }

    #[test]
    fn collect_total_cost_tolerates_malformed_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let learn = tmp.path().join(".roko").join("learn");
        std::fs::create_dir_all(&learn).unwrap();
        let content = [
            r#"{"plan_id":"plan-1","cost_usd":0.02,"task_id":"t1"}"#,
            "this is not json",
            r#"{"plan_id":"plan-1","cost_usd":0.01,"task_id":"t2"}"#,
        ]
        .join("\n");
        std::fs::write(learn.join("efficiency.jsonl"), content).unwrap();

        let cost = collect_total_cost_usd(tmp.path(), "plan-1");
        assert!(cost.is_some());
        assert!((cost.unwrap() - 0.03).abs() < 1e-9);
    }

    // ── Serialization tests ──────────────────────────────────────────

    fn empty_report(status: &str) -> DiagnoseReport {
        DiagnoseReport {
            plan_id: "test-plan".into(),
            status: status.into(),
            source: ReportSource::RunnerSnapshot,
            phase: Some("done".into()),
            iteration: Some(1),
            graph_run: None,
            plan_dir: None,
            failed_task: None,
            tasks: Vec::new(),
            gate_results: vec![],
            run_state: None,
            resume: None,
            git_state: None,
            suggested_recovery: vec![],
            notes: Vec::new(),
            total_cost_usd: None,
        }
    }

    #[test]
    fn report_serializes_with_new_fields() {
        let report = DiagnoseReport {
            failed_task: Some(FailedTaskInfo {
                task_id: "task-1".into(),
                last_error: Some("compile error".into()),
                files_changed: vec!["src/main.rs".into()],
                episode_ids: vec!["ep-1".into(), "ep-2".into()],
            }),
            gate_results: vec![GateResultInfo {
                task_id: None,
                gate_name: "compile:cargo".into(),
                rung: 1,
                passed: false,
                summary: "3 errors".into(),
                duration_ms: 5000,
                classified_errors: vec![ClassifiedError {
                    error_class: ErrorClass::CompileError,
                    file: Some("src/main.rs".into()),
                    line: Some(42),
                    error_summary: "mismatched types".into(),
                    suggestion: Some("Fix the type error".into()),
                }],
            }],
            total_cost_usd: Some(0.42),
            ..empty_report("failed")
        };

        let json = serde_json::to_string_pretty(&report).expect("serialize");
        let parsed: Value = serde_json::from_str(&json).expect("parse");

        // Verify total_cost_usd at top level.
        assert_eq!(parsed["total_cost_usd"].as_f64(), Some(0.42));

        // Verify episode_ids on failed_task.
        let ep_ids = parsed["failed_task"]["episode_ids"]
            .as_array()
            .expect("episode_ids array");
        assert_eq!(ep_ids.len(), 2);
        assert_eq!(ep_ids[0].as_str(), Some("ep-1"));

        // Verify classified_errors on gate_results[0].
        let classified = parsed["gate_results"][0]["classified_errors"]
            .as_array()
            .expect("classified_errors array");
        assert_eq!(classified.len(), 1);
        assert_eq!(classified[0]["error_class"].as_str(), Some("compile_error"));
        assert_eq!(classified[0]["file"].as_str(), Some("src/main.rs"));
        assert_eq!(classified[0]["line"].as_u64(), Some(42));
        assert_eq!(parsed["source"].as_str(), Some("runner_snapshot"));
        assert!(parsed.get("tasks").is_none());
    }

    #[test]
    fn report_omits_null_total_cost() {
        let json = serde_json::to_string(&empty_report("completed")).expect("serialize");
        assert!(!json.contains("total_cost_usd"));
    }

    // ── Graph runs ───────────────────────────────────────────────────

    use crate::graph_checkpoint::start_plan_checkpoint;
    use roko_core::{Body, Kind, Signal};

    const PLAN_ID: &str = "demo";

    /// T2 has two verify steps; T3 waits on T2; T4 is independent.
    const PLAN_TOML: &str = r#"
[meta]
plan = "demo"

[[task]]
id = "T1"
title = "Write the parser"

[[task]]
id = "T2"
title = "Wire the parser"
depends_on = ["T1"]

[[task.verify]]
phase = "structural"
command = "test -f src/parser.rs"

[[task.verify]]
phase = "test"
command = "cargo test -p demo --lib parser -- --include-ignored --test-threads 1 a_filter_long_enough_that_the_recorded_summary_cuts_it_off"

[[task]]
id = "T3"
title = "Document the parser"
depends_on = ["T2"]

[[task]]
id = "T4"
title = "Tidy the changelog"
"#;

    /// A workspace with the plan inside a plan set, as `plans/<set>/<plan>`.
    fn workspace() -> tempfile::TempDir {
        let workspace = tempfile::tempdir().expect("tempdir");
        let plan_dir = workspace.path().join("plans/programme").join(PLAN_ID);
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(plan_dir.join("tasks.toml"), PLAN_TOML).expect("tasks.toml");
        std::fs::create_dir_all(workspace.path().join(".roko/learn")).expect("learn dir");
        workspace
    }

    fn passed_output() -> Vec<Signal> {
        let mut signals = vec![
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("done"))
                .build(),
        ];
        TaskGateVerdict::Passed.stamp(&mut signals);
        signals
    }

    /// Run the plan's checkpoint to `status` with `recorded` tasks completed
    /// and `spent_micro_usd` in its cost ledger.
    fn record_run(
        workdir: &Path,
        recorded: &[&str],
        spent_micro_usd: u64,
        status: GraphCheckpointStatus,
    ) {
        let dir = workdir.join("plans/programme").join(PLAN_ID);
        let plan = Plan {
            id: PLAN_ID.to_string(),
            tasks: TasksFile::parse(&dir.join("tasks.toml")).expect("parse tasks.toml"),
            dir,
        };
        let mut checkpoint = start_plan_checkpoint(workdir, &plan).expect("checkpoint");
        let mut recorder = checkpoint.take_recorder();
        for task_id in recorded {
            recorder
                .record(PLAN_ID, task_id, 0, passed_output())
                .expect("record");
        }
        checkpoint
            .take_cost_ledger()
            .persist(spent_micro_usd, 0)
            .expect("cost ledger");
        checkpoint.finish_with_status(status).expect("finish");
    }

    fn at(timestamp: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(timestamp)
            .expect("timestamp")
            .with_timezone(&Utc)
    }

    fn attempt(task_id: &str, timestamp: &str, success: bool, cost_usd: f64) -> CostRecord {
        CostRecord {
            timestamp: timestamp.to_string(),
            model: "claude-sonnet-4-6".into(),
            provider: "claude_cli".into(),
            role: "implementer".into(),
            plan_id: PLAN_ID.into(),
            task_id: task_id.into(),
            complexity_band: "focused".into(),
            input_tokens: 100,
            output_tokens: 50,
            cached_tokens: 0,
            cost_usd,
            duration_ms: 600_500,
            success,
            session_id: String::new(),
            cost_source: CostSource::CliUsage,
        }
    }

    fn episode(
        id: &str,
        plan_id: &str,
        task_id: &str,
        timestamp: &str,
        failure_reason: Option<&str>,
    ) -> Episode {
        let mut episode = Episode::new(task_id, task_id);
        episode.id = id.to_string();
        episode.timestamp = at(timestamp);
        episode.success = failure_reason.is_none();
        episode.failure_reason = failure_reason.map(str::to_string);
        episode
            .extra
            .insert("plan_id".into(), Value::String(plan_id.to_string()));
        episode
    }

    fn gate_failure(task_id: &str, timestamp: &str, summary: &str) -> GateFailureRecord {
        GateFailureRecord {
            plan_id: PLAN_ID.into(),
            task_id: task_id.into(),
            gate_name: "graph-verify".into(),
            rung: 0,
            failure_kind: GateFailureKind::Resource,
            primary_class: FailureClass::ExternalEnvironment,
            summary: summary.into(),
            recommended_action: GateFailureAction::Blocked,
            cargo_fix_candidate: false,
            replan_candidate: false,
            error_count: 1,
            warning_count: 0,
            timestamp: at(timestamp),
        }
    }

    fn write_jsonl<T: Serialize>(path: &Path, rows: &[T]) {
        let lines: Vec<String> = rows
            .iter()
            .map(|row| serde_json::to_string(row).expect("serialize row"))
            .collect();
        std::fs::write(path, lines.join("\n") + "\n").expect("write jsonl");
    }

    fn task<'a>(report: &'a DiagnoseReport, task_id: &str) -> &'a TaskDiagnosis {
        report
            .tasks
            .iter()
            .find(|task| task.task_id == task_id)
            .unwrap_or_else(|| panic!("task {task_id} in report"))
    }

    /// T1 completed; T2 timed out once, then failed its test step; T3 never
    /// ran behind T2; T4 never started. An earlier run's attempt and another
    /// plan's T2 episode must stay out of the report.
    fn failed_run() -> tempfile::TempDir {
        let workspace = workspace();
        let root = workspace.path();
        record_run(root, &["T1"], 1_250_000, GraphCheckpointStatus::Failed);
        // An earlier run of the plan was archived at 07:00.
        let archived_at = at("2026-09-29T07:00:00Z").timestamp_millis();
        std::fs::write(
            root.join(GRAPH_STATE_DIR)
                .join(PLAN_ID)
                .join(format!("checkpoint.json.bak.{archived_at}")),
            "{}",
        )
        .expect("archived checkpoint");

        let learn = root.join(".roko/learn");
        write_jsonl(
            &learn.join("costs.jsonl"),
            &[
                attempt("T2", "2026-09-28T09:00:00+00:00", false, 0.5),
                attempt("T1", "2026-09-29T07:10:00.000+00:00", true, 0.25),
                attempt("T2", "2026-09-29T07:20:00.100+00:00", false, 0.0),
                attempt("T2", "2026-09-29T07:30:00.100+00:00", false, 1.0),
            ],
        );
        write_jsonl(
            &learn.join("gate-failures.jsonl"),
            &[gate_failure(
                "T2",
                "2026-09-29T07:30:00.090Z",
                "verify[1:test] (`cargo test -p demo --lib parser -- --include-ignored --test-threads 1 a_filter_long",
            )],
        );
        write_jsonl(
            &root.join(".roko/episodes.jsonl"),
            &[
                episode("ep-t1", PLAN_ID, "T1", "2026-09-29T07:10:00.004Z", None),
                episode(
                    "ep-t2-a",
                    PLAN_ID,
                    "T2",
                    "2026-09-29T07:20:00.095Z",
                    Some("provider: timed out after 600000 ms"),
                ),
                episode(
                    "ep-t2-b",
                    PLAN_ID,
                    "T2",
                    "2026-09-29T07:30:00.095Z",
                    Some("verify: gate error (graph-verify): 1/2 verify step(s) failed"),
                ),
                episode(
                    "ep-other-plan",
                    "other-plan",
                    "T2",
                    "2026-09-29T07:31:00Z",
                    Some("provider: other plan"),
                ),
            ],
        );
        workspace
    }

    #[test]
    fn graph_report_explains_a_failed_run() {
        let workspace = failed_run();
        let report = build_report(workspace.path(), PLAN_ID, false).expect("report");

        assert_eq!(report.source, ReportSource::GraphCheckpoint);
        assert_eq!(report.status, "failed");
        assert_eq!(report.plan_dir.as_deref(), Some("plans/programme/demo"));
        let graph_run = report.graph_run.as_ref().expect("graph run");
        assert_eq!(graph_run.checkpoint_status, GraphCheckpointStatus::Failed);
        assert!(graph_run.run_id.starts_with("graph-demo-"));
        assert_eq!(graph_run.spent_usd, Some(1.25));
        assert_eq!(
            graph_run.earlier_runs_before.as_deref(),
            Some("2026-09-29T07:00:00.000Z")
        );

        let states: Vec<(&str, TaskState)> = report
            .tasks
            .iter()
            .map(|task| (task.task_id.as_str(), task.state))
            .collect();
        assert_eq!(
            states,
            [
                ("T1", TaskState::Completed),
                ("T2", TaskState::Failed),
                ("T3", TaskState::NeverRan),
                ("T4", TaskState::NeverRan),
            ]
        );

        let t1 = task(&report, "T1");
        assert_eq!(t1.gate_verdict, Some(TaskGateVerdict::Passed));
        assert_eq!(t1.attempt_count, 1);
        assert!(t1.attempts.is_empty(), "completed history needs --verbose");

        // The attempt from before the archive is left out.
        let t2 = task(&report, "T2");
        assert_eq!(t2.attempt_count, 2);
        assert_eq!(t2.failed_attempts, 2);
        assert_eq!(t2.timed_out_attempts, 1);
        assert!(t2.attempts[0].timed_out);
        assert_eq!(t2.attempts[0].episode_id.as_deref(), Some("ep-t2-a"));
        assert_eq!(
            t2.attempts[1].failure_reason.as_deref(),
            Some("verify: gate error (graph-verify): 1/2 verify step(s) failed")
        );
        assert_eq!(t2.episode_ids, ["ep-t2-a", "ep-t2-b"]);
        let step = t2.gate_failures[0].verify_step.as_ref().expect("step");
        assert_eq!(step.index, 1);
        assert_eq!(step.phase.as_deref(), Some("test"));
        assert!(
            step.command
                .as_deref()
                .is_some_and(|command| command.ends_with("cuts_it_off")),
            "the full command comes from tasks.toml"
        );
        assert!(
            t2.last_error
                .as_deref()
                .is_some_and(|error| error.starts_with("verify[1:test]"))
        );
        assert!(
            t2.reason
                .starts_with("failed after 2 attempts (1 timed out)")
        );

        let t3 = task(&report, "T3");
        assert_eq!(t3.blocked_by, ["T2"]);
        assert_eq!(t3.reason, "never ran: dependency T2 failed");
        let t4 = task(&report, "T4");
        assert!(t4.blocked_by.is_empty());
        assert_eq!(t4.reason, "never ran: the run stopped before it started");

        let failed = report.failed_task.as_ref().expect("failed task");
        assert_eq!(failed.task_id, "T2");
        assert_eq!(report.gate_results.len(), 1);
        assert_eq!(report.gate_results[0].task_id.as_deref(), Some("T2"));

        let run_state = report.run_state.as_ref().expect("run state");
        assert_eq!(run_state.tasks_total, 4);
        assert_eq!(run_state.completed_tasks, ["T1"]);
        assert_eq!(run_state.failed_tasks, ["T2"]);
        assert_eq!(run_state.total_agent_calls, 3);
        assert!((run_state.total_cost_usd - 1.25).abs() < 1e-9);

        let resume = report.resume.as_ref().expect("resume preview");
        assert_eq!(resume.command, "roko plan run plans/programme/demo");
        assert_eq!(resume.preview.action, ResumeAction::Resume);
        assert_eq!(resume.preview.restored_tasks, ["T1"]);
        assert_eq!(resume.preview.tasks_to_run, ["T2", "T3", "T4"]);

        let suggestions = report.suggested_recovery.join("\n");
        assert!(suggestions.contains("T2: 1 attempt timed out"));
        assert!(suggestions.contains("blocked by its environment (external_environment)"));
        assert!(suggestions.contains("cuts_it_off`"));
        assert!(suggestions.contains(
            "Resume with `roko plan run plans/programme/demo`: it restores T1 and runs T2, T3, T4."
        ));
    }

    #[test]
    fn graph_report_serializes_tasks_and_resume() {
        let workspace = failed_run();
        let report = build_report(workspace.path(), PLAN_ID, false).expect("report");
        let json = serde_json::to_value(&report).expect("serialize");

        assert_eq!(json["source"], "graph_checkpoint");
        assert_eq!(json["graph_run"]["checkpoint_status"], "failed");
        assert_eq!(json["tasks"][1]["state"], "failed");
        assert_eq!(json["tasks"][2]["state"], "never_ran");
        assert_eq!(
            json["tasks"][1]["gate_failures"][0]["primary_class"],
            "external_environment"
        );
        assert_eq!(json["resume"]["action"], "resume");
        assert_eq!(json["resume"]["tasks_to_run"][0], "T2");
        assert!(json["tasks"][0].get("attempts").is_none());
    }

    #[test]
    fn a_graph_report_renders_as_readable_text() {
        let workspace = failed_run();
        let report = build_report(workspace.path(), PLAN_ID, false).expect("report");
        let text = render_text(&report, false);

        assert!(text.starts_with("Plan demo: failed\n"), "{text}");
        assert!(serde_json::from_str::<Value>(&text).is_err(), "{text}");
        assert!(text.contains("T2 [failed] Wire the parser"), "{text}");
        assert!(text.contains("last error: verify[1:test]"), "{text}");
        assert!(text.contains("verify step 1 (test): cargo test"), "{text}");
        let second_attempt = "attempt 2: failed after 600.5 s, $1.00";
        assert!(text.contains(second_attempt), "{text}");
        assert!(text.contains("T3 [blocked] Document the parser"), "{text}");
        assert!(text.contains("never ran: dependency T2 failed"), "{text}");
        let resume = "Resume with `roko plan run plans/programme/demo`";
        assert!(text.contains(resume), "{text}");
        assert!(!text.contains("T1 [completed]"), "{text}");
        assert!(text.contains("1 completed task not shown"), "{text}");

        let verbose = render_text(&report, true);
        let completed = "T1 [completed] Write the parser";
        assert!(verbose.contains(completed), "{verbose}");
    }

    /// gap-baab0a: an attempt's tool policy, recorded in its verdict in the
    /// run's `attempts.jsonl`, is listed with its task, with the operation
    /// the broker stopped it at. A verdict without one, and another plan's,
    /// add nothing.
    #[test]
    fn graph_report_lists_the_tool_policy_of_each_attempt() {
        use roko_learn::telemetry::{
            AttemptIdentity, AttemptKey, AttemptOutcome, AttemptVerdictRecord, TelemetryWriter,
            TelemetryWriterConfig,
        };

        let workspace = failed_run();
        let root = workspace.path();
        let run_id = inspect_canonical_checkpoint(root, PLAN_ID)
            .expect("read checkpoint")
            .expect("checkpoint")
            .manifest
            .run_id;
        let policy = ToolPolicyRecord {
            allowed_tools: None,
            forbidden_tools: vec!["web_fetch".into(), "web_search".into()],
            enforcement: "broker".into(),
            denied_operations: vec!["web_search".into()],
            network_off: true,
            denial: Some("web_search denied by policy: parser docs".into()),
        };
        let verdict = |plan: &str, attempt: u32, policy: Option<ToolPolicyRecord>| {
            let key = AttemptKey::new(run_id.as_str(), plan, "T2", attempt);
            let mut verdict = AttemptVerdictRecord::settle(
                AttemptIdentity::new(&key),
                AttemptOutcome::ProviderError,
                true,
            );
            verdict.executed.model_dispatched = Some("gpt-5-codex".to_string());
            verdict.executed.tool_policy = policy;
            verdict
        };
        let run_dir = RokoLayout::for_project(root).run_dir(&run_id);
        let writer = TelemetryWriter::spawn(&run_dir, TelemetryWriterConfig::default())
            .expect("spawn writer");
        assert!(writer.submit(verdict(PLAN_ID, 1, None)));
        assert!(writer.submit(verdict(PLAN_ID, 2, Some(policy.clone()))));
        assert!(writer.submit(verdict("other-plan", 1, Some(policy.clone()))));
        assert_eq!(writer.close().written, 3);

        let report = build_report(root, PLAN_ID, false).expect("report");
        let t2 = task(&report, "T2");
        assert_eq!(t2.tool_policies.len(), 1, "{:?}", t2.tool_policies);
        assert_eq!(t2.tool_policies[0].attempt, 2);
        assert_eq!(t2.tool_policies[0].model.as_deref(), Some("gpt-5-codex"));
        assert_eq!(t2.tool_policies[0].policy, policy);
        assert!(task(&report, "T1").tool_policies.is_empty());

        let json = serde_json::to_value(&report).expect("serialize");
        let recorded = &json["tasks"][1]["tool_policies"][0];
        assert_eq!(recorded["attempt"], 2);
        assert_eq!(recorded["enforcement"], "broker");
        assert_eq!(
            recorded["denial"],
            "web_search denied by policy: parser docs"
        );

        let text = render_text(&report, false);
        for line in [
            "attempt 2 on gpt-5-codex: tool policy enforced by broker",
            "asked: forbid web_fetch, web_search",
            "enforced: deny web_search; network off",
            "stopped at: web_search denied by policy: parser docs",
        ] {
            assert!(text.contains(line), "{text}");
        }
    }

    #[test]
    fn graph_report_for_a_succeeded_run_counts_retries() {
        let workspace = workspace();
        let root = workspace.path();
        record_run(
            root,
            &["T1", "T2", "T3", "T4"],
            400_000,
            GraphCheckpointStatus::Succeeded,
        );
        write_jsonl(
            &root.join(".roko/learn/costs.jsonl"),
            &[
                attempt("T2", "2026-09-29T07:20:00+00:00", false, 0.0),
                attempt("T2", "2026-09-29T07:40:00+00:00", true, 0.4),
            ],
        );
        write_jsonl(
            &root.join(".roko/episodes.jsonl"),
            &[episode(
                "ep-1",
                PLAN_ID,
                "T2",
                "2026-09-29T07:20:00.010Z",
                Some("provider: timed out after 600000 ms"),
            )],
        );

        let report = build_report(root, PLAN_ID, false).expect("report");
        assert_eq!(report.status, "completed");
        assert!(report.failed_task.is_none());
        assert!(
            report
                .tasks
                .iter()
                .all(|task| task.state == TaskState::Completed)
        );
        let t2 = task(&report, "T2");
        assert_eq!(
            (t2.attempt_count, t2.failed_attempts, t2.timed_out_attempts),
            (2, 1, 1)
        );
        assert!(t2.attempts.is_empty());
        assert_eq!(
            t2.reason,
            "completed after 1 failed attempt (1 timed out); its verify steps passed"
        );
        assert_eq!(
            report.suggested_recovery,
            ["Plan completed successfully. No recovery needed."]
        );

        let verbose = build_report(root, PLAN_ID, true).expect("verbose report");
        let t2 = task(&verbose, "T2");
        assert_eq!(t2.attempts.len(), 2);
        assert_eq!(t2.episode_ids, ["ep-1"]);
    }

    /// A verify step that ran out of time leaves an attempt reason that only
    /// counts failed steps, since its authored `fail_msg` stands in for "timed
    /// out"; its gate-failure record is what says it timed out. The dispatch
    /// side has a test of the same name.
    #[test]
    fn a_verify_step_timeout_is_recorded_as_a_timeout() {
        let workspace = workspace();
        let root = workspace.path();
        record_run(root, &["T1"], 100_000, GraphCheckpointStatus::Failed);
        let learn = root.join(".roko/learn");
        write_jsonl(
            &learn.join("costs.jsonl"),
            &[
                attempt("T2", "2026-09-29T07:20:00.100+00:00", false, 0.05),
                attempt("T2", "2026-09-29T07:30:00.100+00:00", false, 0.05),
            ],
        );
        let summary = "verify[0:structural] (`test -f src/parser.rs`): the check failed";
        let mut timed_out = gate_failure("T2", "2026-09-29T07:30:00.090Z", summary);
        timed_out.failure_kind = GateFailureKind::Timeout;
        timed_out.primary_class = FailureClass::Unknown;
        timed_out.recommended_action = GateFailureAction::Retry;
        write_jsonl(&learn.join("gate-failures.jsonl"), &[timed_out]);
        let reason = format!("verify: 1/2 verify step(s) failed:\n\n{summary}");
        write_jsonl(
            &root.join(".roko/episodes.jsonl"),
            &[
                episode(
                    "ep-1",
                    PLAN_ID,
                    "T2",
                    "2026-09-29T07:20:00.095Z",
                    Some("provider: exit 1"),
                ),
                episode(
                    "ep-2",
                    PLAN_ID,
                    "T2",
                    "2026-09-29T07:30:00.095Z",
                    Some(reason.as_str()),
                ),
            ],
        );

        let report = build_report(root, PLAN_ID, false).expect("report");
        let t2 = task(&report, "T2");
        // Only the second attempt ended after the timed-out step.
        assert_eq!((t2.failed_attempts, t2.timed_out_attempts), (2, 1));
        assert!(!t2.attempts[0].timed_out);
        assert!(t2.attempts[1].timed_out);
        assert_eq!(t2.gate_failures[0].failure_kind, GateFailureKind::Timeout);
        assert_eq!(
            t2.reason,
            format!("failed after 2 attempts (1 timed out); last error: {summary}")
        );
        assert_eq!(
            report.gate_results[0].classified_errors[0].error_class,
            ErrorClass::Timeout
        );
        let suggestions = report.suggested_recovery.join("\n");
        assert!(suggestions.contains("T2: 1 attempt timed out"));
        assert!(
            suggestions.contains("or its verify step's `timeout_ms`, or split the task."),
            "{suggestions}"
        );
    }

    #[test]
    fn graph_report_without_the_plan_definition_uses_recorded_tasks() {
        let workspace = failed_run();
        let root = workspace.path();
        std::fs::remove_dir_all(root.join("plans")).expect("remove plans");

        let report = build_report(root, PLAN_ID, false).expect("report");
        assert!(report.plan_dir.is_none());
        assert!(report.resume.is_none());
        assert!(
            report
                .notes
                .iter()
                .any(|note| note.contains("was not found"))
        );
        let states: Vec<(&str, TaskState)> = report
            .tasks
            .iter()
            .map(|task| (task.task_id.as_str(), task.state))
            .collect();
        assert_eq!(
            states,
            [("T1", TaskState::Completed), ("T2", TaskState::Failed)]
        );
        assert!(task(&report, "T2").title.is_none());
    }

    #[test]
    fn graph_report_of_an_interrupted_run_marks_attempted_tasks_incomplete() {
        let workspace = workspace();
        let root = workspace.path();
        record_run(root, &["T1"], 0, GraphCheckpointStatus::Interrupted);
        write_jsonl(
            &root.join(".roko/learn/costs.jsonl"),
            &[attempt("T2", "2026-09-29T07:20:00+00:00", false, 0.1)],
        );

        let report = build_report(root, PLAN_ID, false).expect("report");
        assert_eq!(report.status, "interrupted");
        assert_eq!(task(&report, "T2").state, TaskState::Incomplete);
        assert_eq!(
            task(&report, "T3").reason,
            "never ran: dependency T2 did not complete"
        );
        assert!(report.failed_task.is_none());
    }

    #[test]
    fn verify_step_reads_the_label_and_the_full_command() {
        let tasks = TasksFile::parse_str(PLAN_TOML).expect("parse");
        let t2 = tasks.tasks.iter().find(|task| task.id == "T2");
        let step = verify_step(
            "verify[0:structural] (`test -f src/parser.rs`): missing",
            t2,
        )
        .expect("step");
        assert_eq!(step.index, 0);
        assert_eq!(step.phase.as_deref(), Some("structural"));
        assert_eq!(step.command.as_deref(), Some("test -f src/parser.rs"));
        let bare = verify_step("verify[3] (`true`)", t2).expect("bare label");
        assert_eq!((bare.index, bare.phase, bare.command), (3, None, None));
        assert!(verify_step("gate error: something else", t2).is_none());
    }

    // ── Fallbacks ────────────────────────────────────────────────────

    #[test]
    fn a_plan_without_a_graph_checkpoint_uses_the_runner_snapshot() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let state = workspace.path().join(".roko/state");
        std::fs::create_dir_all(&state).expect("state dir");
        let executor = serde_json::json!({
            "schema_version": 1,
            "plan_states": {
                "legacy-plan": {
                    "plan_id": "legacy-plan",
                    "current_phase": { "kind": "implementing" },
                    "iteration": 2,
                    "last_error": "gate compile:cargo failed",
                    "files_changed": ["src/lib.rs"],
                    "gate_results": [{
                        "gate_name": "compile:cargo",
                        "rung": 1,
                        "passed": false,
                        "summary": "3 errors",
                        "duration_ms": 1200
                    }]
                }
            }
        });
        std::fs::write(
            state.join("executor.json"),
            serde_json::to_vec(&executor).expect("serialize"),
        )
        .expect("executor.json");

        let report = build_report(workspace.path(), "legacy-plan", false).expect("report");
        assert_eq!(report.source, ReportSource::RunnerSnapshot);
        // The phase, not the last error, says the plan is still being worked on.
        assert_eq!(report.phase.as_deref(), Some("implementing"));
        assert_eq!(report.status, "running");
        assert_eq!(report.iteration, Some(2));
        let failed = report.failed_task.as_ref().expect("failed task");
        assert_eq!(
            failed.last_error.as_deref(),
            Some("gate compile:cargo failed")
        );
        assert_eq!(failed.files_changed, ["src/lib.rs"]);
        assert_eq!(report.gate_results[0].gate_name, "compile:cargo");
        assert!(report.graph_run.is_none());
        assert!(report.tasks.is_empty());
    }

    /// The report for `legacy-plan` from a Runner-v2 executor snapshot whose
    /// state for the plan is `plan_state`.
    fn runner_snapshot_report(plan_state: Value) -> DiagnoseReport {
        let workspace = tempfile::tempdir().expect("tempdir");
        let state = workspace.path().join(".roko/state");
        std::fs::create_dir_all(&state).expect("state dir");
        let executor = serde_json::json!({
            "schema_version": 1,
            "plan_states": { "legacy-plan": plan_state }
        });
        std::fs::write(
            state.join("executor.json"),
            serde_json::to_vec(&executor).expect("serialize"),
        )
        .expect("executor.json");
        build_report(workspace.path(), "legacy-plan", false).expect("report")
    }

    #[test]
    fn a_runner_snapshot_reports_its_phase() {
        let running = serde_json::json!({
            "plan_id": "legacy-plan",
            "current_phase": { "kind": "implementing" },
            "iteration": 1,
            "paused": false
        });
        let report = runner_snapshot_report(running);
        assert_eq!(report.phase.as_deref(), Some("implementing"));
        assert_eq!(report.status, "running");
        assert!(report.failed_task.is_none());

        let paused = serde_json::json!({
            "plan_id": "legacy-plan",
            "current_phase": { "kind": "auto-fixing" },
            "paused": true
        });
        let report = runner_snapshot_report(paused);
        assert_eq!(report.phase.as_deref(), Some("auto-fixing"));
        assert_eq!(report.status, "paused");
        let suggestions = report.suggested_recovery.join("\n");
        assert!(suggestions.contains("Plan is paused"), "{suggestions}");

        let failed = serde_json::json!({
            "plan_id": "legacy-plan",
            "current_phase": { "kind": "failed", "reason": "AllTasksFailed" }
        });
        let report = runner_snapshot_report(failed);
        assert_eq!(report.phase.as_deref(), Some("failed"));
        assert_eq!(report.status, "failed");

        // Older snapshots wrote the phase as a plain name.
        let complete = phase_name(&serde_json::json!("complete"));
        assert_eq!(complete.as_deref(), Some("complete"));
        assert_eq!(derive_status(complete.as_ref(), None), "completed");
    }

    #[test]
    fn missing_run_state_names_both_locations_and_the_checkpointed_plans() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let other = workspace.path().join(GRAPH_STATE_DIR).join("other-plan");
        std::fs::create_dir_all(&other).expect("checkpoint dir");
        std::fs::write(other.join("checkpoint.json"), "{}").expect("checkpoint");

        let error = build_report(workspace.path(), "nonexistent", false)
            .expect_err("no run state")
            .to_string();
        assert!(
            error.contains("No run state for plan 'nonexistent'"),
            "{error}"
        );
        assert!(error.contains(".roko/state/graph"), "{error}");
        assert!(error.contains("state-snapshot.json"), "{error}");
        assert!(
            error.contains("Plans with a Graph checkpoint: other-plan."),
            "{error}"
        );
    }
}
