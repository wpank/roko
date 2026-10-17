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

use super::{accept_blocks, dry_run_fs, overview, source_checks};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

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
         redirection or test count around it. To make one fail before its task is done, name \
         the new tests in the command (and in the task's description), or put a `grep -q` for \
         them before it. Keep the rest of the plan as it is. Output the complete plan as a \
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

/// Repair a generated draft's mechanical defects before it is checked
/// (gap-b95913), rather than send it back to the planner for them: its
/// `meta.total` becomes its number of tasks, each task keeps its first
/// `policy.max_read_files_per_task` `read_files`, and its line ranges in files
/// no earlier task changes fit those files
/// ([`crate::plan_policy::fit_ranges_to_files`]). Returns one line per repair.
fn repair_generated_draft(
    plan: &mut TasksFile,
    policy: crate::plan_policy::PlanExecutionPolicy,
    workdir: &Path,
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
    let keep = policy.max_read_files_per_task;
    for task in &mut plan.tasks {
        if let Some(context) = task.context.as_mut()
            && context.read_files.len() > keep
        {
            let dropped: Vec<String> = context
                .read_files
                .drain(keep..)
                .map(|read_file| read_file.path)
                .collect();
            repairs.push(format!(
                "{}: kept its first {keep} read_files and dropped {}",
                task.id,
                dropped.join(", ")
            ));
        }
    }
    repairs.extend(crate::plan_policy::fit_ranges_to_files(plan, workdir));
    repairs
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
         - `files` lists only real paths that exist in the codebase (no placeholders)\n\
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
    } = request;
    let requested_plans_root = requested_plans_root.or(match source {
        PlanSource::Regenerate(plan_dir) => plan_dir.parent(),
        _ => None,
    });
    let workdir = workdir.to_path_buf();
    async {
        let t_total = Instant::now();
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
            emit(PlanProgress {
                stage,
                attempt,
                attempts,
                reason,
                model: model.to_string(),
                calls: spend.calls() + u32::from(calling),
                cost_usd: spend.spent_usd(),
                max_usd: capped,
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
                emit(PlanProgress {
                    calls: spend.calls(),
                    cost_usd: spend.spent_usd(),
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
        let checks = source_checks::source_checks(&source.content);
        if !checks.is_empty() {
            tracing::info!(
                slug,
                checks = checks.len(),
                "plan generate: source checks in the planner prompt"
            );
            extra.push_str("\n\n---\n\n");
            extra.push_str(&source_checks::planner_block(&checks, &source.kind));
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
        report("planning", (1, 1), None, planner_model.as_str());
        let call = run_agent_capture_silent_with_usage(planner_call).await?;
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
                tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                let retry = run_agent_capture_silent_with_usage(planner_call).await?;
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
        let extract_and_validate = |raw: &str| -> std::result::Result<String, String> {
            let toml_content = extract_fenced_block(raw, "toml")
                .or_else(|| extract_fenced_block(raw, "tasks.toml"))
                .or_else(|| extract_toml_content_fallback(raw));
            tracing::info!(
                has_toml_block = toml_content.is_some(),
                toml_block_len = toml_content.map(|s| s.len()).unwrap_or(0),
                "plan generate: fenced block extraction"
            );
            let toml_content =
                toml_content.ok_or_else(|| "no TOML block found in agent output".to_string())?;

            // Fast structural pre-check: verify required sections are present
            // before attempting a full TOML parse.
            if !toml_content.contains("[meta]") {
                return Err("TOML block is missing the required [meta] section".to_string());
            }
            if !toml_content.contains("[[task]]") {
                return Err("TOML block is missing required [[task]] entries".to_string());
            }

            let validated = validate_and_fix_generated_plan(
                toml_content,
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
            let mut repairs = repair_generated_draft(&mut parsed, policy, workdir_ref);
            repairs.extend(source_checks::normalize_covers(&mut parsed, &checks));
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
            let policy_issues = crate::plan_policy::validate_plan_budgets(&parsed, policy);
            if !policy_issues.is_empty() {
                return Err(format!(
                    "generated plan violates the `{}` structural budget:\n{}",
                    template_kind.label(),
                    policy_issues
                        .iter()
                        .map(|issue| format!("  - {issue}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }
            // 3225 (decision 3203): a task over its tier's size limits is
            // a spec defect the planner fixes before a cheap model meets
            // it. Its tier is never raised here, since that would move
            // the work to a pricier rung without the planner choosing to.
            let size_issues = crate::plan_policy::validate_tier_sizes(&parsed, policy);
            if !size_issues.is_empty() {
                return Err(format!(
                    "generated plan has tasks over their tier's size limits; split each \
                     on independent outputs with disjoint files, or raise its tier:\n{}",
                    size_issues
                        .iter()
                        .map(|issue| format!("  - {issue}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
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
                return Ok(validated);
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
                Ok(re_serialized)
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
        let try_extract_and_validate = |raw: &str| -> std::result::Result<String, String> {
            let validated = extract_and_validate(raw)?;
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
                    Ok(validated)
                }
            }
        };

        // gap-dc5d7c: what became of the source's checks in a candidate
        // plan, and how many it does not keep (all of them when it does not
        // parse).
        let coverage_of = |candidate: &str| {
            TasksFile::parse_str(candidate)
                .ok()
                .map(|plan| source_checks::SourceCoverage::of(&checks, &plan))
        };
        let missing_of = |candidate: &str| {
            coverage_of(candidate).map_or(checks.len(), |coverage| coverage.unkept())
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
                let retry_result = run_agent_capture_silent_with_usage(AgentExecOpts {
                    prompt: &retry_prompt,
                    model: retry_model,
                    ..planner_call
                })
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
                let retry = run_agent_capture_silent_with_usage(AgentExecOpts {
                    prompt: &prompt,
                    ..planner_call
                })
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
        // and the plan that keeps more is kept.
        let mut checks_regenerated = false;
        if let Ok(candidate) = &validated_toml
            && let Some(coverage) = coverage_of(candidate)
            && coverage.unkept() > 0
            && !skip_for_cap("keep the source checks it left out")
        {
            let missing = coverage.unkept();
            let why = format!(
                "{missing} of the source's {} checks are not kept as verify steps",
                checks.len()
            );
            eprintln!("  {why}; asking the planner once more");
            report("regenerating", (1, 1), Some(why), planner_model.as_str());
            let prompt = source_checks::regeneration_prompt(&task_prompt, candidate, &coverage);
            let retry = run_agent_capture_silent_with_usage(AgentExecOpts {
                prompt: &prompt,
                ..planner_call
            })
            .await;
            if let Ok(retry) = &retry {
                spend.record(retry).await;
                report_spend();
            }
            checks_regenerated = true;
            if let Ok(retry) = retry
                && retry.exit_code == 0
                && let Ok(better) = try_extract_and_validate(&retry.output)
                && missing_of(&better) < missing
                && hidden_output_lines(&better).len() <= hidden_output_lines(candidate).len()
            {
                validated_toml = Ok(better);
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
                let retry = run_agent_capture_silent_with_usage(AgentExecOpts {
                    prompt: &prompt,
                    ..planner_call
                })
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

        // gap-dc5d7c: what became of each of the source's checks.
        let source_coverage = validated_toml
            .as_ref()
            .ok()
            .filter(|_| !checks.is_empty())
            .and_then(|plan| coverage_of(plan));
        if let Some(coverage) = &source_coverage {
            println!("📋 {}", coverage.summary_line());
            for line in coverage.detail_lines() {
                println!("   {line}");
            }
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
                let plan_md_content = extract_fenced_block(&output, "plan.md")
                    .or_else(|| extract_fenced_block(&output, "markdown"))
                    .or_else(|| extract_fenced_block(&output, "md"));
                if let Some(plan_md) = plan_md_content {
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
            let has_toml_like = output.contains("[meta]") || output.contains("[[task]]");
            let toml_hint = if has_toml_like {
                "\nhint: The model output TOML without proper fencing. \
                 Try a more capable model or check the plan_generate system prompt."
            } else {
                ""
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
                 The agent output ({} bytes) did not contain a parseable ```toml block.\n\
                 hint: Try again, or create plans/{slug}/tasks.toml manually.{toml_hint}{drafts}",
                output.len()
            ));
        }

        let extraction_ms = t_phase.elapsed().as_millis();
        tracing::info!(
            extraction_and_write_ms = extraction_ms,
            "plan generate: TOML extracted and written"
        );
        let t_phase = Instant::now();
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
            source_checks: source_coverage.map(|coverage| coverage.record(checks_regenerated)),
            cost_usd: spend.spent_usd(),
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

                        if role == "implementer" && !files.is_empty() {
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
        };
        let partial = GenerationOutcome {
            process_success: true,
            artifact_valid: false,
            validation_report: None,
            spec_quality: None,
            source_checks: None,
            cost_usd: 0.0,
        };
        let failure = GenerationOutcome {
            process_success: false,
            artifact_valid: true,
            validation_report: None,
            spec_quality: None,
            source_checks: None,
            cost_usd: 0.0,
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
    /// 3225 (decision 3203): a generated task over its tier's size limits
    /// fails validation. A mechanical task with six files goes back to the
    /// planner with the limit it exceeds, and the split plan is written.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generation_rejects_a_task_over_its_tier_size() {
        const OVERSIZED: &str = "\
            [meta]\nplan = \"widget\"\ntotal = 1\ndone = 0\nstatus = \"ready\"\n\n\
            [[task]]\nid = \"T1\"\ntitle = \"Add the widget modules\"\n\
            description = \"Create the six widget modules.\"\nstatus = \"ready\"\n\
            role = \"implementer\"\ntier = \"mechanical\"\nfiles = [\"src/a.rs\", \
            \"src/b.rs\", \"src/c.rs\", \"src/d.rs\", \"src/e.rs\", \"src/f.rs\"]\n\
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
        assert!(retry.contains("raise its tier to"), "{retry}");
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
                rungs: Some(vec![roko_core::config::routing::LadderRung {
                    name: "docs".to_string(),
                    model: "gpt-4o-mini".to_string(),
                }]),
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
