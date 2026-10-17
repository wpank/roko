//! `roko plan import --mori`: mori plan directories as roko plans
//! (gap-624658).
//!
//! mori, the orchestrator roko replaces, kept each plan in
//! `.mori/plans/<plan>/`: a `tasks.toml` whose tasks carry prose
//! `acceptance` and no runnable checks, the checks in `verify-tasks.toml` and
//! `verify.sh`, reviewer checks in `review-tasks.toml`, docs work in
//! `scribe-tasks.toml`, a `rubric.md`, a `decomposition.md` and the plan
//! document. [`import_mori_plan`] maps them onto one roko plan without a
//! model, and only reads the mori directory:
//!
//! - Tasks keep their id, title, files, acceptance and the hints roko acts
//!   on. `category = "docs"` makes a scribe and anything else an implementer;
//!   `complexity_band` picks the tier (fast, standard and complex give
//!   mechanical, focused and integrative). A task that names no file may
//!   change its crate's directory, or, with no crate either, becomes a
//!   quick-reviewer that runs checks.
//! - Checks: the blocking `verify-tasks.toml` entries whose tests or
//!   invariant the task's acceptance names (behind a `grep` for each named
//!   test, so a filter that matches nothing cannot pass), acceptance lines
//!   that give a command to pass ("Checkpoint: `cmd`", "`cmd` exits 0"), the
//!   task's own `command` and `test_fns`, and its "File exists" lines as one
//!   step. A task with none of these gets `cargo check -p <crate>`, or else a
//!   check that its files exist. A file check that already passes in the
//!   workspace says so (`expect = "pass_on_base"`), so the spec gate does not
//!   take finished work for a check that cannot fail.
//! - The blocking checks no task claims, `verify.sh`'s `run_check` steps and
//!   the rubric's commands are the whole-plan check (`[[meta.verify]]`).
//!   mori's non-blocking checks are listed for the reviewer instead.
//! - `review-tasks.toml` becomes one quick-reviewer task after the others,
//!   and `scribe-tasks.toml` docs tasks.
//! - `depends_on = ["08e:T9"]` becomes `depends_on_plan` on the plan whose
//!   directory is or starts with `08e`, and mori's `exclusive_files` lock
//!   becomes dependencies between tasks that write the same path.
//! - A task over its tier's size limits gets the smallest tier it fits, up to
//!   integrative. A task with more files than that is narrowed to their
//!   directories (the full list goes beside the plan) and, when there are
//!   still too many, split into parts. A task with more checks has its
//!   `cargo test` checks of one package joined into one run, and the checks
//!   still past the limit move to the whole-plan check.
//!
//! The plan document, the rubric, the decomposition and mori's other
//! references are copied beside `tasks.toml`, and [`write_mori_import`]
//! writes the plan's `brief.md` as `roko plan prepare` does, which lists the
//! rubric and the decomposition. What has no roko equivalent is named in the
//! [`MoriImportReport`].

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Context as _, Result, anyhow, bail};
use regex::Regex;
use roko_core::task::TaskTier;
use roko_gate::spec_quality::vacuous_reason;
use serde::Serialize;
use toml::{Table, Value};

use crate::plan_brief::{DECOMPOSITION_FILE, RUBRIC_FILE};
use crate::plan_policy::{PlanExecutionPolicy, TierSizeLimits};
use crate::task_parser::TasksFile;
use crate::workspace_lock::WorkspaceLockGuard;

/// mori hints roko acts on, kept as they are.
const KEPT_HINTS: [&str; 8] = [
    "category",
    "complexity_band",
    "reasoning_level",
    "quality_profile",
    "context_weight",
    "research_before_edit",
    "escalate_on_retry",
    "plan_section",
];

/// mori task keys roko does not act on (`plan validate` warns about them,
/// PLAN_039 and PLAN_047): dropped, and counted in the report.
const DROPPED_HINTS: [&str; 12] = [
    "parallel_group",
    "exclusive_files",
    "tags",
    "dependency_tags",
    "fixture_keys",
    "sidecar_requirements",
    "integration_surfaces",
    "speed_priority",
    "preferred_model",
    "preferred_provider",
    "example_pattern",
    "skills",
];

/// mori task keys the import maps itself.
const MAPPED_KEYS: [&str; 10] = [
    "id",
    "title",
    "status",
    "files",
    "acceptance",
    "depends_on",
    "estimated_minutes",
    "context_files",
    "command",
    "test_fns",
];

/// mori run state in `[meta]`, which is not part of a plan's spec.
const RUN_STATE: [&str; 6] = [
    "iteration",
    "done",
    "status",
    "verify_passed",
    "last_gate",
    "completed_at",
];

/// mori references copied beside `tasks.toml` as they are. The rubric and
/// the decomposition are the documents `roko plan prepare --full` writes, so
/// the plan's brief lists them.
const REFERENCES: [&str; 4] = [
    RUBRIC_FILE,
    DECOMPOSITION_FILE,
    "prd-extract.md",
    "testing-backlog.md",
];

/// The mori files the import reads or copies; any other file of the plan
/// directory is named in the report.
const KNOWN_FILES: [&str; 10] = [
    "tasks.toml",
    "plan.md",
    "verify-tasks.toml",
    "review-tasks.toml",
    "scribe-tasks.toml",
    "verify.sh",
    RUBRIC_FILE,
    DECOMPOSITION_FILE,
    "prd-extract.md",
    "testing-backlog.md",
];

/// Paths of mori's own scripts and state, which a roko workspace lacks.
const MORI_INTERNAL: [&str; 2] = ["plans/context/", ".mori/"];

/// The largest tier the import raises a task to.
const TOP_TIER: TaskTier = TaskTier::Integrative;

/// The most `read_files` one task gets (`plan validate` allows 16).
const MAX_READ_FILES: usize = 15;

/// The most words of a docs entry's concept the docs task quotes.
const CONCEPT_WORDS: usize = 16;

/// The most section headings one docs check looks for.
const MAX_SECTIONS: usize = 6;

static COMMAND_START: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(cargo|bash|sh|test|python3?|just|node|npm|npx|pnpm|yarn|forge|go|make)\b")
        .expect("command regex")
});
static BACKTICKED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`([^`]+)`").expect("backtick regex"));
static FILE_EXISTS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^File exists:\s*`?([^`\s]+)`?\s*$").expect("file exists regex"));
static EXITS_ZERO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bexits? 0\b").expect("exits regex"));
static PIPED_RUNNER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(cargo (?:test|check|build)\b[^|>]*?)\s*(?:2>&1\s*)?\|\s*(?:grep|rg|tail|head|wc)\b.*$",
    )
    .expect("piped runner regex")
});
static CARGO_TEST_FILTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(cargo test(?:\s+(?:-p|--package|--test)\s+[\w.-]+|\s+--lib|\s+--tests)*)\s+([\w:]+)$",
    )
    .expect("cargo test filter regex")
});
static RUN_CHECK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\s*run_check(?:_if_crate\s+\S+)?\s+"[^"]*"\s+"([^"]*)"\s+(.+?)\s*$"#)
        .expect("run_check regex")
});
static PACKAGE_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(-p\s+)(?:[\w.-]+/)+([\w.-]+)").expect("package path regex"));
static TEST_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").expect("test name regex"));
static SHELL_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(?:ba)?sh\s+-c\s+(?:'([^']*)'|"([^"]*)")\s*$"#).expect("shell script regex")
});

/// Which of a mori plan's own task files become roko tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoriImportOptions {
    /// One quick-reviewer task from `review-tasks.toml`.
    pub reviewer: bool,
    /// Docs tasks from `scribe-tasks.toml`.
    pub scribe: bool,
}

impl Default for MoriImportOptions {
    fn default() -> Self {
        Self {
            reviewer: true,
            scribe: true,
        }
    }
}

/// One mori plan as a roko plan, not yet written.
#[derive(Debug, Clone)]
pub struct MoriImport {
    /// The roko plan id: the mori plan directory's name.
    pub plan_id: String,
    /// The roko `tasks.toml`.
    pub tasks_toml: String,
    /// Files to write beside `tasks.toml`, as `(name, contents)`.
    pub companions: Vec<(String, String)>,
    /// What the import mapped, carried, dropped and changed.
    pub report: MoriImportReport,
}

/// What `roko plan import --mori` did with one mori plan.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MoriImportReport {
    /// The roko plan id.
    pub plan: String,
    /// The mori plan directory.
    pub source: String,
    /// The mori plan's tasks.
    pub mori_tasks: usize,
    /// Tasks written, the reviewer and docs tasks included.
    pub tasks: usize,
    /// mori tasks marked done, imported as ready like the rest.
    pub done_in_mori: usize,
    /// Where the tasks' checks came from.
    pub checks: CheckCounts,
    /// Whole-plan checks (`[[meta.verify]]`).
    pub plan_checks: usize,
    /// mori's non-blocking checks, listed for the reviewer.
    pub advisory_checks: usize,
    /// `depends_on` entries that name a task of another plan.
    pub cross_plan: Vec<CrossPlanDependency>,
    /// Dependencies added so that two tasks never write the same path at
    /// once, as `[task, the task it now waits for]`.
    pub serialized: Vec<[String; 2]>,
    /// Tasks given a higher tier than their `complexity_band`.
    pub tier_changes: Vec<TierChange>,
    /// Tasks whose files were narrowed to directories.
    pub narrowed: Vec<NarrowedFiles>,
    /// Tasks split into parts.
    pub splits: Vec<TaskSplit>,
    /// Tasks whose only check is that their files exist: mori gave them no
    /// runnable check and no crate.
    pub structural_only: Vec<String>,
    /// Tasks left out: nothing to write and nothing to check.
    pub dropped_tasks: Vec<String>,
    /// Everything else that was dropped, copied or could not be mapped.
    pub notes: Vec<String>,
}

/// Where an import's task checks came from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CheckCounts {
    /// Blocking `verify-tasks.toml` entries the task's acceptance names.
    pub verify_tasks: usize,
    /// Acceptance lines that give a command to pass.
    pub acceptance: usize,
    /// Files from acceptance "File exists" lines.
    pub file_exists: usize,
    /// A mori task's own `command` or `test_fns`.
    pub task_command: usize,
    /// `cargo check -p <crate>` for a task with no other check.
    pub compile_fallback: usize,
    /// `cargo test` checks joined into another check of their package, so
    /// that a task fits its tier's limit.
    pub joined: usize,
    /// Checks past a task's tier limit, moved to the whole-plan check.
    pub moved_to_plan: usize,
}

/// A `depends_on` entry that names a task of another mori plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CrossPlanDependency {
    /// The task that depends on it.
    pub task: String,
    /// The mori reference, such as `08e:T9`.
    pub reference: String,
    /// The plan the task now waits for as a whole, when one was found.
    pub plan: Option<String>,
}

/// A task given a higher tier than its `complexity_band`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TierChange {
    /// The task.
    pub task: String,
    /// The tier its band gives.
    pub from: String,
    /// The tier it fits.
    pub to: String,
    /// The limit it is over at `from`.
    pub why: String,
}

/// A task whose files were narrowed to the directories that hold them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NarrowedFiles {
    /// The task.
    pub task: String,
    /// How many files mori named.
    pub files: usize,
    /// The directories the task now writes.
    pub to: Vec<String>,
}

/// A task split into parts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskSplit {
    /// The mori task.
    pub task: String,
    /// Its parts; the last runs the task's checks once the others are done.
    pub parts: Vec<String>,
    /// Why it was split.
    pub why: String,
}

/// One mori plan imported into a plans directory.
#[derive(Debug, Clone)]
pub struct ImportedPlan {
    /// The plan directory written.
    pub dir: PathBuf,
    /// What the import did.
    pub report: MoriImportReport,
}

/// The plan directories under a `.mori/plans` directory: every
/// subdirectory with a `tasks.toml`, by name.
///
/// # Errors
///
/// When `root` cannot be read.
pub fn mori_plan_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(root)
        .with_context(|| format!("read {}", root.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join("tasks.toml").is_file())
        .collect();
    dirs.sort();
    Ok(dirs)
}

/// The workspace lock `roko plan import` takes. A fresh import takes none: as
/// `roko plan generate` does (#226), it reads the workspace and writes only
/// new plan directories, each moved into place whole, so it runs beside the
/// roko serve that holds the workspace (bug-dfe4c5). Replacing plans
/// (`force`) takes the exclusive lock, since a run may be using them.
///
/// # Errors
///
/// With `force`, when another process holds the workspace; the error names a
/// roko serve that does.
pub fn import_lock(workspace: &Path, force: bool) -> Result<Option<WorkspaceLockGuard>> {
    if !force {
        return Ok(None);
    }
    crate::workspace_lock::acquire_workspace_lock(&workspace.join(".roko"))
        .map(Some)
        .map_err(
            |error| match crate::doctor::serve_holding_workspace(workspace) {
                Some(serve) => anyhow!(
                    "roko serve (PID {}) holds this workspace, and --force replaces plans a run \
                     there may be using: import into a new --out or --set, or stop serve first",
                    serve.pid
                ),
                None => error,
            },
        )
}

/// Import mori plans into `plans_root`, a directory inside `workspace`, and
/// write them: each of `sources` is a mori plan directory or a directory of
/// them (`.mori/plans/`). Nothing is written when an import fails, or when a
/// plan directory holds a `tasks.toml` already and `force` is off.
///
/// # Errors
///
/// When a source holds no mori plan, two plans share a name, `plans_root` is
/// outside `workspace`, an import fails, a plan exists and `force` is off,
/// or a file cannot be written.
pub fn import_mori_plans(
    sources: &[PathBuf],
    workspace: &Path,
    plans_root: &Path,
    options: MoriImportOptions,
    force: bool,
) -> Result<Vec<ImportedPlan>> {
    let mut dirs = Vec::new();
    for source in sources {
        if source.join("tasks.toml").is_file() {
            dirs.push(source.clone());
            continue;
        }
        let found = mori_plan_dirs(source)?;
        if found.is_empty() {
            bail!(
                "{} is not a mori plan directory (no tasks.toml) and holds none",
                source.display()
            );
        }
        dirs.extend(found);
    }
    let root = relative_path(plans_root, workspace)?;
    let mut imports = Vec::new();
    let mut names = HashSet::new();
    for dir in &dirs {
        let name = dir
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("{} has no plan name", dir.display()))?;
        if !names.insert(name.to_string()) {
            bail!("two mori plans are named {name}; import them into different plan sets");
        }
        let plan_dir = if root.is_empty() {
            name.to_string()
        } else {
            format!("{root}/{name}")
        };
        let import = import_mori_plan(dir, workspace, &plan_dir, options)?;
        imports.push((plans_root.join(name), import));
    }
    let existing: Vec<String> = imports
        .iter()
        .filter(|(dir, _)| dir.join("tasks.toml").exists())
        .map(|(dir, _)| dir.display().to_string())
        .collect();
    if !existing.is_empty() && !force {
        bail!(
            "these plan directories hold a plan already; pass --force to replace them: {}",
            existing.join(", ")
        );
    }
    let mut imported = Vec::new();
    for (dir, import) in imports {
        write_mori_import(&import, workspace, &dir, force)?;
        imported.push(ImportedPlan {
            dir,
            report: import.report,
        });
    }
    Ok(imported)
}

/// Write an import into `plan_dir`: `tasks.toml`, its companions, and the
/// plan's `brief.md` (`roko plan prepare`). A `plan_dir` that holds a
/// `tasks.toml` already is refused unless `force`. A new plan is written
/// beside its place, in a hidden directory plan discovery skips, and moved in
/// whole, so a reader such as roko serve sees all of it or none (bug-dfe4c5).
///
/// # Errors
///
/// When `plan_dir` holds a plan and `force` is off, or a file cannot be
/// written.
pub fn write_mori_import(
    import: &MoriImport,
    workspace: &Path,
    plan_dir: &Path,
    force: bool,
) -> Result<()> {
    if plan_dir.join("tasks.toml").exists() && !force {
        bail!(
            "{} holds a plan already; pass --force to replace it",
            plan_dir.display()
        );
    }
    if plan_dir.exists() {
        return write_plan_files(import, workspace, plan_dir);
    }
    let (Some(parent), Some(name)) = (
        plan_dir.parent(),
        plan_dir.file_name().and_then(|name| name.to_str()),
    ) else {
        bail!("{} names no plan directory", plan_dir.display());
    };
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let staging = parent.join(format!(".{name}.import-{}", std::process::id()));
    if staging.exists() {
        fs::remove_dir_all(&staging).with_context(|| format!("remove {}", staging.display()))?;
    }
    let written = write_plan_files(import, workspace, &staging).and_then(|()| {
        fs::rename(&staging, plan_dir).with_context(|| {
            if plan_dir.exists() {
                format!(
                    "{} appeared while the import ran; pass --force to replace it",
                    plan_dir.display()
                )
            } else {
                format!("move {} to {}", staging.display(), plan_dir.display())
            }
        })
    });
    if written.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    written
}

/// Write an import's files into `dir`: `tasks.toml`, its companions and the
/// plan's `brief.md`.
fn write_plan_files(import: &MoriImport, workspace: &Path, dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let tasks = dir.join("tasks.toml");
    fs::write(&tasks, &import.tasks_toml).with_context(|| format!("write {}", tasks.display()))?;
    for (name, contents) in &import.companions {
        let path = dir.join(name);
        fs::write(&path, contents).with_context(|| format!("write {}", path.display()))?;
    }
    crate::plan_brief::prepare(dir, workspace, true)?;
    Ok(())
}

/// Import the mori plan in `src` (a `.mori/plans/<plan>/` directory) as a
/// roko plan that will live in `<workspace>/<plan_dir>/`. `workspace` decides
/// which of mori's context files and checked paths exist; `plan_dir` is
/// relative to it, `/`-separated.
///
/// # Errors
///
/// When `src` has no readable `tasks.toml` with a task in it, or the plan
/// written would not parse (an import bug).
pub fn import_mori_plan(
    src: &Path,
    workspace: &Path,
    plan_dir: &str,
    options: MoriImportOptions,
) -> Result<MoriImport> {
    let mori = MoriPlan::read(src)?;
    let mut importer = Importer::new(&mori, workspace, plan_dir);

    let ids: HashSet<String> = mori
        .tasks
        .iter()
        .filter_map(|task| str_of(task, "id"))
        .map(|id| id.trim().to_string())
        .collect();
    let mut drafts: Vec<Draft> = mori
        .tasks
        .iter()
        .filter_map(|task| importer.task_draft(task, &ids))
        .collect();
    if drafts.is_empty() {
        bail!(
            "{} has no task with an id",
            src.join("tasks.toml").display()
        );
    }
    importer.note_hints();
    let mut plan_checks = importer.plan_checks();
    importer.fill_check_tasks(&mut drafts, &plan_checks);
    let taken: HashSet<String> = drafts.iter().map(|draft| draft.id.clone()).collect();
    let mut drafts = importer.fit_sizes(drafts, &mut plan_checks, &taken);
    importer.serialize_shared_writers(&mut drafts);
    importer.add_context(&mut drafts);

    // The reviewer and the docs tasks come last: they read the finished work.
    let leaves: Vec<String> = drafts
        .iter()
        .filter(|draft| {
            !drafts
                .iter()
                .any(|other| other.depends_on.contains(&draft.id))
        })
        .map(|draft| draft.id.clone())
        .collect();
    let mut taken = taken;
    taken.extend(drafts.iter().map(|draft| draft.id.clone()));
    let reviewer = if options.reviewer {
        importer.review_task(&drafts, &plan_checks, &leaves, &taken)
    } else {
        if !mori.review.is_empty() {
            importer.report.notes.push(format!(
                "review-tasks.toml ({} checks) was not imported (--no-reviewer)",
                mori.review.len()
            ));
        }
        None
    };
    let docs = if options.scribe {
        importer.docs_tasks(&leaves, &taken)
    } else {
        if !mori.scribe.is_empty() {
            importer.report.notes.push(format!(
                "scribe-tasks.toml ({} entries) was not imported (--no-scribe)",
                mori.scribe.len()
            ));
        }
        Vec::new()
    };
    let has_reviewer = reviewer.is_some();
    let has_docs = !docs.is_empty();
    for mut extra in reviewer.into_iter().chain(docs) {
        importer.fit_tier(&mut extra, &mut plan_checks);
        drafts.push(extra);
    }

    let companions = importer.companions(src, &drafts, has_reviewer, has_docs);
    let meta = importer.meta(&drafts, &plan_checks);
    importer.report.tasks = drafts.len();
    importer.report.plan_checks = plan_checks.len();
    let tasks_toml = render(&mori.id, src, &meta, &plan_checks, &drafts);
    TasksFile::parse_str(&tasks_toml)
        .with_context(|| format!("the plan imported from {} does not parse", src.display()))?;
    Ok(MoriImport {
        plan_id: mori.id.clone(),
        tasks_toml,
        companions,
        report: importer.report,
    })
}

/// One import as text: what was written and what to know about it.
#[must_use]
pub fn render_report(report: &MoriImportReport, dir: &Path) -> String {
    let mut out = format!(
        "{}: {} tasks from {} mori tasks -> {}\n",
        report.plan,
        report.tasks,
        report.mori_tasks,
        dir.display()
    );
    let checks = &report.checks;
    let _ = writeln!(
        out,
        "  checks: {} from verify-tasks.toml, {} from acceptance, {} files from \"File exists\", \
         {} task commands, {} `cargo check` fallbacks ({} joined into one run per package, {} \
         moved to the whole-plan check); {} whole-plan; {} non-blocking listed for the reviewer",
        checks.verify_tasks,
        checks.acceptance,
        checks.file_exists,
        checks.task_command,
        checks.compile_fallback,
        checks.joined,
        checks.moved_to_plan,
        report.plan_checks,
        report.advisory_checks
    );
    for dependency in &report.cross_plan {
        let target = dependency.plan.as_deref().map_or_else(
            || "no such plan beside it, so it is not kept".to_string(),
            |plan| format!("depends_on_plan \"{plan}\""),
        );
        let _ = writeln!(
            out,
            "  cross-plan: {} needs {}: {target}",
            dependency.task, dependency.reference
        );
    }
    for change in &report.tier_changes {
        let _ = writeln!(
            out,
            "  tier: {} {} -> {} ({})",
            change.task, change.from, change.to, change.why
        );
    }
    for narrowed in &report.narrowed {
        let _ = writeln!(
            out,
            "  narrowed: {} {} files -> {}",
            narrowed.task,
            narrowed.files,
            narrowed.to.join(", ")
        );
    }
    for split in &report.splits {
        let _ = writeln!(
            out,
            "  split: {} -> {} ({})",
            split.task,
            split.parts.join(", "),
            split.why
        );
    }
    if !report.structural_only.is_empty() {
        let _ = writeln!(
            out,
            "  only checked for their files: {}",
            report.structural_only.join(", ")
        );
    }
    if !report.dropped_tasks.is_empty() {
        let _ = writeln!(out, "  left out: {}", report.dropped_tasks.join(", "));
    }
    for note in &report.notes {
        let _ = writeln!(out, "  note: {note}");
    }
    out
}

// ---------------------------------------------------------------------------
// Reading a mori plan.

/// A mori plan directory, read.
struct MoriPlan {
    src: PathBuf,
    id: String,
    /// The names of its files.
    present: HashSet<String>,
    meta: Table,
    tasks: Vec<Table>,
    plan_md: Option<String>,
    verify: Vec<Table>,
    review: Vec<Table>,
    scribe: Vec<Table>,
    rubric: Option<String>,
    verify_sh: Option<String>,
    /// The other plans beside it, by name.
    siblings: Vec<String>,
    /// What could not be read.
    notes: Vec<String>,
}

impl MoriPlan {
    fn read(src: &Path) -> Result<Self> {
        let id = src
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .with_context(|| format!("{} has no plan name", src.display()))?
            .to_string();
        let path = src.join("tasks.toml");
        let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let mut table: Table =
            toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        let meta = match table.remove("meta") {
            Some(Value::Table(meta)) => meta,
            _ => Table::new(),
        };
        let tasks = entries(table.remove("task"));
        if tasks.is_empty() {
            bail!("{} has no [[task]]", path.display());
        }
        let mut notes = Vec::new();
        let verify = read_entries(src, "verify-tasks.toml", &mut notes);
        let review = read_entries(src, "review-tasks.toml", &mut notes);
        let scribe = read_entries(src, "scribe-tasks.toml", &mut notes);
        let siblings = src
            .parent()
            .and_then(|root| mori_plan_dirs(root).ok())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|dir| dir.file_name()?.to_str().map(ToOwned::to_owned))
            .collect();
        let present = fs::read_dir(src)
            .with_context(|| format!("read {}", src.display()))?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .filter_map(|entry| entry.file_name().to_str().map(ToOwned::to_owned))
            .collect();
        Ok(Self {
            src: src.to_path_buf(),
            id,
            present,
            meta,
            tasks,
            plan_md: fs::read_to_string(src.join("plan.md")).ok(),
            verify,
            review,
            scribe,
            rubric: fs::read_to_string(src.join(RUBRIC_FILE)).ok(),
            verify_sh: fs::read_to_string(src.join("verify.sh")).ok(),
            siblings,
            notes,
        })
    }

    fn has_file(&self, name: &str) -> bool {
        self.present.contains(name)
    }
}

/// The tables of an array of tables.
fn entries(value: Option<Value>) -> Vec<Table> {
    match value {
        Some(Value::Array(items)) => items
            .into_iter()
            .filter_map(|item| match item {
                Value::Table(table) => Some(table),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The `[[task]]` entries of an optional mori file; one that does not parse
/// is named in `notes`.
fn read_entries(src: &Path, name: &str, notes: &mut Vec<String>) -> Vec<Table> {
    let Ok(text) = fs::read_to_string(src.join(name)) else {
        return Vec::new();
    };
    match toml::from_str::<Table>(&text) {
        Ok(mut table) => entries(table.remove("task")),
        Err(error) => {
            let error = error.to_string();
            notes.push(format!(
                "{name} does not parse, so nothing in it is mapped: {}",
                error.lines().next().unwrap_or_default()
            ));
            Vec::new()
        }
    }
}

// ---------------------------------------------------------------------------
// Tasks under construction.

/// One verify step.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Step {
    phase: String,
    command: String,
    fail_msg: String,
    /// The step already passes in the workspace (`expect = "pass_on_base"`).
    passes_on_base: bool,
}

/// A check of one file, joined with the task's other file checks into one
/// step.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FileCheck {
    file: String,
    command: String,
    /// The file is in the workspace already.
    exists: bool,
}

/// A roko task under construction.
#[derive(Debug, Clone, Default)]
struct Draft {
    id: String,
    title: String,
    role: &'static str,
    tier: TaskTier,
    files: Vec<String>,
    depends_on: Vec<String>,
    depends_on_plan: Vec<String>,
    estimated_minutes: Option<i64>,
    hints: Vec<(String, Value)>,
    acceptance: Vec<String>,
    /// The description, before the sentence naming context files.
    lead: String,
    /// mori's `context_files`.
    context_files: Vec<String>,
    /// `(path, why)`.
    read_files: Vec<(String, String)>,
    checks: Vec<Step>,
    file_checks: Vec<FileCheck>,
    /// The phase and message of the joined file step.
    file_phase: &'static str,
    file_fail: &'static str,
    /// A part of a split task that is not its last, so its own checks are
    /// only that its files exist.
    leading_part: bool,
    /// The full file list of a narrowed task, written beside the plan as
    /// `(name, contents)`.
    file_list: Option<(String, String)>,
    /// `# mori import:` comments written above the task.
    comments: Vec<String>,
}

impl Draft {
    fn new(id: &str, title: &str, role: &'static str, tier: TaskTier) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            role,
            tier,
            file_phase: "structural",
            file_fail: "these files must exist",
            ..Self::default()
        }
    }

    /// The verify steps the task gets: its checks, then its file checks as
    /// one step.
    fn steps(&self) -> Vec<Step> {
        let mut steps = self.checks.clone();
        if !self.file_checks.is_empty() {
            let files: Vec<&str> = self
                .file_checks
                .iter()
                .map(|check| check.file.as_str())
                .collect();
            let shown = match files.len() {
                0..=10 => files.join(", "),
                n => format!("{}, and {} more", files[..10].join(", "), n - 10),
            };
            steps.push(Step {
                phase: self.file_phase.to_string(),
                command: self
                    .file_checks
                    .iter()
                    .map(|check| check.command.as_str())
                    .collect::<Vec<_>>()
                    .join(" && "),
                fail_msg: format!("{}: {shown}", self.file_fail),
                passes_on_base: self.file_checks.iter().all(|check| check.exists),
            });
        }
        steps
    }

    fn step_count(&self) -> usize {
        self.checks.len() + usize::from(!self.file_checks.is_empty())
    }

    /// Add a runnable check, unless it can never fail or the task has it.
    fn add_check(&mut self, phase: &str, command: &str, fail_msg: &str) -> bool {
        push_step(&mut self.checks, phase, command, fail_msg)
    }

    fn add_file_check(&mut self, file: &str, command: String, exists: bool) -> bool {
        if self.file_checks.iter().any(|check| check.file == file) {
            return false;
        }
        self.file_checks.push(FileCheck {
            file: file.to_string(),
            command,
            exists,
        });
        true
    }

    /// Check that each of the task's files exists.
    fn add_structural_checks(&mut self, workspace: &Path) {
        for file in self.files.clone() {
            let (command, exists) = exists_check(&file, workspace);
            self.add_file_check(&file, command, exists);
        }
    }
}

/// The state of one plan's import.
struct Importer<'a> {
    mori: &'a MoriPlan,
    workspace: &'a Path,
    plan_dir: &'a str,
    /// The plan's title, for task descriptions.
    title: String,
    /// What every task's description ends with: where the build steps and
    /// the PRD extract are.
    lead_tail: String,
    /// mori's blocking `verify-tasks.toml` checks.
    blocking: Vec<&'a Table>,
    /// mori's non-blocking ones, for the reviewer.
    advisory: Vec<&'a Table>,
    /// The blocking checks a task took, by index.
    claimed: HashSet<usize>,
    /// Crates mori keeps under `apps/`.
    apps: HashSet<String>,
    /// mori task keys roko does not act on, with how many tasks set each.
    dropped_hints: BTreeMap<&'static str, usize>,
    /// mori task keys the import does not know.
    unknown_keys: Vec<String>,
    report: MoriImportReport,
}

impl<'a> Importer<'a> {
    fn new(mori: &'a MoriPlan, workspace: &'a Path, plan_dir: &'a str) -> Self {
        let title = mori
            .plan_md
            .as_deref()
            .and_then(markdown_title)
            .unwrap_or_else(|| mori.id.clone());
        let mut lead_tail = String::new();
        for (file, what) in [
            (DECOMPOSITION_FILE, "The build steps are in"),
            ("prd-extract.md", "The PRD sections it cites are in"),
        ] {
            if mori.has_file(file) {
                let _ = write!(lead_tail, " {what} `{plan_dir}/{file}`.");
            }
        }
        let usable = |entry: &&Table| {
            str_of(entry, "command")
                .is_some_and(|command| !is_mori_internal(command) && !never_fails(command))
        };
        let (blocking, advisory): (Vec<&Table>, Vec<&Table>) = mori
            .verify
            .iter()
            .filter(usable)
            .partition(|entry| entry.get("blocking").and_then(Value::as_bool) != Some(false));
        let apps = mori
            .tasks
            .iter()
            .flat_map(|task| strings(task, "files"))
            .filter_map(|file| {
                let rest = file.trim_start_matches("./").strip_prefix("apps/")?;
                let name = rest.split('/').next()?;
                (!name.is_empty() && rest.len() > name.len()).then(|| name.to_string())
            })
            .collect();
        let report = MoriImportReport {
            plan: mori.id.clone(),
            source: mori.src.display().to_string(),
            mori_tasks: mori.tasks.len(),
            notes: mori.notes.clone(),
            ..MoriImportReport::default()
        };
        Self {
            mori,
            workspace,
            plan_dir,
            title,
            lead_tail,
            blocking,
            advisory,
            claimed: HashSet::new(),
            apps,
            dropped_hints: BTreeMap::new(),
            unknown_keys: Vec::new(),
            report,
        }
    }

    /// The directory of crate `name`: `apps/<name>` when mori's tasks or the
    /// workspace keep it there, else `crates/<name>`.
    fn crate_dir(&self, name: &str) -> String {
        if self.apps.contains(name) || self.workspace.join("apps").join(name).is_dir() {
            format!("apps/{name}")
        } else {
            format!("crates/{name}")
        }
    }

    /// The task one mori `[[task]]` becomes, with its checks; `None` for a
    /// task without an id.
    fn task_draft(&mut self, task: &Table, ids: &HashSet<String>) -> Option<Draft> {
        let Some(id) = str_of(task, "id")
            .map(str::trim)
            .filter(|id| !id.is_empty())
        else {
            self.report
                .notes
                .push("a [[task]] without an id was skipped".to_string());
            return None;
        };
        let id = id.to_string();
        if str_of(task, "status") == Some("done") {
            self.report.done_in_mori += 1;
        }
        for key in task.keys() {
            if let Some(hint) = DROPPED_HINTS.into_iter().find(|hint| *hint == key.as_str()) {
                *self.dropped_hints.entry(hint).or_default() += 1;
            } else if !KEPT_HINTS.contains(&key.as_str())
                && !MAPPED_KEYS.contains(&key.as_str())
                && !self.unknown_keys.contains(key)
            {
                self.unknown_keys.push(key.clone());
            }
        }
        let title = str_of(task, "title")
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .unwrap_or(&id)
            .to_string();
        let tier = match str_of(task, "complexity_band") {
            Some("fast") => TaskTier::Mechanical,
            Some("complex") => TaskTier::Integrative,
            _ => TaskTier::Focused,
        };
        let role = if str_of(task, "category") == Some("docs") {
            "scribe"
        } else {
            "implementer"
        };
        let mut draft = Draft::new(&id, &title, role, tier);
        draft.files = self.task_files(&id, task);
        (draft.depends_on, draft.depends_on_plan) = self.dependencies(&id, task, ids);
        draft.estimated_minutes = task
            .get("estimated_minutes")
            .and_then(Value::as_integer)
            .filter(|minutes| *minutes > 0);
        draft.hints = KEPT_HINTS
            .into_iter()
            .filter_map(|key| task.get(key).map(|value| (key.to_string(), value.clone())))
            .collect();
        draft.acceptance = strings(task, "acceptance");
        draft.lead = self.lead(task, &title);
        draft.context_files = strings(task, "context_files");
        let crate_name = crate_of(task, &draft.files);
        let crate_dir = crate_name.as_deref().map(|name| self.crate_dir(name));
        self.task_checks(
            &mut draft,
            task,
            crate_name.as_deref(),
            crate_dir.as_deref(),
        );

        // A task that writes must say what: its crate, at least.
        if draft.files.is_empty() && draft.role != "quick-reviewer" {
            if let Some(dir) = &crate_dir {
                draft.files.push(format!("{dir}/"));
                draft.comments.push(format!(
                    "mori import: mori named no file, so the task may change its crate, `{dir}/`"
                ));
            } else {
                draft.role = "quick-reviewer";
                draft.comments.push(
                    "mori import: mori named no file and no crate, so the task runs checks and \
                     writes nothing"
                        .to_string(),
                );
                self.report.notes.push(format!(
                    "{id}: mori named no file and no crate, so it is a quick-reviewer"
                ));
            }
        }
        Some(draft)
    }

    /// The task's files; a pattern becomes the directory it matches in.
    fn task_files(&mut self, id: &str, task: &Table) -> Vec<String> {
        let mut files: Vec<String> = Vec::new();
        for file in strings(task, "files") {
            let file = file.trim().trim_start_matches("./");
            if file.is_empty() {
                continue;
            }
            let path = if file.contains(['*', '?', '[']) {
                let Some(dir) = glob_dir(file) else {
                    self.report.notes.push(format!(
                        "{id}: the file pattern `{file}` names no directory, so it is dropped"
                    ));
                    continue;
                };
                dir
            } else {
                file.to_string()
            };
            if !files.contains(&path) {
                files.push(path);
            }
        }
        files
    }

    /// The task's dependencies inside the plan and on other plans. A
    /// cross-plan reference names a task (`08e:T9`); roko waits for that
    /// task's whole plan.
    fn dependencies(
        &mut self,
        id: &str,
        task: &Table,
        ids: &HashSet<String>,
    ) -> (Vec<String>, Vec<String>) {
        let mut depends_on: Vec<String> = Vec::new();
        let mut depends_on_plan: Vec<String> = Vec::new();
        for dependency in strings(task, "depends_on") {
            let dependency = dependency.trim().to_string();
            let local = match dependency.split_once(':') {
                Some((prefix, task_id)) => {
                    let plan = self.resolve_plan(prefix);
                    if plan.as_deref() == Some(self.mori.id.as_str()) {
                        Some(task_id.trim().to_string())
                    } else {
                        if let Some(plan) = &plan
                            && !depends_on_plan.contains(plan)
                        {
                            depends_on_plan.push(plan.clone());
                        }
                        self.report.cross_plan.push(CrossPlanDependency {
                            task: id.to_string(),
                            reference: dependency.clone(),
                            plan,
                        });
                        None
                    }
                }
                None => Some(dependency.clone()),
            };
            let Some(local) = local else {
                continue;
            };
            if !ids.contains(&local) {
                self.report.notes.push(format!(
                    "{id}: `depends_on` names `{dependency}`, which is no task of this plan, so \
                     it is dropped"
                ));
            } else if local != id && !depends_on.contains(&local) {
                depends_on.push(local);
            }
        }
        (depends_on, depends_on_plan)
    }

    /// The plan beside this one whose directory is `prefix` or starts with
    /// `<prefix>-`: `08e` names `08e-sonification-crate`.
    fn resolve_plan(&self, prefix: &str) -> Option<String> {
        let prefix = prefix.trim();
        if prefix.is_empty() {
            return None;
        }
        let dashed = format!("{prefix}-");
        self.mori
            .siblings
            .iter()
            .find(|name| *name == prefix)
            .or_else(|| {
                self.mori
                    .siblings
                    .iter()
                    .find(|name| name.starts_with(&dashed))
            })
            .cloned()
    }

    /// The start of a task's description.
    fn lead(&self, task: &Table, title: &str) -> String {
        let contract = "The acceptance criteria below are the contract; the verification \
                        commands decide.";
        if self.mori.plan_md.is_none() {
            return format!(
                "Part of {}. Implement this task: {title}. {contract}{}",
                self.title, self.lead_tail
            );
        }
        let section = str_of(task, "plan_section")
            .map(str::trim)
            .filter(|section| !section.is_empty())
            .unwrap_or("What to build");
        format!(
            "Part of {}. Implement the section '{section}' of the plan document `{}/plan.md` \
             for this task: {title}. {contract}{}",
            self.title, self.plan_dir, self.lead_tail
        )
    }

    /// The task's checks, in order: the blocking `verify-tasks.toml` entries
    /// its acceptance names, its acceptance commands, its own `command` and
    /// `test_fns`, its "File exists" lines, and `cargo check` when nothing
    /// else checks it.
    fn task_checks(
        &mut self,
        draft: &mut Draft,
        task: &Table,
        crate_name: Option<&str>,
        crate_dir: Option<&str>,
    ) {
        let acceptance = draft.acceptance.join("\n");
        let named: Vec<(usize, &Table)> = self
            .blocking
            .iter()
            .enumerate()
            .filter(|(_, entry)| names_entry(&acceptance, entry))
            .map(|(index, entry)| (index, *entry))
            .collect();
        for (index, entry) in named {
            self.claimed.insert(index);
            let command = str_of(entry, "command").unwrap_or_default();
            let tests = test_names(entry);
            let dir = self.guard_dir(entry, command, crate_dir);
            let phase = str_of(entry, "type").unwrap_or("test");
            if draft.add_check(
                phase,
                &guarded(command, &tests, &dir),
                &entry_message(entry),
            ) {
                self.report.checks.verify_tasks += 1;
            }
        }
        for command in acceptance_commands(&draft.acceptance) {
            let message = format!("Checkpoint from the acceptance criteria: {command}");
            if draft.add_check("checkpoint", &command, &message) {
                self.report.checks.acceptance += 1;
            }
        }
        if let Some(command) = str_of(task, "command").filter(|command| !is_mori_internal(command))
            && draft.add_check("command", command, "the mori task's own check")
        {
            self.report.checks.task_command += 1;
        }
        let tests: Vec<String> = strings(task, "test_fns")
            .iter()
            .filter_map(|name| test_name(name))
            .collect();
        if let (Some(name), Some(dir)) = (crate_name, crate_dir)
            && !tests.is_empty()
        {
            let command = format!("cargo test -p {name} -- {}", tests.join(" "));
            let message = format!("the tests the mori task names: {}", tests.join(", "));
            if draft.add_check("test", &guarded(&command, &tests, dir), &message) {
                self.report.checks.task_command += 1;
            }
        }
        let named_files: Vec<String> = draft
            .acceptance
            .iter()
            .filter_map(|line| {
                let path = FILE_EXISTS.captures(line.trim())?.get(1)?.as_str();
                let path = path.trim_start_matches("./");
                (!path.contains(['*', '?', '['])).then(|| path.to_string())
            })
            .collect();
        for path in named_files {
            let command = format!("test -f {}", shell_quote(&path));
            let exists = self.workspace.join(&path).is_file();
            if draft.add_file_check(&path, command, exists) {
                self.report.checks.file_exists += 1;
            }
        }
        if draft.step_count() == 0
            && let Some(name) = crate_name
            && draft.add_check(
                "compile",
                &format!("cargo check -p {name}"),
                &format!("{name} must compile"),
            )
        {
            self.report.checks.compile_fallback += 1;
        }
    }

    /// Where a check's test-name guard looks: the entry's crate, the task's,
    /// the crate its `-p` names, or the whole workspace.
    fn guard_dir(&self, entry: &Table, command: &str, crate_dir: Option<&str>) -> String {
        if let Some(name) = str_of(entry, "crate").filter(|name| !name.trim().is_empty()) {
            return self.crate_dir(name.trim());
        }
        if let Some(dir) = crate_dir {
            return dir.to_string();
        }
        let words: Vec<&str> = command.split_whitespace().collect();
        words
            .windows(2)
            .find(|pair| pair[0] == "-p")
            .map_or_else(|| ".".to_string(), |pair| self.crate_dir(pair[1]))
    }

    /// Notes on the hints and keys the import dropped, and on mori's run
    /// state.
    fn note_hints(&mut self) {
        let dropped: Vec<&str> = RUN_STATE
            .into_iter()
            .filter(|key| self.mori.meta.contains_key(*key))
            .collect();
        if !dropped.is_empty() {
            self.report.notes.push(format!(
                "[meta] run state is dropped (it is not part of the spec): {}",
                dropped.join(", ")
            ));
        }
        if self.report.done_in_mori > 0 {
            self.report.notes.push(format!(
                "mori marked {} of {} tasks done; they are imported as ready, so roko's checks \
                 decide (a task whose checks pass on an unchanged tree settles as already \
                 satisfied)",
                self.report.done_in_mori, self.report.mori_tasks
            ));
        }
        if !self.dropped_hints.is_empty() {
            let named: Vec<String> = self
                .dropped_hints
                .iter()
                .map(|(hint, count)| format!("{hint} ({count})"))
                .collect();
            self.report.notes.push(format!(
                "task hints roko does not act on are dropped: {}; tasks that write the same path \
                 wait for each other instead of exclusive_files",
                named.join(", ")
            ));
        }
        if !self.unknown_keys.is_empty() {
            self.report.notes.push(format!(
                "task keys the import does not know are dropped: {}",
                self.unknown_keys.join(", ")
            ));
        }
    }

    /// The whole-plan check: the blocking checks no task claimed, then
    /// `verify.sh`'s steps, then the rubric's commands.
    fn plan_checks(&mut self) -> Vec<Step> {
        let mut steps = Vec::new();
        let unclaimed: Vec<&Table> = self
            .blocking
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.claimed.contains(index))
            .map(|(_, entry)| *entry)
            .collect();
        for entry in unclaimed {
            let command = str_of(entry, "command").unwrap_or_default();
            let dir = self.guard_dir(entry, command, None);
            push_step(
                &mut steps,
                str_of(entry, "type").unwrap_or("verify"),
                &guarded(command, &test_names(entry), &dir),
                &entry_message(entry),
            );
        }
        if let Some(script) = &self.mori.verify_sh {
            let found = verify_sh_steps(script);
            if found.is_empty() {
                self.report.notes.push(
                    "verify.sh has no run_check step, so nothing in it is mapped".to_string(),
                );
            }
            let mut repaired = 0;
            for (what, command) in found {
                if is_mori_internal(&command) {
                    continue;
                }
                let fixed = PACKAGE_PATH.replace_all(&command, "$1$2").into_owned();
                repaired += usize::from(fixed != command);
                push_step(&mut steps, "verify.sh", &fixed, &what);
            }
            if repaired > 0 {
                self.report.notes.push(format!(
                    "verify.sh: {repaired} steps named a package by its path (`-p apps/x`); \
                     they name it by its name (`-p x`)"
                ));
            }
        }
        if let Some(rubric) = &self.mori.rubric {
            for command in BACKTICKED
                .captures_iter(rubric)
                .filter_map(|found| found.get(1))
                .map(|found| found.as_str().trim())
                .filter(|command| COMMAND_START.is_match(command) && !is_mori_internal(command))
            {
                let message = format!("Rubric: `{command}` must pass");
                push_step(&mut steps, "rubric", command, &message);
            }
        }
        steps
    }

    /// Give each check task (a quick-reviewer from a mori task that names no
    /// file) that has no check of its own the checks of the tasks it waits
    /// for, so it checks their work. One that waits for no task with a check
    /// is left out: the whole-plan check, which runs once every task is done,
    /// is what is left for it to check. The tasks that waited for it wait for
    /// what it waited for.
    fn fill_check_tasks(&mut self, drafts: &mut Vec<Draft>, plan_checks: &[Step]) {
        let limit = tier_limits(TOP_TIER).max_verify_steps;
        let mut dropped: HashMap<String, Vec<String>> = HashMap::new();
        for index in 0..drafts.len() {
            if drafts[index].role != "quick-reviewer" || drafts[index].step_count() > 0 {
                continue;
            }
            let ancestors = ancestor_ids(drafts, index);
            let mut steps: Vec<Step> = Vec::new();
            for structural in [false, true] {
                for draft in drafts.iter().filter(|draft| ancestors.contains(&draft.id)) {
                    for step in draft.steps() {
                        if (step.phase == "structural") == structural {
                            push_owned_step(&mut steps, step);
                        }
                    }
                }
            }
            steps.truncate(limit);
            let draft = &mut drafts[index];
            if steps.is_empty() {
                dropped.insert(draft.id.clone(), draft.depends_on.clone());
                continue;
            }
            draft.comments.push(
                "mori import: mori gave this check task no command, so it runs the checks of \
                 the tasks it waits for"
                    .to_string(),
            );
            draft.checks = steps;
        }
        if dropped.is_empty() {
            return;
        }
        drafts.retain(|draft| !dropped.contains_key(&draft.id));
        for draft in drafts.iter_mut() {
            draft.depends_on = rewire(&draft.depends_on, &dropped);
        }
        let covered = if plan_checks.is_empty() {
            "the plan has no check for it to run"
        } else {
            "the whole-plan check covers it"
        };
        let mut ids: Vec<&String> = dropped.keys().collect();
        ids.sort();
        for id in ids {
            self.report.dropped_tasks.push(id.clone());
            self.report.notes.push(format!(
                "{id} is left out: it names no file, no crate and no check, and waits for no \
                 task with a check; {covered}"
            ));
        }
    }

    /// Fit each task to the size limits of a tier: narrow and split it when
    /// it has more files than the top tier takes, check its files when
    /// nothing else checks it, then raise its tier as far as the top tier.
    fn fit_sizes(
        &mut self,
        drafts: Vec<Draft>,
        plan_checks: &mut Vec<Step>,
        taken: &HashSet<String>,
    ) -> Vec<Draft> {
        let limit = tier_limits(TOP_TIER).max_files;
        let mut fitted = Vec::new();
        let mut renames: HashMap<String, String> = HashMap::new();
        for mut draft in drafts {
            if draft.files.len() > limit {
                self.narrow(&mut draft, limit);
            }
            let parts = if draft.files.len() > limit {
                self.split(draft, limit, taken, &mut renames)
            } else {
                vec![draft]
            };
            for mut part in parts {
                if part.step_count() == 0 && !part.files.is_empty() {
                    part.add_structural_checks(self.workspace);
                    if !part.leading_part {
                        self.report.structural_only.push(part.id.clone());
                    }
                }
                self.fit_tier(&mut part, plan_checks);
                fitted.push(part);
            }
        }
        for draft in &mut fitted {
            for dependency in &mut draft.depends_on {
                if let Some(last) = renames.get(dependency) {
                    dependency.clone_from(last);
                }
            }
        }
        fitted
    }

    /// Narrow a task's files to the directories that hold them, or to their
    /// crates' (or top) directories when those are still too many. The full
    /// list goes beside the plan for the task to read.
    fn narrow(&mut self, draft: &mut Draft, limit: usize) {
        let original = std::mem::take(&mut draft.files);
        let parents = collapse(original.iter().map(|file| parent_dir(file)));
        let narrowed = if parents.len() <= limit {
            parents
        } else {
            collapse(original.iter().map(|file| top_dir(file)))
        };
        let name = format!("mori-files-{}.txt", file_safe(&draft.id));
        let mut list = original.join("\n");
        list.push('\n');
        draft.read_files.push((
            format!("{}/{name}", self.plan_dir),
            format!("the {} files mori named for this task", original.len()),
        ));
        draft.comments.push(format!(
            "mori import: {} files narrowed to {} directories; the full list is in {name}",
            original.len(),
            narrowed.len()
        ));
        draft.file_list = Some((name, list));
        self.report.narrowed.push(NarrowedFiles {
            task: draft.id.clone(),
            files: original.len(),
            to: narrowed.clone(),
        });
        draft.files = narrowed;
    }

    /// Split a task into parts of at most `limit` files each. Every part
    /// keeps the task's acceptance; the last runs its checks and waits for
    /// the others, and the tasks that waited for the task wait for it.
    fn split(
        &mut self,
        draft: Draft,
        limit: usize,
        taken: &HashSet<String>,
        renames: &mut HashMap<String, String>,
    ) -> Vec<Draft> {
        let chunks: Vec<Vec<String>> = draft.files.chunks(limit).map(<[String]>::to_vec).collect();
        let count = chunks.len();
        let ids: Vec<String> = (0..count)
            .map(|index| part_id(&draft.id, index, taken))
            .collect();
        let why = format!(
            "{} directories, and {TOP_TIER} tasks take at most {limit}",
            draft.files.len()
        );
        let mut parts = Vec::new();
        for (index, files) in chunks.into_iter().enumerate() {
            let last = index + 1 == count;
            let mut part = draft.clone();
            part.id.clone_from(&ids[index]);
            part.title = format!("{} (part {} of {count})", draft.title, index + 1);
            part.files = files;
            if last {
                part.depends_on.extend(ids[..index].iter().cloned());
            } else {
                part.checks.clear();
                part.file_checks.clear();
                part.leading_part = true;
            }
            part.lead = format!(
                "Part {} of {count} of the mori task {}: this part covers the files listed for \
                 it{}. {}",
                index + 1,
                draft.id,
                if last {
                    ", once the other parts are done, and runs the task's checks"
                } else {
                    ""
                },
                draft.lead
            );
            part.comments.push(format!(
                "mori import: part {} of {count} of {} ({why})",
                index + 1,
                draft.id
            ));
            parts.push(part);
        }
        renames.insert(draft.id.clone(), ids[count - 1].clone());
        self.report.splits.push(TaskSplit {
            task: draft.id,
            parts: ids,
            why,
        });
        parts
    }

    /// Raise `draft` to the smallest tier at or above its own that it fits,
    /// as far as the top tier. Checks past the top tier's limit go to the
    /// whole-plan check, and a description past it is cut.
    fn fit_tier(&mut self, draft: &mut Draft, plan_checks: &mut Vec<Step>) {
        let start = draft.tier;
        let why = over_limit(draft, start);
        if draft.step_count() > tier_limits(TOP_TIER).max_verify_steps {
            let joined = join_cargo_tests(&mut draft.checks);
            if joined > 0 {
                self.report.checks.joined += joined;
                draft.comments.push(format!(
                    "mori import: {joined} `cargo test` checks joined into the run of their \
                     package, to fit {TOP_TIER}'s {} checks",
                    tier_limits(TOP_TIER).max_verify_steps
                ));
            }
        }
        let tier = TaskTier::ALL
            .into_iter()
            .filter(|tier| *tier >= start && *tier <= TOP_TIER)
            .find(|tier| over_limit(draft, *tier).is_none())
            .unwrap_or(TOP_TIER.max(start));
        let limits = tier_limits(tier);
        while draft.step_count() > limits.max_verify_steps {
            let Some(step) = draft.checks.pop() else {
                break;
            };
            self.report.checks.moved_to_plan += 1;
            draft.comments.push(format!(
                "mori import: a check moved to the whole-plan check, since {tier} tasks take {} \
                 checks: {}",
                limits.max_verify_steps, step.command
            ));
            push_owned_step(plan_checks, step);
        }
        if words(&draft.lead) > limits.max_description_words {
            draft.lead = cut_words(&draft.lead, limits.max_description_words);
        }
        if tier != start
            && let Some(why) = why
        {
            draft.comments.push(format!(
                "mori import: tier raised from {start} to {tier}: {why}"
            ));
            self.report.tier_changes.push(TierChange {
                task: draft.id.clone(),
                from: start.label().to_string(),
                to: tier.label().to_string(),
                why,
            });
        }
        draft.tier = tier;
    }

    /// mori serialized tasks that write the same path at run time
    /// (`exclusive_files`, a lock). roko needs that order in the plan, so
    /// each later writer waits for the earlier one, in definition order.
    fn serialize_shared_writers(&mut self, drafts: &mut [Draft]) {
        for later in 0..drafts.len() {
            for earlier in 0..later {
                let shared = drafts[earlier]
                    .files
                    .iter()
                    .any(|a| drafts[later].files.iter().any(|b| paths_overlap(a, b)));
                if !shared {
                    continue;
                }
                let earlier_id = drafts[earlier].id.clone();
                let later_id = drafts[later].id.clone();
                if waits_for(drafts, &later_id, &earlier_id)
                    || waits_for(drafts, &earlier_id, &later_id)
                {
                    continue;
                }
                drafts[later].depends_on.push(earlier_id.clone());
                self.report.serialized.push([later_id, earlier_id]);
            }
        }
    }

    /// Each task's read files and description: the plan document, then the
    /// context files that exist or that a task it waits for writes; the
    /// rest are named in the description.
    fn add_context(&self, drafts: &mut [Draft]) {
        for index in 0..drafts.len() {
            let produced = ancestor_outputs(drafts, index);
            let draft = &mut drafts[index];
            let mut read: Vec<(String, String)> = Vec::new();
            if self.mori.plan_md.is_some() {
                read.push((
                    format!("{}/plan.md", self.plan_dir),
                    "the plan document this task implements".to_string(),
                ));
            }
            read.append(&mut draft.read_files);
            let mut missing: Vec<String> = Vec::new();
            for file in &draft.context_files {
                if read.iter().any(|(path, _)| path == file) || missing.contains(file) {
                    continue;
                }
                let path = self.workspace.join(file);
                let ready = produced.contains(file)
                    || (path.is_file() && fs::read_to_string(&path).is_ok());
                if ready && read.len() < MAX_READ_FILES {
                    read.push((
                        file.clone(),
                        "context file named by the mori task".to_string(),
                    ));
                } else {
                    missing.push(file.clone());
                }
            }
            draft.read_files = read;
            let budget = tier_limits(draft.tier).max_description_words;
            draft.lead = describe(&draft.lead, &missing, budget);
        }
    }

    /// One quick-reviewer task from `review-tasks.toml`, after the plan's
    /// other tasks: its checks to review against and mori's non-blocking
    /// checks in its description, and the plan's blocking checks as its own,
    /// so it reviews finished work. `None` when there is nothing to review
    /// or nothing to check.
    fn review_task(
        &mut self,
        drafts: &[Draft],
        plan_checks: &[Step],
        leaves: &[String],
        taken: &HashSet<String>,
    ) -> Option<Draft> {
        let mori = self.mori;
        let entries = &mori.review;
        if entries.is_empty() {
            return None;
        }
        let tier = TOP_TIER;
        let limits = tier_limits(tier);
        let mut checks: Vec<Step> = Vec::new();
        for step in drafts
            .iter()
            .flat_map(Draft::steps)
            .filter(|step| step.phase != "structural")
            .chain(plan_checks.iter().take(2).cloned())
        {
            if checks.len() >= limits.max_verify_steps {
                break;
            }
            push_owned_step(&mut checks, step);
        }
        if checks.is_empty() {
            for step in drafts.iter().flat_map(Draft::steps) {
                if checks.len() >= limits.max_verify_steps {
                    break;
                }
                push_owned_step(&mut checks, step);
            }
        }
        if checks.is_empty() {
            self.report.notes.push(format!(
                "review-tasks.toml ({} checks) has no reviewer task: the plan has no check for \
                 one to run",
                entries.len()
            ));
            return None;
        }
        let has_rubric = mori.has_file(RUBRIC_FILE);
        let mut text = format!(
            "Review {}'s finished work against its rubric{} and these checks. Answer with a \
             verdict, PASS, REVISE or BLOCK, and for each failed check the file and line that \
             fails it.",
            self.mori.id,
            if has_rubric {
                format!(" (`{}/{RUBRIC_FILE}`)", self.plan_dir)
            } else {
                String::new()
            }
        );
        let budget = limits.max_description_words;
        // Keep room for the advisory checks' heading and the pointers.
        let review_budget = budget.saturating_sub(60);
        let mut listed = 0;
        for entry in entries {
            let mut block = format!(
                "\n- [{}] {}",
                str_of(entry, "severity").unwrap_or("major"),
                one_line(
                    str_of(entry, "title")
                        .or_else(|| str_of(entry, "id"))
                        .unwrap_or("check")
                )
            );
            for check in strings(entry, "check")
                .into_iter()
                .chain(strings(entry, "checks"))
                .take(3)
            {
                let _ = write!(block, "\n  - {}", one_line(&check));
            }
            if words(&text) + words(&block) > review_budget {
                break;
            }
            text.push_str(&block);
            listed += 1;
        }
        if listed < entries.len() {
            let _ = write!(
                text,
                "\n- And {} more in `{}/review-tasks.toml`.",
                entries.len() - listed,
                self.plan_dir
            );
        }
        let advisory: Vec<String> = self
            .advisory
            .iter()
            .filter_map(|entry| {
                let command = str_of(entry, "command")?;
                Some(format!("{}: `{}`", entry_message(entry), command.trim()))
            })
            .collect();
        self.report.advisory_checks = advisory.len();
        if !advisory.is_empty() {
            text.push_str(
                "\n\nmori's non-blocking checks, as advice (run what you can and report; do not \
                 block on them):",
            );
            let mut shown = 0;
            for line in &advisory {
                let line = format!("\n- {}", one_line(line));
                if words(&text) + words(&line) + 12 > budget {
                    break;
                }
                text.push_str(&line);
                shown += 1;
            }
            if shown < advisory.len() {
                let _ = write!(
                    text,
                    "\n- And {} more in `{}/verify-tasks.toml`.",
                    advisory.len() - shown,
                    self.plan_dir
                );
            }
        }
        let mut draft = Draft::new(
            &unique_id("REVIEW", taken),
            &format!(
                "Review {} against its rubric and review checks",
                self.mori.id
            ),
            "quick-reviewer",
            tier,
        );
        draft.depends_on = leaves.to_vec();
        draft.lead = text;
        if self.mori.plan_md.is_some() {
            draft.read_files.push((
                format!("{}/plan.md", self.plan_dir),
                "the plan document under review".to_string(),
            ));
        }
        if has_rubric {
            draft.read_files.push((
                format!("{}/{RUBRIC_FILE}", self.plan_dir),
                "the shared review rubric".to_string(),
            ));
        }
        draft.read_files.push((
            format!("{}/review-tasks.toml", self.plan_dir),
            "every review check".to_string(),
        ));
        draft.checks = checks;
        Some(draft)
    }

    /// Scribe tasks from `scribe-tasks.toml`: the module documents it names
    /// and its diagrams, in `docs/<plan>/`, at most the top tier's file limit
    /// per task, each checked by file, heading and diagram count.
    fn docs_tasks(&mut self, leaves: &[String], taken: &HashSet<String>) -> Vec<Draft> {
        let mori = self.mori;
        let plan = mori.id.clone();
        let mut outputs: Vec<(String, String, FileCheck)> = Vec::new();
        let mut diagrams: Vec<String> = Vec::new();
        let mut unmapped: BTreeMap<String, usize> = BTreeMap::new();
        for entry in &mori.scribe {
            if let Some(output) = str_of(entry, "output_file").map(str::trim)
                && !output.is_empty()
            {
                let name = output.rsplit('/').next().unwrap_or(output);
                let path = format!("docs/{plan}/{name}");
                if outputs.iter().any(|(known, _, _)| *known == path) {
                    continue;
                }
                let sections = strings(entry, "sections");
                let line = format!(
                    "- `{path}`: {}{}",
                    one_line(str_of(entry, "title").unwrap_or_default()),
                    if sections.is_empty() {
                        String::new()
                    } else {
                        format!("; sections: {}", sections.join(", "))
                    }
                );
                let mut command = format!("test -s {}", shell_quote(&path));
                for section in sections.iter().take(MAX_SECTIONS) {
                    let heading = heading_pattern(section);
                    if !heading.is_empty() {
                        let _ = write!(
                            command,
                            " && grep -qi {} {}",
                            shell_quote(&format!("^#.*{heading}")),
                            shell_quote(&path)
                        );
                    }
                }
                let exists = self.workspace.join(&path).is_file();
                let check = FileCheck {
                    file: path.clone(),
                    command,
                    exists,
                };
                outputs.push((path, line, check));
            } else if matches!(str_of(entry, "type"), Some("diagram" | "interaction")) {
                let concept = str_of(entry, "concept")
                    .map(|concept| format!(" ({})", cut_words(&one_line(concept), CONCEPT_WORDS)))
                    .unwrap_or_default();
                diagrams.push(format!(
                    "- a ```mermaid {} block: {}{concept}",
                    str_of(entry, "diagram_type").unwrap_or("flowchart"),
                    one_line(str_of(entry, "title").unwrap_or_default())
                ));
            } else {
                let kind = str_of(entry, "type").unwrap_or("untyped").to_string();
                *unmapped.entry(kind).or_default() += 1;
            }
        }
        if !unmapped.is_empty() {
            let named: Vec<String> = unmapped
                .iter()
                .map(|(kind, count)| format!("{count} {kind}"))
                .collect();
            self.report.notes.push(format!(
                "scribe-tasks.toml: {} entries name no output file and are not tasks; the docs \
                 tasks can read them in the copied scribe-tasks.toml",
                named.join(", ")
            ));
        }
        if outputs.is_empty() && diagrams.is_empty() {
            return Vec::new();
        }
        let limits = tier_limits(TOP_TIER);
        let diagram_file = format!("docs/{plan}/diagrams.md");
        // The diagrams file counts against the last part's files.
        let mut groups: Vec<Vec<(String, String, FileCheck)>> = Vec::new();
        for output in outputs {
            match groups.last_mut() {
                Some(group) if group.len() < limits.max_files => group.push(output),
                _ => groups.push(vec![output]),
            }
        }
        if !diagrams.is_empty()
            && groups
                .last()
                .is_none_or(|group| group.len() >= limits.max_files)
        {
            groups.push(Vec::new());
        }
        let count = groups.len();
        let mut tasks = Vec::new();
        for (index, group) in groups.into_iter().enumerate() {
            let last = index + 1 == count;
            let id = if count == 1 {
                unique_id("DOCS", taken)
            } else {
                unique_id(&format!("DOCS{}", index + 1), taken)
            };
            let title = if count == 1 {
                format!("Document {plan}")
            } else {
                format!("Document {plan} (part {} of {count})", index + 1)
            };
            let mut draft = Draft::new(&id, &title, "scribe", TaskTier::Mechanical);
            draft.file_phase = "doc";
            draft.file_fail = "these documents must exist with their sections and diagrams";
            draft.depends_on = leaves.to_vec();
            let mut text = format!(
                "Document what {plan} built, from the code the earlier tasks wrote. Write:"
            );
            let mut lines: Vec<String> = Vec::new();
            for (path, line, check) in group {
                lines.push(line);
                draft.files.push(path);
                draft.file_checks.push(check);
            }
            if last && !diagrams.is_empty() {
                lines.push(format!(
                    "- in `{diagram_file}`, one block for each of these diagrams:"
                ));
                lines.extend(diagrams.iter().cloned());
                draft.files.push(diagram_file.clone());
                draft.file_checks.push(FileCheck {
                    file: diagram_file.clone(),
                    command: format!(
                        "test \"$(grep -c '^```mermaid' {})\" -ge {}",
                        shell_quote(&diagram_file),
                        diagrams.len()
                    ),
                    exists: self.workspace.join(&diagram_file).is_file(),
                });
            }
            let total = lines.len();
            let mut shown = 0;
            for line in &lines {
                if words(&text) + words(line) + 12 > limits.max_description_words {
                    break;
                }
                text.push('\n');
                text.push_str(line);
                shown += 1;
            }
            if shown < total {
                let _ = write!(
                    text,
                    "\n- And {} more lines of this list in `{}/scribe-tasks.toml`.",
                    total - shown,
                    self.plan_dir
                );
            }
            draft.lead = text;
            if self.mori.plan_md.is_some() {
                draft.read_files.push((
                    format!("{}/plan.md", self.plan_dir),
                    "what was meant to be built".to_string(),
                ));
            }
            draft.read_files.push((
                format!("{}/scribe-tasks.toml", self.plan_dir),
                "every docs entry, with its requirements".to_string(),
            ));
            tasks.push(draft);
        }
        tasks
    }

    /// The files written beside `tasks.toml`.
    fn companions(
        &mut self,
        src: &Path,
        drafts: &[Draft],
        has_reviewer: bool,
        has_docs: bool,
    ) -> Vec<(String, String)> {
        let mut companions: Vec<(String, String)> = Vec::new();
        if let Some(text) = &self.mori.plan_md {
            companions.push(("plan.md".to_string(), strip_reuse_appendix(text)));
        } else {
            self.report
                .notes
                .push("no plan.md: the tasks have only their titles and acceptance".to_string());
        }
        let mut copied: Vec<&str> = REFERENCES.to_vec();
        if has_reviewer {
            copied.push("review-tasks.toml");
        }
        if has_docs {
            copied.push("scribe-tasks.toml");
        }
        if !self.mori.verify.is_empty() {
            copied.push("verify-tasks.toml");
        }
        for name in copied {
            if let Ok(text) = fs::read_to_string(src.join(name)) {
                companions.push((name.to_string(), text));
            }
        }
        for draft in drafts {
            if let Some(list) = &draft.file_list
                && !companions.iter().any(|(name, _)| *name == list.0)
            {
                companions.push(list.clone());
            }
        }
        let unmapped = unmapped_files(src);
        if !unmapped.is_empty() {
            self.report.notes.push(format!(
                "not imported (mori's run-time and generation files; roko writes its own \
                 brief.md): {}",
                unmapped.join(", ")
            ));
        }
        companions
    }

    /// The plan's `[meta]`. mori's run state is left out.
    fn meta(&self, drafts: &[Draft], plan_checks: &[Step]) -> Vec<(String, Value)> {
        let total = i64::try_from(drafts.len()).unwrap_or(i64::MAX);
        let mut meta = vec![
            ("plan".to_string(), Value::String(self.mori.id.clone())),
            ("title".to_string(), Value::String(self.title.clone())),
            ("status".to_string(), Value::String("ready".to_string())),
            ("total".to_string(), Value::Integer(total)),
            ("done".to_string(), Value::Integer(0)),
            ("skip_enrichment".to_string(), Value::Boolean(true)),
        ];
        if let Some(minutes) = self
            .mori
            .meta
            .get("estimated_total_minutes")
            .and_then(Value::as_integer)
            .filter(|minutes| *minutes > 0)
        {
            meta.push((
                "estimated_total_minutes".to_string(),
                Value::Integer(minutes),
            ));
        }
        if let Some(parallel) = self
            .mori
            .meta
            .get("max_parallel")
            .and_then(Value::as_integer)
            .filter(|parallel| *parallel > 0)
        {
            meta.push((
                "max_parallel".to_string(),
                Value::Integer(parallel.min(total.max(1))),
            ));
        }
        // mori gated each task on its own checks and the plan on its chain;
        // a workspace rung that runs the whole suite after each task would
        // hold the other tasks' unwritten work against it.
        let implementers_checked = drafts
            .iter()
            .filter(|draft| draft.role == "implementer")
            .all(|draft| draft.step_count() > 0);
        if !plan_checks.is_empty() && implementers_checked {
            meta.push(("workspace_rungs".to_string(), Value::Boolean(false)));
        }
        meta
    }
}

// ---------------------------------------------------------------------------
// Sizes.

/// The size limits `plan validate` holds a task of `tier` to.
fn tier_limits(tier: TaskTier) -> TierSizeLimits {
    PlanExecutionPolicy::for_environment().tier_size_limits(tier)
}

/// The first limit of `tier` that `draft` is over, in words.
fn over_limit(draft: &Draft, tier: TaskTier) -> Option<String> {
    let limits = tier_limits(tier);
    if draft.files.len() > limits.max_files {
        return Some(format!(
            "{} files ({tier} limit {})",
            draft.files.len(),
            limits.max_files
        ));
    }
    let steps = draft.step_count();
    if steps > limits.max_verify_steps {
        return Some(format!(
            "{steps} checks ({tier} limit {})",
            limits.max_verify_steps
        ));
    }
    let words = words(&draft.lead);
    if words > limits.max_description_words {
        return Some(format!(
            "{words}-word description ({tier} limit {})",
            limits.max_description_words
        ));
    }
    None
}

/// The id of part `index` of task `id`: `T7a`, `T7b`… unless that is taken.
fn part_id(id: &str, index: usize, taken: &HashSet<String>) -> String {
    let candidate = match u8::try_from(index) {
        Ok(index) if index < 26 => format!("{id}{}", char::from(b'a' + index)),
        _ => format!("{id}-p{}", index + 1),
    };
    if taken.contains(&candidate) {
        unique_id(&format!("{id}-part{}", index + 1), taken)
    } else {
        candidate
    }
}

/// `base`, or `base2`, `base3`… when that is taken.
fn unique_id(base: &str, taken: &HashSet<String>) -> String {
    let mut id = base.to_string();
    let mut n = 2;
    while taken.contains(&id) {
        id = format!("{base}{n}");
        n += 1;
    }
    id
}

/// The directory that holds `path` (a directory stays itself).
fn parent_dir(path: &str) -> String {
    if path.ends_with('/') {
        return path.to_string();
    }
    path.rfind('/')
        .map_or_else(|| path.to_string(), |at| path[..=at].to_string())
}

/// The crate directory that holds `path` (`crates/<x>/`, `apps/<x>/`), or
/// its top directory.
fn top_dir(path: &str) -> String {
    let mut parts = path.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(base @ ("crates" | "apps")), Some(name), Some(_)) if !name.is_empty() => {
            format!("{base}/{name}/")
        }
        (Some(first), Some(_), _) if !first.is_empty() => format!("{first}/"),
        _ => path.to_string(),
    }
}

/// `paths` without repeats and without paths a directory among them holds.
fn collapse(paths: impl Iterator<Item = String>) -> Vec<String> {
    let mut unique: Vec<String> = Vec::new();
    for path in paths {
        if !unique.contains(&path) {
            unique.push(path);
        }
    }
    let all = unique.clone();
    unique.retain(|path| {
        !all.iter()
            .any(|other| other != path && other.ends_with('/') && path.starts_with(other.as_str()))
    });
    unique
}

/// `test -d` for a directory, `test -e` for anything else, and whether it
/// passes in `workspace` already.
fn exists_check(path: &str, workspace: &Path) -> (String, bool) {
    if let Some(dir) = path.strip_suffix('/') {
        (
            format!("test -d {}", shell_quote(dir)),
            workspace.join(dir).is_dir(),
        )
    } else {
        (
            format!("test -e {}", shell_quote(path)),
            workspace.join(path).exists(),
        )
    }
}

/// The description: `lead`, then the context files that do not exist yet,
/// as many as `budget` words leave room for.
fn describe(lead: &str, missing: &[String], budget: usize) -> String {
    let mut text = lead.to_string();
    if missing.is_empty() {
        return text;
    }
    let intro = "Read these files first if they exist when you start (earlier tasks may create \
                 them):";
    // Room for the intro and an "and N more".
    let mut room = budget.saturating_sub(words(lead) + words(intro) + 3);
    let mut named: Vec<String> = Vec::new();
    for file in missing {
        if room == 0 {
            break;
        }
        named.push(format!("`{file}`"));
        room -= 1;
    }
    if named.is_empty() {
        return text;
    }
    let _ = write!(text, "\n\n{intro} {}", named.join(", "));
    if named.len() < missing.len() {
        let _ = write!(text, ", and {} more", missing.len() - named.len());
    }
    text.push('.');
    text
}

fn words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// The first `max` words of `text`, with `…` when it had more.
fn cut_words(text: &str, max: usize) -> String {
    if words(text) <= max {
        return text.to_string();
    }
    let mut kept: Vec<&str> = text
        .split_whitespace()
        .take(max.saturating_sub(1))
        .collect();
    kept.push("…");
    kept.join(" ")
}

// ---------------------------------------------------------------------------
// The plan's DAG.

/// The ids of the tasks `drafts[index]` waits for, directly or not.
fn ancestor_ids(drafts: &[Draft], index: usize) -> HashSet<String> {
    let by_id: HashMap<&str, &Draft> = drafts
        .iter()
        .map(|draft| (draft.id.as_str(), draft))
        .collect();
    let mut seen: HashSet<String> = HashSet::new();
    let mut pending: Vec<&str> = drafts[index]
        .depends_on
        .iter()
        .map(String::as_str)
        .collect();
    while let Some(id) = pending.pop() {
        if !seen.insert(id.to_string()) {
            continue;
        }
        if let Some(draft) = by_id.get(id) {
            pending.extend(draft.depends_on.iter().map(String::as_str));
        }
    }
    seen
}

/// The paths the tasks `drafts[index]` waits for write.
fn ancestor_outputs(drafts: &[Draft], index: usize) -> HashSet<String> {
    let ancestors = ancestor_ids(drafts, index);
    drafts
        .iter()
        .filter(|draft| ancestors.contains(&draft.id))
        .flat_map(|draft| draft.files.iter().cloned())
        .collect()
}

/// Whether task `id` waits for task `other`, directly or not.
fn waits_for(drafts: &[Draft], id: &str, other: &str) -> bool {
    drafts
        .iter()
        .position(|draft| draft.id == id)
        .is_some_and(|index| ancestor_ids(drafts, index).contains(other))
}

/// `depends_on` with each left-out task replaced by what it waited for.
fn rewire(depends_on: &[String], dropped: &HashMap<String, Vec<String>>) -> Vec<String> {
    let mut rewired: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut pending: Vec<String> = depends_on.to_vec();
    while !pending.is_empty() {
        let dependency = pending.remove(0);
        if !seen.insert(dependency.clone()) {
            continue;
        }
        match dropped.get(&dependency) {
            Some(next) => pending.extend(next.iter().cloned()),
            None if !rewired.contains(&dependency) => rewired.push(dependency),
            None => {}
        }
    }
    rewired
}

/// Whether two task paths overlap: the same path, or a directory and a path
/// inside it (the Graph engine's rule).
fn paths_overlap(a: &str, b: &str) -> bool {
    let (a, b) = (a.trim_end_matches('/'), b.trim_end_matches('/'));
    a == b
        || b.strip_prefix(a).is_some_and(|rest| rest.starts_with('/'))
        || a.strip_prefix(b).is_some_and(|rest| rest.starts_with('/'))
}

// ---------------------------------------------------------------------------
// Reading mori's text.

fn str_of<'a>(table: &'a Table, key: &str) -> Option<&'a str> {
    table.get(key).and_then(Value::as_str)
}

fn strings(table: &Table, key: &str) -> Vec<String> {
    match table.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        Some(Value::String(item)) => vec![item.clone()],
        _ => Vec::new(),
    }
}

/// The first `# ` heading of a markdown document.
fn markdown_title(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(ToOwned::to_owned)
}

/// `text` on one line.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The plan document without the "Mori Reuse Appendix" mori generated.
fn strip_reuse_appendix(text: &str) -> String {
    let body = text
        .split("<!-- mori:generated-reuse begin -->")
        .next()
        .unwrap_or_default();
    format!("{}\n", body.trim_end())
}

/// The files of the mori plan directory the import neither reads nor copies.
fn unmapped_files(src: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(src) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_string();
            let dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
            Some(if dir { format!("{name}/") } else { name })
        })
        .filter(|name| !KNOWN_FILES.contains(&name.as_str()))
        .collect();
    names.sort();
    names
}

/// The crate a mori task works in: its `crate:<name>` tag, else the first
/// `crates/<name>/` or `apps/<name>/` path it writes.
fn crate_of(task: &Table, files: &[String]) -> Option<String> {
    strings(task, "tags")
        .into_iter()
        .find_map(|tag| {
            tag.strip_prefix("crate:")
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            files.iter().find_map(|file| {
                let mut parts = file.split('/');
                match (parts.next(), parts.next(), parts.next()) {
                    (Some("crates" | "apps"), Some(name), Some(_)) if !name.is_empty() => {
                        Some(name.to_string())
                    }
                    _ => None,
                }
            })
        })
}

/// The directory a file pattern matches in, with a trailing `/`:
/// `crates/x/src/*.rs` matches in `crates/x/src/`. `None` when the pattern
/// starts with a wildcard.
fn glob_dir(pattern: &str) -> Option<String> {
    let literal = &pattern[..pattern.find(['*', '?', '[']).unwrap_or(pattern.len())];
    let dir = &literal[..=literal.rfind('/')?];
    (dir != "/").then(|| dir.to_string())
}

/// A test function's name: the last segment of a `::` path, when that is
/// an identifier other than `tests`.
fn test_name(name: &str) -> Option<String> {
    let last = name.rsplit("::").next().unwrap_or_default().trim();
    (TEST_NAME.is_match(last) && last != "tests").then(|| last.to_string())
}

/// The names of the tests a `verify-tasks.toml` entry runs: its `test_fns`
/// and `test_fn`, else the `test_…` filter of its `cargo test` command.
fn test_names(entry: &Table) -> Vec<String> {
    let mut names = strings(entry, "test_fns");
    names.extend(strings(entry, "test_fn"));
    if names.is_empty()
        && let Some(command) = str_of(entry, "command")
        && command.trim_start().starts_with("cargo test")
        && let Some(last) = command
            .split(" -- ")
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .last()
        && last.starts_with("test_")
    {
        names.push(last.to_string());
    }
    let mut tests: Vec<String> = Vec::new();
    for name in names.iter().filter_map(|name| test_name(name)) {
        if !tests.contains(&name) {
            tests.push(name);
        }
    }
    tests
}

/// Whether a task's acceptance names a `verify-tasks.toml` entry: every
/// test it runs, or its invariant.
fn names_entry(acceptance: &str, entry: &Table) -> bool {
    let tests = test_names(entry);
    let by_tests = !tests.is_empty() && tests.iter().all(|test| contains_word(acceptance, test));
    let by_invariant = str_of(entry, "invariant")
        .map(str::trim)
        .is_some_and(|invariant| !invariant.is_empty() && contains_word(acceptance, invariant));
    by_tests || by_invariant
}

/// The message a `verify-tasks.toml` check fails with.
fn entry_message(entry: &Table) -> String {
    let title = one_line(str_of(entry, "title").unwrap_or_default());
    let invariant = str_of(entry, "invariant")
        .map(str::trim)
        .filter(|invariant| !invariant.is_empty());
    match (invariant, title.is_empty()) {
        (Some(invariant), false) => format!("{invariant}: {title}"),
        (_, false) => title,
        _ => str_of(entry, "id").unwrap_or("check").to_string(),
    }
}

/// Backticked commands from acceptance lines that say they must pass.
fn acceptance_commands(lines: &[String]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for line in lines {
        if !(line.to_lowercase().contains("checkpoint") || EXITS_ZERO.is_match(line)) {
            continue;
        }
        for command in BACKTICKED
            .captures_iter(line)
            .filter_map(|found| found.get(1))
            .map(|found| found.as_str().trim())
        {
            if COMMAND_START.is_match(command)
                && !is_mori_internal(command)
                && !found.iter().any(|known| known == command)
            {
                found.push(command.to_string());
            }
        }
    }
    found
}

/// `verify.sh`'s `run_check "<step>" "<what>" <command…>` steps, as `(what,
/// command)`.
fn verify_sh_steps(script: &str) -> Vec<(String, String)> {
    script
        .lines()
        .filter_map(|line| RUN_CHECK.captures(line))
        .filter_map(|found| {
            let what = found.get(1)?.as_str().to_string();
            let command = found.get(2)?.as_str().trim().to_string();
            (!command.is_empty()).then_some((what, command))
        })
        .collect()
}

fn is_mori_internal(command: &str) -> bool {
    MORI_INTERNAL.iter().any(|path| command.contains(path))
}

/// Whether a check can never fail: empty, or vacuous by the spec-quality
/// rule (HF2), also inside a `bash -c '…'` (mori's `reference` checks).
fn never_fails(command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() || vacuous_reason(command).is_some() {
        return true;
    }
    SHELL_SCRIPT
        .captures(command)
        .and_then(|found| found.get(1).or_else(|| found.get(2)))
        .is_some_and(|script| vacuous_reason(script.as_str()).is_some())
}

/// Whether `word` appears in `text` with no identifier character on either
/// side.
fn contains_word(text: &str, word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices(word).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

/// A heading's words as a `grep` pattern: anything but a letter, a digit or
/// a space matches any character.
fn heading_pattern(section: &str) -> String {
    one_line(section)
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' {
                c
            } else {
                '.'
            }
        })
        .collect()
}

/// mori's `cargo test … 2>&1 | grep -q '… ok'` hides the runner's output
/// (`plan validate` flags it, PLAN_050); the name guard keeps the check red
/// until the test exists, so the pipe can go.
fn unpiped(command: &str) -> String {
    let command = command.trim();
    PIPED_RUNNER
        .captures(command)
        .and_then(|found| found.get(1))
        .map_or_else(
            || command.to_string(),
            |found| found.as_str().trim().to_string(),
        )
}

/// `command` after a `grep` in `dir` for each named test, so a `cargo test`
/// filter that matches nothing cannot pass.
fn guarded(command: &str, tests: &[String], dir: &str) -> String {
    let command = unpiped(command);
    if tests.is_empty() || !command.starts_with("cargo test") {
        return command;
    }
    let guards: Vec<String> = tests
        .iter()
        .map(|test| {
            format!(
                "grep -rqw --exclude-dir=target --exclude-dir=.git {} {}",
                shell_quote(&format!("fn {test}")),
                shell_quote(dir)
            )
        })
        .collect();
    format!("{} && {command}", guards.join(" && "))
}

/// Add a check to `steps` unless it can never fail or `steps` has it.
fn push_step(steps: &mut Vec<Step>, phase: &str, command: &str, fail_msg: &str) -> bool {
    let command = unpiped(command);
    if never_fails(&command) || steps.iter().any(|step| step.command == command) {
        return false;
    }
    steps.push(Step {
        phase: phase.to_string(),
        command,
        fail_msg: one_line(fail_msg),
        passes_on_base: false,
    });
    true
}

/// Join `cargo test` checks that differ only in their test filter into one
/// run of their package (and target), after all their guards:
/// `cargo test -p x a` and `cargo test -p x b` become `cargo test -p x -- a
/// b`, which runs the tests either filter names. Returns how many checks
/// were joined into another.
fn join_cargo_tests(checks: &mut Vec<Step>) -> usize {
    /// One joined run: its prefix, its place in the checks, its guards, its
    /// filters and the messages of the checks it holds.
    struct Run {
        prefix: String,
        at: usize,
        guards: Vec<String>,
        filters: Vec<String>,
        messages: Vec<String>,
    }
    let mut kept: Vec<Step> = Vec::new();
    let mut runs: Vec<Run> = Vec::new();
    let mut joined = 0;
    for step in std::mem::take(checks) {
        let Some((guards, prefix, filter)) = cargo_test_parts(&step.command) else {
            kept.push(step);
            continue;
        };
        if let Some(run) = runs.iter_mut().find(|run| run.prefix == prefix) {
            for guard in guards {
                if !run.guards.contains(&guard) {
                    run.guards.push(guard);
                }
            }
            if !run.filters.contains(&filter) {
                run.filters.push(filter);
            }
            run.messages.push(step.fail_msg);
            joined += 1;
            continue;
        }
        runs.push(Run {
            prefix,
            at: kept.len(),
            guards,
            filters: vec![filter],
            messages: vec![step.fail_msg.clone()],
        });
        kept.push(step);
    }
    for run in runs.into_iter().filter(|run| run.messages.len() > 1) {
        let runner = format!("{} -- {}", run.prefix, run.filters.join(" "));
        let step = &mut kept[run.at];
        step.command = if run.guards.is_empty() {
            runner
        } else {
            format!("{} && {runner}", run.guards.join(" && "))
        };
        step.fail_msg = cut_words(&run.messages.join("; "), 60);
    }
    *checks = kept;
    joined
}

/// A `cargo test` check with one test filter, as its name guards, its
/// runner without the filter, and the filter; `None` for any other check.
fn cargo_test_parts(command: &str) -> Option<(Vec<String>, String, String)> {
    let mut parts: Vec<&str> = command.split(" && ").collect();
    let runner = parts.pop()?;
    if !parts.iter().all(|part| part.starts_with("grep -rqw ")) {
        return None;
    }
    let found = CARGO_TEST_FILTER.captures(runner.trim())?;
    Some((
        parts.iter().map(|part| (*part).to_string()).collect(),
        found.get(1)?.as_str().to_string(),
        found.get(2)?.as_str().to_string(),
    ))
}

/// Add `step` to `steps` unless `steps` has its command.
fn push_owned_step(steps: &mut Vec<Step>, step: Step) {
    if !steps.iter().any(|known| known.command == step.command) {
        steps.push(step);
    }
}

/// `raw` as one shell word.
fn shell_quote(raw: &str) -> String {
    if !raw.is_empty()
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:@%+=,".contains(c))
    {
        return raw.to_string();
    }
    format!("'{}'", raw.replace('\'', "'\\''"))
}

/// `id` as part of a file name.
fn file_safe(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// `path` relative to `base`, `/`-separated; empty when they are the same.
fn relative_path(path: &Path, base: &Path) -> Result<String> {
    let relative = path.strip_prefix(base).with_context(|| {
        format!(
            "the plans directory {} is not inside the workspace {}",
            path.display(),
            base.display()
        )
    })?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            _ => bail!(
                "the plans directory {} is not inside the workspace {}",
                path.display(),
                base.display()
            ),
        }
    }
    Ok(parts.join("/"))
}

// ---------------------------------------------------------------------------
// Writing TOML. The plan is for people to review, so keys keep a fixed
// order and long text stays readable.

fn render(
    plan_id: &str,
    src: &Path,
    meta: &[(String, Value)],
    plan_checks: &[Step],
    drafts: &[Draft],
) -> String {
    let mut out = format!(
        "# Imported from the mori plan `{plan_id}` ({}) by `roko plan import --mori`.\n\
         # The import report names what was mapped, carried as notes or dropped.\n[meta]\n",
        one_line(&src.display().to_string())
    );
    for (key, value) in meta {
        let _ = writeln!(out, "{key} = {}", render_value(value));
    }
    for step in plan_checks {
        out.push_str("\n[[meta.verify]]\n");
        render_step(&mut out, step);
    }
    for draft in drafts {
        out.push('\n');
        for comment in &draft.comments {
            let _ = writeln!(out, "# {}", one_line(comment));
        }
        out.push_str("[[task]]\n");
        let mut field = |key: &str, value: &str| {
            let _ = writeln!(out, "{key} = {value}");
        };
        field("id", &toml_string(&draft.id));
        field("title", &toml_string(&draft.title));
        field("description", &toml_string(&draft.lead));
        field("status", "\"ready\"");
        field("role", &toml_string(draft.role));
        field("tier", &toml_string(draft.tier.label()));
        field("files", &string_array(&draft.files));
        field("depends_on", &string_array(&draft.depends_on));
        if !draft.depends_on_plan.is_empty() {
            field("depends_on_plan", &string_array(&draft.depends_on_plan));
        }
        if let Some(minutes) = draft.estimated_minutes {
            field("estimated_minutes", &minutes.to_string());
        }
        for (key, value) in &draft.hints {
            field(key, &render_value(value));
        }
        if !draft.acceptance.is_empty() {
            field("acceptance", &string_array(&draft.acceptance));
        }
        if !draft.read_files.is_empty() {
            out.push_str("\n[task.context]\nread_files = [\n");
            for (path, why) in &draft.read_files {
                let _ = writeln!(
                    out,
                    "    {{ path = {}, why = {} }},",
                    toml_string(path),
                    toml_string(why)
                );
            }
            out.push_str("]\n");
        }
        for step in draft.steps() {
            out.push_str("\n[[task.verify]]\n");
            render_step(&mut out, &step);
        }
    }
    out
}

fn render_step(out: &mut String, step: &Step) {
    let _ = writeln!(out, "phase = {}", toml_string(&step.phase));
    let _ = writeln!(out, "command = {}", toml_string(&step.command));
    let _ = writeln!(out, "fail_msg = {}", toml_string(&step.fail_msg));
    if step.passes_on_base {
        out.push_str("expect = \"pass_on_base\"\n");
    }
}

fn string_array(items: &[String]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let mut out = String::from("[\n");
    for item in items {
        let _ = writeln!(out, "    {},", toml_string(item));
    }
    out.push(']');
    out
}

fn render_value(value: &Value) -> String {
    match value {
        Value::String(text) => toml_string(text),
        Value::Array(items) if items.iter().all(Value::is_str) => {
            let items: Vec<String> = items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect();
            string_array(&items)
        }
        Value::Table(table) => {
            let inner: Vec<String> = table
                .iter()
                .map(|(key, value)| format!("{} = {}", toml_key(key), render_value(value)))
                .collect();
            format!("{{ {} }}", inner.join(", "))
        }
        other => other.to_string(),
    }
}

fn toml_key(key: &str) -> String {
    if !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        key.to_string()
    } else {
        toml_string(key)
    }
}

/// A TOML string: a basic one on one line; for text with line breaks, a
/// literal multi-line one, or a basic multi-line one when the literal form
/// cannot hold the text.
fn toml_string(text: &str) -> String {
    if !text.contains('\n') {
        return format!("\"{}\"", escape(text, false));
    }
    let literal = !text.contains("'''")
        && !text.ends_with('\'')
        && !text
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t');
    if literal {
        format!("'''\n{text}'''")
    } else {
        format!("\"\"\"\n{}\"\"\"", escape(text, true))
    }
}

/// `text` escaped for a basic string; `multiline` keeps its line breaks.
fn escape(text: &str, multiline: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' if multiline => out.push('\n'),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04X}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan_validate::{Severity, validate_plans_dir_with_workdir};

    const PLAN_MD: &str = "# Core types and voices\n\n## Quick Reference\n\nT1 then T2.\n\n\
                           ## What to build\n\nA core crate.\n\n\
                           <!-- mori:generated-reuse begin -->\n## Mori Reuse Appendix\n\nold\n";

    /// A mori plan directory: `files` as `(name, contents)`.
    fn mori_plan(root: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = root.join(name);
        fs::create_dir_all(&dir).expect("mori plan dir");
        for (file, contents) in files {
            fs::write(dir.join(file), contents).expect("mori file");
        }
        dir
    }

    fn core_tasks() -> String {
        let many: Vec<String> = (1..=12)
            .map(|n| format!("\"crates/demo-core/src/gen/m{n}.rs\""))
            .chain((1..=3).map(|n| format!("\"crates/demo-core/src/out/o{n}.rs\"")))
            .collect();
        format!(
            r#"[meta]
plan = "01-core"
iteration = 3
total = 6
done = 1
status = "pending"
max_parallel = 9
estimated_total_minutes = 90

[[task]]
id = "T1"
title = "Scaffold the core crate"
status = "done"
files = ["crates/demo-core/Cargo.toml", "crates/demo-core/src/lib.rs"]
acceptance = [
    "File exists: crates/demo-core/src/lib.rs",
    "Checkpoint: `cargo check -p demo-core` exits 0",
]
depends_on = []
complexity_band = "fast"
category = "scaffolding"
tags = ["crate:demo-core"]
parallel_group = "A"
exclusive_files = true
context_files = ["docs/missing-guide.md"]

[[task]]
id = "T2"
title = "Implement the oscillator"
status = "pending"
files = ["crates/demo-core/src/lib.rs", "crates/demo-core/src/osc.rs"]
acceptance = ["`test_osc_is_audible` passes", "No allocation in process"]
depends_on = []
complexity_band = "standard"
category = "implementation"
reasoning_level = "high"
escalate_on_retry = true
speed_priority = "balanced"
exclusive_files = true

[[task]]
id = "T3"
title = "Write the generators"
status = "pending"
files = [{many}]
acceptance = ["Every generator compiles"]
depends_on = ["T1"]
complexity_band = "standard"
category = "scaffolding"

[[task]]
id = "T4"
title = "Write the guide"
status = "pending"
files = ["docs/demo/guide.md"]
acceptance = ["The guide explains the oscillator"]
depends_on = ["T2"]
category = "docs"

[[task]]
id = "V1"
title = "Verify every invariant passes"
status = "pending"
files = []
acceptance = ["INV-1 passes"]
depends_on = ["T2", "T9"]
category = "verification"

[[task]]
id = "V2"
title = "Placeholder"
status = "pending"
files = []
acceptance = ["Nothing to do"]
depends_on = []
"#,
            many = many.join(", ")
        )
    }

    const CORE_VERIFY: &str = r#"
[[task]]
id = "UT1"
title = "Oscillator tests"
type = "unit_test"
crate = "demo-core"
command = "cargo test -p demo-core test_osc 2>&1 | grep -q 'test result: ok'"
test_fns = ["test_osc_is_audible"]
blocking = true

[[task]]
id = "CG1"
title = "Compile the test suite"
type = "compile"
command = "cargo test -p demo-core --no-run"
blocking = true

[[task]]
id = "LC1"
title = "Lifecycle"
type = "lifecycle"
command = "cargo test -p demo-core --release lifecycle"
blocking = false

[[task]]
id = "REF1"
title = "Reference"
type = "reference"
command = "bash -c 'echo verified'"
blocking = false
"#;

    const CORE_REVIEW: &str = r#"
[[task]]
id = "R1"
title = "No allocation on the audio path"
severity = "critical"
check = ["process() allocates nothing"]

[[task]]
id = "R2"
title = "Ports match the plan"
check = ["Every port of the Quick Reference exists"]
"#;

    const CORE_SCRIBE: &str = r#"
[[task]]
id = "D1"
type = "module_doc"
title = "Document demo-core"
output_file = "plans/context/docs/01-demo-core.md"
sections = ["context", "api (public)"]

[[task]]
id = "M1"
type = "diagram"
title = "Oscillator flow"
diagram_type = "flowchart"
concept = "pitch to frequency to samples"

[[task]]
id = "F1"
type = "formula"
title = "Pitch formula"
"#;

    const CORE_VERIFY_SH: &str = "#!/bin/bash\nrun_check() { :; }\n\
                                  run_check \"test-crates/demo-core\" \"Tests: demo-core\" \
                                  cargo test -p crates/demo-core\n\
                                  run_check \"ctx\" \"Context\" bash plans/context/check.sh\n";

    const VOICES_TASKS: &str = r#"[meta]
plan = "02-voices"

[[task]]
id = "T1"
title = "Add voices"
status = "pending"
files = ["crates/demo-voices/src/lib.rs"]
acceptance = ["Checkpoint: `cargo test -p demo-voices` exits 0"]
depends_on = ["01:T2", "07z:T1"]
"#;

    /// gap-624658: two mori plans, one depending on the other, import into
    /// roko plans that pass `plan validate` with no diagnostic: roles,
    /// checks from verify-tasks.toml, acceptance and "File exists" lines,
    /// the whole-plan check from the unclaimed checks, verify.sh and the
    /// rubric, the reviewer and docs tasks, the cross-plan dependency, the
    /// file lock as a dependency, a narrowed task, and the companions.
    #[test]
    fn plan_import_mori_writes_a_plan_that_validates() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mori_root = temp.path().join("bardo/.mori/plans");
        let core_tasks = core_tasks();
        mori_plan(
            &mori_root,
            "01-core",
            &[
                ("tasks.toml", core_tasks.as_str()),
                ("plan.md", PLAN_MD),
                ("verify-tasks.toml", CORE_VERIFY),
                ("review-tasks.toml", CORE_REVIEW),
                ("scribe-tasks.toml", CORE_SCRIBE),
                (
                    "rubric.md",
                    "# Rubric\n\n- [ ] `cargo clippy -p demo-core` passes\n",
                ),
                ("decomposition.md", "# Decomposition\n\n### Step 1\n"),
                ("verify.sh", CORE_VERIFY_SH),
                ("research.md", "learned history\n"),
            ],
        );
        mori_plan(&mori_root, "02-voices", &[("tasks.toml", VOICES_TASKS)]);
        let workspace = temp.path().join("ws");
        fs::create_dir_all(workspace.join("plans")).expect("plans dir");
        let plans = workspace.join("plans");

        let imported = import_mori_plans(
            &[mori_root.clone()],
            &workspace,
            &plans,
            MoriImportOptions::default(),
            false,
        )
        .expect("import");
        assert_eq!(imported.len(), 2);

        let report =
            validate_plans_dir_with_workdir(&plans, None, Some(&workspace)).expect("validate");
        let diagnostics: Vec<String> = report
            .plans
            .iter()
            .flat_map(|plan| &plan.diagnostics)
            .map(|diagnostic| format!("{} {}", diagnostic.rule_id, diagnostic.message))
            .collect();
        assert_eq!(report.totals.errors, 0, "{diagnostics:#?}");
        assert_eq!(report.totals.warnings, 0, "{diagnostics:#?}");
        assert!(
            report
                .plans
                .iter()
                .flat_map(|plan| &plan.diagnostics)
                .all(|diagnostic| diagnostic.severity != Severity::Error)
        );

        let core = TasksFile::parse(&plans.join("01-core/tasks.toml")).expect("core plan");
        let task = |id: &str| {
            core.tasks
                .iter()
                .find(|task| task.id == id)
                .unwrap_or_else(|| panic!("task {id}"))
        };
        let commands = |id: &str| -> Vec<String> {
            task(id)
                .verify
                .iter()
                .map(|step| step.command.clone())
                .collect()
        };
        assert_eq!(core.meta.title.as_deref(), Some("Core types and voices"));
        assert_eq!(task("T1").role.as_deref(), Some("implementer"));
        assert_eq!(task("T1").tier, "mechanical");
        assert_eq!(task("T1").status, "ready", "mori's done is not carried");
        assert_eq!(
            commands("T1"),
            [
                "cargo check -p demo-core",
                "test -f crates/demo-core/src/lib.rs"
            ]
        );
        // The blocking check T2's acceptance names, unpiped, behind its guard.
        assert_eq!(
            commands("T2"),
            [
                "grep -rqw --exclude-dir=target --exclude-dir=.git 'fn test_osc_is_audible' \
                 crates/demo-core && cargo test -p demo-core test_osc"
            ]
        );
        // mori's file lock: T2 writes lib.rs after T1.
        assert_eq!(task("T2").depends_on, ["T1"]);
        assert_eq!(task("T4").role.as_deref(), Some("scribe"));
        assert_eq!(commands("T4"), ["test -e docs/demo/guide.md"]);
        // 15 files narrowed to their 2 directories, the list beside the plan.
        assert_eq!(
            task("T3").files,
            ["crates/demo-core/src/gen/", "crates/demo-core/src/out/"]
        );
        let list = fs::read_to_string(plans.join("01-core/mori-files-T3.txt")).expect("list");
        assert_eq!(list.lines().count(), 15);
        // V1 names no file and no crate: a check task running T2's checks;
        // V2 has nothing to do or check, so it is left out.
        assert_eq!(task("V1").role.as_deref(), Some("quick-reviewer"));
        assert!(task("V1").files.is_empty());
        assert_eq!(commands("V1"), commands("T2"));
        assert!(core.tasks.iter().all(|task| task.id != "V2"));
        // The whole-plan check: the unclaimed compile check, verify.sh's
        // step with its package named by name, the rubric's command.
        let plan_checks: Vec<&str> = core
            .meta
            .verify
            .iter()
            .map(|step| step.command.as_str())
            .collect();
        assert_eq!(
            plan_checks,
            [
                "cargo test -p demo-core --no-run",
                "cargo test -p demo-core",
                "cargo clippy -p demo-core"
            ]
        );
        assert_eq!(core.meta.workspace_rungs, Some(false));
        assert_eq!(core.meta.max_parallel, Some(7), "clamped to the 7 tasks");
        let review = task("REVIEW");
        assert_eq!(review.role.as_deref(), Some("quick-reviewer"));
        let review_text = review.description.as_deref().unwrap_or_default();
        assert!(
            review_text.contains("No allocation on the audio path"),
            "{review_text}"
        );
        assert!(review_text.contains("--release lifecycle"), "{review_text}");
        assert!(!review_text.contains("echo verified"), "{review_text}");
        let docs = task("DOCS");
        assert_eq!(
            docs.files,
            ["docs/01-core/01-demo-core.md", "docs/01-core/diagrams.md"]
        );
        assert!(docs.depends_on.contains(&"T4".to_string()));

        let voices = TasksFile::parse(&plans.join("02-voices/tasks.toml")).expect("voices plan");
        assert_eq!(voices.tasks[0].depends_on_plan, ["01-core"]);
        let voices_report = &imported[1].report;
        assert_eq!(voices_report.cross_plan.len(), 2);
        assert_eq!(voices_report.cross_plan[1].plan, None, "07z names no plan");

        let core_report = &imported[0].report;
        assert_eq!(core_report.done_in_mori, 1);
        assert_eq!(core_report.dropped_tasks, ["V2"]);
        assert_eq!(core_report.advisory_checks, 1);
        assert!(
            core_report
                .notes
                .iter()
                .any(|note| note.contains("research.md")),
            "{:#?}",
            core_report.notes
        );
        let plan_md = fs::read_to_string(plans.join("01-core/plan.md")).expect("plan.md");
        assert!(!plan_md.contains("Mori Reuse Appendix"));
        let brief = fs::read_to_string(plans.join("01-core/brief.md")).expect("brief.md");
        assert!(
            brief.contains("| Decomposition | `decomposition.md` |"),
            "{brief}"
        );
        assert!(brief.contains("| Review rubric | `rubric.md` |"), "{brief}");
        for copied in [
            "rubric.md",
            "review-tasks.toml",
            "verify-tasks.toml",
            "scribe-tasks.toml",
        ] {
            assert!(plans.join("01-core").join(copied).is_file(), "{copied}");
        }

        // A second import over the first is refused unless forced.
        let again = import_mori_plans(
            &[mori_root.clone()],
            &workspace,
            &plans,
            MoriImportOptions::default(),
            false,
        );
        assert!(again.is_err());
        import_mori_plans(
            &[mori_root],
            &workspace,
            &plans,
            MoriImportOptions::default(),
            true,
        )
        .expect("forced import");
    }

    /// bug-dfe4c5: a fresh import takes no workspace lock, so it runs beside
    /// the roko serve that holds the workspace, and moves each new plan into
    /// place whole; replacing plans (`--force`) needs the lock, and the
    /// refusal names serve.
    #[test]
    fn plan_import_runs_beside_the_serve_that_holds_the_workspace() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("ws");
        let plans = workspace.join("plans");
        let serve_lock = crate::workspace_lock::acquire_workspace_lock(&workspace.join(".roko"))
            .expect("serve's lock");
        let endpoint = roko_serve::endpoint::ServeEndpoint {
            pid: std::process::id(),
            url: "http://127.0.0.1:6792".to_string(),
            workdir: workspace.clone(),
            started_at: "2026-10-17T10:00:00Z".to_string(),
        };
        roko_serve::endpoint::write_endpoint(&workspace, &endpoint).expect("serve.json");
        let tasks = "[meta]\nplan = \"08g-voices\"\n\n[[task]]\nid = \"T1\"\n\
                     title = \"Write the voices doc\"\nfiles = [\"docs/voices.md\"]\n\
                     acceptance = [\"File exists: docs/voices.md\"]\ncategory = \"docs\"\n";
        let mori_root = temp.path().join("mori");
        mori_plan(&mori_root, "08g-voices", &[("tasks.toml", tasks)]);

        let lock = import_lock(&workspace, false).expect("a fresh import takes no lock");
        assert!(lock.is_none());
        let import = |force| {
            import_mori_plans(
                std::slice::from_ref(&mori_root),
                &workspace,
                &plans,
                MoriImportOptions::default(),
                force,
            )
        };
        import(false).expect("import beside serve");
        assert!(plans.join("08g-voices/tasks.toml").is_file());
        assert!(plans.join("08g-voices/brief.md").is_file());
        let left: Vec<String> = fs::read_dir(&plans)
            .expect("plans")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(left, ["08g-voices"], "no staging directory is left");
        let found = crate::orchestrator::plan_discovery::discover_plans(&plans).expect("plans");
        assert_eq!(found.len(), 1);

        let again = import(false).expect_err("the plan exists");
        assert!(again.to_string().contains("--force"), "{again:#}");
        let Err(refused) = import_lock(&workspace, true) else {
            panic!("serve holds the workspace, so --force gets no lock");
        };
        assert!(
            refused
                .to_string()
                .starts_with(&format!("roko serve (PID {})", std::process::id())),
            "{refused:#}"
        );
        drop(serve_lock);
        let lock = import_lock(&workspace, true).expect("the lock is free");
        assert!(lock.is_some());
        import(true).expect("replace under the lock");
    }

    /// gap-624658: a task whose files exist already gets a file check that
    /// says it passes on the base, so the spec gate does not refuse work
    /// mori finished; one with more crates than a task takes is split.
    #[test]
    fn plan_import_mori_marks_finished_files_and_splits_wide_tasks() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("ws");
        fs::create_dir_all(workspace.join("docs")).expect("docs");
        fs::write(workspace.join("docs/done.md"), "# Done\n").expect("done.md");
        let crates: Vec<String> = (1..=12)
            .map(|n| format!("\"crates/c{n}/src/lib.rs\""))
            .collect();
        let tasks = format!(
            r#"[meta]
plan = "A1-audit"

[[task]]
id = "D1"
title = "Write the finished doc"
files = ["docs/done.md"]
acceptance = ["File exists: docs/done.md"]
category = "docs"

[[task]]
id = "A1"
title = "Audit every crate"
files = [{}]
acceptance = ["No hardcoded secrets"]
depends_on = ["D1"]
"#,
            crates.join(", ")
        );
        let src = mori_plan(temp.path(), "A1-audit", &[("tasks.toml", tasks.as_str())]);
        let import = import_mori_plan(
            &src,
            &workspace,
            "plans/A1-audit",
            MoriImportOptions::default(),
        )
        .expect("import");
        let plan = TasksFile::parse_str(&import.tasks_toml).expect("parse");
        let d1 = plan.tasks.iter().find(|task| task.id == "D1").expect("D1");
        assert_eq!(d1.verify[0].command, "test -f docs/done.md");
        assert_eq!(
            d1.verify[0].expect,
            Some(crate::task_parser::VerifyExpect::PassOnBase)
        );
        let ids: Vec<&str> = plan.tasks.iter().map(|task| task.id.as_str()).collect();
        assert_eq!(ids, ["D1", "A1a", "A1b"]);
        let a1b = &plan.tasks[2];
        assert_eq!(a1b.files.len(), 2);
        assert!(a1b.depends_on.contains(&"A1a".to_string()));
        assert_eq!(import.report.splits[0].parts, ["A1a", "A1b"]);
        assert_eq!(import.report.narrowed[0].to.len(), 12);
    }

    #[test]
    fn plan_import_helpers_map_mori_commands() {
        assert_eq!(
            unpiped("cargo test -p x t 2>&1 | grep -q 'test result: ok'"),
            "cargo test -p x t"
        );
        assert_eq!(
            unpiped("cargo check -p x | tee log"),
            "cargo check -p x | tee log"
        );
        assert_eq!(
            unpiped("cargo check -p x 2>&1 | tail -1 | grep -q Finished"),
            "cargo check -p x"
        );
        let step = |command: &str| Step {
            phase: "unit_test".to_string(),
            command: command.to_string(),
            fail_msg: command.to_string(),
            passes_on_base: false,
        };
        let mut checks = vec![
            step("grep -rqw 'fn test_a' crates/x && cargo test -p x test_a"),
            step("cargo check -p x"),
            step("grep -rqw 'fn test_b' crates/x && cargo test -p x test_b"),
            step("cargo test -p y test_c"),
            step("cargo test -p x --release test_d"),
        ];
        assert_eq!(join_cargo_tests(&mut checks), 1);
        let commands: Vec<&str> = checks.iter().map(|step| step.command.as_str()).collect();
        assert_eq!(
            commands,
            [
                "grep -rqw 'fn test_a' crates/x && grep -rqw 'fn test_b' crates/x && cargo test \
                 -p x -- test_a test_b",
                "cargo check -p x",
                "cargo test -p y test_c",
                "cargo test -p x --release test_d"
            ]
        );
        assert_eq!(
            guarded("cargo test -p x t", &["t".to_string()], "crates/x"),
            "grep -rqw --exclude-dir=target --exclude-dir=.git 'fn t' crates/x && cargo test -p x t"
        );
        assert_eq!(
            guarded("cargo check -p x", &["t".to_string()], "."),
            "cargo check -p x"
        );
        let steps = verify_sh_steps(
            "run_check \"a\" \"Tests: x\" cargo test -p apps/x\n\
             run_check_if_crate y \"b\" \"Suite: y\" cargo test -p y\nrun_check() {\n",
        );
        assert_eq!(
            steps,
            [
                ("Tests: x".to_string(), "cargo test -p apps/x".to_string()),
                ("Suite: y".to_string(), "cargo test -p y".to_string())
            ]
        );
        assert_eq!(
            PACKAGE_PATH.replace_all("cargo test -p apps/x --lib", "$1$2"),
            "cargo test -p x --lib"
        );
        assert_eq!(
            acceptance_commands(&[
                "Checkpoint after Step 2: `cargo check -p x`".to_string(),
                "`python3 -m unittest tests.test_osc` exits 0".to_string(),
                "Run `cargo test` sometime".to_string(),
                "Checkpoint: `bash plans/context/verify.sh` exits 0".to_string(),
            ]),
            ["cargo check -p x", "python3 -m unittest tests.test_osc"]
        );
        assert!(contains_word("run `test_a` now", "test_a"));
        assert!(!contains_word("run test_ab now", "test_a"));
        assert_eq!(
            glob_dir("crates/x/src/*.rs").as_deref(),
            Some("crates/x/src/")
        );
        assert_eq!(glob_dir("*.rs"), None);
        assert_eq!(
            collapse(
                ["a/b/", "a/b/c.rs", "a/d.rs", "a/b/"]
                    .into_iter()
                    .map(String::from)
            ),
            ["a/b/", "a/d.rs"]
        );
        assert_eq!(top_dir("apps/term/src/x.rs"), "apps/term/");
        assert_eq!(top_dir("docs/a/b.md"), "docs/");
        assert_eq!(top_dir("Cargo.toml"), "Cargo.toml");
        assert!(paths_overlap("crates/x/", "crates/x/src/lib.rs"));
        assert!(!paths_overlap("crates/x", "crates/xy/lib.rs"));
        for text in [
            "plain",
            "with \"quotes\" and \\",
            "two\nlines",
            "ends in a quote'\nsecond",
            "has ''' inside\nand more",
        ] {
            let parsed: Table =
                toml::from_str(&format!("value = {}", toml_string(text))).expect("toml");
            assert_eq!(parsed["value"].as_str(), Some(text), "{text:?}");
        }
    }

    /// The mori corpus (`ROKO_MORI_CORPUS`, a `.mori/plans` directory) as roko
    /// plans: every plan imports, and the counts `plan validate` passes
    /// with and without `--strict` are printed. Run it with
    /// `ROKO_MORI_CORPUS=<bardo>/.mori/plans cargo test -p roko-cli --lib
    /// plan_import_mori_corpus -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads a mori corpus named by ROKO_MORI_CORPUS"]
    fn plan_import_mori_corpus_validates() {
        let Some(corpus) = std::env::var_os("ROKO_MORI_CORPUS").map(PathBuf::from) else {
            eprintln!("ROKO_MORI_CORPUS is not set");
            return;
        };
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path();
        let plans = workspace.join("plans");
        fs::create_dir_all(&plans).expect("plans dir");
        let imported = import_mori_plans(
            &[corpus],
            workspace,
            &plans,
            MoriImportOptions::default(),
            false,
        )
        .expect("every mori plan imports");
        let report =
            validate_plans_dir_with_workdir(&plans, None, Some(workspace)).expect("validate");
        let mut by_rule: BTreeMap<(String, String), usize> = BTreeMap::new();
        let mut with_errors = 0;
        let mut with_diagnostics = 0;
        for plan in &report.plans {
            with_diagnostics += usize::from(!plan.diagnostics.is_empty());
            with_errors += usize::from(
                plan.diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.severity == Severity::Error),
            );
            for diagnostic in &plan.diagnostics {
                let severity = format!("{:?}", diagnostic.severity);
                *by_rule
                    .entry((severity, diagnostic.rule_id.clone()))
                    .or_default() += 1;
            }
        }
        let total = report.totals.plans_checked;
        let tasks: usize = imported.iter().map(|plan| plan.report.tasks).sum();
        let mori_tasks: usize = imported.iter().map(|plan| plan.report.mori_tasks).sum();
        let sum = |count: fn(&MoriImportReport) -> usize| -> usize {
            imported.iter().map(|plan| count(&plan.report)).sum()
        };
        eprintln!(
            "mori corpus: {} plans imported ({mori_tasks} mori tasks -> {tasks} tasks); \
             plan validate passes {} of {total}, --strict {} of {total}",
            imported.len(),
            total - with_errors,
            total - with_diagnostics
        );
        eprintln!(
            "checks: {} verify-tasks, {} acceptance, {} file-exists files, {} task commands, \
             {} cargo check fallbacks, {} joined, {} moved to the plan; {} whole-plan; {} \
             serialization edges; {} cross-plan; {} tier raises; {} narrowed; {} split; {} only \
             file checks; {} left out",
            sum(|r| r.checks.verify_tasks),
            sum(|r| r.checks.acceptance),
            sum(|r| r.checks.file_exists),
            sum(|r| r.checks.task_command),
            sum(|r| r.checks.compile_fallback),
            sum(|r| r.checks.joined),
            sum(|r| r.checks.moved_to_plan),
            sum(|r| r.plan_checks),
            sum(|r| r.serialized.len()),
            sum(|r| r.cross_plan.len()),
            sum(|r| r.tier_changes.len()),
            sum(|r| r.narrowed.len()),
            sum(|r| r.splits.len()),
            sum(|r| r.structural_only.len()),
            sum(|r| r.dropped_tasks.len()),
        );
        for ((severity, rule), count) in &by_rule {
            eprintln!("  {severity} {rule}: {count}");
        }
        // The prototype (plans-research's mori2roko.py) passed 132 and 19.
        assert!(total - with_errors >= 132, "{by_rule:#?}");
        assert!(total - with_diagnostics >= 19, "{by_rule:#?}");
    }
}
