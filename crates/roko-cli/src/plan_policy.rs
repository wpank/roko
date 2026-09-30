//! Fail-fast execution policy for generated plans and declared task context.
//!
//! The runner must not spend an agent turn rediscovering information the plan
//! claimed to provide. This module validates those claims before execution and
//! renders a small, exact context packet immediately before dispatch.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::path::{Component, Path, PathBuf};

use roko_core::task::TaskTier;

use crate::task_parser::{TaskDef, TasksFile};

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

/// Binding default-template task ceiling used by direct plan generation and
/// regeneration paths that do not originate from PRD template frontmatter.
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
    pub require_one_verify: bool,
    pub reject_duplicate_verify: bool,
    pub bounded_context_only: bool,
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
            require_one_verify: false,
            reject_duplicate_verify: false,
            bounded_context_only: false,
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
            require_one_verify: true,
            reject_duplicate_verify: true,
            bounded_context_only: true,
        }
    }

    /// Limits for a newly generated artifact. The template-specific task
    /// ceiling is binding rather than prompt-only guidance.
    #[must_use]
    pub const fn generated(max_tasks: usize) -> Self {
        Self {
            max_tasks,
            max_files_per_task: FAST_MAX_FILES_PER_TASK,
            max_read_files_per_task: FAST_MAX_READ_FILES,
            max_verify_steps_per_task: FAST_MAX_VERIFY_STEPS,
            max_symbols_per_task: FAST_MAX_CONTEXT_ITEMS,
            max_anti_patterns_per_task: FAST_MAX_CONTEXT_ITEMS,
            max_prior_failures_per_task: FAST_MAX_PRIOR_FAILURES,
            max_range_lines: FAST_MAX_RANGE_LINES,
            max_declared_context_bytes: FAST_MAX_DECLARED_CONTEXT_BYTES,
            max_serial_owners_per_file: Some(FAST_MAX_SERIAL_OWNERS_PER_FILE),
            require_one_verify: true,
            reject_duplicate_verify: true,
            bounded_context_only: true,
        }
    }

    /// Generated-plan budget aligned with the lane that will execute it.
    /// Without this clamp a FAST generation could spend a full strategist
    /// turn producing eight tasks that the four-task execution policy then
    /// rejects before dispatch.
    #[must_use]
    pub fn generated_for_environment(max_tasks: usize) -> Self {
        Self::generated(effective_generated_task_limit(max_tasks))
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
    fn exceeded_by(self, task: &TaskDef) -> Vec<String> {
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
        if task.verify.len() > self.max_verify_steps {
            over.push(format!(
                "{} verify steps (limit {})",
                task.verify.len(),
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
        check_count(
            &mut issues,
            task,
            "PLAN_BUDGET_READ_FILES",
            "context.read_files",
            task.context
                .as_ref()
                .map_or(0, |context| context.read_files.len()),
            policy.max_read_files_per_task,
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
/// path, or a directory and a path inside it. The engine runs such tasks one
/// after the other, so the plan gives up parallelism its author meant to have.
/// A task without `files` declares nothing and is never flagged.
fn concurrent_overlaps(tasks: &TasksFile) -> Vec<PlanPolicyViolation> {
    let dependency_map = tasks
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task.depends_on.as_slice()))
        .collect::<HashMap<_, _>>();
    let mut issues = Vec::new();
    for (index, first) in tasks.tasks.iter().enumerate() {
        for second in &tasks.tasks[index + 1..] {
            let overlap = roko_graph::exclusion::first_overlap(&first.files, &second.files);
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

/// Validate paths, regular-file identity, explicit symbol anchors, and an
/// explicitly named source PRD before the plan enters the runner.
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
                match parse_line_range(range, usize::MAX) {
                    Ok((start, end)) => {
                        let line_count = std::fs::read_to_string(&full_path)
                            .map(|content| content.lines().count())
                            .unwrap_or(0);
                        let resolved_end = end.unwrap_or(line_count);
                        if start == 0 || start > resolved_end || resolved_end > line_count {
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

    if let Some(source_prd) = tasks
        .meta
        .source_prd
        .as_deref()
        .map(str::trim)
        .filter(|source| !source.is_empty())
    {
        if validate_artifact_slug(source_prd).is_err() {
            issues.push(PlanPolicyViolation::plan(
                "PLAN_SOURCE_PRD_PATH",
                format!("source_prd `{source_prd}` is not a safe artifact slug"),
            ));
        } else {
            let prd_root = workspace_root.join(".roko").join("prd");
            let candidates = [
                prd_root.join("published").join(format!("{source_prd}.md")),
                prd_root.join("drafts").join(format!("{source_prd}.md")),
                prd_root.join("draft").join(format!("{source_prd}.md")),
            ];
            if !candidates.iter().any(|path| path.is_file()) {
                issues.push(PlanPolicyViolation::plan(
                    "PLAN_SOURCE_PRD_MISSING",
                    format!(
                        "source_prd `{source_prd}` is explicit but no published/drafts artifact exists"
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

/// Render exact, line-numbered context for prompt assembly. Unlike preflight,
/// dispatch has no future-output exception: every dependency should already be
/// materialized in the attempt worktree.
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
    let anchors = context
        .symbols
        .iter()
        .filter_map(|symbol| explicit_symbol_anchor(symbol).map(|anchor| (symbol, anchor)))
        .collect::<Vec<_>>();
    let mut anchor_found = vec![false; anchors.len()];
    let mut output = String::from(
        "# Declared source context (authoritative and bounded)\n\
         Use these supplied snippets directly. Do not search home directories, session history, \
         other worktrees, Git unreachable objects, or the web. If required context is absent, \
         stop and request plan repair instead of exploring.\n",
    );

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
        if let Some(range) = read_file.lines.as_deref() {
            let (start, end) = parse_line_range(range, lines.len())?;
            let end = end.unwrap_or(lines.len());
            if start == 0 || start > end || end > lines.len() {
                return Err(format!(
                    "line range `{range}` is outside `{}` ({} lines)",
                    read_file.path,
                    lines.len()
                ));
            }
            if end.saturating_sub(start).saturating_add(1) > policy.max_range_lines {
                return Err(format!(
                    "line range `{range}` exceeds the {} line context limit",
                    policy.max_range_lines
                ));
            }
            ranges.push((start, end));
        }
        for (index, (_, anchor)) in anchors.iter().enumerate() {
            if let Some(line) = find_anchor_line(&content, anchor) {
                anchor_found[index] = true;
                let start = line.saturating_sub(ANCHOR_CONTEXT_RADIUS).max(1);
                let end = line.saturating_add(ANCHOR_CONTEXT_RADIUS).min(lines.len());
                ranges.push((start, end));
            }
        }
        if ranges.is_empty() {
            ranges.push((1, lines.len().min(80)));
        }
        let ranges = merge_ranges(ranges);
        for (start, end) in ranges {
            output.push_str(&format!(
                "\n<declared-file path=\"{}\" lines=\"{}-{}\" why=\"{}\">\n",
                read_file.path,
                start,
                end,
                read_file.why.replace('"', "'")
            ));
            for line_number in start..=end {
                if let Some(line) = lines.get(line_number.saturating_sub(1)) {
                    output.push_str(&format!("{line_number:>6} | {line}\n"));
                }
            }
            output.push_str("</declared-file>\n");
        }
        if output.len() > policy.max_declared_context_bytes {
            return Err(format!(
                "declared snippets for task {} exceed the {} byte prompt budget; narrow read_files ranges",
                task.id, policy.max_declared_context_bytes
            ));
        }
    }

    let missing = anchors
        .iter()
        .zip(anchor_found)
        .filter_map(|((_, anchor), found)| (!found).then_some(anchor.as_str()))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "explicit symbol anchors were not found in declared context: {}",
            missing.join(", ")
        ));
    }
    Ok(output)
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

fn validate_artifact_slug(slug: &str) -> Result<(), ()> {
    if slug.is_empty()
        || slug.starts_with('.')
        || slug.starts_with('-')
        || slug.contains("..")
        || !slug
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        Err(())
    } else {
        Ok(())
    }
}

fn transitive_dependency_outputs(
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

fn parse_line_range(range: &str, line_count: usize) -> Result<(usize, Option<usize>), String> {
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

fn explicit_symbol_anchor(symbol: &str) -> Option<String> {
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

fn find_anchor_line(content: &str, anchor: &str) -> Option<usize> {
    let final_segment = anchor.rsplit("::").next().unwrap_or(anchor);
    content.lines().enumerate().find_map(|(index, line)| {
        (contains_identifier(line, anchor) || contains_identifier(line, final_segment))
            .then_some(index + 1)
    })
}

fn contains_identifier(line: &str, identifier: &str) -> bool {
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
        }
    }

    fn tasks(task: TaskDef) -> TasksFile {
        TasksFile {
            meta: TaskMeta {
                plan: "p1".into(),
                iteration: 0,
                total: 1,
                done: 0,
                status: "ready".into(),
                superseded_by: None,
                max_parallel: Some(1),
                estimated_total_minutes: 1,
                skip_enrichment: false,
                source_prd: None,
                failure_policy: None,
                workspace_rungs: None,
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
