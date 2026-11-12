//! What `roko run --plan "<request>"` shows while it writes the plan
//! (gap-3436e7), in roko's design system, before the run view (gap-2c87f0)
//! takes over:
//! - a header with the request, the planner's model and the cap;
//! - on a terminal, one live line that says what the writer is doing, over
//!   its time and spend on glass; in a pipe or CI, a line each time it moves
//!   on;
//! - then the plan written, its tasks as `task N`, and what to do next.
//!
//! The plan generator prints nothing of its own meanwhile
//! (`PlanRequest::quiet`).

use std::collections::BTreeMap;
use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::graph_execution::run_view::LiveRegion;
use crate::plan_generate::{PlanFailsValidation, PlanProgress, PlanProgressSink};
use crate::task_parser::{TaskQualityWarning, TasksFile};
use crate::term_style::{self, Glyph, Role, Style};

/// What the header says.
#[derive(Clone, Debug, Default)]
pub struct WriteHeader {
    /// The person's request, as they wrote it.
    pub request: String,
    /// The model that writes the plan, by its plain name.
    pub model: Option<String>,
    /// What writing it may spend.
    pub cap_usd: Option<f64>,
    /// The plain names of the models the writer may call, by their keys in
    /// `roko.toml`: whom the live line says it is asking.
    pub model_names: BTreeMap<String, String>,
}

impl WriteHeader {
    /// `◆ roko  run “<request>”`, then a faint line: writing a plan first,
    /// the model, the cap. Appended output gets both on one line.
    #[must_use]
    pub fn lines(&self, style: Style) -> Vec<String> {
        let dot = format!(" {} ", style.glyph(Glyph::Dot));
        let mut facts = vec!["writing a plan first".to_string()];
        facts.extend(self.model.clone());
        if let Some(cap) = self.cap_usd.filter(|cap| *cap > 0.0) {
            facts.push(format!("cap {}", term_style::money(cap)));
        }
        if style.live() {
            // "◆ roko  run “" and "”": the request takes what is left.
            let room = style.width().saturating_sub(15);
            let title = format!(
                "run \u{201c}{}\u{201d}",
                term_style::truncate(&self.request, room)
            );
            term_style::header(style, &title, &facts.join(&dot))
        } else {
            vec![format!(
                "{} {}  run \u{201c}{}\u{201d}{dot}{}",
                style.paint(Role::Mark, style.glyph(Glyph::Mark)),
                style.bold("roko"),
                self.request,
                facts.join(&dot)
            )]
        }
    }
}

/// What the writer is doing, in the design system's words; `names` gives a
/// model's plain name by its key.
#[must_use]
pub fn doing(progress: &PlanProgress, names: &BTreeMap<String, String>) -> String {
    let model = names.get(&progress.model).unwrap_or(&progress.model);
    match progress.stage {
        "planning" => format!("asking {model}"),
        "retrying" => format!("asking {model} again"),
        "fixing" => "fixing the plan".to_string(),
        "regenerating" => "making the plan better".to_string(),
        "writing" => "saving the plan".to_string(),
        "recording" => "noting what it learned".to_string(),
        other => other.replace('_', " "),
    }
}

/// The live lines while the plan is written: what the writer is doing (and,
/// faint under it, why it is doing it again), then the time and spend on
/// glass.
#[must_use]
pub fn live_lines(
    progress: Option<&PlanProgress>,
    names: &BTreeMap<String, String>,
    elapsed: Duration,
    cap_usd: Option<f64>,
    style: Style,
    frame: usize,
) -> Vec<String> {
    let head = format!(
        "  {} {}  ",
        style.paint(Role::Live, style.spinner(frame)),
        style.bold("Writing the plan")
    );
    let room = style
        .width()
        .saturating_sub(1 + term_style::visible_width(&head));
    let what = progress.map_or_else(|| "starting".to_string(), |progress| doing(progress, names));
    let mut lines = vec![format!(
        "{head}{}",
        style.paint(Role::Live, term_style::truncate(&what, room))
    )];
    if let Some(reason) = progress.and_then(|progress| progress.reason.as_deref()) {
        let room = style.width().saturating_sub(5);
        let reason = term_style::truncate(&first_line(reason), room);
        lines.push(format!("    {}", style.paint(Role::Dim, reason)));
    }
    let spent = progress.map_or(0.0, |progress| progress.cost_usd);
    let mut footer = format!(
        "  {} {}",
        style.glass(term_style::clock(elapsed)),
        style.glass(term_style::money(spent))
    );
    if let Some(cap) = cap_usd.filter(|cap| *cap > 0.0) {
        footer.push_str(&style.paint(Role::Dim, format!(" of {}", term_style::money(cap))));
    }
    lines.push(String::new());
    lines.push(footer);
    lines
}

/// The line appended output prints each time the writer moves on.
#[must_use]
pub fn progress_line(
    progress: &PlanProgress,
    names: &BTreeMap<String, String>,
    style: Style,
) -> String {
    let dot = format!(" {} ", style.glyph(Glyph::Dot));
    let mut line = format!(
        "  {} writing the plan{dot}{}",
        style.paint(Role::Live, style.glyph(Glyph::Running)),
        doing(progress, names)
    );
    if let Some(reason) = &progress.reason {
        line.push_str(&dot);
        line.push_str(&first_line(reason));
    }
    line
}

/// One task of the plan written: its number, title, and the tasks it waits
/// on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrittenTask {
    /// Its number in the plan, from 1.
    pub number: usize,
    /// Its title.
    pub title: String,
    /// The numbers of the tasks it runs after.
    pub after: Vec<usize>,
}

/// The plan as it was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrittenPlan {
    /// Its directory, as the person types it.
    pub dir: String,
    /// Its tasks, in plan order.
    pub tasks: Vec<WrittenTask>,
    /// Its checks: every task's verify steps.
    pub checks: usize,
    /// Whether it has a plan check (`[meta] verify`).
    pub plan_check: bool,
    /// What roko notices about it, in plain words, faint.
    pub notes: Vec<String>,
}

impl WrittenPlan {
    /// The plan `tasks`, written in `dir`.
    #[must_use]
    pub fn of(tasks: &TasksFile, dir: String) -> Self {
        let number = |id: &str| {
            tasks
                .tasks
                .iter()
                .position(|task| task.id == id)
                .map(|at| at + 1)
        };
        let label = |id: &str| number(id).map_or_else(|| id.to_string(), |n| format!("task {n}"));
        let labels = |ids: &[String]| {
            term_style::listed(&ids.iter().map(|id| label(id)).collect::<Vec<_>>())
        };
        let warnings = tasks.quality_warnings();
        let mut without_reads = Vec::new();
        let mut without_checks = Vec::new();
        let mut notes = Vec::new();
        for warning in &warnings {
            match warning {
                TaskQualityWarning::MissingReadFiles { task_id } => {
                    without_reads.push(task_id.clone());
                }
                TaskQualityWarning::MissingVerify { task_id } => {
                    without_checks.push(task_id.clone());
                }
                TaskQualityWarning::LongDescription {
                    task_id,
                    word_count,
                } => notes.push(format!(
                    "{}'s description runs to {word_count} words; a shorter one is easier to follow.",
                    label(task_id)
                )),
                TaskQualityWarning::TooManyTasks { task_count } => notes.push(format!(
                    "It has {task_count} tasks; more than 20 is hard to follow in one plan."
                )),
                TaskQualityWarning::UnusedHints { task_id, fields } => notes.push(format!(
                    "{} sets {}, which roko doesn't act on yet.",
                    label(task_id),
                    fields.join(", ")
                )),
            }
        }
        if !without_reads.is_empty() {
            let (name, verb) = if without_reads.len() == 1 {
                ("its agent starts", "names")
            } else {
                ("their agents start", "name")
            };
            notes.insert(
                0,
                format!(
                    "{} {verb} no files to read first, so {name} without that context.",
                    capitalized(&labels(&without_reads))
                ),
            );
        }
        if !without_checks.is_empty() {
            notes.push(format!(
                "{} has no check.",
                capitalized(&labels(&without_checks))
            ));
        }
        Self {
            dir,
            tasks: tasks
                .tasks
                .iter()
                .enumerate()
                .map(|(index, task)| WrittenTask {
                    number: index + 1,
                    title: task.title.clone(),
                    after: task.depends_on.iter().filter_map(|id| number(id)).collect(),
                })
                .collect(),
            checks: tasks.tasks.iter().map(|task| task.verify.len()).sum(),
            plan_check: !tasks.meta.verify.is_empty(),
            notes,
        }
    }
}

/// The lines the writing ends with when the plan was written: what it is,
/// its tasks as the run's table will show them, waiting, and the notes.
#[must_use]
pub fn written_lines(
    plan: &WrittenPlan,
    elapsed: Duration,
    spent_usd: f64,
    style: Style,
) -> Vec<String> {
    let dot = format!(" {} ", style.glyph(Glyph::Dot));
    let mut facts = vec![
        term_style::plural(plan.tasks.len(), "task", "tasks"),
        term_style::plural(plan.checks, "check", "checks"),
    ];
    if plan.plan_check {
        facts.push("a plan check".to_string());
    }
    facts.push(term_style::duration(elapsed));
    facts.push(term_style::money(spent_usd));
    let word = format!(
        "  {} {}  ",
        style.paint(Role::Pass, style.glyph(Glyph::Pass)),
        style.paint_bold(Role::Pass, "Plan written")
    );
    let one = format!("{word}{}{dot}{}", plan.dir, facts.join(&dot));
    // On a terminal the facts go under the directory when the line won't fit.
    let mut lines = if style.live() && term_style::visible_width(&one) >= style.width() {
        let room = style
            .width()
            .saturating_sub(1 + term_style::visible_width(&word));
        vec![
            format!("{word}{}", term_style::truncate(&plan.dir, room)),
            format!("  {}", style.paint(Role::Dim, facts.join(&dot))),
        ]
    } else {
        vec![one]
    };
    lines.push(String::new());
    let label_w = plan
        .tasks
        .iter()
        .map(|task| format!("task {}", task.number).len())
        .max()
        .unwrap_or(6);
    let title_w = plan
        .tasks
        .iter()
        .map(|task| term_style::visible_width(&task.title))
        .max()
        .unwrap_or(0);
    let has_after = plan.tasks.iter().any(|task| !task.after.is_empty());
    // "  ○ task N  title  after task M": the title gives way on a terminal.
    let room = if style.live() {
        style
            .width()
            .saturating_sub(1 + 4 + label_w + 2 + if has_after { 2 + 16 } else { 0 })
    } else {
        usize::MAX
    };
    let title_w = title_w.min(room).max(12);
    for task in &plan.tasks {
        let label = term_style::pad(&format!("task {}", task.number), label_w);
        let title = term_style::truncate(&task.title, title_w);
        let after = if task.after.is_empty() {
            String::new()
        } else {
            let after: Vec<String> = task.after.iter().map(|n| format!("task {n}")).collect();
            style.paint(Role::Dim, format!("after {}", term_style::listed(&after)))
        };
        let line = format!(
            "  {} {label}  {}  {after}",
            style.paint(Role::Dim, style.glyph(Glyph::Waiting)),
            term_style::pad(&title, title_w)
        );
        lines.push(line.trim_end().to_string());
    }
    if !plan.notes.is_empty() {
        lines.push(String::new());
        let room = style.width().saturating_sub(3);
        for note in &plan.notes {
            let wrapped = if style.live() {
                term_style::wrap(note, room)
            } else {
                vec![note.clone()]
            };
            for line in wrapped {
                lines.push(format!("  {}", style.paint(Role::Dim, line)));
            }
        }
    }
    lines
}

/// The lines the writing ends with when it failed: one sentence, and what
/// to do next. A plan written that can't run names its first problem, and
/// Next shows them all; anything else says why, and Next asks again.
#[must_use]
pub fn failed_lines(error: &anyhow::Error, request: &str, style: Style) -> Vec<String> {
    let (why, next) = match error.downcast_ref::<PlanFailsValidation>() {
        Some(failed) => {
            let dir = failed.plan_dir.display().to_string();
            let problem = failed
                .errors
                .first()
                .map_or_else(|| "it has problems".to_string(), |first| first_line(first));
            (
                format!("the plan it wrote can't run yet: {problem}"),
                format!("roko plan validate {dir}"),
            )
        }
        None => (
            first_line(&format!("{error:#}")),
            format!("roko run --plan {}", shell_quoted(request)),
        ),
    };
    let sentence = format!("roko couldn't write a plan for this request: {why}");
    let room = style.width().saturating_sub(5);
    let mut lines = Vec::new();
    for (index, line) in if style.live() {
        term_style::wrap(&sentence, room)
    } else {
        vec![sentence]
    }
    .into_iter()
    .enumerate()
    {
        if index == 0 {
            lines.push(format!(
                "  {} {line}",
                style.paint(Role::Fail, style.glyph(Glyph::Fail))
            ));
        } else {
            lines.push(format!("    {line}"));
        }
    }
    lines.extend(term_style::next_lines(style, &next));
    lines
}

/// The question a terminal run asks once the plan is written.
#[must_use]
pub fn question(style: Style) -> String {
    format!(
        "\n  {} {} ",
        style.paint_bold(Role::You, "Run it now?"),
        style.paint(Role::Dim, "[Y/n]")
    )
}

/// The first line of `text`, trimmed.
fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_string()
}

/// `text` with its first letter capital.
fn capitalized(text: &str) -> String {
    let mut characters = text.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(characters).collect()
    })
}

/// `text` as one shell word in double quotes, safe to paste.
fn shell_quoted(text: &str) -> String {
    let mut quoted = String::from("\"");
    for character in text.chars() {
        if matches!(character, '"' | '\\' | '$' | '`') {
            quoted.push('\\');
        }
        quoted.push(character);
    }
    quoted.push('"');
    quoted
}

/// The view of one plan being written, on stderr: the header at once, then
/// on a terminal the live lines, redrawn ten times a second on a thread of
/// its own, or in a pipe a line each time the writer moves on.
pub struct PlanWriteView {
    style: Style,
    cap_usd: Option<f64>,
    names: Arc<BTreeMap<String, String>>,
    started: Instant,
    progress: Arc<Mutex<Option<PlanProgress>>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<Option<LiveRegion<std::io::Stderr>>>>,
}

impl PlanWriteView {
    /// Print `header` and start showing the writing.
    #[must_use]
    pub fn start(header: &WriteHeader, style: Style) -> Self {
        {
            let mut err = std::io::stderr().lock();
            for line in header.lines(style) {
                let _ = writeln!(err, "{line}");
            }
        }
        let progress: Arc<Mutex<Option<PlanProgress>>> = Arc::default();
        let stop = Arc::new(AtomicBool::new(false));
        let started = Instant::now();
        let names = Arc::new(header.model_names.clone());
        let thread = style
            .live()
            .then(|| {
                let (progress, stop, cap) =
                    (Arc::clone(&progress), Arc::clone(&stop), header.cap_usd);
                let names = Arc::clone(&names);
                std::thread::Builder::new()
                    .name("roko-plan-write-view".to_string())
                    .spawn(move || {
                        let mut region = LiveRegion::new(std::io::stderr());
                        let mut frame = 0;
                        while !stop.load(Ordering::SeqCst) {
                            let lines = {
                                let progress = progress.lock().ok();
                                let progress =
                                    progress.as_ref().and_then(|progress| progress.as_ref());
                                live_lines(progress, &names, started.elapsed(), cap, style, frame)
                            };
                            let _ = region.draw(&lines);
                            frame += 1;
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        Some(region)
                    })
                    .map_err(|error| tracing::warn!(%error, "the plan view could not start"))
                    .ok()
            })
            .flatten();
        Self {
            style,
            cap_usd: header.cap_usd,
            names,
            started,
            progress,
            stop,
            thread,
        }
    }

    /// Where the plan generator reports what it is doing.
    #[must_use]
    pub fn sink(&self) -> PlanProgressSink {
        let progress = Arc::clone(&self.progress);
        let names = Arc::clone(&self.names);
        let (style, append) = (self.style, self.thread.is_none());
        Arc::new(move |stage: &PlanProgress| {
            let moved_on = match progress.lock() {
                Ok(mut last) => {
                    let moved_on = last.as_ref().is_none_or(|last| {
                        last.stage != stage.stage || last.attempt != stage.attempt
                    });
                    *last = Some(stage.clone());
                    moved_on
                }
                Err(_) => true,
            };
            if append && moved_on {
                let line = progress_line(stage, &names, style);
                let _ = writeln!(std::io::stderr().lock(), "{line}");
            }
        })
    }

    /// How long the writing has taken.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// What the writing has spent, as the generator last said.
    #[must_use]
    pub fn spent(&self) -> f64 {
        self.progress
            .lock()
            .ok()
            .and_then(|progress| progress.as_ref().map(|progress| progress.cost_usd))
            .unwrap_or(0.0)
    }

    /// The cap the header named.
    #[must_use]
    pub const fn cap_usd(&self) -> Option<f64> {
        self.cap_usd
    }

    /// Stop showing the writing: on a terminal `lines` take the live lines'
    /// place; elsewhere they are appended.
    pub fn finish(mut self, lines: &[String]) {
        self.stop.store(true, Ordering::SeqCst);
        let region = self
            .thread
            .take()
            .and_then(|thread| thread.join().ok().flatten());
        match region {
            Some(mut region) => {
                let _ = region.finish(lines);
            }
            None => {
                let mut err = std::io::stderr().lock();
                for line in lines {
                    let _ = writeln!(err, "{line}");
                }
            }
        }
    }
}

impl Drop for PlanWriteView {
    /// A writing that ends early, on an error, stops its live lines where
    /// they are, with the cursor back.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress(
        stage: &'static str,
        attempt: u32,
        reason: Option<&str>,
        cost: f64,
    ) -> PlanProgress {
        PlanProgress {
            stage,
            attempt,
            attempts: 3,
            reason: reason.map(str::to_string),
            model: "planner".to_string(),
            calls: attempt,
            cost_usd: cost,
            max_usd: Some(2.0),
            elapsed_ms: 3_100,
            phases: crate::plan_generate::PlanPhases::default(),
        }
    }

    const PLAN: &str = r#"
[meta]
plan = "add-a-farewell-command"

[[meta.verify]]
phase = "runs"
command = "python3 cli.py --bye world | grep -qx 'goodbye, world'"

[[task]]
id = "T1"
title = "Add farewell to the greeting module"
role = "implementer"
files = ["greeting.py"]
verify = [{ phase = "imports", command = "python3 -c 'import greeting'" }]

[[task]]
id = "T2"
title = "Say goodbye from the command line"
role = "implementer"
files = ["cli.py"]
depends_on = ["T1"]
verify = [{ phase = "runs", command = "python3 cli.py --bye world" }]
"#;

    /// gap-3436e7: writing a plan reads like the run view. The header
    /// quotes the request with the model and the cap; the live line says
    /// what the writer is doing, why it does it again, and the time and
    /// spend on glass; a pipe gets a line each time it moves on. The plan
    /// written is one line of what it holds, its tasks as `task N`, waiting,
    /// each after the ones it waits on, and what roko notices in plain
    /// words. A failed write is one sentence and a command to paste.
    #[test]
    fn writing_a_plan_reads_like_the_run_view() {
        let style = Style::plain();
        let header = WriteHeader {
            request: "Add a farewell command".to_string(),
            model: Some("glm-4.7".to_string()),
            cap_usd: Some(2.0),
            ..WriteHeader::default()
        };
        let names = BTreeMap::from([("planner".to_string(), "glm-4.7".to_string())]);
        assert_eq!(
            header.lines(style.with_live(true)).join("\n"),
            "◆ roko  run “Add a farewell command”\n  writing a plan first · glm-4.7 · cap $2.00\n"
        );
        assert_eq!(
            header.lines(style),
            ["◆ roko  run “Add a farewell command” · writing a plan first · glm-4.7 · cap $2.00"]
        );

        let fixing = progress(
            "fixing",
            2,
            Some("the reply held no plan that validates\nmore"),
            0.0123,
        );
        assert_eq!(
            live_lines(
                Some(&fixing),
                &names,
                Duration::from_secs(12),
                Some(2.0),
                style,
                4
            )
            .join("\n"),
            "  ⠼ Writing the plan  fixing the plan\n    the reply held no plan that validates\n\n  0:12 $0.01 of $2.00"
        );
        assert_eq!(
            progress_line(&progress("planning", 1, None, 0.0), &names, style),
            "  ● writing the plan · asking glm-4.7"
        );

        let tasks = TasksFile::parse_str(PLAN).expect("tasks.toml");
        let plan = WrittenPlan::of(&tasks, "plans/add-a-farewell-command".to_string());
        let text = written_lines(&plan, Duration::from_millis(3_100), 0.0123, style).join("\n");
        insta::assert_snapshot!("plan_written", text);
        let narrow = written_lines(
            &plan,
            Duration::from_millis(3_100),
            0.0123,
            style.with_live(true).with_width(80),
        );
        assert!(
            narrow
                .iter()
                .all(|line| term_style::visible_width(line) < 80),
            "{narrow:#?}"
        );
        insta::assert_snapshot!("plan_written_80", narrow.join("\n"));

        let failed = failed_lines(
            &anyhow::anyhow!("the model's answer had no plan in it"),
            "Add a \"farewell\" command",
            style,
        );
        assert_eq!(
            failed.join("\n"),
            "  ✗ roko couldn't write a plan for this request: the model's answer had no plan in it\n\n  Next  roko run --plan \"Add a \\\"farewell\\\" command\""
        );
        let invalid = anyhow::Error::new(PlanFailsValidation {
            plan_dir: "plans/add-a-farewell-command".into(),
            errors: vec!["PLAN_045 task 'T2': it has open questions".to_string()],
            cost_usd: 0.01,
        });
        assert_eq!(
            failed_lines(&invalid, "Add a farewell command", style).join("\n"),
            "  ✗ roko couldn't write a plan for this request: the plan it wrote can't run yet: PLAN_045 task 'T2': it has open questions\n\n  Next  roko plan validate plans/add-a-farewell-command"
        );
    }
}
