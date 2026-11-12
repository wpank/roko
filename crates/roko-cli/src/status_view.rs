//! What `roko status` says (gap-7a3f11), in roko's design system: what is
//! running, how the last runs ended, what they spent, and the one thing to do
//! next, as a command to paste. roko's internals (signal counts, efficiency
//! events, gate thresholds) are `roko status --details`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::graph_checkpoint::GraphCheckpointStatus;
use crate::term_style::{self, Glyph, Role, Style};

/// The most recent runs `roko status` lists.
pub const SHOWN_RUNS: usize = 5;

/// How a run stands, in the design system's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    /// roko is working on it.
    Running,
    /// Every check passed.
    Passed,
    /// It finished, but no check ran on some of its work.
    NotChecked,
    /// A check still failed after the last try, or roko hit an error.
    Failed,
    /// The person stopped it.
    Stopped,
    /// roko itself stopped (a restart or a crash): it can resume.
    Interrupted,
}

impl RunState {
    fn from_checkpoint(status: GraphCheckpointStatus) -> Self {
        match status {
            GraphCheckpointStatus::Running => Self::Running,
            GraphCheckpointStatus::Succeeded => Self::Passed,
            GraphCheckpointStatus::Unverified => Self::NotChecked,
            GraphCheckpointStatus::Failed => Self::Failed,
            GraphCheckpointStatus::Cancelled => Self::Stopped,
            GraphCheckpointStatus::Interrupted => Self::Interrupted,
        }
    }
}

/// One run, as its plan's checkpoint recorded it.
#[derive(Clone, Debug, PartialEq)]
pub struct RunEntry {
    /// The plan's id: what `roko diagnose` takes.
    pub plan_id: String,
    /// What the person calls it: the plan's id, or a one-task run's request.
    pub name: String,
    /// Where it stands.
    pub state: RunState,
    /// When it last changed, as Unix milliseconds.
    pub at_ms: i64,
    /// Tasks that passed, and all of them.
    pub tasks: (usize, usize),
    /// What it spent.
    pub spent_usd: f64,
    /// The plan's directory, as a person types it: what `roko run` takes.
    pub dir: String,
    /// The command that takes its delivered work into the checkout, while
    /// the checkout doesn't have it.
    pub merge_command: Option<String>,
}

/// What `roko status` reports.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorkspaceStatus {
    /// The project's name: its directory's.
    pub project: String,
    /// The runs, newest first.
    pub runs: Vec<RunEntry>,
    /// Plans under `plans/`.
    pub plans: usize,
    /// Spent today and this month (UTC days).
    pub today_usd: f64,
    /// Spent this month.
    pub month_usd: f64,
    /// Calls with no known price, which the spend leaves out (gap-2bec05).
    pub unpriced_calls: usize,
}

/// Read the workspace's runs from their plans' checkpoints, and its spend
/// from `.roko/learn/costs.jsonl`.
pub async fn collect(workdir: &Path) -> WorkspaceStatus {
    let plan_dirs = crate::plan::plan_dirs_by_id(workdir);
    let mut runs = Vec::new();
    let graph = workdir.join(".roko/state/graph");
    let entries = std::fs::read_dir(&graph)
        .map(|entries| entries.flatten().collect::<Vec<_>>())
        .unwrap_or_default();
    for entry in entries {
        let Ok(bytes) = std::fs::read(entry.path().join("checkpoint.json")) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        let plan_id = manifest["plan_id"].as_str().map_or_else(
            || entry.file_name().to_string_lossy().into_owned(),
            str::to_string,
        );
        let Some(status) =
            crate::graph_checkpoint::canonical_checkpoint_status_now(workdir, &plan_id)
        else {
            continue;
        };
        let run_id = manifest["run_id"].as_str().unwrap_or_default();
        // The plan's tasks: a plan under plans/, or a one-task run's plan.
        let one_task = workdir.join(".roko/runs").join(run_id).join("tasks.toml");
        let (tasks_path, one_task_run) = match plan_dirs.get(&plan_id) {
            Some(plan_dir) => (plan_dir.dir.join("tasks.toml"), false),
            None if !run_id.is_empty() && one_task.is_file() => (one_task, true),
            None => continue,
        };
        let Ok(tasks) = crate::task_parser::TasksFile::parse(&tasks_path) else {
            continue;
        };
        let total = tasks.tasks.len();
        let state = RunState::from_checkpoint(status);
        let passed = if state == RunState::Passed {
            total
        } else {
            crate::graph_checkpoint::inspect_canonical_checkpoint(workdir, &plan_id)
                .ok()
                .flatten()
                .map_or(0, |inspection| inspection.passed_tasks())
        };
        let name = if one_task_run {
            format!(
                "\u{201c}{}\u{201d}",
                tasks.meta.title.as_deref().unwrap_or(plan_id.as_str())
            )
        } else {
            plan_id.clone()
        };
        let dir = shown(workdir, tasks_path.parent().unwrap_or(workdir));
        let merge_command =
            match crate::graph_checkpoint::recorded_batch_delivery(workdir, &plan_id) {
                Some(delivery) => {
                    crate::graph_execution::batch::merge_command(workdir, &delivery.branch).await
                }
                None => None,
            };
        let spent_micro = manifest["committed"]["spent_micro_usd"]
            .as_u64()
            .unwrap_or(0);
        runs.push(RunEntry {
            plan_id,
            name,
            state,
            at_ms: manifest["updated_at_ms"].as_i64().unwrap_or(0),
            tasks: (passed.min(total), total),
            spent_usd: spent_micro as f64 / 1_000_000.0,
            dir,
            merge_command,
        });
    }
    runs.sort_by(|left, right| right.at_ms.cmp(&left.at_ms));
    let costs = roko_learn::costs_log::CostsLog::at(&workdir.join(".roko/learn/costs.jsonl"));
    let days = costs.daily_cost(31).await.unwrap_or_default();
    let unpriced_calls = costs.unpriced_calls().await.unwrap_or(0);
    let month = chrono::Utc::now().format("%Y-%m").to_string();
    WorkspaceStatus {
        project: workdir.file_name().map_or_else(
            || workdir.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        ),
        runs,
        plans: plan_dirs.len(),
        today_usd: days.last().map_or(0.0, |(_, usd)| *usd),
        month_usd: days
            .iter()
            .filter(|(day, _)| day.starts_with(&month))
            .map(|(_, usd)| usd)
            .sum(),
        unpriced_calls,
    }
}

/// `path` as a person types it in `workdir`.
fn shown(workdir: &Path, path: &Path) -> String {
    path.strip_prefix(workdir)
        .map_or_else(|_| path.to_path_buf(), PathBuf::from)
        .display()
        .to_string()
}

/// When something happened, as the design system says it: `just now`,
/// `12 min ago`, `today 14:05`, `yesterday 09:12`, `12 Nov`.
#[must_use]
pub fn ago(at_ms: i64, now_ms: i64) -> String {
    use chrono::{Local, TimeZone};
    let elapsed = Duration::from_millis(u64::try_from(now_ms - at_ms).unwrap_or(0));
    if elapsed.as_secs() < 60 {
        return "just now".to_string();
    }
    if elapsed.as_secs() < 3600 {
        return format!("{} min ago", elapsed.as_secs() / 60);
    }
    let (Some(at), Some(now)) = (
        Local.timestamp_millis_opt(at_ms).single(),
        Local.timestamp_millis_opt(now_ms).single(),
    ) else {
        return String::new();
    };
    if at.date_naive() == now.date_naive() {
        format!("today {}", at.format("%H:%M"))
    } else if now.date_naive().pred_opt() == Some(at.date_naive()) {
        format!("yesterday {}", at.format("%H:%M"))
    } else {
        at.format("%-d %b").to_string()
    }
}

/// The one thing to do next, from the newest run that wants something: a
/// failed run's diagnosis, a stopped run's resume, or delivered work's merge.
#[must_use]
pub fn next_step(status: &WorkspaceStatus) -> Option<String> {
    status.runs.iter().find_map(|run| match run.state {
        RunState::Failed => Some(format!("roko diagnose {}", run.plan_id)),
        RunState::Interrupted | RunState::Stopped => {
            Some(format!("roko run {} --resume-plan", run.dir))
        }
        RunState::Passed | RunState::NotChecked => run.merge_command.clone(),
        RunState::Running => None,
    })
}

/// What `roko status` prints.
#[must_use]
pub fn lines(status: &WorkspaceStatus, style: Style, now_ms: i64) -> Vec<String> {
    let dot = format!(" {} ", style.glyph(Glyph::Dot));
    let mut facts = vec![term_style::plural(status.plans, "plan", "plans")];
    facts.push(format!("{} today", term_style::money(status.today_usd)));
    facts.push(format!(
        "{} this month",
        term_style::money(status.month_usd)
    ));
    let mut out = vec![
        format!(
            "{} {}  status {}",
            style.paint(Role::Mark, style.glyph(Glyph::Mark)),
            style.bold("roko"),
            style.bold(&status.project)
        ),
        format!("  {}", style.paint(Role::Dim, facts.join(&dot))),
    ];
    if status.unpriced_calls > 0 {
        out.push(format!(
            "  {}",
            style.paint(
                Role::Dim,
                format!(
                    "{} {} no known price; the spend leaves {} out.",
                    term_style::plural(status.unpriced_calls, "call", "calls"),
                    if status.unpriced_calls == 1 {
                        "has"
                    } else {
                        "have"
                    },
                    if status.unpriced_calls == 1 {
                        "it"
                    } else {
                        "them"
                    }
                )
            )
        ));
    }
    out.push(String::new());
    if status.runs.is_empty() {
        out.push(format!(
            "  {}",
            style.paint(
                Role::Dim,
                "No runs yet. Run a plan with `roko run plans/<plan>`."
            )
        ));
        return out;
    }
    let shown_runs: Vec<&RunEntry> = status.runs.iter().take(SHOWN_RUNS).collect();
    let rows: Vec<(String, String, String, &RunEntry)> = shown_runs
        .iter()
        .map(|run| {
            let (glyph, word) = match run.state {
                RunState::Running => (
                    style.paint(Role::Live, style.glyph(Glyph::Running)),
                    "running",
                ),
                RunState::Passed => (style.paint(Role::Pass, style.glyph(Glyph::Pass)), "done"),
                RunState::NotChecked => (
                    style.paint(Role::Dim, style.glyph(Glyph::NotChecked)),
                    "not checked",
                ),
                RunState::Failed => (style.paint(Role::Fail, style.glyph(Glyph::Fail)), "failed"),
                RunState::Stopped => (
                    style.paint(Role::Dim, style.glyph(Glyph::Stopped)),
                    "stopped",
                ),
                RunState::Interrupted => (
                    style.paint(Role::You, style.glyph(Glyph::Interrupted)),
                    "interrupted",
                ),
            };
            let when = if run.state == RunState::Running {
                word.to_string()
            } else {
                format!("{word} {}", ago(run.at_ms, now_ms))
            };
            let (done, all) = run.tasks;
            let tasks = format!("{done}/{all} {}", if all == 1 { "task" } else { "tasks" });
            (glyph, when, tasks, *run)
        })
        .collect();
    let when_w = rows
        .iter()
        .map(|(_, when, _, _)| when.len())
        .max()
        .unwrap_or(10);
    // The name takes what the other columns leave, at most 28 columns, and
    // a row never reaches the terminal's last column: "  G " + name + "  " +
    // when + "  " + tasks (9) + "  " + money (6) + "  " + "not merged" (10).
    let name_w = shown_runs
        .iter()
        .map(|run| term_style::visible_width(&run.name))
        .max()
        .unwrap_or(8)
        .clamp(8, 28)
        .min(
            style
                .width()
                .saturating_sub(4 + 2 + when_w + 2 + 9 + 2 + 6 + 2 + 10 + 1),
        )
        .max(8);
    for (glyph, when, tasks, run) in &rows {
        let note = if run.merge_command.is_some() {
            style.paint(Role::Dim, "not merged")
        } else {
            String::new()
        };
        let line = format!(
            "  {glyph} {}  {}  {}  {}  {note}",
            term_style::pad(&term_style::truncate(&run.name, name_w), name_w),
            term_style::pad(when, when_w),
            term_style::pad(tasks, 9),
            term_style::pad_left(&term_style::money(run.spent_usd), 6),
        );
        out.push(line.trim_end().to_string());
    }
    if status.runs.len() > SHOWN_RUNS {
        out.push(format!(
            "  {}",
            style.paint(
                Role::Dim,
                format!("and {} earlier", status.runs.len() - SHOWN_RUNS)
            )
        ));
    }
    if let Some(next) = next_step(status) {
        out.extend(term_style::next_lines(style, &next));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_794_480_000_000;

    fn run(
        name: &str,
        state: RunState,
        minutes_ago: i64,
        tasks: (usize, usize),
        usd: f64,
    ) -> RunEntry {
        RunEntry {
            plan_id: name.to_string(),
            name: name.to_string(),
            state,
            at_ms: NOW - minutes_ago * 60_000,
            tasks,
            spent_usd: usd,
            dir: format!("plans/{name}"),
            merge_command: None,
        }
    }

    #[test]
    fn status_says_what_runs_and_how_the_last_runs_ended() {
        let mut greeting = run("greeting", RunState::Passed, 3, (3, 3), 0.4363);
        greeting.merge_command = Some("git merge --ff-only roko/batch/graph-9a438141".into());
        let mut farewell = run("run-1", RunState::Passed, 4, (1, 1), 0.1007);
        farewell.name = "\u{201c}Give greeting.py a farewell(name) function\u{201d}".into();
        let status = WorkspaceStatus {
            project: "greeting-app".into(),
            runs: vec![
                run("broken", RunState::Failed, 2, (0, 1), 0.302),
                greeting,
                farewell,
            ],
            plans: 3,
            today_usd: 0.839,
            month_usd: 0.839,
            unpriced_calls: 0,
        };
        let text = lines(&status, Style::plain(), NOW).join("\n");
        insta::assert_snapshot!("status", text);
        assert_eq!(next_step(&status).as_deref(), Some("roko diagnose broken"));
        // At 80 columns the names give way: no row reaches the last column.
        let narrow = lines(&status, Style::plain().with_live(true).with_width(80), NOW);
        assert!(
            narrow
                .iter()
                .all(|line| term_style::visible_width(line) < 80),
            "{narrow:#?}"
        );
    }

    #[test]
    fn the_next_step_is_the_newest_runs_that_wants_one() {
        let mut passed = run("greeting", RunState::Passed, 3, (3, 3), 0.44);
        passed.merge_command = Some("git merge --ff-only roko/batch/x".into());
        let interrupted = run("long", RunState::Interrupted, 1, (1, 4), 0.2);
        let status = WorkspaceStatus {
            runs: vec![
                run("now", RunState::Running, 0, (0, 2), 0.0),
                interrupted,
                passed,
            ],
            ..WorkspaceStatus::default()
        };
        assert_eq!(
            next_step(&status).as_deref(),
            Some("roko run plans/long --resume-plan")
        );
        let merged = WorkspaceStatus {
            runs: vec![run("done", RunState::Passed, 3, (3, 3), 0.44)],
            ..WorkspaceStatus::default()
        };
        assert_eq!(next_step(&merged), None);
    }

    #[test]
    fn a_workspace_with_no_runs_says_how_to_start() {
        let status = WorkspaceStatus {
            project: "empty".into(),
            ..WorkspaceStatus::default()
        };
        let text = lines(&status, Style::plain(), NOW);
        assert!(
            text.iter().any(|line| line.contains("No runs yet")),
            "{text:#?}"
        );
    }

    #[test]
    fn times_read_as_people_say_them() {
        assert_eq!(ago(NOW - 20_000, NOW), "just now");
        assert_eq!(ago(NOW - 12 * 60_000, NOW), "12 min ago");
    }
}
