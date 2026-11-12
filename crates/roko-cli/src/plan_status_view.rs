//! What `roko plan status <dir>` says (gap-5043c6), in roko's design system:
//! the plan, how its last run ended, when, and what it spent; its tasks as
//! the run's own table shows them, read from the run, so a task that failed
//! reads failed whatever its `tasks.toml` says (bug-5013e8); the plan check;
//! where the work went; and the one thing to do next, as a command to paste.
//! `--json` is unchanged.

use std::path::Path;
use std::time::Instant;

use crate::commands::diagnose::{self, DiagnoseReport};
use crate::graph_checkpoint::{GraphCheckpointStatus, RecordedBatchDelivery};
use crate::graph_execution::run_view::{
    self, PlanCheckRow, PlanTasks, RunHeader, RunModel, TaskState,
};
use crate::term_style::{self, Glyph, Role, Style};

/// What `roko plan status` reports of one plan.
#[derive(Debug)]
pub struct PlanStatus {
    /// The plan's id.
    pub plan_id: String,
    /// Its directory, as the person types it: what `roko run` takes.
    pub dir: String,
    /// Its tasks, id and title, from its `tasks.toml`.
    pub tasks: Vec<(String, String)>,
    /// Its last run, as `roko diagnose` reports it; `None` when it never ran.
    pub run: Option<DiagnoseReport>,
    /// Where the run delivered the work.
    pub delivery: Option<RecordedBatchDelivery>,
    /// The command that takes the delivered work into the checkout, while
    /// the checkout doesn't have it.
    pub merge_command: Option<String>,
}

impl PlanStatus {
    /// Plan `plan_id`, whose `tasks.toml` in `plan_dir` lists `tasks`, and
    /// its last run in `workdir`.
    pub async fn collect(
        workdir: &Path,
        plan_dir: &Path,
        plan_id: &str,
        tasks: Vec<(String, String)>,
    ) -> Self {
        let checkpointed = crate::graph_checkpoint::inspect_canonical_checkpoint(workdir, plan_id)
            .ok()
            .flatten()
            .is_some();
        let run = checkpointed
            .then(|| diagnose::build_report(workdir, plan_id, false).ok())
            .flatten();
        let delivery = crate::graph_checkpoint::recorded_batch_delivery(workdir, plan_id);
        let merge_command = match &delivery {
            Some(delivery) => {
                crate::graph_execution::batch::merge_command(workdir, &delivery.branch).await
            }
            None => None,
        };
        Self {
            plan_id: plan_id.to_string(),
            dir: plan_dir
                .strip_prefix(workdir)
                .unwrap_or(plan_dir)
                .display()
                .to_string(),
            tasks,
            run,
            delivery,
            merge_command,
        }
    }

    fn status(&self) -> Option<GraphCheckpointStatus> {
        self.run
            .as_ref()
            .and_then(|run| run.graph_run.as_ref())
            .map(|run| run.checkpoint_status)
    }

    /// The task table: the run's, or every task waiting for a plan that
    /// never ran, with the plan check the delivery ran.
    fn model(&self, now: Instant) -> RunModel {
        let mut model = match &self.run {
            Some(run) if !run.tasks.is_empty() => diagnose::run_model(run, now),
            _ => RunModel::new(
                RunHeader {
                    plans: vec![PlanTasks {
                        plan_id: self.plan_id.clone(),
                        tasks: self.tasks.clone(),
                    }],
                    parallel: 1,
                    ..RunHeader::default()
                },
                now,
            ),
        };
        let checks: Vec<&crate::graph_execution::delivery::DeliveryCheck> = self
            .delivery
            .iter()
            .flat_map(|delivery| &delivery.checks)
            .collect();
        if model.plan_checks.is_empty() && !checks.is_empty() {
            let commands: Vec<&str> = checks.iter().map(|check| check.command.as_str()).collect();
            let elapsed = checks
                .iter()
                .map(|check| std::time::Duration::from_millis(check.duration_ms))
                .sum();
            model.plan_checks.push(PlanCheckRow::recorded(
                &self.plan_id,
                "plan check",
                &commands.join(" && "),
                TaskState::Passed,
                None,
                Some(elapsed),
            ));
        }
        model
    }

    /// The one thing to do next: diagnose a failure, carry on after a stop,
    /// merge delivered work, or run a plan that never ran.
    #[must_use]
    pub fn next_step(&self) -> Option<String> {
        match self.status() {
            None => Some(format!("roko run {}", self.dir)),
            Some(GraphCheckpointStatus::Failed) => Some(format!("roko diagnose {}", self.plan_id)),
            Some(GraphCheckpointStatus::Interrupted | GraphCheckpointStatus::Cancelled) => {
                Some(format!("roko run {} --resume-plan", self.dir))
            }
            Some(GraphCheckpointStatus::Succeeded | GraphCheckpointStatus::Unverified) => {
                self.merge_command.clone()
            }
            Some(GraphCheckpointStatus::Running) => Some("roko dashboard".to_string()),
        }
    }
}

/// What `roko plan status` prints, at `now_ms`.
#[must_use]
pub fn lines(status: &PlanStatus, style: Style, now_ms: i64) -> Vec<String> {
    let dot = format!(" {} ", style.glyph(Glyph::Dot));
    let mut facts = vec![status.dir.clone()];
    match status.run.as_ref().and_then(|run| run.graph_run.as_ref()) {
        Some(run) => {
            let state = match run.checkpoint_status {
                GraphCheckpointStatus::Running => "running",
                GraphCheckpointStatus::Succeeded => "done",
                GraphCheckpointStatus::Unverified => "not checked",
                GraphCheckpointStatus::Failed => "failed",
                GraphCheckpointStatus::Cancelled => "stopped",
                GraphCheckpointStatus::Interrupted => "interrupted",
            };
            let at = run
                .updated_at
                .as_deref()
                .and_then(|at| chrono::DateTime::parse_from_rfc3339(at).ok())
                .map(|at| at.timestamp_millis());
            facts.push(match at {
                Some(at) if run.checkpoint_status != GraphCheckpointStatus::Running => {
                    format!("{state} {}", crate::status_view::ago(at, now_ms))
                }
                _ => state.to_string(),
            });
            if let Some(run_state) = status.run.as_ref().and_then(|run| run.run_state.as_ref()) {
                let (done, all) = (run_state.tasks_completed, run_state.tasks_total);
                facts.push(format!(
                    "{done}/{all} {}",
                    if all == 1 { "task" } else { "tasks" }
                ));
            }
            if let Some(spent) = run.spent_usd {
                facts.push(term_style::money(spent));
            }
        }
        None => {
            facts.push("not run yet".to_string());
            facts.push(term_style::plural(status.tasks.len(), "task", "tasks"));
        }
    }
    let title = format!("plan {}", style.bold(&status.plan_id));
    let mut out = term_style::header(style, &title, &facts.join(&dot));
    let now = Instant::now();
    out.extend(run_view::table_lines(
        &status.model(now),
        style,
        now,
        0,
        false,
    ));
    if status.tasks.is_empty() {
        out.push(format!("  {}", style.paint(Role::Dim, "It has no tasks.")));
    }
    if let Some(delivery) = &status.delivery {
        // Next names the branch when there is work to merge.
        let mut said = match &status.merge_command {
            Some(_) => "The work is on its own branch, not merged yet.".to_string(),
            None => format!("The work was delivered on {}.", delivery.branch),
        };
        if delivery.partial {
            said.push_str(" Only the tasks that passed are in it.");
        }
        out.push(String::new());
        let lines = if style.live() {
            term_style::wrap(&said, style.width().saturating_sub(3))
        } else {
            vec![said]
        };
        for line in lines {
            out.push(format!("  {}", style.paint(Role::Dim, line)));
        }
    }
    if let Some(next) = status.next_step() {
        out.extend(term_style::next_lines(style, &next));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use roko_core::{Body, Kind, Signal};
    use roko_fs::RokoLayout;
    use roko_graph::cells::task_executor::TaskGateVerdict;
    use roko_learn::telemetry::{
        AttemptIdentity, AttemptKey, AttemptOutcome, AttemptVerdictRecord, TelemetryWriter,
        TelemetryWriterConfig, VerifyStepVerdict,
    };

    use crate::graph_checkpoint::start_plan_checkpoint;
    use crate::runner::plan_loader::Plan;
    use crate::task_parser::TasksFile;

    const TASKS: &str = r#"
[meta]
plan = "demo"

[[task]]
id = "T1"
title = "Write the parser"
role = "implementer"
files = ["parser.py"]
depends_on = []
verify = [{ phase = "imports", command = "python3 -c 'import parser'" }]

[[task]]
id = "T2"
title = "Wire the parser"
role = "implementer"
files = ["cli.py"]
depends_on = ["T1"]
verify = [{ phase = "runs", command = "python3 cli.py" }]

[[task]]
id = "T3"
title = "Document the parser"
role = "implementer"
files = ["README.md"]
depends_on = ["T2"]
verify = [{ phase = "usage", command = "grep -q parser README.md" }]
"#;

    /// A run of plan `demo` that ended `status`: T1 passed on one try; T2
    /// failed check `runs` on each of its `t2_tries`; T3 never ran.
    fn demo_run(status: GraphCheckpointStatus, t2_tries: u32) -> tempfile::TempDir {
        let workspace = tempfile::tempdir().expect("tempdir");
        let root = workspace.path();
        let dir = root.join("plans/demo");
        std::fs::create_dir_all(&dir).expect("plan dir");
        std::fs::write(dir.join("tasks.toml"), TASKS).expect("tasks.toml");
        let plan = Plan {
            id: "demo".to_string(),
            tasks: TasksFile::parse(&dir.join("tasks.toml")).expect("parse tasks.toml"),
            dir,
        };
        let mut checkpoint = start_plan_checkpoint(root, &plan).expect("checkpoint");
        let mut output = vec![
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("done"))
                .build(),
        ];
        TaskGateVerdict::Passed.stamp(&mut output);
        checkpoint
            .take_recorder()
            .record("demo", "T1", 0, output)
            .expect("record T1");
        checkpoint
            .take_cost_ledger()
            .persist(410_000, 0, Vec::new())
            .expect("cost ledger");
        let run_id = checkpoint.run_id().to_string();
        checkpoint.finish_with_status(status).expect("finish");

        let verdict = |task: &str, attempt: u32, outcome, failed: Option<&str>| {
            let key = AttemptKey::new(run_id.as_str(), "demo", task, attempt);
            let mut verdict =
                AttemptVerdictRecord::settle(AttemptIdentity::new(&key), outcome, true);
            let started = 1_794_476_000_000 + i64::from(attempt) * 10_000;
            verdict.executed.model_dispatched = Some("glm-4.7".to_string());
            verdict.timing.attempt_started_at = Some(started);
            verdict.timing.settled_at = Some(started + 4_500);
            verdict.cost.billed_usd = Some(0.1);
            verdict.steps = failed
                .map(|rung| VerifyStepVerdict {
                    rung: rung.to_string(),
                    passed: Some(false),
                    exit_code: Some(1),
                    ..VerifyStepVerdict::default()
                })
                .into_iter()
                .collect();
            verdict
        };
        let writer = TelemetryWriter::spawn(
            &RokoLayout::for_project(root).run_dir(&run_id),
            TelemetryWriterConfig::default(),
        )
        .expect("spawn writer");
        assert!(writer.submit(verdict("T1", 1, AttemptOutcome::Passed, None)));
        for attempt in 1..=t2_tries {
            let failed = Some("custom:runs");
            assert!(writer.submit(verdict("T2", attempt, AttemptOutcome::GateFailed, failed)));
        }
        assert_eq!(writer.close().written, 1 + u64::from(t2_tries));
        workspace
    }

    async fn status_of(root: &Path) -> PlanStatus {
        let tasks = TasksFile::parse(&root.join("plans/demo/tasks.toml"))
            .expect("parse tasks.toml")
            .tasks
            .iter()
            .map(|task| (task.id.clone(), task.title.clone()))
            .collect();
        PlanStatus::collect(root, &root.join("plans/demo"), "demo", tasks).await
    }

    fn text_of(status: &PlanStatus) -> String {
        let at = status
            .run
            .as_ref()
            .and_then(|run| run.graph_run.as_ref())
            .and_then(|run| run.updated_at.as_deref())
            .and_then(|at| chrono::DateTime::parse_from_rfc3339(at).ok())
            .map_or(0, |at| at.timestamp_millis());
        lines(status, Style::plain(), at + 180_000).join("\n")
    }

    /// bug-5013e8: each task's row says what the run did with it, whatever
    /// its tasks.toml says: a task that failed reads failed, with its tries
    /// and the check it failed; a task that passed reads passed, also in a
    /// run that stopped later; one that waited on a failure did not run.
    /// Next diagnoses a failure and carries on after a stop. A plan that
    /// never ran waits, and Next runs it.
    #[tokio::test]
    async fn plan_status_rows_say_what_the_run_did() {
        let workspace = demo_run(GraphCheckpointStatus::Failed, 2);
        let text = text_of(&status_of(workspace.path()).await);
        assert!(text.contains("\n  ✓ task 1  Write the parser "), "{text}");
        assert!(text.contains("\n  ✗ task 2  Wire the parser "), "{text}");
        assert!(text.contains("check runs failed"), "{text}");
        assert!(
            text.contains("\n  ⊘ task 3  Document the parser "),
            "{text}"
        );
        assert!(!text.contains("ready"), "{text}");
        insta::assert_snapshot!("plan_status_failed", text);

        let workspace = demo_run(GraphCheckpointStatus::Interrupted, 1);
        let text = text_of(&status_of(workspace.path()).await);
        assert!(text.contains("\n  ✓ task 1  Write the parser "), "{text}");
        assert!(text.contains("\n  ■ task 2  Wire the parser "), "{text}");
        assert!(
            text.contains("\n  ○ task 3  Document the parser "),
            "{text}"
        );
        assert!(
            text.ends_with("\n  Next  roko run plans/demo --resume-plan"),
            "{text}"
        );

        let workspace = tempfile::tempdir().expect("tempdir");
        let dir = workspace.path().join("plans/demo");
        std::fs::create_dir_all(&dir).expect("plan dir");
        std::fs::write(dir.join("tasks.toml"), TASKS).expect("tasks.toml");
        let text = text_of(&status_of(workspace.path()).await);
        assert!(
            text.contains("plans/demo · not run yet · 3 tasks"),
            "{text}"
        );
        assert_eq!(text.matches("waiting").count(), 3, "{text}");
        assert!(text.ends_with("\n  Next  roko run plans/demo"), "{text}");
    }
}
