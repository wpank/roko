//! Red flags: rejecting an attempt before its verify steps run.
//!
//! An attempt the provider reports as successful goes through this screen
//! before any verify step (C1 implication 5, `meyerson2025solving`). It is
//! rejected when:
//!
//! - its output ran away: it reported more output tokens than `[gates]
//!   max_output_tokens` allows its role;
//! - its role's output is the product (a reviewer's verdict, a planner's
//!   plan), and the output is empty, or a fenced TOML or JSON block in it is
//!   cut off or does not parse. The provider adapters already fail a
//!   non-zero exit, a turn cap hit and, for the Claude CLI, an empty
//!   response;
//! - its role's post-dispatch safety check, the one ACP runs
//!   ([`SafetyLayer::post_dispatch_check`]), blocks it: a secret in an exact
//!   format in the output ([`scrub::high_confidence_secret`]), a changed path
//!   outside the working tree, or a changed file under a role whose contract
//!   forbids file writes. A `NAME = value` match alone, which code such as
//!   `let token = next()` also produces, is logged, not blocking;
//! - the task's changes tamper with what checks it
//!   ([`roko_gate::attempt_diff`], S05 check A1): a weakened, deleted or
//!   skipped test, an edited verify script, `tasks.toml`, pinned acceptance
//!   test or gate config. Changes outside the task's `files` are logged,
//!   and rejected only under `[gates] diff_scope = "enforce"`;
//! - it is an implementer attempt at a task that names `files`, and the task
//!   has changed nothing, or added only stub lines
//!   ([`roko_gate::analyze_diff`]): "no changes". Other roles, refactors
//!   (role `refactorer`) and tasks without `files` are exempt. When a
//!   declared file that git ignores exists on disk, the rejection says so:
//!   no diff can show a change to it. An attempt that changed nothing, at a
//!   task with authored verify steps, is judged by them first: when they
//!   pass on the unchanged tree, the task's work was already there, as on a
//!   `--fresh` rerun of a finished task, and the attempt settles as already
//!   satisfied (gap-9eb1e1).
//!
//! A rejection costs no compile or test run, except the verify steps of an
//! unchanged tree. It settles like a failed verify step, `gate_failed` and
//! blamed on the agent, so no learner credits it; the verdict's
//! `failure_class.rung` names the check (`pre_verify:<check>`), and its
//! message is the next attempt's feedback.

use std::path::Component;

use roko_agent::safety::{SafetyLayer, SafetyViolation, ViolationSeverity, ViolationType, scrub};
use roko_core::config::gates::DiffScope;
use roko_gate::attempt_diff::{
    AttemptChange, AttemptDiffPolicy, ChangeKind, DiffFinding, PinnedTest, check_attempt_diff,
    scripts_run_by,
};
use roko_gate::{DiffPayload, analyze_diff};

use super::diff_snapshot::AttemptDiff;
use super::sibling_settle::declares;
use super::*;

/// Prefix of the gate name, and so of the verdict's `failure_class.rung`,
/// of an attempt the screen rejected: `pre_verify:<check>`.
pub(super) const PRE_VERIFY_GATE_PREFIX: &str = "pre_verify:";

/// Most `files` entries a "no changes" message names.
const NAMED_FILES: usize = 5;

/// Most attempt diff findings a message lists.
const LISTED_FINDINGS: usize = 10;

/// Why the screen rejected an attempt.
#[derive(Debug)]
struct Rejection {
    /// The check, e.g. `no_changes`.
    check: &'static str,
    /// What was wrong and what to do instead.
    message: String,
    /// The attempt left the working tree as it found it: the task's own
    /// verify steps decide first whether its work was already there
    /// (gap-9eb1e1).
    unchanged_tree: bool,
}

/// What the screen made of an attempt it did not reject.
#[derive(Debug)]
pub(super) enum Screened {
    /// Nothing stops the attempt.
    Clear,
    /// The attempt changed nothing, but the task has authored verify steps:
    /// they run on the tree as it is, and settle it
    /// ([`GraphTaskDispatcher::settle_unchanged_tree`]).
    UnchangedTree(UnchangedTree),
}

/// A "no changes" rejection that waits for the task's verify steps.
#[derive(Debug)]
pub(super) struct UnchangedTree {
    rejection: Rejection,
    /// Findings recorded without blocking, for the rejection's feedback.
    notes: Vec<String>,
}

impl GraphTaskDispatcher {
    /// Screen an attempt the provider reported as successful, before its
    /// verify steps run. A rejection is an `Err`, a verify failure of gate
    /// `pre_verify:<check>`, and its message is left as the next attempt's
    /// feedback. An attempt that changed nothing, at a task with authored
    /// verify steps, is left to them ([`Screened::UnchangedTree`]).
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn screen_attempt(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
        workdir: &Path,
        attempt_key: &str,
        attempt_number: u32,
        progress_tx: Option<&tokio::sync::mpsc::Sender<GraphTaskEvent>>,
    ) -> Result<Screened> {
        let role = task.role.as_deref().unwrap_or("implementer");
        // First, so the attempt's changed files are kept whatever the screen
        // decides (`take_changed_files`).
        let diff = self.attempt_diff(spec, task, attempt_key, workdir).await;
        let mut rejection = output_red_flag(&self.config, role, dispatch);
        // Findings recorded without blocking, carried into the feedback of a
        // rejection by a later check.
        let mut notes = Vec::new();
        if rejection.is_none() {
            rejection =
                self.post_dispatch_red_flag(spec, task, role, dispatch, diff.as_ref(), &mut notes);
        }
        if rejection.is_none()
            && let Some(diff) = &diff
        {
            rejection = match self
                .attempt_diff_red_flag(spec, task, workdir, attempt_number, diff)
                .await
            {
                Some(rejection) => Some(rejection),
                None => no_changes_red_flag(task, role, diff, attempt_number).await,
            };
        }
        let Some(rejection) = rejection else {
            return Ok(Screened::Clear);
        };
        if rejection.unchanged_tree {
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                attempt = attempt_number,
                "attempt changed nothing; its verify steps decide whether it was already done"
            );
            return Ok(Screened::UnchangedTree(UnchangedTree { rejection, notes }));
        }
        let lead = "Rejected before its verify steps ran";
        let error = self
            .reject_attempt(
                spec,
                task,
                attempt_number,
                progress_tx,
                lead,
                &rejection,
                &notes,
            )
            .await;
        Err(error)
    }

    /// Reject an attempt: its message, led by `lead` and followed by
    /// `notes`, is logged, shown on the TUI and the progress channel, and left
    /// as the next attempt's feedback. Returns the verify failure it settles
    /// as.
    #[allow(clippy::too_many_arguments)]
    async fn reject_attempt(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt_number: u32,
        progress_tx: Option<&tokio::sync::mpsc::Sender<GraphTaskEvent>>,
        lead: &str,
        rejection: &Rejection,
        notes: &[String],
    ) -> RokoError {
        let gate = format!("{PRE_VERIFY_GATE_PREFIX}{}", rejection.check);
        let mut message = format!("{lead} ({gate}). {}", rejection.message);
        if !notes.is_empty() {
            message.push_str("\n\nAlso recorded, not blocking:\n");
            message.push_str(&notes.join("\n"));
        }
        tracing::warn!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            attempt = attempt_number,
            %gate,
            reason = %message,
            "attempt rejected by the pre-verify screen"
        );
        if let Some(tui) = &self.tui_bridge {
            tui.gate_result_with_output(&spec.plan_id, &task.id, &gate, false, Some(&message));
        }
        if let Some(progress_tx) = progress_tx {
            let _ = progress_tx
                .send(GraphTaskEvent::Progress {
                    message: format!("rejected by the pre-verify screen: {gate}"),
                    completed: None,
                    total: None,
                })
                .await;
        }
        if let Some(feedback) = GateFeedback::from_raw(&message) {
            self.gate_retry_context.record(
                &spec.plan_id,
                &task.id,
                feedback,
                attempt_number.saturating_add(1),
            );
        }
        RokoError::Verify { gate, message }
    }

    /// Settle an attempt that changed nothing, once its verify steps have run
    /// on the tree it left (`verified`).
    ///
    /// A pass means the task's work was already there, as on a `--fresh`
    /// rerun of a finished task: the attempt settles as
    /// [`TaskGateVerdict::AlreadySatisfied`], neither a learning success nor
    /// a failure, and the task completes (gap-9eb1e1). Anything else rejects
    /// the attempt as `pre_verify:no_changes`, with how its steps ended.
    pub(super) async fn settle_unchanged_tree(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt_number: u32,
        progress_tx: Option<&tokio::sync::mpsc::Sender<GraphTaskEvent>>,
        unchanged: UnchangedTree,
        verified: Result<TaskGateVerdict>,
    ) -> Result<TaskGateVerdict> {
        let how_they_ended = match verified {
            Ok(TaskGateVerdict::Passed) => {
                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    attempt = attempt_number,
                    "attempt changed nothing, and its verify steps pass on the unchanged tree: \
                     the task was already satisfied"
                );
                if let Some(progress_tx) = progress_tx {
                    let message = "already satisfied: the verify steps pass on the unchanged tree";
                    let _ = progress_tx
                        .send(GraphTaskEvent::Progress {
                            message: message.to_string(),
                            completed: None,
                            total: None,
                        })
                        .await;
                }
                return Ok(TaskGateVerdict::AlreadySatisfied);
            }
            Ok(verdict) => format!("they settled as `{}`", verdict.as_str()),
            Err(RokoError::Verify { message, .. }) => message,
            Err(error) => error.to_string(),
        };
        let UnchangedTree {
            mut rejection,
            notes,
        } = unchanged;
        rejection.message.push_str(
            "\n\nIts verify steps, run on the unchanged tree to see whether the task was \
             already done, did not pass: ",
        );
        rejection.message.push_str(&how_they_ended);
        let lead = "Rejected after its verify steps ran on the unchanged tree";
        let error = self
            .reject_attempt(
                spec,
                task,
                attempt_number,
                progress_tx,
                lead,
                &rejection,
                &notes,
            )
            .await;
        Err(error)
    }

    /// The role's post-dispatch safety check, the one ACP runs
    /// ([`SafetyLayer::post_dispatch_check`]), over the attempt's output and
    /// the paths its task changed.
    ///
    /// A blocking violation rejects the attempt: a changed path outside the
    /// working tree, a changed file under a role whose contract forbids file
    /// writes, or a secret in the output. The check scrubs the output with
    /// the whole default policy, whose `NAME = value` rule also matches code a
    /// summary quotes (`let token = next()`, `api_key: Option<String>`), so a
    /// secret blocks only when [`scrub::high_confidence_secret`] finds one in
    /// an exact format. Other findings are logged and pushed to `notes`.
    /// Without a diff (a working tree git cannot snapshot), only the output is
    /// checked.
    fn post_dispatch_red_flag(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        role: &str,
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
        diff: Option<&AttemptDiff>,
        notes: &mut Vec<String>,
    ) -> Option<Rejection> {
        let changed_files: Vec<String> = diff
            .map(|diff| {
                diff.changes
                    .iter()
                    .map(|change| change.path.clone())
                    .collect()
            })
            .unwrap_or_default();
        let output = dispatch.result.output.body.as_text().unwrap_or_default();
        let exact_secret = scrub::high_confidence_secret(output);
        let (blocks, recorded): (Vec<SafetyViolation>, Vec<SafetyViolation>) =
            SafetyLayer::from_config(&self.config)
                .with_contract(effective_agent_contract(role, task, &self.config))
                .post_dispatch_check(&spec.plan_id, &task.id, role, output, &changed_files)
                .into_iter()
                .partition(|violation| {
                    violation.severity == ViolationSeverity::Block
                        && (violation.violation_type != ViolationType::SecretLeak || exact_secret)
                });
        for finding in &recorded {
            let why = if finding.violation_type == ViolationType::SecretLeak {
                "a NAME = value match only, which code also produces"
            } else {
                "a warning"
            };
            tracing::warn!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                violation = %finding.violation_type,
                message = %finding.message,
                why,
                "post-dispatch safety finding recorded, not blocking"
            );
            let note = format!("- {}: {} ({why})", finding.violation_type, finding.message);
            notes.push(note);
        }
        if blocks.is_empty() {
            return None;
        }
        let violations = blocks
            .iter()
            .map(|violation| format!("- {}: {}", violation.violation_type, violation.message))
            .collect::<Vec<_>>()
            .join("\n");
        Some(Rejection {
            check: "safety",
            unchanged_tree: false,
            message: format!(
                "Safety: the attempt broke its role's post-dispatch contract:\n{violations}\n\
                 Keep credentials out of the output, and change only what the `{role}` role \
                 may change."
            ),
        })
    }

    /// The attempt diff check (S05 check A1) over the task's changes: a
    /// tamper finding rejects the attempt; scope findings are logged, and
    /// reject it only under `[gates] diff_scope = "enforce"`.
    async fn attempt_diff_red_flag(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        workdir: &Path,
        attempt_number: u32,
        diff: &AttemptDiff,
    ) -> Option<Rejection> {
        let policy = self.attempt_diff_policy(spec, task, workdir);
        let mut changes = Vec::with_capacity(diff.changes.len());
        for changed in &diff.changes {
            let kind = match changed.status {
                'A' | 'C' => ChangeKind::Added,
                'D' => ChangeKind::Deleted,
                'R' => ChangeKind::Renamed,
                _ => ChangeKind::Modified,
            };
            let mut change = AttemptChange::new(kind, changed.path.clone());
            change.old_path.clone_from(&changed.old_path);
            if policy.needs_text(&change.path) || policy.needs_text(change.old_path()) {
                if let Some(blob) = &changed.old_blob {
                    change.before = diff.blob_text(blob).await;
                }
                if let Some(blob) = &changed.new_blob {
                    change.after = diff.blob_text(blob).await;
                }
            }
            changes.push(change);
        }
        let (tamper, scope): (Vec<DiffFinding>, Vec<DiffFinding>) =
            check_attempt_diff(&changes, &policy)
                .into_iter()
                .partition(|finding| finding.kind.is_tamper());
        if !scope.is_empty() {
            tracing::warn!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                attempt = attempt_number,
                findings = %finding_list(&scope),
                "attempt changed paths outside its task's files"
            );
        }
        if !tamper.is_empty() {
            return Some(Rejection {
                check: "tamper",
                unchanged_tree: false,
                message: format!(
                    "Tampering: the task's changes weaken or edit what checks it:\n{}\nRestore \
                     them. Tests, verify scripts, pinned acceptance tests and gate \
                     configuration are not the task's to weaken; add new tests instead.",
                    finding_list(&tamper)
                ),
            });
        }
        (!scope.is_empty() && self.config.gates.diff_scope == DiffScope::Enforce).then(|| {
            Rejection {
                check: "scope",
                unchanged_tree: false,
                message: format!(
                    "Out of scope: the task changed paths its files do not name \
                     ([gates] diff_scope = \"enforce\"):\n{}\nChange only {}, and undo the rest.",
                    finding_list(&scope),
                    named_files(&task.files)
                ),
            }
        })
    }

    /// What the task may change: its `files`, the scripts its verify steps
    /// run, and its pinned acceptance tests, with the plan's `accept/`
    /// directory, as paths in `workdir`.
    fn attempt_diff_policy(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        workdir: &Path,
    ) -> AttemptDiffPolicy {
        let plan_dir = Path::new(spec.plan_dir.trim());
        let plan_dir_on_disk = if plan_dir.is_absolute() {
            plan_dir.to_path_buf()
        } else {
            self.workdir.join(plan_dir)
        };
        // The plan directory as a path in the working tree, when inside it.
        let plan_dir_in_tree = if plan_dir.as_os_str().is_empty() {
            None
        } else if plan_dir.is_relative() {
            Some(plan_dir.to_path_buf())
        } else {
            [self.workdir.as_path(), workdir]
                .into_iter()
                .find_map(|root| relative_to(plan_dir, root))
        };
        let pinned_tests = task
            .accept
            .iter()
            .flat_map(|accept| &accept.files)
            .map(|entry| PinnedTest {
                src: plan_dir_in_tree
                    .as_ref()
                    .map(|dir| tree_path(&dir.join(&entry.src))),
                dest: tree_path(Path::new(&entry.dest)),
                text: std::fs::read_to_string(plan_dir_on_disk.join(&entry.src)).ok(),
            })
            .collect();
        // The task's own verify steps, then the workspace rungs that run
        // after them unless its plan opts out
        // (`verification::attempt_verify_steps`).
        let rungs = self.plan_rungs(spec);
        AttemptDiffPolicy {
            task_files: task.files.clone(),
            verify_scripts: task
                .verify
                .iter()
                .filter(|step| !crate::task_accept::is_pinned_step(step))
                .map(|step| step.command.as_str())
                .chain(rungs.map(|rung| rung.command.as_str()))
                .flat_map(scripts_run_by)
                .collect(),
            pinned_tests,
            accept_dirs: plan_dir_in_tree
                .iter()
                .map(|dir| tree_path(&dir.join("accept")))
                .collect(),
        }
    }
}

/// Findings, one per line, at most [`LISTED_FINDINGS`] of them.
fn finding_list(findings: &[DiffFinding]) -> String {
    let mut lines: Vec<String> = findings
        .iter()
        .take(LISTED_FINDINGS)
        .map(|finding| format!("- {finding}"))
        .collect();
    if findings.len() > LISTED_FINDINGS {
        lines.push(format!("- and {} more", findings.len() - LISTED_FINDINGS));
    }
    lines.join("\n")
}

/// `path` relative to `root` when it lies inside it, comparing canonical
/// paths as well.
fn relative_to(path: &Path, root: &Path) -> Option<PathBuf> {
    if let Ok(relative) = path.strip_prefix(root) {
        return Some(relative.to_path_buf());
    }
    let path = path.canonicalize().ok()?;
    let root = root.canonicalize().ok()?;
    path.strip_prefix(root).ok().map(Path::to_path_buf)
}

/// `path` as the working tree names it: `/`-separated, with `.` and `..`
/// resolved lexically.
fn tree_path(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::ParentDir => {
                parts.pop();
            }
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    parts.join("/")
}

/// A red flag on the attempt's output: past its role's output-token cap, or
/// a malformed product.
fn output_red_flag(
    config: &RokoConfig,
    role: &str,
    dispatch: &crate::dispatch_v2::AgentResultDispatch,
) -> Option<Rejection> {
    let output_tokens = u64::from(dispatch.result.usage.output_tokens);
    if let Some(cap) = config.gates.max_output_tokens_for(role)
        && output_tokens > cap
    {
        return Some(Rejection {
            check: "overlong_output",
            unchanged_tree: false,
            message: format!(
                "Red flag, overlong output: the attempt reported {output_tokens} output tokens, \
                 more than the {cap} allowed for role `{role}` ([gates] max_output_tokens). \
                 Runaway output is discarded; do the task in fewer, shorter steps."
            ),
        });
    }
    if !output_is_the_product(role) {
        return None;
    }
    let output = dispatch.result.output.body.as_text().unwrap_or_default();
    malformed_product(output).map(|problem| Rejection {
        check: "malformed_output",
        unchanged_tree: false,
        message: format!(
            "Red flag, malformed output: the {role}'s output is its product, and {problem}. \
             Answer with the complete, well-formed result."
        ),
    })
}

/// Roles whose output, rather than a diff, is the product: a verdict or a
/// plan.
fn output_is_the_product(role: &str) -> bool {
    matches!(
        role.trim().to_ascii_lowercase().replace('_', "-").as_str(),
        "reviewer"
            | "quick-reviewer"
            | "architect"
            | "auditor"
            | "critic"
            | "planner"
            | "strategist"
    )
}

/// Why `output`, a product such as a verdict or a plan, is malformed: it is
/// empty, or one of its fenced TOML or JSON blocks is never closed or does
/// not parse. Free text is not judged: the Graph path asks for no particular
/// format yet (gap-f4b935).
fn malformed_product(output: &str) -> Option<String> {
    if output.trim().is_empty() {
        return Some("it is empty".to_string());
    }
    let mut lines = output.lines();
    while let Some(line) = lines.next() {
        let Some(language) = line.trim().strip_prefix("```") else {
            continue;
        };
        let language = language.trim().to_ascii_lowercase();
        let mut block = String::new();
        let mut closed = false;
        for inner in lines.by_ref() {
            if inner.trim_start().starts_with("```") {
                closed = true;
                break;
            }
            block.push_str(inner);
            block.push('\n');
        }
        if language != "toml" && language != "json" {
            continue;
        }
        if !closed {
            return Some(format!("its ```{language} block is cut off"));
        }
        let error = if language == "toml" {
            toml::from_str::<toml::Value>(&block)
                .err()
                .map(|error| error.to_string())
        } else {
            serde_json::from_str::<serde_json::Value>(&block)
                .err()
                .map(|error| error.to_string())
        };
        if let Some(error) = error {
            let error: String = error
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or_default()
                .chars()
                .take(200)
                .collect();
            return Some(format!("its ```{language} block does not parse: {error}"));
        }
    }
    None
}

/// Whether an attempt must leave a change: an implementer's, at a task that
/// names `files`.
fn expects_changes(role: &str, task: &TaskDef) -> bool {
    role.trim().eq_ignore_ascii_case("implementer")
        && task.files.iter().any(|file| !file.trim().is_empty())
}

/// The "no changes" red flag: an implementer attempt at a task that names
/// `files` whose task has changed nothing, or added only stub lines.
async fn no_changes_red_flag(
    task: &TaskDef,
    role: &str,
    diff: &AttemptDiff,
    attempt_number: u32,
) -> Option<Rejection> {
    if !expects_changes(role, task) {
        return None;
    }
    // A change to a path both this task and a sibling declare may be this
    // task's: it cannot be called unchanged, nor judged a stub.
    let shared = diff.sibling_paths.iter().any(|path| {
        task.files
            .iter()
            .any(|file| declares(file, Path::new(path)))
    });
    if shared {
        return None;
    }
    let files = named_files(&task.files);
    if diff.changes.is_empty() {
        // A resumed run: its earlier attempts ran in another process, and
        // whatever they changed is part of this base.
        if diff.base_is_this_attempts && attempt_number > 0 {
            return None;
        }
        // A declared file git ignores may well have changed on disk: no diff
        // shows it, and the rejection must not claim the tree is untouched.
        let ignored = diff.ignored_on_disk(&task.files).await;
        if ignored.is_empty() && !task.verify.is_empty() {
            // The task's own verify steps may show its work was already
            // there, as on a `--fresh` rerun of a finished task: the screen
            // leaves the attempt to them (gap-9eb1e1).
            return Some(Rejection {
                check: "no_changes",
                message: format!(
                    "Red flag, no changes: the task names files to change ({files}), and its \
                     attempts have left the working tree as they found it. Make the change the \
                     task asks for."
                ),
                unchanged_tree: true,
            });
        }
        if !ignored.is_empty() {
            let (which, them) = if ignored.len() == 1 {
                ("is", "it")
            } else {
                ("are", "them")
            };
            return Some(Rejection {
                check: "no_changes",
                unchanged_tree: false,
                message: format!(
                    "Red flag, no changes: the task names {}, which {which} gitignored, so \
                     roko's diff and delivery cannot see changes to {them}. The task's work must \
                     land in files git tracks.",
                    named_files(&ignored)
                ),
            });
        }
        return Some(Rejection {
            check: "no_changes",
            unchanged_tree: false,
            message: format!(
                "Red flag, no changes: the task names files to change ({files}), and its \
                 attempts have left the working tree as they found it. Make the change the \
                 task asks for; its verify steps run only on a changed tree."
            ),
        });
    }
    let patch = diff.patch().await?;
    analyze_diff(&DiffPayload::new(patch))
        .all_added_are_forbidden
        .then(|| Rejection {
            check: "no_changes",
            unchanged_tree: false,
            message: format!(
                "Red flag, no changes: every line the task added to {files} is a stub, such as \
                 `todo!()`, `unimplemented!()` or a bare `Ok(())`. Replace the stubs with the \
                 implementation."
            ),
        })
}

/// The first few `files`, in backticks.
fn named_files(files: &[String]) -> String {
    let mut named: Vec<String> = files
        .iter()
        .filter(|file| !file.trim().is_empty())
        .take(NAMED_FILES)
        .map(|file| format!("`{}`", file.trim()))
        .collect();
    if files.len() > NAMED_FILES {
        named.push(format!("and {} more", files.len() - NAMED_FILES));
    }
    named.join(", ")
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::diff_snapshot::tests::commit_repo;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, jsonl_rows_where, make_spec, make_test_dispatcher, no_auto_fix,
        verify_step,
    };

    /// A fake Claude CLI that answers with `text` (JSON-escaped) and reports
    /// `output_tokens`, after running `edit` in its working directory.
    fn provider(edit: &str, text: &str, output_tokens: u32) -> String {
        format!(
            r#"#!/bin/sh
set -eu
cat >/dev/null
{edit}
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"{text}"}}}}'
printf '%s\n' '{{"type":"result","session_id":"s","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{{"input_tokens":5,"output_tokens":{output_tokens}}}}}'
"#
        )
    }

    /// Dispatch `task` once, expecting a pre-verify rejection; returns its
    /// gate and message, and checks that the verify step's `marker` was
    /// never created.
    async fn rejected(
        dispatcher: &GraphTaskDispatcher,
        task: &TaskDef,
        marker: &Path,
    ) -> (String, String) {
        let error = dispatcher
            .dispatch(&make_spec(task), Vec::new(), &CellContext::new())
            .await
            .expect_err("the attempt is rejected");
        let RokoError::Verify { gate, message } = error else {
            panic!("expected a verify failure, got {error}");
        };
        assert!(!marker.exists(), "the verify steps must not run: {message}");
        (gate, message)
    }

    #[tokio::test]
    async fn empty_diff_fails_before_verify_runs() {
        let temp = tempdir().expect("tempdir");
        commit_repo(
            temp.path(),
            &[("src/lib.rs", "pub fn one() -> u8 {\n    1\n}\n")],
        );
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        let marker = temp.path().join("verify-ran");
        // Without authored verify steps nothing can show the task's work was
        // already there, so the screen rejects the attempt before any step
        // runs. Authored steps get the say first
        // (`a_fresh_rerun_of_a_satisfied_task_is_not_rejected_as_no_changes`).
        task.files = vec!["src/lib.rs".to_string()];

        let (gate, message) = rejected(&dispatcher, &task, &marker).await;
        assert_eq!(gate, "pre_verify:no_changes");
        assert!(message.contains("no changes"), "{message}");
        assert!(message.contains("`src/lib.rs`"), "{message}");

        // The message is the next attempt's feedback.
        let next = dispatcher.next_retry_attempt(&make_spec(&task).plan_id, &task.id);
        let feedback = next.feedback.expect("feedback for the next attempt");
        assert!(feedback.raw_output.contains("no changes"), "{feedback:?}");

        // A task without `files`, like a non-implementer role, is exempt.
        task.files.clear();
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];
        let outputs = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("a task without files may change nothing");
        assert_eq!(
            TaskGateVerdict::from_signals(&outputs),
            Some(TaskGateVerdict::Passed)
        );
        assert!(marker.exists(), "its verify step ran");
    }

    /// gap-9eb1e1: an attempt that changes nothing, at a task whose work is
    /// already in the tree, as on a `--fresh` rerun of a finished task, is
    /// settled by the task's verify steps. They pass, so the task was already
    /// satisfied: its own outcome, which teaches no learner. On a tree
    /// without the work they fail, and the attempt is rejected as before.
    #[tokio::test]
    async fn a_fresh_rerun_of_a_satisfied_task_is_not_rejected_as_no_changes() {
        let done = "pub fn one() -> u8 {\n    1\n}\npub fn two() -> u8 {\n    one() + one()\n}\n";
        let temp = tempdir().expect("tempdir");
        commit_repo(temp.path(), &[("src/lib.rs", done)]);
        let runs_dir = temp.path().join(".roko/runs");
        let feedback = GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        let marker = temp.path().join("verify-ran");
        task.files = vec!["src/lib.rs".to_string()];
        task.verify = vec![verify_step(
            "structural",
            &format!(
                "grep -q 'pub fn two' src/lib.rs && touch {}",
                marker.display()
            ),
        )];
        let ctx = CellContext::new().with_run_id("fresh-rerun".to_string());
        let outputs = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the task's work was already there");
        assert_eq!(
            TaskGateVerdict::from_signals(&outputs),
            Some(TaskGateVerdict::AlreadySatisfied)
        );
        assert!(marker.exists(), "its verify step passed");
        // Closing the run's writer flushes its lines.
        drop(dispatcher);
        let verdicts = jsonl_rows_where(
            &runs_dir.join("fresh-rerun").join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let verdict = &verdicts[0];
        assert_eq!(verdict["outcome"], "already_satisfied");
        assert_eq!(verdict["gate_verdict"], "already_satisfied");
        assert_eq!(verdict["blame"], "none");
        assert!(verdict["learning_label"].is_null(), "{verdict}");

        // Without the work, the verify steps fail on the unchanged tree, and
        // the attempt is rejected for changing nothing.
        let temp = tempdir().expect("tempdir");
        commit_repo(
            temp.path(),
            &[("src/lib.rs", "pub fn one() -> u8 {\n    1\n}\n")],
        );
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        let marker = temp.path().join("verify-ran");
        task.files = vec!["src/lib.rs".to_string()];
        task.verify = vec![verify_step(
            "structural",
            &format!(
                "grep -q 'pub fn two' src/lib.rs && touch {}",
                marker.display()
            ),
        )];
        let (gate, message) = rejected(&dispatcher, &task, &marker).await;
        assert_eq!(gate, "pre_verify:no_changes");
        assert!(message.contains("as they found it"), "{message}");
        assert!(message.contains("did not pass"), "{message}");
        assert!(message.contains("grep -q 'pub fn two'"), "{message}");
    }

    #[tokio::test]
    async fn no_changes_names_gitignored_declared_files() {
        let temp = tempdir().expect("tempdir");
        commit_repo(
            temp.path(),
            &[
                (".gitignore", "out/\n"),
                ("src/lib.rs", "pub fn one() -> u8 {\n    1\n}\n"),
            ],
        );
        let writes_ignored = provider("mkdir -p out && printf 'one\\n' > out/one.txt", "done", 10);
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            &writes_ignored,
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        let marker = temp.path().join("verify-ran");
        task.files = vec!["out/one.txt".to_string()];
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];

        let (gate, message) = rejected(&dispatcher, &task, &marker).await;
        assert_eq!(gate, "pre_verify:no_changes");
        assert!(
            message.contains("the task names `out/one.txt`, which is gitignored"),
            "{message}"
        );
        assert!(
            message.contains("roko's diff and delivery cannot see changes to it"),
            "{message}"
        );
        assert!(!message.contains("as they found it"), "{message}");
    }

    #[tokio::test]
    async fn a_real_change_passes_and_stub_only_changes_do_not() {
        let temp = tempdir().expect("tempdir");
        commit_repo(
            temp.path(),
            &[("src/lib.rs", "pub fn one() -> u8 {\n    1\n}\n")],
        );
        let stub = provider(
            "printf 'pub fn two() -> u8 { todo!() }\\n' >> src/lib.rs",
            "done",
            10,
        );
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, &stub, no_auto_fix, GraphFeedbackContext::default()).await;
        let marker = temp.path().join("verify-ran");
        task.files = vec!["src/lib.rs".to_string()];
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];
        let (gate, message) = rejected(&dispatcher, &task, &marker).await;
        assert_eq!(gate, "pre_verify:no_changes");
        assert!(message.contains("stub"), "{message}");

        // The same task, once a later attempt writes the body, passes.
        std::fs::write(
            temp.path().join("src/lib.rs"),
            "pub fn one() -> u8 {\n    1\n}\npub fn two() -> u8 {\n    one() + one()\n}\n",
        )
        .expect("implement");
        std::fs::write(
            temp.path().join("fake-claude-stream.sh"),
            provider(":", "done", 10),
        )
        .expect("rewrite provider");
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("the change is real now");
        assert!(marker.exists());
    }

    #[tokio::test]
    async fn tampering_attempt_fails_before_verify() {
        let temp = tempdir().expect("tempdir");
        commit_repo(
            temp.path(),
            &[
                ("src/lib.rs", "pub fn one() -> u8 {\n    1\n}\n"),
                (
                    "tests/math.rs",
                    "#[test]\nfn one_is_one() {\n    assert_eq!(c5::one(), 1);\n}\n",
                ),
                ("plans/p/accept/one.sh", "echo 'ok 1'\necho '# pass 1'\n"),
            ],
        );
        // A real change to its own file, beside a skipped test and a
        // weakened pinned acceptance test.
        let tamper = provider(
            "printf 'pub fn two() -> u8 {\\n    one() + one()\\n}\\n' >> src/lib.rs\n\
             printf '#[test]\\n#[ignore]\\nfn later() {\\n    assert!(true);\\n}\\n' >> tests/math.rs\n\
             printf 'exit 0\\n' >> plans/p/accept/one.sh",
            "done",
            10,
        );
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, &tamper, no_auto_fix, GraphFeedbackContext::default())
                .await;
        let marker = temp.path().join("verify-ran");
        task.files = vec!["src/lib.rs".to_string()];
        task.accept = Some(crate::task_accept::TaskAccept {
            files: vec![crate::task_accept::AcceptFile {
                src: "accept/one.sh".to_string(),
                dest: "tests/accept/one.sh".to_string(),
                runner: "sh {dest}".to_string(),
                count: 1,
                timeout_ms: None,
            }],
        });
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];
        let mut spec = make_spec(&task);
        spec.plan_dir = "plans/p".to_string();

        let error = dispatcher
            .dispatch(&spec, Vec::new(), &CellContext::new())
            .await
            .expect_err("tampering fails the attempt");
        let RokoError::Verify { gate, message } = error else {
            panic!("expected a verify failure, got {error}");
        };
        assert!(!marker.exists(), "the verify steps must not run");
        assert_eq!(gate, "pre_verify:tamper");
        assert!(message.contains("skip_added `tests/math.rs`"), "{message}");
        assert!(
            message.contains("accept_edited `plans/p/accept/one.sh`"),
            "{message}"
        );
        assert!(
            !message.contains("src/lib.rs"),
            "the task's own change is not a finding: {message}"
        );
    }

    /// A workspace rung checks the task as its own verify steps do, so
    /// editing a script the rung runs is tampering too.
    #[tokio::test]
    async fn editing_a_workspace_rung_script_is_tampering() {
        let temp = tempdir().expect("tempdir");
        commit_repo(
            temp.path(),
            &[
                ("src/lib.rs", "pub fn one() -> u8 {\n    1\n}\n"),
                ("scripts/lint.sh", "exit 1\n"),
            ],
        );
        let tamper = provider(
            "printf 'pub fn two() -> u8 {\\n    one() + one()\\n}\\n' >> src/lib.rs\n\
             printf 'exit 0\\n' > scripts/lint.sh",
            "done",
            10,
        );
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            &tamper,
            |config| {
                no_auto_fix(config);
                config.gates.custom_rungs = vec![roko_core::config::GateRungConfig {
                    name: "lint".to_string(),
                    command: "sh scripts/lint.sh".to_string(),
                    timeout_secs: 10,
                    required: true,
                    parallel_with: Vec::new(),
                }];
            },
            GraphFeedbackContext::default(),
        )
        .await;
        let marker = temp.path().join("verify-ran");
        task.files = vec!["src/lib.rs".to_string()];
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];

        let (gate, message) = rejected(&dispatcher, &task, &marker).await;
        assert_eq!(gate, "pre_verify:tamper");
        assert!(
            message.contains("verify_script_edited `scripts/lint.sh`"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn scope_findings_fail_the_attempt_only_when_enforced() {
        for diff_scope in [DiffScope::Record, DiffScope::Enforce] {
            let temp = tempdir().expect("tempdir");
            commit_repo(
                temp.path(),
                &[
                    ("src/lib.rs", "pub fn one() -> u8 {\n    1\n}\n"),
                    ("README.md", "# c5\n"),
                ],
            );
            let wanders = provider(
                "printf 'pub fn two() -> u8 {\\n    one() + one()\\n}\\n' >> src/lib.rs\n\
                 printf 'Also this.\\n' >> README.md",
                "done",
                10,
            );
            let (dispatcher, mut task) = make_test_dispatcher(
                &temp,
                &wanders,
                |config| {
                    no_auto_fix(config);
                    config.gates.diff_scope = diff_scope;
                },
                GraphFeedbackContext::default(),
            )
            .await;
            let marker = temp.path().join("verify-ran");
            task.files = vec!["src/lib.rs".to_string()];
            task.verify = vec![verify_step(
                "structural",
                &format!("touch {}", marker.display()),
            )];

            if diff_scope == DiffScope::Record {
                dispatcher
                    .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
                    .await
                    .expect("a scope finding is only recorded");
                assert!(marker.exists());
            } else {
                let (gate, message) = rejected(&dispatcher, &task, &marker).await;
                assert_eq!(gate, "pre_verify:scope");
                assert!(message.contains("outside_scope `README.md`"), "{message}");
            }
        }
    }

    #[tokio::test]
    async fn the_pre_verify_screen_runs_post_dispatch_check() {
        // A reviewer, whose contract forbids file writes, changes a file.
        let temp = tempdir().expect("tempdir");
        commit_repo(
            temp.path(),
            &[("src/lib.rs", "pub fn one() -> u8 {\n    1\n}\n")],
        );
        let writes = provider("printf '// reviewed\\n' >> src/lib.rs", "Looks right.", 10);
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, &writes, no_auto_fix, GraphFeedbackContext::default())
                .await;
        let marker = temp.path().join("verify-ran");
        task.role = Some("reviewer".to_string());
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];
        let (gate, message) = rejected(&dispatcher, &task, &marker).await;
        assert_eq!(gate, "pre_verify:safety");
        assert!(message.contains("contract_violation"), "{message}");
        assert!(message.contains("forbids file writes"), "{message}");

        // An implementer's output carries a credential in an exact format.
        // Git cannot snapshot this tree, so the output alone is checked.
        let temp = tempdir().expect("tempdir");
        let credential = format!("ghp_{}", "a".repeat(36));
        let leaks = provider(":", &format!("Pushed with {credential}."), 10);
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, &leaks, no_auto_fix, GraphFeedbackContext::default()).await;
        let marker = temp.path().join("verify-ran");
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];
        let (gate, message) = rejected(&dispatcher, &task, &marker).await;
        assert_eq!(gate, "pre_verify:safety");
        assert!(message.contains("secret_leak"), "{message}");
        assert!(
            !message.contains(&credential),
            "the rejection must not repeat the secret: {message}"
        );
    }

    #[tokio::test]
    async fn a_name_value_secret_match_is_recorded_without_blocking() {
        // Code a summary quotes matches the scrubber's `NAME = value` rule,
        // but is no credential: the attempt goes on to its verify steps.
        let temp = tempdir().expect("tempdir");
        let quotes = provider(
            ":",
            "Added api_key: Option<String> to the config, and the client does let token = next();",
            10,
        );
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, &quotes, no_auto_fix, GraphFeedbackContext::default())
                .await;
        let marker = temp.path().join("verify-ran");
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];
        let outputs = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("a NAME = value match does not block");
        assert_eq!(
            TaskGateVerdict::from_signals(&outputs),
            Some(TaskGateVerdict::Passed)
        );
        assert!(marker.exists(), "its verify step ran");
    }

    #[tokio::test]
    async fn overlong_output_fails_before_verify_runs() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            &provider(":", "a very long answer", 5_000),
            |config| {
                no_auto_fix(config);
                config
                    .gates
                    .max_output_tokens
                    .insert("implementer".to_string(), 1_000);
            },
            GraphFeedbackContext::default(),
        )
        .await;
        let marker = temp.path().join("verify-ran");
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];

        let (gate, message) = rejected(&dispatcher, &task, &marker).await;
        assert_eq!(gate, "pre_verify:overlong_output");
        assert!(message.contains("5000 output tokens"), "{message}");
        assert!(message.contains("1000"), "{message}");
    }

    #[tokio::test]
    async fn malformed_reviewer_output_fails_before_verify_runs() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            &provider(":", r"Review done.\n```toml\n[verdict\noverall = \n```", 40),
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        let marker = temp.path().join("verify-ran");
        task.role = Some("reviewer".to_string());
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];

        let (gate, message) = rejected(&dispatcher, &task, &marker).await;
        assert_eq!(gate, "pre_verify:malformed_output");
        assert!(message.contains("does not parse"), "{message}");
    }

    #[test]
    fn products_are_judged_by_their_fenced_blocks() {
        assert_eq!(malformed_product("  \n").as_deref(), Some("it is empty"));
        assert_eq!(malformed_product("APPROVE: looks right"), None);
        assert_eq!(
            malformed_product("```toml\n[verdict]\noverall = \"approve\"\n```\n"),
            None
        );
        assert_eq!(
            malformed_product("```json\n{\"status\": \"pass\"}\n```"),
            None
        );
        // Another language's block is skipped whole, even when it holds
        // text that looks like a fence.
        assert_eq!(malformed_product("```rust\nlet x = 1;\n```\ndone"), None);
        assert_eq!(
            malformed_product("```toml\n[verdict]\noverall = \"approve\"\n").as_deref(),
            Some("its ```toml block is cut off")
        );
        assert!(
            malformed_product("```json\n{\"status\": }\n```")
                .is_some_and(|problem| problem.starts_with("its ```json block does not parse"))
        );
        assert!(output_is_the_product("quick_reviewer"));
        assert!(!output_is_the_product("implementer"));
    }
}
