//! The run view (gap-2c87f0): what `roko run` and `roko plan run` show while a
//! plan runs, and the summary a run ends with, in roko's design system
//! (`tmp/hermes-roko-v8/design/roko-design-system.md`, §5.3).
//!
//! On a terminal the view prints a header that stays, then redraws one line
//! per task in place (only the lines that changed) over a footer of glass
//! chips: tasks done, elapsed time, spend. In a pipe, in CI or on a dumb
//! terminal it appends one line per event instead, with the same words in the
//! same order. It reads the run's dashboard events, the ones the TUI reads.
//!
//! The view writes to stderr; the summary ([`summary_lines`]) goes to stdout,
//! so `--json` and scripts that read stdout are unaffected.

use std::collections::BTreeMap;
use std::io::Write;
use std::time::{Duration, Instant};

use roko_core::DashboardEvent;

use crate::term_style::{self, Glyph, Role, Style};

/// What the view knows before the run starts.
#[derive(Clone, Debug, Default)]
pub struct RunHeader {
    /// What runs, as the person named it: a plan's name, or their request.
    pub subject: String,
    /// The plans, in the order they run, with their tasks.
    pub plans: Vec<PlanTasks>,
    /// How many tasks of a plan may run at once.
    pub parallel: usize,
    /// The model the tasks start on, when they all start on one.
    pub model: Option<String>,
    /// The spend cap, when one is set.
    pub cap_usd: Option<f64>,
    /// One more fact for the faint line: `on roko serve`.
    pub note: Option<String>,
    /// A one-task run: the header holds its request, so its task's line
    /// shows no title.
    pub one_task: bool,
}

/// One plan's tasks, in their declared order.
#[derive(Clone, Debug, Default)]
pub struct PlanTasks {
    /// The plan's id.
    pub plan_id: String,
    /// Its tasks: id and title.
    pub tasks: Vec<(String, String)>,
}

impl RunHeader {
    /// The header of a run of `plans` from `plans_dir`: a plan named by its
    /// name, a one-task run (under `.roko/runs/`) by its request, a set by its
    /// directory; how many tasks run at once (`parallel` when set, else the
    /// plans' `max_parallel`); the model the tasks start on when they all
    /// name one; and `cap_usd` for a single plan.
    #[must_use]
    pub fn for_plans(
        plans: &[crate::runner::plan_loader::Plan],
        plans_dir: &std::path::Path,
        workdir: &std::path::Path,
        parallel: usize,
        cap_usd: Option<f64>,
        style: Style,
    ) -> Self {
        let one_task_run =
            plans_dir.starts_with(roko_fs::RokoLayout::for_project(workdir).runs_dir());
        let subject = match plans {
            [plan] if one_task_run => format!(
                "run \u{201c}{}\u{201d}",
                plan.tasks.meta.title.as_deref().unwrap_or(&plan.id)
            ),
            [plan] => format!("run {}", style.bold(&plan.id)),
            _ => format!(
                "run {}",
                style.bold(
                    plans_dir
                        .strip_prefix(workdir)
                        .unwrap_or(plans_dir)
                        .display()
                )
            ),
        };
        let parallel = if parallel > 0 {
            parallel
        } else {
            plans
                .iter()
                .filter_map(|plan| plan.tasks.meta.max_parallel)
                .max()
                .map_or(1, |most| most as usize)
        };
        let mut hints = plans
            .iter()
            .flat_map(|plan| plan.tasks.tasks.iter())
            .map(|task| task.model_hint.clone());
        let first = hints.next().flatten();
        let model = first.filter(|first| hints.all(|hint| hint.as_deref() == Some(first.as_str())));
        Self {
            subject,
            plans: plans
                .iter()
                .map(|plan| PlanTasks {
                    plan_id: plan.id.clone(),
                    tasks: plan
                        .tasks
                        .tasks
                        .iter()
                        .map(|task| (task.id.clone(), task.title.clone()))
                        .collect(),
                })
                .collect(),
            parallel,
            model,
            cap_usd: cap_usd.filter(|cap| plans.len() == 1 && *cap > 0.0),
            note: None,
            one_task: one_task_run,
        }
    }

    fn task_count(&self) -> usize {
        self.plans.iter().map(|plan| plan.tasks.len()).sum()
    }

    /// The header: `◆ roko  run greeting`, then a faint line of what the run
    /// holds. Appended output gets both on one line.
    #[must_use]
    pub fn lines(&self, style: Style) -> Vec<String> {
        let mut facts = Vec::new();
        if self.plans.len() > 1 {
            facts.push(term_style::plural(self.plans.len(), "plan", "plans"));
        }
        facts.push(term_style::plural(self.task_count(), "task", "tasks"));
        if self.parallel > 1 && self.task_count() > 1 {
            facts.push(format!("up to {} at once", self.parallel));
        }
        if let Some(model) = &self.model {
            facts.push(model.clone());
        }
        if let Some(cap) = self.cap_usd.filter(|cap| *cap > 0.0) {
            facts.push(format!("cap {}", term_style::money(cap)));
        }
        facts.extend(self.note.clone());
        let dot = format!(" {} ", style.glyph(Glyph::Dot));
        let mark = format!(
            "{} {}  {}",
            style.paint(Role::Mark, style.glyph(Glyph::Mark)),
            style.bold("roko"),
            self.subject
        );
        if style.live() {
            vec![
                fit(&mark, style.width()),
                format!(
                    "  {}",
                    style.paint(
                        Role::Dim,
                        term_style::truncate(&facts.join(&dot), style.width().saturating_sub(3))
                    )
                ),
                String::new(),
            ]
        } else {
            vec![format!("{mark}{dot}{}", facts.join(&dot))]
        }
    }
}

/// Where a task stands, in the design system's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    /// Not started.
    Waiting,
    /// Running now.
    Running,
    /// Passed its checks (or was already done).
    Passed,
    /// Finished with no check run on its work.
    NotChecked,
    /// Failed.
    Failed,
    /// Did not run: a task it needs did not pass, or it was skipped.
    Skipped,
    /// Stopped with the run.
    Stopped,
}

impl TaskState {
    fn from_outcome(outcome: &str) -> Self {
        match outcome {
            "passed"
            | "passed_with_preexisting_failures"
            | "accepted_with_failures"
            | "already_satisfied" => Self::Passed,
            "unverified" => Self::NotChecked,
            "failed" => Self::Failed,
            "blocked" | "skipped" | "condition-skipped" => Self::Skipped,
            _ => Self::Stopped,
        }
    }

    const fn finished(self) -> bool {
        !matches!(self, Self::Waiting | Self::Running)
    }
}

/// One task's line.
#[derive(Clone, Debug)]
pub struct TaskRow {
    /// Its plan.
    pub plan_id: String,
    /// Its id.
    pub id: String,
    /// Its number in its plan, from 1: the person reads `task 3`.
    pub number: usize,
    /// Its title.
    pub title: String,
    /// Where it stands.
    pub state: TaskState,
    /// The try running now, or the last one, from 1.
    pub try_no: u32,
    /// What it is doing now.
    pub activity: Option<String>,
    started: Option<Instant>,
    ended: Option<Instant>,
    /// Spend by try.
    spend: BTreeMap<u32, f64>,
    /// The checks its latest try passed.
    pub checks_passed: u32,
    /// The check its latest failed try failed on.
    pub failed_check: Option<String>,
    /// A try that failed and was tried again: `try 1: check usage failed`.
    pub retry_note: Option<String>,
    /// The retry note, until the next try starts: a last failed try gets one
    /// too, and is not tried again.
    pending_retry: bool,
    /// Why it failed, or did not run: the check's error line.
    pub failure: Option<String>,
}

impl TaskRow {
    fn new(plan_id: &str, id: &str, number: usize, title: &str) -> Self {
        Self {
            plan_id: plan_id.to_string(),
            id: id.to_string(),
            number,
            title: title.to_string(),
            state: TaskState::Waiting,
            try_no: 0,
            activity: None,
            started: None,
            ended: None,
            spend: BTreeMap::new(),
            checks_passed: 0,
            failed_check: None,
            retry_note: None,
            pending_retry: false,
            failure: None,
        }
    }

    /// How long it has run, or ran.
    #[must_use]
    pub fn elapsed(&self, now: Instant) -> Option<Duration> {
        let started = self.started?;
        Some(self.ended.unwrap_or(now).saturating_duration_since(started))
    }

    /// What it has spent, every try together.
    #[must_use]
    pub fn spent(&self) -> f64 {
        self.spend.values().sum()
    }

    /// `1 try`, `2 tries`.
    #[must_use]
    pub fn tries(&self) -> String {
        term_style::plural(self.try_no.max(1) as usize, "try", "tries")
    }

    /// `task 3`.
    #[must_use]
    pub fn label(&self) -> String {
        format!("task {}", self.number)
    }
}

/// A plan's whole-plan check.
#[derive(Clone, Debug)]
pub struct PlanCheckRow {
    /// Its plan.
    pub plan_id: String,
    /// The check's name.
    pub name: String,
    /// The command it runs.
    pub command: String,
    /// Where it stands.
    pub state: TaskState,
    started: Instant,
    ended: Option<Instant>,
    /// Why it failed.
    pub reason: Option<String>,
}

/// What happened, for appended output: one line each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// Task `index` started.
    Started(usize),
    /// Task `index` failed a try and runs again.
    Retry(usize),
    /// Task `index` finished.
    Finished(usize),
    /// Plan `id`'s plan check started.
    PlanCheckStarted(String),
    /// Plan `id`'s plan check finished.
    PlanCheckFinished(String),
    /// roko started checking the plan's checks on the code as it is.
    CheckingStarted,
    /// It finished checking them.
    CheckingFinished,
}

/// The step before any task runs (the spec gate): roko runs each task's
/// checks on the code as it is, where they must fail, since a check that
/// passes before the work proves nothing.
#[derive(Clone, Debug)]
pub struct Checking {
    started: Instant,
    ended: Option<Instant>,
    /// Check steps to run.
    pub steps: u32,
    /// Check steps run so far.
    pub done: u32,
}

/// What the view knows of the run so far.
#[derive(Clone, Debug)]
pub struct RunModel {
    /// The run's header.
    pub header: RunHeader,
    /// Every task, in plan order then declared order.
    pub tasks: Vec<TaskRow>,
    /// Each plan's whole-plan check, once it started.
    pub plan_checks: Vec<PlanCheckRow>,
    /// The plan's checks run on the code as it is, before any task.
    pub checking: Option<Checking>,
    started: Instant,
}

impl RunModel {
    /// The run as it starts: every task waiting.
    #[must_use]
    pub fn new(header: RunHeader, now: Instant) -> Self {
        let tasks = header
            .plans
            .iter()
            .flat_map(|plan| {
                plan.tasks
                    .iter()
                    .enumerate()
                    .map(|(index, (id, title))| TaskRow::new(&plan.plan_id, id, index + 1, title))
            })
            .collect();
        Self {
            header,
            tasks,
            plan_checks: Vec::new(),
            checking: None,
            started: now,
        }
    }

    fn task(&self, plan_id: &str, task_id: &str) -> Option<usize> {
        self.tasks
            .iter()
            .position(|task| task.plan_id == plan_id && task.id == task_id)
    }

    fn owns(&self, plan_id: &str) -> bool {
        self.header.plans.iter().any(|plan| plan.plan_id == plan_id)
    }

    /// The row of task `task_id`, added at its plan's end when the header did
    /// not list it.
    fn row(&mut self, plan_id: &str, task_id: &str, title: &str) -> Option<usize> {
        if let Some(index) = self.task(plan_id, task_id) {
            return Some(index);
        }
        if !self.owns(plan_id) {
            return None;
        }
        let number = self
            .tasks
            .iter()
            .filter(|task| task.plan_id == plan_id)
            .count()
            + 1;
        let at = self
            .tasks
            .iter()
            .rposition(|task| task.plan_id == plan_id)
            .map_or(self.tasks.len(), |last| last + 1);
        self.tasks
            .insert(at, TaskRow::new(plan_id, task_id, number, title));
        Some(at)
    }

    /// Take in one of the run's events; say what appended output prints for
    /// it.
    pub fn apply(&mut self, event: &DashboardEvent, now: Instant) -> Vec<Note> {
        match event {
            DashboardEvent::TaskStarted {
                plan_id,
                task_id,
                title,
                ..
            } => {
                let Some(index) = self.row(plan_id, task_id, title) else {
                    return Vec::new();
                };
                let task = &mut self.tasks[index];
                if task.state == TaskState::Running {
                    return Vec::new();
                }
                task.state = TaskState::Running;
                task.started.get_or_insert(now);
                task.try_no = task.try_no.max(1);
                vec![Note::Started(index)]
            }
            DashboardEvent::AgentSpawned {
                plan_id,
                task_id,
                attempt,
                ..
            } => {
                let Some(index) = self.task(plan_id, task_id) else {
                    return Vec::new();
                };
                let task = &mut self.tasks[index];
                task.started.get_or_insert(now);
                let mut notes = Vec::new();
                if *attempt > task.try_no {
                    task.try_no = *attempt;
                    task.checks_passed = 0;
                    // The try a retry note announced has started.
                    if task.pending_retry && *attempt > 1 {
                        task.pending_retry = false;
                        notes.push(Note::Retry(index));
                    }
                }
                if !task.state.finished() {
                    task.state = TaskState::Running;
                    task.activity = Some("starting".to_string());
                }
                notes
            }
            DashboardEvent::AgentOutput {
                plan_id,
                task_id,
                content,
                ..
            } => {
                if let (Some(index), Some(activity)) =
                    (self.task(plan_id, task_id), activity_of(content))
                    && self.tasks[index].state == TaskState::Running
                {
                    self.tasks[index].activity = Some(activity);
                }
                Vec::new()
            }
            DashboardEvent::GateRungStarted {
                plan_id,
                task_id,
                rung_name,
            } => {
                if let Some(index) = self.task(plan_id, task_id) {
                    self.tasks[index].activity =
                        Some(format!("checking {}", check_name(rung_name)));
                }
                Vec::new()
            }
            DashboardEvent::GateResult {
                plan_id,
                task_id,
                gate,
                passed,
                output_text,
            } => {
                if let Some(index) = self.task(plan_id, task_id) {
                    let task = &mut self.tasks[index];
                    if *passed {
                        task.checks_passed += 1;
                    } else {
                        task.failed_check = Some(check_name(gate));
                        task.failure = output_text.as_deref().and_then(error_line);
                    }
                }
                Vec::new()
            }
            DashboardEvent::RetryFeedback {
                plan_id,
                task_id,
                attempt,
                failing_steps,
                check,
                ..
            } => {
                let Some(index) = self.task(plan_id, task_id) else {
                    return Vec::new();
                };
                let task = &mut self.tasks[index];
                let what = failing_steps
                    .first()
                    .map(|step| check_name(step))
                    .or_else(|| task.failed_check.clone())
                    .unwrap_or_else(|| check_name(check));
                task.retry_note = Some(format!("try {attempt}: check {what} failed"));
                task.pending_retry = true;
                Vec::new()
            }
            DashboardEvent::CallCost {
                plan_id,
                task_id,
                attempt,
                attempt_cost_usd,
                ..
            } => {
                if let Some(index) = self.task(plan_id, task_id) {
                    let spent = self.tasks[index].spend.entry(*attempt).or_insert(0.0);
                    *spent = spent.max(*attempt_cost_usd);
                }
                Vec::new()
            }
            DashboardEvent::TaskCompleted {
                plan_id,
                task_id,
                outcome,
            } => {
                let Some(index) = self.task(plan_id, task_id) else {
                    return Vec::new();
                };
                let task = &mut self.tasks[index];
                if task.state.finished() {
                    return Vec::new();
                }
                task.state = TaskState::from_outcome(outcome);
                task.ended = Some(now);
                task.activity = None;
                vec![Note::Finished(index)]
            }
            DashboardEvent::TaskBlocked {
                plan_id,
                task_id,
                title,
                blocked_by,
                reason,
            } => {
                let Some(index) = self.row(plan_id, task_id, title) else {
                    return Vec::new();
                };
                let by = blocked_by
                    .as_deref()
                    .and_then(|by| self.task(plan_id, by))
                    .map(|by| self.tasks[by].label());
                let task = &mut self.tasks[index];
                if task.state.finished() {
                    return Vec::new();
                }
                task.state = TaskState::Skipped;
                task.failure = Some(by.map_or_else(
                    || reason.clone(),
                    |by| format!("{by} did not pass, so it did not run"),
                ));
                vec![Note::Finished(index)]
            }
            DashboardEvent::PlanCheckStarted {
                plan_id,
                name,
                command,
                ..
            } => {
                if !self.owns(plan_id) {
                    return Vec::new();
                }
                match self
                    .plan_checks
                    .iter_mut()
                    .find(|check| &check.plan_id == plan_id)
                {
                    Some(check) => {
                        check.name.clone_from(name);
                        check.command.clone_from(command);
                        check.state = TaskState::Running;
                    }
                    None => self.plan_checks.push(PlanCheckRow {
                        plan_id: plan_id.clone(),
                        name: name.clone(),
                        command: command.clone(),
                        state: TaskState::Running,
                        started: now,
                        ended: None,
                        reason: None,
                    }),
                }
                vec![Note::PlanCheckStarted(plan_id.clone())]
            }
            DashboardEvent::PlanCheckFinished {
                plan_id,
                passed,
                reason,
                stopped_by,
                ..
            } => {
                let Some(check) = self
                    .plan_checks
                    .iter_mut()
                    .find(|check| &check.plan_id == plan_id)
                else {
                    return Vec::new();
                };
                if check.state.finished() {
                    return Vec::new();
                }
                check.state = if *passed {
                    TaskState::Passed
                } else if stopped_by.is_some() {
                    TaskState::Stopped
                } else {
                    TaskState::Failed
                };
                check.ended = Some(now);
                check.reason.clone_from(reason);
                vec![Note::PlanCheckFinished(plan_id.clone())]
            }
            DashboardEvent::SpecGateStarted { steps_total, .. } if *steps_total > 0 => {
                self.checking = Some(Checking {
                    started: now,
                    ended: None,
                    steps: *steps_total,
                    done: 0,
                });
                vec![Note::CheckingStarted]
            }
            DashboardEvent::SpecGateStep { .. } => {
                if let Some(checking) = self.checking.as_mut() {
                    checking.done += 1;
                }
                Vec::new()
            }
            DashboardEvent::SpecGateFinished { .. } => match self.checking.as_mut() {
                Some(checking) if checking.ended.is_none() => {
                    checking.ended = Some(now);
                    vec![Note::CheckingFinished]
                }
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// The tasks that passed, and how many there are.
    #[must_use]
    pub fn progress(&self) -> (usize, usize) {
        let done = self
            .tasks
            .iter()
            .filter(|task| task.state == TaskState::Passed)
            .count();
        (done, self.tasks.len())
    }

    /// What the run has spent so far.
    #[must_use]
    pub fn spent(&self) -> f64 {
        self.tasks.iter().map(TaskRow::spent).sum()
    }

    /// The checks that passed on the work that was kept (each passed task's
    /// last try, and each plan check that passed), and the checks the work
    /// that failed failed on.
    #[must_use]
    pub fn checks(&self) -> (u32, u32) {
        let passed: u32 = self
            .tasks
            .iter()
            .filter(|task| task.state == TaskState::Passed)
            .map(|task| task.checks_passed)
            .sum::<u32>()
            + self
                .plan_checks
                .iter()
                .filter(|check| check.state == TaskState::Passed)
                .count() as u32;
        let failed = self
            .tasks
            .iter()
            .filter(|task| task.state == TaskState::Failed && task.failed_check.is_some())
            .count() as u32
            + self
                .plan_checks
                .iter()
                .filter(|check| check.state == TaskState::Failed)
                .count() as u32;
        (passed, failed)
    }

    /// How long the run has taken.
    #[must_use]
    pub fn elapsed(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started)
    }

    /// The first failure, as one sentence: `task 3's check usage still failed
    /// after 2 tries`.
    #[must_use]
    pub fn first_failure(&self) -> Option<String> {
        if let Some(task) = self
            .tasks
            .iter()
            .find(|task| task.state == TaskState::Failed)
        {
            return Some(match &task.failed_check {
                Some(check) if task.try_no > 1 => format!(
                    "{}'s check {check} still failed after {}",
                    task.label(),
                    task.tries()
                ),
                Some(check) => format!("{}'s check {check} failed", task.label()),
                None => format!("{} failed", task.label()),
            });
        }
        self.plan_checks
            .iter()
            .find(|check| check.state == TaskState::Failed)
            .map(|check| match &check.reason {
                Some(reason) => format!("{}'s plan check failed: {reason}", check.plan_id),
                None => format!("{}'s plan check failed", check.plan_id),
            })
    }
}

/// The name a person reads for a check: `verify[0:imports]` reads `imports`,
/// `rung[compile]` reads `compile`.
#[must_use]
pub fn check_name(raw: &str) -> String {
    let raw = raw.split(" (").next().unwrap_or(raw).trim();
    let inner = raw
        .split_once('[')
        .and_then(|(_, rest)| rest.split_once(']'))
        .map_or(raw, |(inner, _)| inner);
    inner.rsplit(':').next().unwrap_or(inner).trim().to_string()
}

/// The line of a check's output that says what went wrong: the last line that
/// reads like an error, else the last line.
fn error_line(output: &str) -> Option<String> {
    let lines: Vec<&str> = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("---"))
        .collect();
    lines
        .iter()
        .rev()
        .find(|line| {
            let lower = line.to_ascii_lowercase();
            (lower.contains("error") || lower.contains("failed")) && !lower.starts_with("traceback")
        })
        .or_else(|| lines.last())
        .map(|line| (*line).to_string())
}

/// What a dashboard agent-output record says the agent is doing.
fn activity_of(content: &str) -> Option<String> {
    let record = content.strip_prefix(crate::runner::tui_bridge::STREAM_RECORD_PREFIX)?;
    let value: serde_json::Value = serde_json::from_str(record).ok()?;
    match value["kind"].as_str()? {
        "tool_start" => {
            let tool = value["payload"]["tool"].as_str()?;
            let target = value["payload"]["target"].as_str().unwrap_or_default();
            Some(describe_tool(tool, target))
        }
        "reasoning" => Some("thinking".to_string()),
        _ => None,
    }
}

/// `read_file greeting.py` reads `reading greeting.py`.
fn describe_tool(tool: &str, target: &str) -> String {
    let name = tool.to_ascii_lowercase();
    let file = target
        .rsplit('/')
        .find(|part| !part.trim().is_empty())
        .unwrap_or(target)
        .trim();
    let verb = if name.contains("read") || name == "view" || name == "cat" {
        "reading"
    } else if name.contains("write") || name.contains("create") {
        "writing"
    } else if name.contains("edit") || name.contains("patch") || name.contains("replace") {
        "editing"
    } else if name.contains("bash")
        || name.contains("shell")
        || name.contains("exec")
        || name == "run"
    {
        let command = target.split_whitespace().next().unwrap_or("a command");
        return format!("running {command}");
    } else if ["grep", "search", "glob", "find", "list", "ls"]
        .iter()
        .any(|word| name.contains(word))
    {
        return "searching".to_string();
    } else {
        return tool.replace('_', " ");
    };
    if file.is_empty() {
        verb.to_string()
    } else {
        format!("{verb} {file}")
    }
}

// ── Layout ────────────────────────────────────────────────────────────────

const TRIES_W: usize = 9;
const TIME_W: usize = 7;
const MONEY_W: usize = 7;

/// The task table's column widths at the style's width.
struct Columns {
    label: usize,
    title: usize,
}

impl Columns {
    fn of(model: &RunModel, style: Style) -> Self {
        let label = model
            .tasks
            .iter()
            .map(|task| task.label().len())
            .max()
            .unwrap_or(6);
        // "  G " + label + "  " + title + "  " + tries + time + money, and
        // never the terminal's last column.
        let fixed = 4 + label + 2 + 2 + TRIES_W + TIME_W + MONEY_W;
        let room = style.width().saturating_sub(1 + fixed).max(12);
        // A one-task run's request is its header; its line needs no title.
        if model.header.one_task {
            return Self { label, title: 0 };
        }
        let longest = model
            .tasks
            .iter()
            .map(|task| term_style::visible_width(&task.title))
            .max()
            .unwrap_or(24)
            .max(24);
        Self {
            label,
            title: longest.min(room),
        }
    }

    /// Where the line under a task starts: under its title.
    fn indent(&self) -> String {
        " ".repeat(4 + self.label + 2)
    }
}

/// `line` cut so it never reaches the terminal's last column.
fn fit(line: &str, width: usize) -> String {
    if term_style::visible_width(line) < width {
        line.to_string()
    } else {
        term_style::truncate(&term_style::strip_ansi(line), width.saturating_sub(1))
    }
}

fn glyph_for(state: TaskState, style: Style, frame: usize) -> String {
    match state {
        TaskState::Waiting => style.paint(Role::Dim, style.glyph(Glyph::Waiting)),
        TaskState::Running => style.paint(Role::Live, style.spinner(frame)),
        TaskState::Passed => style.paint(Role::Pass, style.glyph(Glyph::Pass)),
        TaskState::NotChecked => style.paint(Role::Dim, style.glyph(Glyph::NotChecked)),
        TaskState::Failed => style.paint(Role::Fail, style.glyph(Glyph::Fail)),
        TaskState::Skipped => style.paint(Role::Dim, style.glyph(Glyph::Skipped)),
        TaskState::Stopped => style.paint(Role::Dim, style.glyph(Glyph::Stopped)),
    }
}

fn task_lines(
    task: &TaskRow,
    columns: &Columns,
    style: Style,
    now: Instant,
    frame: usize,
) -> Vec<String> {
    let glyph = glyph_for(task.state, style, frame);
    let quiet = matches!(task.state, TaskState::Waiting | TaskState::Skipped);
    let label = term_style::pad(&task.label(), columns.label);
    let title = term_style::truncate(&task.title, columns.title);
    let (label, title) = if quiet {
        (style.paint(Role::Dim, label), style.paint(Role::Dim, title))
    } else {
        (label, title)
    };
    let head = if columns.title == 0 {
        format!("  {glyph} {label}  ")
    } else {
        format!(
            "  {glyph} {label}  {}  ",
            term_style::pad(&title, columns.title)
        )
    };
    let tail = match task.state {
        TaskState::Waiting => style.paint(Role::Dim, "waiting"),
        TaskState::Running => {
            let doing = task
                .activity
                .clone()
                .unwrap_or_else(|| "starting".to_string());
            let doing = if task.try_no > 1 {
                format!("try {} · {doing}", task.try_no)
            } else {
                doing
            };
            // What it is doing may use the rest of the line.
            let room = style
                .width()
                .saturating_sub(1 + term_style::visible_width(&head))
                .max(TRIES_W + TIME_W + MONEY_W);
            style.paint(Role::Live, term_style::truncate(&doing, room))
        }
        TaskState::Skipped => style.paint(Role::Dim, "did not run"),
        TaskState::Stopped => style.paint(Role::Dim, "stopped"),
        TaskState::NotChecked => style.paint(Role::Dim, "not checked"),
        TaskState::Passed | TaskState::Failed => {
            let time = task
                .elapsed(now)
                .map(term_style::duration)
                .unwrap_or_default();
            let spent = task.spent();
            let money = if spent > 0.0 {
                term_style::money(spent)
            } else {
                String::new()
            };
            format!(
                "{}{}{}",
                term_style::pad(&task.tries(), TRIES_W),
                term_style::pad_left(&time, TIME_W),
                term_style::pad_left(&money, MONEY_W),
            )
        }
    };
    let mut lines = vec![fit(format!("{head}{tail}").trim_end(), style.width())];
    let indent = columns.indent();
    let room = style.width().saturating_sub(indent.len() + 1);
    if task.state == TaskState::Failed {
        // What it failed on, once: the check, then its error.
        let check = task.failed_check.as_ref().map_or_else(
            || "it failed".to_string(),
            |check| format!("check {check} failed"),
        );
        let line = match &task.failure {
            Some(failure) => format!("{check}: {failure}"),
            None => check,
        };
        lines.push(format!(
            "{indent}{}",
            style.paint(Role::Dim, term_style::truncate(&line, room))
        ));
    } else if let Some(note) = &task.retry_note {
        let shown = if task.state == TaskState::Passed {
            format!("{note}, so roko tried again")
        } else {
            note.clone()
        };
        lines.push(format!(
            "{indent}{}",
            style.paint(Role::Dim, term_style::truncate(&shown, room))
        ));
    }
    if task.state == TaskState::Skipped
        && let Some(why) = &task.failure
    {
        lines.push(format!(
            "{indent}{}",
            style.paint(Role::Dim, term_style::truncate(why, room))
        ));
    }
    lines
}

fn plan_check_lines(
    check: &PlanCheckRow,
    columns: &Columns,
    style: Style,
    now: Instant,
    frame: usize,
) -> Vec<String> {
    let glyph = glyph_for(check.state, style, frame);
    // "plan check", then its command (faint) where a task's label, title and
    // tries go, so its time lines up with theirs.
    let label = "plan check";
    let room = (columns.label + 2 + columns.title.max(24) + 2 + TRIES_W)
        .saturating_sub(label.len() + 2 + 2);
    let command = style.paint(Role::Dim, term_style::truncate(&check.command, room));
    let tail = match check.state {
        TaskState::Running => style.paint(Role::Live, "checking"),
        TaskState::Stopped => style.paint(Role::Dim, "stopped"),
        _ => {
            let time = term_style::duration(
                check
                    .ended
                    .unwrap_or(now)
                    .saturating_duration_since(check.started),
            );
            term_style::pad_left(&time, TIME_W)
        }
    };
    let mut lines = vec![fit(
        format!(
            "  {glyph} {label}  {}  {tail}",
            term_style::pad(&command, room)
        )
        .trim_end(),
        style.width(),
    )];
    if check.state == TaskState::Failed
        && let Some(reason) = &check.reason
    {
        let indent = columns.indent();
        let room = style.width().saturating_sub(indent.len() + 1);
        lines.push(format!(
            "{indent}{}",
            style.paint(Role::Dim, term_style::truncate(reason, room))
        ));
    }
    lines
}

/// The task table: a heading per plan when there are several, each task's
/// line (and the line under it, for a retry or a failure), then each plan's
/// check. `live` adds the footer of glass chips (tasks done, elapsed time,
/// spend) and keeps the table within the terminal's height.
#[must_use]
pub fn table_lines(
    model: &RunModel,
    style: Style,
    now: Instant,
    frame: usize,
    live: bool,
) -> Vec<String> {
    let columns = Columns::of(model, style);
    let several = model.header.plans.len() > 1;
    // (how much it matters while live: 0 most, lines)
    let mut rows: Vec<(u8, Vec<String>)> = Vec::new();
    for plan in &model.header.plans {
        if several {
            rows.push((0, vec![format!("  {}", style.bold(&plan.plan_id))]));
        }
        for task in model
            .tasks
            .iter()
            .filter(|task| task.plan_id == plan.plan_id)
        {
            let rank = match task.state {
                TaskState::Running | TaskState::Failed => 0,
                TaskState::Passed | TaskState::NotChecked | TaskState::Stopped => 1,
                TaskState::Waiting | TaskState::Skipped => 2,
            };
            rows.push((rank, task_lines(task, &columns, style, now, frame)));
        }
        if let Some(check) = model
            .plan_checks
            .iter()
            .find(|check| check.plan_id == plan.plan_id)
        {
            rows.push((0, plan_check_lines(check, &columns, style, now, frame)));
        }
    }
    if let Some(checking) = model
        .checking
        .as_ref()
        .filter(|checking| checking.ended.is_none())
    {
        rows.insert(
            0,
            (
                0,
                vec![format!(
                    "  {} {}  {}",
                    style.paint(Role::Live, style.spinner(frame)),
                    style.paint(Role::Live, "checking the plan's checks"),
                    style.paint(
                        Role::Dim,
                        format!("{} of {}", checking.done, checking.steps)
                    )
                )],
            ),
        );
    }
    let total: usize = rows.iter().map(|(_, lines)| lines.len()).sum();
    // The header's three lines and the footer's two stay on the screen too.
    let budget = style.height().saturating_sub(6).max(4);
    let mut lines: Vec<String> = Vec::new();
    if live && total > budget {
        let mut keep = vec![false; rows.len()];
        let mut used = 1; // the "and N more" line
        for rank in 0..3 {
            for (index, (row_rank, row)) in rows.iter().enumerate() {
                if *row_rank == rank && used + row.len() <= budget {
                    keep[index] = true;
                    used += row.len();
                }
            }
        }
        let (mut done, mut waiting) = (0, 0);
        for (index, (rank, row)) in rows.iter().enumerate() {
            if keep[index] {
                lines.extend(row.iter().cloned());
            } else if *rank == 1 {
                done += 1;
            } else {
                waiting += 1;
            }
        }
        let mut more = Vec::new();
        if done > 0 {
            more.push(format!("{done} done"));
        }
        if waiting > 0 {
            more.push(format!("{waiting} waiting"));
        }
        lines.push(format!(
            "  {}",
            style.paint(Role::Dim, format!("and {} more", more.join(", ")))
        ));
    } else {
        for (_, row) in rows {
            lines.extend(row);
        }
    }
    if live {
        let (done, all) = model.progress();
        let mut footer = format!(
            "  {} {} {}",
            style.glass(format!("{done}/{all}")),
            style.glass(term_style::clock(model.elapsed(now))),
            style.glass(term_style::money(model.spent())),
        );
        if let Some(cap) = model.header.cap_usd.filter(|cap| *cap > 0.0) {
            footer.push_str(&style.paint(Role::Dim, format!(" of {}", term_style::money(cap))));
        }
        lines.push(String::new());
        lines.push(footer);
    }
    lines
}

/// The line appended output prints for `note`.
#[must_use]
pub fn note_line(model: &RunModel, note: &Note, style: Style, now: Instant) -> String {
    let dot = format!(" {} ", style.glyph(Glyph::Dot));
    match note {
        Note::Started(index) => {
            let task = &model.tasks[*index];
            format!(
                "  {} {}  {}",
                style.paint(Role::Live, style.glyph(Glyph::Running)),
                task.label(),
                task.title
            )
        }
        Note::Retry(index) => {
            let task = &model.tasks[*index];
            format!(
                "  {} {}  {}; trying again",
                style.paint(Role::You, style.glyph(Glyph::Retry)),
                task.label(),
                task.retry_note.as_deref().unwrap_or("a try failed")
            )
        }
        Note::Finished(index) => {
            let task = &model.tasks[*index];
            let glyph = glyph_for(task.state, style, 0);
            let mut facts = vec![
                match task.state {
                    TaskState::Passed => "passed",
                    TaskState::Failed => "failed",
                    TaskState::NotChecked => "not checked",
                    TaskState::Skipped => "did not run",
                    _ => "stopped",
                }
                .to_string(),
            ];
            if matches!(task.state, TaskState::Passed | TaskState::Failed) {
                facts.push(task.tries());
                if let Some(elapsed) = task.elapsed(now) {
                    facts.push(term_style::duration(elapsed));
                }
                if task.spent() > 0.0 {
                    facts.push(term_style::money(task.spent()));
                }
            }
            let mut line = format!("  {glyph} {}  {}", task.label(), facts.join(&dot));
            if task.state == TaskState::Failed {
                if let Some(check) = &task.failed_check {
                    line.push_str(&format!(": check {check} failed"));
                }
            } else if task.state == TaskState::Skipped
                && let Some(why) = &task.failure
            {
                line.push_str(&format!(": {why}"));
            }
            line
        }
        Note::PlanCheckStarted(plan_id) => {
            let command = model
                .plan_checks
                .iter()
                .find(|check| &check.plan_id == plan_id)
                .map_or("", |check| check.command.as_str());
            format!(
                "  {} plan check  {command}",
                style.paint(Role::Live, style.glyph(Glyph::Running))
            )
        }
        Note::PlanCheckFinished(plan_id) => {
            let Some(check) = model
                .plan_checks
                .iter()
                .find(|check| &check.plan_id == plan_id)
            else {
                return String::new();
            };
            let glyph = glyph_for(check.state, style, 0);
            let took = term_style::duration(
                check
                    .ended
                    .unwrap_or(now)
                    .saturating_duration_since(check.started),
            );
            let verdict = match check.state {
                TaskState::Passed => "passed",
                TaskState::Stopped => "stopped",
                _ => "failed",
            };
            let mut line = format!("  {glyph} plan check  {verdict}{dot}{took}");
            if let (TaskState::Failed, Some(reason)) = (check.state, &check.reason) {
                line.push_str(&format!(": {reason}"));
            }
            line
        }
        Note::CheckingStarted => format!(
            "  {} checking the plan's checks{dot}{} steps",
            style.paint(Role::Live, style.glyph(Glyph::Running)),
            model.checking.as_ref().map_or(0, |checking| checking.steps)
        ),
        Note::CheckingFinished => {
            let took = model
                .checking
                .as_ref()
                .map_or_else(String::new, |checking| {
                    term_style::duration(
                        checking
                            .ended
                            .unwrap_or(now)
                            .saturating_duration_since(checking.started),
                    )
                });
            format!(
                "  {} checked the plan's checks{dot}{took}",
                style.paint(Role::Pass, style.glyph(Glyph::Pass))
            )
        }
    }
}

// ── The summary ───────────────────────────────────────────────────────────

/// How a run ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Every plan passed.
    Done,
    /// It finished, but some work had no check run on it.
    NotChecked,
    /// A plan failed.
    Failed,
    /// It was stopped: `Ctrl-C`, a signal, a deadline.
    Stopped(String),
}

/// What a run's summary says.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    /// How it ended.
    pub outcome: Outcome,
    /// The one reason it did not pass, as a sentence.
    pub reason: Option<String>,
    /// Tasks that passed, and all of them.
    pub tasks: (usize, usize),
    /// Checks that passed on the kept work, and checks that failed.
    pub checks: (u32, u32),
    /// How long it took.
    pub elapsed: Duration,
    /// What it spent.
    pub spent_usd: f64,
    /// Its spend cap.
    pub cap_usd: Option<f64>,
    /// Where the work is, as a sentence: `The work is on its own branch: +9 −1
    /// in 3 files.` `None` when nothing was delivered.
    pub delivered: Option<String>,
    /// What a person should know besides: one line each.
    pub notes: Vec<String>,
    /// The one thing to do next: a command to paste.
    pub next: Option<String>,
}

/// The summary's lines: the result, where the work is, then Next.
#[must_use]
pub fn summary_lines(summary: &Summary, style: Style) -> Vec<String> {
    let dot = format!(" {} ", style.glyph(Glyph::Dot));
    let (glyph, word) = match &summary.outcome {
        Outcome::Done => (
            style.paint(Role::Pass, style.glyph(Glyph::Pass)),
            style.paint_bold(Role::Pass, "Done"),
        ),
        Outcome::NotChecked => (
            style.paint(Role::Dim, style.glyph(Glyph::NotChecked)),
            style.bold("Not checked"),
        ),
        Outcome::Failed => (
            style.paint(Role::Fail, style.glyph(Glyph::Fail)),
            style.paint_bold(Role::Fail, "Failed"),
        ),
        Outcome::Stopped(_) => (
            style.paint(Role::Dim, style.glyph(Glyph::Stopped)),
            style.bold("Stopped"),
        ),
    };
    let (done, all) = summary.tasks;
    let (passed, failed) = summary.checks;
    let mut facts = vec![format!(
        "{} {}",
        style.bold(format!("{done}/{all}")),
        if all == 1 { "task" } else { "tasks" }
    )];
    if failed > 0 {
        let all_checks = passed + failed;
        facts.push(format!(
            "{passed} of {all_checks} {}",
            if all_checks == 1 { "check" } else { "checks" }
        ));
    } else if passed > 0 {
        facts.push(format!(
            "{} {}",
            style.bold(passed),
            if passed == 1 {
                "check passed"
            } else {
                "checks passed"
            }
        ));
    }
    facts.push(style.bold(term_style::duration(summary.elapsed)));
    let money = style.bold(term_style::money(summary.spent_usd));
    facts.push(match summary.cap_usd.filter(|cap| *cap > 0.0) {
        Some(cap) => format!("{money} of {}", term_style::money(cap)),
        None => money,
    });
    let mut lines = Vec::new();
    match (&summary.outcome, &summary.reason) {
        (Outcome::Done, _) | (_, None) => {
            lines.push(format!("  {glyph} {word}  {}", facts.join(&dot)));
        }
        (_, Some(reason)) => {
            lines.push(format!("  {glyph} {word}  {reason}"));
            lines.push(format!(
                "  {}",
                style.paint(Role::Dim, term_style::strip_ansi(&facts.join(&dot)))
            ));
        }
    }
    let room = style.width().saturating_sub(3);
    match &summary.delivered {
        Some(delivered) => {
            const UNCHANGED: &str = "Your checkout is unchanged.";
            if term_style::visible_width(delivered) + 1 + UNCHANGED.len() <= room {
                lines.push(format!(
                    "  {}",
                    style.paint(Role::Dim, format!("{delivered} {UNCHANGED}"))
                ));
            } else {
                lines.push(format!("  {}", style.paint(Role::Dim, delivered)));
                lines.push(format!("  {}", style.paint(Role::Dim, UNCHANGED)));
            }
        }
        None if summary.outcome != Outcome::Done => lines.push(format!(
            "  {}",
            style.paint(
                Role::Dim,
                "Nothing was delivered; your checkout is unchanged."
            )
        )),
        None => {}
    }
    for note in &summary.notes {
        lines.push(format!("  {}", style.paint(Role::Dim, note)));
    }
    if let Some(next) = &summary.next {
        lines.push(String::new());
        let word = style.paint_bold(Role::You, "Next");
        // A command is never cut: it goes on a line of its own when it
        // doesn't fit beside the word.
        if "Next  ".len() + term_style::visible_width(next) <= room {
            lines.push(format!("  {word}  {next}"));
        } else {
            lines.push(format!("  {word}"));
            lines.push(format!("  {next}"));
        }
    }
    lines
}

// ── Drawing on a terminal ─────────────────────────────────────────────────

/// A block of lines kept at the bottom of the terminal and redrawn in place:
/// only the lines that changed are written, inside one synchronized update,
/// with the cursor hidden until [`LiveRegion::finish`].
pub struct LiveRegion<W: Write> {
    out: W,
    drawn: Vec<String>,
    hid_cursor: bool,
}

impl<W: Write> LiveRegion<W> {
    /// A region that draws to `out`.
    pub const fn new(out: W) -> Self {
        Self {
            out,
            drawn: Vec::new(),
            hid_cursor: false,
        }
    }

    /// Make the region read `lines`, rewriting only what changed.
    pub fn draw(&mut self, lines: &[String]) -> std::io::Result<()> {
        if lines == self.drawn.as_slice() {
            return Ok(());
        }
        let mut buf = String::from("\x1b[?2026h");
        if !self.hid_cursor {
            buf.push_str("\x1b[?25l");
            self.hid_cursor = true;
        }
        if !self.drawn.is_empty() {
            buf.push_str(&format!("\x1b[{}F", self.drawn.len()));
        }
        for (index, line) in lines.iter().enumerate() {
            if self.drawn.get(index) != Some(line) {
                buf.push_str("\x1b[2K");
                buf.push_str(line);
            }
            buf.push('\n');
        }
        if lines.len() < self.drawn.len() {
            buf.push_str("\x1b[J");
        }
        buf.push_str("\x1b[?2026l");
        self.drawn = lines.to_vec();
        self.out.write_all(buf.as_bytes())?;
        self.out.flush()
    }

    /// Draw `lines` one last time, where they stay, and show the cursor.
    pub fn finish(&mut self, lines: &[String]) -> std::io::Result<()> {
        self.draw(lines)?;
        self.drawn.clear();
        self.show_cursor()
    }

    fn show_cursor(&mut self) -> std::io::Result<()> {
        if self.hid_cursor {
            self.hid_cursor = false;
            self.out.write_all(b"\x1b[?25h")?;
            self.out.flush()?;
        }
        Ok(())
    }
}

impl<W: Write> Drop for LiveRegion<W> {
    fn drop(&mut self) {
        let _ = self.show_cursor();
    }
}

// ── The view ──────────────────────────────────────────────────────────────

/// The live view of one run, following its dashboard events on stderr until
/// [`RunView::finish`]. A `silent` view draws nothing and only keeps count,
/// for a run the TUI shows. It runs on a thread of its own, so a run that
/// keeps its runtime busy (the spec gate's checks run synchronously) still
/// moves on the screen.
pub struct RunView {
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<RunModel>>,
}

impl RunView {
    /// Print the header and start following `hub`'s events from now on.
    pub fn start(
        hub: &crate::state_hub::SharedStateHub,
        header: RunHeader,
        style: Style,
        silent: bool,
    ) -> Self {
        Self::follow(Vec::new(), hub.subscribe_events(), header, style, silent)
    }

    /// [`Self::start`], first taking in every event `hub` already holds: a
    /// mirror of a run that another process (`roko serve`) runs.
    pub fn start_replaying(
        hub: &crate::state_hub::SharedStateHub,
        header: RunHeader,
        style: Style,
    ) -> Self {
        let subscription = hub.subscribe_events_from(0);
        let replay = subscription
            .replay
            .into_iter()
            .map(|envelope| envelope.payload)
            .collect();
        Self::follow(replay, subscription.live, header, style, false)
    }

    fn follow(
        replay: Vec<DashboardEvent>,
        events: tokio::sync::broadcast::Receiver<roko_runtime::event_bus::Envelope<DashboardEvent>>,
        header: RunHeader,
        style: Style,
        silent: bool,
    ) -> Self {
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let model = RunModel::new(header, Instant::now());
        let thread = std::thread::Builder::new()
            .name("roko-run-view".to_string())
            .spawn(move || {
                match tokio::runtime::Builder::new_current_thread()
                    .enable_time()
                    .build()
                {
                    Ok(runtime) => runtime
                        .block_on(follow_events(model, replay, events, stopped, style, silent)),
                    Err(error) => {
                        tracing::warn!(%error, "the run view could not start");
                        model
                    }
                }
            })
            .map_err(|error| tracing::warn!(%error, "the run view could not start"))
            .ok();
        Self {
            stop: Some(stop),
            thread,
        }
    }

    /// Stop following the run, leave the table as it ended, and say what the
    /// view saw.
    pub async fn finish(mut self) -> Option<RunModel> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        let thread = self.thread.take()?;
        tokio::task::spawn_blocking(move || thread.join().ok())
            .await
            .ok()
            .flatten()
    }
}

impl Drop for RunView {
    /// A run that ends early, on an error, stops its view where it is, with
    /// the cursor back: the error prints below its last frame.
    fn drop(&mut self) {
        // Dropping the sender, unsent, tells the view to stop without a last
        // frame.
        self.stop.take();
    }
}

/// The view's loop: take in the run's events, redraw on a terminal (10 times
/// a second, only what changed) or append a line per event, until stopped.
async fn follow_events(
    mut model: RunModel,
    replay: Vec<DashboardEvent>,
    mut events: tokio::sync::broadcast::Receiver<roko_runtime::event_bus::Envelope<DashboardEvent>>,
    mut stopped: tokio::sync::oneshot::Receiver<()>,
    style: Style,
    silent: bool,
) -> RunModel {
    let mut region = (!silent && style.live()).then(|| LiveRegion::new(std::io::stderr()));
    if !silent {
        let mut err = std::io::stderr().lock();
        for line in model.header.lines(style) {
            let _ = writeln!(err, "{line}");
        }
    }
    let append = !silent && region.is_none();
    let take = |model: &mut RunModel, event: &DashboardEvent| {
        let now = Instant::now();
        let notes = model.apply(event, now);
        if append {
            let mut err = std::io::stderr().lock();
            for note in &notes {
                let _ = writeln!(err, "{}", note_line(model, note, style, now));
            }
        }
    };
    for event in &replay {
        take(&mut model, event);
    }
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut frame = 0usize;
    let finished = loop {
        tokio::select! {
            biased;
            stop = &mut stopped => break stop.is_ok(),
            received = events.recv() => match received {
                Ok(envelope) => take(&mut model, &envelope.payload),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break true,
            },
            _ = tick.tick() => {
                frame += 1;
                if let Some(region) = region.as_mut() {
                    let style = live_style(style);
                    let _ = region.draw(&table_lines(&model, style, Instant::now(), frame, true));
                }
            }
        }
    };
    if !finished {
        return model;
    }
    // What arrived before the stop still counts.
    while let Ok(envelope) = events.try_recv() {
        take(&mut model, &envelope.payload);
    }
    if let Some(mut region) = region {
        // The live footer goes; the table stays, as it ended.
        let style = live_style(style);
        let _ = region.finish(&table_lines(&model, style, Instant::now(), 0, false));
    }
    model
}

/// `style` at the terminal's size now: the person may have resized it.
fn live_style(style: Style) -> Style {
    match crossterm::terminal::size() {
        Ok((cols, rows)) => style
            .with_width(usize::from(cols))
            .with_height(usize::from(rows)),
        Err(_) => style,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> RunHeader {
        RunHeader {
            subject: "run greeting".to_string(),
            plans: vec![PlanTasks {
                plan_id: "greeting".to_string(),
                tasks: vec![
                    ("T1".to_string(), "Write the greeting module".to_string()),
                    (
                        "T2".to_string(),
                        "Add the command-line entry point".to_string(),
                    ),
                    ("T3".to_string(), "Document how to run it".to_string()),
                ],
            }],
            parallel: 2,
            model: Some("glm-4.7".to_string()),
            cap_usd: Some(3.0),
            note: None,
            one_task: false,
        }
    }

    fn started(task: &str) -> DashboardEvent {
        DashboardEvent::TaskStarted {
            plan_id: "greeting".into(),
            task_id: task.into(),
            title: String::new(),
            phase: "implement".into(),
        }
    }

    fn spawned(task: &str, attempt: u32) -> DashboardEvent {
        DashboardEvent::AgentSpawned {
            agent_id: format!("agent-{task}-{attempt}"),
            plan_id: "greeting".into(),
            task_id: task.into(),
            attempt,
            role: "implementer".into(),
            model: "glm-4.7".into(),
            provider: "fake".into(),
        }
    }

    fn tool(task: &str, name: &str, target: &str) -> DashboardEvent {
        let record = serde_json::json!({
            "kind": "tool_start",
            "payload": {"tool_id": "t", "tool": name, "target": target}
        });
        DashboardEvent::AgentOutput {
            agent_id: "a".into(),
            plan_id: "greeting".into(),
            task_id: task.into(),
            attempt: 1,
            content: format!(
                "{}{record}",
                crate::runner::tui_bridge::STREAM_RECORD_PREFIX
            ),
        }
    }

    fn cost(task: &str, attempt: u32, usd: f64) -> DashboardEvent {
        DashboardEvent::CallCost {
            plan_id: "greeting".into(),
            task_id: task.into(),
            attempt,
            call: 1,
            model: "glm-4.7".into(),
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cost_usd: Some(usd),
            attempt_cost_usd: usd,
            reasoning_tokens: 0,
            finish_reason: None,
            wall_ms: None,
            first_token_ms: None,
            outcome: None,
        }
    }

    fn gate(task: &str, name: &str, passed: bool) -> DashboardEvent {
        DashboardEvent::GateResult {
            plan_id: "greeting".into(),
            task_id: task.into(),
            gate: name.into(),
            passed,
            output_text: (!passed).then(|| {
                "Traceback (most recent call last):\n  File \"settings.py\", line 2\n\
                 SyntaxError: invalid syntax"
                    .into()
            }),
        }
    }

    fn done(task: &str, outcome: &str) -> DashboardEvent {
        DashboardEvent::TaskCompleted {
            plan_id: "greeting".into(),
            task_id: task.into(),
            outcome: outcome.into(),
        }
    }

    fn retry(task: &str, attempt: u32, step: &str) -> DashboardEvent {
        DashboardEvent::RetryFeedback {
            plan_id: "greeting".into(),
            task_id: task.into(),
            attempt,
            next_attempt: attempt + 1,
            check: "verify".into(),
            failing_steps: vec![format!(
                "{step} (`grep -q 'python3 cli.py' README.md`): exit code: 1"
            )],
            diagnosis: None,
            errors: Vec::new(),
        }
    }

    fn plan_check(passed: Option<bool>) -> DashboardEvent {
        match passed {
            None => DashboardEvent::PlanCheckStarted {
                plan_id: "greeting".into(),
                stage: "delivery".into(),
                step: 1,
                steps: 1,
                name: "runs".into(),
                command: "python3 cli.py world".into(),
                scope: String::new(),
                packages: Vec::new(),
                source: String::new(),
            },
            Some(passed) => DashboardEvent::PlanCheckFinished {
                plan_id: "greeting".into(),
                stage: "delivery".into(),
                step: 1,
                steps: 1,
                name: "runs".into(),
                scope: String::new(),
                packages: Vec::new(),
                passed,
                exit_code: Some(i32::from(!passed)),
                reason: None,
                stopped_by: None,
                elapsed_ms: 100,
            },
        }
    }

    /// The greeting run of round v8's captures, 9.2 s in: task 1 passed, tasks
    /// 2 and 3 run side by side, and task 3's first try failed its usage
    /// check.
    fn greeting_at_nine_seconds() -> (RunModel, Instant) {
        let t0 = Instant::now();
        let mut model = RunModel::new(header(), t0);
        for (secs, event) in [
            (0.0, started("T1")),
            (0.1, spawned("T1", 1)),
            (3.9, cost("T1", 1, 0.1007)),
            (4.0, gate("T1", "verify[0:imports]", true)),
            (4.1, gate("T1", "rung[compile]", true)),
            (5.3, done("T1", "passed")),
            (5.3, started("T2")),
            (5.3, started("T3")),
            (5.4, spawned("T2", 1)),
            (5.4, spawned("T3", 1)),
            (6.9, gate("T3", "verify[0:usage]", false)),
            (7.0, retry("T3", 1, "verify[0:usage]")),
            (7.2, spawned("T3", 2)),
            (8.4, tool("T2", "write_file", "cli.py")),
            (8.9, tool("T3", "read_file", "README.md")),
        ] {
            model.apply(&event, t0 + Duration::from_secs_f64(secs));
        }
        (model, t0)
    }

    fn finish_greeting(model: &mut RunModel, t0: Instant) {
        let at = |secs: f64| t0 + Duration::from_secs_f64(secs);
        for (secs, event) in [
            (9.5, cost("T2", 1, 0.1007)),
            (9.6, gate("T2", "verify[0:runs]", true)),
            (9.7, gate("T2", "rung[compile]", true)),
            (10.2, done("T2", "passed")),
            (14.0, cost("T3", 1, 0.1342)),
            (14.0, cost("T3", 2, 0.1007)),
            (15.0, gate("T3", "verify[0:usage]", true)),
            (15.1, gate("T3", "rung[compile]", true)),
            (15.4, done("T3", "passed")),
            (15.5, plan_check(None)),
            (15.6, plan_check(Some(true))),
        ] {
            model.apply(&event, at(secs));
        }
    }

    #[test]
    fn check_names_read_as_words() {
        assert_eq!(check_name("verify[0:imports]"), "imports");
        assert_eq!(check_name("rung[compile]"), "compile");
        assert_eq!(
            check_name("verify[0:usage] (`grep -q x README.md`): exit code: 1"),
            "usage"
        );
        assert_eq!(check_name("plan-branch"), "plan-branch");
    }

    #[test]
    fn tool_calls_read_as_what_the_agent_does() {
        assert_eq!(
            describe_tool("read_file", "greeting.py"),
            "reading greeting.py"
        );
        assert_eq!(describe_tool("write_file", "src/cli.py"), "writing cli.py");
        assert_eq!(describe_tool("edit_file", "README.md"), "editing README.md");
        assert_eq!(describe_tool("bash", "cargo test -p x"), "running cargo");
        assert_eq!(describe_tool("grep", "hello"), "searching");
    }

    #[test]
    fn the_header_names_the_run_and_what_it_holds() {
        let live = Style::plain().with_live(true);
        assert_eq!(
            header().lines(live),
            [
                "◆ roko  run greeting",
                "  3 tasks · up to 2 at once · glm-4.7 · cap $3.00",
                "",
            ]
        );
        assert_eq!(
            header().lines(Style::plain()),
            ["◆ roko  run greeting · 3 tasks · up to 2 at once · glm-4.7 · cap $3.00"]
        );
    }

    #[test]
    fn the_live_table_shows_what_each_task_is_doing() {
        let (model, t0) = greeting_at_nine_seconds();
        for width in [80, 100] {
            let style = Style::plain()
                .with_live(true)
                .with_width(width)
                .with_height(32);
            let lines = table_lines(&model, style, t0 + Duration::from_secs_f64(9.2), 4, true);
            insta::assert_snapshot!(format!("live_table_{width}"), lines.join("\n"));
            for line in &lines {
                assert!(term_style::visible_width(line) < width, "{line:?}");
            }
        }
    }

    #[test]
    fn a_finished_run_ends_with_time_spend_checks_and_next() {
        let (mut model, t0) = greeting_at_nine_seconds();
        finish_greeting(&mut model, t0);
        assert_eq!(model.progress(), (3, 3));
        assert_eq!(model.checks(), (7, 0));
        let style = Style::plain().with_width(100);
        let table = table_lines(&model, style, t0 + Duration::from_secs_f64(16.9), 0, false);
        let summary = Summary {
            outcome: Outcome::Done,
            reason: None,
            tasks: model.progress(),
            checks: model.checks(),
            elapsed: Duration::from_secs_f64(16.9),
            spent_usd: 0.4363,
            cap_usd: Some(3.0),
            delivered: Some("The work is on its own branch: +9 −1 in 3 files.".into()),
            notes: Vec::new(),
            next: Some("git merge --ff-only roko/batch/graph-9a438141".into()),
        };
        let mut text = table;
        text.push(String::new());
        text.extend(summary_lines(&summary, style));
        insta::assert_snapshot!("finished_run", text.join("\n"));
    }

    #[test]
    fn a_failed_run_says_which_check_failed_and_what_to_do() {
        let t0 = Instant::now();
        let mut model = RunModel::new(header(), t0);
        let at = |secs: u64| t0 + Duration::from_secs(secs);
        model.apply(&started("T1"), t0);
        model.apply(&spawned("T1", 1), t0);
        model.apply(&gate("T1", "verify[0:imports]", false), at(4));
        model.apply(&retry("T1", 1, "verify[0:imports]"), at(4));
        model.apply(&spawned("T1", 2), at(5));
        model.apply(&gate("T1", "verify[0:imports]", false), at(9));
        model.apply(&done("T1", "failed"), at(10));
        model.apply(
            &DashboardEvent::TaskBlocked {
                plan_id: "greeting".into(),
                task_id: "T2".into(),
                title: String::new(),
                blocked_by: Some("T1".into()),
                reason: "T1 failed".into(),
            },
            at(10),
        );
        let style = Style::plain();
        let mut text = table_lines(&model, style, at(10), 0, false);
        let summary = Summary {
            outcome: Outcome::Failed,
            reason: model.first_failure(),
            tasks: model.progress(),
            checks: model.checks(),
            elapsed: Duration::from_secs_f64(12.3),
            spent_usd: 0.302,
            cap_usd: Some(3.0),
            delivered: None,
            notes: Vec::new(),
            next: Some("roko diagnose greeting".into()),
        };
        text.push(String::new());
        text.extend(summary_lines(&summary, style));
        insta::assert_snapshot!("failed_run", text.join("\n"));
    }

    #[test]
    fn appended_output_says_each_event_once() {
        let t0 = Instant::now();
        let mut model = RunModel::new(header(), t0);
        let style = Style::plain();
        let mut lines = Vec::new();
        for (secs, event) in [
            (0.0, started("T1")),
            (0.1, started("T1")),
            (3.9, cost("T1", 1, 0.1007)),
            (5.3, done("T1", "passed")),
            (5.4, done("T1", "passed")),
            (5.5, started("T3")),
            (5.5, spawned("T3", 1)),
            (6.9, gate("T3", "verify[0:usage]", false)),
            (7.0, retry("T3", 1, "verify[0:usage]")),
            (7.2, spawned("T3", 2)),
        ] {
            let now = t0 + Duration::from_secs_f64(secs);
            for note in model.apply(&event, now) {
                lines.push(note_line(&model, &note, style, now));
            }
        }
        insta::assert_snapshot!("appended", lines.join("\n"));
    }

    #[test]
    fn a_last_failed_try_is_not_said_to_run_again() {
        let t0 = Instant::now();
        let mut model = RunModel::new(header(), t0);
        let mut notes = Vec::new();
        for event in [
            started("T1"),
            spawned("T1", 1),
            retry("T1", 1, "verify[0:imports]"),
            spawned("T1", 2),
            // The retry budget is spent: no third try follows this one.
            retry("T1", 2, "verify[0:imports]"),
            done("T1", "failed"),
        ] {
            notes.extend(model.apply(&event, t0));
        }
        assert_eq!(notes, [Note::Started(0), Note::Retry(0), Note::Finished(0)]);
    }

    #[test]
    fn a_tall_plan_keeps_the_live_table_on_the_screen() {
        let t0 = Instant::now();
        let mut header = header();
        header.plans[0].tasks = (1..=30)
            .map(|n| (format!("T{n}"), format!("Task number {n}")))
            .collect();
        let mut model = RunModel::new(header, t0);
        for n in 1..=10 {
            model.apply(&started(&format!("T{n}")), t0);
            model.apply(
                &done(&format!("T{n}"), "passed"),
                t0 + Duration::from_secs(1),
            );
        }
        model.apply(&started("T11"), t0 + Duration::from_secs(1));
        let style = Style::plain().with_live(true).with_height(24);
        let lines = table_lines(&model, style, t0 + Duration::from_secs(2), 0, true);
        assert!(lines.len() + 3 <= 24, "{} lines", lines.len());
        assert!(
            lines.iter().any(|line| line.contains("task 11")),
            "{lines:#?}"
        );
        assert!(lines.iter().any(|line| line.contains("more")), "{lines:#?}");
    }

    #[test]
    fn the_live_region_rewrites_only_the_lines_that_changed() {
        let mut out = Vec::new();
        {
            let mut region = LiveRegion::new(&mut out);
            region
                .draw(&["a".to_string(), "b".to_string()])
                .expect("draw");
            region
                .draw(&["a".to_string(), "c".to_string()])
                .expect("redraw");
            region
                .finish(&["a".to_string(), "c".to_string()])
                .expect("finish");
        }
        let text = String::from_utf8(out).expect("utf8");
        // The second frame moves up two lines, leaves "a" alone and rewrites
        // the second line; the cursor comes back once.
        assert!(text.contains("\x1b[2F\n\x1b[2Kc\n"), "{text:?}");
        assert_eq!(text.matches("\x1b[2Ka").count(), 1, "{text:?}");
        assert_eq!(text.matches("\x1b[?25h").count(), 1, "{text:?}");
        assert!(text.ends_with("\x1b[?25h"), "{text:?}");
    }
}
