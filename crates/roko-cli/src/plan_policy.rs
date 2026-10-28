//! Fail-fast execution policy for generated plans and declared task context.
//!
//! The runner must not spend an agent turn rediscovering information the plan
//! claimed to provide. This module validates those claims before execution and
//! renders a small, exact context packet immediately before dispatch.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::path::{Component, Path, PathBuf};

use roko_core::task::TaskTier;

use crate::task_parser::{ReadFile, TaskDef, TasksFile};

/// Conservative absolute limits for ordinary plans. These are safety bounds,
/// not generation targets; FAST and generated-plan policies are much tighter.
const NORMAL_MAX_TASKS: usize = 64;
const NORMAL_MAX_FILES_PER_TASK: usize = 32;
const NORMAL_MAX_READ_FILES: usize = 16;
const NORMAL_MAX_VERIFY_STEPS: usize = 8;
const NORMAL_MAX_CONTEXT_ITEMS: usize = 32;
const NORMAL_MAX_RANGE_LINES: usize = 1_000;
const NORMAL_MAX_DECLARED_CONTEXT_BYTES: usize = 64 * 1024;

const FAST_MAX_TASKS: usize = 4;
const FAST_MAX_FILES_PER_TASK: usize = 8;
const FAST_MAX_READ_FILES: usize = 3;
const FAST_MAX_VERIFY_STEPS: usize = 1;
const FAST_MAX_CONTEXT_ITEMS: usize = 8;
const FAST_MAX_PRIOR_FAILURES: usize = 4;
const FAST_MAX_RANGE_LINES: usize = 240;
const FAST_MAX_DECLARED_CONTEXT_BYTES: usize = 24 * 1024;
const FAST_MAX_SERIAL_OWNERS_PER_FILE: usize = 2;
const ANCHOR_CONTEXT_RADIUS: usize = 8;

/// Extensions of the source files a CMake target lists (bug-daa54e).
pub(crate) const CMAKE_SOURCES: &[&str] = &["c", "cc", "cpp", "cxx", "m", "mm"];

/// File extensions of C and C++ headers.
pub(crate) const C_HEADERS: &[&str] = &["h", "hh", "hpp", "hxx"];

/// Binding default-template task ceiling for plan generation and regeneration
/// (the `default` plan template).
pub const DEFAULT_GENERATED_TASK_LIMIT: usize = 8;

/// Structural limits applied to a plan before it can execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanExecutionPolicy {
    pub max_tasks: usize,
    pub max_files_per_task: usize,
    pub max_read_files_per_task: usize,
    pub max_verify_steps_per_task: usize,
    pub max_symbols_per_task: usize,
    pub max_anti_patterns_per_task: usize,
    pub max_prior_failures_per_task: usize,
    pub max_range_lines: usize,
    pub max_declared_context_bytes: usize,
    pub max_serial_owners_per_file: Option<usize>,
    /// Verify steps every task must have: one in a FAST or generated plan,
    /// whose tasks a runner-owned check settles.
    pub min_verify_steps_per_task: usize,
    /// Exactly one verify step per task (the FAST lane).
    pub require_one_verify: bool,
    pub reject_duplicate_verify: bool,
    pub bounded_context_only: bool,
    /// Whether a task may read as many files as it owns when it owns more
    /// than `max_read_files_per_task`: a generated plan outside the FAST lane
    /// (bug-417c4a). Round v7's phase-4.2 T1 owned 6 files and could read 3,
    /// its spec among them, so each draft anchored a symbol in a file it did
    /// not read.
    pub reads_cover_owned_files: bool,
}

impl PlanExecutionPolicy {
    /// Safety bounds for an ordinary execution.
    #[must_use]
    pub const fn normal() -> Self {
        Self {
            max_tasks: NORMAL_MAX_TASKS,
            max_files_per_task: NORMAL_MAX_FILES_PER_TASK,
            max_read_files_per_task: NORMAL_MAX_READ_FILES,
            max_verify_steps_per_task: NORMAL_MAX_VERIFY_STEPS,
            max_symbols_per_task: NORMAL_MAX_CONTEXT_ITEMS,
            max_anti_patterns_per_task: NORMAL_MAX_CONTEXT_ITEMS,
            max_prior_failures_per_task: NORMAL_MAX_CONTEXT_ITEMS,
            max_range_lines: NORMAL_MAX_RANGE_LINES,
            max_declared_context_bytes: NORMAL_MAX_DECLARED_CONTEXT_BYTES,
            max_serial_owners_per_file: None,
            min_verify_steps_per_task: 0,
            require_one_verify: false,
            reject_duplicate_verify: false,
            bounded_context_only: false,
            reads_cover_owned_files: false,
        }
    }

    /// Tight five-minute-loop bounds. A rejected plan should be repaired or
    /// regenerated as a cohesive task instead of consuming several turns.
    #[must_use]
    pub const fn fast() -> Self {
        Self {
            max_tasks: FAST_MAX_TASKS,
            max_files_per_task: FAST_MAX_FILES_PER_TASK,
            max_read_files_per_task: FAST_MAX_READ_FILES,
            max_verify_steps_per_task: FAST_MAX_VERIFY_STEPS,
            max_symbols_per_task: FAST_MAX_CONTEXT_ITEMS,
            max_anti_patterns_per_task: FAST_MAX_CONTEXT_ITEMS,
            max_prior_failures_per_task: FAST_MAX_PRIOR_FAILURES,
            max_range_lines: FAST_MAX_RANGE_LINES,
            max_declared_context_bytes: FAST_MAX_DECLARED_CONTEXT_BYTES,
            max_serial_owners_per_file: Some(FAST_MAX_SERIAL_OWNERS_PER_FILE),
            min_verify_steps_per_task: 1,
            require_one_verify: true,
            reject_duplicate_verify: true,
            bounded_context_only: true,
            reads_cover_owned_files: false,
        }
    }

    /// Limits for a newly generated artifact. The template-specific task
    /// ceiling is binding rather than prompt-only guidance. A task has at
    /// least one verify step and at most its tier's limit
    /// ([`TierSizeLimits::max_verify_steps`]), so each acceptance criterion
    /// can have its own (gap-dde99e).
    #[must_use]
    pub const fn generated(max_tasks: usize) -> Self {
        Self {
            max_tasks,
            max_files_per_task: FAST_MAX_FILES_PER_TASK,
            max_read_files_per_task: FAST_MAX_READ_FILES,
            max_verify_steps_per_task: NORMAL_MAX_VERIFY_STEPS,
            max_symbols_per_task: FAST_MAX_CONTEXT_ITEMS,
            max_anti_patterns_per_task: FAST_MAX_CONTEXT_ITEMS,
            max_prior_failures_per_task: FAST_MAX_PRIOR_FAILURES,
            max_range_lines: FAST_MAX_RANGE_LINES,
            max_declared_context_bytes: FAST_MAX_DECLARED_CONTEXT_BYTES,
            max_serial_owners_per_file: Some(FAST_MAX_SERIAL_OWNERS_PER_FILE),
            min_verify_steps_per_task: 1,
            require_one_verify: false,
            reject_duplicate_verify: true,
            bounded_context_only: true,
            reads_cover_owned_files: true,
        }
    }

    /// Generated-plan budget aligned with the lane that will execute it.
    /// Without this clamp a FAST generation could spend a full strategist
    /// turn producing eight tasks that the four-task execution policy then
    /// rejects before dispatch. A FAST plan's tasks keep their one check.
    #[must_use]
    pub fn generated_for_environment(max_tasks: usize) -> Self {
        let policy = Self::generated(effective_generated_task_limit(max_tasks));
        if env_flag_enabled("ROKO_FAST_MODE") {
            Self {
                max_verify_steps_per_task: FAST_MAX_VERIFY_STEPS,
                require_one_verify: true,
                reads_cover_owned_files: false,
                ..policy
            }
        } else {
            policy
        }
    }

    /// The files `task` may read besides the plan's sources: the policy's
    /// limit, or as many as it owns when it owns more and
    /// [`Self::reads_cover_owned_files`] (bug-417c4a).
    #[must_use]
    pub fn max_read_files_for(&self, task: &TaskDef) -> usize {
        if self.reads_cover_owned_files {
            self.max_read_files_per_task.max(task.files.len())
        } else {
            self.max_read_files_per_task
        }
    }

    /// Resolve the execution lane without exposing environment reads across
    /// the rest of prompt assembly and plan loading.
    #[must_use]
    pub fn for_environment() -> Self {
        if env_flag_enabled("ROKO_FAST_MODE") {
            Self::fast()
        } else {
            Self::normal()
        }
    }
}

/// Effective generator ceiling for the current execution lane.
#[must_use]
pub fn effective_generated_task_limit(max_tasks: usize) -> usize {
    if env_flag_enabled("ROKO_FAST_MODE") {
        max_tasks.min(FAST_MAX_TASKS)
    } else {
        max_tasks
    }
}

// ---- Size limits per executor tier ---------------------------------------

/// How big a task of one tier may be: the work a model on that tier's rung
/// can be expected to finish. `plan validate` warns about a task over them
/// (`PLAN_TIER_SIZE`); `plan run` does not check them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierSizeLimits {
    /// Output `files` a task may declare.
    pub max_files: usize,
    /// Largest `max_loc` a task may declare: the tier's line budget.
    pub max_loc: u32,
    /// Words in the task's `description`.
    pub max_description_words: usize,
    /// Authored `verify` steps.
    pub max_verify_steps: usize,
}

impl TierSizeLimits {
    /// Default limits for `tier`. The line budget is [`TaskTier::max_loc`];
    /// files, description words and verify steps sit at or above what the
    /// tier's tasks in `plans/` used at p95 on 2026-09-30 (mechanical: 3
    /// files, 3 verify steps), and the top tier gets the normal policy's
    /// bounds.
    #[must_use]
    pub const fn for_tier(tier: TaskTier) -> Self {
        let (max_files, max_description_words, max_verify_steps) = match tier {
            TaskTier::Mechanical => (3, 300, 3),
            TaskTier::Focused => (5, 350, 4),
            TaskTier::Integrative => (10, 500, 6),
            TaskTier::Architectural => (NORMAL_MAX_FILES_PER_TASK, 800, NORMAL_MAX_VERIFY_STEPS),
        };
        Self {
            max_files,
            max_loc: tier.max_loc(),
            max_description_words,
            max_verify_steps,
        }
    }

    /// Each limit `task` exceeds, as `"6 files (limit 3)"`.
    pub(crate) fn exceeded_by(self, task: &TaskDef) -> Vec<String> {
        self.exceeded_with(task, task.verify.len())
    }

    /// [`Self::exceeded_by`] with `task` counted at `steps` verify steps: the
    /// steps it will have once each check of its source has one of its own
    /// (gap-4cca5f).
    pub(crate) fn exceeded_with(self, task: &TaskDef, steps: usize) -> Vec<String> {
        let mut over = Vec::new();
        if task.files.len() > self.max_files {
            over.push(format!(
                "{} files (limit {})",
                task.files.len(),
                self.max_files
            ));
        }
        if let Some(max_loc) = task.max_loc.filter(|&max_loc| max_loc > self.max_loc) {
            over.push(format!("max_loc = {max_loc} (limit {})", self.max_loc));
        }
        let words = task
            .description
            .as_deref()
            .map_or(0, |description| description.split_whitespace().count());
        if words > self.max_description_words {
            over.push(format!(
                "{words}-word description (limit {})",
                self.max_description_words
            ));
        }
        if steps > self.max_verify_steps {
            over.push(format!(
                "{steps} verify steps (limit {})",
                self.max_verify_steps
            ));
        }
        over
    }
}

impl PlanExecutionPolicy {
    /// Size limits for `tier` in this lane: the tier's defaults, never above
    /// the lane's own per-task bounds.
    #[must_use]
    pub fn tier_size_limits(&self, tier: TaskTier) -> TierSizeLimits {
        let limits = TierSizeLimits::for_tier(tier);
        TierSizeLimits {
            max_files: limits.max_files.min(self.max_files_per_task),
            max_verify_steps: limits.max_verify_steps.min(self.max_verify_steps_per_task),
            ..limits
        }
    }
}

/// Tasks over their tier's size limits (`PLAN_TIER_SIZE`), one violation per
/// task naming each limit it exceeds and the smallest higher tier it fits, if
/// any. The tier is read by [`TaskDef::tier_class`].
#[must_use]
pub fn validate_tier_sizes(
    tasks: &TasksFile,
    policy: PlanExecutionPolicy,
) -> Vec<PlanPolicyViolation> {
    tasks
        .tasks
        .iter()
        .filter_map(|task| {
            let tier = task.tier_class();
            let over = policy.tier_size_limits(tier).exceeded_by(task);
            if over.is_empty() {
                return None;
            }
            let hint = TaskTier::ALL
                .into_iter()
                .filter(|candidate| *candidate > tier)
                .find(|candidate| {
                    policy
                        .tier_size_limits(*candidate)
                        .exceeded_by(task)
                        .is_empty()
                })
                .map_or_else(
                    || "split it into smaller tasks".to_string(),
                    |fits| format!("split it into smaller tasks, or raise its tier to {fits}"),
                );
            Some(PlanPolicyViolation::task(
                task,
                "PLAN_TIER_SIZE",
                format!(
                    "{tier} task is over its tier's size limits: {}; {hint}",
                    over.join(", ")
                ),
            ))
        })
        .collect()
}

/// One actionable contract failure. These failures are deterministic and do
/// not require an agent call to diagnose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanPolicyViolation {
    pub task_id: Option<String>,
    pub code: &'static str,
    pub message: String,
}

impl PlanPolicyViolation {
    fn plan(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            task_id: None,
            code,
            message: message.into(),
        }
    }

    fn task(task: &TaskDef, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            task_id: Some(task.id.clone()),
            code,
            message: message.into(),
        }
    }

    /// Whether the plan may still run with this finding; see
    /// [`is_advisory_code`].
    #[must_use]
    pub fn is_advisory(&self) -> bool {
        is_advisory_code(self.code)
    }
}

/// Whether a finding with this code leaves the plan runnable. Tasks that
/// could run together but write overlapping files (`PLAN_CONCURRENT_OVERLAP`)
/// run safely one after the other, so that finding is for the plan's author,
/// not a reason to refuse the run.
#[must_use]
pub fn is_advisory_code(code: &str) -> bool {
    code == "PLAN_CONCURRENT_OVERLAP"
}

impl fmt::Display for PlanPolicyViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(task_id) = &self.task_id {
            write!(f, "{} [{task_id}]: {}", self.code, self.message)
        } else {
            write!(f, "{}: {}", self.code, self.message)
        }
    }
}

/// Validate structural execution budgets without touching the filesystem.
#[must_use]
pub fn validate_plan_budgets(
    tasks: &TasksFile,
    policy: PlanExecutionPolicy,
) -> Vec<PlanPolicyViolation> {
    let mut issues = Vec::new();
    if tasks.tasks.len() > policy.max_tasks {
        issues.push(PlanPolicyViolation::plan(
            "PLAN_BUDGET_TASKS",
            format!(
                "plan has {} tasks; this lane permits at most {} cohesive tasks",
                tasks.tasks.len(),
                policy.max_tasks
            ),
        ));
    }
    if (policy.bounded_context_only || tasks.meta.total != 0)
        && tasks.meta.total as usize != tasks.tasks.len()
    {
        issues.push(PlanPolicyViolation::plan(
            "PLAN_BUDGET_TOTAL",
            format!(
                "meta.total is {} but the plan defines {} tasks",
                tasks.meta.total,
                tasks.tasks.len()
            ),
        ));
    }
    if tasks.meta.max_parallel == Some(0) {
        issues.push(PlanPolicyViolation::plan(
            "PLAN_BUDGET_PARALLEL",
            "meta.max_parallel must be at least 1",
        ));
    }
    if let Some(max_parallel) = tasks.meta.max_parallel
        && max_parallel as usize > tasks.tasks.len().max(1)
    {
        issues.push(PlanPolicyViolation::plan(
            "PLAN_BUDGET_PARALLEL",
            format!(
                "meta.max_parallel is {max_parallel} but only {} tasks exist",
                tasks.tasks.len()
            ),
        ));
    }

    let mut verify_owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut file_owners: BTreeMap<String, Vec<&TaskDef>> = BTreeMap::new();
    for task in &tasks.tasks {
        check_count(
            &mut issues,
            task,
            "PLAN_BUDGET_FILES",
            "files",
            task.files.len(),
            policy.max_files_per_task,
        );
        // bug-60ad88: the documents the plan is written from, which every
        // task reads, are no part of a task's own budget; gap-5db3d9: nor
        // are its checks' tests and its sources' headers.
        check_count(
            &mut issues,
            task,
            "PLAN_BUDGET_READ_FILES",
            "context.read_files",
            task.context.as_ref().map_or(0, |context| {
                context
                    .read_files
                    .iter()
                    .filter(|read| counts_toward_read_budget(&tasks.meta.sources, task, &read.path))
                    .count()
            }),
            policy.max_read_files_for(task),
        );
        check_count(
            &mut issues,
            task,
            "PLAN_BUDGET_VERIFY",
            "verify steps",
            task.verify.len(),
            policy.max_verify_steps_per_task,
        );
        if policy.require_one_verify && task.verify.len() != 1 {
            issues.push(PlanPolicyViolation::task(
                task,
                "PLAN_VERIFY_OWNER",
                format!(
                    "the bounded lane requires exactly one runner-owned focused verify step; found {}",
                    task.verify.len()
                ),
            ));
        } else if task.verify.len() < policy.min_verify_steps_per_task {
            issues.push(PlanPolicyViolation::task(
                task,
                "PLAN_VERIFY_OWNER",
                format!(
                    "every task needs at least {} runner-owned verify step(s), one per acceptance \
                     criterion; found {}",
                    policy.min_verify_steps_per_task,
                    task.verify.len()
                ),
            ));
        }

        if let Some(context) = &task.context {
            check_count(
                &mut issues,
                task,
                "PLAN_BUDGET_SYMBOLS",
                "context.symbols",
                context.symbols.len(),
                policy.max_symbols_per_task,
            );
            check_count(
                &mut issues,
                task,
                "PLAN_BUDGET_ANTI_PATTERNS",
                "context.anti_patterns",
                context.anti_patterns.len(),
                policy.max_anti_patterns_per_task,
            );
            check_count(
                &mut issues,
                task,
                "PLAN_BUDGET_PRIOR_FAILURES",
                "context.prior_failures",
                context.prior_failures.len(),
                policy.max_prior_failures_per_task,
            );
            let context_bytes = context
                .read_files
                .iter()
                .map(|file| {
                    file.path.len() + file.why.len() + file.lines.as_deref().map_or(0, str::len)
                })
                .chain(context.symbols.iter().map(String::len))
                .chain(context.anti_patterns.iter().map(String::len))
                .chain(context.prior_failures.iter().map(String::len))
                .sum::<usize>();
            if context_bytes > policy.max_declared_context_bytes {
                issues.push(PlanPolicyViolation::task(
                    task,
                    "PLAN_BUDGET_CONTEXT",
                    format!(
                        "declared context metadata is {context_bytes} bytes; limit is {}",
                        policy.max_declared_context_bytes
                    ),
                ));
            }
        }

        for file in &task.files {
            if let Err(reason) = validate_repo_relative_path(file) {
                issues.push(PlanPolicyViolation::task(
                    task,
                    "PLAN_OUTPUT_PATH",
                    format!("unsafe output path `{file}`: {reason}"),
                ));
            }
            file_owners.entry(file.clone()).or_default().push(task);
        }
        for verify in &task.verify {
            verify_owners
                .entry(normalize_verify_command(&verify.command))
                .or_default()
                .push(task.id.clone());
        }
    }

    if policy.reject_duplicate_verify {
        for (command, owners) in verify_owners {
            if !command.is_empty() && owners.len() > 1 {
                issues.push(PlanPolicyViolation::plan(
                    "PLAN_VERIFY_DUPLICATE",
                    format!(
                        "tasks {} repeat the same verification `{command}`; assign one observable outcome and one owner",
                        owners.join(", ")
                    ),
                ));
            }
        }
    }

    if let Some(max_owners) = policy.max_serial_owners_per_file {
        let dependency_map = tasks
            .tasks
            .iter()
            .map(|task| (task.id.as_str(), task.depends_on.as_slice()))
            .collect::<HashMap<_, _>>();
        for (file, owners) in file_owners {
            // bug-daa54e: a build-registration file is a list that several
            // tasks each add an entry to, not work to merge into one owner.
            if roko_graph::exclusion::is_registration_file(&file) {
                continue;
            }
            let serial_owners = owners
                .iter()
                .filter(|owner| {
                    owners.iter().any(|other| {
                        owner.id != other.id
                            && (depends_on_transitively(
                                &dependency_map,
                                owner.id.as_str(),
                                other.id.as_str(),
                            ) || depends_on_transitively(
                                &dependency_map,
                                other.id.as_str(),
                                owner.id.as_str(),
                            ))
                    })
                })
                .map(|task| task.id.as_str())
                .collect::<BTreeSet<_>>();
            if serial_owners.len() > max_owners {
                issues.push(PlanPolicyViolation::plan(
                    "PLAN_FRAGMENTED_OWNERSHIP",
                    format!(
                        "`{file}` is split across {} serial tasks ({}); merge shared-context edits into at most {max_owners} cohesive owners",
                        serial_owners.len(),
                        serial_owners.into_iter().collect::<Vec<_>>().join(", ")
                    ),
                ));
            }
        }
    }

    // With one task at a time no two tasks run together, whatever they write.
    if plan_max_parallel(tasks) > 1 {
        issues.extend(concurrent_overlaps(tasks));
    }

    issues
}

/// `PLAN_CONCURRENT_OVERLAP`: two tasks that can run at the same time, because
/// neither depends on the other, declare overlapping `files`.
///
/// Overlap is the Graph engine's own rule (`roko_graph::exclusion`): the same
/// path, or a directory and a path inside it. In a shared working tree the
/// engine runs such tasks one after the other, so the plan gives up
/// parallelism its author meant to have. Two tasks that name the same
/// build-registration file, each adding its entry, are not flagged
/// (bug-daa54e). A task without `files` declares nothing and is never
/// flagged.
fn concurrent_overlaps(tasks: &TasksFile) -> Vec<PlanPolicyViolation> {
    let dependency_map = tasks
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task.depends_on.as_slice()))
        .collect::<HashMap<_, _>>();
    let mut issues = Vec::new();
    for (index, first) in tasks.tasks.iter().enumerate() {
        for second in &tasks.tasks[index + 1..] {
            let overlap = roko_graph::exclusion::first_overlap_except_registration(
                &first.files,
                &second.files,
            );
            let Some((first_path, second_path)) = overlap else {
                continue;
            };
            if depends_on_transitively(&dependency_map, &first.id, &second.id)
                || depends_on_transitively(&dependency_map, &second.id, &first.id)
            {
                continue;
            }
            let (first_id, second_id) = (&first.id, &second.id);
            let paths = if first_path == second_path {
                format!("both write `{first_path}`")
            } else {
                format!("write the overlapping paths `{first_path}` and `{second_path}`")
            };
            issues.push(PlanPolicyViolation::plan(
                "PLAN_CONCURRENT_OVERLAP",
                format!(
                    "tasks {first_id} and {second_id} can run at the same time and {paths}; \
                     make one depend on the other, or give them disjoint files"
                ),
            ));
        }
    }
    issues
}

/// How many of a plan's tasks may run at the same time.
///
/// `[meta] max_parallel` when the plan sets it. Omitted, as many as the plan
/// has tasks when every task that can write declares its `files`: the Graph
/// engine keeps tasks whose files overlap apart, and the DAG and
/// `[conductor] max_agents` bound the rest. Otherwise one, since what the
/// task [`task_with_unknown_writes`] names writes is unknown.
#[must_use]
pub fn plan_max_parallel(tasks: &TasksFile) -> u32 {
    match tasks.meta.max_parallel {
        Some(max_parallel) => max_parallel,
        None if task_with_unknown_writes(tasks).is_some() => 1,
        None => u32::try_from(tasks.tasks.len().max(1)).unwrap_or(u32::MAX),
    }
}

/// The first task whose role can write files but that declares none. No
/// other task may run beside it, since what it writes is unknown.
#[must_use]
pub fn task_with_unknown_writes(tasks: &TasksFile) -> Option<&TaskDef> {
    tasks.tasks.iter().find(|task| {
        let role = task.role.as_deref().unwrap_or("implementer");
        task.files.is_empty() && crate::task_parser::role_capabilities(role).write
    })
}

fn check_count(
    issues: &mut Vec<PlanPolicyViolation>,
    task: &TaskDef,
    code: &'static str,
    label: &str,
    count: usize,
    limit: usize,
) {
    if count > limit {
        issues.push(PlanPolicyViolation::task(
            task,
            code,
            format!("{label} has {count} entries; limit is {limit}"),
        ));
    }
}

/// Whether `task`'s read of `path` counts toward its read budget
/// (`PLAN_BUDGET_READ_FILES`). The documents the plan is written from,
/// `sources`, do not: every task reads them (bug-60ad88). Nor do a test one of
/// the task's checks runs ([`test_run_by`]) and the header of a source it
/// changes ([`header_beside`]), which generation declares for it
/// (gap-5db3d9).
#[must_use]
pub(crate) fn counts_toward_read_budget(sources: &[String], task: &TaskDef, path: &str) -> bool {
    !sources.iter().any(|source| source == path)
        && !task
            .verify
            .iter()
            .any(|step| test_run_by(&step.command, path))
        && !header_beside(path, &task.files)
}

/// The name a test file goes by: its file name up to the first `.`, without
/// its test affix, as `presets` of `presets_test.cpp`, `lowpass` of
/// `test_lowpass.py` and `cart` of `cart.test.ts`. `None` for a file not
/// named as a test.
#[must_use]
pub(crate) fn test_affix_stem(path: &str) -> Option<&str> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let (stem, extensions) = name.split_once('.')?;
    let stem = if extensions.starts_with("test.") || extensions.starts_with("spec.") {
        stem
    } else {
        ["_tests", "_test", "_spec"]
            .iter()
            .find_map(|affix| stem.strip_suffix(affix))
            .or_else(|| stem.strip_prefix("test_"))?
    };
    (!stem.is_empty()).then_some(stem)
}

/// Whether the check `command` runs the test file `path` (gap-5db3d9): one
/// of its commands has for a word the file's path; or, for a file named as a
/// test, its name without the extension, as a program or a target names it
/// (`ctest -R presets_test`, `cargo test --test widget_tests`); or its name
/// without the test affix ([`test_affix_stem`]) and, for another word, one of
/// the file's directories, or that directory after a prefix of letters. So
/// `tools/spec.sh 5_2 presets` runs `tests/spec/p5_2/presets_test.cpp`, and
/// not `tests/spec/p5/presets_test.cpp`, which `tools/spec.sh 5 presets` runs.
#[must_use]
pub(crate) fn test_run_by(command: &str, path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let target = name.split('.').next().unwrap_or(name);
    let stem = test_affix_stem(path);
    let directories: Vec<&str> = path.split('/').rev().skip(1).collect();
    let names_directory = |word: &str| {
        !word.is_empty()
            && directories.iter().any(|directory| {
                directory
                    .strip_suffix(word)
                    .is_some_and(|prefix| prefix.chars().all(|c| c.is_ascii_alphabetic()))
            })
    };
    command.split(['&', '|', ';', '\n']).any(|part| {
        let words: Vec<&str> = part
            .split_whitespace()
            .map(|word| word.trim_matches(['\'', '"']).trim_start_matches("./"))
            .collect();
        words.iter().any(|word| {
            *word == path || (stem.is_some() && word.rsplit('/').next() == Some(target))
        }) || stem.is_some_and(|stem| {
            words.contains(&stem) && words.iter().any(|word| names_directory(word))
        })
    })
}

/// Whether `header` is the header of one of the C or C++ sources `files`
/// (gap-5db3d9): the source's name with a header's extension, in the source's
/// directory or under the `include` directory beside the `src` directory the
/// source is in, as `core/include/undertow/presets.h` is of
/// `core/src/presets.cpp`.
#[must_use]
pub(crate) fn header_beside(header: &str, files: &[String]) -> bool {
    let extension_in = |path: &Path, extensions: &[&str]| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
    };
    let header = Path::new(header);
    if !extension_in(header, C_HEADERS) {
        return false;
    }
    let header_dir = header.parent().unwrap_or(Path::new(""));
    files.iter().map(Path::new).any(|source| {
        let source_dir = source.parent().unwrap_or(Path::new(""));
        extension_in(source, CMAKE_SOURCES)
            && source.file_stem() == header.file_stem()
            && (source_dir == header_dir || include_beside_src(source_dir, header_dir))
    })
}

/// Whether `header_dir` is under the `include` directory beside the `src`
/// directory `source_dir` is in or under (`core/include/undertow` for
/// `core/src`).
fn include_beside_src(source_dir: &Path, header_dir: &Path) -> bool {
    let mut root = PathBuf::new();
    for part in source_dir.components() {
        if part.as_os_str() == "src" {
            return header_dir.starts_with(root.join("include"));
        }
        root.push(part);
    }
    false
}

/// Validate paths, regular-file identity, and explicit symbol anchors before
/// the plan enters the runner.
#[must_use]
pub fn validate_plan_context(
    tasks: &TasksFile,
    workspace_root: &Path,
    plan_dir: &Path,
    policy: PlanExecutionPolicy,
) -> Vec<PlanPolicyViolation> {
    let mut issues = validate_plan_budgets(tasks, policy);
    let canonical_root = match workspace_root.canonicalize() {
        Ok(root) => root,
        Err(error) => {
            issues.push(PlanPolicyViolation::plan(
                "PLAN_CONTEXT_ROOT",
                format!(
                    "cannot resolve workspace root {}: {error}",
                    workspace_root.display()
                ),
            ));
            return issues;
        }
    };
    let task_by_id = tasks
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect::<HashMap<_, _>>();

    for task in &tasks.tasks {
        let Some(context) = &task.context else {
            continue;
        };
        let dependency_outputs = transitive_dependency_outputs(task, &task_by_id);
        let mut readable_contents = Vec::new();
        for read_file in &context.read_files {
            let relative = match validate_repo_relative_path(&read_file.path) {
                Ok(path) => path,
                Err(reason) => {
                    issues.push(PlanPolicyViolation::task(
                        task,
                        "PLAN_CONTEXT_PATH",
                        format!("unsafe read_files path `{}`: {reason}", read_file.path),
                    ));
                    continue;
                }
            };
            let full_path = workspace_root.join(&relative);
            if !full_path.exists() {
                if !dependency_outputs.contains(&read_file.path) {
                    issues.push(PlanPolicyViolation::task(
                        task,
                        "PLAN_CONTEXT_MISSING",
                        format!(
                            "declared context file `{}` does not exist and no dependency produces it",
                            read_file.path
                        ),
                    ));
                }
                continue;
            }
            match validate_regular_file(&full_path, &canonical_root) {
                Ok(()) => match std::fs::read_to_string(&full_path) {
                    Ok(content) => readable_contents.push((read_file.path.as_str(), content)),
                    Err(error) => issues.push(PlanPolicyViolation::task(
                        task,
                        "PLAN_CONTEXT_READ",
                        format!("cannot read `{}` as UTF-8 text: {error}", read_file.path),
                    )),
                },
                Err(reason) => issues.push(PlanPolicyViolation::task(
                    task,
                    "PLAN_CONTEXT_FILE_TYPE",
                    format!("invalid context file `{}`: {reason}", read_file.path),
                )),
            }
            if let Some(range) = read_file.lines.as_deref() {
                // A range in a file an earlier task changes is fitted to the
                // file once that task has run (bug-7ff055): the file's
                // length now says nothing about the range the task reads.
                let changed_earlier = dependency_outputs.contains(&read_file.path);
                match parse_line_range(range, usize::MAX) {
                    Ok((start, end)) => {
                        let line_count = std::fs::read_to_string(&full_path)
                            .map(|content| content.lines().count())
                            .unwrap_or(0);
                        let resolved_end = end.unwrap_or(line_count);
                        let reversed = end.is_some_and(|end| start > end);
                        let outside =
                            start == 0 || start > resolved_end || resolved_end > line_count;
                        if reversed || (outside && !changed_earlier) {
                            issues.push(PlanPolicyViolation::task(
                                task,
                                "PLAN_CONTEXT_RANGE",
                                format!(
                                    "line range `{range}` is outside `{}` ({} lines)",
                                    read_file.path, line_count
                                ),
                            ));
                        } else if resolved_end.saturating_sub(start).saturating_add(1)
                            > policy.max_range_lines
                        {
                            issues.push(PlanPolicyViolation::task(
                                task,
                                "PLAN_CONTEXT_RANGE_BUDGET",
                                format!(
                                    "line range `{range}` spans more than {} lines",
                                    policy.max_range_lines
                                ),
                            ));
                        }
                    }
                    Err(reason) => issues.push(PlanPolicyViolation::task(
                        task,
                        "PLAN_CONTEXT_RANGE",
                        format!("invalid line range `{range}`: {reason}"),
                    )),
                }
            }
        }

        for symbol in &context.symbols {
            let Some(anchor) = explicit_symbol_anchor(symbol) else {
                continue;
            };
            let found = readable_contents
                .iter()
                .any(|(_, content)| find_anchor_line(content, &anchor).is_some());
            if !found
                && !context
                    .read_files
                    .iter()
                    .any(|read_file| dependency_outputs.contains(&read_file.path))
            {
                issues.push(PlanPolicyViolation::task(
                    task,
                    "PLAN_CONTEXT_SYMBOL",
                    format!(
                        "explicit symbol anchor `{anchor}` was not found in any declared read_files entry"
                    ),
                ));
            }
        }
    }

    if !plan_dir.join("tasks.toml").is_file() {
        issues.push(PlanPolicyViolation::plan(
            "PLAN_ARTIFACT_MISSING",
            format!("{} is missing tasks.toml", plan_dir.display()),
        ));
    }
    issues
}

/// Fit `task`'s declared line ranges to its files as `workspace_root`, a
/// try's checkout, holds them ([`fit_range`]), whichever task changed them
/// (bug-fa22cc). The planner wrote the ranges before the plan's work: an
/// earlier task may have changed a file since (bug-7ff055), and so may the
/// task's own last try, whose checkout a retry reuses, as phase 5.1's T2#1
/// rewrote `tools/render-demos.sh` from 121 lines to 43 and T2#2 to #4 died
/// on `1-121`. A malformed range is left for [`render_declared_context`] to
/// refuse. Returns one line per range it changed, for the log.
pub(crate) fn fit_ranges_to_checkout(task: &mut TaskDef, workspace_root: &Path) -> Vec<String> {
    let Some(context) = task.context.as_mut() else {
        return Vec::new();
    };
    context
        .read_files
        .iter_mut()
        .filter_map(|read_file| fit_range(read_file, workspace_root))
        .collect()
}

/// Make each explicit symbol anchor of `task` that none of its declared
/// files holds, as `workspace_root`, a try's checkout, has them, a note
/// rather than an anchor ([`symbol_note`], bug-41791b). The planner wrote the
/// anchors before the plan's work, and an earlier task, or the task's own last
/// try, may have renamed or removed the symbol since: the entry keeps its
/// text, and the try's prompt shows the files as they are. A task that
/// declares no file reads none to anchor in, and is left alone. Returns one
/// line per anchor it dropped, for the log.
pub(crate) fn drop_missing_anchors(task: &mut TaskDef, workspace_root: &Path) -> Vec<String> {
    let Some(context) = task.context.as_mut() else {
        return Vec::new();
    };
    if context.read_files.is_empty() {
        return Vec::new();
    }
    let texts: Vec<String> = context
        .read_files
        .iter()
        .filter_map(|read_file| validate_repo_relative_path(&read_file.path).ok())
        .filter_map(|relative| std::fs::read_to_string(workspace_root.join(relative)).ok())
        .collect();
    let mut dropped = Vec::new();
    for symbol in &mut context.symbols {
        let Some(anchor) = explicit_symbol_anchor(symbol) else {
            continue;
        };
        if texts
            .iter()
            .any(|text| find_anchor_line(text, &anchor).is_some())
        {
            continue;
        }
        dropped.push(format!(
            "`{anchor}` is in no file the try reads, so its symbol entry is a note, not an anchor"
        ));
        *symbol = symbol_note(symbol, &anchor);
    }
    dropped
}

/// Leave out of `task`'s declared reads each file that `workspace_root`, a
/// try's checkout, no longer holds (bug-41791b): the task's own last try, or
/// an earlier task, may have removed or renamed it since the plan was
/// written, and a read of a missing file would fail the try's prompt. A path
/// that is not a safe relative path stays, for [`render_declared_context`] to
/// refuse. Returns one line per read it left out, for the log.
pub(crate) fn drop_missing_reads(task: &mut TaskDef, workspace_root: &Path) -> Vec<String> {
    let Some(context) = task.context.as_mut() else {
        return Vec::new();
    };
    let mut dropped = Vec::new();
    context.read_files.retain(|read_file| {
        let missing = validate_repo_relative_path(&read_file.path)
            .is_ok_and(|relative| !workspace_root.join(relative).exists());
        if missing {
            dropped.push(format!(
                "`{}` is not in the try's checkout, so the try does not read it",
                read_file.path
            ));
        }
        !missing
    });
    dropped
}

/// Fit `plan`'s declared line ranges in files no earlier task of the plan
/// changes to those files as `workspace_root` holds them (gap-b95913), the
/// way [`fit_ranges_to_checkout`] fits every range of a try: the planner
/// guessed how long the file is. Returns one line per range it changed,
/// naming its task.
pub(crate) fn fit_ranges_to_files(plan: &mut TasksFile, workspace_root: &Path) -> Vec<String> {
    let changed_earlier: Vec<HashSet<String>> = {
        let task_by_id = plan
            .tasks
            .iter()
            .map(|task| (task.id.as_str(), task))
            .collect::<HashMap<_, _>>();
        plan.tasks
            .iter()
            .map(|task| transitive_dependency_outputs(task, &task_by_id))
            .collect()
    };
    let mut fitted = Vec::new();
    for (task, changed_earlier) in plan.tasks.iter_mut().zip(changed_earlier) {
        let Some(context) = task.context.as_mut() else {
            continue;
        };
        for read_file in &mut context.read_files {
            if !changed_earlier.contains(&read_file.path)
                && let Some(change) = fit_range(read_file, workspace_root)
            {
                fitted.push(format!("{}: {change}", task.id));
            }
        }
    }
    fitted
}

/// Fit `read_file`'s line range to its file as `workspace_root` holds it
/// ([`fit_line_range`]): a range that runs past the end of the file is cut at
/// its last line, and one that starts past it, or runs from line 1 past it,
/// becomes an unranged entry, which shows the file within the context budget.
/// A range inside the file, or a malformed one, is left alone. Returns what
/// changed, for the log.
fn fit_range(read_file: &mut ReadFile, workspace_root: &Path) -> Option<String> {
    let range = read_file.lines.clone()?;
    let relative = validate_repo_relative_path(&read_file.path).ok()?;
    let content = std::fs::read_to_string(workspace_root.join(relative)).ok()?;
    let line_count = content.lines().count();
    let lines = match fit_line_range(&range, line_count).ok()? {
        FittedRange::Within(..) => return None,
        FittedRange::Cut(start, end) => Some(format!("{start}-{end}")),
        FittedRange::Whole => None,
    };
    let change = format!(
        "`{range}` of `{}` ({line_count} lines) became {}",
        read_file.path,
        lines.as_deref().map_or_else(
            || "the whole file".to_string(),
            |lines| format!("`{lines}`")
        )
    );
    read_file.lines = lines;
    Some(change)
}

/// Render exact, line-numbered context for prompt assembly. Unlike preflight,
/// dispatch has no future-output exception: every dependency should already be
/// materialized in the attempt worktree. The dispatcher has fitted the ranges
/// to the files there ([`fit_ranges_to_checkout`]), and a range past the end
/// of its file is fitted here too, never refused ([`fit_line_range`],
/// bug-fa22cc).
pub fn render_declared_context(
    task: &TaskDef,
    workspace_root: &Path,
    policy: PlanExecutionPolicy,
) -> Result<String, String> {
    let Some(context) = &task.context else {
        return Ok(String::new());
    };
    if context.read_files.is_empty() {
        return Ok(String::new());
    }
    if context.read_files.len() > policy.max_read_files_per_task {
        return Err(format!(
            "task {} declares {} read files; limit is {}",
            task.id,
            context.read_files.len(),
            policy.max_read_files_per_task
        ));
    }

    let canonical_root = workspace_root
        .canonicalize()
        .map_err(|error| format!("resolve workspace root: {error}"))?;
    // An anchor no declared file holds shows nothing, and never fails the
    // try: the plan's work may have renamed it (bug-41791b), and the
    // dispatcher makes such an entry a note ([`drop_missing_anchors`]).
    let anchors = context
        .symbols
        .iter()
        .filter_map(|symbol| explicit_symbol_anchor(symbol))
        .collect::<Vec<_>>();
    let mut output = String::from(
        "# Declared source context (authoritative and bounded)\n\
         Use these supplied snippets directly. Do not search home directories, session history, \
         other worktrees, Git unreachable objects, or the web. If required context is absent, \
         stop and request plan repair instead of exploring.\n",
    );

    // Explicit ranges and symbol anchors render first, so their size is known:
    // they fail the task when they overrun the budget. Each file declared
    // without a range then gets a fair share of what is left, and is cut to
    // fit, with a marker, rather than fail the task (bug-7c8a57).
    let mut blocks = Vec::with_capacity(context.read_files.len());
    let mut unranged = Vec::new();
    for read_file in &context.read_files {
        let relative = validate_repo_relative_path(&read_file.path)
            .map_err(|reason| format!("unsafe context path `{}`: {reason}", read_file.path))?;
        let full_path = workspace_root.join(relative);
        validate_regular_file(&full_path, &canonical_root)
            .map_err(|reason| format!("invalid context file `{}`: {reason}", read_file.path))?;
        let content = std::fs::read_to_string(&full_path)
            .map_err(|error| format!("read context file `{}`: {error}", read_file.path))?;
        let lines = content.lines().collect::<Vec<_>>();
        let mut ranges = Vec::<(usize, usize)>::new();
        // A range past the end of the file is fitted to it, never refused
        // (bug-fa22cc): the file changed since the plan was written.
        if let Some((start, end)) = fitted_lines(read_file, lines.len())? {
            if end.saturating_sub(start).saturating_add(1) > policy.max_range_lines {
                let range = read_file.lines.as_deref().unwrap_or_default();
                return Err(format!(
                    "line range `{range}` exceeds the {} line context limit",
                    policy.max_range_lines
                ));
            }
            ranges.push((start, end));
        }
        for anchor in &anchors {
            if let Some(line) = find_anchor_line(&content, anchor) {
                let start = line.saturating_sub(ANCHOR_CONTEXT_RADIUS).max(1);
                let end = line.saturating_add(ANCHOR_CONTEXT_RADIUS).min(lines.len());
                ranges.push((start, end));
            }
        }
        if ranges.is_empty() {
            unranged.push((blocks.len(), read_file, content));
            blocks.push(String::new());
            continue;
        }
        let block = merge_ranges(ranges)
            .into_iter()
            .map(|(start, end)| declared_block(read_file, &lines, start, end))
            .collect::<String>();
        blocks.push(block);
    }
    let fixed = output.len() + blocks.iter().map(String::len).sum::<usize>();
    if fixed > policy.max_declared_context_bytes {
        return Err(format!(
            "declared snippets for task {} exceed the {} byte prompt budget; narrow read_files ranges",
            task.id, policy.max_declared_context_bytes
        ));
    }

    let mut remaining = policy.max_declared_context_bytes - fixed;
    let mut left = unranged.len();
    for (index, read_file, content) in unranged {
        let block = unranged_block(
            read_file,
            &content,
            policy.max_range_lines,
            remaining / left,
        );
        remaining = remaining.saturating_sub(block.len());
        left -= 1;
        blocks[index] = block;
    }
    output.extend(blocks);
    Ok(output)
}

/// [`render_declared_context`] for the prompt's own section of declared
/// files (bug-a8a787): the same files, checked the same way, fitted into
/// `budget` bytes rather than cut by the prompt's cap. The plan's `sources`
/// (its `[meta] sources`, the documents it is written from) come first and
/// are cut last; the other reads keep their order. A read cut short, or left
/// out, is followed by a line naming the lines not shown, for the agent to
/// read itself: `[docs/specs/phase-4-ui.md lines 212-264 not shown: read_file
/// them]`. A file declared without a range shows up to the policy's line
/// limit, and one whose range starts at its first line names the lines it has
/// past that range (bug-c5f406).
pub fn render_declared_files(
    task: &TaskDef,
    workspace_root: &Path,
    policy: PlanExecutionPolicy,
    budget: usize,
    sources: &[String],
) -> Result<String, String> {
    // The checks, and the errors that fail the task, stay the plan policy's.
    let checked = render_declared_context(task, workspace_root, policy)?;
    let Some(context) = task.context.as_ref().filter(|_| !checked.is_empty()) else {
        return Ok(String::new());
    };
    let anchors: Vec<String> = context
        .symbols
        .iter()
        .filter_map(|symbol| explicit_symbol_anchor(symbol))
        .collect();
    let mut reads = Vec::with_capacity(context.read_files.len());
    for (order, read_file) in context.read_files.iter().enumerate() {
        let content = std::fs::read_to_string(workspace_root.join(&read_file.path))
            .map_err(|error| format!("read context file `{}`: {error}", read_file.path))?;
        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();
        let mut ranges = Vec::new();
        // A range from the first line reads the file from its top, and the
        // lines a file has grown past it, on a try's kept tree or by an
        // earlier task, are named rather than left out without a word
        // (bug-c5f406). A range that starts further in is a part the planner
        // chose.
        let mut past_range = None;
        if let Some((start, end)) = fitted_lines(read_file, total)? {
            ranges.push((start, end));
            if start == 1 && end < total {
                past_range = Some((end + 1, total));
            }
        }
        for anchor in &anchors {
            if let Some(line) = find_anchor_line(&content, anchor) {
                let start = line.saturating_sub(ANCHOR_CONTEXT_RADIUS).max(1);
                ranges.push((start, line.saturating_add(ANCHOR_CONTEXT_RADIUS).min(total)));
            }
        }
        // A file without a range shows whole, to the policy's line limit.
        let mut hidden = Vec::new();
        if ranges.is_empty() && total > 0 {
            ranges.push((1, total.min(policy.max_range_lines)));
            if total > policy.max_range_lines {
                hidden.push((policy.max_range_lines + 1, total));
            }
        }
        let ranges = merge_ranges(ranges);
        if let Some((from, to)) = past_range {
            hidden.extend(lines_not_in(from, to, &ranges));
        }
        let blocks = ranges
            .into_iter()
            .filter(|(start, end)| start <= end)
            .map(|(start, end)| {
                let shown = (start..=end)
                    .filter_map(|number| {
                        lines.get(number - 1).map(|line| {
                            // The prompt shows a tab as four spaces; measure it so.
                            numbered_line(number, &line.replace('\t', "    "))
                        })
                    })
                    .collect();
                (start, end, shown)
            })
            .collect();
        reads.push(DeclaredRead {
            read_file,
            total,
            blocks,
            hidden,
            source: sources.iter().any(|source| *source == read_file.path),
            order,
        });
    }
    Ok(fit_declared_reads(reads, budget))
}

/// A declared file as the prompt shows it: its blocks, each `(start, end,
/// numbered lines)`, and the ranges it leaves out.
struct DeclaredRead<'a> {
    read_file: &'a ReadFile,
    total: usize,
    blocks: Vec<(usize, usize, Vec<String>)>,
    hidden: Vec<(usize, usize)>,
    source: bool,
    order: usize,
}

/// The heading of the declared files the prompt shows (bug-a8a787).
const DECLARED_FILES_INTRO: &str = "# Declared source context (authoritative and bounded)\n\
     Use these supplied snippets directly. Do not search home directories, session history, \
     other worktrees, Git unreachable objects, or the web. A file cut short names the lines \
     not shown: read those with read_file.\n";

/// The line that names what of `read` is not shown: `ranges`, each `(start,
/// end)`.
fn not_shown_line(read: &DeclaredRead<'_>, ranges: &[(usize, usize)]) -> String {
    let ranges = ranges
        .iter()
        .map(|(start, end)| {
            if start == end {
                start.to_string()
            } else {
                format!("{start}-{end}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let lines = if ranges.contains(['-', ',']) {
        "lines"
    } else {
        "line"
    };
    format!(
        "[{} {lines} {ranges} not shown: read_file them]\n",
        read.read_file.path
    )
}

/// `reads` fitted into `budget` bytes, sources first: each block whole while
/// it fits, else as many of its lines as do, and a line naming what is not
/// shown. Room for every such line is kept back first, so a cut can always
/// say what it left out.
fn fit_declared_reads(mut reads: Vec<DeclaredRead<'_>>, budget: usize) -> String {
    reads.sort_by_key(|read| (!read.source, read.order));
    let reserve: usize = reads
        .iter()
        .map(|read| {
            // The longest line it can need: every range at the file's end.
            let worst = vec![(read.total, read.total); read.blocks.len() + read.hidden.len()];
            not_shown_line(read, &worst).len()
        })
        .sum();
    let mut room = budget.saturating_sub(DECLARED_FILES_INTRO.len() + reserve);
    let mut output = String::from(DECLARED_FILES_INTRO);
    for read in &reads {
        let mut hidden = read.hidden.clone();
        for (start, end, lines) in &read.blocks {
            let frame = declared_file_tag(read.read_file, *start, *end, read.total).len()
                + DECLARED_FILE_END.len();
            let body: usize = lines.iter().map(String::len).sum();
            if frame + body <= room {
                output.push_str(&declared_file_tag(read.read_file, *start, *end, read.total));
                output.extend(lines.iter().map(String::as_str));
                output.push_str(DECLARED_FILE_END);
                room -= frame + body;
                continue;
            }
            let mut used = frame;
            let shown = lines
                .iter()
                .take_while(|line| {
                    let fits = used + line.len() <= room;
                    if fits {
                        used += line.len();
                    }
                    fits
                })
                .count();
            if shown > 0 {
                let last = start + shown - 1;
                output.push_str(&declared_file_tag(read.read_file, *start, last, read.total));
                output.extend(lines[..shown].iter().map(String::as_str));
                output.push_str(DECLARED_FILE_END);
                room = room.saturating_sub(used);
            }
            if start + shown <= *end {
                hidden.push((start + shown, *end));
            }
        }
        if !hidden.is_empty() {
            output.push_str(&not_shown_line(read, &merge_ranges(hidden)));
        }
    }
    output
}

/// The closing tag of a declared file block.
const DECLARED_FILE_END: &str = "</declared-file>\n";

/// The opening tag of a declared file block showing lines `start..=end` of a
/// file `total` lines long.
fn declared_file_tag(read_file: &ReadFile, start: usize, end: usize, total: usize) -> String {
    format!(
        "\n<declared-file path=\"{}\" lines=\"{start}-{end}\" total=\"{total}\" why=\"{}\">\n",
        read_file.path,
        read_file.why.replace('"', "'")
    )
}

/// Line `line_number` of a declared file, numbered.
fn numbered_line(line_number: usize, line: &str) -> String {
    format!("{line_number:>6} | {line}\n")
}

/// Lines `start..=end` of the declared file `lines` as a block.
fn declared_block(read_file: &ReadFile, lines: &[&str], start: usize, end: usize) -> String {
    let mut block = declared_file_tag(read_file, start, end, lines.len());
    for line_number in start..=end {
        if let Some(line) = lines.get(line_number.saturating_sub(1)) {
            block.push_str(&numbered_line(line_number, line));
        }
    }
    block.push_str(DECLARED_FILE_END);
    block
}

/// The line that ends a declared file cut short after its first `shown` of
/// `total` lines.
fn truncation_marker(shown: usize, total: usize) -> String {
    if shown == 0 {
        format!(
            "[... truncated: showed none of its {total} lines; add a lines range to read_files]\n"
        )
    } else {
        format!(
            "[... truncated: showed lines 1-{shown} of {total}; add a lines range to read_files]\n"
        )
    }
}

/// A `read_files` entry declared without a range: the whole file when it has
/// at most `max_lines` lines and its block fits in `share` bytes; else its
/// first lines that do, and a marker saying the rest was cut. Only a share
/// too small for the tag and the marker overruns it.
fn unranged_block(read_file: &ReadFile, content: &str, max_lines: usize, share: usize) -> String {
    let lines = content.lines().collect::<Vec<_>>();
    let total = lines.len();
    if total <= max_lines {
        let whole = declared_block(read_file, &lines, 1, total);
        if whole.len() <= share {
            return whole;
        }
    }
    let end = total.min(max_lines);
    let frame = declared_file_tag(read_file, 1, end, total).len()
        + truncation_marker(end, total).len()
        + DECLARED_FILE_END.len();
    let mut room = share.saturating_sub(frame);
    let mut body = String::new();
    let mut shown = 0;
    for (line_number, line) in (1..=end).zip(&lines) {
        let numbered = numbered_line(line_number, line);
        if numbered.len() > room {
            break;
        }
        room -= numbered.len();
        body.push_str(&numbered);
        shown = line_number;
    }
    let mut block = declared_file_tag(read_file, 1, shown, total);
    block.push_str(&body);
    block.push_str(&truncation_marker(shown, total));
    block.push_str(DECLARED_FILE_END);
    block
}

/// Return a normalized repository-relative path or a precise rejection.
fn validate_repo_relative_path(path: &str) -> Result<PathBuf, &'static str> {
    if path.trim().is_empty() {
        return Err("path is empty");
    }
    if path.contains('\0') {
        return Err("path contains NUL");
    }
    if path
        .chars()
        .any(|character| matches!(character, '*' | '?' | '[' | ']'))
    {
        return Err("globs are not allowed in an exact context contract");
    }
    if path.contains('\\') {
        return Err("backslash path separators are not allowed");
    }
    let path = Path::new(path);
    if path.is_absolute() {
        return Err("absolute paths are not repository-relative");
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => return Err("`.` components are not allowed"),
            Component::ParentDir => return Err("`..` components are not allowed"),
            Component::RootDir | Component::Prefix(_) => {
                return Err("path escapes the repository root");
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        Err("path is empty")
    } else {
        Ok(normalized)
    }
}

fn validate_regular_file(path: &Path, canonical_root: &Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("metadata unavailable: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err("symlinks are not accepted as declared context".to_string());
    }
    if !metadata.file_type().is_file() {
        return Err("path is not a regular file".to_string());
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize: {error}"))?;
    if !canonical.starts_with(canonical_root) {
        return Err("canonical path escapes the repository root".to_string());
    }
    Ok(())
}

pub(crate) fn transitive_dependency_outputs(
    task: &TaskDef,
    task_by_id: &HashMap<&str, &TaskDef>,
) -> HashSet<String> {
    let mut outputs = HashSet::new();
    let mut pending = task
        .depends_on
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut visited = HashSet::new();
    while let Some(task_id) = pending.pop() {
        if !visited.insert(task_id) {
            continue;
        }
        if let Some(dependency) = task_by_id.get(task_id) {
            outputs.extend(dependency.files.iter().cloned());
            pending.extend(dependency.depends_on.iter().map(String::as_str));
        }
    }
    outputs
}

fn depends_on_transitively(
    dependencies: &HashMap<&str, &[String]>,
    task_id: &str,
    candidate_dependency: &str,
) -> bool {
    let mut pending = dependencies
        .get(task_id)
        .into_iter()
        .flat_map(|deps| deps.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let mut visited = HashSet::new();
    while let Some(dependency) = pending.pop() {
        if dependency == candidate_dependency {
            return true;
        }
        if visited.insert(dependency) {
            pending.extend(
                dependencies
                    .get(dependency)
                    .into_iter()
                    .flat_map(|deps| deps.iter().map(String::as_str)),
            );
        }
    }
    false
}

fn normalize_verify_command(command: &str) -> String {
    command.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A declared read's line range fitted to its file (bug-fa22cc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FittedRange {
    /// Lines `start..=end`, inside the file as declared.
    Within(usize, usize),
    /// Lines `start..=end`: the declared range ran past the file's end and
    /// is cut at its last line.
    Cut(usize, usize),
    /// The whole file: the declared range started past its end, or ran from
    /// line 1 past it.
    Whole,
}

/// `range`, a declared read's line range, fitted to a file of `line_count`
/// lines (bug-fa22cc). A range past the end was written for the file as it
/// was before a task, or the task's own last try, changed it: one that starts
/// past the end, or runs from line 1 past it, reads the whole file, and
/// another one that runs past the end is cut at the last line. `Err` for a
/// malformed range: no numbers, line 0, or an end before its start.
fn fit_line_range(range: &str, line_count: usize) -> Result<FittedRange, String> {
    let (start, end) = parse_line_range(range, usize::MAX)?;
    if end.is_some_and(|end| start > end) {
        return Err("the range ends before it starts".to_string());
    }
    let end = end.unwrap_or(line_count);
    Ok(if start > line_count || (start == 1 && end > line_count) {
        FittedRange::Whole
    } else if end > line_count {
        FittedRange::Cut(start, line_count)
    } else {
        FittedRange::Within(start, end)
    })
}

/// The lines `read_file`'s declared range shows of its file of `line_count`
/// lines, fitted to it ([`fit_line_range`]): `None` without a range, or for
/// one that shows the whole file. `Err` names a malformed range.
fn fitted_lines(read_file: &ReadFile, line_count: usize) -> Result<Option<(usize, usize)>, String> {
    let Some(range) = read_file.lines.as_deref() else {
        return Ok(None);
    };
    match fit_line_range(range, line_count) {
        Ok(FittedRange::Within(start, end) | FittedRange::Cut(start, end)) => {
            Ok(Some((start, end)))
        }
        Ok(FittedRange::Whole) => Ok(None),
        Err(reason) => Err(format!(
            "invalid line range `{range}` of `{}`: {reason}",
            read_file.path
        )),
    }
}

pub(crate) fn parse_line_range(
    range: &str,
    line_count: usize,
) -> Result<(usize, Option<usize>), String> {
    let trimmed = range.trim();
    let (start, end) = trimmed
        .split_once('-')
        .ok_or_else(|| "expected START-END or START-".to_string())?;
    if end.contains('-') {
        return Err("range contains more than one separator".to_string());
    }
    let start = start
        .trim()
        .parse::<usize>()
        .map_err(|_| "start is not a positive integer".to_string())?;
    if start == 0 {
        return Err("line numbers are one-based".to_string());
    }
    let end = if end.trim().is_empty() {
        (line_count != usize::MAX).then_some(line_count)
    } else {
        Some(
            end.trim()
                .parse::<usize>()
                .map_err(|_| "end is not a positive integer".to_string())?,
        )
    };
    if end == Some(0) {
        return Err("line numbers are one-based".to_string());
    }
    Ok((start, end))
}

/// `symbol`, an anchor no file the task reads holds, written as a note that
/// is no anchor: `Look::drawStaticLayer (in no file the task reads): <what
/// the planner said of it>`. Plan generation writes it for a symbol no file
/// holds yet (bug-417c4a), and a try for one its files no longer hold
/// (bug-41791b).
pub(crate) fn symbol_note(symbol: &str, anchor: &str) -> String {
    let trimmed = symbol.trim();
    let said = trimmed
        .split_once('—')
        .map(|(_, said)| said.trim())
        .or_else(|| {
            trimmed
                .strip_prefix('`')
                .and_then(|rest| rest.split_once('`'))
                .map(|(_, said)| said.trim_start_matches([' ', ':', '-']).trim())
        })
        .filter(|said| !said.is_empty());
    match said {
        Some(said) => format!("{anchor} (in no file the task reads): {said}"),
        None => format!("{anchor} (in no file the task reads)"),
    }
}

pub(crate) fn explicit_symbol_anchor(symbol: &str) -> Option<String> {
    let trimmed = symbol.trim();
    let candidate = if let Some(exact) = trimmed.strip_prefix("exact:") {
        exact.trim()
    } else if let Some(rest) = trimmed.strip_prefix('`') {
        rest.split_once('`')?.0.trim()
    } else if let Some((anchor, _)) = trimmed.split_once('—') {
        anchor.trim()
    } else if !trimmed.chars().any(char::is_whitespace) {
        trimmed
    } else {
        return None;
    };
    let candidate = candidate
        .trim_end_matches("()")
        .trim_matches(|character: char| matches!(character, '`' | ',' | ';'));
    if candidate.is_empty()
        || candidate.split("::").any(|segment| {
            segment.is_empty()
                || !segment.chars().enumerate().all(|(index, character)| {
                    character == '_'
                        || character.is_ascii_alphanumeric()
                            && (index > 0 || !character.is_ascii_digit())
                })
        })
    {
        None
    } else {
        Some(candidate.to_string())
    }
}

pub(crate) fn find_anchor_line(content: &str, anchor: &str) -> Option<usize> {
    let final_segment = anchor.rsplit("::").next().unwrap_or(anchor);
    content.lines().enumerate().find_map(|(index, line)| {
        (contains_identifier(line, anchor) || contains_identifier(line, final_segment))
            .then_some(index + 1)
    })
}

pub(crate) fn contains_identifier(line: &str, identifier: &str) -> bool {
    line.match_indices(identifier).any(|(start, _)| {
        let before = line[..start].chars().next_back();
        let end = start + identifier.len();
        let after = line[end..].chars().next();
        before.is_none_or(|character| !is_identifier_character(character))
            && after.is_none_or(|character| !is_identifier_character(character))
    })
}

const fn is_identifier_character(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn merge_ranges(mut ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    ranges.sort_unstable();
    let mut merged = Vec::<(usize, usize)>::new();
    for (start, end) in ranges {
        if let Some((_, previous_end)) = merged.last_mut()
            && start <= previous_end.saturating_add(1)
        {
            *previous_end = (*previous_end).max(end);
        } else {
            merged.push((start, end));
        }
    }
    merged
}

/// The parts of lines `from..=to` that none of `ranges`, sorted and merged,
/// covers.
fn lines_not_in(from: usize, to: usize, ranges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut parts = Vec::new();
    let mut next = from;
    for &(start, end) in ranges {
        if start > to {
            break;
        }
        if start > next {
            parts.push((next, start - 1));
        }
        next = next.max(end.saturating_add(1));
    }
    if next <= to {
        parts.push((next, to));
    }
    parts
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_parser::{ReadFile, TaskContext, TaskMeta, VerifyStep};
    use tempfile::tempdir;

    fn task() -> TaskDef {
        TaskDef {
            id: "T1".into(),
            title: "Edit exact symbol".into(),
            description: Some("Update the existing widget.".into()),
            role: Some("implementer".into()),
            status: "ready".into(),
            tier: "focused".into(),
            frequency: None,
            model_hint: None,
            replan_strategy: None,
            max_loc: Some(50),
            files: vec!["src/lib.rs".into()],
            allowed_tools: None,
            denied_tools: None,
            mcp_servers: None,
            depends_on: vec![],
            depends_on_plan: vec![],
            split_into: None,
            context: Some(TaskContext {
                read_files: vec![ReadFile {
                    path: "src/lib.rs".into(),
                    lines: Some("1-3".into()),
                    why: "exact implementation".into(),
                }],
                symbols: vec!["Widget — existing struct".into()],
                ..TaskContext::default()
            }),
            verify: vec![VerifyStep {
                phase: "structural".into(),
                command: "grep -q Widget src/lib.rs".into(),
                fail_msg: None,
                timeout_ms: 1_000,
                scope: Vec::new(),
                covers: Vec::new(),
                expect: None,
            }],
            timeout_secs: 60,
            max_retries: 0,
            acceptance: vec![],
            acceptance_contract: None,
            accept: None,
            domain: None,
            estimated_minutes: None,
            crates_touched: None,
            sequence: 0,
            spec: Default::default(),
            hints: Default::default(),
        }
    }

    fn tasks(task: TaskDef) -> TasksFile {
        TasksFile {
            meta: TaskMeta {
                plan: "p1".into(),
                title: None,
                iteration: 0,
                total: 1,
                done: 0,
                status: "ready".into(),
                superseded_by: None,
                max_parallel: Some(1),
                estimated_total_minutes: 1,
                skip_enrichment: false,
                failure_policy: None,
                workspace_rungs: None,
                verify: Vec::new(),
                cold_verify: false,
                approval: None,
                allow_unverified: false,
                outbound: None,
                dropped_checks: Vec::new(),
                sources: Vec::new(),
            },
            tasks: vec![task],
        }
    }

    #[test]
    fn context_contract_rejects_escape_and_missing_exact_anchor() {
        let root = tempdir().expect("root");
        std::fs::create_dir(root.path().join("src")).expect("src");
        std::fs::write(root.path().join("src/lib.rs"), "pub struct Other;\n").expect("source");
        let mut plan_task = task();
        plan_task
            .context
            .as_mut()
            .unwrap()
            .read_files
            .push(ReadFile {
                path: "../secret".into(),
                lines: None,
                why: "unsafe".into(),
            });
        let plan_dir = root.path().join("plans/p1");
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(plan_dir.join("tasks.toml"), "[meta]").expect("manifest");
        let issues = validate_plan_context(
            &tasks(plan_task),
            root.path(),
            &plan_dir,
            PlanExecutionPolicy::normal(),
        );
        assert!(issues.iter().any(|issue| issue.code == "PLAN_CONTEXT_PATH"));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "PLAN_CONTEXT_SYMBOL")
        );
    }

    /// A plan written before the PRD pipeline went (2026-10-02) may still set
    /// `[meta] source_prd`. It loads, and validates exactly like the same plan
    /// without it: nothing reads the key any more.
    #[test]
    fn a_plan_that_names_a_source_prd_still_loads() {
        let root = tempdir().expect("root");
        std::fs::create_dir(root.path().join("src")).expect("src");
        std::fs::write(root.path().join("src/lib.rs"), "pub struct Widget;\n").expect("source");
        let plan_dir = root.path().join("plans/p1");
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(plan_dir.join("tasks.toml"), "[meta]").expect("manifest");
        let toml = |meta: &str| {
            format!(
                "[meta]\nplan = \"p1\"\n{meta}\n[[task]]\nid = \"T1\"\ntitle = \"A\"\n\
                 status = \"ready\"\n"
            )
        };
        let with = TasksFile::parse_str(&toml("source_prd = \"missing-prd\"")).expect("old plan");
        let without = TasksFile::parse_str(&toml("")).expect("plan");
        let validate = |plan: &TasksFile| {
            validate_plan_context(plan, root.path(), &plan_dir, PlanExecutionPolicy::normal())
        };
        assert_eq!(validate(&with), validate(&without));
    }

    #[test]
    fn renderer_inlines_numbered_range_and_anchor_context() {
        let root = tempdir().expect("root");
        std::fs::create_dir(root.path().join("src")).expect("src");
        std::fs::write(
            root.path().join("src/lib.rs"),
            "header\npub struct Widget;\nfooter\n",
        )
        .expect("source");
        let rendered = render_declared_context(&task(), root.path(), PlanExecutionPolicy::fast())
            .expect("render context");
        assert!(rendered.contains("path=\"src/lib.rs\""));
        assert!(rendered.contains("2 | pub struct Widget;"));
        assert!(rendered.contains("Do not search home directories"));
    }

    /// [`task`] with `paths` declared as read files without ranges, and no
    /// symbol anchors.
    fn unranged_task(paths: &[&str]) -> TaskDef {
        let mut plan_task = task();
        let context = plan_task.context.as_mut().expect("context");
        context.read_files = paths
            .iter()
            .map(|path| ReadFile {
                path: (*path).to_string(),
                lines: None,
                why: "whole file".into(),
            })
            .collect();
        context.symbols.clear();
        plan_task
    }

    /// Writes `count` lines, each `width` characters long, to `path`.
    fn write_lines(root: &Path, path: &str, count: usize, width: usize) {
        let content = (1..=count)
            .map(|n| format!("{:<width$}\n", format!("// line {n}")))
            .collect::<String>();
        std::fs::write(root.join(path), content).expect("write source");
    }

    /// bug-7c8a57: a file declared without a range is injected whole, not
    /// just its first 80 lines, and its tag says how long it is.
    #[test]
    fn renderer_injects_unranged_file_past_line_80() {
        let root = tempdir().expect("root");
        std::fs::create_dir(root.path().join("src")).expect("src");
        write_lines(root.path(), "src/lib.rs", 300, 12);
        let rendered = render_declared_context(
            &unranged_task(&["src/lib.rs"]),
            root.path(),
            PlanExecutionPolicy::normal(),
        )
        .expect("render context");
        assert!(
            rendered.contains("lines=\"1-300\" total=\"300\""),
            "{rendered}"
        );
        assert!(rendered.contains("    81 | // line 81"), "{rendered}");
        assert!(rendered.contains("   300 | // line 300"), "{rendered}");
        assert!(!rendered.contains("truncated"), "{rendered}");
    }

    /// A file declared without a range that is longer than the policy's
    /// range limit is cut there, with a marker saying what was shown, and
    /// the task is not failed for it.
    #[test]
    fn renderer_marks_an_unranged_file_cut_at_the_line_limit() {
        let root = tempdir().expect("root");
        std::fs::create_dir(root.path().join("src")).expect("src");
        write_lines(root.path(), "src/lib.rs", 300, 12);
        let policy = PlanExecutionPolicy::fast();
        let limit = policy.max_range_lines;
        let rendered =
            render_declared_context(&unranged_task(&["src/lib.rs"]), root.path(), policy)
                .expect("a file without a range never fails the task");
        assert!(
            rendered.contains(&format!("lines=\"1-{limit}\" total=\"300\"")),
            "{rendered}"
        );
        assert!(
            rendered.contains(&format!(
                "[... truncated: showed lines 1-{limit} of 300; add a lines range to \
                 read_files]\n</declared-file>"
            )),
            "{rendered}"
        );
        assert!(
            !rendered.contains(&format!("{:>6} | ", limit + 1)),
            "{rendered}"
        );
    }

    /// Files declared without a range share what the prompt budget leaves
    /// and are cut to fit it, each with a marker; an explicit range that
    /// overruns the budget still fails the task.
    #[test]
    fn unranged_files_share_the_budget_and_explicit_ranges_still_fail() {
        let root = tempdir().expect("root");
        std::fs::create_dir(root.path().join("src")).expect("src");
        // 200 lines of 200 characters: about 40 KiB each, past FAST's budget.
        write_lines(root.path(), "src/a.rs", 200, 200);
        write_lines(root.path(), "src/b.rs", 200, 200);
        let policy = PlanExecutionPolicy::fast();
        let rendered = render_declared_context(
            &unranged_task(&["src/a.rs", "src/b.rs"]),
            root.path(),
            policy,
        )
        .expect("files without a range never fail the task");
        assert!(
            rendered.len() <= policy.max_declared_context_bytes,
            "{} bytes",
            rendered.len()
        );
        assert_eq!(
            rendered.matches("[... truncated: showed lines 1-").count(),
            2,
            "{rendered}"
        );
        assert!(
            rendered.contains("path=\"src/b.rs\" lines=\"1-"),
            "{rendered}"
        );

        let mut ranged = unranged_task(&["src/a.rs"]);
        ranged.context.as_mut().expect("context").read_files[0].lines = Some("1-200".into());
        let error = render_declared_context(&ranged, root.path(), policy)
            .expect_err("an explicit range over the budget fails the task");
        assert!(error.contains("byte prompt budget"), "{error}");
    }

    /// bug-7ff055: a range the planner guessed in a file an earlier task
    /// changes passes the plan check, and at dispatch fits the file as that
    /// task left it: cut at its last line, or the whole file when it starts
    /// past the end. The plan check still refuses the same range in a file
    /// no earlier task changes, but since bug-fa22cc a try fits it to its
    /// checkout too, and prompt assembly never refuses a range past the end.
    #[test]
    fn ranges_in_files_an_earlier_task_changes_fit_the_file() {
        let root = tempdir().expect("root");
        std::fs::create_dir(root.path().join("src")).expect("src");
        let plan_dir = root.path().join("plan");
        std::fs::create_dir(&plan_dir).expect("plan dir");
        std::fs::write(plan_dir.join("tasks.toml"), "").expect("tasks.toml");
        // Before the run, the file T1 rewrites has 10 lines.
        write_lines(root.path(), "src/out.py", 10, 12);
        write_lines(root.path(), "src/other.py", 17, 12);

        let mut writer = unranged_task(&[]);
        writer.files = vec!["src/out.py".into()];
        let guessed = |path: &str, lines: &str| ReadFile {
            path: path.into(),
            lines: Some(lines.into()),
            why: "guessed".into(),
        };
        let mut reader = unranged_task(&[]);
        reader.id = "T2".into();
        reader.depends_on = vec!["T1".into()];
        reader.files = vec!["src/test_out.py".into()];
        reader.context.as_mut().expect("context").read_files = vec![
            guessed("src/out.py", "5-25"),
            guessed("src/out.py", "20-30"),
        ];
        let mut plan = tasks(writer);
        plan.tasks.push(reader);
        let range_issues = |plan: &TasksFile| {
            validate_plan_context(plan, root.path(), &plan_dir, PlanExecutionPolicy::normal())
                .into_iter()
                .filter(|issue| issue.code == "PLAN_CONTEXT_RANGE")
                .map(|issue| issue.message)
                .collect::<Vec<_>>()
        };
        assert!(range_issues(&plan).is_empty(), "{:?}", range_issues(&plan));

        // T1 ran and left the file 17 lines long.
        write_lines(root.path(), "src/out.py", 17, 12);
        let mut dispatched = plan.tasks[1].clone();
        let fitted = fit_ranges_to_checkout(&mut dispatched, root.path());
        assert_eq!(fitted.len(), 2, "{fitted:?}");
        let ranges = dispatched
            .context
            .as_ref()
            .expect("context")
            .read_files
            .iter()
            .map(|read_file| read_file.lines.clone())
            .collect::<Vec<_>>();
        assert_eq!(ranges, [Some("5-17".to_string()), None]);
        let rendered =
            render_declared_context(&dispatched, root.path(), PlanExecutionPolicy::normal())
                .expect("the fitted ranges render");
        assert!(
            rendered.contains("lines=\"5-17\" total=\"17\""),
            "{rendered}"
        );
        assert!(
            rendered.contains("lines=\"1-17\" total=\"17\""),
            "{rendered}"
        );

        // The plan check refuses the same range in a file no earlier task
        // changes; a try fits it, and so does prompt assembly when no try
        // did.
        let mut strict = plan.clone();
        strict.tasks[1]
            .context
            .as_mut()
            .expect("context")
            .read_files = vec![guessed("src/other.py", "5-25")];
        assert_eq!(range_issues(&strict).len(), 1);
        let unchanged = strict.tasks[1].clone();
        let rendered =
            render_declared_context(&unchanged, root.path(), PlanExecutionPolicy::normal())
                .expect("a range past the end renders fitted");
        assert!(
            rendered.contains("lines=\"5-17\" total=\"17\""),
            "{rendered}"
        );
        let mut fitted_task = unchanged.clone();
        assert_eq!(
            fit_ranges_to_checkout(&mut fitted_task, root.path()),
            ["`5-25` of `src/other.py` (17 lines) became `5-17`"]
        );
    }

    /// bug-41791b: at dispatch, an anchor none of the task's declared files
    /// holds becomes a note that keeps its text, a declared file the try's
    /// checkout no longer holds drops out of its reads, and prompt assembly
    /// never fails on either; an anchor a file holds, and an unsafe path,
    /// stay as they were.
    #[test]
    fn a_missing_anchor_or_file_is_left_out_of_a_try() {
        let root = tempdir().expect("root");
        std::fs::create_dir(root.path().join("src")).expect("src");
        std::fs::write(root.path().join("src/lib.rs"), "pub struct Widget;\n").expect("source");
        let mut plan_task = unranged_task(&["src/lib.rs", "src/gone.rs"]);
        let context = plan_task.context.as_mut().expect("context");
        context.symbols = vec![
            "Widget — the widget".to_string(),
            "`Gadget`: renamed by the last try".to_string(),
            "a plain note".to_string(),
        ];
        let unrepaired = plan_task.clone();

        assert_eq!(
            drop_missing_reads(&mut plan_task, root.path()),
            ["`src/gone.rs` is not in the try's checkout, so the try does not read it"]
        );
        assert_eq!(
            drop_missing_anchors(&mut plan_task, root.path()),
            ["`Gadget` is in no file the try reads, so its symbol entry is a note, not an anchor"]
        );
        let context = plan_task.context.as_ref().expect("context");
        assert_eq!(
            context.symbols,
            [
                "Widget — the widget",
                "Gadget (in no file the task reads): renamed by the last try",
                "a plain note"
            ]
        );
        let reads: Vec<&str> = context
            .read_files
            .iter()
            .map(|read_file| read_file.path.as_str())
            .collect();
        assert_eq!(reads, ["src/lib.rs"]);
        let rendered =
            render_declared_context(&plan_task, root.path(), PlanExecutionPolicy::normal())
                .expect("the repaired context renders");
        assert!(rendered.contains("1 | pub struct Widget;"), "{rendered}");
        // Unrepaired, a missing anchor no longer fails the render either.
        let mut anchored = unrepaired;
        anchored.context.as_mut().expect("context").read_files.pop();
        render_declared_context(&anchored, root.path(), PlanExecutionPolicy::normal())
            .expect("a missing anchor renders nothing");
        // An unsafe path stays, for the render to refuse.
        let mut unsafe_read = unranged_task(&["../secret"]);
        assert!(drop_missing_reads(&mut unsafe_read, root.path()).is_empty());
        assert!(
            render_declared_context(&unsafe_read, root.path(), PlanExecutionPolicy::normal())
                .is_err()
        );
    }

    /// bug-fa22cc: a declared range is fitted to its file as it is now. One
    /// inside the file stays; one that runs past the end is cut at the last
    /// line; one that starts past the end, or runs from line 1 past it, as
    /// phase 5.1's `1-121` of a script T2#1 had cut to 43 lines did, reads the
    /// whole file. Only a malformed range is refused.
    #[test]
    fn a_range_past_the_end_fits_the_file() {
        for (range, fitted) in [
            ("1-43", Ok(FittedRange::Within(1, 43))),
            ("5-20", Ok(FittedRange::Within(5, 20))),
            ("5-", Ok(FittedRange::Within(5, 43))),
            ("1-121", Ok(FittedRange::Whole)),
            ("44-60", Ok(FittedRange::Whole)),
            ("50-", Ok(FittedRange::Whole)),
            ("5-121", Ok(FittedRange::Cut(5, 43))),
            ("9-3", Err("the range ends before it starts".to_string())),
            ("0-3", Err("line numbers are one-based".to_string())),
            ("lines", Err("expected START-END or START-".to_string())),
        ] {
            assert_eq!(fit_line_range(range, 43), fitted, "{range}");
        }
        let root = tempdir().expect("root");
        std::fs::create_dir(root.path().join("tools")).expect("tools");
        write_lines(root.path(), "tools/render-demos.sh", 43, 12);
        let mut task = unranged_task(&[]);
        task.context.as_mut().expect("context").read_files = vec![ReadFile {
            path: "tools/render-demos.sh".into(),
            lines: Some("1-121".into()),
            why: "the script the task rewrites".into(),
        }];
        let rendered = render_declared_context(&task, root.path(), PlanExecutionPolicy::normal())
            .expect("the whole file renders");
        assert!(
            rendered.contains("lines=\"1-43\" total=\"43\""),
            "{rendered}"
        );
        let files = render_declared_files(
            &task,
            root.path(),
            PlanExecutionPolicy::normal(),
            100_000,
            &[],
        )
        .expect("the prompt's declared files");
        assert!(files.contains("    43 | // line 43"), "{files}");
        assert!(!files.contains("not shown: read_file them"), "{files}");
        assert_eq!(
            fit_ranges_to_checkout(&mut task, root.path()),
            ["`1-121` of `tools/render-demos.sh` (43 lines) became the whole file"]
        );
        // A malformed range still fails the try, naming the file.
        task.context.as_mut().expect("context").read_files[0].lines = Some("9-3".into());
        let error = render_declared_context(&task, root.path(), PlanExecutionPolicy::normal())
            .expect_err("a reversed range");
        assert_eq!(
            error,
            "invalid line range `9-3` of `tools/render-demos.sh`: the range ends before it starts"
        );
    }

    /// bug-c5f406: phase 4.3's T2 declared `plugin/ui/Displays.cpp` as
    /// `1-254`, the planner's `1-260` cut to the file's length when the plan
    /// was written. Its third try ran on a kept tree where the file had 290
    /// lines, and its prompt showed lines 1-247 and named 248-254 only. A
    /// range from line 1 past the file's end when the plan is written now
    /// becomes the whole file (bug-fa22cc's fit), shown as it is when the
    /// prompt is built; a range from line 1 that the file has grown past
    /// names the lines past it, with any cut's; a part the planner chose
    /// stays as it is.
    #[test]
    fn a_grown_file_shows_or_names_the_lines_past_its_planned_range() {
        let root = tempdir().expect("root");
        std::fs::create_dir(root.path().join("src")).expect("src");
        for path in ["src/a.rs", "src/b.rs", "src/c.rs"] {
            write_lines(root.path(), path, 10, 12);
        }
        let planned = |path: &str, lines: &str| ReadFile {
            path: path.into(),
            lines: Some(lines.into()),
            why: "planned".into(),
        };
        let mut reader = unranged_task(&[]);
        reader.context.as_mut().expect("context").read_files = vec![
            planned("src/a.rs", "1-10"),
            planned("src/b.rs", "1-12"),
            planned("src/c.rs", "3-6"),
        ];
        let mut plan = tasks(reader);
        assert_eq!(
            fit_ranges_to_files(&mut plan, root.path()),
            ["T1: `1-12` of `src/b.rs` (10 lines) became the whole file"]
        );

        // A try's tree, where each file has grown to 15 lines: the range the
        // plan kept names the lines past it, and the whole file shows them.
        for path in ["src/a.rs", "src/b.rs", "src/c.rs"] {
            write_lines(root.path(), path, 15, 12);
        }
        let policy = PlanExecutionPolicy::normal();
        let room = policy.max_declared_context_bytes;
        let shown = render_declared_files(&plan.tasks[0], root.path(), policy, room, &[])
            .expect("the fitted plan renders");
        assert!(
            shown.contains("path=\"src/a.rs\" lines=\"1-10\" total=\"15\""),
            "{shown}"
        );
        assert!(
            shown.contains("[src/a.rs lines 11-15 not shown: read_file them]\n"),
            "{shown}"
        );
        assert!(
            shown.contains("path=\"src/b.rs\" lines=\"1-15\" total=\"15\""),
            "{shown}"
        );
        assert!(
            shown.contains("path=\"src/c.rs\" lines=\"3-6\" total=\"15\""),
            "{shown}"
        );
        assert!(
            !shown.contains("[src/b.rs") && !shown.contains("[src/c.rs"),
            "{shown}"
        );

        // Lines an anchor shows past the range are not named.
        let text = (1..=30)
            .map(|n| match n {
                25 => "struct Grown;\n".to_string(),
                _ => format!("// line {n}\n"),
            })
            .collect::<String>();
        std::fs::write(root.path().join("src/a.rs"), text).expect("write source");
        let mut anchored = plan.tasks[0].clone();
        let context = anchored.context.as_mut().expect("context");
        context.read_files = vec![planned("src/a.rs", "1-10")];
        context.symbols = vec!["exact:Grown".into()];
        let shown = render_declared_files(&anchored, root.path(), policy, room, &[])
            .expect("the anchored plan renders");
        assert!(shown.contains("lines=\"17-30\" total=\"30\""), "{shown}");
        assert!(
            shown.contains("[src/a.rs lines 11-16 not shown: read_file them]\n"),
            "{shown}"
        );

        // T2#3's cut: the note names the lines past the range with its own.
        write_lines(root.path(), "src/a.rs", 290, 12);
        let mut written = plan.tasks[0].clone();
        written.context.as_mut().expect("context").read_files = vec![planned("src/a.rs", "1-254")];
        let short = render_declared_files(&written, root.path(), policy, 4_000, &[])
            .expect("the cut renders");
        let tag = "path=\"src/a.rs\" lines=\"1-";
        let at = short.find(tag).expect("the cut block") + tag.len();
        let last: usize = short[at..]
            .split('"')
            .next()
            .and_then(|end| end.parse().ok())
            .expect("its last line");
        assert!(last < 254, "{short}");
        assert!(
            short.contains(&format!(
                "[src/a.rs lines {}-290 not shown: read_file them]\n",
                last + 1
            )),
            "{short}"
        );
    }

    /// gap-1d1fa6: a task bigger than its tier allows is flagged, with the
    /// smallest tier it fits; the same task one tier up is not.
    #[test]
    fn task_over_its_tier_limits_is_flagged() {
        let policy = PlanExecutionPolicy::normal();
        let flagged = |task: &TaskDef| {
            validate_tier_sizes(&tasks(task.clone()), policy)
                .into_iter()
                .map(|issue| {
                    assert_eq!(issue.code, "PLAN_TIER_SIZE");
                    assert_eq!(issue.task_id.as_deref(), Some("T1"));
                    issue.message
                })
                .collect::<Vec<_>>()
        };
        assert!(flagged(&task()).is_empty(), "a small focused task fits");

        // The item's example task declares max_loc = 200, above even
        // integrative's 150-line budget, so this one declares 150.
        let mut wide = task();
        wide.tier = "mechanical".into();
        wide.files = (1..=6).map(|n| format!("src/part_{n}.rs")).collect();
        wide.max_loc = Some(150);
        assert_eq!(
            flagged(&wide),
            [
                "mechanical task is over its tier's size limits: 6 files (limit 3), \
                 max_loc = 150 (limit 20); split it into smaller tasks, or raise its tier \
                 to integrative"
            ]
        );
        wide.tier = "integrative".into();
        assert!(flagged(&wide).is_empty(), "the same task fits integrative");
        wide.max_loc = Some(200);
        assert_eq!(
            flagged(&wide),
            [
                "integrative task is over its tier's size limits: max_loc = 200 (limit 150); \
                 split it into smaller tasks, or raise its tier to architectural"
            ]
        );

        // Description words and verify steps count too, and a top-tier task
        // can only be split.
        let mut sprawling = task();
        sprawling.tier = "Architectural".into();
        sprawling.description = Some("word ".repeat(801));
        sprawling.verify = vec![sprawling.verify[0].clone(); 9];
        assert_eq!(
            flagged(&sprawling),
            [
                "architectural task is over its tier's size limits: 801-word description \
                 (limit 800), 9 verify steps (limit 8); split it into smaller tasks"
            ]
        );

        // The line budgets are the generator's, and a lane's own per-task
        // bounds cap every tier.
        for tier in TaskTier::ALL {
            assert_eq!(TierSizeLimits::for_tier(tier).max_loc, tier.max_loc());
        }
        let fast = PlanExecutionPolicy::fast();
        let architectural = fast.tier_size_limits(TaskTier::Architectural);
        assert_eq!(
            (architectural.max_files, architectural.max_verify_steps),
            (8, 1)
        );
    }

    #[test]
    fn generated_budget_rejects_duplicate_verify_and_serial_fragmentation() {
        let first = task();
        let mut second = task();
        second.id = "T2".into();
        second.depends_on = vec!["T1".into()];
        let mut third = task();
        third.id = "T3".into();
        third.depends_on = vec!["T2".into()];
        let mut plan = tasks(first);
        plan.meta.total = 3;
        plan.tasks.extend([second, third]);
        let issues = validate_plan_budgets(&plan, PlanExecutionPolicy::generated(8));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "PLAN_VERIFY_DUPLICATE")
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "PLAN_FRAGMENTED_OWNERSHIP")
        );
    }

    /// gap-dde99e: a generated task gives each acceptance criterion its own
    /// verify step, up to its tier's limit, and still needs one. The FAST
    /// lane keeps exactly one.
    #[test]
    fn generated_policy_keeps_a_step_per_acceptance_criterion() {
        let step = |test: &str, criterion: &str| VerifyStep {
            phase: "test".into(),
            command: format!("python3 -m unittest tests.test_widget.WidgetTest.{test}"),
            fail_msg: None,
            timeout_ms: 1_000,
            scope: Vec::new(),
            covers: vec![criterion.into()],
            expect: None,
        };
        let mut plan_task = task();
        plan_task.acceptance = vec![
            "AC1: a widget spins once per call".into(),
            "AC2: a stopped widget stays still".into(),
            "AC3: a reset widget starts at zero".into(),
        ];
        plan_task.verify = vec![
            step("test_spins", "AC1"),
            step("test_stops", "AC2"),
            step("test_resets", "AC3"),
        ];
        let plan = tasks(plan_task.clone());
        let generated = PlanExecutionPolicy::generated(8);
        let issues = validate_plan_budgets(&plan, generated);
        assert!(issues.is_empty(), "{issues:?}");
        let sizes = validate_tier_sizes(&plan, generated);
        assert!(sizes.is_empty(), "{sizes:?}");

        // A focused task has at most four: split it, or raise its tier.
        let mut over = plan_task.clone();
        over.verify = (1..=5).map(|n| step(&format!("test_{n}"), "AC1")).collect();
        let sizes = validate_tier_sizes(&tasks(over), generated);
        assert!(
            sizes
                .iter()
                .any(|issue| issue.message.contains("5 verify steps (limit 4)")),
            "{sizes:?}"
        );

        let mut unchecked = plan_task;
        unchecked.verify.clear();
        let issues = validate_plan_budgets(&tasks(unchecked), generated);
        assert!(
            issues.iter().any(|issue| issue.code == "PLAN_VERIFY_OWNER"),
            "{issues:?}"
        );

        let fast = validate_plan_budgets(&plan, PlanExecutionPolicy::fast());
        assert!(
            fast.iter().any(|issue| issue.code == "PLAN_VERIFY_OWNER"),
            "{fast:?}"
        );
    }

    /// gap-5db3d9: the test a check runs is the one its words name, and
    /// neither that test nor the header of a source the task changes counts
    /// toward the task's read budget.
    #[test]
    fn a_checks_test_and_a_sources_header_count_toward_no_read_budget() {
        for (path, stem) in [
            ("tests/spec/p5_2/presets_test.cpp", Some("presets")),
            ("tests/test_lowpass.py", Some("lowpass")),
            ("web/cart.test.ts", Some("cart")),
            ("spec/models/user_spec.rb", Some("user")),
            ("crates/seq/tests/sequencer_tests.rs", Some("sequencer")),
            ("core/src/presets.cpp", None),
            ("tests/_test.cpp", None),
        ] {
            assert_eq!(test_affix_stem(path), stem, "{path}");
        }
        for (command, path, runs) in [
            (
                "tools/spec.sh 5_2 presets",
                "tests/spec/p5_2/presets_test.cpp",
                true,
            ),
            (
                "tools/spec.sh 5 presets",
                "tests/spec/p5_2/presets_test.cpp",
                false,
            ),
            (
                "tools/spec.sh 5 presets",
                "tests/spec/p5/presets_test.cpp",
                true,
            ),
            (
                "tools/spec.sh 5_1 render",
                "tests/spec/p5/render_test.cpp",
                false,
            ),
            (
                "tools/spec.sh 4 display 'P4.06*'",
                "tests/spec/p4/display_test.cpp",
                true,
            ),
            (
                "tools/render-demos.sh && tools/spec.sh 5_1 demos",
                "tests/spec/p5_1/demos_test.cpp",
                true,
            ),
            // Each command of a step runs its own tests.
            (
                "tools/spec.sh 5_2 presets && tools/spec.sh 5 demos",
                "tests/spec/p5/presets_test.cpp",
                false,
            ),
            (
                "ctest --test-dir build -R presets_test",
                "tests/spec/p5/presets_test.cpp",
                true,
            ),
            (
                "cargo test --test sequencer_tests",
                "crates/seq/tests/sequencer_tests.rs",
                true,
            ),
            (
                "python3 -m pytest tests/test_lowpass.py",
                "tests/test_lowpass.py",
                true,
            ),
            ("tools/spec.sh 5_2 presets", "core/src/presets.cpp", false),
        ] {
            assert_eq!(test_run_by(command, path), runs, "{command} / {path}");
        }
        let source = ["core/src/presets.cpp".to_string()];
        for (header, beside) in [
            ("core/include/undertow/presets.h", true),
            ("core/src/presets.h", true),
            ("include/presets.h", false),
            ("core/include/undertow/params.h", false),
            ("core/src/presets.cpp", false),
        ] {
            assert_eq!(header_beside(header, &source), beside, "{header}");
        }
        assert!(header_beside(
            "include/widget.hpp",
            &["src/widget.cc".to_string()]
        ));

        let mut plan_task = task();
        plan_task.files = source.to_vec();
        plan_task.verify = vec![VerifyStep {
            phase: "test".into(),
            command: "tools/spec.sh 5_2 presets".into(),
            fail_msg: None,
            timeout_ms: 1_000,
            scope: Vec::new(),
            covers: Vec::new(),
            expect: None,
        }];
        let sources = ["docs/specs/phase-5.2-presets.md".to_string()];
        for (path, counts) in [
            ("docs/specs/phase-5.2-presets.md", false),
            ("tests/spec/p5_2/presets_test.cpp", false),
            ("core/include/undertow/presets.h", false),
            ("tests/spec/p5/presets_test.cpp", true),
            ("core/include/undertow/params.h", true),
            ("core/src/presets.cpp", true),
        ] {
            assert_eq!(
                counts_toward_read_budget(&sources, &plan_task, path),
                counts,
                "{path}"
            );
        }
    }

    /// Two tasks, T1 and T2, that both write `src/lib.rs`, in a plan that
    /// runs two tasks at a time.
    fn two_writers_of_one_file() -> TasksFile {
        let mut second = task();
        second.id = "T2".into();
        let mut plan = tasks(task());
        plan.meta.total = 2;
        plan.meta.max_parallel = Some(2);
        plan.tasks.push(second);
        plan
    }

    fn flags_concurrent_overlap(plan: &TasksFile) -> bool {
        validate_plan_budgets(plan, PlanExecutionPolicy::normal())
            .iter()
            .any(|issue| issue.code == "PLAN_CONCURRENT_OVERLAP")
    }

    /// gap-a8d786: tasks that neither depends on can run at the same time.
    /// When their files overlap, the finding names both tasks and the path,
    /// and it leaves the plan runnable.
    #[test]
    fn concurrent_tasks_sharing_a_file_are_flagged() {
        let mut plan = two_writers_of_one_file();
        let issues = validate_plan_budgets(&plan, PlanExecutionPolicy::normal());
        let overlaps = issues
            .iter()
            .filter(|issue| issue.code == "PLAN_CONCURRENT_OVERLAP")
            .collect::<Vec<_>>();
        assert_eq!(overlaps.len(), 1, "{issues:?}");
        let message = &overlaps[0].message;
        assert!(message.contains("T1 and T2"), "{message}");
        assert!(message.contains("`src/lib.rs`"), "{message}");
        assert!(overlaps[0].is_advisory());

        // A directory covers the files below it, as in the Graph engine.
        plan.tasks[1].files = vec!["src".into()];
        assert!(flags_concurrent_overlap(&plan));
        plan.tasks[1].files = vec!["src/lib".into()];
        assert!(!flags_concurrent_overlap(&plan));
    }

    /// The same two tasks are not flagged once one depends on the other, or
    /// when the plan runs one task at a time.
    #[test]
    fn serial_tasks_sharing_a_file_are_not_flagged() {
        let mut plan = two_writers_of_one_file();
        assert!(flags_concurrent_overlap(&plan));

        plan.tasks[1].depends_on = vec!["T1".into()];
        assert!(!flags_concurrent_overlap(&plan), "T2 runs after T1");

        plan.tasks[1].depends_on.clear();
        plan.meta.max_parallel = Some(1);
        assert!(!flags_concurrent_overlap(&plan), "one task at a time");
    }

    /// gap-272448: an omitted `max_parallel` lets as many tasks run at once
    /// as the plan has, when every task that can write declares its files,
    /// and one otherwise. An authored value wins.
    #[test]
    fn omitted_max_parallel_resolves_from_declared_files() {
        let mut plan = two_writers_of_one_file();
        plan.meta.max_parallel = None;
        assert_eq!(plan_max_parallel(&plan), 2);
        // Now the two tasks can run together, so their shared file counts.
        assert!(flags_concurrent_overlap(&plan));

        // A researcher cannot write, so it declares no files and changes
        // nothing.
        let mut researcher = task();
        researcher.id = "T3".into();
        researcher.role = Some("researcher".into());
        researcher.files.clear();
        plan.tasks.push(researcher);
        assert_eq!(task_with_unknown_writes(&plan).map(|task| &task.id), None);
        assert_eq!(plan_max_parallel(&plan), 3);

        // An implementer that declares no files writes something unknown.
        plan.tasks[1].files.clear();
        assert_eq!(
            task_with_unknown_writes(&plan).map(|task| task.id.as_str()),
            Some("T2")
        );
        assert_eq!(plan_max_parallel(&plan), 1);

        plan.meta.max_parallel = Some(2);
        assert_eq!(plan_max_parallel(&plan), 2);
    }
}
