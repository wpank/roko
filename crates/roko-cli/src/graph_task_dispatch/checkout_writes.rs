//! What changed in the person's own checkout while an attempt's checks ran
//! (bug-b7d9ad).
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
//! [`CheckoutState`] is the checkout's dirty paths as `git status` lists them,
//! untracked files included, each with its size and mtime, leaving out what
//! roko itself writes there during a run: `.roko/`, the plan's own directory
//! and nested checkouts such as worktrees. `GraphTaskDispatcher::screen_and_verify`
//! takes one before an attempt's checks and one after, and what changed between
//! them changed while the checks ran:
//! - the hub hears a warning diagnosis that names the paths, how to drop or
//!   keep them, and the other tasks that ran meanwhile, any of which may have
//!   written them;
//! - the attempt's verdict lists the paths as `checkout_written` scope
//!   findings, which the run summary, `roko diagnose` and the end of `roko plan
//!   run` read;
//! - an attempt that changed nothing settles `unverified`, not
//!   `already_satisfied`, since its checks may have tested the person's
//!   checkout instead of its own tree. That costs no retry: a path compiled
//!   into the build is not the agent's to fix.
//!
//! The person editing their own checkout meanwhile reads the same.

use std::collections::{BTreeMap, BTreeSet};
use std::time::SystemTime;

use roko_core::dashboard_snapshot::{DiagnosisSeverity, DiagnosisSummary};
use roko_learn::telemetry::ScopeFinding;
use roko_learn::telemetry::records::SCOPE_CHECKOUT_WRITTEN;

use super::*;

/// The most paths a diagnosis names.
const NAMED_PATHS: usize = 5;

/// `git status`'s code of an untracked path.
const UNTRACKED: &str = "??";

/// One dirty path of the checkout: its `git status` code, and its size and
/// mtime while it exists.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DirtyPath {
    status: String,
    file: Option<(u64, SystemTime)>,
}

/// The dirty paths of a git checkout (bug-b7d9ad).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CheckoutState {
    paths: BTreeMap<String, DirtyPath>,
}

impl CheckoutState {
    /// The dirty paths of the git checkout at `dir`, untracked files
    /// included, leaving out the directories `skipped` names (relative to
    /// `dir`); `None` when git cannot list them.
    pub(super) async fn of(dir: &Path, skipped: Vec<String>) -> Option<Self> {
        let output = tokio::process::Command::new("git")
            .args([
                "--no-optional-locks",
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
            ])
            .current_dir(dir)
            .stdin(std::process::Stdio::null())
            .output()
            .await
            .ok()
            .filter(|output| output.status.success())?;
        let dir = dir.to_path_buf();
        tokio::task::spawn_blocking(move || Self::parse(&dir, &output.stdout, &skipped))
            .await
            .ok()
    }

    /// The state `git status --porcelain=v1 -z` printed for the checkout at
    /// `dir`: `XY path`, each ended by a NUL, a rename or copy followed by
    /// the path it came from. A path under a directory of `skipped` is left
    /// out, and so is a nested checkout, which git lists as its directory.
    fn parse(dir: &Path, porcelain: &[u8], skipped: &[String]) -> Self {
        let mut paths = BTreeMap::new();
        let mut entries = porcelain
            .split(|byte| *byte == 0)
            .filter(|entry| !entry.is_empty());
        while let Some(entry) = entries.next() {
            if matches!(entry.first(), Some(b'R' | b'C')) {
                entries.next();
            }
            let Some(path) = entry.get(3..) else {
                continue;
            };
            let path = String::from_utf8_lossy(path).into_owned();
            let under = |root: &String| {
                path == *root
                    || path
                        .strip_prefix(root.as_str())
                        .is_some_and(|rest| rest.starts_with('/'))
            };
            if path.ends_with('/') || skipped.iter().any(under) {
                continue;
            }
            let file = std::fs::metadata(dir.join(&path))
                .ok()
                .and_then(|meta| Some((meta.len(), meta.modified().ok()?)));
            let status = String::from_utf8_lossy(&entry[..2]).into_owned();
            paths.insert(path, DirtyPath { status, file });
        }
        Self { paths }
    }

    /// What changed since `before`, by kind; `alongside` is left empty.
    pub(super) fn written_since(&self, before: &Self) -> CheckoutWritten {
        let mut written = CheckoutWritten::default();
        let all: BTreeSet<&String> = self.paths.keys().chain(before.paths.keys()).collect();
        for path in all {
            let (then, now) = (before.paths.get(path), self.paths.get(path));
            if then == now {
                continue;
            }
            match (then, now) {
                // The person's own changes were there already.
                (Some(_), _) => written.touched.push(path.clone()),
                (None, Some(now)) if now.status == UNTRACKED => written.added.push(path.clone()),
                (None, _) => written.changed.push(path.clone()),
            }
        }
        written
    }
}

/// The person's checkout as an attempt's checks begin (bug-b7d9ad).
pub(super) struct CheckoutWatch {
    before: CheckoutState,
    /// The directories left out, relative to the checkout.
    skipped: Vec<String>,
    /// From here on, the attempts in flight beside the checks.
    mark: super::sibling_settle::WriterMark,
}

/// What changed in the person's checkout while an attempt's checks ran.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct CheckoutWritten {
    /// Tracked files that were clean before and changed.
    pub(super) changed: Vec<String>,
    /// Files that were not there before and are now, untracked.
    pub(super) added: Vec<String>,
    /// Paths that had changes of the person's own before and changed again,
    /// or went back: dropping them would drop those changes too.
    pub(super) touched: Vec<String>,
    /// The other tasks with an attempt in flight meanwhile, as
    /// `"{plan_id}/{task_id}"`, any of which may have written them instead.
    pub(super) alongside: Vec<String>,
}

impl CheckoutWritten {
    /// Whether nothing changed.
    pub(super) fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.added.is_empty() && self.touched.is_empty()
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
    /// The person's checkout before an attempt of plan `spec` that runs in
    /// `effective_workdir` runs its checks; `None` when the attempt runs in
    /// that checkout itself, or git cannot list it (bug-b7d9ad).
    pub(super) async fn checkout_before_checks(
        &self,
        spec: &TaskExecutionSpec,
        effective_workdir: &Path,
    ) -> Option<CheckoutWatch> {
        if effective_workdir == self.workdir.as_path() {
            return None;
        }
        // What roko itself writes in the checkout during a run.
        let mut skipped = vec![".roko".to_string()];
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
        match CheckoutState::of(&self.workdir, skipped.clone()).await {
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

    /// What changed in the person's checkout since `watch` began, while the
    /// checks of `task_key` (`"{plan_id}/{task_id}"`) ran; nothing without a
    /// `watch`.
    pub(super) async fn checkout_written(
        &self,
        watch: Option<CheckoutWatch>,
        task_key: &str,
    ) -> CheckoutWritten {
        let Some(CheckoutWatch {
            before,
            skipped,
            mark,
        }) = watch
        else {
            return CheckoutWritten::default();
        };
        let mut written = CheckoutState::of(&self.workdir, skipped)
            .await
            .map(|after| after.written_since(&before))
            .unwrap_or_default();
        if !written.is_empty() {
            written.alongside = self.in_flight.sibling_tasks_since(&mark, task_key);
        }
        self.in_flight.release(&mark);
        written
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

    /// Say that the person's checkout changed (`written`) while the checks of
    /// attempt `attempt_number` of `task` ran in `checkout`, and whether the
    /// attempt settles `unverified` for it: in the log, on the hub as a
    /// warning diagnosis, in the attempt's verdict as `checkout_written`
    /// scope findings, and for the end of the plan run (bug-b7d9ad).
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
            alongside = ?written.alongside,
            warm_build_trees = trees.len(),
            unverified,
            "the person's checkout changed while the attempt's checks ran in its own"
        );
        scope_findings.extend(paths.iter().map(|path| ScopeFinding {
            path: path.clone(),
            kind: SCOPE_CHECKOUT_WRITTEN.to_string(),
        }));
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
        subject: format!(
            "The run changed your checkout while {}'s checks ran",
            task.id
        ),
        detail: format!(
            "Task `{}` of plan `{}` runs in its own worktree, but while its checks ran, the run \
             modified your own checkout at `{}` ({}). roko left these changes there and did not \
             deliver them. The likely cause is {cause}. {outcome}{others}",
            task.id,
            spec.plan_id,
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

    fn state(entries: &[(&str, &str, Option<u64>)]) -> CheckoutState {
        let at = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        CheckoutState {
            paths: entries
                .iter()
                .map(|(status, path, size)| {
                    (
                        (*path).to_string(),
                        DirtyPath {
                            status: (*status).to_string(),
                            file: size.map(|size| (size, at)),
                        },
                    )
                })
                .collect(),
        }
    }

    /// bug-b7d9ad: `git status -z` entries become dirty paths, leaving out a
    /// rename's source, roko's own `.roko/`, the plan's directory and a
    /// nested checkout; between two states a clean tracked file that changed
    /// is `changed`, a new untracked one `added`, and one that held changes
    /// already `touched`; a path the same in both is none of them.
    #[test]
    fn a_checkout_state_names_what_changed() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "a").expect("a.txt");
        let porcelain = b" M a.txt\0R  new.txt\0old.txt\0?? .roko/state.json\0\
                          ?? plans/p1/status.json\0?? plans/p10.txt\0?? wt/attempt-1/\0\
                          ?? art/x.png\0";
        let skipped = [".roko".to_string(), "plans/p1".to_string()];
        let parsed = CheckoutState::parse(dir.path(), porcelain, &skipped);
        let paths: Vec<&str> = parsed.paths.keys().map(String::as_str).collect();
        assert_eq!(paths, ["a.txt", "art/x.png", "new.txt", "plans/p10.txt"]);
        assert_eq!(parsed.paths["a.txt"].status, " M");
        assert_eq!(parsed.paths["a.txt"].file.map(|(size, _)| size), Some(1));
        assert_eq!(parsed.paths["art/x.png"].file, None, "no such file");

        let before = state(&[(" M", "kept.txt", Some(3)), (" M", "grown.txt", Some(3))]);
        let after = state(&[
            (" M", "kept.txt", Some(3)),
            (" M", "grown.txt", Some(9)),
            ("??", "artifacts/ui/editor-idle.png", Some(158_253)),
            (" M", "artifacts/ui/editor-playing.png", Some(231_652)),
        ]);
        let written = after.written_since(&before);
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
        assert!(after.written_since(&after).is_empty());
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
    }
}
