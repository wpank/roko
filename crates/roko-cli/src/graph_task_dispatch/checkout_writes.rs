//! What changed in the person's own checkout while an attempt ran in a
//! checkout of its own (bug-b7d9ad, reg-d2deba).
//!
//! An attempt that runs in a worktree of its own, or in a scratch copy, should
//! change only that. A path compiled in from the person's checkout breaks
//! this: a warm build tree roko clones into the worktree (gap-6754ab) keeps
//! what was built with the checkout's path, such as Undertow's
//! `UNDERTOW_SOURCE_DIR`, and a check that writes through it writes into the
//! person's checkout. In round v7's rehearsal p41, T2's check wrote the review
//! PNGs into the base checkout. T2's worktree held nothing, the attempt
//! settled `already_satisfied`, and the delivery had no PNGs. In a live run
//! that base is the person's real checkout.
//!
//! roko watches the person's checkout ([`CheckoutState`]: its dirty paths,
//! untracked files included, each with its size and mtime), leaving out what
//! roko itself writes there during a run: `.roko/`, the plan's own directory
//! and nested checkouts such as worktrees. It watches over three windows:
//! - the try itself, from its dispatch to its end (reg-d2deba): the agent's
//!   own commands and builds, and its own checks, `run_tests` with
//!   `build: "checks"` and the hand-off check, which gap-6df2bf lets stand
//!   for the gate's steps;
//! - each run of the try's own checks, which `run_checks` records with the
//!   paths it changed, so a path the try's window shows is put down to its
//!   checks or to a command its agent ran;
//! - the gate's verify steps (`GraphTaskDispatcher::screen_and_verify`).
//!
//! What changed:
//! - the hub hears a warning diagnosis that names the paths, which changed
//!   outside the checks, how to drop or keep them, and the other tasks that
//!   ran meanwhile, any of which may have written them;
//! - the attempt's verdict lists the paths as `checkout_written` scope
//!   findings, which the run summary, `roko diagnose` and the end of `roko plan
//!   run` read;
//! - an attempt that changed nothing, whose verify steps pass at the gate
//!   while the checkout changes, settles `unverified`, not
//!   `already_satisfied`, since its checks may have tested the person's
//!   checkout instead of its own tree. That costs no retry: a path compiled
//!   into the build is not the agent's to fix.
//!
//! The person editing their own checkout meanwhile reads the same.

use std::collections::{BTreeMap, BTreeSet};

use roko_core::dashboard_snapshot::{DiagnosisSeverity, DiagnosisSummary};
use roko_learn::telemetry::ScopeFinding;
use roko_learn::telemetry::records::SCOPE_CHECKOUT_WRITTEN;
use roko_std::tool::builtin::checkout_state::{CheckoutChanges, CheckoutState, ROKO_DIR, is_under};
use roko_std::tool::builtin::task_checks::{read_check_runs, task_check_runs_path};

use super::*;

/// The most paths a diagnosis names.
const NAMED_PATHS: usize = 5;

/// The person's checkout as a window of an attempt begins: the try, or its
/// gate's steps (bug-b7d9ad, reg-d2deba).
pub(super) struct CheckoutWatch {
    before: CheckoutState,
    /// The directories left out, relative to the checkout.
    skipped: Vec<String>,
    /// From here on, the attempts in flight beside it.
    mark: super::sibling_settle::WriterMark,
}

/// What changed in the person's checkout while an attempt ran.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CheckoutWritten {
    /// Tracked files that were clean before and changed.
    pub(super) changed: Vec<String>,
    /// Files that were not there before and are now, untracked.
    pub(super) added: Vec<String>,
    /// Paths that had changes of the person's own before and changed again,
    /// or went back: dropping them would drop those changes too.
    pub(super) touched: Vec<String>,
    /// Of all of them, those that changed while none of the attempt's checks
    /// ran: a build or command its agent ran itself wrote them (reg-d2deba).
    pub(super) outside_checks: Vec<String>,
    /// The other tasks with an attempt in flight meanwhile, as
    /// `"{plan_id}/{task_id}"`, any of which may have written them instead.
    pub(super) alongside: Vec<String>,
}

impl CheckoutWritten {
    /// `changes`, all made while checks ran, beside no other task.
    fn of(changes: CheckoutChanges) -> Self {
        let CheckoutChanges {
            changed,
            added,
            touched,
        } = changes;
        Self {
            changed,
            added,
            touched,
            ..Self::default()
        }
    }

    /// Whether nothing changed.
    pub(super) fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.added.is_empty() && self.touched.is_empty()
    }

    /// What changed in this window or in `later`, one after the other: a path
    /// keeps the kind it first had, and a path `later`'s checks changed
    /// counts as changed by checks.
    pub(super) fn merged(&self, later: &Self) -> Self {
        let mut merged = self.clone();
        let known: BTreeSet<String> = self.paths().into_iter().collect();
        let new = |paths: &[String]| -> Vec<String> {
            paths
                .iter()
                .filter(|path| !known.contains(*path))
                .cloned()
                .collect()
        };
        merged.changed.extend(new(&later.changed));
        merged.added.extend(new(&later.added));
        merged.touched.extend(new(&later.touched));
        let by_checks: BTreeSet<&String> = later
            .paths_ref()
            .filter(|path| !later.outside_checks.contains(path))
            .collect();
        merged
            .outside_checks
            .retain(|path| !by_checks.contains(path));
        merged.outside_checks.extend(new(&later.outside_checks));
        for key in &later.alongside {
            if !merged.alongside.contains(key) {
                merged.alongside.push(key.clone());
            }
        }
        merged.alongside.sort();
        merged
    }

    fn paths_ref(&self) -> impl Iterator<Item = &String> {
        self.changed.iter().chain(&self.added).chain(&self.touched)
    }

    /// Every path that changed, sorted.
    pub(super) fn paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self
            .changed
            .iter()
            .chain(&self.added)
            .chain(&self.touched)
            .cloned()
            .collect();
        paths.sort();
        paths
    }
}

impl GraphTaskDispatcher {
    /// The person's checkout as a window of an attempt of plan `spec` that
    /// runs in `effective_workdir` begins: the try, or its gate's steps;
    /// `None` when the attempt runs in that checkout itself, or git cannot
    /// list it (bug-b7d9ad, reg-d2deba).
    pub(super) async fn watch_checkout(
        &self,
        spec: &TaskExecutionSpec,
        effective_workdir: &Path,
    ) -> Option<CheckoutWatch> {
        if effective_workdir == self.workdir.as_path() {
            return None;
        }
        // What roko itself writes in the checkout during a run.
        let mut skipped = vec![ROKO_DIR.to_string()];
        let plan_dir = Path::new(&spec.plan_dir);
        let plan_dir = if plan_dir.is_absolute() {
            plan_dir.strip_prefix(&self.workdir).ok()
        } else {
            Some(plan_dir)
        };
        if let Some(plan_dir) = plan_dir.filter(|dir| !dir.as_os_str().is_empty()) {
            skipped.push(plan_dir.to_string_lossy().trim_end_matches('/').to_string());
        }
        let mark = self.in_flight.mark();
        match CheckoutState::of(&self.workdir, &skipped).await {
            Some(before) => Some(CheckoutWatch {
                before,
                skipped,
                mark,
            }),
            None => {
                self.in_flight.release(&mark);
                None
            }
        }
    }

    /// What changed in the person's checkout since `watch` began, while
    /// `task_key` (`"{plan_id}/{task_id}"`) ran; nothing without a `watch`.
    pub(super) async fn checkout_written(
        &self,
        watch: Option<CheckoutWatch>,
        task_key: &str,
    ) -> CheckoutWritten {
        self.checkout_written_with_skipped(watch, task_key).await.0
    }

    /// [`Self::checkout_written`], and the directories its watch left out.
    async fn checkout_written_with_skipped(
        &self,
        watch: Option<CheckoutWatch>,
        task_key: &str,
    ) -> (CheckoutWritten, Vec<String>) {
        let Some(CheckoutWatch {
            before,
            skipped,
            mark,
        }) = watch
        else {
            return (CheckoutWritten::default(), Vec::new());
        };
        let mut written = CheckoutState::of(&self.workdir, &skipped)
            .await
            .map(|after| CheckoutWritten::of(after.changes_since(&before)))
            .unwrap_or_default();
        if !written.is_empty() {
            written.alongside = self.in_flight.sibling_tasks_since(&mark, task_key);
        }
        self.in_flight.release(&mark);
        (written, skipped)
    }

    /// What changed in the person's checkout over the try of `task`, attempt
    /// `attempt_number` (`attempt_key`), since `watch` began at its dispatch
    /// (reg-d2deba). A path a run of the try's own checks changed
    /// (`run_tests` with `build: "checks"`, the hand-off check), as
    /// `run_checks` recorded it, is put down to its checks; any other path to
    /// a build or command its agent ran itself. When anything changed it is
    /// said at once, in the log, on the hub and for the end of the plan run,
    /// since the try may end before its gate; the caller records the
    /// returned findings on the attempt's verdict, and passes what changed on
    /// to the gate's window ([`Self::settle_task_verification`]).
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn try_checkout_written(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt_number: u32,
        attempt_key: &str,
        checkout: &Path,
        watch: Option<CheckoutWatch>,
    ) -> (CheckoutWritten, Vec<ScopeFinding>) {
        let task_key = format!("{}/{}", spec.plan_id, task.id);
        let (mut written, skipped) = self.checkout_written_with_skipped(watch, &task_key).await;
        if written.is_empty() {
            return (written, Vec::new());
        }
        let by_checks: BTreeSet<String> =
            read_check_runs(&task_check_runs_path(&self.workdir, attempt_key))
                .into_iter()
                .flat_map(|run| run.checkout_written)
                .filter(|path| !skipped.iter().any(|root| is_under(path, root)))
                .collect();
        written.outside_checks = written
            .paths()
            .into_iter()
            .filter(|path| !by_checks.contains(path))
            .collect();
        let mut findings = Vec::new();
        self.report_checkout_written(
            spec,
            task,
            attempt_number,
            checkout,
            &written,
            false,
            &mut findings,
        );
        (written, findings)
    }

    /// The paths of each task of plan `plan_id` whose checks changed the
    /// person's checkout in this run, by task (bug-b7d9ad).
    pub(crate) fn checkout_writes(&self, plan_id: &str) -> BTreeMap<String, Vec<String>> {
        self.checkout_writes
            .lock()
            .get(plan_id)
            .map(|tasks| {
                tasks
                    .iter()
                    .map(|(task, paths)| (task.clone(), paths.iter().cloned().collect()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Say that the person's checkout changed (`written`) while attempt
    /// `attempt_number` of `task` ran in `checkout`, and whether the attempt
    /// settles `unverified` for it: in the log, on the hub as a warning
    /// diagnosis, in the attempt's verdict as `checkout_written` scope
    /// findings (`scope_findings` gains those of the paths it lacks), and for
    /// the end of the plan run (bug-b7d9ad, reg-d2deba). A later report of the
    /// same attempt replaces the hub's diagnosis.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn report_checkout_written(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt_number: u32,
        checkout: &Path,
        written: &CheckoutWritten,
        unverified: bool,
        scope_findings: &mut Vec<ScopeFinding>,
    ) {
        let paths = written.paths();
        let trees = crate::orchestrator::worktree::warm_build_dirs(checkout, None);
        tracing::warn!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            attempt = attempt_number,
            paths = ?paths,
            outside_checks = ?written.outside_checks,
            alongside = ?written.alongside,
            warm_build_trees = trees.len(),
            unverified,
            "the person's checkout changed while the attempt ran in its own"
        );
        for path in &paths {
            let known = scope_findings
                .iter()
                .any(|finding| finding.kind == SCOPE_CHECKOUT_WRITTEN && finding.path == *path);
            if !known {
                scope_findings.push(ScopeFinding {
                    path: path.clone(),
                    kind: SCOPE_CHECKOUT_WRITTEN.to_string(),
                });
            }
        }
        self.checkout_writes
            .lock()
            .entry(spec.plan_id.clone())
            .or_default()
            .entry(task.id.clone())
            .or_default()
            .extend(paths.iter().cloned());
        let Some(tui) = &self.tui_bridge else {
            return;
        };
        tui.diagnosis(checkout_written_diagnosis(
            spec,
            task,
            &self.workdir,
            written,
            &trees,
            unverified,
        ));
    }
}

/// The hub's warning that the person's checkout at `workdir` changed
/// (`written`) while the checks of `task` ran (bug-b7d9ad): which paths, the
/// likely cause, how to drop or keep them, and the other tasks that ran
/// meanwhile. `trees` are the build trees of the attempt's worktree.
fn checkout_written_diagnosis(
    spec: &TaskExecutionSpec,
    task: &TaskDef,
    workdir: &Path,
    written: &CheckoutWritten,
    trees: &[PathBuf],
    unverified: bool,
) -> DiagnosisSummary {
    let mut kinds = Vec::new();
    if !written.changed.is_empty() {
        kinds.push(format!("modified: {}", named(&written.changed)));
    }
    if !written.added.is_empty() {
        kinds.push(format!("added: {}", named(&written.added)));
    }
    if !written.touched.is_empty() {
        kinds.push(format!(
            "modified over changes of yours: {}",
            named(&written.touched)
        ));
    }
    let cause = if trees.is_empty() {
        "a path the checks name in full, outside the directory they run in".to_string()
    } else {
        let names: Vec<String> = trees
            .iter()
            .map(|tree| format!("`{}`", tree.display()))
            .collect();
        format!(
            "a path compiled into a build tree of the attempt's worktree ({}) that still names \
             your checkout: roko starts each worktree with copies of your checkout's build trees",
            names.join(", ")
        )
    };
    let outcome = if unverified {
        "The task is unverified: it changed nothing in its own tree, and its checks may have \
         tested your checkout instead."
    } else {
        "Its checks may have read your checkout instead of the attempt's tree."
    };
    let plan_prefix = format!("{}/", spec.plan_id);
    let beside: Vec<&str> = written
        .alongside
        .iter()
        .map(|key| key.strip_prefix(&plan_prefix).unwrap_or(key))
        .collect();
    let others = if beside.is_empty() {
        String::new()
    } else {
        format!(
            " Other tasks ran at the same time ({}), so any of them may have made these changes \
             instead.",
            beside.join(", ")
        )
    };
    // reg-d2deba: a path no run of its checks changed was written by a build
    // or command its agent ran itself.
    let outside = if written.outside_checks.is_empty() {
        String::new()
    } else {
        format!(
            " Of these, {} changed while none of its checks ran: a build or command its agent \
             ran itself (`run_tests` or `bash`) wrote there, unless you changed them yourself.",
            named(&written.outside_checks)
        )
    };
    let during = if written.outside_checks.is_empty() {
        format!("{}'s checks", task.id)
    } else {
        task.id.clone()
    };
    let checkout = shell_word(&workdir.to_string_lossy());
    let mut undo = Vec::new();
    if !written.changed.is_empty() {
        undo.push(format!(
            "`git -C {checkout} checkout -- {}`",
            shell_words(&written.changed)
        ));
    }
    if !written.added.is_empty() {
        undo.push(format!(
            "`git -C {checkout} clean -f -- {}`",
            shell_words(&written.added)
        ));
    }
    let mut action = if undo.is_empty() {
        String::new()
    } else {
        format!("To drop them: {}. ", undo.join(" and "))
    };
    if !written.touched.is_empty() {
        action.push_str(&format!(
            "Look at {} before dropping anything: they held changes of yours already. ",
            named(&written.touched)
        ));
    }
    action.push_str(
        "Or keep them and commit them yourself. Then make the project's checks write under the \
         directory they run in (a path chosen at run time, not compiled in), and run the task \
         again.",
    );
    DiagnosisSummary {
        id: format!("{SCOPE_CHECKOUT_WRITTEN}:{}/{}", spec.plan_id, task.id),
        severity: DiagnosisSeverity::Warn,
        subject: format!("The run changed your checkout while {during} ran"),
        detail: format!(
            "Task `{}` of plan `{}` runs in its own worktree, but while {} ran, the run modified \
             your own checkout at `{}` ({}). roko left these changes there and did not deliver \
             them. The likely cause is {cause}.{outside} {outcome}{others}",
            task.id,
            spec.plan_id,
            if written.outside_checks.is_empty() {
                "its checks"
            } else {
                "it"
            },
            workdir.display(),
            kinds.join("; ")
        ),
        suggested_action: Some(action),
        ..DiagnosisSummary::default()
    }
}

/// `paths` in backquotes, at most [`NAMED_PATHS`] of them and how many more.
fn named(paths: &[String]) -> String {
    let mut named: Vec<String> = paths
        .iter()
        .take(NAMED_PATHS)
        .map(|path| format!("`{path}`"))
        .collect();
    if paths.len() > NAMED_PATHS {
        named.push(format!("{} more", paths.len() - NAMED_PATHS));
    }
    named.join(", ")
}

/// `paths` as shell words, space-separated.
fn shell_words(paths: &[String]) -> String {
    paths
        .iter()
        .map(|path| shell_word(path))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `word` as one shell word: as it is when only safe characters make it up,
/// else in single quotes.
fn shell_word(word: &str) -> String {
    let safe = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-+@:,=".contains(c));
    if safe {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_execution::WorktreeExecutionWorkspaceProvider;
    use crate::graph_task_dispatch::tests::{
        FIXTURE_PROVIDER_TIMEOUT_MS, VERIFY_PROVIDER, final_turn, make_spec,
        make_test_dispatcher_with, no_auto_fix, spawn_openai_mock, tool_call_turn, verify_step,
    };
    use crate::orchestrator::worktree::{WorktreeConfig, WorktreeManager};

    fn git(dir: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .status()
            .expect("run git");
        assert!(status.success(), "git {args:?}");
    }

    /// reg-d2deba: since gap-6df2bf a gate step the try's own checks passed on
    /// its final tree does not run again, so a check that writes into the
    /// person's checkout writes there only in the try's own `run_tests`. The
    /// try's window says so all the same, and puts that file down to its
    /// checks; a file a command of its agent's wrote there is put down to the
    /// agent. Both reach the hub, the verdict and the end of the plan run.
    #[tokio::test]
    async fn a_try_s_own_checks_that_write_into_the_checkout_say_so() {
        let repo = tempfile::tempdir().expect("repo");
        let worktrees = tempfile::tempdir().expect("worktrees");
        git(repo.path(), &["init", "--quiet", "-b", "main"]);
        std::fs::write(repo.path().join("README.md"), "base\n").expect("README");
        git(repo.path(), &["add", "README.md"]);
        git(
            repo.path(),
            &[
                "-c",
                "user.email=o@example.test",
                "-c",
                "user.name=O",
                "commit",
                "--quiet",
                "-m",
                "base",
            ],
        );
        let shot = repo.path().join("artifacts/shot.png");
        let note = repo.path().join("notes/agent.txt");
        let quoted = |path: &Path| shell_word(&path.to_string_lossy());
        // The task's check writes through a path into the person's checkout,
        // as a build path compiled in from it does.
        let check = format!(
            "mkdir -p {} && printf png > {} && test -f a.txt",
            quoted(&repo.path().join("artifacts")),
            quoted(&shot)
        );
        let by_hand = format!(
            "mkdir -p {} && printf note > {}",
            quoted(&repo.path().join("notes")),
            quoted(&note)
        );
        let (base_url, _requests) = spawn_openai_mock(vec![
            tool_call_turn(
                "w1",
                "write_file",
                serde_json::json!({ "path": "a.txt", "content": "done\n" }),
            ),
            tool_call_turn("c1", "run_tests", serde_json::json!({ "build": "checks" })),
            tool_call_turn("b1", "bash", serde_json::json!({ "command": by_hand })),
            final_turn("done"),
        ]);
        let provider = Arc::new(WorktreeExecutionWorkspaceProvider::new(
            WorktreeManager::new(WorktreeConfig {
                repo_root: repo.path().to_path_buf(),
                base_branch: "HEAD".to_string(),
                worktrees_root: worktrees.path().to_path_buf(),
                max_live: None,
                idle_ttl: std::time::Duration::from_secs(3600),
            }),
        ));
        let hub = crate::state_hub::StateHub::default_capacity();
        let sender = hub.sender();
        let runs_dir = repo.path().join(".roko/runs");
        let (dispatcher, mut task) = make_test_dispatcher_with(
            &repo,
            VERIFY_PROVIDER,
            |config| {
                no_auto_fix(config);
                api_implementer(config, &base_url);
            },
            GraphFeedbackContext {
                runs_dir: Some(runs_dir.clone()),
                ..GraphFeedbackContext::default()
            },
            |dispatcher| {
                dispatcher
                    .with_workspace_provider(provider)
                    .with_tui_bridge(crate::runner::tui_bridge::TuiBridge::new(sender))
            },
        )
        .await;
        task.files = vec!["a.txt".to_string()];
        task.verify = vec![verify_step("test", &check)];
        task.model_hint = Some("api-model".to_string());
        let spec = make_spec(&task);
        let run = "own-checks";

        dispatcher
            .dispatch(
                &spec,
                Vec::new(),
                &CellContext::new().with_run_id(run.to_string()),
            )
            .await
            .expect("the try passes");

        assert!(
            shot.exists() && note.exists(),
            "both landed in the checkout"
        );
        let diagnoses: Vec<DiagnosisSummary> = hub
            .replay_from(0)
            .into_iter()
            .filter_map(|envelope| match envelope.payload {
                roko_core::DashboardEvent::Diagnosis { summary }
                    if summary.id.starts_with("checkout_written:") =>
                {
                    Some(summary)
                }
                _ => None,
            })
            .collect();
        assert_eq!(diagnoses.len(), 1, "{diagnoses:?}");
        let summary = &diagnoses[0];
        assert_eq!(
            summary.id,
            format!("checkout_written:{}/{}", spec.plan_id, task.id)
        );
        assert_eq!(
            summary.subject,
            format!("The run changed your checkout while {} ran", task.id)
        );
        for part in [
            "(added: `artifacts/shot.png`, `notes/agent.txt`)",
            "Of these, `notes/agent.txt` changed while none of its checks ran",
        ] {
            assert!(summary.detail.contains(part), "{part}: {}", summary.detail);
        }
        assert_eq!(
            dispatcher.checkout_writes(&spec.plan_id),
            [(
                task.id.clone(),
                vec![
                    "artifacts/shot.png".to_string(),
                    "notes/agent.txt".to_string()
                ]
            )]
            .into_iter()
            .collect::<BTreeMap<_, _>>()
        );

        // Closing the run's writer flushes its lines.
        drop(dispatcher);
        let verdicts = crate::graph_task_dispatch::tests::jsonl_rows_where(
            &runs_dir.join(run).join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let verdict = &verdicts[0];
        assert_eq!(verdict["outcome"], "passed", "{verdict}");
        assert_eq!(
            verdict["steps"][0]["self_checked"], true,
            "the gate took the try's own pass: {verdict}"
        );
        let written: Vec<&str> = verdict["scope_findings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|finding| finding["kind"] == SCOPE_CHECKOUT_WRITTEN)
            .filter_map(|finding| finding["path"].as_str())
            .collect();
        assert_eq!(
            written,
            ["artifacts/shot.png", "notes/agent.txt"],
            "{verdict}"
        );
    }

    /// `config` with `api-model`, an OpenAI-compatible model at `base_url`,
    /// for a task that names it, the helper calls still on the fake CLI, and
    /// no hand-off check, so the try ends when its agent hands back.
    fn api_implementer(config: &mut RokoConfig, base_url: &str) {
        config.providers.insert(
            "mock_api".to_string(),
            roko_core::config::schema::ProviderConfig {
                kind: roko_core::agent::ProviderKind::OpenAiCompat,
                base_url: Some(base_url.to_string()),
                api_key_env: Some("PATH".to_string()),
                timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                ..roko_core::config::schema::ProviderConfig::default()
            },
        );
        config.models.insert(
            "api-model".to_string(),
            roko_core::config::schema::ModelProfile {
                provider: "mock_api".to_string(),
                slug: "api-model-1".to_string(),
                context_window: 128_000,
                max_output: Some(1_024),
                max_tools: Some(32),
                supports_tools: true,
                tool_format: "openai_json".to_string(),
                cost_input_per_m: Some(1.0),
                cost_output_per_m: Some(2.0),
                ..roko_core::config::schema::ModelProfile::default()
            },
        );
        config
            .models
            .get_mut("stream-model")
            .expect("the fake CLI's model")
            .supports_tools = true;
        config.routing.fast_task_model = "stream-model".to_string();
        config.gates.hand_off_continues = 0;
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 0;
    }

    /// bug-b7d9ad: `git status -z` entries become dirty paths, leaving out a
    /// rename's source, roko's own `.roko/`, the plan's directory and a
    /// nested checkout; between two states a clean tracked file that changed
    /// is `changed`, a new untracked one `added`, and one that held changes
    /// already `touched`; a path the same in both is none of them.
    /// reg-d2deba: the try's window and its gate's add up, a path keeping its
    /// first kind, and one the gate's checks changed is no longer put down to
    /// the agent's own commands.
    #[test]
    fn a_checkout_state_names_what_changed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = |name: &str, body: &str| {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
            std::fs::write(path, body).expect("write");
        };
        file("a.txt", "a");
        let porcelain = b" M a.txt\0R  new.txt\0old.txt\0?? .roko/state.json\0\
                          ?? plans/p1/status.json\0?? plans/p10.txt\0?? wt/attempt-1/\0\
                          ?? art/x.png\0";
        let skipped = [ROKO_DIR.to_string(), "plans/p1".to_string()];
        let parsed = CheckoutState::parse(dir.path(), porcelain, &skipped);
        let paths: Vec<&str> = parsed.paths().collect();
        assert_eq!(paths, ["a.txt", "art/x.png", "new.txt", "plans/p10.txt"]);

        file("kept.txt", "abc");
        file("grown.txt", "abc");
        let before = CheckoutState::parse(dir.path(), b" M kept.txt\0 M grown.txt\0", &[]);
        file("grown.txt", "abcdefghi");
        file("artifacts/ui/editor-idle.png", "png");
        file("artifacts/ui/editor-playing.png", "png");
        let after = CheckoutState::parse(
            dir.path(),
            b" M kept.txt\0 M grown.txt\0 M artifacts/ui/editor-playing.png\0\
              ?? artifacts/ui/editor-idle.png\0",
            &[],
        );
        let written = CheckoutWritten::of(after.changes_since(&before));
        assert_eq!(written.changed, ["artifacts/ui/editor-playing.png"]);
        assert_eq!(written.added, ["artifacts/ui/editor-idle.png"]);
        assert_eq!(written.touched, ["grown.txt"]);
        assert_eq!(
            written.paths(),
            [
                "artifacts/ui/editor-idle.png",
                "artifacts/ui/editor-playing.png",
                "grown.txt"
            ]
        );
        assert!(after.changes_since(&after).is_empty());

        // The try wrote two files, one by a command of its agent's; the gate's
        // checks then wrote that one again, and a third.
        let try_window = CheckoutWritten {
            added: vec!["art/a.png".to_string(), "art/b.png".to_string()],
            outside_checks: vec!["art/b.png".to_string()],
            alongside: vec!["p/T3".to_string()],
            ..CheckoutWritten::default()
        };
        let gate_window = CheckoutWritten {
            changed: vec!["art/c.png".to_string()],
            touched: vec!["art/b.png".to_string()],
            alongside: vec!["p/T2".to_string()],
            ..CheckoutWritten::default()
        };
        let merged = try_window.merged(&gate_window);
        assert_eq!(merged.added, ["art/a.png", "art/b.png"]);
        assert_eq!(merged.changed, ["art/c.png"]);
        assert!(merged.touched.is_empty(), "{merged:?}");
        assert!(merged.outside_checks.is_empty(), "{merged:?}");
        assert_eq!(merged.alongside, ["p/T2", "p/T3"]);
    }

    /// bug-b7d9ad: the hub's warning names the person's checkout and what
    /// changed in it, says how to drop each kind of change or keep them, and
    /// names the other tasks that ran meanwhile.
    #[test]
    fn the_checkout_warning_says_how_to_drop_or_keep_the_changes() {
        let spec = TaskExecutionSpec {
            plan_id: "p4".to_string(),
            ..TaskExecutionSpec::default()
        };
        let mut task = crate::graph_task_dispatch::tests::make_task_def("focused");
        task.id = "T2".to_string();
        let written = CheckoutWritten {
            changed: vec!["artifacts/ui/editor idle.png".to_string()],
            added: vec!["artifacts/ui/new.png".to_string()],
            touched: vec!["README.md".to_string()],
            alongside: vec!["p4/T3".to_string(), "p5/T1".to_string()],
            ..CheckoutWritten::default()
        };
        let summary = checkout_written_diagnosis(
            &spec,
            &task,
            Path::new("/Users/me/undertow"),
            &written,
            &[PathBuf::from("build")],
            true,
        );
        assert_eq!(summary.id, "checkout_written:p4/T2");
        assert_eq!(
            summary.subject,
            "The run changed your checkout while T2's checks ran"
        );
        let detail = &summary.detail;
        for part in [
            "the run modified your own checkout at `/Users/me/undertow` (modified: \
             `artifacts/ui/editor idle.png`; added: `artifacts/ui/new.png`; modified over \
             changes of yours: `README.md`)",
            "a build tree of the attempt's worktree (`build`)",
            "The task is unverified",
            "Other tasks ran at the same time (T3, p5/T1)",
        ] {
            assert!(detail.contains(part), "{part}: {detail}");
        }
        let action = summary.suggested_action.expect("what to do");
        for part in [
            "`git -C /Users/me/undertow checkout -- 'artifacts/ui/editor idle.png'`",
            "`git -C /Users/me/undertow clean -f -- artifacts/ui/new.png`",
            "Look at `README.md` before dropping anything",
            "Or keep them and commit them yourself.",
        ] {
            assert!(action.contains(part), "{part}: {action}");
        }
        assert_eq!(shell_word("it's"), r"'it'\''s'");

        // reg-d2deba: a path no run of its checks changed is put down to a
        // command its agent ran.
        let by_agent = CheckoutWritten {
            outside_checks: vec!["artifacts/ui/new.png".to_string()],
            ..written
        };
        let summary = checkout_written_diagnosis(
            &spec,
            &task,
            Path::new("/Users/me/undertow"),
            &by_agent,
            &[],
            false,
        );
        assert_eq!(
            summary.subject,
            "The run changed your checkout while T2 ran"
        );
        for part in [
            "but while it ran, the run modified your own checkout",
            "Of these, `artifacts/ui/new.png` changed while none of its checks ran: a build or \
             command its agent ran itself (`run_tests` or `bash`) wrote there, unless you \
             changed them yourself.",
        ] {
            assert!(summary.detail.contains(part), "{part}: {}", summary.detail);
        }
    }
}
