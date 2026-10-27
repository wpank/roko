//! The plan generator: one pipeline that turns a request into a plan.
//!
//! Every plan-generating path runs [`generate_plan`] (gap-2623b2): `roko run`
//! when it plans, `roko plan generate` (a prompt, a file, notes or a backlog
//! spec), `roko plan regenerate`, and the serve route `POST
//! /api/plans/generate`. The planner model writes a tasks.toml, which is
//! repaired, validated, scored for spec quality and checked against the
//! generated-plan policy (retrying, and escalating the model, when it fails)
//! before it is written to `plans/<slug>/`, with its `plan.md` and any
//! planner-written acceptance tests.
//!
//! This code lived in `prd.rs` until the PRD pipeline was removed on
//! 2026-10-02 (tmp/workflow-audit).

use super::{
    accept_blocks, dry_run_fs, file_paths, overview, runnable, sizes, source_checks, source_reads,
    symbol_reads,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::agent_config::command_from_config;
use crate::agent_exec::{
    AgentCrashClass, AgentExecOpts, classify_agent_crash, persist_capture_episode_distilled_by,
    run_agent_capture_silent_with_usage, timed_out_after_secs,
};
use crate::model_selection::{resolve_chat_host_planner_model, resolve_planner_model};
use crate::plan_authoring::AuthoringSpend;
use crate::runner::tui_bridge::TuiBridge;
use crate::task_parser::{META_KEYS, TASK_KEYS, TasksFile, VERIFY_KEYS, suggest_field_correction};
use crate::workspace_paths::plans_dir as workspace_plans_dir;
use anyhow::{Context as _, Result, anyhow};
use indexmap::IndexMap;
use roko_core::config::routing::LadderConfig;
use roko_core::config::schema::ModelProfile;
use roko_core::io::atomic_write_str;
use roko_learn::runtime_feedback::GenerationSourceChecks;
pub use roko_learn::runtime_feedback::{ArtifactValidationReport, GenerationOutcome};

/// How many tasks the tasks.toml files at `paths` hold together.
fn generated_task_count(paths: &[PathBuf]) -> Result<usize> {
    let mut task_count = 0usize;
    for path in paths {
        let tasks_file =
            TasksFile::parse(path).with_context(|| format!("parse {}", path.display()))?;
        task_count = task_count.saturating_add(tasks_file.tasks.len());
    }
    Ok(task_count)
}

fn normalize_task_title(title: &str) -> String {
    title
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn preserve_completed_task_status(
    old_tasks: Option<&TasksFile>,
    mut regenerated: TasksFile,
    plan_dir: &Path,
) -> TasksFile {
    use crate::task_parser::PlanMutation;

    if let Some(old_tasks) = old_tasks {
        let completed: Vec<&crate::task_parser::TaskDef> = old_tasks
            .tasks
            .iter()
            .filter(|task| task.status.eq_ignore_ascii_case("done"))
            .collect();

        let mutations: Vec<PlanMutation> = regenerated
            .tasks
            .iter()
            .filter_map(|task| {
                let normalized = normalize_task_title(&task.title);
                let already_done = completed.iter().any(|old| {
                    let old_title = normalize_task_title(&old.title);
                    old.id == task.id
                        || old_title == normalized
                        || old_title.contains(&normalized)
                        || normalized.contains(&old_title)
                });
                if already_done {
                    Some(PlanMutation::MarkTaskDone {
                        task_id: task.id.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        regenerated.apply_mutations(mutations);

        regenerated.meta.iteration = old_tasks.meta.iteration.saturating_add(1);
        if regenerated.meta.plan.trim().is_empty() {
            regenerated.meta.plan = old_tasks.meta.plan.clone();
        }
    }

    if regenerated.meta.plan.trim().is_empty() {
        regenerated.meta.plan = plan_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown-plan".to_string());
    }

    // apply_mutations already calls recount_meta; call it once more to handle
    // the case where old_tasks was None (no mutations were applied).
    regenerated.recount_meta();

    regenerated
}

/// The document a plan in `plan_dir` is regenerated from: its `plan.md`.
pub fn plan_source_document(plan_dir: &Path) -> Result<PathBuf> {
    let path = plan_dir.join("plan.md");
    if path.exists() {
        return Ok(path);
    }
    Err(anyhow!(
        "no plan.md in {} to regenerate the plan from",
        plan_dir.display()
    ))
}

/// Characters of the model's output that a failed plan generation prints.
const FAILURE_OUTPUT_CHARS: usize = 2000;
/// Characters of the model's output that a non-retriable agent error quotes.
const AGENT_ERROR_PREVIEW_CHARS: usize = 500;
/// Where, under the workspace's `.roko/`, a generation keeps the planner's
/// replies that held no plan that validates, one directory per plan slug
/// (bug-f0d60c).
const REJECTED_DRAFTS_DIR: &str = "plan-drafts";
/// Characters of a validation error that a progress report quotes.
const PROGRESS_REASON_CHARS: usize = 200;

/// The turns every planner call may take, generating or revising a plan
/// (bug-09cc5f). Planning is Delta work (`OperatingFrequency::select` puts
/// plan regeneration there): a planner reading a large repository used up
/// the Claude CLI's default of `OperatingFrequency::Theta` turns, 10, before
/// it wrote the plan.
pub(crate) const PLANNER_MAX_TURNS: u32 =
    roko_core::operating_frequency::OperatingFrequency::Delta.turn_limit();

/// What a failed planner call printed, as a line to add to its error: the
/// provider's own message is what says a key is wrong or a server is
/// unreachable (bug-59ed90). Empty when it printed nothing.
fn agent_said(output: &str) -> String {
    let output = output.trim();
    if output.is_empty() {
        String::new()
    } else {
        format!(
            "\nThe planner said: {}",
            crate::run::truncate(output, AGENT_ERROR_PREVIEW_CHARS)
        )
    }
}

/// Default model escalation chain: haiku -> sonnet -> opus.
///
/// When the workspace configures models, chain models it does not configure
/// are skipped so we never escalate to a model it cannot run.
const DEFAULT_ESCALATION_CHAIN: &[&str] =
    &["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-6"];

/// Return the next-tier model for escalation on validation failures.
///
/// Checks `tier_models` config first (keys: `"haiku"`, `"sonnet"`, `"opus"`),
/// falling back to [`DEFAULT_ESCALATION_CHAIN`]. `current` and the chain
/// entries are compared by slug, so a `[models.*]` key matches the chain entry
/// for its slug.
///
/// Escalation only moves up from `current`, which starts as the planner model.
/// It returns `None`, and the retry keeps `current`, when `current` is at the
/// top of the chain, when no configured model sits above it, or when `current`
/// is not in the chain at all: a model outside the chain, such as a newer
/// frontier planner, has no known rank, so no chain model is known to be
/// stronger.
///
/// When `models` is non-empty, only chain models it configures (by key or
/// slug) are eligible. An empty map disables filtering (backward compat).
fn next_tier_model(
    current: Option<&str>,
    tier_models: &HashMap<String, String>,
    models: &IndexMap<String, ModelProfile>,
) -> Option<String> {
    // Build the chain from config or defaults.
    let chain: Vec<&str> = if tier_models.is_empty() {
        DEFAULT_ESCALATION_CHAIN.to_vec()
    } else {
        // Config keys in escalation order.
        ["haiku", "sonnet", "opus"]
            .iter()
            .filter_map(|k| tier_models.get(*k).map(String::as_str))
            .collect()
    };

    // Find the current model's rank; a model outside the chain stays put.
    let current_slug = model_slug(models, current?);
    let pos = chain
        .iter()
        .position(|m| model_slug(models, m) == current_slug)?;

    // Candidates above the current position. When models are configured, only
    // return one the workspace configures so we don't escalate to an
    // unavailable model.
    let mut candidates = chain[pos + 1..].iter().copied();
    if models.is_empty() {
        candidates.next().map(str::to_string)
    } else {
        candidates
            .find(|m| model_is_configured(models, m))
            .map(str::to_string)
    }
}

/// The provider slug `model` names: the slug of its `[models.*]` entry when it
/// is a key there, else `model` itself.
fn model_slug<'a>(models: &'a IndexMap<String, ModelProfile>, model: &'a str) -> &'a str {
    models
        .get(model)
        .map(|profile| profile.slug.trim())
        .filter(|slug| !slug.is_empty())
        .unwrap_or(model)
}

/// Whether `model` names a `[models.*]` entry, by key or by slug.
fn model_is_configured(models: &IndexMap<String, ModelProfile>, model: &str) -> bool {
    models.contains_key(model) || models.values().any(|profile| profile.slug.trim() == model)
}

/// What a plan is generated from. Every plan-generating command runs
/// [`generate_plan`] (gap-2623b2), so planner fixes land in one place.
#[derive(Debug, Clone, Copy)]
pub enum PlanSource<'a> {
    /// A request's text: `roko run`'s prompt, `roko plan generate`'s prompt,
    /// file or notes, or the prompt the serve route was sent.
    Text {
        /// The text to plan from.
        text: &'a str,
        /// What the text is (`"prompt"`, `"file"`, `"notes"`).
        kind: &'a str,
    },
    /// A plan directory (`roko plan regenerate`), regenerated in place from
    /// its `plan.md`. Tasks marked done stay done.
    Regenerate(&'a Path),
}

/// One run of the plan generator ([`generate_plan`]).
pub struct PlanRequest<'a> {
    /// What the plan is generated from.
    pub source: PlanSource<'a>,
    /// The plan's slug: its `meta.plan` and its directory name.
    pub slug: &'a str,
    /// The workspace the plan is for.
    pub workdir: &'a Path,
    /// Where plan directories go. `None` is the workspace plans directory,
    /// or a regenerated plan's parent.
    pub plans_root: Option<&'a Path>,
    /// More for the planner to read after the source: `--context` files or
    /// earlier validation findings.
    pub context: Option<&'a str>,
    /// Plan in a scratch copy of the workspace, and only report.
    pub dry_run: bool,
    /// Why an earlier plan failed, for replanning.
    pub failure_context: Option<&'a str>,
    /// The planner model. `None` is `[authoring] planner_model`.
    pub model: Option<&'a str>,
    /// The planner's reasoning effort. `None` is `[agent] effort`.
    pub effort: Option<&'a str>,
    /// Where each agent call's spend is published.
    pub live: Option<TuiBridge>,
    /// Nobody reviews the plan before it runs: a chat host asked for it, or
    /// `roko run` runs it next (bug-7a08a8). The planner decides every open
    /// choice itself, and an open question that carries its own answer is
    /// written into its task, since an open question keeps a plan from
    /// running (PLAN_045).
    pub unattended: bool,
    /// A chat host asked for the plan (bug-a0432b): only a model roko's own
    /// tool loop runs writes it, as only such models run a chat host's tasks
    /// (gap-cf427e), so the host's request never runs on the person's own CLI
    /// agent.
    pub for_chat_host: bool,
    /// Where generation reports what it is doing as it moves on
    /// (gap-11d870), for a caller that shows it while the plan is written.
    pub progress: Option<PlanProgressSink>,
    /// What the planner calls may cost together, in USD (gap-18bfa2); 0
    /// sets no cap. `None` is `[authoring] max_generation_usd`.
    pub max_usd: Option<f64>,
    /// The workspace file the source's text is, when it is one: a spec read
    /// through `spec_path` or `--from-file` (bug-60ad88). Every task reads
    /// it.
    pub source_path: Option<&'a Path>,
}

impl<'a> PlanRequest<'a> {
    /// A request for the plan `slug` from `source`, with every option unset.
    #[must_use]
    pub fn new(source: PlanSource<'a>, slug: &'a str, workdir: &'a Path) -> Self {
        Self {
            source,
            slug,
            workdir,
            plans_root: None,
            context: None,
            dry_run: false,
            failure_context: None,
            model: None,
            effort: None,
            live: None,
            unattended: false,
            for_chat_host: false,
            progress: None,
            max_usd: None,
            source_path: None,
        }
    }
}

/// What [`generate_plan`] is doing, reported to [`PlanRequest::progress`]
/// each time it moves on (gap-11d870). roko serve shows it in the
/// operation's status, so a host can say what is happening instead of
/// only "running".
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PlanProgress {
    /// `planning`: the planner's first call. `retrying`: a planner call
    /// failed (it crashed, or timed out) and runs again. `fixing`: the reply
    /// held no plan that validates, so the planner is asked again.
    /// `regenerating`: the planner rewrites a valid plan once to improve it.
    /// `writing`: the plan's files are written. `recording`: the
    /// generation's learning episode is recorded; its distillation runs
    /// after the generation ends.
    pub stage: &'static str,
    /// This stage's attempt, from 1.
    pub attempt: u32,
    /// The most attempts this stage makes.
    pub attempts: u32,
    /// Why the stage runs: what failed, or what the plan lacked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The model the stage's planner call runs on.
    pub model: String,
    /// The planner calls made so far, a running one included.
    pub calls: u32,
    /// What the planner calls that returned cost together, in USD.
    pub cost_usd: f64,
    /// What they may cost together (gap-18bfa2); none when uncapped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_usd: Option<f64>,
    /// How long the generation has run, in ms (gap-61c95c).
    pub elapsed_ms: u64,
    /// Where that time went, so far (gap-61c95c).
    pub phases: PlanPhases,
}

/// Where a plan generation's time went (gap-61c95c), in ms, each phase
/// cumulative and `None` until it has run. The phases add up to at most
/// [`PlanProgress::elapsed_ms`]: a planner call still running is in neither.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct PlanPhases {
    /// Reading the source and the workspace and lifting the source's checks
    /// into the planner prompt, before the first planner call.
    pub lift_ms: Option<u64>,
    /// Inside planner calls: the first, its retries (their backoff
    /// included), fixes and regenerations.
    pub planner_ms: Option<u64>,
    /// Between planner calls: repairing and checking their drafts, and
    /// writing the plan.
    pub repair_ms: Option<u64>,
    /// Validating the written plan.
    pub validate_ms: Option<u64>,
}

/// The clock of one plan generation (gap-61c95c): where its time goes,
/// marked as it moves on.
struct PlanClock {
    started: Instant,
    marks: std::sync::Mutex<PlanClockMarks>,
}

#[derive(Default)]
struct PlanClockMarks {
    /// When the first planner call started: the lift's end.
    lifted: Option<Instant>,
    /// The planner calls that ended, together.
    planner: Option<Duration>,
    /// When the planner call running now started.
    calling: Option<Instant>,
    /// When the written plan's validation started: the repairs' end.
    validating: Option<Instant>,
    validate: Option<Duration>,
}

impl PlanClock {
    fn new(started: Instant) -> Self {
        Self {
            started,
            marks: std::sync::Mutex::default(),
        }
    }

    fn marks(&self) -> std::sync::MutexGuard<'_, PlanClockMarks> {
        self.marks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The lift ends: the first planner call is about to start.
    fn lifted(&self) {
        self.marks().lifted.get_or_insert_with(Instant::now);
    }

    /// Run `call`, a planner call, counting its time as the planner's. The
    /// first one ends the lift.
    async fn planner<T>(&self, call: impl std::future::Future<Output = T>) -> T {
        {
            let now = Instant::now();
            let mut marks = self.marks();
            marks.lifted.get_or_insert(now);
            marks.calling = Some(now);
        }
        let out = call.await;
        let mut marks = self.marks();
        if let Some(started) = marks.calling.take() {
            *marks.planner.get_or_insert_default() += started.elapsed();
        }
        out
    }

    /// The written plan's validation starts.
    fn validating(&self) {
        self.marks().validating = Some(Instant::now());
    }

    /// The written plan's validation ended.
    fn validated(&self) {
        let mut marks = self.marks();
        marks.validate = marks.validating.map(|started| started.elapsed());
    }

    /// How long the generation has run, and where the time went.
    fn read(&self) -> (u64, PlanPhases) {
        let now = Instant::now();
        let marks = self.marks();
        let ms = |span: Duration| u64::try_from(span.as_millis()).unwrap_or(u64::MAX);
        // A running planner call and the validation are no repair.
        let lift = marks
            .lifted
            .map(|lifted| lifted.saturating_duration_since(self.started));
        let lifted = marks.lifted.unwrap_or(now);
        let repairs_end = marks.validating.unwrap_or(now);
        let calling = marks.calling.map_or(Duration::ZERO, |started| {
            repairs_end.saturating_duration_since(started)
        });
        let repair = marks
            .planner
            .map(|planner| {
                repairs_end
                    .saturating_duration_since(lifted)
                    .saturating_sub(planner)
                    .saturating_sub(calling)
            })
            .filter(|repair| !repair.is_zero());
        let phases = PlanPhases {
            lift_ms: lift.map(ms),
            planner_ms: marks.planner.map(ms),
            repair_ms: repair.map(ms),
            validate_ms: marks.validate.map(ms),
        };
        (ms(now.saturating_duration_since(self.started)), phases)
    }
}

/// Where [`generate_plan`] reports its [`PlanProgress`].
pub type PlanProgressSink = Arc<dyn Fn(&PlanProgress) + Send + Sync>;

/// A generated plan that was written but cannot run: `roko plan validate`
/// finds errors in it (bug-7a08a8). [`generate_plan`] returns it as its
/// error, so no caller reports such a plan as written and ready.
#[derive(Debug)]
pub struct PlanFailsValidation {
    /// Where the plan was written.
    pub plan_dir: PathBuf,
    /// One line per error, as `PLAN_045 task 'T2': …`.
    pub errors: Vec<String>,
    /// What writing the plan cost, in USD (bug-a0432b).
    pub cost_usd: f64,
}

impl std::fmt::Display for PlanFailsValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the plan was written to {}, but it cannot run: {} error(s)",
            self.plan_dir.display(),
            self.errors.len()
        )?;
        for error in &self.errors {
            write!(f, "\n- {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for PlanFailsValidation {}

/// What an unattended plan's planner reads after the request (bug-7a08a8).
const UNATTENDED_PLANNER_NOTE: &str = "Nobody will review this plan or answer questions \
     about it before it runs. Decide every choice the request leaves open yourself, write each \
     decision into the task it affects (its description or an acceptance criterion, starting \
     with \"Assumption:\"), and leave every task's `open_questions` empty.";

/// Words that start an open question's own answer, as in "…? Assumption: …".
const ANSWER_MARKERS: &[&str] = &[
    "assumption:",
    "assume:",
    "assuming:",
    "default:",
    "decision:",
];

/// Whether an open question carries its own answer after one of
/// [`ANSWER_MARKERS`].
fn has_own_answer(question: &str) -> bool {
    let lower = question.to_ascii_lowercase();
    ANSWER_MARKERS.iter().any(|marker| {
        lower
            .find(marker)
            .is_some_and(|at| !lower[at + marker.len()..].trim().is_empty())
    })
}

/// The plan `plan` (a tasks.toml) with each open question that carries its own
/// answer moved into its task's description, and one `task <id>: <question>`
/// line per question moved (bug-7a08a8). A question with no answer stays, and
/// keeps the plan from running (PLAN_045).
fn fold_answered_questions(plan: &str) -> Result<(String, Vec<String>)> {
    let mut root: toml::Value = toml::from_str(plan).context("parse the plan")?;
    let mut folded = Vec::new();
    let tasks = root.get_mut("task").and_then(toml::Value::as_array_mut);
    for task in tasks
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_table_mut)
    {
        let questions: Vec<String> = task
            .get("open_questions")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(toml::Value::as_str)
            .map(str::trim)
            .filter(|question| !question.is_empty())
            .map(str::to_string)
            .collect();
        let (answered, open): (Vec<String>, Vec<String>) = questions
            .into_iter()
            .partition(|question| has_own_answer(question));
        if answered.is_empty() {
            continue;
        }
        let id = task
            .get("id")
            .and_then(toml::Value::as_str)
            .unwrap_or("?")
            .to_string();
        let mut description = task
            .get("description")
            .and_then(toml::Value::as_str)
            .unwrap_or_default()
            .trim_end()
            .to_string();
        description.push_str("\n\nDecided while planning, with nobody to ask:");
        for question in answered {
            description.push_str("\n- ");
            description.push_str(&question);
            folded.push(format!("task {id}: {question}"));
        }
        task.insert("description".to_string(), toml::Value::String(description));
        if open.is_empty() {
            task.remove("open_questions");
        } else {
            let open = open.into_iter().map(toml::Value::String).collect();
            task.insert("open_questions".to_string(), toml::Value::Array(open));
        }
    }
    if folded.is_empty() {
        return Ok((plan.to_string(), folded));
    }
    let plan = toml::to_string_pretty(&root).context("write the plan back")?;
    Ok((plan, folded))
}

/// How much of its source a planner reads inline, and how many repository
/// files it may open, by its context window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlannerBudget {
    /// Characters of source in the prompt.
    source_chars: usize,
    /// Repository files the planner may read.
    read_files: usize,
}

impl PlannerBudget {
    /// The old fixed caps: the budget of a planner whose window is unknown,
    /// and the floor for every planner.
    const SMALL: Self = Self {
        source_chars: 8_000,
        read_files: 5,
    };

    /// A quarter of a `window`-token context for the source (about four
    /// characters a token), and one file per 2,000 tokens of another quarter.
    fn for_context_window(window: Option<u64>) -> Self {
        let Some(quarter) = window.map(|window| usize::try_from(window / 4).unwrap_or(usize::MAX))
        else {
            return Self::SMALL;
        };
        Self {
            source_chars: quarter.saturating_mul(4).max(Self::SMALL.source_chars),
            read_files: (quarter / 2_000).max(Self::SMALL.read_files),
        }
    }
}

/// The context window a `[models.*]` entry (by key, else by slug) states for
/// `model`.
pub(crate) fn planner_context_window(
    models: &IndexMap<String, ModelProfile>,
    model: &str,
) -> Option<u64> {
    models
        .get(model)
        .or_else(|| models.values().find(|profile| profile.slug.trim() == model))
        .map(|profile| profile.context_window)
        .filter(|&window| window > 0)
}

/// A candidate plan's spec-quality scores, by the static rules
/// ([`roko_gate::spec_quality`], sq-3), as generation acts on them (3218).
#[derive(Debug, Clone)]
struct SpecScores {
    /// One `task <id> <rule>: <detail>` line per hard fail.
    hard_fails: Vec<String>,
    /// The mean task score, 0–100.
    mean: f64,
    /// The lowest task score.
    min: f64,
    /// Tasks per band.
    bands: std::collections::BTreeMap<&'static str, usize>,
    /// One line per task below `allow_threshold`, naming the rules it scored
    /// 0 on.
    weak: Vec<String>,
}

impl SpecScores {
    /// Whether generation asks the planner once more: a plan mean below
    /// `allow_threshold`, or a task below `block_threshold`.
    fn is_weak(&self, config: &roko_core::config::SpecQualityConfig) -> bool {
        self.mean < config.allow_threshold || self.min < config.block_threshold
    }

    /// The line generation prints.
    fn summary_line(&self) -> String {
        let bands: Vec<String> = ["A", "B", "C", "D"]
            .iter()
            .map(|band| format!("{band} {}", self.bands.get(band).copied().unwrap_or(0)))
            .collect();
        format!(
            "Spec quality: mean {:.1}, lowest task {:.1}; bands {}",
            self.mean,
            self.min,
            bands.join(", ")
        )
    }

    /// What [`GenerationOutcome`] records.
    fn record(&self, regenerated: bool) -> roko_learn::runtime_feedback::GenerationSpecQuality {
        roko_learn::runtime_feedback::GenerationSpecQuality {
            mean: self.mean,
            min: self.min,
            bands: self
                .bands
                .iter()
                .map(|(band, count)| ((*band).to_string(), *count))
                .collect(),
            regenerated,
        }
    }
}

/// Score a candidate `tasks.toml` against the workspace at `workdir`; `None`
/// when `[spec_quality] mode = "off"` or nothing could be scored.
fn spec_scores(
    candidate: &str,
    workdir: &Path,
    config: &roko_core::config::SpecQualityConfig,
) -> Option<SpecScores> {
    if !config.is_on() {
        return None;
    }
    let scratch = tempfile::tempdir().ok()?;
    let path = scratch.path().join("tasks.toml");
    std::fs::write(&path, candidate).ok()?;
    let report = roko_gate::spec_quality::lint_files(&[path], workdir);
    if report.tasks.is_empty() {
        return None;
    }
    let scores: Vec<f64> = report.tasks.iter().map(|record| record.score).collect();
    let mean = scores.iter().sum::<f64>() / scores.len() as f64;
    let min = scores.iter().copied().fold(f64::INFINITY, f64::min);
    let mut bands = std::collections::BTreeMap::new();
    let mut hard_fails = Vec::new();
    let mut weak = Vec::new();
    for record in &report.tasks {
        *bands.entry(record.band).or_insert(0) += 1;
        for &rule in &record.hard_fail {
            let detail = record
                .hard_fail_detail
                .get(rule)
                .filter(|details| !details.is_empty())
                .map_or_else(
                    || hard_fail_name(rule).to_string(),
                    |details| details.join("; "),
                );
            hard_fails.push(format!("task {} {rule}: {detail}", record.task_id));
        }
        if record.score < config.allow_threshold {
            let zero: Vec<String> = roko_gate::spec_quality::RULES
                .iter()
                .filter(|rule| !record.excluded.contains(&rule.id))
                .filter(|rule| record.rules.get(rule.id).is_some_and(|score| *score <= 0.0))
                .map(|rule| format!("{} ({})", rule.id, rule.name))
                .collect();
            weak.push(format!(
                "- task {} (score {:.1}): {}",
                record.task_id,
                record.score,
                if zero.is_empty() {
                    "no rule scored 0; strengthen the partial ones".to_string()
                } else {
                    format!("scored 0 on {}", zero.join(", "))
                }
            ));
        }
    }
    Some(SpecScores {
        hard_fails,
        mean,
        min,
        bands,
        weak,
    })
}

/// What a hard fail means, as `HARD_FAILS` names it.
fn hard_fail_name(rule: &str) -> &'static str {
    roko_gate::spec_quality::HARD_FAILS
        .iter()
        .find(|(id, _)| *id == rule)
        .map_or("hard fail", |(_, name)| *name)
}

/// The prompt that asks the planner once more for a weak plan (3218): the
/// original request, the plan it wrote, and each weak task's rules that
/// scored 0.
fn spec_regeneration_prompt(task_prompt: &str, plan: &str, scores: &SpecScores) -> String {
    format!(
        "{task_prompt}\n\n---\n\nYour plan below scored low on spec quality (mean {:.1}, \
         lowest task {:.1}). Each weak task lists the spec rules it scored 0 on:\n{}\n\n\
         ```toml\n{plan}\n```\n\n\
         Rewrite the whole plan so every weak task meets those rules, and keep what already \
         works. Output the complete plan as a single ```toml fenced block, followed only by \
         the ```accept:accept/<file> blocks of the tests it pins.",
        scores.mean,
        scores.min,
        scores.weak.join("\n")
    )
}

/// bug-59ed90: the checks of a candidate `tasks.toml` that hide their test
/// runner's output, one PLAN_050 line each; none when it does not parse.
fn hidden_output_lines(candidate: &str) -> Vec<String> {
    TasksFile::parse_str(candidate).map_or_else(
        |_| Vec::new(),
        |tasks_file| {
            crate::plan_validate::hidden_output_checks(&tasks_file)
                .iter()
                .map(|check| format!("- {}", check.message()))
                .collect()
        },
    )
}

/// The prompt that asks the planner once more for a plan whose checks hide
/// their test runner's output (bug-59ed90): the original request, the plan it
/// wrote, and each such check.
fn hidden_output_regeneration_prompt(task_prompt: &str, plan: &str, hidden: &[String]) -> String {
    format!(
        "{task_prompt}\n\n---\n\nThese checks of your plan below hide their test runner's \
         output, so when one fails neither the retry nor the person can see why:\n{}\n\n\
         ```toml\n{plan}\n```\n\n\
         Rewrite each of those checks to run its test command on its own, with no pipe, \
         redirection or test count around it: a check whose command the source gives runs that \
         command alone, unchanged. To make one of your own checks fail before its task is done, \
         name the new tests in the command (and in the task's description), or put a `grep -q` \
         for them before it. Keep the rest of the plan as it is. Output the complete plan as a \
         single ```toml fenced block, followed only by the ```accept:accept/<file> blocks of \
         the tests it pins.",
        hidden.join("\n")
    )
}

/// The prompt that asks the planner again when its `reply` held no plan that
/// validates (bug-f0d60c): the original request with its source, the reply
/// quoted up to `reply_chars` characters, and what was wrong with it. A retry
/// that saw only the error, the reply's first 2,000 characters and a one-task
/// template rebuilt a smaller plan from the template, and the source's
/// acceptance checks were lost.
fn extraction_retry_prompt(
    task_prompt: &str,
    kind: &str,
    reply: &str,
    error: &str,
    reply_chars: usize,
) -> String {
    let quoted = crate::run::truncate(reply, reply_chars);
    let cut = if quoted.len() < reply.len() {
        format!("\n[reply truncated at {reply_chars} chars]")
    } else {
        String::new()
    };
    format!(
        "{task_prompt}\n\n---\n\nYour reply below held no plan that validates:\n{error}\n\n\
         ````text\n{quoted}{cut}\n````\n\n\
         Fix only what the errors name. Keep every task and every verify step of your reply, \
         and every requirement and acceptance criterion of the {kind} above: do not shorten \
         the plan or start again from a smaller one. Output the complete plan as a single \
         ```toml fenced block, followed only by the ```accept:accept/<file> blocks of the \
         tests it pins. The meta field is `plan`, not `name`."
    )
}

/// Keep the planner's `reply`, rejected for `error`, as `rejected-<n>.md` in
/// `.roko/plan-drafts/<slug>/` of `workdir` (bug-f0d60c), so what a retry
/// changed can be read afterwards. The first draft a generation keeps
/// replaces the drafts an earlier generation of the slug kept. A draft that
/// cannot be written is only logged.
fn keep_rejected_draft(
    workdir: &Path,
    slug: &str,
    n: u32,
    reply: &str,
    error: &str,
) -> Option<PathBuf> {
    let dir = workdir.join(".roko").join(REJECTED_DRAFTS_DIR).join(slug);
    if n == 1
        && let Ok(entries) = std::fs::read_dir(&dir)
    {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with("rejected-") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    let path = dir.join(format!("rejected-{n}.md"));
    let text = format!(
        "# Rejected draft {n} of plan `{slug}`\n\n\
         It held no plan that validates:\n\n{error}\n\n\
         ## The planner's reply\n\n{reply}\n"
    );
    match std::fs::create_dir_all(&dir).and_then(|()| atomic_write_str(&path, &text)) {
        Ok(()) => Some(path),
        Err(error) => {
            tracing::warn!(
                path = %path.display(),
                %error,
                "plan generate: could not keep a rejected draft"
            );
            None
        }
    }
}

/// The extensions of the documents a task may write at any length
/// ([`repair_generated_draft`]).
const DOCUMENT_EXTENSIONS: &[&str] = &["adoc", "markdown", "md", "rst", "txt"];

/// Repair a generated draft's mechanical defects before it is checked
/// (gap-b95913), rather than send it back to the planner for them: its
/// `meta.total` becomes its number of tasks, each task keeps its first
/// `policy.max_read_files_per_task` `read_files`, and its line ranges in files
/// no earlier task changes fit those files
/// ([`crate::plan_policy::fit_ranges_to_files`]). A task that writes only
/// documents keeps its tier: a `max_loc` over the tier's limit becomes the
/// limit (bug-27f00b), since a document's length is not what a tier sizes,
/// and splitting one document across tasks fragments its ownership. Returns
/// one line per repair.
fn repair_generated_draft(
    plan: &mut TasksFile,
    policy: crate::plan_policy::PlanExecutionPolicy,
    workdir: &Path,
    sources: &[String],
) -> Vec<String> {
    let mut repairs = Vec::new();
    let total = u32::try_from(plan.tasks.len()).unwrap_or(u32::MAX);
    if plan.meta.total != total {
        repairs.push(format!(
            "meta.total {} became {total}, its number of tasks",
            plan.meta.total
        ));
        plan.meta.total = total;
    }
    for task in &mut plan.tasks {
        let limit = policy.tier_size_limits(task.tier_class()).max_loc;
        let documents = !task.files.is_empty()
            && task.files.iter().all(|file| {
                Path::new(file).extension().is_some_and(|extension| {
                    DOCUMENT_EXTENSIONS
                        .iter()
                        .any(|known| extension.eq_ignore_ascii_case(known))
                })
            });
        if documents && let Some(max_loc) = task.max_loc.filter(|max_loc| *max_loc > limit) {
            repairs.push(format!(
                "{}: max_loc {max_loc} became {limit}, its {} tier's limit, as it writes only \
                 documents",
                task.id,
                task.tier_class()
            ));
            task.max_loc = Some(limit);
        }
    }
    // bug-417c4a: the documents the plan is written from are no part of a
    // task's reads, and a generated task may read as many files as it owns.
    for task in &mut plan.tasks {
        let keep = policy.max_read_files_for(task);
        let Some(context) = task.context.as_mut() else {
            continue;
        };
        let mut counted = 0;
        let mut dropped = Vec::new();
        context.read_files.retain(|read_file| {
            if sources.contains(&read_file.path) {
                return true;
            }
            counted += 1;
            if counted > keep {
                dropped.push(read_file.path.clone());
            }
            counted <= keep
        });
        if !dropped.is_empty() {
            repairs.push(format!(
                "{}: kept its first {keep} read_files besides the plan's sources and dropped {}",
                task.id,
                dropped.join(", ")
            ));
        }
    }
    repairs.extend(crate::plan_policy::fit_ranges_to_files(plan, workdir));
    repairs
}

/// `toml`, a draft, with each verify step written as its bare command
/// (`verify = ["tools/check.sh"]` in `[meta]` or a task, or one string) made
/// a step table that runs it (gap-4dd77d): round v7's phase-3 re-plan wrote
/// one, and the draft failed to parse and went back for a whole new one. A
/// draft that does not parse, or has no such step, comes back as it was.
/// Returns the draft and one line per step changed.
fn wrap_command_steps(toml: &str) -> (String, Vec<String>) {
    let Ok(mut root) = toml::from_str::<toml::Value>(&crate::task_parser::repair_toml(toml)) else {
        return (toml.to_string(), Vec::new());
    };
    let mut lines = Vec::new();
    if let Some(meta) = root.get_mut("meta").and_then(toml::Value::as_table_mut) {
        lines.extend(wrap_table_steps(meta, "`[meta]`"));
    }
    if let Some(tasks) = root.get_mut("task").and_then(toml::Value::as_array_mut) {
        for task in tasks.iter_mut().filter_map(toml::Value::as_table_mut) {
            let owner = task
                .get("id")
                .and_then(toml::Value::as_str)
                .unwrap_or("a task")
                .to_string();
            lines.extend(wrap_table_steps(task, &owner));
        }
    }
    match (lines.is_empty(), toml::to_string(&root)) {
        (false, Ok(wrapped)) => (wrapped, lines),
        _ => (toml.to_string(), Vec::new()),
    }
}

/// The verify steps of `table` written as bare commands made step tables,
/// `phase = "test"`, with one line each naming `owner`.
fn wrap_table_steps(table: &mut toml::map::Map<String, toml::Value>, owner: &str) -> Vec<String> {
    let Some(verify) = table.get_mut("verify") else {
        return Vec::new();
    };
    if let toml::Value::String(command) = verify {
        *verify = toml::Value::Array(vec![toml::Value::String(std::mem::take(command))]);
    }
    let Some(steps) = verify.as_array_mut() else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    for step in steps {
        let toml::Value::String(command) = step else {
            continue;
        };
        let command = std::mem::take(command);
        lines.push(format!(
            "{owner}: the verify step `{command}`, written as its bare command, became a step \
             that runs it"
        ));
        let mut wrapped = toml::map::Map::new();
        wrapped.insert("phase".to_string(), toml::Value::String("test".to_string()));
        wrapped.insert("command".to_string(), toml::Value::String(command));
        *step = toml::Value::Table(wrapped);
    }
    lines
}

/// A plan source, read for the planner.
struct ReadSource<'a> {
    origin: PlanSource<'a>,
    /// What the planner plans from.
    content: String,
    /// What the prompt calls it (`prompt`, `file`, `notes`, `plan`).
    kind: String,
    /// The file the content came from, which the planner need not reopen.
    path: Option<PathBuf>,
    /// Keywords for the repository context, beside the slug's.
    title: String,
    template: crate::plan_generate::PlanTemplateKind,
    /// The plan being regenerated, for [`PlanSource::Regenerate`].
    regeneration: Option<Regeneration>,
}

/// A plan being regenerated in place: its directory and current tasks.toml.
struct Regeneration {
    plan_dir: PathBuf,
    existing_toml: String,
    existing: Option<TasksFile>,
}

impl<'a> ReadSource<'a> {
    fn read(origin: PlanSource<'a>) -> Result<Self> {
        let read = |path: &Path| {
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
        };
        Ok(match origin {
            PlanSource::Text { text, kind } => Self {
                origin,
                content: text.to_string(),
                kind: kind.to_string(),
                path: None,
                title: String::new(),
                template: crate::plan_generate::PlanTemplateKind::resolve(None),
                regeneration: None,
            },
            PlanSource::Regenerate(plan_dir) => {
                let existing_toml = read(&plan_dir.join("tasks.toml"))?;
                let path = plan_source_document(plan_dir)?;
                Self {
                    origin,
                    content: read(&path)?,
                    kind: "plan".to_string(),
                    path: Some(path),
                    title: plan_dir
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    template: crate::plan_generate::PlanTemplateKind::resolve(None),
                    regeneration: Some(Regeneration {
                        plan_dir: plan_dir.to_path_buf(),
                        existing: TasksFile::parse_str(&existing_toml).ok(),
                        existing_toml,
                    }),
                }
            }
        })
    }

    /// The `task_id` and task kind of the planner call's episode.
    fn episode(&self, slug: &str) -> (String, &'static str) {
        match self.origin {
            PlanSource::Text { .. } => (format!("plan:generate:{slug}"), "plan-generate"),
            PlanSource::Regenerate(_) => (format!("plan:regenerate:{slug}"), "plan-regenerate"),
        }
    }
}

/// The planner's task prompt: `source` within `budget`, what to output, and
/// the checks the output must pass. `extra` follows the source.
fn plan_task_prompt(
    source: &ReadSource<'_>,
    slug: &str,
    budget: PlannerBudget,
    template_guidance: &str,
    extra: &str,
) -> String {
    let kind = source.kind.as_str();
    let content = if source.content.len() > budget.source_chars {
        let boundary = source.content.floor_char_boundary(budget.source_chars);
        format!(
            "{}\n\n[{kind} content truncated at {} chars]",
            &source.content[..boundary],
            budget.source_chars
        )
    } else {
        source.content.clone()
    };
    let (task, shape) = match &source.regeneration {
        Some(regeneration) => (
            "Regenerate the plan below from its source document, with full modern metadata."
                .to_string(),
            format!(
                "Keep each task's id where the task still applies; tasks already marked done \
                 stay done.\n\n## Existing tasks.toml\n\n```toml\n{}\n```",
                regeneration.existing_toml
            ),
        ),
        None => (
            format!("Generate an implementation plan from the {kind} below."),
            "Each requirement in it becomes one or more tasks. Each acceptance criterion \
             becomes a task verification command."
                .to_string(),
        ),
    };
    let reopen = source.path.as_ref().map_or_else(String::new, |path| {
        format!(" — do NOT read {} again", path.display())
    });
    format!(
        "{task}\n\n\
         Plan slug (use exactly in meta.plan): {slug}\n\n\
         IMPORTANT: The {kind} content is included inline{reopen}. You may read up to \
         {read_files} codebase files to understand existing structure, but then you MUST \
         produce your output.\n\n\
         {shape}\n\n\
         Do NOT create files directly. Instead, output the plan content \
         as follows:\n\n\
         1. Output a fenced block tagged `toml` containing the tasks.toml content.\n\
         2. Optionally output a fenced block tagged `plan.md` containing the plan narrative.\n\n\
         TOML quality checklist (every task MUST pass all of these):\n\
         - `meta.plan` matches the slug exactly: {slug} (use `plan =`, NOT `name =`)\n\
         - Every task has `id`, `title`, `description`, `status = \"ready\"`, `role`, and `tier`\n\
         - `files` lists the concrete paths the task changes or creates: files that exist, and \
           new files at the paths they will have (no placeholders, globs or bare directories)\n\
         - `depends_on` only references task ids defined in this same plan\n\
         - No `model_hint` field: `tier` and `role` pick each task's model; add `rung` \
           (for example `rung = \"strong\"`) only when a task needs more than its tier's \
           start rung\n\
         - No `mcp_servers` field unless the task genuinely requires an MCP server\n\
         - Every `[[task.verify]]` entry has `phase` and `command`\n\
         - Output ONLY a fenced ```toml block followed optionally by a fenced \
           ```plan.md block — no prose, no explanation outside those blocks\n\n\
         {template_guidance}\n\
         {kind} content:\n{content}{extra}",
        read_files = budget.read_files,
    )
}

/// The tasks of `plan` that write into `plan_dir`, the directory the plan is
/// written to (bug-27f00b), each as "T1 writes `plans/x/plan.md`".
fn own_plan_writers(plan: &TasksFile, plan_dir: &Path, workdir: &Path) -> Vec<String> {
    let Ok(own) = plan_dir.strip_prefix(workdir) else {
        return Vec::new();
    };
    let own = own.to_string_lossy();
    let own = own.trim_end_matches('/');
    plan.tasks
        .iter()
        .filter_map(|task| {
            let file = task.files.iter().find(|file| {
                let file = file.trim_start_matches("./");
                file == own
                    || file
                        .strip_prefix(own)
                        .is_some_and(|rest| rest.starts_with('/'))
            })?;
            Some(format!("{} writes `{file}`", task.id))
        })
        .collect()
}

/// `plan_md` with a "Repairs" section that lists what roko changed in the
/// planner's draft before it validated it (gap-0fb7d9), when it changed
/// anything: a person reading the plan sees what the planner wrote and what
/// roko fixed. The Hermes plugin's plan sheet finds the section by its
/// heading, its first line and its `- ` items: keep all three as they are.
fn with_repairs(mut plan_md: String, repairs: &[String]) -> String {
    if repairs.is_empty() {
        return plan_md;
    }
    if !plan_md.ends_with('\n') {
        plan_md.push('\n');
    }
    plan_md
        .push_str("\n## Repairs\n\nroko changed the planner's draft before it validated it:\n\n");
    for repair in repairs {
        plan_md.push_str(&format!("- {}\n", repair.replace('\n', " ")));
    }
    plan_md
}

/// The plan report's note on the build-registration files several tasks
/// list (bug-daa54e): which, and how their tasks run: together with a
/// worktree per task, their edits meeting at merge, and in turn in one shared
/// tree. `None` when no task shares one.
fn shared_registration_note(plan: &TasksFile) -> Option<String> {
    let mut shared: Vec<(String, Vec<String>)> = Vec::new();
    for task in &plan.tasks {
        for file in task
            .files
            .iter()
            .filter(|file| roko_graph::exclusion::is_registration_file(file))
        {
            match shared.iter_mut().find(|(listed, _)| listed == file) {
                Some((_, tasks)) => tasks.push(task.id.clone()),
                None => shared.push((file.clone(), vec![task.id.clone()])),
            }
        }
    }
    let shared: Vec<String> = shared
        .into_iter()
        .filter(|(_, tasks)| tasks.len() > 1)
        .map(|(file, tasks)| format!("`{file}` ({})", tasks.join(", ")))
        .collect();
    (!shared.is_empty()).then(|| {
        format!(
            "Shared build-registration files: {}. With a worktree per task, tasks that share \
             only these still run in parallel, and their edits meet at merge; in one shared tree \
             they take turns.",
            shared.join(", ")
        )
    })
}

/// The most top-level entries [`workspace_note`] names.
const WORKSPACE_NOTE_ENTRIES: usize = 20;

/// The planner prompt's note on a workspace it gets no repository map of
/// (bug-27f00b): what its top level holds, hidden entries left out, and
/// that a file a task creates belongs in its `files`.
fn workspace_note(workdir: &Path) -> String {
    let mut entries: Vec<String> = std::fs::read_dir(workdir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
                    (!name.starts_with('.')).then(|| if dir { format!("{name}/") } else { name })
                })
                .collect()
        })
        .unwrap_or_default();
    entries.sort();
    let more = entries.len().saturating_sub(WORKSPACE_NOTE_ENTRIES);
    entries.truncate(WORKSPACE_NOTE_ENTRIES);
    let held = match (entries.is_empty(), more) {
        (true, _) => "nothing".to_string(),
        (false, 0) => entries.join(", "),
        (false, more) => format!("{} and {more} more", entries.join(", ")),
    };
    format!(
        "## Workspace\n\nIts top level holds: {held}. A file a task creates goes in that task's \
         `files`, at the path it will have.\n"
    )
}

/// The most failure patterns the planner prompt lists.
const KNOWN_FAILURE_PATTERNS: usize = 5;

/// The planner prompt's "Known failure patterns" block (backlog 4126): up to
/// [`KNOWN_FAILURE_PATTERNS`] patterns of the workspace's error-pattern store
/// that recurred, have a verified fix and are about a crate whose
/// `crates/<name>` path `source` mentions (`ErrorPatternStore::resolved_for`),
/// with the keys of those patterns. `None` when none applies.
fn known_failure_patterns(workdir: &Path, source: &str) -> Option<(String, Vec<String>)> {
    use roko_learn::error_pattern_store::{ERROR_PATTERNS_FILE, ErrorPatternStore};
    use std::fmt::Write as _;

    let paths: Vec<String> = source
        .split_whitespace()
        .filter(|word| word.contains("crates/"))
        .map(str::to_string)
        .collect();
    if paths.is_empty() {
        return None;
    }
    let learn_dir = workdir.join(".roko").join("learn");
    let store = ErrorPatternStore::load(&learn_dir.join(ERROR_PATTERNS_FILE));
    let patterns = store.resolved_for(&paths, KNOWN_FAILURE_PATTERNS);
    if patterns.is_empty() {
        return None;
    }
    let mut block = String::from(
        "## Known failure patterns\n\
         These failures kept recurring in the crates this plan touches, each with the fix a \
         verified retry recorded. Plan for them: a verify step that catches them, a narrower \
         task, or the pitfall named in the task description.\n",
    );
    for (index, pattern) in patterns.iter().enumerate() {
        let (digest, seen) = (&pattern.digest, pattern.occurrences);
        let _ = writeln!(block, "{}. {digest} (seen {seen} times)", index + 1);
        if let Some(gate) = &pattern.gate {
            let _ = writeln!(block, "   Verify: {gate}");
        }
        if let Some(fix) = &pattern.resolution {
            let _ = writeln!(block, "   Fix: {fix}");
        }
    }
    let keys = patterns.iter().map(|pattern| pattern.key.clone()).collect();
    Some((block, keys))
}

/// The slug of the plan in `plan_dir`: its `meta.plan`, else the directory's
/// name.
#[must_use]
pub fn plan_dir_slug(plan_dir: &Path) -> String {
    TasksFile::parse(&plan_dir.join("tasks.toml"))
        .ok()
        .map(|tasks| tasks.meta.plan)
        .filter(|plan| !plan.trim().is_empty())
        .unwrap_or_else(|| {
            plan_dir.file_name().map_or_else(
                || "unknown-plan".to_string(),
                |name| name.to_string_lossy().into_owned(),
            )
        })
}

/// Write `validated_toml`, a regeneration of `regeneration`'s plan, into
/// `plan_dir`, keeping done tasks done.
fn write_regenerated_plan(
    regeneration: &Regeneration,
    plan_dir: &Path,
    validated_toml: &str,
) -> Result<()> {
    let regenerated = TasksFile::parse_str(validated_toml)?;
    let merged =
        preserve_completed_task_status(regeneration.existing.as_ref(), regenerated, plan_dir);
    let rendered = toml::to_string_pretty(&merged).context("serialize regenerated tasks.toml")?;
    atomic_write_str(&plan_dir.join("tasks.toml"), &rendered)
        .with_context(|| format!("write tasks.toml to {}", plan_dir.display()))?;
    println!("📋 Regenerated tasks.toml in {}", plan_dir.display());
    Ok(())
}

/// Why a planner call failed, for [`PlanProgress::reason`]: the time limit it
/// ran into, or its crash class's hint (gap-11d870).
fn planner_call_failure(class: AgentCrashClass, output: &str) -> String {
    match timed_out_after_secs(output) {
        Some(limit) => {
            format!("the planner call ran into roko's {limit} s limit for one call and was stopped")
        }
        None => format!("the planner call failed: {}", class.recovery_hint()),
    }
}

/// Generate one plan from `request`'s source. The planner model writes a
/// tasks.toml, which is repaired, validated and checked against the
/// generated-plan policy (retrying, and escalating the model, when it fails)
/// before it is written under the plans root.
pub async fn generate_plan(request: PlanRequest<'_>) -> Result<(PathBuf, GenerationOutcome)> {
    let PlanRequest {
        source,
        slug,
        workdir,
        plans_root: requested_plans_root,
        context,
        dry_run,
        failure_context,
        model,
        effort,
        live,
        unattended,
        for_chat_host,
        progress,
        max_usd,
        source_path,
    } = request;
    let requested_plans_root = requested_plans_root.or(match source {
        PlanSource::Regenerate(plan_dir) => plan_dir.parent(),
        _ => None,
    });
    let workdir = workdir.to_path_buf();
    async {
        let t_total = Instant::now();
        // gap-61c95c: where the generation's time goes, in its progress.
        let clock = PlanClock::new(t_total);
        let t_phase = Instant::now();
        let source = ReadSource::read(source)?;
        let template_kind = source.template;
        let template_guidance = crate::plan_generate::render_plan_template_guidance(template_kind);
        println!("📋 Generating a plan from the {}: {slug}", source.kind);

        let dry_run_workdir = if dry_run {
            Some(dry_run_fs::DryRunWorkspace::new(&workdir)?)
        } else {
            None
        };
        let workdir_ref = dry_run_workdir
            .as_ref()
            .map_or(workdir.as_path(), |temp| temp.path());
        // Every agent call below is recorded against the plan as it returns,
        // the same way task dispatch records its spend.
        let spend = AuthoringSpend::generation(workdir_ref, slug, live);

        let resolved = crate::load_resolved_config(workdir_ref)?;
        // gap-18bfa2: what the planner calls may cost together. Once they
        // have, no further call is made.
        let cap = max_usd
            .unwrap_or_else(|| {
                roko_core::config::loader::load_config_unified(workdir_ref).map_or(
                    roko_core::config::schema::DEFAULT_MAX_GENERATION_USD,
                    |config| config.authoring.max_generation_usd,
                )
            })
            .max(0.0);
        let capped = (cap > 0.0).then_some(cap);
        let cap_reached = || {
            capped.filter(|cap| spend.spent_usd() >= *cap).map(|cap| {
                format!(
                    "plan generation reached its spend cap: its {} planner call(s) cost \
                     ${:.4} of ${cap:.2}",
                    spend.calls(),
                    spend.spent_usd()
                )
            })
        };
        // An improvement the cap leaves out is said, and skipped.
        let skip_for_cap = |improvement: &str| match cap_reached() {
            Some(reached) => {
                eprintln!("  {reached}, so the planner is not asked to {improvement}");
                true
            }
            None => false,
        };
        // Generated `rung` hints must name a rung of this ladder.
        let ladder = crate::plan_validate::workspace_ladder(workdir_ref);
        // Callers that pass no model (the serve runtime) plan with the
        // planner model.
        let planner_model = match model {
            Some(model) => model.to_string(),
            None if for_chat_host => {
                resolve_chat_host_planner_model(workdir_ref, "a chat host's request")?
            }
            None => resolve_planner_model(workdir_ref, None, "plan generation")?,
        };
        // bug-a0432b: a chat host's plan writing is distilled by its planner,
        // and its retries escalate only to a model roko's own tool loop runs.
        let distiller = for_chat_host.then_some(planner_model.as_str());
        let tool_loop_models = if for_chat_host {
            roko_core::config::loader::load_config_unified(workdir_ref)
                .map(|config| config.tool_loop_models())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        // gap-11d870: say what the generation is doing each time it moves
        // on. A stage that makes a planner call counts that call.
        let last_stage = std::sync::Mutex::new(None::<PlanProgress>);
        let emit = |stage: PlanProgress| {
            if let Some(progress) = &progress {
                progress(&stage);
            }
            *last_stage
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(stage);
        };
        let report = |stage: &'static str,
                      (attempt, attempts): (u32, u32),
                      reason: Option<String>,
                      model: &str| {
            let calling = !matches!(stage, "writing" | "recording");
            let (elapsed_ms, phases) = clock.read();
            emit(PlanProgress {
                stage,
                attempt,
                attempts,
                reason,
                model: model.to_string(),
                calls: spend.calls() + u32::from(calling),
                cost_usd: spend.spent_usd(),
                max_usd: capped,
                elapsed_ms,
                phases,
            });
        };
        // bug-1602d4: the stage again once its planner call has ended, with
        // the call counted and its spend in, rather than only when the next
        // stage starts.
        let report_spend = || {
            let last = last_stage
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            if let Some(stage) = last {
                let (elapsed_ms, phases) = clock.read();
                emit(PlanProgress {
                    calls: spend.calls(),
                    cost_usd: spend.spent_usd(),
                    elapsed_ms,
                    phases,
                    ..stage
                });
            }
        };
        let system = augment_generator_system_prompt(
            crate::plan_generate::build_generator_system_prompt(workdir_ref),
            failure_context,
        );
        // A requested plans root in the workspace follows a dry run into its
        // scratch copy.
        let plans_root = requested_plans_root.map_or_else(
            || workspace_plans_dir(workdir_ref),
            |root| {
                root.strip_prefix(&workdir).map_or_else(
                    |_| root.to_path_buf(),
                    |relative| workdir_ref.join(relative),
                )
            },
        );
        // A regenerated plan keeps its directory, whatever its slug.
        let plan_dir = match &source.regeneration {
            Some(regeneration) => {
                plans_root.join(regeneration.plan_dir.file_name().unwrap_or_default())
            }
            None => plans_root.join(slug),
        };
        let planner_effort = effort.unwrap_or(resolved.config.agent.effort.as_str());
        let tasks_before = dry_run_fs::snapshot_tasks_files(&plans_root);
        let init_ms = t_phase.elapsed().as_millis();

        // Build repo context to ground the planning agent in actual repository
        // structure. Keywords come from the plan slug and the source's title.
        let t_phase = Instant::now();
        let title = source.title.as_str();
        let mut feature_keywords: Vec<String> = slug
            .split(|c: char| c == '-' || c == '_' || c.is_whitespace())
            .chain(title.split(|c: char| c == '-' || c == '_' || c.is_whitespace()))
            .filter(|w| w.len() >= 3)
            .map(|w| w.to_lowercase())
            .collect();
        feature_keywords.sort_unstable();
        feature_keywords.dedup();
        feature_keywords.truncate(10);
        let keyword_refs: Vec<&str> = feature_keywords.iter().map(String::as_str).collect();
        // Skip repo context scanning for workspaces without source code
        // (e.g. freshly-initialized workspaces from `roko init`).
        let has_source_code = workdir_ref.join("src").is_dir()
            || workdir_ref.join("crates").is_dir()
            || workdir_ref.join("lib").is_dir()
            || workdir_ref.join("Cargo.toml").is_file()
            || workdir_ref.join("package.json").is_file();
        let repo_context_section: Option<String> = if has_source_code {
            match crate::repo_context::build_repo_context(workdir_ref, &keyword_refs).await {
                Ok(repo_context) => {
                    if !repo_context.context_root_verified {
                        eprintln!(
                            "warning: repository context not verified for keywords {:?}; \
                             generated plan may reference nonexistent code.",
                            feature_keywords
                        );
                    }
                    Some(repo_context.to_prompt_section())
                }
                Err(err) => {
                    eprintln!(
                        "warning: repository context unavailable for keywords {:?}: {err}",
                        feature_keywords
                    );
                    None
                }
            }
        } else {
            None // Empty workspace — skip context scanning
        };
        let context_ms = t_phase.elapsed().as_millis();
        let t_phase = Instant::now();
        let mut extra = String::new();
        if let Some(context) = context.map(str::trim).filter(|context| !context.is_empty()) {
            extra.push_str("\n\n");
            extra.push_str(context);
        }
        if let Some(repo_context) = &repo_context_section {
            extra.push_str("\n\n---\n\n");
            extra.push_str(repo_context);
        } else {
            // bug-27f00b: without a repository map, the planner still sees
            // what the workspace holds, and that a new file is a task's to
            // create.
            extra.push_str("\n\n---\n\n");
            extra.push_str(&workspace_note(workdir_ref));
        }
        // What has kept failing in the crates the source touches, and what
        // fixed it (backlog 4126). Nothing withholds the block yet, so its
        // pattern ids are logged for a later plan-level arm to measure it.
        if let Some((patterns, pattern_ids)) = known_failure_patterns(&workdir, &source.content) {
            tracing::info!(
                slug,
                ?pattern_ids,
                "plan generate: known failure patterns in the planner prompt"
            );
            extra.push_str("\n\n---\n\n");
            extra.push_str(&patterns);
        }
        // gap-dc5d7c: the checks the source states, each to be kept as a
        // verify step or dropped with a reason.
        let mut checks = source_checks::source_checks(&source.content);
        // Round v7, phase 0: a chat host that read a spec sends a summary
        // that names it and states no check. Such a request is planned with
        // the checks of the documents it names, which the planner reads.
        let documents = if checks.is_empty() {
            let mut documents = source_checks::named_documents(&source.content, workdir_ref);
            documents.retain(|document| !source_checks::source_checks(&document.text).is_empty());
            checks = source_checks::document_checks(&documents);
            documents
        } else {
            Vec::new()
        };
        if !documents.is_empty() {
            let names: Vec<String> = documents
                .iter()
                .map(|document| format!("`{}`", document.path))
                .collect();
            println!(
                "📋 The {} states no checks itself; planning with the {} checks of {}",
                source.kind,
                checks.len(),
                names.join(", ")
            );
            let chars = PlannerBudget::for_context_window(planner_context_window(
                &resolved.config.models,
                &planner_model,
            ))
            .source_chars;
            extra.push_str("\n\n---\n\n");
            extra.push_str(&source_checks::documents_block(
                &documents,
                &source.kind,
                chars,
            ));
        }
        // gap-ecafde: what this machine can run; bug-8fe462: the commands
        // that run checks stated as unittest ids, wildcards expanded.
        let probe = runnable::Probe::default();
        source_checks::attach_runs(&mut checks, workdir_ref, &probe);
        if !checks.is_empty() {
            tracing::info!(
                slug,
                checks = checks.len(),
                "plan generate: source checks in the planner prompt"
            );
            extra.push_str("\n\n---\n\n");
            extra.push_str(&source_checks::planner_block(
                &checks,
                &source_checks::checks_stated_by(&source.kind, &documents),
            ));
        }
        // gap-ecafde: the test runners this machine has, which a check must
        // use to be able to pass.
        if let Some(runners) = runnable::runners_block(workdir_ref, &probe) {
            extra.push_str("\n\n---\n\n");
            extra.push_str(&runners);
        }
        if unattended {
            extra.push_str("\n\n---\n\n");
            extra.push_str(UNATTENDED_PLANNER_NOTE);
        }

        // A planner with a large context window sees the whole source
        // (gap-2623b2); the old fixed caps are the floor.
        let budget = PlannerBudget::for_context_window(planner_context_window(
            &resolved.config.models,
            &planner_model,
        ));
        let task_prompt = plan_task_prompt(&source, slug, budget, &template_guidance, &extra);

        let prompt_ms = t_phase.elapsed().as_millis();
        let t_phase = Instant::now();
        let (task_id, task_kind) = source.episode(slug);
        let effective_model = Some(planner_model.as_str());
        let plan_agent_command =
            command_from_config(workdir_ref).unwrap_or_else(|| "claude".to_string());
        let plan_started = Instant::now();
        eprintln!("  Generating plan: {slug}");
        // Every planner call is the strategist with read-only tools under the
        // planner's own turn cap (bug-09cc5f); the calls after this one change
        // its prompt, and a validation retry its model.
        let planner_call = AgentExecOpts {
            prompt: &task_prompt,
            workdir: workdir_ref,
            model: effective_model,
            effort: Some(planner_effort),
            system_prompt: Some(&system),
            resume_session: None,
            env_vars: &resolved.config.agent.env,
            role: Some("strategist"),
            allowed_tools: Some("Read,Grep,Glob"),
            max_turns: Some(PLANNER_MAX_TURNS),
        };
        clock.lifted();
        report("planning", (1, 1), None, planner_model.as_str());
        let call = clock
            .planner(run_agent_capture_silent_with_usage(planner_call))
            .await?;
        spend.record(&call).await;
        report_spend();
        let (exit_code, output) = (call.exit_code, call.output);
        let agent_ms = t_phase.elapsed().as_millis();
        let t_phase = Instant::now();
        if exit_code == 0 {
            eprintln!("  ✓ Plan generated for: {slug}");
        } else {
            eprintln!("  ✗ Plan generation failed");
        }
        tracing::info!(
            exit_code,
            output_len = output.len(),
            output_trimmed_len = output.trim().len(),
            "plan generate: agent returned"
        );
        // ── Crash classification and retry for non-zero exit codes ──────
        let (_exit_code, output) = if exit_code != 0 {
            let crash_class = classify_agent_crash(&output);
            let _ = persist_capture_episode_distilled_by(
                workdir_ref,
                &plan_agent_command,
                effective_model,
                task_kind,
                &task_id,
                &task_prompt,
                &output,
                false,
                plan_started.elapsed().as_millis() as u64,
                None,
                distiller,
            )
            .await;

            // Auth errors: fail fast with actionable message.
            if matches!(crash_class, AgentCrashClass::AuthenticationError) {
                return Err(anyhow!(
                    "plan generation agent auth failure (exit code {exit_code}): {}{}\n\
                     Hint: check the API key of the planner model `{planner_model}`'s provider \
                     in roko.toml or ~/.roko/.env",
                    crash_class.recovery_hint(),
                    agent_said(&output)
                ));
            }

            // bug-09cc5f: a planner that used every turn it had is not asked
            // again; the same prompt under the same cap would most likely
            // stop there too.
            if matches!(crash_class, AgentCrashClass::TurnCapReached) {
                return Err(anyhow!(
                    "plan generation stopped: the planner ran out of turns before it wrote the \
                     plan. Hint: {}{}",
                    crash_class.recovery_hint(),
                    agent_said(&output)
                ));
            }

            // Non-retriable errors (model not found, context overflow): fail
            // immediately because a retry will hit the same permanent error.
            if !crash_class.is_retriable() {
                let preview = crate::run::truncate(&output, AGENT_ERROR_PREVIEW_CHARS);
                return Err(anyhow!(
                    "plan generation agent failed (exit code {exit_code}, {crash_class:?}): \
                     {preview}\nHint: {}",
                    crash_class.recovery_hint()
                ));
            }

            // Retriable errors (rate limit, network, timeout, unknown): retry
            // with exponential back-off. Unknown crashes (signal/OOM/transient)
            // get 2 attempts; rate-limit / network get 3. A call that ran into
            // the time limit gets 1: each attempt can take that long again
            // (gap-11d870).
            let max_crash_retries: u32 = match crash_class {
                AgentCrashClass::Timeout => 1,
                AgentCrashClass::Unknown => 2,
                _ => 3,
            };
            let mut last_exit_code = exit_code;
            let mut last_output = output;

            for attempt in 1..=max_crash_retries {
                let retry_class = classify_agent_crash(&last_output);
                if let Some(reached) = cap_reached() {
                    return Err(anyhow!(
                        "{reached}, so it stopped before retrying a planner call that failed: \
                         {}{}\nHint: raise `[authoring] max_generation_usd`, or pass \
                         `--max-usd` to `roko plan generate`",
                        retry_class.recovery_hint(),
                        agent_said(&last_output)
                    ));
                }
                report(
                    "retrying",
                    (attempt + 1, max_crash_retries + 1),
                    Some(planner_call_failure(retry_class, &last_output)),
                    planner_model.as_str(),
                );
                // Exponential back-off: 1 s, 2 s, 4 s …
                let backoff_secs = 1u64 << (attempt - 1).min(3);
                eprintln!(
                    "  plan generation agent {} (attempt {}/{}) \u{2014} \
                     waiting {backoff_secs}s then retrying\u{2026}",
                    retry_class.recovery_hint(),
                    attempt,
                    max_crash_retries + 1,
                );
                // The backoff is the planner's time, as its retry is.
                let retry = clock
                    .planner(async {
                        tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                        run_agent_capture_silent_with_usage(planner_call).await
                    })
                    .await?;
                spend.record(&retry).await;
                report_spend();
                last_exit_code = retry.exit_code;
                last_output = retry.output;
                if last_exit_code == 0 {
                    eprintln!("  \u{2713} Retry {attempt} succeeded");
                    break;
                }
            }

            if last_exit_code != 0 {
                let final_class = classify_agent_crash(&last_output);
                if let Some(limit) = timed_out_after_secs(&last_output) {
                    return Err(anyhow!(
                        "plan generation stopped after {} planner calls: the last ran into \
                         roko's {limit} s limit for one call and was stopped. Hint: {}",
                        max_crash_retries + 1,
                        final_class.recovery_hint(),
                    ));
                }
                return Err(anyhow!(
                    "plan generation agent failed after {} attempts \
                     (last exit code {last_exit_code}): {}{}",
                    max_crash_retries + 1,
                    final_class.recovery_hint(),
                    agent_said(&last_output)
                ));
            }

            (last_exit_code, last_output)
        } else {
            (exit_code, output)
        };
        if output.trim().is_empty() {
            let _ = persist_capture_episode_distilled_by(
                workdir_ref,
                &plan_agent_command,
                effective_model,
                task_kind,
                &task_id,
                &task_prompt,
                &output,
                false,
                plan_started.elapsed().as_millis() as u64,
                None,
                distiller,
            )
            .await;
            return Err(anyhow!(
                "plan generation agent returned empty output for {slug} — \
                 the model may not support the required output format"
            ));
        }

        // Write files from agent output (strategist can't write files directly).
        // Try fenced ```toml block first, then ```tasks.toml, then unfenced TOML.
        //
        // If extraction or validation fails, retry up to 2 times with the
        // request, the reply and its errors (bug-f0d60c).
        // gap-9e212c: the texts whose notes may size the plan's tasks.
        let size_sources: Vec<&str> = std::iter::once(source.content.as_str())
            .chain(documents.iter().map(|document| document.text.as_str()))
            .collect();
        // bug-60ad88: the workspace documents the plan is written from, the
        // file the source is and the documents it names, which every task
        // reads.
        let mut source_documents: Vec<String> = source_path
            .and_then(|path| source_reads::workspace_document(path, &workdir))
            .into_iter()
            .collect();
        for document in &documents {
            if !source_documents.contains(&document.path) {
                source_documents.push(document.path.clone());
            }
        }
        // bug-7c6f1b: a source that adds no file (phase 2.1's "No new
        // files") gets no plan that lists one.
        let no_new_files = size_sources
            .iter()
            .any(|text| file_paths::says_no_new_files(text));
        // gap-5c7f9a: the task count the notes for the planner state, and a
        // draft's count when it differs.
        let stated_tasks = sizes::stated_task_count(&size_sources);
        let count_off = |plan: &str| -> Option<(usize, usize)> {
            let stated = stated_tasks?;
            let drafted = TasksFile::parse_str(plan).ok()?.tasks.len();
            (drafted != stated).then_some((stated, drafted))
        };
        // gap-0fb7d9: with the draft it validates, what roko changed in it.
        let extract_and_validate =
            |raw: &str| -> std::result::Result<(String, Vec<String>), String> {
                let toml_content = extract_fenced_block(raw, "toml")
                    .or_else(|| extract_fenced_block(raw, "tasks.toml"))
                    .or_else(|| extract_toml_content_fallback(raw));
                tracing::info!(
                    has_toml_block = toml_content.is_some(),
                    toml_block_len = toml_content.map(|s| s.len()).unwrap_or(0),
                    "plan generate: fenced block extraction"
                );
                let toml_content = toml_content
                    .ok_or_else(|| "no TOML block found in agent output".to_string())?;

                // Fast structural pre-check: verify required sections are present
                // before attempting a full TOML parse.
                if !toml_content.contains("[meta]") {
                    return Err("TOML block is missing the required [meta] section".to_string());
                }
                if !toml_content.contains("[[task]]") {
                    return Err("TOML block is missing required [[task]] entries".to_string());
                }

                // gap-4dd77d: a verify step written as its bare command is
                // that step, not a parse error and a new draft.
                let (toml_content, wrapped) = wrap_command_steps(toml_content);
                let validated = validate_and_fix_generated_plan(
                    &toml_content,
                    slug,
                    &resolved.config.models,
                    resolved.config.agent.model.as_deref(),
                    &ladder,
                )
                .map_err(|e| format!("{e:#}"))?;
                let mut parsed = TasksFile::parse_str(&validated).map_err(|error| {
                    format!("generated plan failed runtime parsing after repair: {error:#}")
                })?;
                let policy = crate::plan_policy::PlanExecutionPolicy::generated_for_environment(
                    template_kind.max_task_count(),
                );
                // gap-b95913: what the planner need not be asked to fix; and
                // gap-dc5d7c: a step that covers a source check by its id names
                // a criterion of its task that states the check.
                let mut repairs = wrapped;
                repairs.extend(repair_generated_draft(
                    &mut parsed,
                    policy,
                    workdir_ref,
                    &source_documents,
                ));
                // bug-7c6f1b: under "No new files", a `files` path the
                // workspace lacks names the file of that name its task reads;
                // bug-ce1e4b: never one the source names.
                repairs.extend(file_paths::repair_file_paths(
                    &mut parsed,
                    workdir_ref,
                    no_new_files,
                    &size_sources,
                ));
                // bug-60ad88: every task reads the documents the plan is
                // written from.
                // The documents are no part of a generated task's budget;
                // the lane that runs the plan still caps its reads.
                repairs.extend(source_reads::repair_source_reads(
                    &mut parsed,
                    workdir_ref,
                    &source_documents,
                    crate::plan_policy::PlanExecutionPolicy::for_environment()
                        .max_read_files_per_task,
                ));
                // bug-417c4a: a symbol a task anchors in a file it does not
                // read brings that file into its reads, or becomes a note
                // when no file holds it yet, instead of a new draft.
                repairs.extend(symbol_reads::repair_symbol_anchors(
                    &mut parsed,
                    workdir_ref,
                    policy,
                ));
                // gap-9e212c: a task whose size its source states gets the
                // smallest tier that covers it.
                repairs.extend(sizes::repair_tiers(&mut parsed, &size_sources, policy));
                repairs.extend(source_checks::normalize_covers(&mut parsed, &checks));
                // bug-96115c: a check the source names as the plan's final check
                // runs in `[[meta.verify]]`, its command unchanged.
                repairs.extend(source_checks::add_plan_checks(&mut parsed, &checks));
                // gap-ecafde: a pytest step without pytest runs as unittest, and a
                // test file a task's check needs joins its files.
                repairs.extend(runnable::repair(&mut parsed, workdir_ref, &probe));
                // bug-d758f6: an implementer keeps its role's own tools, a shell
                // and edit_file among them, whatever list the planner copied.
                repairs.extend(runnable::repair_shell_tools(&mut parsed));
                // bug-df9aff: a task that creates a file may write it, and a task
                // waits for the tasks that write the modules its tests import.
                repairs.extend(runnable::repair_write_tools(&mut parsed, workdir_ref));
                repairs.extend(runnable::repair_dependencies(&mut parsed, workdir_ref));
                // bug-daa54e: a task that creates a C or C++ source lists the
                // CMake file that must name it, unless a task it waits for
                // globs it in (bug-7a5aca).
                repairs.extend(runnable::repair_build_lists(
                    &mut parsed,
                    workdir_ref,
                    &size_sources,
                ));
                // bug-8fe462: a step that cannot run, over checks roko has the
                // commands for, runs those commands.
                let stuck = runnable::problems(&parsed, workdir_ref, &probe);
                repairs.extend(source_checks::apply_runs(&mut parsed, &checks, &stuck));
                // gap-8b1f4f: a check that comes with its own command is never
                // dropped, so its task runs it below.
                repairs.extend(source_checks::refuse_drops(&mut parsed, &checks));
                // bug-72dc0d: each source check whose command roko knows runs in
                // the task that owns it, whatever command the planner wrote.
                let stuck = runnable::problems(&parsed, workdir_ref, &probe);
                repairs.extend(source_checks::assign_runs(
                    &mut parsed,
                    &checks,
                    &stuck,
                    |task| policy.tier_size_limits(task.tier_class()).max_verify_steps,
                ));
                // bug-56a806: a step that runs a source check's command runs the
                // source's commands alone, without a guard of the planner's own.
                repairs.extend(source_checks::run_exactly(&mut parsed, &checks, |task| {
                    policy.tier_size_limits(task.tier_class()).max_verify_steps
                }));
                // bug-8723fc: an implementer with no check builds with the
                // workspace's own build system, never one it lacks.
                repairs.extend(runnable::repair_build_checks(
                    &mut parsed,
                    workdir_ref,
                    &probe,
                ));
                // bug-3d0414: a criterion no step covers that quotes one
                // step's command whole is that step's.
                repairs.extend(source_checks::cover_quoted_criteria(&mut parsed));
                // gap-4dd77d: a task over its tier's size limits takes the
                // smallest tier it fits, as the size check suggests, instead
                // of a new draft; a task no tier fits still goes back.
                repairs.extend(sizes::raise_tiers(&mut parsed, policy));
                let validated = if repairs.is_empty() {
                    validated
                } else {
                    for repair in &repairs {
                        eprintln!("  Repaired the draft: {repair}");
                    }
                    tracing::info!(?repairs, "plan generate: repaired the draft");
                    toml::to_string_pretty(&parsed)
                        .map_err(|error| format!("re-serialize the repaired draft: {error}"))?
                };
                // bug-16b798: every gate's findings go back at once, so that one
                // fix-up can answer them all (phase 2's budget findings hid its
                // tier-size ones until the next draft).
                let mut refusals = Vec::new();
                let listed = |lines: Vec<String>| {
                    lines
                        .iter()
                        .map(|line| format!("  - {line}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                let policy_issues = crate::plan_policy::validate_plan_budgets(&parsed, policy);
                if !policy_issues.is_empty() {
                    refusals.push(format!(
                        "generated plan violates the `{}` structural budget:\n{}",
                        template_kind.label(),
                        listed(policy_issues.iter().map(ToString::to_string).collect())
                    ));
                }
                // 3225 (decision 3203): a task over the size limits of every
                // tier is a spec defect the planner fixes, by a split, before
                // a cheap model meets it. One a higher tier fits was raised
                // above (gap-4dd77d, the round v7 lead's call): a new draft
                // cost a planner call and, in phase 4.3, the checks the
                // regeneration had put back.
                let size_issues = crate::plan_policy::validate_tier_sizes(&parsed, policy);
                if !size_issues.is_empty() {
                    refusals.push(format!(
                        "generated plan has tasks over their tier's size limits; split each \
                     on independent outputs with disjoint files, or raise its tier:\n{}",
                        listed(size_issues.iter().map(ToString::to_string).collect())
                    ));
                }
                if no_new_files {
                    let added = file_paths::new_files(&parsed, workdir_ref);
                    if !added.is_empty() {
                        refusals.push(format!(
                            "generated plan lists files that do not exist, but the source says no \
                         new files: each task changes files the workspace has:\n{}",
                            listed(added)
                        ));
                    }
                }
                // bug-27f00b: the plan roko writes is the plan. A task that writes
                // into its directory writes a plan, not what the request asks for.
                let own_writers = own_plan_writers(&parsed, &plan_dir, workdir_ref);
                if !own_writers.is_empty() {
                    refusals.push(format!(
                    "generated plan has tasks that write the plan's own files, but the plan roko \
                     writes is the plan: its tasks build what the request asks for and write the \
                     project's files (a spec goes in `docs/spec.md`, in one task):\n{}",
                    listed(own_writers)
                ));
                }
                if !refusals.is_empty() {
                    return Err(refusals.join("\n"));
                }

                // Run the same context check that plan_loader.rs enforces at
                // load time so the written tasks.toml is guaranteed to pass.
                // PLAN_ARTIFACT_MISSING is suppressed because tasks.toml has
                // not been written yet.
                let mut ctx_violations = crate::plan_policy::validate_plan_context(
                    &parsed,
                    workdir_ref,
                    &plan_dir,
                    crate::plan_policy::PlanExecutionPolicy::for_environment(),
                );
                ctx_violations.retain(|v| v.code != "PLAN_ARTIFACT_MISSING");

                if ctx_violations.is_empty() {
                    return Ok((validated, repairs));
                }

                // Collect paths flagged as PLAN_CONTEXT_MISSING and drop them
                // so the written tasks.toml always passes the loader's check.
                let missing_paths: HashSet<String> = ctx_violations
                    .iter()
                    .filter(|v| v.code == "PLAN_CONTEXT_MISSING")
                    .filter_map(|v| {
                        v.message
                            .strip_prefix("declared context file `")
                            .and_then(|s| s.find('`').map(|end| s[..end].to_string()))
                    })
                    .collect();

                // bug-1c75cc: a range that starts inside its file but ends past
                // its end (a planner counting the empty line after the final
                // newline) is clamped to the file instead of costing the plan.
                let mut fixed = parsed.clone();
                let clamped = clamp_context_ranges_past_eof(&mut fixed, workdir_ref);

                if missing_paths.is_empty() && clamped == 0 {
                    // Nothing here can be repaired.
                    return Err(format!(
                        "generated plan violates execution context policy:\n{}",
                        ctx_violations
                            .iter()
                            .map(|v| format!("  - {v}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ));
                }

                // Drop the unreachable entries and re-serialize.
                for task in &mut fixed.tasks {
                    if let Some(ctx) = task.context.as_mut() {
                        ctx.read_files.retain(|f| !missing_paths.contains(&f.path));
                    }
                }
                let re_serialized = toml::to_string_pretty(&fixed)
                    .map_err(|e| format!("re-serialize after dropping missing read_files: {e}"))?;
                let re_parsed = TasksFile::parse_str(&re_serialized)
                    .map_err(|e| format!("re-parse after dropping missing read_files: {e}"))?;

                // Second context check — must be clean now.
                let mut remaining = crate::plan_policy::validate_plan_context(
                    &re_parsed,
                    workdir_ref,
                    &plan_dir,
                    crate::plan_policy::PlanExecutionPolicy::for_environment(),
                );
                remaining.retain(|v| v.code != "PLAN_ARTIFACT_MISSING");

                if remaining.is_empty() {
                    if !missing_paths.is_empty() {
                        let mut missing: Vec<&str> =
                            missing_paths.iter().map(String::as_str).collect();
                        missing.sort_unstable();
                        repairs.push(format!(
                            "dropped context files that do not exist: `{}`",
                            missing.join("`, `")
                        ));
                    }
                    if clamped > 0 {
                        repairs.push(format!(
                            "clamped {clamped} context line range(s) to the end of their file"
                        ));
                    }
                    Ok((re_serialized, repairs))
                } else {
                    Err(format!(
                        "generated plan still violates execution context policy \
                     after dropping {} missing read_files and clamping {clamped} line \
                     ranges:\n{}",
                        missing_paths.len(),
                        remaining
                            .iter()
                            .map(|v| format!("  - {v}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ))
                }
            };

        // 3218: a spec hard fail (a verify step that can never fail, a missing
        // context file, ...) is a validation failure, so the retries below
        // name the task, the rule and the detail.
        //
        // 3221: the planner may write acceptance tests as `accept:<path>`
        // blocks; every `[task.accept]` src must be one of them (or a test
        // the plan already has), and the tests of the plan that is written
        // go beside its tasks.toml.
        let spec_config = resolved.config.spec_quality.clone();
        // A Mutex, not a RefCell: the closure below lives across awaits, and
        // the serve route needs this future to be Send.
        let accept_for: std::sync::Mutex<HashMap<String, Vec<accept_blocks::AcceptBlock>>> =
            std::sync::Mutex::default();
        // bug-7a08a8: the open questions an unattended plan's planner answered
        // itself, moved into their tasks, by the plan they were moved in.
        let folded_for: std::sync::Mutex<HashMap<String, Vec<String>>> =
            std::sync::Mutex::default();
        // gap-0fb7d9: what roko changed in each plan's draft, by the plan.
        let repairs_for: std::sync::Mutex<HashMap<String, Vec<String>>> =
            std::sync::Mutex::default();
        let try_extract_and_validate = |raw: &str| -> std::result::Result<String, String> {
            let (validated, repairs) = extract_and_validate(raw)?;
            let validated = if unattended {
                let (plan, folded) =
                    fold_answered_questions(&validated).map_err(|error| format!("{error:#}"))?;
                if !folded.is_empty() {
                    folded_for
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .insert(plan.clone(), folded);
                }
                plan
            } else {
                validated
            };
            let accept = accept_blocks::extract(raw)?;
            accept_blocks::check_sources(&validated, &accept, &plan_dir)?;
            match spec_scores(&validated, workdir_ref, &spec_config) {
                Some(scores) if !scores.hard_fails.is_empty() => Err(format!(
                    "generated plan has spec hard fails; every check must be able to fail \
                     and every context file must exist:\n{}",
                    scores
                        .hard_fails
                        .iter()
                        .map(|line| format!("  - {line}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )),
                _ => {
                    accept_for
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .insert(validated.clone(), accept);
                    repairs_for
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .insert(validated.clone(), repairs);
                    Ok(validated)
                }
            }
        };

        // gap-dc5d7c: what became of the source's checks in a candidate
        // plan; gap-ecafde: which of its verify steps cannot pass as written;
        // bug-27f00b: which of its tasks no check can see.
        let assess = |candidate: &str| {
            TasksFile::parse_str(candidate).ok().map(|plan| {
                let unrunnable = runnable::problems(&plan, workdir_ref, &probe);
                let unchecked = runnable::unchecked(&plan, workdir_ref, &probe);
                let coverage = source_checks::SourceCoverage::of(&checks, &plan, &unrunnable);
                (coverage, unrunnable, unchecked)
            })
        };
        // How far a candidate falls short: the source checks it does not
        // keep, then its steps that cannot pass and its tasks no check can
        // see (everything when it does not parse). Fewer is better, checks
        // first.
        let missing_of = |candidate: &str| {
            assess(candidate).map_or(
                (checks.len(), usize::MAX),
                |(coverage, unrunnable, unchecked)| {
                    (coverage.unkept(), unrunnable.len() + unchecked.len())
                },
            )
        };

        // First attempt uses the output we already have.
        let mut validated_toml = try_extract_and_validate(&output);
        // bug-f0d60c: each reply that held no plan that validates is kept.
        let mut rejected_drafts = 0u32;
        let mut keep_draft = |reply: &str, error: &str| {
            rejected_drafts += 1;
            if let Some(path) =
                keep_rejected_draft(workdir_ref, slug, rejected_drafts, reply, error)
            {
                eprintln!("  Kept the rejected draft in {}", path.display());
            }
        };
        if let Err(error) = &validated_toml {
            keep_draft(&output, error);
        }

        // gap-18bfa2: why the retries stopped early, if the cap stopped them.
        let mut stopped_at_cap: Option<String> = None;
        // Retry up to 2 times on extraction/validation failure, escalating
        // to a higher-tier model when TOML validation keeps failing.
        if validated_toml.is_err() {
            let max_retries = 2u32;
            let mut escalated_model: Option<String> = None;
            let mut last_output = output.clone();

            for attempt in 1..=max_retries {
                let t_retry = Instant::now();
                if let Some(reached) = cap_reached() {
                    eprintln!("  {reached}; not asking the planner again");
                    stopped_at_cap = Some(reached);
                    break;
                }

                // Escalate model on format/validation failures (not auth/network),
                // only ever upward from the planner model, and only to a model
                // this workspace configures.
                let current_model = escalated_model.as_deref().or(effective_model);
                if resolved.config.agent.escalation.escalate_model {
                    if let Some(next) = next_tier_model(
                        current_model,
                        &resolved.config.agent.tier_models,
                        &resolved.config.models,
                    ) && (!for_chat_host || tool_loop_models.contains(&next))
                    {
                        tracing::info!(
                            from = current_model.unwrap_or("<default>"),
                            to = next.as_str(),
                            attempt,
                            "plan generate: escalating model tier on validation failure"
                        );
                        eprintln!(
                            "⬆️  Escalating model: {} → {next}",
                            current_model.unwrap_or("<default>"),
                        );
                        escalated_model = Some(next);
                    }
                }
                let retry_model = escalated_model.as_deref().or(effective_model);

                tracing::warn!(
                    attempt,
                    max_retries,
                    model = retry_model.unwrap_or("<default>"),
                    err = validated_toml.as_ref().unwrap_err().as_str(),
                    "plan generate: TOML extraction/validation failed, retrying"
                );
                eprintln!(
                    "⚠️  The planner's reply held no plan that validates (attempt {}/{}); asking \
                     again with its errors…",
                    attempt,
                    max_retries + 1,
                );
                let error = validated_toml.as_ref().unwrap_err();
                report(
                    "fixing",
                    (attempt, max_retries),
                    Some(format!(
                        "the reply held no plan that validates: {}",
                        crate::run::truncate(error, PROGRESS_REASON_CHARS)
                    )),
                    retry_model.unwrap_or(planner_model.as_str()),
                );
                let retry_prompt = extraction_retry_prompt(
                    &task_prompt,
                    &source.kind,
                    &last_output,
                    error,
                    budget.source_chars,
                );
                let retry_result = clock
                    .planner(run_agent_capture_silent_with_usage(AgentExecOpts {
                        prompt: &retry_prompt,
                        model: retry_model,
                        ..planner_call
                    }))
                    .await;
                if let Ok(retry) = &retry_result {
                    spend.record(retry).await;
                    report_spend();
                }

                match retry_result.map(|retry| (retry.exit_code, retry.output)) {
                    Ok((0, retry_output)) if !retry_output.trim().is_empty() => {
                        validated_toml = try_extract_and_validate(&retry_output);
                        match &validated_toml {
                            Ok(_) => {
                                tracing::info!(
                                    attempt,
                                    model = retry_model.unwrap_or("<default>"),
                                    retry_ms = t_retry.elapsed().as_millis() as u64,
                                    "plan generate: retry succeeded"
                                );
                                eprintln!("✅ Retry {attempt} succeeded");
                                break;
                            }
                            Err(error) => keep_draft(&retry_output, error),
                        }
                        last_output = retry_output;
                    }
                    // Non-zero exit (auth/network errors) — do NOT escalate model,
                    // just log and retry with the same model.
                    Ok((code, _)) => {
                        tracing::warn!(
                            attempt,
                            code,
                            retry_ms = t_retry.elapsed().as_millis() as u64,
                            "plan generate: retry agent failed (not escalating — agent error)"
                        );
                        // Revert escalation: auth/network failures won't be fixed
                        // by a different model.
                        escalated_model = None;
                    }
                    Err(err) => {
                        tracing::warn!(
                            attempt,
                            %err,
                            retry_ms = t_retry.elapsed().as_millis() as u64,
                            "plan generate: retry agent error (not escalating — transport error)"
                        );
                        escalated_model = None;
                    }
                }
            }
        }

        // bug-59ed90: a check that hides its test runner's output fails
        // without saying why, so the planner is asked once more to rewrite
        // such checks, and the plan that hides fewer is kept.
        if let Ok(candidate) = &validated_toml {
            let hidden = hidden_output_lines(candidate);
            if !hidden.is_empty() && !skip_for_cap("rewrite the checks that hide their output") {
                eprintln!(
                    "  {} check(s) hide their test runner's output; asking the planner once more",
                    hidden.len()
                );
                report(
                    "regenerating",
                    (1, 1),
                    Some(format!(
                        "{} check(s) hide their test runner's output",
                        hidden.len()
                    )),
                    planner_model.as_str(),
                );
                let prompt = hidden_output_regeneration_prompt(&task_prompt, candidate, &hidden);
                let retry = clock
                    .planner(run_agent_capture_silent_with_usage(AgentExecOpts {
                        prompt: &prompt,
                        ..planner_call
                    }))
                    .await;
                if let Ok(retry) = &retry {
                    spend.record(retry).await;
                    report_spend();
                }
                if let Ok(retry) = retry
                    && retry.exit_code == 0
                    && let Ok(better) = try_extract_and_validate(&retry.output)
                    && hidden_output_lines(&better).len() < hidden.len()
                    && missing_of(&better) <= missing_of(candidate)
                {
                    validated_toml = Ok(better);
                }
            }
        }

        // gap-dc5d7c: a plan that does not keep some of the source's
        // checks (no step checks them, or only a grep or a build does, and no
        // reason says why no command can) is sent back once, naming them,
        // and the plan that keeps more is kept. gap-ecafde: so is a plan with
        // a verify step that cannot pass as written; bug-27f00b: and one with
        // a task no check can see.
        let mut checks_regenerated = false;
        if let Ok(candidate) = &validated_toml
            && let Some((coverage, unrunnable, unchecked)) = assess(candidate)
            && (coverage.unkept() > 0
                || !unrunnable.is_empty()
                || !unchecked.is_empty()
                || count_off(candidate).is_some())
            && !skip_for_cap("keep the source checks it left out")
        {
            let missing = (coverage.unkept(), unrunnable.len() + unchecked.len());
            let tasks = count_off(candidate);
            let mut parts = Vec::new();
            if let Some((stated, drafted)) = tasks {
                parts.push(format!(
                    "the plan has {drafted} task(s) where the source's notes state {stated}"
                ));
            }
            if coverage.unkept() > 0 {
                parts.push(format!(
                    "{} of the source's {} checks are not kept as verify steps",
                    coverage.unkept(),
                    checks.len()
                ));
            }
            if !unrunnable.is_empty() {
                parts.push(format!(
                    "{} verify step(s) cannot pass as written",
                    unrunnable.len()
                ));
            }
            if !unchecked.is_empty() {
                parts.push(format!(
                    "{} task(s) have no check that can see their work",
                    unchecked.len()
                ));
            }
            let why = parts.join(", and ");
            eprintln!("  {why}; asking the planner once more");
            report("regenerating", (1, 1), Some(why), planner_model.as_str());
            let prompt = source_checks::regeneration_prompt(
                &task_prompt,
                candidate,
                &coverage,
                &unrunnable,
                &unchecked,
                tasks,
            );
            let retry = clock
                .planner(run_agent_capture_silent_with_usage(AgentExecOpts {
                    prompt: &prompt,
                    ..planner_call
                }))
                .await;
            if let Ok(retry) = &retry {
                spend.record(retry).await;
                report_spend();
            }
            checks_regenerated = true;
            // bug-72dc0d: the plan asked for is kept only when it does
            // better, and when it is not, the person is told why.
            let better = match retry {
                Err(error) => Err(format!("the planner call failed: {error:#}")),
                Ok(retry) if retry.exit_code != 0 => Err(format!(
                    "the planner exited with status {}",
                    retry.exit_code
                )),
                Ok(retry) => match try_extract_and_validate(&retry.output) {
                    Err(error) => {
                        keep_draft(&retry.output, &error);
                        Err(format!("its plan does not validate: {error}"))
                    }
                    // gap-5c7f9a: as many checks kept and the notes' task
                    // count met is better too.
                    Ok(better)
                        if (missing_of(&better), count_off(&better).is_some())
                            >= (missing, tasks.is_some()) =>
                    {
                        let (unkept, problems) = missing_of(&better);
                        let count =
                            count_off(&better).map_or_else(String::new, |(stated, drafted)| {
                                format!(", and {drafted} task(s) where the notes state {stated}")
                            });
                        Err(format!(
                            "it leaves {unkept} source check(s) unkept and {problems} step(s) or \
                             task(s) that cannot pass or be seen{count}, against {} and {} before",
                            missing.0, missing.1
                        ))
                    }
                    Ok(better)
                        if hidden_output_lines(&better).len()
                            > hidden_output_lines(candidate).len() =>
                    {
                        Err("more of its checks hide their test runner's output".to_string())
                    }
                    Ok(better) => Ok(better),
                },
            };
            match better {
                Ok(better) => validated_toml = Ok(better),
                Err(why) => {
                    eprintln!("  The plan written again is not kept, so the first stands: {why}");
                    tracing::warn!(slug, %why, "plan generate: regeneration not kept");
                }
            }
        }

        // 3218: score the plan. A weak one (a mean below `[spec_quality]
        // allow_threshold`, or a task below `block_threshold`) is regenerated
        // once with each weak task's missing rules, and the better plan is
        // kept: no hard fail, a higher mean and no more checks that hide
        // their runner's output (bug-59ed90).
        let mut spec_quality = None;
        if let Ok(candidate) = &validated_toml
            && let Some(scores) = spec_scores(candidate, workdir_ref, &spec_config)
        {
            let mut kept = (candidate.clone(), scores);
            let weak = kept.1.is_weak(&spec_config);
            if weak && !skip_for_cap("raise the plan's spec quality") {
                eprintln!(
                    "  Spec quality is low (mean {:.1}, lowest task {:.1}); asking the planner \
                     once more",
                    kept.1.mean, kept.1.min
                );
                report(
                    "regenerating",
                    (1, 1),
                    Some(format!(
                        "the plan's spec quality is low (mean {:.1}, lowest task {:.1})",
                        kept.1.mean, kept.1.min
                    )),
                    planner_model.as_str(),
                );
                let prompt = spec_regeneration_prompt(&task_prompt, &kept.0, &kept.1);
                let retry = clock
                    .planner(run_agent_capture_silent_with_usage(AgentExecOpts {
                        prompt: &prompt,
                        ..planner_call
                    }))
                    .await;
                if let Ok(retry) = &retry {
                    spend.record(retry).await;
                    report_spend();
                }
                if let Ok(retry) = retry
                    && retry.exit_code == 0
                    && let Ok(better) = try_extract_and_validate(&retry.output)
                    && let Some(better_scores) = spec_scores(&better, workdir_ref, &spec_config)
                    && better_scores.mean > kept.1.mean
                    && hidden_output_lines(&better).len() <= hidden_output_lines(&kept.0).len()
                    && missing_of(&better) <= missing_of(&kept.0)
                {
                    kept = (better, better_scores);
                }
            }
            println!("📋 {}", kept.1.summary_line());
            spec_quality = Some(kept.1.record(weak));
            validated_toml = Ok(kept.0);
        }

        // gap-dc5d7c: what became of each of the source's checks; and
        // gap-ecafde: each step that still cannot pass as written;
        // bug-27f00b: each task no check can see.
        let assessed = validated_toml.as_ref().ok().and_then(|plan| assess(plan));
        if let Some((_, unrunnable, unchecked)) = &assessed {
            for step in unrunnable {
                eprintln!(
                    "warning: {} step {} cannot pass as written: {}",
                    step.task, step.step, step.why
                );
            }
            for task in unchecked {
                eprintln!(
                    "warning: no check of {} can see its work: {}",
                    task.task,
                    runnable::only_reads(&task.reads)
                );
            }
        }
        let source_coverage = assessed
            .map(|(coverage, _, _)| coverage)
            .filter(|_| !checks.is_empty());
        if let Some(coverage) = &source_coverage {
            println!("📋 {}", coverage.summary_line());
            for line in coverage.detail_lines() {
                println!("   {line}");
            }
            // bug-72dc0d: a plan that keeps fewer than half of the checks
            // its source states, after the planner was asked once more, is not
            // written: it would pass for work its source says is not done.
            if coverage.unkept() * 2 > checks.len() {
                if let Ok(plan) = &validated_toml {
                    keep_draft(plan, &coverage.summary_line());
                }
                let asked = if checks_regenerated {
                    ", after asking the planner once more"
                } else {
                    ""
                };
                return Err(anyhow!(
                    "plan generation wrote no plan for {slug}: it keeps {} of the {} checks its \
                     source states{asked} ({}). A check whose command the source gives runs in \
                     the task that owns it, the one whose files its command names; give each \
                     other check a task that runs it, or ask again.",
                    checks.len() - coverage.unkept(),
                    checks.len(),
                    coverage.summary_line()
                ));
            }
        }

        // gap-0fb7d9: what roko changed in the draft it writes.
        let mut repairs = validated_toml.as_ref().map_or_else(
            |_| Vec::new(),
            |validated| {
                repairs_for
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .remove(validated)
                    .unwrap_or_default()
            },
        );
        // gap-5c7f9a: a plan that splits its tasks other than the notes do
        // is written, and says so.
        if let Ok(plan) = &validated_toml
            && let Some((stated, drafted)) = count_off(plan)
        {
            let asked = if checks_regenerated {
                ", after asking the planner once more"
            } else {
                ""
            };
            let line = format!(
                "the plan has {drafted} task(s) where the source's notes for the planner state \
                 {stated}{asked}: check the split before it runs"
            );
            eprintln!("warning: {line}");
            repairs.push(line);
        }
        if let Ok(validated_toml) = validated_toml {
            report(
                "writing",
                (1, 1),
                source_coverage
                    .as_ref()
                    .map(source_checks::SourceCoverage::summary_line),
                planner_model.as_str(),
            );
            let accept = accept_for
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&validated_toml)
                .unwrap_or_default();
            let folded = folded_for
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&validated_toml)
                .unwrap_or_default();
            if let Some(regeneration) = &source.regeneration {
                write_regenerated_plan(regeneration, &plan_dir, &validated_toml)?;
            } else {
                std::fs::create_dir_all(&plan_dir)
                    .with_context(|| format!("create plan dir {}", plan_dir.display()))?;
                atomic_write_str(&plan_dir.join("tasks.toml"), &validated_toml)
                    .with_context(|| format!("write tasks.toml to {}", plan_dir.display()))?;
                println!(
                    "📋 Wrote tasks.toml ({} bytes) to {}",
                    validated_toml.len(),
                    plan_dir.display()
                );
                if let Some(note) = TasksFile::parse_str(&validated_toml)
                    .ok()
                    .and_then(|tasks| shared_registration_note(&tasks))
                {
                    println!("📋 {note}");
                }
                let plan_md_content = extract_fenced_block(&output, "plan.md")
                    .or_else(|| extract_fenced_block(&output, "markdown"))
                    .or_else(|| extract_fenced_block(&output, "md"));
                if let Some(plan_md) = plan_md_content {
                    let plan_md = with_repairs(plan_md.to_string(), &repairs);
                    atomic_write_str(&plan_dir.join("plan.md"), &plan_md)
                        .with_context(|| format!("write plan.md to {}", plan_dir.display()))?;
                    println!(
                        "📋 Wrote plan.md ({} bytes) to {}",
                        plan_md.len(),
                        plan_dir.display()
                    );
                } else {
                    // gap-11d870: the planner wrote no plan.md, so the plan
                    // gets an overview written from its tasks.
                    let plan_md = TasksFile::parse_str(&validated_toml).map_or_else(
                        |_| {
                            format!(
                                "---\nplan: {slug}\ntitle: {slug}\n---\n\n# {slug}\n\n\
                                 Generated plan.\n"
                            )
                        },
                        |tasks| overview::overview_plan_md(slug, &source.content, &tasks),
                    );
                    let plan_md = with_repairs(plan_md, &repairs);
                    atomic_write_str(&plan_dir.join("plan.md"), &plan_md)
                        .with_context(|| format!("write plan.md to {}", plan_dir.display()))?;
                }
            }

            // 3221: the planner-written tests beside tasks.toml; a test the
            // plan already had stays unless the planner wrote it again.
            accept_blocks::write(&plan_dir, &accept)
                .with_context(|| format!("write the accept tests to {}", plan_dir.display()))?;
            for question in &folded {
                println!("📋 Decided while planning, with nobody to ask: {question}");
            }
        } else {
            // All attempts (initial + retries) failed to produce valid TOML.
            let final_err = validated_toml.unwrap_err();
            let _preview: String = output.chars().take(500).collect();
            // bug-16b798: say why the last draft was refused. A draft that
            // parsed and failed roko's checks is no fencing problem, and a
            // more capable model is unlikely to fix it.
            let unfenced = final_err.starts_with("no TOML block found")
                || final_err.starts_with("TOML block is missing");
            let checked = final_err.starts_with("generated plan violates")
                || final_err.starts_with("generated plan has")
                || final_err.starts_with("generated plan still violates");
            let has_toml_like = output.contains("[meta]") || output.contains("[[task]]");
            let reason = if unfenced {
                let fencing = if has_toml_like {
                    "\nhint: The model output TOML without proper fencing. \
                     Try a more capable model or check the plan_generate system prompt."
                } else {
                    ""
                };
                format!(
                    "The agent output ({} bytes) did not contain a parseable ```toml block.\n\
                     hint: Try again, or create plans/{slug}/tasks.toml manually.{fencing}",
                    output.len()
                )
            } else if checked {
                "The last draft parsed, but did not pass roko's plan checks above.\n\
                 hint: these come from the request's shape or roko's plan limits, so a more \
                 capable model is unlikely to help: change the request or its spec, or ask again."
                    .to_string()
            } else {
                format!(
                    "The last draft's TOML did not parse as a plan.\n\
                     hint: Try again, or create plans/{slug}/tasks.toml manually."
                )
            };
            let _ = persist_capture_episode_distilled_by(
                workdir_ref,
                &plan_agent_command,
                effective_model,
                task_kind,
                &task_id,
                &task_prompt,
                &output,
                false,
                plan_started.elapsed().as_millis() as u64,
                None,
                distiller,
            )
            .await;
            eprintln!("--- Raw model output (first {FAILURE_OUTPUT_CHARS} chars) ---");
            eprintln!("{}", crate::run::truncate(&output, FAILURE_OUTPUT_CHARS));
            eprintln!("--- End raw model output ---");
            // A dry run's drafts went with its scratch copy of the workspace.
            let drafts = if rejected_drafts > 0 && !dry_run {
                format!(
                    "\nThe rejected drafts are in {}.",
                    workdir_ref
                        .join(".roko")
                        .join(REJECTED_DRAFTS_DIR)
                        .join(slug)
                        .display()
                )
            } else {
                String::new()
            };
            if let Some(reached) = stopped_at_cap {
                return Err(anyhow!(
                    "{reached}, and no draft validated, so no plan was written.\n\
                     Last error: {final_err}\n\
                     hint: raise `[authoring] max_generation_usd`, or pass `--max-usd` to `roko \
                     plan generate`.{drafts}"
                ));
            }
            return Err(anyhow!(
                "Plan generation failed after retries: no valid tasks.toml was produced.\n\
                 Last error: {final_err}\n\
                 {reason}{drafts}"
            ));
        }

        let extraction_ms = t_phase.elapsed().as_millis();
        tracing::info!(
            extraction_and_write_ms = extraction_ms,
            "plan generate: TOML extracted and written"
        );
        let t_phase = Instant::now();
        clock.validating();
        let generated_changed = dry_run_fs::changed_tasks_files(&plans_root, &tasks_before);

        let changed = dry_run_fs::changed_tasks_files(&plans_root, &tasks_before);
        let mut artifact_valid = true;
        let mut validation_report: Option<ArtifactValidationReport> = None;
        // bug-7a08a8: why the written plan cannot run, one line each.
        let mut invalid: Vec<String> = Vec::new();

        if dry_run {
            if changed.is_empty() {
                artifact_valid = false;
                invalid.push("no tasks.toml was written".to_string());
                eprintln!("warning: dry-run plan generation did not produce any tasks.toml files");
            } else {
                for path in &changed {
                    if let Err(err) = dry_run_fs::validate_and_print_preview(path) {
                        artifact_valid = false;
                        invalid.push(format!("{}: {err:#}", path.display()));
                        eprintln!(
                            "warning: dry-run validation failed for {}: {err:#}",
                            path.display()
                        );
                    }
                }
            }
        } else {
            dry_run_fs::warn_on_new_or_updated_tasks(&plans_root, &tasks_before);
        }

        let task_count = generated_task_count(&generated_changed)?;
        let max_tasks =
            crate::plan_policy::effective_generated_task_limit(template_kind.max_task_count());
        if task_count > max_tasks {
            eprintln!(
                "⚠️  Generated {task_count} tasks, which exceeds the `{}` template budget of {}",
                template_kind.label(),
                max_tasks
            );
        }

        // Validate the plan this call wrote, not the whole plans root: a broken
        // plan beside it must not fail the generation (bug-2d06bf).
        match crate::plan_validate::validate_plans_dir_with_workdir(
            &plan_dir,
            None,
            Some(workdir_ref),
        ) {
            Ok(report) => {
                if report.totals.errors > 0 {
                    artifact_valid = false;
                    eprintln!(
                        "warning: artifact validation found {} error(s) and {} warning(s) for {}",
                        report.totals.errors, report.totals.warnings, slug
                    );
                }
                invalid.extend(
                    report
                        .plans
                        .iter()
                        .flat_map(|plan| &plan.diagnostics)
                        .filter(|diagnostic| {
                            diagnostic.severity == crate::plan_validate::Severity::Error
                        })
                        .map(|diagnostic| {
                            format!("{} {}", diagnostic.rule_id, diagnostic.message)
                                .replace('\n', "\n  ")
                        }),
                );
                // bug-59ed90: a check the planner left hiding its runner's
                // output is said out loud, so the person can fix it first.
                for diagnostic in report
                    .plans
                    .iter()
                    .flat_map(|plan| &plan.diagnostics)
                    .filter(|diagnostic| diagnostic.rule_id == "PLAN_050")
                {
                    eprintln!("warning: PLAN_050 {}", diagnostic.message);
                }
                validation_report = serde_json::to_value(&report).ok();
                if validation_report.is_none() {
                    artifact_valid = false;
                    invalid.push("its validation report could not be recorded".to_string());
                }
            }
            Err(err) => {
                artifact_valid = false;
                invalid.push(format!("it could not be validated: {err:#}"));
                eprintln!(
                    "warning: artifact validation could not be completed for {}: {err:#}",
                    slug
                );
            }
        }

        let post_ms = t_phase.elapsed().as_millis();
        clock.validated();
        let total_ms = t_total.elapsed().as_millis();
        tracing::info!(
            init_ms,
            context_ms,
            prompt_ms,
            agent_ms,
            post_ms,
            total_ms,
            "plan generate: phase timing"
        );

        let outcome = GenerationOutcome {
            process_success: true,
            artifact_valid,
            validation_report,
            spec_quality,
            source_checks: source_coverage.map(|coverage| GenerationSourceChecks {
                documents: documents
                    .iter()
                    .map(|document| document.path.clone())
                    .collect(),
                ..coverage.record(checks_regenerated)
            }),
            cost_usd: spend.spent_usd(),
            repairs,
        };

        report("recording", (1, 1), None, planner_model.as_str());
        let _ = persist_capture_episode_distilled_by(
            workdir_ref,
            &plan_agent_command,
            effective_model,
            task_kind,
            &task_id,
            &task_prompt,
            &output,
            outcome.fully_successful(),
            plan_started.elapsed().as_millis() as u64,
            None,
            distiller,
        )
        .await;

        // bug-7a08a8: a plan that cannot run is an error, which names why, so
        // no caller reports it as written and ready.
        if !outcome.artifact_valid {
            let plan_dir = plan_dir
                .strip_prefix(workdir_ref)
                .map_or_else(|_| plan_dir.clone(), Path::to_path_buf);
            return Err(PlanFailsValidation {
                plan_dir,
                errors: invalid,
                cost_usd: outcome.cost_usd,
            }
            .into());
        }

        Ok((
            requested_plans_root.map_or_else(|| workspace_plans_dir(&workdir), Path::to_path_buf),
            outcome,
        ))
    }
    .await
}

pub(crate) fn augment_generator_system_prompt(
    mut system_prompt: String,
    failure_context: Option<&str>,
) -> String {
    let Some(failure_context) = failure_context.map(str::trim).filter(|ctx| !ctx.is_empty()) else {
        return system_prompt;
    };

    system_prompt.push_str("\n\n## Failure context for replanning\n");
    system_prompt.push_str(failure_context);
    system_prompt.push_str(
        "\n\nUse this failure context to revise the plan first. Do not reproduce the same task shape.\n",
    );
    system_prompt
}

/// Extract the contents of a fenced code block tagged with `tag` from agent output.
///
/// Looks for `` ```tag `` or `` ```<tag> `` and returns the inner content.
/// Handles nested fences by matching the closing `` ``` `` that sits alone
/// on a line (possibly with trailing whitespace).
pub(crate) fn extract_fenced_block<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let fence_plain = format!("```{tag}");
    let fence_angle = format!("```<{tag}>");
    let start = text
        .find(&fence_plain)
        .or_else(|| text.find(&fence_angle))?;
    let after_fence = &text[start..];
    let newline = after_fence.find('\n')? + 1;
    let inner = &after_fence[newline..];

    // Find a closing ``` that is alone on a line (not followed by more text
    // like ```toml or ```python — those are nested openers).
    let mut search_from = 0;
    loop {
        let candidate = inner[search_from..].find("\n```")?;
        let abs = search_from + candidate;
        let after_ticks = abs + 4; // position after \n```
        // Closing fence: either end-of-string, or next char is \n or whitespace-then-\n
        let rest = &inner[after_ticks..];
        if rest.is_empty()
            || rest.starts_with('\n')
            || rest.trim_start().starts_with('\n')
            || rest.trim_start().is_empty()
        {
            let content = inner[..abs].trim();
            return if content.is_empty() {
                None
            } else {
                Some(&inner[..abs])
            };
        }
        search_from = after_ticks;
    }
}

/// Fallback TOML extraction for agent output that lacks fenced code blocks.
///
/// Scans for a `[meta]` section header and returns everything from that point
/// up to the end of the TOML content, provided it also contains at least one
/// `[[task]]`.  Trailing explanatory text (markdown headings, horizontal rules,
/// prose paragraphs after a blank line) is trimmed so that `toml::from_str`
/// doesn't choke on non-TOML content the LLM appended after the plan.
pub(crate) fn extract_toml_content_fallback(output: &str) -> Option<&str> {
    let meta_start = output.find("[meta]")?;
    // Find the start of the line containing [meta]
    let line_start = output[..meta_start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let candidate = &output[line_start..];
    if !candidate.contains("[[task]]") {
        return None;
    }

    // Walk lines and find the last one that looks like TOML content.
    // Stop when we hit lines that are clearly markdown/prose trailing text.
    let mut last_toml_end = 0; // byte offset into `candidate`
    let mut saw_blank = false;
    for line in candidate.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            saw_blank = true;
            // A blank line inside TOML is fine (between sections), so we don't
            // stop yet — only if the *next* non-blank line looks non-TOML.
            last_toml_end += line.len() + 1; // +1 for '\n'
            continue;
        }

        // Lines that are clearly not TOML — stop here.
        if trimmed.starts_with("## ")
            || trimmed.starts_with("---")
            || trimmed.starts_with("**")
            || trimmed.starts_with("> ")
            || trimmed.starts_with("Note:")
            || trimmed.starts_with("NOTE:")
        {
            break;
        }

        // After a blank line, the next non-blank line must still look like TOML
        // (contains `=`, starts with `[`, or is a `"""` / `'''` continuation).
        if saw_blank
            && !trimmed.contains('=')
            && !trimmed.starts_with('[')
            && !trimmed.starts_with('"')
            && !trimmed.starts_with('\'')
            && !trimmed.starts_with('#')
        // TOML comment
        {
            break;
        }

        saw_blank = false;
        last_toml_end += line.len() + 1;
    }

    let result = candidate[..last_toml_end].trim();
    if result.is_empty() || !result.contains("[[task]]") {
        None
    } else {
        Some(result)
    }
}

/// Required field names for the `[meta]` section.
const REQUIRED_META_FIELDS: &[&str] = &["plan", "total", "status"];

/// Required field names for each `[[task]]`.
const REQUIRED_TASK_FIELDS: &[&str] = &["id", "title", "status", "role", "tier"];

/// Required field names for each `[[task.verify]]` entry.
const REQUIRED_VERIFY_FIELDS: &[&str] = &["phase", "command"];

/// Validate and fix a generated plan TOML string.
///
/// Checks:
/// 1. TOML syntax.
/// 2. Required fields in `[meta]` and `[[task]]`.
/// 3. Unknown / misspelled fields (with suggested corrections applied).
/// 4. `model_hint` values stripped, and `rung` hints kept only when they name
///    one of the task's `ladder` rungs.
/// 5. `meta.plan` matched against the expected slug.
///
/// On fixable issues the TOML is patched and a warning is logged to stderr.
/// On unfixable issues an error is returned.
fn validate_and_fix_generated_plan(
    toml_str: &str,
    slug: &str,
    _models: &IndexMap<String, roko_core::config::schema::ModelProfile>,
    _default_model: Option<&str>,
    ladder: &LadderConfig,
) -> Result<String> {
    // 0. Deterministic repair before parsing.
    let repaired = crate::task_parser::repair_toml(toml_str);
    // Also fix the common LLM mistake of using `name = ` instead of `plan = `
    // in the [meta] section. repair_toml leaves it alone because it parses
    // fine as TOML; we catch it here at the semantic level.
    let repaired = crate::task_parser::fix_meta_name_to_plan(&repaired);
    if repaired != toml_str {
        tracing::info!("plan generate: applied deterministic TOML repair");
    }

    // 1. Parse syntax.
    let mut root: toml::Value =
        toml::from_str(&repaired).map_err(|e| anyhow!("generated plan has invalid TOML: {e}"))?;

    let root_table = root
        .as_table_mut()
        .ok_or_else(|| anyhow!("generated plan TOML root is not a table"))?;

    let mut errors: Vec<String> = Vec::new();

    // -- [meta] validation ---------------------------------------------------
    if let Some(meta_val) = root_table.get_mut("meta") {
        if let Some(meta) = meta_val.as_table_mut() {
            // Flag unknown meta fields.
            let meta_keys: Vec<String> = meta.keys().cloned().collect();
            for key in &meta_keys {
                if !META_KEYS.contains(&key.as_str()) {
                    if let Some(correction) = suggest_field_correction(key, META_KEYS) {
                        if let Some(value) = meta.remove(key.as_str()) {
                            tracing::warn!(
                                "plan generate: [meta] field '{key}' is unknown; \
                                 corrected to '{correction}'"
                            );
                            meta.insert(correction, value);
                        }
                    } else {
                        tracing::warn!("plan generate: [meta] has unknown field '{key}'");
                    }
                }
            }
            // Check required meta fields.
            for &required in REQUIRED_META_FIELDS {
                match meta.get(required) {
                    None => errors.push(format!("[meta] is missing required field '{required}'")),
                    Some(v) if v.as_str().is_some_and(|s| s.trim().is_empty()) => {
                        errors.push(format!("[meta].{required} is empty"));
                    }
                    _ => {}
                }
            }
            // Fix meta.plan if truncated or wrong.
            if let Some(plan_val) = meta.get("plan") {
                if let Some(plan_str) = plan_val.as_str() {
                    if plan_str != slug {
                        if slug.starts_with(plan_str) {
                            tracing::warn!(
                                "plan generate: meta.plan '{plan_str}' appears truncated; \
                                 corrected to '{slug}'"
                            );
                        } else {
                            tracing::warn!(
                                "plan generate: meta.plan '{plan_str}' does not match \
                                 expected slug '{slug}'; corrected"
                            );
                        }
                        meta.insert("plan".to_string(), toml::Value::String(slug.to_string()));
                    }
                }
            }
        }
    } else {
        errors.push("[meta] section is missing".to_string());
    }

    // -- [[task]] validation --------------------------------------------------
    if let Some(tasks_val) = root_table.get_mut("task") {
        if let Some(tasks) = tasks_val.as_array_mut() {
            if tasks.is_empty() {
                errors.push("[[task]] array is present but empty".to_string());
            }
            for (i, task_val) in tasks.iter_mut().enumerate() {
                if let Some(task) = task_val.as_table_mut() {
                    let task_id_label: String = task
                        .get("id")
                        .and_then(toml::Value::as_str)
                        .map(String::from)
                        .unwrap_or_else(|| format!("task #{}", i + 1));

                    // Flag unknown task fields.
                    let task_keys: Vec<String> = task.keys().cloned().collect();
                    for key in &task_keys {
                        if !TASK_KEYS.contains(&key.as_str()) {
                            if let Some(correction) = suggest_field_correction(key, TASK_KEYS) {
                                if let Some(value) = task.remove(key.as_str()) {
                                    tracing::warn!(
                                        "plan generate: {task_id_label}: field '{key}' is unknown; \
                                         corrected to '{correction}'"
                                    );
                                    task.insert(correction, value);
                                }
                            } else {
                                tracing::warn!(
                                    "plan generate: {task_id_label}: unknown field '{key}'"
                                );
                            }
                        }
                    }

                    // Check required task fields.
                    for &required in REQUIRED_TASK_FIELDS {
                        match task.get(required) {
                            None => errors.push(format!(
                                "{task_id_label} is missing required field '{required}'"
                            )),
                            Some(v) if v.as_str().is_some_and(|s| s.trim().is_empty()) => {
                                errors
                                    .push(format!("{task_id_label}: field '{required}' is empty"));
                            }
                            _ => {}
                        }
                    }

                    // Validate status value.
                    if let Some(status_val) = task.get("status").cloned() {
                        if let Some(s) = status_val.as_str() {
                            const VALID_STATUSES: &[&str] = &[
                                "ready",
                                "pending",
                                "blocked",
                                "in_progress",
                                "done",
                                "skipped",
                            ];
                            if !VALID_STATUSES.contains(&s) {
                                tracing::warn!(
                                    "plan generate: {task_id_label}: status '{s}' is invalid; \
                                     defaulting to 'ready'"
                                );
                                task.insert(
                                    "status".to_string(),
                                    toml::Value::String("ready".to_string()),
                                );
                            }
                        }
                    }

                    // Validate role value.
                    if let Some(role_val) = task.get("role").cloned() {
                        if let Some(r) = role_val.as_str() {
                            const VALID_ROLES: &[&str] = crate::task_parser::PLAN_TASK_ROLES;
                            if !VALID_ROLES.contains(&r) {
                                tracing::warn!(
                                    "plan generate: {task_id_label}: role '{r}' is invalid; \
                                     defaulting to 'implementer'"
                                );
                                task.insert(
                                    "role".to_string(),
                                    toml::Value::String("implementer".to_string()),
                                );
                            }
                        }
                    }

                    // Always strip model_hint from generated plans: a model
                    // name ties the plan to one provider, while the tier and
                    // role pick a model on this workspace's routing ladder.
                    if let Some(hint_val) = task.remove("model_hint") {
                        let hint = hint_val.as_str().unwrap_or("<unknown>");
                        tracing::info!(
                            "plan generate: {task_id_label}: removing model_hint '{hint}' \
                             (tier and role pick the model; a task that needs a \
                             stronger one names a `rung`)"
                        );
                    }

                    // gap-dbf2a6: keep a `rung` hint that names one of the
                    // task's ladder rungs; drop any other.
                    if let Some(rung) = crate::plan_validate::drop_unknown_rung(task, ladder) {
                        tracing::warn!(
                            "plan generate: {task_id_label}: removing rung {rung}: no rung of the \
                             routing ladder has that name"
                        );
                    }

                    // Validate [[task.verify]] sub-entries.
                    if let Some(verify_val) = task.get_mut("verify") {
                        if let Some(steps) = verify_val.as_array_mut() {
                            for (si, step_val) in steps.iter_mut().enumerate() {
                                if let Some(step) = step_val.as_table_mut() {
                                    let step_keys: Vec<String> = step.keys().cloned().collect();
                                    for key in &step_keys {
                                        if !VERIFY_KEYS.contains(&key.as_str()) {
                                            if let Some(correction) =
                                                suggest_field_correction(key, VERIFY_KEYS)
                                            {
                                                if let Some(value) = step.remove(key.as_str()) {
                                                    tracing::warn!(
                                                        "plan generate: {task_id_label} verify[{si}]: \
                                                         field '{key}' corrected to '{correction}'"
                                                    );
                                                    step.insert(correction, value);
                                                }
                                            } else {
                                                tracing::warn!(
                                                    "plan generate: {task_id_label} verify[{si}]: \
                                                     unknown field '{key}'"
                                                );
                                            }
                                        }
                                    }

                                    // Check required verify fields.
                                    for &required in REQUIRED_VERIFY_FIELDS {
                                        if step
                                            .get(required)
                                            .and_then(toml::Value::as_str)
                                            .is_none_or(|s| s.trim().is_empty())
                                        {
                                            errors.push(format!(
                                                "{task_id_label} verify[{si}]: \
                                                 missing required field '{required}'"
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Auto-add verify entries for implementer tasks with files
                    // but no verify block. Researchers/strategists/scribes skip.
                    if task.get("verify").is_none() {
                        let role = task
                            .get("role")
                            .and_then(toml::Value::as_str)
                            .unwrap_or("implementer");
                        let files: Vec<String> = task
                            .get("files")
                            .and_then(toml::Value::as_array)
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(toml::Value::as_str)
                                    .map(String::from)
                                    .collect()
                            })
                            .unwrap_or_default();

                        // bug-8723fc: only a task that writes Rust gets a
                        // cargo check here; repair_build_checks gives the
                        // others their workspace's own build.
                        let rust = files
                            .iter()
                            .any(|file| file.ends_with(".rs") || file.ends_with("Cargo.toml"));
                        if role == "implementer" && rust {
                            // Infer crate name from file paths for compile check
                            let crate_name = infer_crate_from_paths(&files);
                            let compile_cmd = match &crate_name {
                                Some(c) => format!("cargo check -p {c}"),
                                None => "cargo check --workspace".to_string(),
                            };
                            let auto_verify = vec![make_verify_entry(
                                "compile",
                                &compile_cmd,
                                &format!(
                                    "{} must compile",
                                    crate_name.as_deref().unwrap_or("workspace"),
                                ),
                            )];

                            task.insert("verify".to_string(), toml::Value::Array(auto_verify));
                            tracing::info!(
                                "plan generate: {task_id_label}: auto-added one focused compile verify"
                            );
                        }
                    }
                }
            }
        }
    } else {
        errors.push("[[task]] array is missing".to_string());
    }

    if !errors.is_empty() {
        let joined = errors.join("\n  - ");
        return Err(anyhow!(
            "generated plan TOML has {n} validation error(s):\n  - {joined}",
            n = errors.len()
        ));
    }

    // Serialize the (possibly patched) TOML back to a string.
    let mut serialized = toml::to_string_pretty(&root)
        .map_err(|e| anyhow!("failed to re-serialize fixed plan TOML: {e}"))?;

    // -- Angle-bracket placeholder replacement --------------------------------
    // LLMs sometimes emit literal `<relevant-lib>`, `<crate>`, `<path>` etc.
    // instead of concrete values. Replace known placeholders with slug-derived
    // defaults so downstream tooling doesn't choke on them.
    let path_default = format!("crates/{slug}/src/lib.rs");
    let replacements: &[(&str, &str)] = &[
        ("<relevant-lib>", slug),
        ("<binary-crate>", slug),
        ("<crate>", slug),
        ("<module>", "lib"),
        ("<path>", &path_default),
        ("<file>", &path_default),
        ("<test_name>", "test_placeholder"),
    ];
    for &(placeholder, replacement) in replacements {
        if serialized.contains(placeholder) {
            tracing::info!(
                "plan generate: replaced placeholder '{}' with '{}'",
                placeholder,
                replacement
            );
            serialized = serialized.replace(placeholder, replacement);
        }
    }

    // If we did any replacements, verify the TOML still parses.
    if replacements.iter().any(|(ph, _)| toml_str.contains(*ph)) {
        let _: toml::Value = toml::from_str(&serialized)
            .map_err(|e| anyhow!("TOML became invalid after placeholder replacement: {e}"))?;
    }

    Ok(serialized)
}

/// Infer the crate name from a list of file paths.
/// Looks for `crates/<name>/` pattern. Returns `None` if no crate path found.
fn infer_crate_from_paths(files: &[String]) -> Option<String> {
    for f in files {
        // Match patterns like "crates/roko-cli/src/foo.rs"
        if let Some(rest) = f.strip_prefix("crates/") {
            if let Some(slash) = rest.find('/') {
                return Some(rest[..slash].to_string());
            }
        }
    }
    None
}

/// Build a `[[task.verify]]` TOML table value.
fn make_verify_entry(phase: &str, command: &str, fail_msg: &str) -> toml::Value {
    let mut table = toml::value::Table::new();
    table.insert("phase".to_string(), toml::Value::String(phase.to_string()));
    table.insert(
        "command".to_string(),
        toml::Value::String(command.to_string()),
    );
    table.insert(
        "fail_msg".to_string(),
        toml::Value::String(fail_msg.to_string()),
    );
    toml::Value::Table(table)
}

/// Slugify a title.
///
/// Output is capped at 200 bytes, truncated at the last hyphen before the
/// limit (word boundary) to avoid filesystem path-length errors.
pub fn slugify(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    const MAX_LEN: usize = 200;
    if slug.len() <= MAX_LEN {
        return slug;
    }

    // Truncate at the last hyphen before the limit (word boundary).
    if let Some(pos) = slug[..MAX_LEN].rfind('-') {
        slug[..pos].to_string()
    } else {
        slug[..MAX_LEN].to_string()
    }
}

/// Clamp each `read_files` line range in `tasks` that starts inside its file
/// but ends past the file's last line (bug-1c75cc), and return how many it
/// clamped. Paths resolve against `workspace_root`, as `validate_plan_context`
/// resolves them; a range it cannot read or parse, or one that starts past the
/// end, is left for that check to refuse. Only generation repairs ranges:
/// hand-written plans stay strict.
fn clamp_context_ranges_past_eof(
    tasks: &mut crate::task_parser::TasksFile,
    workspace_root: &std::path::Path,
) -> usize {
    let mut clamped = 0;
    for task in &mut tasks.tasks {
        let Some(context) = task.context.as_mut() else {
            continue;
        };
        for read_file in &mut context.read_files {
            let Some(range) = read_file.lines.as_deref() else {
                continue;
            };
            let Ok((start, Some(end))) = crate::plan_policy::parse_line_range(range, usize::MAX)
            else {
                continue;
            };
            let Ok(content) = std::fs::read_to_string(workspace_root.join(&read_file.path)) else {
                continue;
            };
            let line_count = content.lines().count();
            if start >= 1 && start <= line_count && end > line_count {
                read_file.lines = Some(format!("{start}-{line_count}"));
                clamped += 1;
            }
        }
    }
    clamped
}

#[cfg(test)]
mod tests {
    use super::*;

    /// bug-59ed90: a failed planner call's error carries what it printed,
    /// which is where a provider says its key is wrong or it is unreachable.
    #[test]
    fn a_failed_planner_call_says_what_the_planner_printed() {
        assert_eq!(agent_said("  \n"), "");
        assert_eq!(
            agent_said("error: 401 Unauthorized: invalid api key\n"),
            "\nThe planner said: error: 401 Unauthorized: invalid api key"
        );
        let long = "y".repeat(AGENT_ERROR_PREVIEW_CHARS + 50);
        assert_eq!(
            agent_said(&long),
            format!(
                "\nThe planner said: {}",
                "y".repeat(AGENT_ERROR_PREVIEW_CHARS)
            )
        );
    }

    /// bug-d3c72e: a failed plan generation prints the head of the model's
    /// output. Cutting it at byte 2000 panicked when that byte fell inside a
    /// multi-byte character; the cut now falls on a char boundary.
    #[test]
    fn failure_output_cuts_at_a_char_boundary() {
        let output = format!("{}é and more", "x".repeat(FAILURE_OUTPUT_CHARS - 1));
        assert!(!output.is_char_boundary(FAILURE_OUTPUT_CHARS));

        let printed = crate::run::truncate(&output, FAILURE_OUTPUT_CHARS);

        assert_eq!(printed.chars().count(), FAILURE_OUTPUT_CHARS);
        assert!(printed.ends_with('é'));
        let preview = crate::run::truncate(&output, AGENT_ERROR_PREVIEW_CHARS);
        assert_eq!(preview, "x".repeat(AGENT_ERROR_PREVIEW_CHARS));
        assert_eq!(crate::run::truncate("short", FAILURE_OUTPUT_CHARS), "short");
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Agent Self-Improvement"), "agent-self-improvement");
        assert_eq!(slugify("  foo  BAR  "), "foo-bar");
        assert_eq!(slugify("hello"), "hello");
    }

    #[test]
    fn slugify_long_input_capped_at_200() {
        // Build a title that would produce a slug longer than 200 bytes.
        let long_title = "word ".repeat(100); // 500 chars → ~400+ byte slug
        let slug = slugify(&long_title);
        assert!(slug.len() <= 200, "slug len {} > 200", slug.len());
        // Should end at a word boundary (no trailing hyphen).
        assert!(!slug.ends_with('-'));
        assert!(!slug.is_empty());
    }

    #[test]
    fn slugify_exactly_200_not_truncated() {
        // Build a slug that's exactly 200 bytes — should not be truncated.
        let word = "abcdefghij"; // 10 chars
        // 19 words of 10 + 18 hyphens = 208; 18 words = 198; need filler
        let title = format!("{} ab", vec![word; 19].join(" ")); // yields 19*10+18+3 = 211 → truncated
        let slug = slugify(&title);
        assert!(slug.len() <= 200, "slug len {} > 200", slug.len());
    }

    #[test]
    fn slugify_short_input_unchanged() {
        let slug = slugify("small title");
        assert_eq!(slug, "small-title");
        assert!(slug.len() < 200);
    }

    #[test]
    fn generation_outcome_labels_distinguish_process_from_artifact() {
        let success = GenerationOutcome {
            process_success: true,
            artifact_valid: true,
            validation_report: None,
            spec_quality: None,
            source_checks: None,
            cost_usd: 0.0,
            repairs: Vec::new(),
        };
        let partial = GenerationOutcome {
            process_success: true,
            artifact_valid: false,
            validation_report: None,
            spec_quality: None,
            source_checks: None,
            cost_usd: 0.0,
            repairs: Vec::new(),
        };
        let failure = GenerationOutcome {
            process_success: false,
            artifact_valid: true,
            validation_report: None,
            spec_quality: None,
            source_checks: None,
            cost_usd: 0.0,
            repairs: Vec::new(),
        };

        assert!(success.fully_successful());
        assert_eq!(success.status_label(), "success");
        assert!(!partial.fully_successful());
        assert_eq!(partial.status_label(), "partial_success");
        assert!(!failure.fully_successful());
        assert_eq!(failure.status_label(), "failure");
    }

    #[test]
    fn extract_fenced_block_finds_toml() {
        let text = "Some text\n```toml\n[tasks]\nname = \"test\"\n```\nMore text";
        let block = extract_fenced_block(text, "toml").unwrap();
        assert!(block.contains("[tasks]"));
        assert!(block.contains("name = \"test\""));
    }

    #[test]
    fn extract_fenced_block_returns_none_for_missing() {
        assert!(extract_fenced_block("no blocks here", "toml").is_none());
    }

    #[test]
    fn extract_fenced_block_skips_nested_fences() {
        // Agent output might include code samples with their own fences
        let text = "Here is the plan:\n```toml\n[[tasks]]\nid = \"T1\"\n\
                    # Example bash:\n```bash\necho hello\n```\n\
                    verify = \"cargo test\"\n```\nDone.";
        let block = extract_fenced_block(text, "toml").unwrap();
        assert!(block.contains("id = \"T1\""), "should contain task");
        assert!(block.contains("```bash"), "should include the nested fence");
    }

    #[test]
    fn extract_fenced_block_handles_angle_bracket_tag() {
        let text = "Output:\n```<plan.md>\n# My Plan\n\nSteps here.\n```\n";
        let block = extract_fenced_block(text, "plan.md").unwrap();
        assert!(block.contains("# My Plan"));
    }

    #[test]
    fn extract_fenced_block_returns_none_for_empty_block() {
        let text = "```toml\n\n```\n";
        assert!(extract_fenced_block(text, "toml").is_none());
    }

    #[test]
    fn extract_fenced_block_multiple_blocks_gets_first() {
        let text = "```toml\nfirst = true\n```\n\n```toml\nsecond = true\n```\n";
        let block = extract_fenced_block(text, "toml").unwrap();
        assert!(block.contains("first = true"));
        assert!(!block.contains("second = true"));
    }

    #[test]
    fn fallback_extracts_clean_toml() {
        let text = "[meta]\nplan = \"test\"\ntotal = 2\nstatus = \"ready\"\n\n\
                    [[task]]\nid = \"T1\"\ntitle = \"First\"\n";
        let block = extract_toml_content_fallback(text).unwrap();
        assert!(block.contains("[meta]"));
        assert!(block.contains("[[task]]"));
    }

    #[test]
    fn fallback_trims_trailing_markdown() {
        let text = "Here is your plan:\n\n\
                    [meta]\nplan = \"test\"\ntotal = 1\nstatus = \"ready\"\n\n\
                    [[task]]\nid = \"T1\"\ntitle = \"Do stuff\"\n\n\
                    ## Notes\n\nThis plan covers the main requirements.\n";
        let block = extract_toml_content_fallback(text).unwrap();
        assert!(block.contains("[meta]"));
        assert!(block.contains("[[task]]"));
        assert!(
            !block.contains("## Notes"),
            "should trim trailing markdown heading"
        );
        assert!(
            !block.contains("This plan covers"),
            "should trim trailing prose"
        );
    }

    #[test]
    fn fallback_trims_trailing_horizontal_rule() {
        let text = "[meta]\nplan = \"x\"\ntotal = 1\nstatus = \"ready\"\n\n\
                    [[task]]\nid = \"T1\"\ntitle = \"A\"\n\n\
                    ---\n\nSome explanation here.\n";
        let block = extract_toml_content_fallback(text).unwrap();
        assert!(!block.contains("---"), "should trim trailing hr");
        assert!(!block.contains("explanation"));
    }

    #[test]
    fn fallback_trims_trailing_prose_after_blank() {
        let text = "[meta]\nplan = \"x\"\ntotal = 1\nstatus = \"ready\"\n\n\
                    [[task]]\nid = \"T1\"\ntitle = \"A\"\n\n\
                    This plan implements the feature as described in the spec.\n";
        let block = extract_toml_content_fallback(text).unwrap();
        assert!(
            !block.contains("This plan implements"),
            "should trim trailing prose paragraph"
        );
    }

    #[test]
    fn fallback_preserves_toml_comments() {
        let text = "[meta]\nplan = \"x\"\ntotal = 1\nstatus = \"ready\"\n\n\
                    # Task section\n[[task]]\nid = \"T1\"\ntitle = \"A\"\n";
        let block = extract_toml_content_fallback(text).unwrap();
        assert!(
            block.contains("# Task section"),
            "TOML comments should be kept"
        );
    }

    #[test]
    fn fallback_returns_none_without_task() {
        let text = "[meta]\nplan = \"x\"\n";
        assert!(extract_toml_content_fallback(text).is_none());
    }

    /// backlog 4126: the planner prompt lists the recurring failure patterns
    /// with a verified fix for the crates its source touches. With fixed
    /// patterns for `crates/a` and `crates/b`, a source touching `crates/a`
    /// gets only the first; a pattern of `crates/a` seen once, or without a
    /// fix, stays out, and a source that names no crate gets no block.
    #[test]
    fn planner_prompt_lists_resolved_patterns_for_touched_crates() {
        use roko_learn::error_pattern_store::{
            ErrorPatternStore, GateFailureObservation, GateFailureSource,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let mut store = ErrorPatternStore::empty();
        for (key, command, times, fixed) in [
            ("verify::a", "cargo test -p a", 2, true),
            ("verify::b", "cargo test -p b", 3, true),
            ("verify::a-once", "cargo clippy -p a", 1, true),
            ("verify::a-unfixed", "cargo build -p a", 2, false),
        ] {
            for _ in 0..times {
                store.observe_gate_failure(GateFailureObservation::new(
                    key,
                    "plan-x",
                    Some("T1".to_string()),
                    command,
                    "verify",
                    format!("verify[0:test] `{command}` failed: exit code: 101"),
                    GateFailureSource::GateClassification,
                ));
            }
            if fixed {
                store.record_resolution(key, &format!("Fixed `{command}`"), "gr-x:plan-x:T1:2");
            }
        }
        store
            .save(&temp.path().join(".roko/learn/error-patterns.json"))
            .expect("save the patterns");

        let source = "Make crates/a/src/lib.rs return a greeting.";
        let (block, ids) = known_failure_patterns(temp.path(), source).expect("patterns");
        assert_eq!(ids, ["verify::a"]);
        assert!(block.contains("Fix: Fixed `cargo test -p a`"), "{block}");
        assert!(!block.contains("-p b"), "{block}");
        assert!(known_failure_patterns(temp.path(), "Write a README.").is_none());
    }

    #[test]
    fn augment_generator_system_prompt_skips_empty_context() {
        let prompt = augment_generator_system_prompt("base prompt".to_string(), Some("   "));
        assert_eq!(prompt, "base prompt");
    }

    /// gap-2623b2: a planner with a large context window reads a
    /// 20,000-character request whole, with a file-read budget to match; a
    /// planner whose window is unknown keeps the old 8,000-character and
    /// 5-file caps, which are also every planner's floor.
    #[test]
    fn generator_prompt_keeps_a_long_request_whole() {
        let mut request = String::from("# Parser\n\n");
        while request.len() < 20_000 {
            request.push_str("The parser keeps every line of its input.\n");
        }
        request.push_str("END-OF-REQUEST\n");
        let source = ReadSource::read(PlanSource::Text {
            text: &request,
            kind: "spec",
        })
        .expect("read the request");

        let mut models = IndexMap::new();
        models.insert(
            "frontier".to_string(),
            ModelProfile {
                slug: "frontier-1".to_string(),
                context_window: 200_000,
                ..ModelProfile::default()
            },
        );
        let prompt = |model: &str| {
            let budget = PlannerBudget::for_context_window(planner_context_window(&models, model));
            plan_task_prompt(&source, "parser", budget, "", "")
        };

        let frontier = prompt("frontier");
        assert!(
            frontier.contains(&request),
            "the whole request reaches the planner"
        );
        assert!(!frontier.contains("content truncated"));
        assert!(frontier.contains("read up to 25 codebase files"));
        assert_eq!(prompt("frontier-1"), frontier, "a slug finds its entry");

        let unknown = prompt("some-other-model");
        assert!(unknown.contains("[spec content truncated at 8000 chars]"));
        assert!(!unknown.contains("END-OF-REQUEST"));
        assert!(unknown.contains("read up to 5 codebase files"));
        assert_eq!(
            PlannerBudget::for_context_window(Some(8_000)),
            PlannerBudget::SMALL
        );
    }

    /// The request the `widget` plan is generated from in the tests below.
    #[cfg(unix)]
    const WIDGET_REQUEST: PlanSource<'static> = PlanSource::Text {
        text: "# Widget\n\nAdd a widget module.\n",
        kind: "prompt",
    };

    /// The plan the fake planner answers with in the tests below.
    #[cfg(unix)]
    const WIDGET_PLAN: &str = "\
        [meta]\nplan = \"widget\"\ntotal = 1\ndone = 0\nstatus = \"ready\"\n\n\
        [[task]]\nid = \"T1\"\ntitle = \"Add the widget module\"\n\
        description = \"Create src/widget.rs.\"\nstatus = \"ready\"\n\
        role = \"implementer\"\ntier = \"focused\"\nfiles = [\"src/widget.rs\"]\n\
        depends_on = []\n\n[task.context]\nread_files = []\n\n\
        [[task.verify]]\nphase = \"check\"\ncommand = \"test -f src/widget.rs\"\n";

    /// Make `workdir`'s `planner` model a fake `claude_cli` script in `bin`
    /// that answers every call with `plan_toml` and logs it, and return the
    /// call log. The plan is never regenerated for its spec score.
    #[cfg(unix)]
    fn write_fake_planner(workdir: &Path, bin: &Path, plan_toml: &str) -> PathBuf {
        write_scripted_planner(
            workdir,
            bin,
            &[plan_toml],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        )
    }

    /// Make `workdir`'s `planner` model a fake `claude_cli` script in `bin`
    /// that answers its n-th call with `plans[n]` (the last plan after that;
    /// a plan that starts with a fence is the whole reply),
    /// keeps each call's arguments and input as `prompt-<n>.txt`, and logs
    /// it; return the call log. `spec_quality` is the body of the
    /// workspace's `[spec_quality]` section.
    #[cfg(unix)]
    fn write_scripted_planner(
        workdir: &Path,
        bin: &Path,
        plans: &[&str],
        spec_quality: &str,
    ) -> PathBuf {
        use std::os::unix::fs::PermissionsExt as _;

        for (index, plan_toml) in plans.iter().enumerate() {
            let text = if plan_toml.starts_with("```") {
                (*plan_toml).to_string()
            } else {
                format!("```toml\n{plan_toml}```\n")
            };
            std::fs::write(
                bin.join(format!("reply-{index}.jsonl")),
                format!(
                    "{}\n{}\n",
                    serde_json::json!({
                        "type": "content_block_delta",
                        "delta": {"text": text},
                    }),
                    serde_json::json!({
                        "type": "result",
                        "session_id": "planner",
                        "model": "claude-sonnet-4-6",
                        "total_cost_usd": 0.0,
                        "usage": {"input_tokens": 1, "output_tokens": 1},
                        "is_error": false,
                    }),
                ),
            )
            .expect("planner reply");
        }
        let last = bin.join(format!("reply-{}.jsonl", plans.len().saturating_sub(1)));
        // Work after a call (episode distillation) runs on the default model,
        // a second fake that answers nothing, so the log counts planner calls.
        // A validation retry climbs `[agent.tier_models]` from the planner
        // (to claude-opus-4-6 by default, which no fake serves); with the
        // planner as the only tier, every retry reaches the scripted planner.
        let calls = bin.join("calls.log");
        let planner = bin.join("planner.sh");
        let background = bin.join("background.sh");
        let planner_body = format!(
            "n=$(cat '{calls}' 2>/dev/null | wc -l | tr -d ' ')\n\
             printf '%s\\n' \"$@\" > '{bin}/prompt-'\"$n\"'.txt'\n\
             cat >> '{bin}/prompt-'\"$n\"'.txt'\n\
             echo call >> '{calls}'\n\
             reply='{bin}/reply-'\"$n\"'.jsonl'\n\
             [ -f \"$reply\" ] || reply='{last}'\n\
             cat \"$reply\"",
            calls = calls.display(),
            bin = bin.display(),
            last = last.display()
        );
        for (script, body) in [
            (&planner, planner_body),
            (
                &background,
                format!("cat >/dev/null\nsed -n 2p '{}'", last.display()),
            ),
        ] {
            std::fs::write(script, format!("#!/bin/sh\nset -eu\n{body}\n"))
                .expect("fake provider script");
            std::fs::set_permissions(script, std::fs::Permissions::from_mode(0o755))
                .expect("make the script executable");
        }
        std::fs::write(
            workdir.join("roko.toml"),
            format!(
                "[agent]\ndefault_model = \"background\"\ncommand = {background:?}\n\
                 bare_mode = false\n\n\
                 [agent.tier_models]\nsonnet = \"planner\"\n\n\
                 [providers.fake]\nkind = \"claude_cli\"\ncommand = {planner:?}\n\n\
                 [providers.quiet]\nkind = \"claude_cli\"\ncommand = {background:?}\n\n\
                 [models.planner]\nprovider = \"fake\"\nslug = \"claude-sonnet-4-6\"\n\
                 context_window = 200000\n\n\
                 [models.background]\nprovider = \"quiet\"\nslug = \"claude-sonnet-4-6\"\n\n\
                 [spec_quality]\n{spec_quality}",
                planner = planner.display().to_string(),
                background = background.display().to_string()
            ),
        )
        .expect("roko.toml");
        calls
    }

    /// 3218: generation scores its plan. A stub planner whose first plan has
    /// a verify step that can never fail and whose second is clean writes
    /// the clean plan after exactly two calls, and the retry names the hard
    /// fail. A stub whose plans stay weak is asked once more and no further,
    /// and the outcome records the scores.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generation_regenerates_once_on_a_spec_hard_fail() {
        let vacuous = WIDGET_PLAN.replace(
            "\"test -f src/widget.rs\"",
            "\"test -f src/widget.rs || true\"",
        );
        assert_ne!(vacuous, WIDGET_PLAN);

        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&vacuous, WIDGET_PLAN],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        let (_, outcome) = generate_plan(request)
            .await
            .expect("generate the widget plan");
        let written = std::fs::read_to_string(workdir.join("plans/widget/tasks.toml"))
            .expect("the widget plan");
        assert!(!written.contains("|| true"), "{written}");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(
            log.lines().count(),
            2,
            "the vacuous plan, then the clean one"
        );
        let retry = std::fs::read_to_string(bin.path().join("prompt-1.txt")).expect("retry");
        assert!(
            retry.contains("task T1 HF2: step 1: ends in `|| true`"),
            "{retry}"
        );
        let scored = outcome.spec_quality.expect("the plan was scored");
        assert!(!scored.regenerated, "{scored:?}");
        assert_eq!(scored.bands.values().sum::<usize>(), 1, "{scored:?}");

        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[WIDGET_PLAN],
            "allow_threshold = 100.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        let (_, outcome) = generate_plan(request)
            .await
            .expect("generate the widget plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 2, "one plan, then one regeneration");
        let regenerate = std::fs::read_to_string(bin.path().join("prompt-1.txt")).expect("ask");
        assert!(
            regenerate.contains("scored low on spec quality"),
            "{regenerate}"
        );
        let scored = outcome.spec_quality.expect("the plan was scored");
        assert!(scored.regenerated, "{scored:?}");
        assert!(
            scored.min <= scored.mean && scored.mean < 100.0,
            "{scored:?}"
        );
    }

    /// gap-18bfa2: once a generation's planner calls have cost its cap, it
    /// makes no further call. A retry it needs stops it, with an error that
    /// names the cap; an improvement it would ask for is skipped and the plan
    /// in hand is written. Each stage reports the cap.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn plan_generation_stops_at_its_spend_cap() {
        // Any call costs more than this cap.
        const CAP: f64 = 1e-9;

        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &["```text\nNo plan yet.\n```\n", WIDGET_PLAN],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let (sink, stages) = recorded_progress();
        let request = PlanRequest {
            model: Some("planner"),
            progress: Some(sink),
            max_usd: Some(CAP),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        let error = generate_plan(request)
            .await
            .expect_err("the retry is over the cap");
        let message = format!("{error:#}");
        assert!(message.contains("reached its spend cap"), "{message}");
        assert!(message.contains("no plan was written"), "{message}");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "the first call, and no retry");
        assert!(!workdir.join("plans/widget").exists());
        let stages = stages.lock().expect("stages");
        assert!(
            stages.iter().all(|stage| stage.max_usd == Some(CAP)),
            "{stages:?}"
        );

        // A weak plan is written as it is, without the regeneration.
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[WIDGET_PLAN],
            "allow_threshold = 100.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            max_usd: Some(CAP),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        let (_, outcome) = generate_plan(request)
            .await
            .expect("the plan in hand is written");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "no regeneration over the cap");
        assert!(outcome.cost_usd > CAP, "{outcome:?}");
        assert!(workdir.join("plans/widget/tasks.toml").is_file());
    }

    /// gap-b95913: a draft whose `meta.total` is 0, whose task reads four
    /// files, and whose range starts past the end of its file is repaired,
    /// not sent back: the plan is written after one planner call.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generated_draft_repairs_total_ranges_and_read_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        std::fs::create_dir_all(workdir.join("docs")).expect("docs");
        for name in ["a", "b", "c", "d"] {
            std::fs::write(workdir.join(format!("docs/{name}.md")), "line\n".repeat(71))
                .expect("a doc");
        }
        let draft = WIDGET_PLAN.replace("total = 1", "total = 0").replace(
            "read_files = []",
            "read_files = [\n\
             { path = \"docs/a.md\", lines = \"200-400\", why = \"the spec\" },\n\
             { path = \"docs/b.md\", why = \"the second part\" },\n\
             { path = \"docs/c.md\", why = \"the third part\" },\n\
             { path = \"docs/d.md\", why = \"the last part\" },\n]",
        );
        assert!(draft.contains("total = 0") && draft.contains("docs/d.md"));
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&draft],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        generate_plan(request)
            .await
            .expect("generate the widget plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "the draft, repaired; no retry");

        let written =
            TasksFile::parse(&workdir.join("plans/widget/tasks.toml")).expect("the widget plan");
        assert_eq!(written.meta.total, 1);
        let read_files = &written.tasks[0]
            .context
            .as_ref()
            .expect("the task's context")
            .read_files;
        let paths: Vec<&str> = read_files.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(paths, ["docs/a.md", "docs/b.md", "docs/c.md"]);
        assert_eq!(read_files[0].lines, None, "the range past the end is gone");
    }

    /// gap-ecafde: a check that cannot pass as written is repaired, or sent
    /// back. A test file a task's own check needs joins the task's files; a
    /// check that names a test its file does not define goes back to the
    /// planner once, named, and the plan whose checks can all pass is
    /// written.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generation_flags_verify_steps_that_cannot_pass() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        std::fs::create_dir_all(workdir.join("tests")).expect("tests");
        std::fs::write(
            workdir.join("tests/test_widget.py"),
            "import unittest\n\nclass WidgetTest(unittest.TestCase):\n    \
             def test_spins(self):\n        pass\n",
        )
        .expect("a test module");
        let plan = |test: &str| {
            WIDGET_PLAN.replace(
                "\"test -f src/widget.rs\"",
                &format!(
                    "\"grep -q 'def test_stops' tests/test_stops.py && python3 -m unittest \
                     tests.test_stops\"\n\n[[task.verify]]\nphase = \"test\"\n\
                     command = \"python3 -m unittest tests.test_widget.WidgetTest.{test}\""
                ),
            )
        };
        let (wobbles, spins) = (plan("test_wobbles"), plan("test_spins"));
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&wobbles, &spins],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        generate_plan(request)
            .await
            .expect("generate the widget plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(
            log.lines().count(),
            2,
            "the plan, then the plan whose checks can pass"
        );
        let again = std::fs::read_to_string(bin.path().join("prompt-1.txt")).expect("the retry");
        assert!(
            again.contains(
                "- T1 step 2: it names `test_wobbles`, which `tests/test_widget.py` does not \
                 define and no task writes"
            ),
            "{again}"
        );
        let written =
            TasksFile::parse(&workdir.join("plans/widget/tasks.toml")).expect("the widget plan");
        let task = &written.tasks[0];
        assert!(
            task.files.iter().any(|file| file == "tests/test_stops.py"),
            "{:?}",
            task.files
        );
        assert_eq!(
            task.verify[1].command,
            "python3 -m unittest tests.test_widget.WidgetTest.test_spins"
        );
    }

    /// bug-27f00b: asked for a synth in one sentence, the planner wrote five
    /// strategist tasks whose 11 checks grep the hello-world demo's plan.md
    /// for phrases it does not have. No task of the plan writes that file, so
    /// the base settles each check: all 11 fail there, and no task can make
    /// one pass. Replayed through a fake planner, generation finds the 11,
    /// asks the planner once more naming each, and writes the plan that
    /// specifies first, whose checks read the file its own task writes.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_check_no_task_can_change_is_settled_on_the_base() {
        const SLUG: &str = "create-a-roko-plan-for-a-macos-vst3-synthesizer";
        let (temp, baseline) = super::super::runnable::tests::no_spec_baseline();
        let workdir = temp.path();
        let spec_first = format!(
            "[meta]\nplan = \"{SLUG}\"\ntotal = 1\ndone = 0\nstatus = \"ready\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"Write the synth's spec\"\n\
             goal = \"docs/spec.md specifies the synth, with numbered acceptance criteria.\"\n\
             description = \"Write docs/spec.md: what the synth is, the choices the request \
             leaves open (decided), its non-goals, and numbered acceptance criteria.\"\n\
             acceptance = [\"AC1: docs/spec.md has an `## Acceptance criteria` section.\", \
             \"AC2: docs/spec.md numbers its criteria `AC1`, `AC2` and so on.\"]\n\
             non_goals = [\"Do not write code.\"]\nopen_questions = []\nstatus = \"ready\"\n\
             tier = \"focused\"\nmax_loc = 50\nfiles = [\"docs/spec.md\"]\n\
             allowed_tools = [\"read_file\", \"write_file\"]\ndenied_tools = []\n\
             depends_on = []\nrole = \"scribe\"\n\n[task.context]\nread_files = []\n\n\
             [[task.verify]]\nphase = \"structural\"\n\
             command = \"grep -q '^## Acceptance criteria' docs/spec.md\"\ncovers = [\"AC1\"]\n\
             expect = \"fail_on_base\"\n\n[[task.verify]]\nphase = \"structural\"\n\
             command = \"grep -qE '^- AC[0-9]+:' docs/spec.md\"\ncovers = [\"AC2\"]\n\
             expect = \"fail_on_base\"\n"
        );
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&baseline, &spec_first],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(
                PlanSource::Text {
                    text: "Create a roko plan for a macOS VST3 synthesizer plugin aimed at \
                           hypnotic techno basslines, sub, and rumbles.",
                    kind: "prompt",
                },
                SLUG,
                workdir,
            )
        };
        generate_plan(request).await.expect("generate the plan");

        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(
            log.lines().count(),
            2,
            "the baseline plan, then the one whose checks can pass"
        );
        let first = std::fs::read_to_string(bin.path().join("prompt-0.txt")).expect("the ask");
        assert!(
            first.contains(
                "## Workspace\n\nIts top level holds: README.md, plans/, roko.toml. A file a task \
                 creates goes in that task's `files`"
            ),
            "{first}"
        );
        let again = std::fs::read_to_string(bin.path().join("prompt-1.txt")).expect("the retry");
        let settled: Vec<&str> = again
            .lines()
            .filter(|line| {
                line.ends_with(
                    "it fails on the base, and no task can make it pass: it reads only \
                     `plans/demos/demo-hello-world/plan.md`, which no task of this plan writes",
                ) || line.ends_with(
                    "it fails on the base, and no task can make it pass: it reads only \
                     `plans/demos/demo-hello-world/tasks.toml`, which no task of this plan writes",
                )
            })
            .collect();
        assert_eq!(settled.len(), 11, "{again}");
        for task in ["T1", "T2", "T3", "T4", "T5"] {
            assert!(
                settled
                    .iter()
                    .any(|line| line.starts_with(&format!("- {task} step 1: "))),
                "{task}: {settled:?}"
            );
        }
        let written = TasksFile::parse(&workdir.join(format!("plans/{SLUG}/tasks.toml")))
            .expect("the written plan");
        assert_eq!(written.tasks.len(), 1);
        assert_eq!(written.tasks[0].role.as_deref(), Some("scribe"));
        assert_eq!(written.tasks[0].files, ["docs/spec.md"]);
    }

    /// bug-daa54e: two parallel tasks each create a C++ source that
    /// `core/CMakeLists.txt` must list. Generation adds the file to both and
    /// writes the plan after one call: tasks that share only a
    /// build-registration file are no longer an overlap it sends back.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn parallel_tasks_share_the_cmake_list_they_add_to() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        std::fs::create_dir_all(workdir.join("core/src")).expect("core");
        std::fs::write(
            workdir.join("core/CMakeLists.txt"),
            "add_library(core STATIC src/params.cpp)\n",
        )
        .expect("the core list");
        std::fs::write(workdir.join("core/src/params.cpp"), "// params\n").expect("params");
        let task = |id: &str, module: &str| {
            format!(
                "[[task]]\nid = \"{id}\"\ntitle = \"{module}\"\n\
                 description = \"Add core/src/{module}.cpp.\"\nstatus = \"ready\"\n\
                 role = \"implementer\"\ntier = \"focused\"\nfiles = [\"core/src/{module}.cpp\"]\n\
                 depends_on = []\n\n[task.context]\nread_files = []\n\n\
                 [[task.verify]]\nphase = \"test\"\ncommand = \"tools/spec.sh 1 {module}\"\n\n"
            )
        };
        let plan = format!(
            "[meta]\nplan = \"p1\"\ntotal = 2\ndone = 0\nstatus = \"ready\"\n\n{}{}",
            task("T1", "oscillator"),
            task("T2", "filter")
        );
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&plan],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "p1", workdir)
        };
        generate_plan(request).await.expect("generate the plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "repaired, not sent back");
        let written = TasksFile::parse(&workdir.join("plans/p1/tasks.toml")).expect("the plan");
        for task in &written.tasks {
            assert!(
                task.files.iter().any(|file| file == "core/CMakeLists.txt"),
                "{}: {:?}",
                task.id,
                task.files
            );
            assert!(task.depends_on.is_empty(), "{}: still parallel", task.id);
        }
        let note = shared_registration_note(&written).expect("a shared list");
        assert!(note.contains("`core/CMakeLists.txt` (T1, T2)"), "{note}");
    }

    /// gap-0fb7d9: what roko changed in its planner's draft reaches the
    /// generation's outcome, which serve returns as the plan-writing result,
    /// and the plan's `plan.md`, not only stderr: here the CMake list two
    /// parallel module tasks add to.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_repaired_draft_returns_its_repair_lines() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        std::fs::create_dir_all(workdir.join("core/src")).expect("core");
        std::fs::write(
            workdir.join("core/CMakeLists.txt"),
            "add_library(core STATIC src/params.cpp)\n",
        )
        .expect("the core list");
        std::fs::write(workdir.join("core/src/params.cpp"), "// params\n").expect("params");
        let task = |id: &str, module: &str| {
            format!(
                "[[task]]\nid = \"{id}\"\ntitle = \"{module}\"\n\
                 description = \"Add core/src/{module}.cpp.\"\nstatus = \"ready\"\n\
                 role = \"implementer\"\ntier = \"focused\"\nfiles = [\"core/src/{module}.cpp\"]\n\
                 depends_on = []\n\n[task.context]\nread_files = []\n\n\
                 [[task.verify]]\nphase = \"test\"\ncommand = \"tools/spec.sh 1 {module}\"\n\n"
            )
        };
        let plan = format!(
            "[meta]\nplan = \"p1\"\ntotal = 2\ndone = 0\nstatus = \"ready\"\n\n{}{}",
            task("T1", "oscillator"),
            task("T2", "filter")
        );
        let bin = tempfile::tempdir().expect("tempdir");
        write_scripted_planner(
            workdir,
            bin.path(),
            &[&plan],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "p1", workdir)
        };
        let (_, outcome) = generate_plan(request).await.expect("generate the plan");
        assert_eq!(outcome.repairs.len(), 2, "{:?}", outcome.repairs);
        assert!(
            outcome
                .repairs
                .iter()
                .all(|line| line.contains("core/CMakeLists.txt")),
            "{:?}",
            outcome.repairs
        );
        let plan_md =
            std::fs::read_to_string(workdir.join("plans/p1/plan.md")).expect("the plan's plan.md");
        assert!(plan_md.contains("\n## Repairs\n"), "{plan_md}");
        for line in &outcome.repairs {
            assert!(plan_md.contains(&format!("- {line}\n")), "{plan_md}");
        }
    }

    /// bug-b59d8e: Undertow phase 2's shape. Three module tasks, two of them
    /// parallel, and a wiring task each list `core/CMakeLists.txt`. Before,
    /// every draft failed the generated-plan budget (4 serial owners of one
    /// file, and T2 and T3 overlapping on it), and no plan was written. A
    /// shared registration file is now neither, so one call writes the plan.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_shared_source_list_in_every_module_task_still_writes_a_plan() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        std::fs::create_dir_all(workdir.join("core/src")).expect("core");
        std::fs::write(
            workdir.join("core/CMakeLists.txt"),
            "add_library(undertow_core STATIC src/params.cpp src/engine.cpp)\n",
        )
        .expect("the core list");
        for source in ["params", "engine"] {
            std::fs::write(workdir.join(format!("core/src/{source}.cpp")), "// core\n")
                .expect("a source");
        }
        let task = |id: &str, module: &str, depends_on: &str| {
            format!(
                "[[task]]\nid = \"{id}\"\ntitle = \"{module}\"\n\
                 description = \"Write core/src/{module}.cpp and list it.\"\n\
                 status = \"ready\"\nrole = \"implementer\"\ntier = \"focused\"\n\
                 files = [\"core/src/{module}.cpp\", \"core/CMakeLists.txt\"]\n\
                 depends_on = [{depends_on}]\n\n[task.context]\nread_files = []\n\n\
                 [[task.verify]]\nphase = \"test\"\ncommand = \"tools/spec.sh 2 {module}\"\n\n"
            )
        };
        let plan = format!(
            "[meta]\nplan = \"p2\"\ntotal = 4\ndone = 0\nstatus = \"ready\"\n\n{}{}{}{}",
            task("T1", "lfo", ""),
            task("T2", "rumble", "\"T1\""),
            task("T3", "output", "\"T1\""),
            task("T4", "engine", "\"T1\", \"T2\", \"T3\""),
        );
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&plan],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "p2", workdir)
        };
        generate_plan(request).await.expect("the plan is written");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "written, not sent back");
        let written = TasksFile::parse(&workdir.join("plans/p2/tasks.toml")).expect("the plan");
        assert!(
            written
                .tasks
                .iter()
                .all(|task| task.files.iter().any(|file| file == "core/CMakeLists.txt"))
        );
        let rumble = written
            .tasks
            .iter()
            .find(|task| task.id == "T2")
            .expect("T2");
        assert_eq!(
            rumble.depends_on,
            ["T1"],
            "T2 and T3 still run side by side"
        );
    }

    /// A CMake workspace with Undertow's presets, `core` and `plugin`, and a
    /// core library that collects its sources with a glob.
    #[cfg(unix)]
    fn cmake_workspace(workdir: &Path) {
        std::fs::create_dir_all(workdir.join("core")).expect("core");
        std::fs::write(workdir.join("CMakeLists.txt"), "add_subdirectory(core)\n").expect("cmake");
        std::fs::write(
            workdir.join("core/CMakeLists.txt"),
            "file(GLOB CORE_SOURCES CONFIGURE_DEPENDS src/*.cpp)\n\
             add_library(undertow_core STATIC ${CORE_SOURCES})\n",
        )
        .expect("the core list");
        std::fs::write(
            workdir.join("CMakePresets.json"),
            r#"{"version": 6,
                "configurePresets": [{"name": "core"}, {"name": "plugin"}],
                "buildPresets": [{"name": "core", "configurePreset": "core"},
                                 {"name": "plugin", "configurePreset": "plugin"}],
                "testPresets": [{"name": "core", "configurePreset": "core"}]}"#,
        )
        .expect("presets");
    }

    /// bug-72dc0d: round v7's phase-2 spec names each check's command in its
    /// table. Asked from a Hermes chat, the planner checked every task with
    /// cargo in a CMake repo (op 9fa7849f), and roko wrote that plan with 1 of
    /// 20 checks kept. Now cargo cannot run there, each check's own command
    /// runs in the task that owns it, and the plan keeps all 20 after one
    /// call. A plan none of whose tasks roko can give its checks to is sent
    /// back once, and then refused: no plan is written.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_spec_that_names_each_tasks_checks_gets_its_commands_or_no_plan() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p2_spec_checks");
        let request = std::fs::read_to_string(fixture.join("request.md")).expect("the spec");
        let plan = std::fs::read_to_string(fixture.join("plan2.toml")).expect("plan 2");
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        cmake_workspace(workdir);
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&plan],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let source = PlanSource::Text {
            text: &request,
            kind: "prompt",
        };
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(source, "p2-undertow-rumble", workdir)
        };
        let (_, outcome) = generate_plan(request).await.expect("the plan is written");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "repaired, not asked again");
        let written = TasksFile::parse(&workdir.join("plans/p2-undertow-rumble/tasks.toml"))
            .expect("the plan");
        let commands: Vec<&str> = written
            .tasks
            .iter()
            .flat_map(|task| &task.verify)
            .map(|step| step.command.as_str())
            .collect();
        assert!(
            commands.iter().all(|command| !command.contains("cargo")),
            "{commands:#?}"
        );
        for (task, run) in [
            ("T1", "tools/spec.sh 2 lfo 'P2.03*'"),
            ("T2", "tools/spec.sh 2 rumble 'P2.08*'"),
            ("T3", "tools/spec.sh 2 output 'P2.09*'"),
            ("T4", "tools/spec.sh 2 engine 'P2.19*'"),
        ] {
            let task = written.tasks.iter().find(|t| t.id == task).expect("a task");
            assert!(
                task.verify.iter().any(|step| step.command.contains(run)),
                "{}: {:?}",
                task.id,
                task.verify
            );
        }
        let checks = outcome
            .source_checks
            .expect("the spec's checks are reported");
        assert_eq!((checks.kept, checks.total), (20, 20), "{checks:?}");
        assert!(!checks.regenerated);

        // A plan whose task no check can be given to: refused after asking
        // once more.
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let motor = "[meta]\nplan = \"widget\"\ntotal = 1\ndone = 0\nstatus = \"ready\"\n\n\
                     [[task]]\nid = \"T1\"\ntitle = \"Add the motor\"\n\
                     description = \"Write src/motor.cpp.\"\nstatus = \"ready\"\n\
                     role = \"implementer\"\ntier = \"focused\"\nfiles = [\"src/motor.cpp\"]\n\
                     depends_on = []\n\n[task.context]\nread_files = []\n\n\
                     [[task.verify]]\nphase = \"test\"\ncommand = \"grep -q motor src/motor.cpp\"\n";
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[motor],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let source = PlanSource::Text {
            text: "# Widget\n\n## Acceptance checks\n\n| Id | Command | Check |\n|---|---|---|\n\
                   | W.01 | `tools/spec.sh widget spin` | the widget spins |\n\
                   | W.02 | `tools/spec.sh widget stop` | the widget stops |\n\
                   | W.03 | `tools/spec.sh widget wobble` | the widget wobbles |\n",
            kind: "prompt",
        };
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(source, "widget", workdir)
        };
        let error = generate_plan(request)
            .await
            .expect_err("a plan that keeps none of its spec's checks");
        let error = format!("{error:#}");
        assert!(
            error.contains("wrote no plan") && error.contains("keeps 0 of the 3 checks"),
            "{error}"
        );
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 2, "asked once more, then refused");
        assert!(!workdir.join("plans/widget/tasks.toml").exists());
    }

    /// bug-56a806: round v7's third phase-2 plan write (op 1a965a97) put a
    /// guard of the planner's own before each of the spec's 15 check
    /// commands, `grep -q '<name>' <file> && tools/spec.sh 2 …`. T4's grep
    /// for `AllocationCounter`, a helper of the spec's tests, fails on correct
    /// code, and roko counted all 24 checks kept. Generation now writes each
    /// step as the spec's own commands alone, after one call, and keeps all 24.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_planners_guards_around_spec_commands_are_dropped() {
        use std::os::unix::fs::PermissionsExt as _;

        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p2_exact_commands");
        let spec = std::fs::read_to_string(fixture.join("request.md")).expect("the spec");
        let plan = std::fs::read_to_string(fixture.join("tasks.toml")).expect("the plan");
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        cmake_workspace(workdir);
        // The files the plan's tasks read, with the symbols they name, and
        // the spec's scripts.
        for (file, text) in [
            ("core/include/undertow/voice.h", "class MonoVoice;\n"),
            (
                "core/include/undertow/engine.h",
                "struct Transport;\nstruct Meters;\n",
            ),
            ("core/src/params.cpp", "constexpr int kNumParams = 57;\n"),
            (
                "core/src/engine.cpp",
                "Transport transport;\nMeters meters;\n",
            ),
            ("tools/spec.sh", "#!/bin/sh\nexit 1\n"),
            ("tools/check.sh", "#!/bin/sh\nexit 1\n"),
        ] {
            let path = workdir.join(file);
            std::fs::create_dir_all(path.parent().expect("a directory")).expect("directory");
            std::fs::write(&path, text).expect("a file");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("permissions");
        }
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&plan],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let source = PlanSource::Text {
            text: &spec,
            kind: "prompt",
        };
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(source, "p2-undertow-rumble-3", workdir)
        };
        let (_, outcome) = generate_plan(request).await.expect("the plan is written");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "repaired, not asked again");
        let written = TasksFile::parse(&workdir.join("plans/p2-undertow-rumble-3/tasks.toml"))
            .expect("the plan");
        let commands: Vec<(&str, &str)> = written
            .tasks
            .iter()
            .flat_map(|task| {
                task.verify
                    .iter()
                    .map(|step| (task.id.as_str(), step.command.as_str()))
            })
            .collect();
        assert_eq!(
            commands,
            [
                ("T1", "tools/spec.sh 2 lfo 'P2.01*'"),
                ("T1", "tools/spec.sh 2 lfo 'P2.02*'"),
                ("T1", "tools/spec.sh 2 lfo 'P2.03*'"),
                ("T2", "tools/spec.sh 2 rumble 'P2.04*'"),
                ("T2", "tools/spec.sh 2 rumble 'P2.05*'"),
                ("T2", "tools/spec.sh 2 rumble 'P2.06*'"),
                ("T2", "tools/spec.sh 2 rumble 'P2.07*'"),
                ("T2", "tools/spec.sh 2 rumble 'P2.08*'"),
                ("T3", "tools/spec.sh 2 output 'P2.09*,P2.10*'"),
                ("T3", "tools/spec.sh 2 output 'P2.11*,P2.12*'"),
                ("T3", "tools/spec.sh 2 output 'P2.20*,P2.21*'"),
                ("T4", "tools/spec.sh 2 engine 'P2.13*,P2.14*,P2.23*'"),
                ("T4", "tools/spec.sh 2 engine 'P2.15*,P2.16*'"),
                ("T4", "tools/spec.sh 2 engine 'P2.17*,P2.22*'"),
                ("T4", "tools/spec.sh 2 engine 'P2.18*,P2.19*'"),
            ]
        );
        let meta: Vec<&str> = written
            .meta
            .verify
            .iter()
            .map(|step| step.command.as_str())
            .collect();
        assert_eq!(meta, ["tools/check.sh"]);
        let checks = outcome
            .source_checks
            .expect("the spec's checks are reported");
        assert_eq!((checks.kept, checks.total), (24, 24), "{checks:?}");
        assert!(!checks.regenerated);
    }

    /// bug-ce1e4b: round v7's phase-5 spec (Undertow `spec/p5-f15`) gives
    /// task 3 a new `tools/render/CMakeLists.txt` and the top-level
    /// `CMakeLists.txt`, which it reads. Replayed with a draft that follows
    /// the spec's notes, generation rewrote the new file to the top-level one,
    /// in T3's files and its text. Now the draft is written after one call
    /// with its files as planned, no file-path repair, and all 13 checks kept.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_new_file_the_source_names_keeps_its_path() {
        use std::os::unix::fs::PermissionsExt as _;

        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p5_ship");
        let spec = std::fs::read_to_string(fixture.join("request.md")).expect("the spec");
        let plan = std::fs::read_to_string(fixture.join("tasks.toml")).expect("the plan");
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        cmake_workspace(workdir);
        // The files the draft's tasks read, long enough for their ranges, and
        // the spec and its scripts.
        let code = "// code\n".repeat(240);
        for file in [
            "core/include/undertow/params.h",
            "core/include/undertow/sequencer.h",
            "core/include/undertow/engine.h",
            "core/include/undertow/filter.h",
            "core/src/filter.cpp",
            "core/src/sequencer.cpp",
            "plugin/PluginProcessor.h",
            "plugin/PluginProcessor.cpp",
            "plugin/CMakeLists.txt",
        ] {
            let path = workdir.join(file);
            std::fs::create_dir_all(path.parent().expect("a directory")).expect("directory");
            std::fs::write(&path, &code).expect("a file");
        }
        std::fs::create_dir_all(workdir.join("docs/specs")).expect("docs");
        std::fs::write(workdir.join("docs/specs/phase-5-ship.md"), &spec).expect("the spec");
        for script in ["tools/spec.sh", "tools/check.sh", "tools/pluginval.sh"] {
            let path = workdir.join(script);
            std::fs::create_dir_all(path.parent().expect("a directory")).expect("directory");
            std::fs::write(&path, "#!/bin/sh\nexit 1\n").expect("a script");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("permissions");
        }
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&plan],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let source = PlanSource::Text {
            text: &spec,
            kind: "prompt",
        };
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(source, "phase-5-ship", workdir)
        };
        let (_, outcome) = generate_plan(request).await.expect("the plan is written");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "written after one call");
        assert!(
            !outcome
                .repairs
                .iter()
                .any(|line| line.contains("in its files and its text")),
            "{:?}",
            outcome.repairs
        );
        let written =
            TasksFile::parse(&workdir.join("plans/phase-5-ship/tasks.toml")).expect("the plan");
        let renderer = written
            .tasks
            .iter()
            .find(|task| task.id == "T3")
            .expect("task 3");
        assert_eq!(
            renderer.files,
            [
                "tools/render/main.cpp",
                "tools/render/CMakeLists.txt",
                "CMakeLists.txt"
            ]
        );
        assert!(
            renderer
                .description
                .as_deref()
                .is_some_and(|text| text.contains("(`tools/render/CMakeLists.txt`,")),
            "{:?}",
            renderer.description
        );
        let checks = outcome
            .source_checks
            .expect("the spec's checks are reported");
        assert_eq!((checks.kept, checks.total), (13, 13), "{checks:?}");
    }

    /// Phase 4.2's editor as its base (Undertow 4db168c) had it, in brief:
    /// each file the plan's drafts name, as long as it was, with the lines
    /// that hold the symbols they anchor where they were, and the spec.
    #[cfg(unix)]
    fn editor_workspace(workdir: &Path, spec: &str) {
        use std::os::unix::fs::PermissionsExt as _;

        cmake_workspace(workdir);
        let code = |lines: usize, held: &[(usize, &str)]| {
            (1..=lines)
                .map(|number| {
                    held.iter().find(|(at, _)| *at == number).map_or_else(
                        || format!("// line {number}\n"),
                        |(_, line)| format!("{line}\n"),
                    )
                })
                .collect::<String>()
        };
        for (file, text) in [
            (
                "plugin/ui/Theme.h",
                code(
                    233,
                    &[
                        (5, "namespace undertow::ui::theme {"),
                        (40, "inline const Panel controlPanels[] = {};"),
                    ],
                ),
            ),
            (
                "plugin/ui/Look.h",
                code(
                    29,
                    &[(
                        23,
                        "    void drawStaticLayer(juce::Graphics&, float physicalScale);",
                    )],
                ),
            ),
            (
                "plugin/ui/Look.cpp",
                code(
                    131,
                    &[
                        (97, "    drawStaticLayer(ig, scale);"),
                        (
                            100,
                            "void Look::drawStaticLayer(juce::Graphics& g, float) {",
                        ),
                    ],
                ),
            ),
            (
                "plugin/PluginEditor.h",
                code(56, &[(40, "    void initialiseControls();")]),
            ),
            (
                "plugin/PluginEditor.cpp",
                code(
                    175,
                    &[
                        (36, "    initialiseControls();"),
                        (47, "void UndertowEditor::initialiseControls() {"),
                    ],
                ),
            ),
            (
                "core/src/engine.cpp",
                code(
                    305,
                    &[
                        (20, "struct Engine::Impl {"),
                        (120, "void Engine::reset() {"),
                    ],
                ),
            ),
            ("plugin/ui/Status.h", code(30, &[(10, "class Status {")])),
            (
                "plugin/ui/Status.cpp",
                code(116, &[(30, "void Status::paint(juce::Graphics& g) {")]),
            ),
            (
                "plugin/ui/Displays.h",
                code(40, &[(12, "class Displays {")]),
            ),
            (
                "plugin/ui/Displays.cpp",
                code(
                    241,
                    &[(150, "void Displays::paintBone(juce::Graphics& g) {")],
                ),
            ),
            ("docs/specs/phase-4.2-layout.md", spec.to_string()),
        ] {
            let path = workdir.join(file);
            std::fs::create_dir_all(path.parent().expect("a directory")).expect("directory");
            std::fs::write(&path, text).expect("a file");
        }
        for script in ["tools/spec.sh", "tools/check.sh"] {
            let path = workdir.join(script);
            std::fs::create_dir_all(path.parent().expect("a directory")).expect("directory");
            std::fs::write(&path, "#!/bin/sh\nexit 1\n").expect("a script");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("permissions");
        }
    }

    /// bug-417c4a: round v7's phase-4.2 plan write (op 1b0a97ed, gpt-5.4)
    /// went back to the planner twice on PLAN_CONTEXT_SYMBOL. Its first draft
    /// anchored `Look::drawStaticLayer` without reading `plugin/ui/Look.cpp`,
    /// its second `UndertowEditor::initialiseControls` without reading
    /// `plugin/PluginEditor.cpp`; the base defines both, in files T1 owns.
    /// Each draft is now written after one call, with that file among T1's
    /// reads and every check of the spec kept.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_symbol_in_a_file_the_task_does_not_read_is_repaired_not_regenerated() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p4_2_symbols");
        let spec = std::fs::read_to_string(fixture.join("request.md")).expect("the spec");
        for (draft, file, symbol) in [
            (
                "draft-1.toml",
                "plugin/ui/Look.cpp",
                "Look::drawStaticLayer",
            ),
            (
                "draft-2.toml",
                "plugin/PluginEditor.cpp",
                "UndertowEditor::initialiseControls",
            ),
        ] {
            let plan = std::fs::read_to_string(fixture.join(draft)).expect("the draft");
            let temp = tempfile::tempdir().expect("tempdir");
            let workdir = temp.path();
            editor_workspace(workdir, &spec);
            let bin = tempfile::tempdir().expect("tempdir");
            let calls = write_scripted_planner(
                workdir,
                bin.path(),
                &[&plan],
                "allow_threshold = 0.0\nblock_threshold = 0.0\n",
            );
            let source = PlanSource::Text {
                text: &spec,
                kind: "prompt",
            };
            let request = PlanRequest {
                model: Some("planner"),
                ..PlanRequest::new(source, "p4-2-the-editor-laid-out-as-designed", workdir)
            };
            let (_, outcome) = generate_plan(request)
                .await
                .unwrap_or_else(|error| panic!("{draft}: {error:#}"));
            let log = std::fs::read_to_string(&calls).expect("planner call log");
            assert_eq!(log.lines().count(), 1, "{draft}: written after one call");
            let line = format!("T1: reads `{file}`, which holds `{symbol}`, a symbol it names");
            assert!(
                outcome.repairs.contains(&line),
                "{draft}: {:?}",
                outcome.repairs
            );
            let written = TasksFile::parse(
                &workdir.join("plans/p4-2-the-editor-laid-out-as-designed/tasks.toml"),
            )
            .expect("the plan");
            let reads: Vec<&str> = written.tasks[0]
                .context
                .iter()
                .flat_map(|context| &context.read_files)
                .map(|read| read.path.as_str())
                .collect();
            assert!(reads.contains(&file), "{draft}: {reads:?}");
            let checks = outcome
                .source_checks
                .expect("the spec's checks are reported");
            assert_eq!((checks.kept, checks.total), (19, 19), "{draft}: {checks:?}");
        }
    }

    /// gap-4dd77d: round v7's phase-3 re-plan (op 3b13af5f) wrote `verify =
    /// ["tools/check.sh"]` in `[meta]`, where step tables go. The draft failed
    /// to parse and went back for a whole new one. The bare command is now
    /// the step that runs it, after one call.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_string_verify_step_is_repaired_not_regenerated() {
        let draft = WIDGET_PLAN.replace(
            "status = \"ready\"\n\n[[task]]",
            "status = \"ready\"\nverify = [\"test -f src/widget.rs\"]\n\n[[task]]",
        );
        assert_ne!(draft, WIDGET_PLAN);
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_fake_planner(workdir, bin.path(), &draft);
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        let (_, outcome) = generate_plan(request).await.expect("the plan is written");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "repaired, not asked again");
        let line = "`[meta]`: the verify step `test -f src/widget.rs`, written as its bare command, \
                    became a step that runs it";
        assert!(
            outcome.repairs.iter().any(|repair| repair == line),
            "{:?}",
            outcome.repairs
        );
        let written = TasksFile::parse(&workdir.join("plans/widget/tasks.toml")).expect("the plan");
        let commands: Vec<&str> = written
            .meta
            .verify
            .iter()
            .map(|step| step.command.as_str())
            .collect();
        assert_eq!(commands, ["test -f src/widget.rs"]);
    }

    /// gap-4dd77d: a task over its tier's size limits that a higher tier
    /// fits went back to the planner for a whole new draft (round v7's phase
    /// 0: an integrative task with 7 steps; phase 4.1: a mechanical one with
    /// 5 files and 4 steps), though the check names the fix. Its tier is now
    /// raised, after one call.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_task_over_its_tier_is_raised_not_regenerated() {
        let steps: String = ["fn", "pub", "struct"]
            .iter()
            .map(|word| {
                format!(
                    "\n[[task.verify]]\nphase = \"check\"\ncommand = \"grep -q {word} src/widget.rs\"\n"
                )
            })
            .collect();
        let draft = WIDGET_PLAN.replace("tier = \"focused\"", "tier = \"mechanical\"") + &steps;
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_fake_planner(workdir, bin.path(), &draft);
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        let (_, outcome) = generate_plan(request).await.expect("the plan is written");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "raised, not asked again");
        let line = "T1: tier mechanical became focused, to fit its 4 verify steps (limit 3)";
        assert!(
            outcome.repairs.iter().any(|repair| repair == line),
            "{:?}",
            outcome.repairs
        );
        let written = TasksFile::parse(&workdir.join("plans/widget/tasks.toml")).expect("the plan");
        assert_eq!(written.tasks[0].tier, "focused");
        assert_eq!(written.tasks[0].verify.len(), 4);
    }

    /// gap-4dd77d: round v7's phase-4.3 plan write (op ff02a005, gpt-5.4-mini)
    /// kept 12 of its spec's 17 checks. The coverage re-ask's regeneration
    /// put T1's four regression checks back, at 5 verify steps on a focused
    /// task (limit 4), and roko threw it away on PLAN_TIER_SIZE and wrote
    /// the draft that lacked them. The regeneration is now kept, with T1
    /// raised to integrative; the P4.07 it dropped to the plan's final check
    /// runs in T2 (gap-8b1f4f), so it keeps all 17.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_coverage_regeneration_over_its_tier_is_kept_with_the_tier_raised() {
        use std::os::unix::fs::PermissionsExt as _;

        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p4_3_tiers");
        let spec = std::fs::read_to_string(fixture.join("request.md")).expect("the spec");
        let first = std::fs::read_to_string(fixture.join("draft-1.toml")).expect("the draft");
        let again =
            std::fs::read_to_string(fixture.join("draft-2.toml")).expect("the regeneration");
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        cmake_workspace(workdir);
        let code = "// code\n".repeat(260);
        for file in [
            "plugin/ui/Theme.h",
            "plugin/ui/Status.h",
            "plugin/ui/Status.cpp",
            "plugin/ui/Displays.cpp",
            "plugin/PluginEditor.cpp",
            "tests/spec/p4_3/panels_test.cpp",
        ] {
            let path = workdir.join(file);
            std::fs::create_dir_all(path.parent().expect("a directory")).expect("directory");
            std::fs::write(&path, &code).expect("a file");
        }
        std::fs::create_dir_all(workdir.join("docs/specs")).expect("docs");
        std::fs::write(workdir.join("docs/specs/phase-4.3-finish.md"), &spec).expect("the spec");
        for script in ["tools/spec.sh", "tools/check.sh"] {
            let path = workdir.join(script);
            std::fs::create_dir_all(path.parent().expect("a directory")).expect("directory");
            std::fs::write(&path, "#!/bin/sh\nexit 1\n").expect("a script");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("permissions");
        }
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&first, &again],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let source = PlanSource::Text {
            text: &spec,
            kind: "prompt",
        };
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(source, "p4-3-the-editor-s-finish", workdir)
        };
        let (_, outcome) = generate_plan(request).await.expect("the plan is written");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 2, "the draft, then the regeneration");
        let line = "T1: tier focused became integrative, to fit its 5 verify steps (limit 4)";
        assert!(
            outcome.repairs.iter().any(|repair| repair == line),
            "{:?}",
            outcome.repairs
        );
        let written = TasksFile::parse(&workdir.join("plans/p4-3-the-editor-s-finish/tasks.toml"))
            .expect("the plan");
        assert_eq!(written.tasks[0].tier, "integrative");
        assert_eq!(written.tasks[0].verify.len(), 5);
        let checks = outcome
            .source_checks
            .expect("the spec's checks are reported");
        assert!(checks.regenerated, "{checks:?}");
        // The draft kept 12 and left 5 in no step.
        assert_eq!(
            (checks.kept, checks.dropped, checks.missing, checks.total),
            (17, 0, 0, 17),
            "{checks:?}"
        );
    }

    /// gap-5c7f9a: round v7's phase-4.1 notes give the plan one task, "One
    /// task, about 5 lines: **Bone only for the one number** (…)", and its
    /// write from chat (op 7cda5da7) had two, the colours and the PNGs; nothing
    /// said so. A draft whose task count differs from the notes' is asked
    /// again in the coverage re-ask, and a one-task answer is kept; a second
    /// two-task answer is written, with a warning line among the repairs.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_draft_with_a_different_task_count_than_the_notes_is_asked_again() {
        use std::os::unix::fs::PermissionsExt as _;

        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p4_1_tasks");
        let spec = std::fs::read_to_string(fixture.join("request.md")).expect("the spec");
        let two = std::fs::read_to_string(fixture.join("two-tasks.toml")).expect("two tasks");
        let one = std::fs::read_to_string(fixture.join("one-task.toml")).expect("one task");
        let workspace = |workdir: &Path| {
            cmake_workspace(workdir);
            let code = |lines: usize, held: &[&str]| {
                let mut text: String = held.iter().map(|line| format!("{line}\n")).collect();
                text.push_str(&"// code\n".repeat(lines));
                text
            };
            for (file, text) in [
                (
                    "plugin/ui/Look.cpp",
                    code(70, &["Look::Look() {}", "void Look::drawRotarySlider() {}"]),
                ),
                (
                    "plugin/ui/Status.cpp",
                    code(90, &["void Status::paint() {}"]),
                ),
                ("artifacts/ui/editor-idle.png", "png".to_string()),
                ("artifacts/ui/editor-playing.png", "png".to_string()),
                ("artifacts/ui/editor-clipping.png", "png".to_string()),
                ("docs/specs/phase-4.1-bone.md", spec.clone()),
            ] {
                let path = workdir.join(file);
                std::fs::create_dir_all(path.parent().expect("a directory")).expect("directory");
                std::fs::write(&path, text).expect("a file");
            }
            for script in ["tools/spec.sh", "tools/check.sh"] {
                let path = workdir.join(script);
                std::fs::create_dir_all(path.parent().expect("a directory")).expect("directory");
                std::fs::write(&path, "#!/bin/sh\nexit 1\n").expect("a script");
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                    .expect("permissions");
            }
        };
        let warning = "the plan has 2 task(s) where the source's notes for the planner state 1, \
                       after asking the planner once more: check the split before it runs";
        for (answers, tasks, warned) in [
            ([two.as_str(), one.as_str()], 1, false),
            ([two.as_str(), two.as_str()], 2, true),
        ] {
            let temp = tempfile::tempdir().expect("tempdir");
            let workdir = temp.path();
            workspace(workdir);
            let bin = tempfile::tempdir().expect("tempdir");
            let calls = write_scripted_planner(
                workdir,
                bin.path(),
                &answers,
                "allow_threshold = 0.0\nblock_threshold = 0.0\n",
            );
            let source = PlanSource::Text {
                text: &spec,
                kind: "prompt",
            };
            let request = PlanRequest {
                model: Some("planner"),
                ..PlanRequest::new(source, "p4-1-bone-only-for-the-one-number", workdir)
            };
            let (_, outcome) = generate_plan(request)
                .await
                .unwrap_or_else(|error| panic!("{tasks} task(s): {error:#}"));
            let log = std::fs::read_to_string(&calls).expect("planner call log");
            assert_eq!(log.lines().count(), 2, "asked again once");
            let again = std::fs::read_to_string(bin.path().join("prompt-1.txt")).expect("the ask");
            assert!(
                again.contains(
                    "The source's notes for the planner state 1 task, and your plan below has 2"
                ),
                "{again}"
            );
            let written = TasksFile::parse(
                &workdir.join("plans/p4-1-bone-only-for-the-one-number/tasks.toml"),
            )
            .expect("the plan");
            assert_eq!(written.tasks.len(), tasks);
            assert_eq!(
                outcome.repairs.iter().any(|line| line == warning),
                warned,
                "{:?}",
                outcome.repairs
            );
        }
    }

    /// bug-8723fc: in a CMake workspace (Undertow's phase 2), the plan writer
    /// taught by Rust examples and added `cargo check --workspace` to a task
    /// with no check. Now the examples, the note on runners and a task with
    /// no check use the workspace's own CMake presets, and name no cargo
    /// command; a Rust workspace's prompt is unchanged.
    #[cfg(unix)]
    #[test]
    fn a_cmake_workspace_gets_no_cargo_defaults() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        cmake_workspace(workdir);
        let prompt = crate::plan_generate::build_generator_system_prompt(workdir);
        for rust in [
            "cargo check",
            "cargo test",
            "crates/roko-core",
            "FundingRate",
        ] {
            assert!(!prompt.contains(rust), "{rust}");
        }
        for cmake in [
            "command = \"cmake --preset core && cmake --build --preset core\"",
            "ctest --preset core -R gain",
            "## This workspace's build",
            "no verify step runs `cargo`",
        ] {
            assert!(prompt.contains(cmake), "{cmake}");
        }
        let rust = tempfile::tempdir().expect("tempdir");
        std::fs::write(rust.path().join("Cargo.toml"), "[workspace]\n").expect("cargo");
        let rust_prompt = crate::plan_generate::build_generator_system_prompt(rust.path());
        assert!(rust_prompt.contains("command = \"cargo check -p roko-core\""));
        assert!(!rust_prompt.contains("## This workspace's build"));

        let draft = "[meta]\nplan = \"p2\"\ntotal = 1\nstatus = \"ready\"\n\n\
                     [[task]]\nid = \"T4\"\ntitle = \"Wire the engine\"\nstatus = \"ready\"\n\
                     role = \"implementer\"\ntier = \"focused\"\n\
                     files = [\"core/src/engine.cpp\"]\n";
        let fixed = validate_and_fix_generated_plan(
            draft,
            "p2",
            &empty_models(),
            None,
            &LadderConfig::default(),
        )
        .expect("the draft is fixed");
        assert!(!fixed.contains("cargo"), "{fixed}");
        let mut plan = TasksFile::parse_str(&fixed).expect("the plan");
        let probe = runnable::Probe::with(&[("cmake", true), ("ctest", true)], &[]);
        let added = runnable::repair_build_checks(&mut plan, workdir, &probe);
        assert_eq!(added.len(), 1, "{added:?}");
        assert_eq!(
            plan.tasks[0].verify[0].command,
            "cmake --preset core && cmake --build --preset core"
        );
        let runners = runnable::runners_block(workdir, &probe).expect("a CMake project");
        assert!(runners.contains("no `Cargo.toml` here"), "{runners}");
    }

    /// bug-16b798: phase 2's three drafts were fenced and parsed, and each
    /// failed the generated-plan budget, yet the error blamed the fencing and
    /// suggested a more capable model; and the budget's findings hid the
    /// drafts' tier-size ones. Every gate's findings now go back at once, and
    /// the failure says the drafts parsed and failed roko's checks.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_budget_failure_names_the_budget_not_the_fencing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let task = |id: &str, depends_on: &str, max_loc: u32| {
            format!(
                "[[task]]\nid = \"{id}\"\ntitle = \"Widget part {id}\"\n\
                 description = \"Extend src/widget.rs.\"\nstatus = \"ready\"\n\
                 role = \"implementer\"\ntier = \"focused\"\nmax_loc = {max_loc}\n\
                 files = [\"src/widget.rs\"]\ndepends_on = [{depends_on}]\n\n\
                 [task.context]\nread_files = []\n\n\
                 [[task.verify]]\nphase = \"test\"\ncommand = \"tools/check.sh widget {id}\"\n\n"
            )
        };
        let plan = format!(
            "[meta]\nplan = \"widget\"\ntotal = 3\ndone = 0\nstatus = \"ready\"\n\n{}{}{}",
            task("T1", "", 500),
            task("T2", "\"T1\"", 40),
            task("T3", "\"T2\"", 40),
        );
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&plan],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        let error = generate_plan(request)
            .await
            .expect_err("no draft passes the checks");
        let error = format!("{error:#}");
        for expected in [
            "PLAN_FRAGMENTED_OWNERSHIP",
            "PLAN_TIER_SIZE",
            "did not pass roko's plan checks",
        ] {
            assert!(error.contains(expected), "{expected}: {error}");
        }
        for wrong in ["parseable ```toml block", "Try a more capable model"] {
            assert!(!error.contains(wrong), "{wrong}: {error}");
        }
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 3, "the draft, then two fix-ups");
        let fixup = std::fs::read_to_string(bin.path().join("prompt-1.txt")).expect("a fix-up");
        assert!(
            fixup.contains("PLAN_FRAGMENTED_OWNERSHIP") && fixup.contains("PLAN_TIER_SIZE"),
            "{fixup}"
        );
    }

    /// Round v7, phase 0: asked naturally, a chat host summarised the spec
    /// and named its file, and roko planned without the spec's checks. A
    /// request that states no checks and names a spec of the workspace now
    /// gets the spec in the planner's prompt and its checks as the plan's
    /// source checks, run by the task that owns them.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_summary_naming_its_spec_is_planned_with_the_specs_checks() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        std::fs::create_dir_all(workdir.join("docs/specs")).expect("docs");
        let spec = "# Widget\n\nSpin the widget.\n\n## Acceptance checks\n\n\
                    | Id | Command | Check |\n|---|---|---|\n\
                    | W.01 | `tools/spec.sh widget spin` | the widget spins |\n\
                    | W.02 | `tools/spec.sh widget stop` | the widget stops |\n";
        std::fs::write(workdir.join("docs/specs/widget.md"), spec).expect("the spec");
        let plan = "[meta]\nplan = \"widget\"\ntotal = 1\ndone = 0\nstatus = \"ready\"\n\n\
                    [[task]]\nid = \"T1\"\ntitle = \"Spin the widget\"\n\
                    description = \"Write src/widget.cpp.\"\nstatus = \"ready\"\n\
                    role = \"implementer\"\ntier = \"focused\"\nfiles = [\"src/widget.cpp\"]\n\
                    depends_on = []\n\n[task.context]\nread_files = []\n\n\
                    [[task.verify]]\nphase = \"test\"\ncommand = \"grep -q spin src/widget.cpp\"\n";
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[plan],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(
                PlanSource::Text {
                    text: "Write the plan for the widget from docs/specs/widget.md. Ensure each \
                           task has checks aligned to the spec. Don't run anything.",
                    kind: "prompt",
                },
                "widget",
                workdir,
            )
        };
        let (_, outcome) = generate_plan(request).await.expect("the plan is written");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1);
        let asked = std::fs::read_to_string(bin.path().join("prompt-0.txt")).expect("the call");
        for expected in [
            "## Documents the prompt names",
            "| W.02 | `tools/spec.sh widget stop` | the widget stops |",
            "`docs/specs/widget.md`, which the prompt names, states the 2 checks below",
        ] {
            assert!(asked.contains(expected), "{expected}: {asked}");
        }
        let checks = outcome
            .source_checks
            .expect("the spec's checks are reported");
        assert_eq!((checks.kept, checks.total), (2, 2), "{checks:?}");
        assert_eq!(checks.documents, ["docs/specs/widget.md"]);
        let written = TasksFile::parse(&workdir.join("plans/widget/tasks.toml")).expect("plan");
        // gap-77220d: each check's command in a step of its own.
        let commands: Vec<&str> = written.tasks[0]
            .verify
            .iter()
            .map(|step| step.command.as_str())
            .collect();
        for command in ["tools/spec.sh widget spin", "tools/spec.sh widget stop"] {
            assert!(commands.contains(&command), "{command}: {commands:?}");
        }
    }

    /// bug-96115c: a request that names its final check gets a plan whose
    /// `[[meta.verify]]` runs it, though the planner wrote none, after one
    /// call: the check counts as kept, so nothing is asked again.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generation_runs_the_requests_final_check_in_meta_verify() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[WIDGET_PLAN],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(
                PlanSource::Text {
                    text: "# Widget\n\nAdd a widget module.\n\n## Notes for the planner\n\n\
                           - `tools/check.sh` is the plan's final check, not any one task's.\n",
                    kind: "file",
                },
                "widget",
                workdir,
            )
        };
        let (_, outcome) = generate_plan(request)
            .await
            .expect("generate the widget plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "repaired, not asked again");
        let written =
            TasksFile::parse(&workdir.join("plans/widget/tasks.toml")).expect("the widget plan");
        let commands: Vec<&str> = written
            .meta
            .verify
            .iter()
            .map(|step| step.command.as_str())
            .collect();
        assert_eq!(commands, ["tools/check.sh"]);
        let checks = outcome.source_checks.expect("the request states a check");
        assert_eq!((checks.kept, checks.total), (1, 1), "{checks:?}");
    }

    /// bug-d758f6: round v7's phase-0 planner copied the prompt's example
    /// tool list into every task of a C++ plan, so no implementer could run
    /// cmake, its own check or see a compiler error. Generation gives an
    /// implementer whose list leaves out a shell its role's own tools, and
    /// leaves a scribe's list alone, without asking the planner again.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generated_implementers_whose_checks_build_keep_a_shell() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        std::fs::create_dir_all(workdir.join("tools")).expect("tools");
        std::fs::write(workdir.join("CMakeLists.txt"), "project(undertow)\n").expect("cmake");
        std::fs::write(workdir.join("tools/spec.sh"), "#!/bin/sh\nexit 0\n").expect("spec.sh");
        let plan = "\
            [meta]\nplan = \"p0\"\ntotal = 2\ndone = 0\nstatus = \"ready\"\n\n\
            [[task]]\nid = \"T1\"\ntitle = \"Parameter table\"\n\
            description = \"Write core/params.h with the 57 parameters.\"\nstatus = \"ready\"\n\
            role = \"implementer\"\ntier = \"focused\"\nfiles = [\"core/params.h\"]\n\
            allowed_tools = [\"read_file\", \"write_file\", \"grep\"]\ndenied_tools = []\n\
            depends_on = []\n\n[task.context]\nread_files = []\n\n\
            [[task.verify]]\nphase = \"test\"\n\
            command = \"grep -q 'kNumParams' core/params.h && tools/spec.sh 0 params 'P0.01*'\"\n\n\
            [[task]]\nid = \"T2\"\ntitle = \"Notes\"\ndescription = \"Write docs/notes.md.\"\n\
            status = \"ready\"\nrole = \"scribe\"\ntier = \"focused\"\nfiles = [\"docs/notes.md\"]\n\
            allowed_tools = [\"read_file\", \"write_file\"]\ndepends_on = [\"T1\"]\n\n\
            [task.context]\nread_files = []\n\n\
            [[task.verify]]\nphase = \"structural\"\ncommand = \"test -f docs/notes.md\"\n";
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[plan],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "p0", workdir)
        };
        generate_plan(request).await.expect("generate the plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "roko repairs the plan itself");
        let written = TasksFile::parse(&workdir.join("plans/p0/tasks.toml")).expect("the plan");
        let implementer = &written.tasks[0];
        let shell = implementer.allowed_tools.as_ref().is_none_or(|tools| {
            ["bash", "edit_file"]
                .iter()
                .all(|tool| tools.iter().any(|kept| kept == tool))
        });
        assert!(shell, "{:?}", implementer.allowed_tools);
        assert!(
            implementer
                .denied_tools
                .iter()
                .flatten()
                .all(|tool| tool != "bash" && tool != "edit_file"),
            "{:?}",
            implementer.denied_tools
        );
        assert_eq!(
            written.tasks[1].allowed_tools.as_deref(),
            Some(&["read_file".to_string(), "write_file".to_string()][..])
        );
    }

    /// bug-27f00b: a live planner gave the one task that writes the spec
    /// `max_loc = 300` on the integrative tier (limit 150), and split the spec
    /// across tasks when told to shrink it, which fragments its ownership. A
    /// task that writes only documents keeps its tier: its `max_loc` becomes
    /// the tier's limit, and the plan is written after one call. A code task
    /// over the limit is still sent back.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_long_document_keeps_its_one_task() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let spec = WIDGET_PLAN
            .replace("files = [\"src/widget.rs\"]", "files = [\"docs/spec.md\"]")
            .replace("test -f src/widget.rs", "test -f docs/spec.md")
            .replace("role = \"implementer\"", "role = \"scribe\"")
            .replace(
                "tier = \"focused\"",
                "tier = \"integrative\"\nmax_loc = 300",
            );
        assert!(spec.contains("max_loc = 300"), "{spec}");
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&spec],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        generate_plan(request)
            .await
            .expect("generate the widget plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "repaired, not sent back");
        let written =
            TasksFile::parse(&workdir.join("plans/widget/tasks.toml")).expect("the widget plan");
        assert_eq!(written.tasks[0].max_loc, Some(150));
        assert_eq!(written.tasks[0].tier, "integrative");

        let code = WIDGET_PLAN.replace("tier = \"focused\"", "tier = \"focused\"\nmax_loc = 300");
        let mut plan = TasksFile::parse_str(&code).expect("a plan");
        let policy = crate::plan_policy::PlanExecutionPolicy::generated(8);
        let repairs = repair_generated_draft(&mut plan, policy, workdir, &[]);
        assert!(repairs.is_empty(), "{repairs:?}");
        assert_eq!(plan.tasks[0].max_loc, Some(300));
    }

    /// bug-27f00b: asked to "only write the plan", a live planner made its
    /// tasks write the plan's own `plan.md`. Such a draft is sent back, with
    /// the task named, and the plan whose tasks write the project's files is
    /// the one written.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_task_that_writes_its_own_plan_is_sent_back() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let own = WIDGET_PLAN
            .replace(
                "files = [\"src/widget.rs\"]",
                "files = [\"plans/widget/plan.md\"]",
            )
            .replace("test -f src/widget.rs", "test -f plans/widget/plan.md");
        assert_ne!(own, WIDGET_PLAN);
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&own, WIDGET_PLAN],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        generate_plan(request)
            .await
            .expect("generate the widget plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 2, "the plan-writing draft, then one");
        let again = std::fs::read_to_string(bin.path().join("prompt-1.txt")).expect("the retry");
        assert!(
            again.contains("- T1 writes `plans/widget/plan.md`"),
            "{again}"
        );
        let written =
            TasksFile::parse(&workdir.join("plans/widget/tasks.toml")).expect("the widget plan");
        assert_eq!(written.tasks[0].files, ["src/widget.rs"]);
    }

    /// bug-df9aff: G1's plan, written by a planner, comes out with every
    /// task able to create its module and the render task after the three
    /// whose modules its tests import, with no second call.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generated_g1_tasks_may_write_and_wait_for_their_imports() {
        let (temp, plan) = super::super::runnable::tests::g1_plan();
        let workdir = temp.path();
        let toml = toml::to_string_pretty(&plan).expect("the G1 plan as TOML");
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&toml],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "g1", workdir)
        };
        generate_plan(request).await.expect("generate the G1 plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "roko repairs the plan itself");
        let written = TasksFile::parse(&workdir.join("plans/g1/tasks.toml")).expect("the plan");
        // bug-d758f6: each implementer keeps its role's own tools, write_file
        // among them, rather than the list it was written with.
        for task in &written.tasks {
            assert!(
                task.allowed_tools
                    .as_ref()
                    .is_none_or(|tools| tools.iter().any(|tool| tool == "write_file")),
                "{}: {:?}",
                task.id,
                task.allowed_tools
            );
        }
        let render = written
            .tasks
            .iter()
            .find(|task| task.id == "T4")
            .expect("the render task");
        assert_eq!(render.depends_on, ["T1", "T2", "T3"]);
    }

    /// bug-8fe462: a spec that states checks as unittest wildcards gets
    /// their real tests. The planner is given the command for each, and a
    /// step it wrote with invented test names runs the real ones instead,
    /// without asking it again.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn wildcard_spec_checks_become_runnable_steps() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        std::fs::create_dir_all(workdir.join("tests")).expect("tests");
        for (file, body) in [
            (
                "tests/test_osc.py",
                "class NoiseTest(unittest.TestCase):\n    def test_white_noise_is_centred(self):\n",
            ),
            (
                "tests/test_svf.py",
                "class SVFTest(unittest.TestCase):\n    \
                 def test_lowpass_attenuates_highs(self):\n        pass\n    \
                 def test_lowpass_passes_lows(self):\n        pass\n",
            ),
        ] {
            std::fs::write(workdir.join(file), format!("import unittest\n\n{body}"))
                .expect("a test module");
        }
        let source = PlanSource::Text {
            text: "# Synth\n\n## Verification\n\n| INV | Test | Constraint |\n|---|---|---|\n\
                   | INV-4 | `tests.test_osc.NoiseTest.*` | white noise is centred |\n\
                   | INV-5 | `tests.test_svf.SVFTest.test_lowpass_*` | LP 400 Hz |\n",
            kind: "file",
        };
        let invented = "[meta]\nplan = \"synth\"\ntotal = 1\ndone = 0\nstatus = \"ready\"\n\n\
            [[task]]\nid = \"T1\"\ntitle = \"Add the noise and the filter\"\n\
            description = \"Create synth/osc.py and synth/svf.py.\"\nstatus = \"ready\"\n\
            role = \"implementer\"\ntier = \"focused\"\n\
            files = [\"synth/osc.py\", \"synth/svf.py\"]\ndepends_on = []\n\n\
            [task.context]\nread_files = []\n\n\
            [[task.verify]]\nphase = \"test\"\n\
            command = \"python3 -m unittest tests.test_osc.NoiseTest.test_white_noise\"\n\
            covers = [\"SC1\"]\n\n\
            [[task.verify]]\nphase = \"test\"\n\
            command = \"python3 -m unittest tests.test_svf.SVFTest.test_lowpass_400hz\"\n\
            covers = [\"SC2\"]\n";
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[invented],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(source, "synth", workdir)
        };
        let (_, outcome) = generate_plan(request)
            .await
            .expect("generate the synth plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "roko fixes the steps itself");
        let asked = std::fs::read_to_string(bin.path().join("prompt-0.txt")).expect("first call");
        assert!(
            asked.contains("(run: `python3 -m unittest tests.test_osc.NoiseTest`)"),
            "{asked}"
        );

        let written =
            TasksFile::parse(&workdir.join("plans/synth/tasks.toml")).expect("the synth plan");
        let commands: Vec<&str> = written.tasks[0]
            .verify
            .iter()
            .map(|step| step.command.as_str())
            .collect();
        assert_eq!(
            commands,
            [
                "python3 -m unittest tests.test_osc.NoiseTest",
                "python3 -m unittest tests.test_svf.SVFTest.test_lowpass_attenuates_highs \
                 tests.test_svf.SVFTest.test_lowpass_passes_lows",
            ]
        );
        let checks = outcome
            .source_checks
            .expect("the source's checks are reported");
        assert_eq!(
            (checks.kept, checks.cannot_run, checks.total),
            (2, 0, 2),
            "{checks:?}"
        );
    }

    /// gap-dc5d7c: generation from a file with a Verification section lists
    /// its checks for the planner. A plan that checks one only with a file
    /// test, leaves one out and drops one with a reason that restates it is
    /// sent back once; the plan that keeps every check is written, and each
    /// check is reported: kept, by the task and the criterion that keep it
    /// (a step that covers a check by its id gets a criterion that states
    /// it), or dropped, with the planner's reason.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn from_file_generation_reports_dropped_source_checks() {
        let source = PlanSource::Text {
            text: "# Widget\n\nAdd a widget module.\n\n## Verification\n\n\
                   - `src/widget.rs` exists\n- the widget spins\n- a person enjoys the widget\n",
            kind: "file",
        };
        let plan = |acceptance: &str, verify: &str, dropped: &str| {
            format!(
                "[meta]\nplan = \"widget\"\ntotal = 1\ndone = 0\nstatus = \"ready\"\n{dropped}\n\
                 [[task]]\nid = \"T1\"\ntitle = \"Add the widget module\"\n\
                 description = \"Create src/widget.rs.\"\nstatus = \"ready\"\n\
                 role = \"implementer\"\ntier = \"focused\"\nfiles = [\"src/widget.rs\"]\n\
                 depends_on = []\nacceptance = [{acceptance}]\n\n\
                 [task.context]\nread_files = []\n{verify}"
            )
        };
        let step = |command: &str, covers: &str| {
            format!(
                "\n[[task.verify]]\nphase = \"test\"\ncommand = \"{command}\"\n\
                 covers = [\"{covers}\"]\n"
            )
        };
        let first = plan(
            "\"AC1: SC1: src/widget.rs exists\"",
            &step("test -f src/widget.rs", "AC1"),
            "dropped_checks = [\"SC3: a person enjoys the widget\"]",
        );
        let second = plan(
            "\"AC1: SC2: the widget spins\"",
            &format!(
                "{}{}",
                step(
                    "python3 -m unittest tests.test_widget.WidgetTest.test_module",
                    "SC1"
                ),
                step(
                    "python3 -m unittest tests.test_widget.WidgetTest.test_spins",
                    "AC1"
                ),
            ),
            "dropped_checks = [\"SC3: needs a person to judge it\"]",
        );
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&first, &second],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(source, "widget", workdir)
        };
        let (_, outcome) = generate_plan(request)
            .await
            .expect("generate the widget plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(
            log.lines().count(),
            2,
            "the plan, then the plan that keeps every check"
        );
        let asked = std::fs::read_to_string(bin.path().join("prompt-0.txt")).expect("first call");
        assert!(asked.contains("## Source checks"), "{asked}");
        assert!(
            asked.contains("- SC3: a person enjoys the widget"),
            "{asked}"
        );
        let again = std::fs::read_to_string(bin.path().join("prompt-1.txt")).expect("the retry");
        for expected in [
            "keeps 0 of the 3 source checks",
            "- SC2: the widget spins",
            "- SC3: a person enjoys the widget",
            "(T1 runs only `test -f src/widget.rs`)",
        ] {
            assert!(again.contains(expected), "{expected}: {again}");
        }

        let checks = outcome
            .source_checks
            .expect("the source's checks are reported");
        assert_eq!(
            (
                checks.total,
                checks.kept,
                checks.shallow,
                checks.dropped,
                checks.missing
            ),
            (3, 2, 0, 1, 0),
            "{checks:?}"
        );
        assert!(checks.regenerated);
        let criterion = |index: usize| {
            let check = &checks.checks[index];
            (
                check.status.as_str(),
                check.task.as_deref(),
                check.criterion.as_deref(),
            )
        };
        assert_eq!(criterion(0), ("kept", Some("T1"), Some("AC2")));
        assert_eq!(criterion(1), ("kept", Some("T1"), Some("AC1")));
        let person = &checks.checks[2];
        assert_eq!(person.status, "dropped");
        assert_eq!(person.reason.as_deref(), Some("needs a person to judge it"));
        let written = std::fs::read_to_string(workdir.join("plans/widget/tasks.toml"))
            .expect("the widget plan");
        for expected in [
            "AC2: SC1: `src/widget.rs` exists",
            "covers = [\"AC2\"]",
            "SC3: needs a person to judge it",
        ] {
            assert!(written.contains(expected), "{expected}: {written}");
        }
    }

    /// bug-f0d60c: a reply that holds no plan that validates is retried with
    /// the request, its source document included, the whole reply and its
    /// error, and no one-task template; the rejected reply is kept with its
    /// error.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn extraction_retry_keeps_the_source_document() {
        // A source with no check headings, so no source check asks for a
        // third call (gap-dc5d7c).
        let source = PlanSource::Text {
            text: "# Widget\n\nAdd a widget module: `widget::spin(3)` returns three turns.\n",
            kind: "file",
        };
        // Longer than the 2,000 characters a retry used to quote, with a key
        // given twice.
        let draft = format!(
            "{WIDGET_PLAN}phase = \"check\"\n# {}\n# the end of the draft\n",
            "padding ".repeat(400)
        );
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&draft, WIDGET_PLAN],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(source, "widget", workdir)
        };
        generate_plan(request)
            .await
            .expect("generate the widget plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 2, "the rejected draft, then the retry");
        assert!(workdir.join("plans/widget/tasks.toml").is_file());

        let first = std::fs::read_to_string(bin.path().join("prompt-0.txt")).expect("first call");
        let retry = std::fs::read_to_string(bin.path().join("prompt-1.txt")).expect("retry");
        for expected in [
            "`widget::spin(3)` returns three turns",
            "# the end of the draft",
            "generated plan has invalid TOML",
        ] {
            assert!(retry.contains(expected), "{expected}: {retry}");
        }
        assert!(!retry.contains("MINIMUM REQUIRED STRUCTURE"), "{retry}");
        // The system prompt's example names the file; the retry adds no
        // mention of its own.
        let named = |text: &str| text.matches("crates/roko-core/src/lib.rs").count();
        assert_eq!(named(&retry), named(&first));

        let drafts = workdir.join(".roko/plan-drafts/widget");
        let kept = std::fs::read_to_string(drafts.join("rejected-1.md")).expect("the draft");
        assert!(kept.contains("generated plan has invalid TOML"), "{kept}");
        assert!(kept.contains("# the end of the draft"), "{kept}");
        assert!(!drafts.join("rejected-2.md").exists());
    }

    /// bug-59ed90: a plan whose check pipes its test runner's output away is
    /// sent back to the planner once, naming the check and the runner. The
    /// rewritten plan is kept when it hides less; a planner that keeps the
    /// pipe is not asked again, and its plan is written as it is.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generation_asks_once_more_when_a_check_hides_its_runner_output() {
        let plain = WIDGET_PLAN.replace(
            "\"test -f src/widget.rs\"",
            "\"python3 -m unittest tests.test_widget\"",
        );
        let hidden = WIDGET_PLAN.replace(
            "\"test -f src/widget.rs\"",
            "\"python3 -m unittest tests.test_widget 2>&1 | grep -q OK\"",
        );
        assert_ne!(plain, hidden);

        for (replies, kept) in [
            (vec![hidden.as_str(), plain.as_str()], plain.as_str()),
            (vec![hidden.as_str()], hidden.as_str()),
        ] {
            let temp = tempfile::tempdir().expect("tempdir");
            let workdir = temp.path();
            let bin = tempfile::tempdir().expect("tempdir");
            let calls = write_scripted_planner(
                workdir,
                bin.path(),
                &replies,
                "allow_threshold = 0.0\nblock_threshold = 0.0\n",
            );
            let request = PlanRequest {
                model: Some("planner"),
                ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
            };
            generate_plan(request)
                .await
                .expect("generate the widget plan");
            let written = std::fs::read_to_string(workdir.join("plans/widget/tasks.toml"))
                .expect("the widget plan");
            let pipes = written.contains("| grep -q OK");
            assert_eq!(pipes, kept.contains("| grep -q OK"), "{written}");
            let log = std::fs::read_to_string(&calls).expect("planner call log");
            assert_eq!(log.lines().count(), 2, "the plan, then one rewrite");
            let ask = std::fs::read_to_string(bin.path().join("prompt-1.txt")).expect("ask");
            assert!(ask.contains("hide their test runner's output"), "{ask}");
            assert!(
                ask.contains(
                    "task 'T1' verify step 1 `python3 -m unittest tests.test_widget 2>&1 | grep \
                     -q OK` pipes the output of `python3 -m unittest tests.test_widget` into \
                     `grep`"
                ),
                "{ask}"
            );
        }
    }

    /// bug-7a08a8: an unattended plan's planner is told nobody answers its
    /// questions, and a question it answered itself goes into its task
    /// instead of keeping the plan from running; an unanswered one stays.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_unattended_plan_decides_its_answered_questions() {
        let answered = WIDGET_PLAN.replace(
            "depends_on = []\n",
            "depends_on = []\nopen_questions = [\"Should the widget module be public? \
             Assumption: yes, it is declared pub.\"]\n",
        );
        assert_ne!(answered, WIDGET_PLAN);

        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        write_scripted_planner(
            workdir,
            bin.path(),
            &[&answered],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            unattended: true,
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        generate_plan(request)
            .await
            .expect("an answered question no longer keeps the plan from running");
        let written = std::fs::read_to_string(workdir.join("plans/widget/tasks.toml"))
            .expect("the widget plan");
        assert!(!written.contains("open_questions"), "{written}");
        assert!(
            written.contains("Decided while planning, with nobody to ask:"),
            "{written}"
        );
        assert!(
            written.contains("Should the widget module be public? Assumption: yes"),
            "{written}"
        );
        let prompt = std::fs::read_to_string(bin.path().join("prompt-0.txt")).expect("prompt");
        assert!(
            prompt.contains("Nobody will review this plan or answer questions"),
            "{prompt}"
        );

        let mixed = answered.replace(
            "open_questions = [",
            "open_questions = [\"Which colour should it be?\", ",
        );
        let (folded, moved) = fold_answered_questions(&mixed).expect("fold the questions");
        assert_eq!(moved.len(), 1, "{moved:?}");
        assert!(
            moved[0].starts_with("task T1: Should the widget"),
            "{moved:?}"
        );
        let tasks = TasksFile::parse_str(&folded).expect("the folded plan parses");
        assert_eq!(
            tasks.tasks[0].spec.open_questions,
            ["Which colour should it be?"]
        );
    }

    /// bug-7a08a8: a written plan that cannot run ends the generation with an
    /// error that names why (an open question nobody answered, PLAN_045), so
    /// no caller reports it as written and ready. The plan stays on disk.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generation_fails_when_the_written_plan_cannot_run() {
        let open = WIDGET_PLAN.replace(
            "depends_on = []\n",
            "depends_on = []\nopen_questions = [\"Which colour should the widget be?\"]\n",
        );
        for unattended in [false, true] {
            let temp = tempfile::tempdir().expect("tempdir");
            let workdir = temp.path();
            let bin = tempfile::tempdir().expect("tempdir");
            write_scripted_planner(
                workdir,
                bin.path(),
                &[&open],
                "allow_threshold = 0.0\nblock_threshold = 0.0\n",
            );
            let request = PlanRequest {
                model: Some("planner"),
                unattended,
                ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
            };
            let error = generate_plan(request)
                .await
                .expect_err("an open question keeps the plan from running");
            let failure = error
                .downcast_ref::<PlanFailsValidation>()
                .unwrap_or_else(|| panic!("a validation failure: {error:#}"));
            assert_eq!(failure.plan_dir, Path::new("plans/widget"), "{failure}");
            assert!(
                failure
                    .errors
                    .iter()
                    .any(|line| line.starts_with("PLAN_045 task 'T1' has open questions")),
                "{failure}"
            );
            let message = failure.to_string();
            assert!(
                message.starts_with(
                    "the plan was written to plans/widget, but it cannot run: 1 error(s)"
                ),
                "{message}"
            );
            assert!(
                message.contains("Which colour should the widget be?"),
                "{message}"
            );
            assert!(workdir.join("plans/widget/tasks.toml").exists());
        }
    }

    /// bug-a0432b: a chat host's plan is written only by a model roko's own
    /// tool loop runs. In a workspace whose models all run on CLI agents it is
    /// refused with the reason a chat host's run gets, and the CLI planner is
    /// never called.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_chat_host_plan_is_never_written_on_a_cli_agent() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_fake_planner(workdir, bin.path(), WIDGET_PLAN);
        // No built-in ladder rung may qualify through a key in the
        // environment, so the test never reaches a real provider.
        let config = workdir.join("roko.toml");
        let mut toml = std::fs::read_to_string(&config).expect("roko.toml");
        toml.push_str("\n[routing.ladder]\nenabled = false\n");
        std::fs::write(&config, toml).expect("roko.toml");
        let request = PlanRequest {
            for_chat_host: true,
            unattended: true,
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        let error = generate_plan(request)
            .await
            .expect_err("no model roko's own tool loop runs is set up");
        let message = format!("{error:#}");
        assert!(
            message.contains("no model can write the plan for a chat host's request"),
            "{message}"
        );
        assert!(
            message.contains("needs a model that runs in roko's own tool loop"),
            "{message}"
        );
        assert!(!calls.exists(), "the CLI planner was called");
        assert!(!workdir.join("plans/widget").exists());
    }

    /// A sink that keeps every stage a generation reports.
    #[cfg(unix)]
    fn recorded_progress() -> (PlanProgressSink, Arc<std::sync::Mutex<Vec<PlanProgress>>>) {
        let stages = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink: PlanProgressSink = Arc::new({
            let stages = Arc::clone(&stages);
            move |stage: &PlanProgress| {
                stages.lock().expect("stages").push(stage.clone());
            }
        });
        (sink, stages)
    }

    /// gap-11d870: a planner call stopped at roko's time limit is a timeout,
    /// not a crash "for an unknown reason": it is retried once, the
    /// progress says why, and the error names the limit.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_timed_out_planner_call_is_reported_as_a_timeout() {
        use std::os::unix::fs::PermissionsExt as _;

        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_fake_planner(workdir, bin.path(), WIDGET_PLAN);
        // The adapters' words for an agent they stopped at its time limit.
        let planner = bin.path().join("planner.sh");
        std::fs::write(
            &planner,
            format!(
                "#!/bin/sh\ncat >/dev/null\necho call >> '{}'\n\
                 echo 'timed out after 300000 ms' >&2\nexit 1\n",
                calls.display()
            ),
        )
        .expect("timing-out planner");
        std::fs::set_permissions(&planner, std::fs::Permissions::from_mode(0o755))
            .expect("make the script executable");
        let (sink, stages) = recorded_progress();
        let request = PlanRequest {
            model: Some("planner"),
            progress: Some(sink),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        let error = generate_plan(request)
            .await
            .expect_err("every planner call times out");
        let message = format!("{error:#}");
        assert!(
            message.contains("stopped after 2 planner calls")
                && message.contains("roko's 300 s limit for one call"),
            "{message}"
        );
        assert!(!message.contains("unknown reason"), "{message}");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 2, "the call, then one retry");

        let stages = stages.lock().expect("stages");
        let names: Vec<&str> = stages.iter().map(|stage| stage.stage).collect();
        // Each stage is reported again once its call has ended (bug-1602d4).
        assert_eq!(
            names,
            ["planning", "planning", "retrying", "retrying"],
            "{stages:?}"
        );
        let retry = &stages[2];
        assert_eq!((retry.attempt, retry.attempts, retry.calls), (2, 2, 2));
        assert!(
            retry
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("roko's 300 s limit for one call")),
            "{retry:?}"
        );
    }

    /// bug-09cc5f: every planner call runs under the planner's own turn cap,
    /// above the Claude CLI's default of `OperatingFrequency::Theta` turns,
    /// which a planner reading a large repository used up before it wrote
    /// the plan. The first call and the retry after a reply that holds no
    /// plan both start the CLI with it.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn planner_call_gets_a_turn_cap_above_the_default() {
        let default = roko_core::operating_frequency::OperatingFrequency::Theta.turn_limit();
        assert!(
            PLANNER_MAX_TURNS > default,
            "{PLANNER_MAX_TURNS} <= {default}"
        );

        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &["```text\nThe plan needs more reading.\n```\n", WIDGET_PLAN],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        generate_plan(request)
            .await
            .expect("generate the widget plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(
            log.lines().count(),
            2,
            "the reply without a plan, then a retry"
        );
        for call in 0..2 {
            let args = std::fs::read_to_string(bin.path().join(format!("prompt-{call}.txt")))
                .expect("the call's arguments");
            let cap = args
                .lines()
                .skip_while(|line| *line != "--max-turns")
                .nth(1);
            assert_eq!(
                cap,
                Some(PLANNER_MAX_TURNS.to_string().as_str()),
                "call {call}: {args}"
            );
        }
    }

    /// bug-09cc5f: a planner call that ran out of turns says so, where it
    /// read "Agent crashed for an unknown reason", and is not asked again:
    /// the same prompt under the same cap would most likely stop there too.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_planner_out_of_turns_is_not_asked_again() {
        use std::os::unix::fs::PermissionsExt as _;

        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_fake_planner(workdir, bin.path(), WIDGET_PLAN);
        // What the Claude CLI prints when it stops at `--max-turns`.
        let planner = bin.path().join("planner.sh");
        std::fs::write(
            &planner,
            format!(
                "#!/bin/sh\ncat >/dev/null\necho call >> '{}'\n\
                 echo '{{\"type\":\"result\",\"subtype\":\"error_max_turns\",\
                 \"is_error\":true,\"num_turns\":{}}}'\nexit 1\n",
                calls.display(),
                PLANNER_MAX_TURNS + 1
            ),
        )
        .expect("planner out of turns");
        std::fs::set_permissions(&planner, std::fs::Permissions::from_mode(0o755))
            .expect("make the script executable");
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        let error = generate_plan(request)
            .await
            .expect_err("the planner runs out of turns");
        let message = format!("{error:#}");
        assert!(
            message.contains("the planner ran out of turns before it wrote the plan"),
            "{message}"
        );
        assert!(
            message.contains(&format!(
                "agent turn cap reached (turns={}, cap={PLANNER_MAX_TURNS})",
                PLANNER_MAX_TURNS + 1
            )),
            "{message}"
        );
        assert!(!message.contains("unknown reason"), "{message}");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 1, "the call, and no retry");
        assert!(!workdir.join("plans/widget").exists());
    }

    /// gap-11d870: a plan whose reply holds no plan.md block gets an
    /// overview written from its tasks: the request, the tasks wave by wave
    /// and their checks, where it used to say only "Generated plan.". The
    /// generation reports each stage as it moves on.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_plan_without_a_plan_md_block_gets_an_overview() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        write_fake_planner(workdir, bin.path(), WIDGET_PLAN);
        let (sink, stages) = recorded_progress();
        let request = PlanRequest {
            model: Some("planner"),
            progress: Some(sink),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        generate_plan(request)
            .await
            .expect("generate the widget plan");

        let plan_md =
            std::fs::read_to_string(workdir.join("plans/widget/plan.md")).expect("plan.md");
        assert!(!plan_md.contains("Generated plan."), "{plan_md}");
        for expected in [
            "---\nplan: widget\ntitle: Widget\n---\n",
            "> Add a widget module.",
            "### Wave 1",
            "- **T1: Add the widget module** (implementer, focused)",
            "  - Goal: Create src/widget.rs.",
            "  - Check (check): `test -f src/widget.rs`",
        ] {
            assert!(
                plan_md.contains(expected),
                "{expected:?} not in:\n{plan_md}"
            );
        }

        let stages = stages.lock().expect("stages");
        let names: Vec<&str> = stages.iter().map(|stage| stage.stage).collect();
        // `planning` is reported again once its call has ended (bug-1602d4).
        assert_eq!(
            names,
            ["planning", "planning", "writing", "recording"],
            "{stages:?}"
        );
        assert_eq!(stages[0].calls, 1, "{stages:?}");
        assert_eq!(stages[0].model, "planner", "{stages:?}");
        assert_eq!(stages[3].calls, 1, "{stages:?}");
    }

    /// gap-61c95c: each progress report says how long the generation has
    /// run and where the time went: the lift before the first planner
    /// call, the planner calls, the repairs between them and the written
    /// plan's validation, each null until it has run, together never more
    /// than the time run. The last report has them all.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn plan_progress_reports_its_elapsed_time_and_phases() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        write_fake_planner(workdir, bin.path(), WIDGET_PLAN);
        // The planner takes a moment to answer.
        let planner = bin.path().join("planner.sh");
        let script = std::fs::read_to_string(&planner).expect("planner script");
        std::fs::write(
            &planner,
            script.replacen("set -eu\n", "set -eu\nsleep 0.4\n", 1),
        )
        .expect("slow the planner down");
        let (sink, stages) = recorded_progress();
        let request = PlanRequest {
            model: Some("planner"),
            progress: Some(sink),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        generate_plan(request)
            .await
            .expect("generate the widget plan");

        let stages = stages.lock().expect("stages");
        let names: Vec<&str> = stages.iter().map(|stage| stage.stage).collect();
        assert_eq!(
            names,
            ["planning", "planning", "writing", "recording"],
            "{stages:?}"
        );
        for pair in stages.windows(2) {
            assert!(pair[0].elapsed_ms <= pair[1].elapsed_ms, "{stages:?}");
        }
        for stage in stages.iter() {
            let phases = stage.phases;
            let spent: u64 = [
                phases.lift_ms,
                phases.planner_ms,
                phases.repair_ms,
                phases.validate_ms,
            ]
            .into_iter()
            .flatten()
            .sum();
            assert!(spent <= stage.elapsed_ms, "{stage:?}");
        }
        let before_the_call = stages[0].phases;
        assert!(before_the_call.lift_ms.is_some(), "{stages:?}");
        assert_eq!(before_the_call.planner_ms, None, "{stages:?}");
        assert_eq!(before_the_call.validate_ms, None, "{stages:?}");
        assert!(
            stages[1].phases.planner_ms.is_some_and(|ms| ms >= 400),
            "{stages:?}"
        );
        assert_eq!(stages[2].phases.validate_ms, None, "{stages:?}");
        let last = stages[3].phases;
        assert!(last.planner_ms.is_some_and(|ms| ms >= 400), "{last:?}");
        assert!(
            last.lift_ms.is_some() && last.repair_ms.is_some() && last.validate_ms.is_some(),
            "{last:?}"
        );
        let json = serde_json::to_value(&stages[3]).expect("progress json");
        assert!(json["elapsed_ms"].is_u64(), "{json}");
        assert!(json["phases"]["planner_ms"].is_u64(), "{json}");
        let first = serde_json::to_value(&stages[0]).expect("progress json");
        assert!(first["phases"]["planner_ms"].is_null(), "{first}");
    }

    /// bug-1602d4: a planner call's spend is reported as soon as the call
    /// ends, as the same stage with the call counted, not first when the
    /// plan is being written, a moment before the generation completes.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn plan_writing_reports_its_spend_as_each_call_ends() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        write_fake_planner(workdir, bin.path(), WIDGET_PLAN);
        let (sink, stages) = recorded_progress();
        let request = PlanRequest {
            model: Some("planner"),
            progress: Some(sink),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        let (_, outcome) = generate_plan(request)
            .await
            .expect("generate the widget plan");
        assert!(outcome.cost_usd > 0.0, "{outcome:?}");

        let stages = stages.lock().expect("stages");
        let (started, ended) = (&stages[0], &stages[1]);
        assert_eq!(
            (started.stage, started.calls),
            ("planning", 1),
            "{stages:?}"
        );
        assert!(started.cost_usd.abs() < 1e-12, "{stages:?}");
        assert_eq!((ended.stage, ended.calls), ("planning", 1), "{stages:?}");
        assert!(
            (ended.cost_usd - outcome.cost_usd).abs() < 1e-9,
            "{stages:?} vs {outcome:?}"
        );
        assert_eq!(stages[2].stage, "writing", "{stages:?}");
    }

    /// 3221: the planner can write its acceptance tests into the plan. A
    /// reply with a tasks.toml block and an `accept:accept/test_slug.py`
    /// block writes both; a `[task.accept]` src that the reply does not
    /// emit sends the plan back to the planner, which emits it next time.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generation_writes_planner_accept_tests_into_the_plan() {
        const ACCEPT_PLAN: &str = "\
            [meta]\nplan = \"widget\"\ntotal = 1\ndone = 0\nstatus = \"ready\"\n\n\
            [[task]]\nid = \"T1\"\ntitle = \"Add the slug helper\"\n\
            description = \"Create src/slug.py with slugify().\"\nstatus = \"ready\"\n\
            role = \"implementer\"\ntier = \"focused\"\nfiles = [\"src/slug.py\"]\n\
            depends_on = []\n\n[task.context]\nread_files = []\n\n\
            [task.accept]\nfiles = [{ src = \"accept/test_slug.py\", \
            dest = \"tests/test_slug.py\", runner = \"python3 -m unittest tests.test_slug\", \
            count = 1 }]\n\n\
            [[task.verify]]\nphase = \"check\"\ncommand = \"test -f src/slug.py\"\n";
        const TEST: &str = "import unittest\n\n\
            class Slug(unittest.TestCase):\n    def test_slug(self):\n        \
            from src.slug import slugify\n        \
            self.assertEqual(slugify(\"A b\"), \"a-b\")\n";
        let with_test =
            format!("```toml\n{ACCEPT_PLAN}```\n\n```accept:accept/test_slug.py\n{TEST}```\n");

        for replies in [
            vec![with_test.as_str()],
            vec![ACCEPT_PLAN, with_test.as_str()],
        ] {
            let temp = tempfile::tempdir().expect("tempdir");
            let workdir = temp.path();
            let bin = tempfile::tempdir().expect("tempdir");
            let calls = write_scripted_planner(
                workdir,
                bin.path(),
                &replies,
                "allow_threshold = 0.0\nblock_threshold = 0.0\n",
            );
            let request = PlanRequest {
                model: Some("planner"),
                ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
            };
            generate_plan(request)
                .await
                .expect("generate the widget plan");
            let plan_dir = workdir.join("plans/widget");
            let tasks = std::fs::read_to_string(plan_dir.join("tasks.toml")).expect("tasks.toml");
            assert!(tasks.contains("accept/test_slug.py"), "{tasks}");
            let test = std::fs::read_to_string(plan_dir.join("accept/test_slug.py"))
                .expect("the accept test beside tasks.toml");
            assert_eq!(test, TEST);
            let log = std::fs::read_to_string(&calls).expect("planner call log");
            assert_eq!(log.lines().count(), replies.len(), "{replies:?}");
            if replies.len() == 2 {
                let retry = std::fs::read_to_string(bin.path().join("prompt-1.txt"))
                    .expect("the retry prompt");
                assert!(
                    retry.contains("src `accept/test_slug.py` is missing"),
                    "{retry}"
                );
            }
        }
    }

    /// bug-a5cd6b: generation writes the plan it was asked for and no other.
    /// An old-format plan and a generated plan, which names no model, keep
    /// their tasks.toml byte for byte, and the planner is called once.
    /// 3225 (decision 3203): a generated task over the size limits of every
    /// tier fails validation. A mechanical task with nine files, more than a
    /// generated task may own, goes back to the planner with the limit it
    /// exceeds, and the split plan is written; one a higher tier fits is
    /// raised instead (gap-4dd77d).
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generation_rejects_a_task_over_its_tier_size() {
        const OVERSIZED: &str = "\
            [meta]\nplan = \"widget\"\ntotal = 1\ndone = 0\nstatus = \"ready\"\n\n\
            [[task]]\nid = \"T1\"\ntitle = \"Add the widget modules\"\n\
            description = \"Create the nine widget modules.\"\nstatus = \"ready\"\n\
            role = \"implementer\"\ntier = \"mechanical\"\nfiles = [\"src/a.rs\", \
            \"src/b.rs\", \"src/c.rs\", \"src/d.rs\", \"src/e.rs\", \"src/f.rs\", \
            \"src/g.rs\", \"src/h.rs\", \"src/i.rs\"]\n\
            depends_on = []\n\n[task.context]\nread_files = []\n\n\
            [[task.verify]]\nphase = \"check\"\ncommand = \"test -f src/a.rs\"\n";
        const SPLIT: &str = "\
            [meta]\nplan = \"widget\"\ntotal = 2\ndone = 0\nstatus = \"ready\"\n\n\
            [[task]]\nid = \"T1\"\ntitle = \"Add the first widget modules\"\n\
            description = \"Create src/a.rs, src/b.rs and src/c.rs.\"\nstatus = \"ready\"\n\
            role = \"implementer\"\ntier = \"mechanical\"\n\
            files = [\"src/a.rs\", \"src/b.rs\", \"src/c.rs\"]\n\
            depends_on = []\n\n[task.context]\nread_files = []\n\n\
            [[task.verify]]\nphase = \"check\"\n\
            command = \"test -f src/a.rs && test -f src/b.rs && test -f src/c.rs\"\n\n\
            [[task]]\nid = \"T2\"\ntitle = \"Add the other widget modules\"\n\
            description = \"Create src/d.rs, src/e.rs and src/f.rs.\"\nstatus = \"ready\"\n\
            role = \"implementer\"\ntier = \"mechanical\"\n\
            files = [\"src/d.rs\", \"src/e.rs\", \"src/f.rs\"]\n\
            depends_on = []\n\n[task.context]\nread_files = []\n\n\
            [[task.verify]]\nphase = \"check\"\n\
            command = \"test -f src/d.rs && test -f src/e.rs && test -f src/f.rs\"\n";

        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[OVERSIZED, SPLIT],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        generate_plan(request)
            .await
            .expect("generate the widget plan");
        let written = std::fs::read_to_string(workdir.join("plans/widget/tasks.toml"))
            .expect("the widget plan");
        let plan = TasksFile::parse_str(&written).expect("parse the written plan");
        assert_eq!(plan.tasks.len(), 2, "{written}");
        assert!(
            plan.tasks.iter().all(|task| task.files.len() == 3),
            "{written}"
        );
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(
            log.lines().count(),
            2,
            "the oversized plan, then the split one"
        );
        let retry = std::fs::read_to_string(bin.path().join("prompt-1.txt")).expect("retry");
        assert!(
            retry.contains("PLAN_TIER_SIZE [T1]: mechanical task is over its tier's size limits"),
            "{retry}"
        );
        assert!(retry.contains("split it into smaller tasks"), "{retry}");
        assert!(!retry.contains("raise its tier to"), "{retry}");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generation_does_not_regenerate_other_plans() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();

        // Two plans are already in plans/: one in the old format, and one as
        // the generator writes it, without a model_hint. Each has a plan.md a
        // regeneration could start from.
        let plans = workdir.join("plans");
        let old_toml = "[meta]\nplan = \"old\"\ntotal = 1\nstatus = \"ready\"\n\n\
                        [[task]]\nid = \"T1\"\ntitle = \"An old task\"\nstatus = \"ready\"\n";
        let hintless_toml = "[meta]\nplan = \"hintless\"\ntotal = 1\nstatus = \"ready\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"A generated task\"\nstatus = \"ready\"\n\
             role = \"implementer\"\ntier = \"focused\"\nfiles = [\"src/hintless.rs\"]\n\
             depends_on = []\n\n[task.context]\nread_files = []\n\n\
             [[task.verify]]\nphase = \"check\"\ncommand = \"test -f src/hintless.rs\"\n";
        for (name, tasks) in [("old", old_toml), ("hintless", hintless_toml)] {
            let dir = plans.join(name);
            std::fs::create_dir_all(&dir).expect("plan directory");
            std::fs::write(dir.join("tasks.toml"), tasks).expect("tasks.toml");
            std::fs::write(
                dir.join("plan.md"),
                format!("---\nplan: {name}\n---\n# {name}\n"),
            )
            .expect("plan.md");
        }

        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_fake_planner(workdir, bin.path(), WIDGET_PLAN);

        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        generate_plan(request)
            .await
            .expect("generation writes the widget plan");

        let widget = std::fs::read_to_string(plans.join("widget").join("tasks.toml"))
            .expect("the widget plan");
        let widget = TasksFile::parse_str(&widget).expect("parse the widget plan");
        assert_eq!(widget.tasks.len(), 1);
        for (name, tasks) in [("old", old_toml), ("hintless", hintless_toml)] {
            let after =
                std::fs::read_to_string(plans.join(name).join("tasks.toml")).expect("tasks.toml");
            assert_eq!(after, tasks, "plans/{name} was rewritten");
        }
        let calls = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(
            calls.lines().count(),
            1,
            "one planner call: the widget plan"
        );
    }

    /// bug-2d06bf: a broken plan beside the generated one does not fail the
    /// generation: generate_plan validates only the plan it wrote.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generate_plan_validates_only_the_plan_it_wrote() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let broken = workdir.join("plans").join("broken");
        std::fs::create_dir_all(&broken).expect("plan directory");
        std::fs::write(broken.join("tasks.toml"), "[meta\nplan = ").expect("tasks.toml");
        let bin = tempfile::tempdir().expect("tempdir");
        write_fake_planner(workdir, bin.path(), WIDGET_PLAN);

        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(WIDGET_REQUEST, "widget", workdir)
        };
        let (_, outcome) = generate_plan(request)
            .await
            .expect("generate the widget plan");

        let report = outcome.validation_report.expect("validation report");
        assert_eq!(report["totals"]["plans_checked"], 1, "{report}");
        // The report lists only plans with findings: never the broken sibling.
        let plans = report["plans"].as_array().expect("plans");
        assert!(
            plans.iter().all(|plan| plan["plan_id"] == "widget"),
            "{report}"
        );
        assert!(!report.to_string().contains("PLAN_001"), "{report}");
    }

    #[test]
    fn augment_generator_system_prompt_includes_failure_context() {
        let prompt = augment_generator_system_prompt(
            "base prompt".to_string(),
            Some("task_id = \"demo\"\nreason = \"gate failure\""),
        );
        assert!(prompt.starts_with("base prompt"));
        assert!(prompt.contains("## Failure context for replanning"));
        assert!(prompt.contains("task_id = \"demo\""));
        assert!(prompt.contains("gate failure"));
        assert!(prompt.contains("Do not reproduce the same task shape."));
    }

    fn empty_models() -> IndexMap<String, roko_core::config::schema::ModelProfile> {
        IndexMap::new()
    }

    fn sample_models() -> IndexMap<String, roko_core::config::schema::ModelProfile> {
        // Build from TOML to avoid enumerating every ModelProfile field.
        let toml_str = r#"
[models.claude-sonnet-4-6]
provider = "anthropic"
slug = "claude-sonnet-4-6"

[models.claude-haiku-4-5]
provider = "anthropic"
slug = "claude-haiku-4-5"
"#;
        let cfg: roko_core::config::schema::RokoConfig =
            roko_core::config::schema::RokoConfig::from_toml(toml_str).unwrap();
        cfg.models
    }

    #[test]
    fn validate_valid_plan_passes() {
        let toml = r#"
[meta]
plan = "my-plan"
total = 2
status = "pending"

[[task]]
id = "T1"
title = "First task"
status = "pending"
role = "implementer"
tier = "focused"

[[task.verify]]
phase = "test"
command = "cargo test"

[[task]]
id = "T2"
title = "Second task"
status = "pending"
role = "implementer"
tier = "mechanical"
depends_on = ["T1"]
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "my-plan",
            &empty_models(),
            None,
            &LadderConfig::default(),
        );
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
    }

    #[test]
    fn validate_fixes_truncated_slug() {
        let toml = r#"
[meta]
plan = "my-pl"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Do thing"
status = "pending"
role = "implementer"
tier = "focused"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "my-plan",
            &empty_models(),
            None,
            &LadderConfig::default(),
        )
        .unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert_eq!(
            parsed["meta"]["plan"].as_str().unwrap(),
            "my-plan",
            "slug should be corrected"
        );
    }

    #[test]
    fn validate_fixes_wrong_slug() {
        let toml = r#"
[meta]
plan = "wrong-slug"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Do thing"
status = "pending"
role = "implementer"
tier = "focused"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "correct-slug",
            &empty_models(),
            None,
            &LadderConfig::default(),
        )
        .unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert_eq!(parsed["meta"]["plan"].as_str().unwrap(), "correct-slug");
    }

    #[test]
    fn validate_fixes_verify_field_typo() {
        let toml = r#"
[meta]
plan = "test"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Do thing"
status = "pending"
role = "implementer"
tier = "focused"

[[task.verify]]
pha = "test"
command = "cargo test"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &empty_models(),
            None,
            &LadderConfig::default(),
        )
        .unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        let verify = parsed["task"][0]["verify"][0].as_table().unwrap();
        assert!(
            verify.contains_key("phase"),
            "pha should be corrected to phase"
        );
        assert!(!verify.contains_key("pha"), "pha should be removed");
    }

    #[test]
    fn validate_removes_unknown_model_hint() {
        let toml = r#"
[meta]
plan = "test"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Do thing"
status = "pending"
role = "implementer"
tier = "focused"
model_hint = "gpt-nonexistent"
"#;
        let models = sample_models();
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &models,
            Some("claude-sonnet-4-6"),
            &LadderConfig::default(),
        )
        .unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert!(
            parsed["task"][0].get("model_hint").is_none(),
            "unknown model_hint should be removed so runtime selects"
        );
    }

    #[test]
    fn validate_removes_model_alias_hint() {
        let toml = r#"
[meta]
plan = "test"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Do thing"
status = "pending"
role = "implementer"
tier = "focused"
model_hint = "haiku"
"#;
        let models = sample_models();
        let result =
            validate_and_fix_generated_plan(toml, "test", &models, None, &LadderConfig::default())
                .unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert!(
            parsed["task"][0].get("model_hint").is_none(),
            "model_hint aliases should be removed so runtime selects"
        );
    }

    /// gap-dbf2a6: a generated plan keeps a `rung` hint that names one of
    /// the task's ladder rungs, drops any other, and still loses
    /// `model_hint`.
    #[test]
    fn generated_plan_keeps_its_rung_hint() {
        let toml = r#"
[meta]
plan = "test"
total = 3
status = "pending"

[[task]]
id = "T1"
title = "Rework the parser's error recovery"
status = "pending"
role = "implementer"
tier = "mechanical"
rung = "strong"
model_hint = "claude-opus-4-6"

[[task]]
id = "T2"
title = "Rename a helper"
status = "pending"
role = "implementer"
tier = "mechanical"
rung = "stronk"

[[task]]
id = "T3"
title = "Document the parser"
status = "pending"
role = "scribe"
tier = "focused"
rung = "docs"
"#;
        let mut ladder = LadderConfig::default();
        ladder
            .roles
            .push(roko_core::config::routing::LadderRoleConfig {
                role: "scribe".to_string(),
                rungs: Some(vec![roko_core::config::routing::LadderRung::new(
                    "docs",
                    "gpt-4o-mini",
                )]),
                ..Default::default()
            });
        let result =
            validate_and_fix_generated_plan(toml, "test", &empty_models(), None, &ladder).unwrap();
        let plan = TasksFile::parse_str(&result).expect("the fixed plan parses");
        let rungs: Vec<Option<&str>> = plan
            .tasks
            .iter()
            .map(|task| task.hints.rung.as_deref())
            .collect();
        assert_eq!(rungs, [Some("strong"), None, Some("docs")]);
        assert_eq!(plan.tasks[0].model_hint, None);
    }

    #[test]
    fn validate_rejects_missing_meta() {
        let toml = r#"
[[task]]
id = "T1"
title = "Do thing"
status = "pending"
role = "implementer"
tier = "focused"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &empty_models(),
            None,
            &LadderConfig::default(),
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("[meta] section is missing"), "msg: {msg}");
    }

    #[test]
    fn validate_rejects_missing_task_array() {
        let toml = r#"
[meta]
plan = "test"
total = 0
status = "pending"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &empty_models(),
            None,
            &LadderConfig::default(),
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("[[task]] array is missing"), "msg: {msg}");
    }

    #[test]
    fn validate_rejects_missing_required_task_fields() {
        let toml = r#"
[meta]
plan = "test"
total = 1
status = "pending"

[[task]]
id = "T1"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &empty_models(),
            None,
            &LadderConfig::default(),
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("missing required field 'title'"), "msg: {msg}");
        assert!(
            msg.contains("missing required field 'status'"),
            "msg: {msg}"
        );
        assert!(msg.contains("missing required field 'role'"), "msg: {msg}");
        assert!(msg.contains("missing required field 'tier'"), "msg: {msg}");
    }

    #[test]
    fn validate_rejects_invalid_toml_syntax() {
        let toml = "this is not valid toml {{{}}}";
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &empty_models(),
            None,
            &LadderConfig::default(),
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("invalid TOML"), "msg: {msg}");
    }

    #[test]
    fn validate_fixes_task_field_typo() {
        let toml = r#"
[meta]
plan = "test"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Do thing"
stat = "pending"
role = "implementer"
tier = "focused"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &empty_models(),
            None,
            &LadderConfig::default(),
        )
        .unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        let task = parsed["task"][0].as_table().unwrap();
        assert!(
            task.contains_key("status"),
            "stat should be corrected to status"
        );
        assert!(!task.contains_key("stat"), "stat should be removed");
    }

    #[test]
    fn validate_fixes_placeholder_crate_names() {
        let toml = r#"
[meta]
plan = "btc-funding-alert-cli"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Set up crate structure"
status = "pending"
role = "implementer"
tier = "focused"
files = ["crates/<relevant-lib>/src/lib.rs"]

[[task.verify]]
phase = "build"
command = "cargo check -p <crate>"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "btc-funding-alert-cli",
            &empty_models(),
            None,
            &LadderConfig::default(),
        )
        .unwrap();
        assert!(
            !result.contains("<relevant-lib>"),
            "placeholder <relevant-lib> should be replaced"
        );
        assert!(
            !result.contains("<crate>"),
            "placeholder <crate> should be replaced"
        );
        assert!(
            result.contains("btc-funding-alert-cli"),
            "slug should appear in output"
        );
        // Verify it's still valid TOML.
        let _parsed: toml::Value = toml::from_str(&result).unwrap();
    }

    #[test]
    fn validate_fixes_placeholder_in_verify_command() {
        let toml = r#"
[meta]
plan = "my-cool-tool"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Implement module"
status = "pending"
role = "implementer"
tier = "focused"
files = ["crates/<binary-crate>/src/<module>.rs"]

[[task.verify]]
phase = "build"
command = "cargo check -p <binary-crate>"

[[task.verify]]
phase = "test"
command = "cargo test -p <crate> -- <test_name>"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "my-cool-tool",
            &empty_models(),
            None,
            &LadderConfig::default(),
        )
        .unwrap();
        // <binary-crate> and <crate> replaced with slug.
        assert!(
            !result.contains("<binary-crate>"),
            "placeholder <binary-crate> should be replaced"
        );
        assert!(
            !result.contains("<crate>"),
            "placeholder <crate> should be replaced"
        );
        assert!(
            !result.contains("<module>"),
            "placeholder <module> should be replaced"
        );
        assert!(
            !result.contains("<test_name>"),
            "placeholder <test_name> should be replaced"
        );
        assert!(
            result.contains("cargo check -p my-cool-tool"),
            "verify command should contain slug: {result}"
        );
        assert!(
            result.contains("cargo test -p my-cool-tool"),
            "verify command should contain slug: {result}"
        );
        // Verify it's still valid TOML.
        let _parsed: toml::Value = toml::from_str(&result).unwrap();
    }

    // ---- next_tier_model tests ----

    /// `[models.*]` entries from `(key, slug)` pairs.
    fn models_with(entries: &[(&str, &str)]) -> IndexMap<String, ModelProfile> {
        entries
            .iter()
            .map(|(key, slug)| {
                let profile = ModelProfile {
                    slug: (*slug).to_string(),
                    ..ModelProfile::default()
                };
                ((*key).to_string(), profile)
            })
            .collect()
    }

    /// `[models.*]` entries keyed by their own slug.
    fn models_for(slugs: &[&str]) -> IndexMap<String, ModelProfile> {
        let entries: Vec<(&str, &str)> = slugs.iter().map(|slug| (*slug, *slug)).collect();
        models_with(&entries)
    }

    #[test]
    fn next_tier_model_escalates_with_empty_configured_set() {
        // No configured models = no filtering (backward compat).
        let empty_tier = HashMap::new();
        let no_models = IndexMap::new();
        assert_eq!(
            next_tier_model(Some("claude-haiku-4-5"), &empty_tier, &no_models),
            Some("claude-sonnet-4-6".to_string()),
        );
        assert_eq!(
            next_tier_model(Some("claude-sonnet-4-6"), &empty_tier, &no_models),
            Some("claude-opus-4-6".to_string()),
        );
    }

    #[test]
    fn next_tier_model_skips_unconfigured() {
        let empty_tier = HashMap::new();
        // Only sonnet is configured — opus should be skipped.
        let configured = models_for(&["claude-sonnet-4-6"]);
        assert_eq!(
            next_tier_model(Some("claude-haiku-4-5"), &empty_tier, &configured),
            Some("claude-sonnet-4-6".to_string()),
        );
        // Sonnet tries to escalate to opus, but opus isn't configured → None.
        assert_eq!(
            next_tier_model(Some("claude-sonnet-4-6"), &empty_tier, &configured),
            None,
        );
    }

    #[test]
    fn next_tier_model_skips_to_higher_configured() {
        let empty_tier = HashMap::new();
        // Only opus is configured — should skip sonnet and land on opus.
        let configured = models_for(&["claude-opus-4-6"]);
        assert_eq!(
            next_tier_model(Some("claude-haiku-4-5"), &empty_tier, &configured),
            Some("claude-opus-4-6".to_string()),
        );
    }

    #[test]
    fn next_tier_model_none_when_no_configured_above() {
        let empty_tier = HashMap::new();
        // Only haiku is configured; already at haiku → nothing above.
        let configured = models_for(&["claude-haiku-4-5"]);
        assert_eq!(
            next_tier_model(Some("claude-haiku-4-5"), &empty_tier, &configured),
            None,
        );
    }

    #[test]
    fn next_tier_model_at_top_returns_none() {
        let empty_tier = HashMap::new();
        let no_models = IndexMap::new();
        // Already at the highest tier — no escalation possible.
        assert_eq!(
            next_tier_model(Some("claude-opus-4-6"), &empty_tier, &no_models),
            None,
        );
    }

    /// bug-477ede: a planner outside the chain used to be "escalated" to the
    /// cheapest configured chain model. The retry now keeps it.
    #[test]
    fn next_tier_model_never_downgrades_an_unknown_model() {
        let empty_tier = HashMap::new();
        // A frontier planner outside the chain, with Haiku configured.
        let configured = models_for(&["claude-opus-5-5", "claude-haiku-4-5"]);
        assert_eq!(
            next_tier_model(Some("claude-opus-5-5"), &empty_tier, &configured),
            None,
        );
        assert_eq!(
            next_tier_model(Some("some-random-model"), &empty_tier, &configured),
            None,
        );
        // Unfiltered (nothing configured), an unknown model stays put too.
        assert_eq!(
            next_tier_model(Some("claude-opus-5-5"), &empty_tier, &IndexMap::new()),
            None,
        );
        // With no current model there is no known rank either.
        assert_eq!(next_tier_model(None, &empty_tier, &configured), None);
    }

    /// Keys that are not slugs, as in this repo's roko.toml: the key
    /// `claude-sonnet` is in the chain through its slug, so the retry moves up
    /// to Opus, never down to Haiku.
    #[test]
    fn next_tier_model_matches_a_model_key_by_its_slug() {
        let empty_tier = HashMap::new();
        let configured = models_with(&[
            ("claude-haiku", "claude-haiku-4-5"),
            ("claude-sonnet", "claude-sonnet-4-6"),
            ("claude-opus", "claude-opus-4-6"),
        ]);
        assert_eq!(
            next_tier_model(Some("claude-sonnet"), &empty_tier, &configured),
            Some("claude-opus-4-6".to_string()),
        );
        // Without Opus configured, the retry stays on Sonnet.
        let configured = models_with(&[
            ("claude-haiku", "claude-haiku-4-5"),
            ("claude-sonnet", "claude-sonnet-4-6"),
        ]);
        assert_eq!(
            next_tier_model(Some("claude-sonnet"), &empty_tier, &configured),
            None,
        );
    }

    #[test]
    fn next_tier_model_follows_tier_models_named_by_key() {
        let tiers = HashMap::from([
            ("haiku".to_string(), "fast".to_string()),
            ("sonnet".to_string(), "mid".to_string()),
            ("opus".to_string(), "deep".to_string()),
        ]);
        let configured = models_with(&[
            ("fast", "claude-haiku-4-5"),
            ("mid", "claude-sonnet-4-6"),
            ("deep", "claude-opus-4-6"),
        ]);
        // A current model named by slug finds its tier entry, named by key.
        assert_eq!(
            next_tier_model(Some("claude-sonnet-4-6"), &tiers, &configured),
            Some("deep".to_string()),
        );
        assert_eq!(next_tier_model(Some("deep"), &tiers, &configured), None);
    }

    #[test]
    fn next_tier_model_none_configured_returns_none() {
        let empty_tier = HashMap::new();
        // No chain model is configured → None.
        let configured = models_for(&["totally-different-model"]);
        assert_eq!(
            next_tier_model(Some("claude-haiku-4-5"), &empty_tier, &configured),
            None,
        );
    }

    // ── context-check integration tests ──────────────────────────────────────

    /// Verify that `validate_plan_context` produces a `PLAN_CONTEXT_MISSING`
    /// violation whose message is parseable by the path-extraction code in the
    /// `try_extract_and_validate` closure.  This test pins the message format so
    /// a future change to `plan_policy` cannot silently break the auto-fix path.
    #[test]
    fn plan_context_missing_message_is_parseable() {
        use crate::plan_policy::{PlanExecutionPolicy, validate_plan_context};
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let plan_dir = dir.path().join("my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        // Write tasks.toml so PLAN_ARTIFACT_MISSING doesn't fire.
        std::fs::write(plan_dir.join("tasks.toml"), "[meta]\n").unwrap();

        // Write only "src/lib.rs"; "src/missing.rs" is intentionally absent.
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "").unwrap();

        // Build a minimal TasksFile via TOML so we don't need to construct
        // every optional field of TaskDef by hand.
        let toml_src = r#"
[meta]
plan = "my-plan"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "test"
role = "implementer"
status = "ready"
tier = "focused"
files = ["src/lib.rs"]

[task.context]
[[task.context.read_files]]
path = "src/lib.rs"
why = "exists"

[[task.context.read_files]]
path = "src/missing.rs"
why = "does not exist on disk"

[[task.verify]]
phase = "structural"
command = "true"
"#;
        let tasks_file =
            crate::task_parser::TasksFile::parse_str(toml_src).expect("test TOML must parse");

        let violations = validate_plan_context(
            &tasks_file,
            dir.path(),
            &plan_dir,
            PlanExecutionPolicy::normal(),
        );

        let missing: Vec<_> = violations
            .iter()
            .filter(|v| v.code == "PLAN_CONTEXT_MISSING")
            .collect();
        assert!(
            !missing.is_empty(),
            "expected a PLAN_CONTEXT_MISSING violation"
        );

        // This is the exact extraction logic used in try_extract_and_validate.
        let path = missing[0]
            .message
            .strip_prefix("declared context file `")
            .and_then(|s| s.find('`').map(|end| s[..end].to_string()))
            .expect("PLAN_CONTEXT_MISSING message format must be parseable");
        assert_eq!(path, "src/missing.rs");
    }

    /// bug-1c75cc: a generated plan whose read_files range ends one line past
    /// the end of its file gets the range clamped to the file; a range that
    /// starts past the end is left for the check to refuse.
    #[test]
    fn generated_plan_context_range_past_eof_is_clamped() {
        use crate::plan_policy::{PlanExecutionPolicy, validate_plan_context};
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let plan_dir = dir.path().join("my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        let sixteen: String = (1..=16).map(|n| format!("// line {n}\n")).collect();
        std::fs::write(dir.path().join("src/lib.rs"), sixteen).unwrap();

        let toml_src = r#"
[meta]
plan = "my-plan"
total = 2
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "one past the end"
role = "implementer"
status = "ready"
tier = "focused"
files = ["src/lib.rs"]

[task.context]
[[task.context.read_files]]
path = "src/lib.rs"
lines = "1-17"
why = "the whole file, counted with the empty line after the final newline"

[[task.verify]]
phase = "structural"
command = "true"

[[task]]
id = "T2"
title = "past the end entirely"
role = "implementer"
status = "ready"
tier = "focused"
files = ["src/lib.rs"]

[task.context]
[[task.context.read_files]]
path = "src/lib.rs"
lines = "20-25"
why = "starts past the end"

[[task.verify]]
phase = "structural"
command = "true"
"#;
        let mut tasks_file =
            crate::task_parser::TasksFile::parse_str(toml_src).expect("test TOML must parse");
        let ranges = |tasks: &crate::task_parser::TasksFile| {
            validate_plan_context(tasks, dir.path(), &plan_dir, PlanExecutionPolicy::normal())
                .into_iter()
                .filter(|v| v.code == "PLAN_CONTEXT_RANGE")
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(ranges(&tasks_file).len(), 2, "{:?}", ranges(&tasks_file));

        assert_eq!(
            clamp_context_ranges_past_eof(&mut tasks_file, dir.path()),
            1
        );
        let t1 = &tasks_file.tasks[0].context.as_ref().unwrap().read_files[0];
        assert_eq!(t1.lines.as_deref(), Some("1-16"));
        let t2 = &tasks_file.tasks[1].context.as_ref().unwrap().read_files[0];
        assert_eq!(
            t2.lines.as_deref(),
            Some("20-25"),
            "a range past the end stays"
        );

        let remaining = ranges(&tasks_file);
        assert_eq!(remaining.len(), 1, "{remaining:?}");
        assert!(remaining[0].contains("20-25"), "{remaining:?}");
    }

    /// Verify that after dropping PLAN_CONTEXT_MISSING paths from a TasksFile
    /// and re-validating, no further PLAN_CONTEXT_MISSING violations remain.
    #[test]
    fn dropping_missing_read_files_clears_context_violations() {
        use crate::plan_policy::{PlanExecutionPolicy, validate_plan_context};
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let plan_dir = dir.path().join("my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        std::fs::write(plan_dir.join("tasks.toml"), "[meta]\n").unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "").unwrap();

        let toml_src = r#"
[meta]
plan = "my-plan"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "test"
role = "implementer"
status = "ready"
tier = "focused"
files = ["src/lib.rs"]

[task.context]
[[task.context.read_files]]
path = "src/lib.rs"
why = "exists"

[[task.context.read_files]]
path = "src/missing.rs"
why = "does not exist on disk"

[[task.verify]]
phase = "structural"
command = "true"
"#;
        let mut tasks_file =
            crate::task_parser::TasksFile::parse_str(toml_src).expect("test TOML must parse");

        // Collect the missing paths.
        let violations = validate_plan_context(
            &tasks_file,
            dir.path(),
            &plan_dir,
            PlanExecutionPolicy::normal(),
        );
        let missing_paths: std::collections::HashSet<String> = violations
            .iter()
            .filter(|v| v.code == "PLAN_CONTEXT_MISSING")
            .filter_map(|v| {
                v.message
                    .strip_prefix("declared context file `")
                    .and_then(|s| s.find('`').map(|end| s[..end].to_string()))
            })
            .collect();
        assert!(!missing_paths.is_empty());

        // Drop the missing paths (mirrors the logic in try_extract_and_validate).
        for task in &mut tasks_file.tasks {
            if let Some(ctx) = task.context.as_mut() {
                ctx.read_files.retain(|f| !missing_paths.contains(&f.path));
            }
        }

        // Re-validate — no PLAN_CONTEXT_MISSING should remain.
        let remaining = validate_plan_context(
            &tasks_file,
            dir.path(),
            &plan_dir,
            PlanExecutionPolicy::normal(),
        );
        let still_missing: Vec<_> = remaining
            .iter()
            .filter(|v| v.code == "PLAN_CONTEXT_MISSING")
            .collect();
        assert!(
            still_missing.is_empty(),
            "no PLAN_CONTEXT_MISSING should remain after dropping: {still_missing:?}"
        );
    }
}
