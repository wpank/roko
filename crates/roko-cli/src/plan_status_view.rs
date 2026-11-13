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

    /// The run's four readings by the rule every surface shares, the design
    /// system's §4.8 (gap-9bfb4e): its tasks, TIME, SPENT of its cap, and
    /// CHECKS, `2/5 tasks · 4m 05s · $0.84 of $3.00 · 7/14 checks`. `None`
    /// for a plan that never ran.
    fn readings(&self, style: Style) -> Option<String> {
        let report = self.run.as_ref()?;
        let run = report.graph_run.as_ref()?;
        let readings = report.readings.as_ref();
        let mut parts = Vec::new();
        if let Some(state) = &report.run_state {
            let (done, all) = (state.tasks_completed, state.tasks_total);
            parts.push(format!(
                "{} {}",
                style.bold(format!("{done}/{all}")),
                if all == 1 { "task" } else { "tasks" }
            ));
        }
        if let Some(time) = readings.and_then(|readings| readings.time) {
            parts.push(style.bold(run_time(time)));
        }
        if let Some(spent) = readings
            .and_then(|readings| readings.spent_usd)
            .or(run.spent_usd)
        {
            let money = style.bold(term_style::money(spent));
            parts.push(match readings.and_then(|readings| readings.cap_usd) {
                Some(cap) => format!("{money} of {}", term_style::money(cap)),
                None => money,
            });
        }
        if let Some(readings) = readings {
            // The plan's own checks count once its check reports; a plan
            // with none written has the one its delivery ran.
            let checks = self
                .delivery
                .as_ref()
                .map_or(&[][..], |delivery| delivery.checks.as_slice());
            let own = if checks.is_empty() {
                readings.plan_checks
            } else {
                readings.plan_checks.max(1)
            };
            let all = readings.checks_total - readings.plan_checks + own;
            let own_passed =
                !checks.is_empty() && checks.iter().all(|check| check.exit_code == Some(0));
            let passed = readings.checks_passed + if own_passed { own } else { 0 };
            if all > 0 {
                parts.push(format!(
                    "{} {}",
                    style.bold(format!("{passed}/{all}")),
                    if all == 1 { "check" } else { "checks" }
                ));
            }
        }
        let dot = format!(" {} ", style.glyph(Glyph::Dot));
        (!parts.is_empty()).then(|| parts.join(&dot))
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

/// A run's TIME in whole seconds, rounded down (gap-9bfb4e): `42s`, then
/// `4m 05s` and `1h 12m`.
fn run_time(elapsed: std::time::Duration) -> String {
    let secs = elapsed.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else {
        term_style::duration(std::time::Duration::from_secs(secs))
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
        }
        None => {
            facts.push("not run yet".to_string());
            facts.push(term_style::plural(status.tasks.len(), "task", "tasks"));
        }
    }
    let title = format!("plan {}", style.bold(&status.plan_id));
    let mut out = term_style::header(style, &title, &facts.join(&dot));
    // The run's four readings, under the header, as the glass shows them
    // (gap-9bfb4e).
    if let Some(readings) = status.readings(style) {
        out.insert(2, format!("  {readings}"));
    }
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
        AttemptIdentity, AttemptKey, AttemptOpenRecord, AttemptOutcome, AttemptVerdictRecord,
        TelemetryWriter, TelemetryWriterConfig, VerifyStepVerdict,
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

    /// A run of plan `demo` that is still going, holding the runner lock as a
    /// live run does: each task in `passed` passed on one try. Returns the
    /// run's id and the lock.
    fn running_run(
        passed: &[&str],
    ) -> (
        tempfile::TempDir,
        String,
        crate::workspace_lock::WorkspaceLockGuard,
    ) {
        run_of(passed, GraphCheckpointStatus::Running)
    }

    /// [`running_run`], its checkpoint left `status`, with $0.15 committed.
    fn run_of(
        passed: &[&str],
        status: GraphCheckpointStatus,
    ) -> (
        tempfile::TempDir,
        String,
        crate::workspace_lock::WorkspaceLockGuard,
    ) {
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
        let mut recorder = checkpoint.take_recorder();
        for task in passed {
            let mut output = vec![
                Signal::builder(Kind::AgentOutput)
                    .body(Body::text("done"))
                    .build(),
            ];
            TaskGateVerdict::Passed.stamp(&mut output);
            recorder.record("demo", task, 0, output).expect("record");
        }
        drop(recorder);
        checkpoint
            .take_cost_ledger()
            .persist(150_000, 0, Vec::new())
            .expect("cost ledger");
        checkpoint
            .finish_with_status(status)
            .expect("commit the checkpoint");
        let run_id = checkpoint.run_id().to_string();
        let writer = TelemetryWriter::spawn(
            &RokoLayout::for_project(root).run_dir(&run_id),
            TelemetryWriterConfig::default(),
        )
        .expect("spawn writer");
        for &task in passed {
            let key = AttemptKey::new(run_id.as_str(), "demo", task, 1);
            let verdict = AttemptVerdictRecord::settle(
                AttemptIdentity::new(&key),
                AttemptOutcome::Passed,
                true,
            );
            assert!(writer.submit(verdict));
        }
        writer.close();
        let lock = crate::workspace_lock::acquire_runner_lock(&root.join(".roko"))
            .expect("the runner lock");
        (workspace, run_id, lock)
    }

    /// bug-addd24: while the run goes, a task whose try started reads
    /// running, with how long it has run, though nothing of the try has
    /// settled; a later try says which it is. Once every task passed, the
    /// plan check the run started reads checking until it finishes.
    #[tokio::test]
    async fn plan_status_shows_what_is_running() {
        let (workspace, run_id, _lock) = running_run(&["T1"]);
        let root = workspace.path();
        let run_dir = RokoLayout::for_project(root).run_dir(&run_id);
        let open = |task: &str, attempt: u32, ago_ms: i64| {
            let key = AttemptKey::new(run_id.as_str(), "demo", task, attempt);
            let started = chrono::Utc::now().timestamp_millis() - ago_ms;
            let writer = TelemetryWriter::spawn(&run_dir, TelemetryWriterConfig::default())
                .expect("spawn writer");
            assert!(writer.submit(AttemptOpenRecord::new(AttemptIdentity::new(&key), started)));
            writer.close();
        };
        open("T2", 1, 40_000);
        let text = text_of(&status_of(root).await);
        assert!(
            text.contains("plans/demo · running\n  1/3 tasks · "),
            "{text}"
        );
        assert!(text.contains("\n  ✓ task 1  Write the parser "), "{text}");
        let row = |text: &str, label: &str| {
            text.lines()
                .find(|line| line.contains(label))
                .map(str::to_string)
                .unwrap_or_else(|| panic!("no row for {label}: {text}"))
        };
        let task2 = row(&text, "task 2");
        assert!(task2.contains("Wire the parser"), "{text}");
        assert!(task2.contains("running · 4"), "{text}");
        assert!(!task2.contains("try"), "{text}");
        let task3 = row(&text, "task 3");
        assert!(task3.contains("○ task 3  Document the parser"), "{text}");
        assert!(task3.ends_with("waiting"), "{text}");
        assert!(!text.contains("plan check"), "{text}");

        // Its first try settled as failed; the second is open.
        let writer = TelemetryWriter::spawn(&run_dir, TelemetryWriterConfig::default())
            .expect("spawn writer");
        let key = AttemptKey::new(run_id.as_str(), "demo", "T2", 1);
        let failed = AttemptVerdictRecord::settle(
            AttemptIdentity::new(&key),
            AttemptOutcome::GateFailed,
            true,
        );
        assert!(writer.submit(failed));
        writer.close();
        open("T2", 2, 5_000);
        let text = text_of(&status_of(root).await);
        assert!(
            row(&text, "task 2").contains("try 2 · running · "),
            "{text}"
        );

        // Every task passed and the plan check runs, then finishes.
        let (workspace, run_id, _lock) = running_run(&["T1", "T2", "T3"]);
        let root = workspace.path();
        let paths = crate::runner::persist::PersistPaths::from_workdir(root).expect("paths");
        let event = |kind: &str| {
            let mut event = serde_json::json!({
                "type": kind,
                "plan_id": "demo",
                "stage": "regression",
                "step": 1,
                "steps": 1,
                "name": "grep",
                "scope": "command",
                "run_id": run_id,
                "ts_millis": chrono::Utc::now().timestamp_millis(),
            });
            if kind == "plan_check_started" {
                event["command"] = serde_json::json!("grep -q parser README.md");
                event["source"] = serde_json::json!("authored");
            } else {
                event["passed"] = serde_json::json!(true);
                event["exit_code"] = serde_json::json!(0);
                event["elapsed_ms"] = serde_json::json!(1_200);
            }
            crate::runner::persist::append_run_index_event(&paths, &run_id, &event, true)
                .expect("append to the run's event index");
        };
        event("plan_check_started");
        let text = text_of(&status_of(root).await);
        assert!(
            text.contains("plans/demo · running\n  3/3 tasks · "),
            "{text}"
        );
        let check = row(&text, "plan check");
        assert!(check.contains("grep -q parser README.md"), "{text}");
        assert!(check.ends_with("checking"), "{text}");

        event("plan_check_finished");
        let text = text_of(&status_of(root).await);
        assert!(!text.contains("checking"), "{text}");
    }

    /// gap-9bfb4e, bug-7a62d9: the header gives the run's four readings as the glass
    /// does, by the rule every surface shares: its tasks; TIME from the run's
    /// start, when roko accepted it; SPENT, which counts what an open try has
    /// spent so far (its latest call), of the run's cap; and CHECKS, every
    /// check the plan has, fixed from the start, of which those passed. Once
    /// the run ends, TIME is its `run_completed.duration_ms`.
    #[tokio::test]
    async fn plan_status_reads_like_the_glass() {
        let (workspace, run_id, _lock) = running_run(&["T1"]);
        let root = workspace.path();
        let run_dir = RokoLayout::for_project(root).run_dir(&run_id);
        let now = chrono::Utc::now().timestamp_millis();
        // The run's manifest says what it may spend.
        let manifest = serde_json::json!({
            "schema_version": "roko.run_manifest/1",
            "run_id": run_id,
            "kind": "plan_run",
            "plan_ids": ["demo"],
            "invocations": [{
                "inv": 1,
                "started_at": chrono::Utc::now().to_rfc3339(),
                "pid": 0,
                "host": "elsewhere",
                "cap_micro_usd": 3_000_000,
            }],
        });
        std::fs::write(run_dir.join("manifest.json"), manifest.to_string()).expect("manifest");
        // T2's try started 40 s ago; its calls have cost $0.69 so far.
        let key = AttemptKey::new(run_id.as_str(), "demo", "T2", 1);
        let writer = TelemetryWriter::spawn(&run_dir, TelemetryWriterConfig::default())
            .expect("spawn writer");
        assert!(writer.submit(AttemptOpenRecord::new(
            AttemptIdentity::new(&key),
            now - 40_000
        )));
        writer.close();
        let paths = crate::runner::persist::PersistPaths::from_workdir(root).expect("paths");
        let index = |event: serde_json::Value| {
            crate::runner::persist::append_run_index_event(&paths, &run_id, &event, true)
                .expect("append to the run's event index");
        };
        index(serde_json::json!({
            "type": "plan_set_loaded", "plans": [], "run_id": run_id, "ts_millis": now - 125_400,
        }));
        for (call, cost) in [(1, 0.31), (2, 0.69)] {
            index(serde_json::json!({
                "type": "call_cost", "plan_id": "demo", "task_id": "T2", "attempt": 1,
                "call": call, "cost_usd": 0.3, "attempt_cost_usd": cost, "run_id": run_id,
                "ts_millis": now - 10_000,
            }));
        }

        let text = text_of(&status_of(root).await);
        let readings = text.lines().nth(2).expect("the readings line");
        assert!(readings.starts_with("  1/3 tasks · 2m 0"), "{text}");
        assert!(readings.contains(" · $0.84 of $3.00 · "), "{text}");
        assert!(readings.ends_with(" · 1/3 checks"), "{text}");

        // Every task passed and the run ended: TIME is the run's own.
        let (workspace, run_id, _lock) =
            run_of(&["T1", "T2", "T3"], GraphCheckpointStatus::Succeeded);
        let root = workspace.path();
        let paths = crate::runner::persist::PersistPaths::from_workdir(root).expect("paths");
        crate::runner::persist::append_run_index_event(
            &paths,
            &run_id,
            &serde_json::json!({
                "type": "run_completed", "duration_ms": 379_014, "outcome": "succeeded",
                "run_id": run_id, "ts_millis": now,
            }),
            true,
        )
        .expect("append to the run's event index");
        let text = text_of(&status_of(root).await);
        let readings = text.lines().nth(2).expect("the readings line");
        assert_eq!(
            readings, "  3/3 tasks · 6m 19s · $0.15 · 3/3 checks",
            "{text}"
        );
    }

    /// The records roko keeps of writing plan `demo` before it ran: the
    /// planner call's spend, under the pseudo task `generate`, and its
    /// episode, `plan:generate:demo`.
    async fn written_by_roko(root: &Path) {
        let spend = crate::plan_authoring::AuthoringSpend::generation(root, "demo", None);
        spend
            .record(&crate::agent_exec::AgentCapture {
                exit_code: 0,
                output: String::new(),
                usage: roko_core::Usage {
                    input_tokens: 48_000,
                    output_tokens: 4_000,
                    cost_usd: 0.22,
                    ..roko_core::Usage::default()
                },
                api_equiv_usd: None,
                price_snapshot_id: None,
                model: "gpt-5.4-mini".to_string(),
                provider: "openai".to_string(),
                duration_ms: 90_000,
            })
            .await;
        let (episode, _) = crate::agent_episode::build_capture_episode(
            "claude",
            Some("gpt-5.4-mini"),
            "plan-generate",
            "plan:generate:demo",
            "the request",
            "the plan",
            true,
            chrono::Utc::now() - chrono::Duration::minutes(6),
            90_000,
            None,
        );
        let episodes = RokoLayout::for_project(root).episodes_path();
        let line = serde_json::to_string(&episode).expect("episode JSON");
        std::fs::write(&episodes, format!("{line}\n")).expect("episodes.jsonl");
    }

    /// bug-2e5023: the records of a plan's writing are not tasks. The status
    /// of a plan roko wrote lists its three tasks and nothing else, and the
    /// planner call is not one of the run's calls.
    #[tokio::test]
    async fn plan_status_lists_only_the_plans_tasks() {
        let workspace = demo_run(GraphCheckpointStatus::Interrupted, 1);
        written_by_roko(workspace.path()).await;
        let status = status_of(workspace.path()).await;
        let text = text_of(&status);
        assert!(text.contains(" task 3  Document the parser "), "{text}");
        assert!(!text.contains("task 4"), "{text}");
        assert!(!text.contains("not in tasks.toml"), "{text}");
        let run = status.run.as_ref().expect("the run's report");
        let ids: Vec<&str> = run.tasks.iter().map(|task| task.task_id.as_str()).collect();
        assert_eq!(ids, ["T1", "T2", "T3"]);
        let run_state = run.run_state.as_ref().expect("the run's state");
        assert_eq!(run_state.total_agent_calls, 0, "{run_state:?}");
        assert_eq!(run_state.total_tokens_in, 0, "{run_state:?}");
    }

    /// What a write would change: each file's bytes, and each file's and
    /// directory's inode and modified time.
    type Entry = (Option<Vec<u8>>, u64, std::time::SystemTime);

    /// Every file and directory under `root`, `.git` included. A file
    /// rewritten with the same bytes has a new inode or time; a file created
    /// or removed changes its directory's time.
    fn files_under(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Entry> {
        use std::os::unix::fs::MetadataExt;

        let mut entries = std::collections::BTreeMap::new();
        let mut dirs = vec![root.to_path_buf()];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).expect("read dir") {
                let path = entry.expect("dir entry").path();
                let metadata = std::fs::symlink_metadata(&path).expect("metadata");
                let modified = metadata.modified().expect("modified time");
                let bytes = if metadata.is_dir() {
                    dirs.push(path.clone());
                    None
                } else {
                    Some(std::fs::read(&path).expect("read file"))
                };
                entries.insert(path, (bytes, metadata.ino(), modified));
            }
        }
        entries
    }

    /// bug-62760a: `roko plan status` only reads (bug-399240's rule: a view
    /// never writes). The checkout's index holds stale stat data for
    /// `tasks.toml`, which `git status` would refresh and write back; every
    /// file under the workspace, `.git/index` included, keeps its bytes, its
    /// inode and its time, and no file comes or goes.
    #[tokio::test]
    async fn plan_status_writes_nothing() {
        let workspace = demo_run(GraphCheckpointStatus::Interrupted, 1);
        let root = workspace.path();
        let git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
                .args([
                    "-c",
                    "commit.gpgsign=false",
                    "-c",
                    "core.hooksPath=/dev/null",
                ])
                .args(args)
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .env_remove("GIT_INDEX_FILE")
                .env_remove("GIT_OPTIONAL_LOCKS")
                .status()
                .expect("git");
            assert!(status.success(), "git {args:?}");
        };
        git(&["init", "-q"]);
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "the plan"]);
        // The file's time moves and its bytes stay.
        std::fs::File::options()
            .write(true)
            .open(root.join("plans/demo/tasks.toml"))
            .expect("open tasks.toml")
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000))
            .expect("set its time");
        let before = files_under(root);

        let status = status_of(root).await;
        assert!(text_of(&status).contains("task 3"));

        let after = files_under(root);
        let changed: Vec<String> = after
            .iter()
            .filter(|(path, entry)| before.get(*path) != Some(*entry))
            .map(|(path, _)| path)
            .chain(before.keys().filter(|path| !after.contains_key(*path)))
            .map(|path| {
                path.strip_prefix(root)
                    .unwrap_or(path)
                    .display()
                    .to_string()
            })
            .collect();
        assert!(changed.is_empty(), "plan status wrote {changed:?}");
    }
}
