//! Handing a failed whole-plan check back to the tasks it names
//! (find-5bcba7).
//!
//! Every task of a plan can pass its own checks while the plan's whole-plan
//! check ([`super::plan_verify`]) fails on their integrated result, such as a
//! clippy lint or a doc example that no task's check runs. Rather than end the
//! plan there, the run hands the failure back: each task that owns a file the
//! failed step's output names gets one more attempt, from the plan's accepted
//! work, with that output as its feedback, and then the plan is checked again.
//!
//! A file belongs to the task that changed it last. Under
//! `--worktree-per-task` that is the task whose accepted commit on the plan
//! branch last changed it; in the shared working tree, the last task in plan
//! order that declares it in its `files`. A file no task of the plan changed,
//! such as one the run's base already had, belongs to none.
//!
//! The hand-backs are bounded: a plan's check is handed back at most
//! [`MAX_HANDBACKS`] times in a run. When the tasks' new attempts pass but
//! change nothing, the check would fail the same way, so it is not run again:
//! the failure goes back to them once more, while hand-backs remain. A
//! failure that names no file a task owns, and a task that fails its own
//! checks on the retry, end the plan as a failed check did before.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Serialize;

use super::batch::PlanWork;
use crate::dispatch::GateFeedback;
use crate::runner::merge::git_output;
use crate::runner::plan_loader::Plan;

/// The most times one plan's failed whole-plan check is handed back to its
/// tasks in a run.
pub const MAX_HANDBACKS: usize = 2;

/// Receipt extension holding the hand-backs ([`Handback`]) that came before a
/// plan's last delivery into its run's batch branch.
pub const HANDBACKS_EXTENSION: &str = "roko.plan_check.handbacks@1";

/// The most bytes of a failed step's output a hand-back keeps to find the
/// files it names; the start of a longer output is dropped.
const MAX_OUTPUT_BYTES: usize = 256 * 1024;

/// The most characters of output the next attempt's feedback quotes.
const MAX_EXCERPT_CHARS: usize = 4_000;

/// Lines of output kept above the first line that names a handed-back file,
/// for the error that line belongs to.
const EXCERPT_LEAD_LINES: usize = 3;

/// A whole-plan check step that failed: its command, and what it wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedPlanCheck {
    /// The step's command.
    pub command: String,
    /// What the step wrote: all of it when it was kept, else its last lines.
    pub output: String,
}

impl FailedPlanCheck {
    /// The failed step `command`, which wrote `output`. Only the last
    /// [`MAX_OUTPUT_BYTES`] of the output are kept.
    #[must_use]
    pub fn new(command: impl Into<String>, output: &str) -> Self {
        Self {
            command: command.into(),
            output: output_tail(output).to_string(),
        }
    }

    /// The failed step of the delivery check log `log` (backlog 3111): the
    /// last command the log holds, `command`, with everything it wrote.
    /// `None` when the log holds no entry for `command`.
    #[must_use]
    pub fn from_check_log(log: &str, command: &str) -> Option<Self> {
        let header = format!("$ {command}\n");
        let start = log.rfind(&header)? + header.len();
        Some(Self::new(command, &log[start..]))
    }
}

/// The end of `text`, at most [`MAX_OUTPUT_BYTES`] long and starting on a
/// character boundary.
fn output_tail(text: &str) -> &str {
    if text.len() <= MAX_OUTPUT_BYTES {
        return text;
    }
    let mut start = text.len() - MAX_OUTPUT_BYTES;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    &text[start..]
}

/// One hand-back of a plan's failed whole-plan check, as the plan's
/// checkpoint and the run's summary record it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Handback {
    /// Which hand-back of the plan it was, from 1.
    pub round: usize,
    /// The failed step's command.
    pub check: String,
    /// The files its output named, each with the task it was handed to.
    pub files: BTreeMap<String, String>,
    /// The tasks it was handed to, in plan order.
    pub tasks: Vec<String>,
    /// How their attempts ended: `passed` (the plan is checked again),
    /// `unchanged` (they passed but changed nothing, so the failure went back
    /// again unchecked), `failed`, `stopped` or `cancelled`.
    pub result: String,
}

/// Which task owns each file a task of `plan` changed, for a hand-back. With
/// the run's repository `repo` (under `--worktree-per-task`), the commits run
/// `run_id` accepted onto the plan's branch decide it; without one, or when
/// the branch holds none, the tasks' declared `files` do
/// ([`declared_owners`]).
pub async fn file_owners(
    repo: Option<&Path>,
    plan: &Plan,
    run_id: &str,
) -> BTreeMap<String, String> {
    if let Some(repo) = repo
        && let Some(work) = PlanWork::read(repo, &plan.id, run_id).await
    {
        let owners = committed_owners(repo, &work).await;
        if !owners.is_empty() {
            return owners;
        }
    }
    declared_owners(plan)
}

/// Each file `work`'s commits changed, with the task of the last commit
/// that changed it. A commit git cannot diff is left out.
async fn committed_owners(repo: &Path, work: &PlanWork) -> BTreeMap<String, String> {
    let mut owners = BTreeMap::new();
    let mut parent = work.base.clone();
    for accepted in &work.commits {
        let args = [
            "diff",
            "--name-only",
            "--no-renames",
            "-z",
            parent.as_str(),
            accepted.commit.as_str(),
        ];
        match git_output(repo, &args).await {
            Ok(changed) => {
                for path in changed.split('\0').filter(|path| !path.is_empty()) {
                    owners.insert(path.to_string(), accepted.task_id.clone());
                }
            }
            Err(error) => tracing::warn!(
                commit = %accepted.commit,
                task_id = %accepted.task_id,
                %error,
                "a hand-back could not tell which files an accepted commit changed"
            ),
        }
        parent.clone_from(&accepted.commit);
    }
    owners
}

/// Each file a task of `plan` declares in its `files`, with the last task
/// in plan order that declares it.
#[must_use]
pub fn declared_owners(plan: &Plan) -> BTreeMap<String, String> {
    let mut owners = BTreeMap::new();
    for task in &plan.tasks.tasks {
        for file in &task.files {
            let file = file.trim().trim_start_matches("./");
            if !file.is_empty() {
                owners.insert(file.to_string(), task.id.clone());
            }
        }
    }
    owners
}

/// The files among `owners` that `output` names, each with its owner. A
/// file is named when its path appears in `output` as a whole path, or as
/// the end of a longer one, such as an absolute path into another checkout.
#[must_use]
pub fn named_files(output: &str, owners: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    owners
        .iter()
        .filter(|(path, _)| names_path(output, path))
        .map(|(path, task)| (path.clone(), task.clone()))
        .collect()
}

/// The tasks `files` were handed to, each once, in `plan`'s order.
#[must_use]
pub fn handed_tasks(plan: &Plan, files: &BTreeMap<String, String>) -> Vec<String> {
    let named: BTreeSet<&str> = files.values().map(String::as_str).collect();
    plan.tasks
        .tasks
        .iter()
        .filter(|task| named.contains(task.id.as_str()))
        .map(|task| task.id.clone())
        .collect()
}

/// Whether `output` names `path`: it appears at the start of a path, or
/// after a `/`, and nothing that continues a path follows it.
fn names_path(output: &str, path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    output.match_indices(path).any(|(start, _)| {
        let before = output[..start].chars().next_back();
        let after = output[start + path.len()..].chars().next();
        before.is_none_or(|c| c == '/' || !continues_path(c))
            && after.is_none_or(|c| !continues_path(c))
    })
}

/// Whether `c` can be part of a path, so that a path next to it is longer.
fn continues_path(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '~' | '+' | '@' | '\\')
}

/// The feedback the next attempt of a task gets when `failure` is handed
/// back to it for `files`: why it runs again, and the step's output from
/// the first line that names one of them. `None` when there is nothing to
/// say.
#[must_use]
pub fn handback_feedback(failure: &FailedPlanCheck, files: &[&str]) -> Option<GateFeedback> {
    let diagnosis = format!(
        "This task passed its own checks and its work was accepted. Then the plan's whole-plan \
         check failed on the plan's integrated work, in file(s) this task changed last: {files}. \
         The failed step is `{command}`; its output follows. This attempt starts from the plan's \
         accepted work, every task's included. Fix the failure in those files, keep this task's \
         own work and checks passing, and change nothing else.",
        files = files.join(", "),
        command = failure.command,
    );
    let raw = format!("$ {}\n{}", failure.command, excerpt(&failure.output, files));
    // The raw output is the feedback: the parsed lists would hide it.
    GateFeedback::from_raw(&raw).map(|feedback| {
        GateFeedback {
            compile_errors: Vec::new(),
            test_failures: Vec::new(),
            clippy_warnings: Vec::new(),
            ..feedback
        }
        .with_diagnosis(&diagnosis)
    })
}

/// The part of `output` the next attempt needs: without cargo's progress
/// lines, from a few lines above the first line that names one of `files`,
/// at most [`MAX_EXCERPT_CHARS`] long.
fn excerpt(output: &str, files: &[&str]) -> String {
    let lines: Vec<&str> = output
        .lines()
        .filter(|line| !is_progress_line(line))
        .collect();
    let first_named = lines
        .iter()
        .position(|line| files.iter().any(|file| names_path(line, file)))
        .unwrap_or(0);
    let mut excerpt = String::new();
    for line in &lines[first_named.saturating_sub(EXCERPT_LEAD_LINES)..] {
        if excerpt.chars().count() + line.chars().count() + 1 > MAX_EXCERPT_CHARS {
            excerpt.push_str("[…]\n");
            break;
        }
        excerpt.push_str(line);
        excerpt.push('\n');
    }
    excerpt
}

/// Whether `line` is one of cargo's progress lines, which say nothing about
/// a failure.
fn is_progress_line(line: &str) -> bool {
    let line = line.trim_start();
    [
        "Compiling ",
        "Checking ",
        "Finished ",
        "Fresh ",
        "Blocking waiting",
        "Downloaded ",
        "Downloading ",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What clippy wrote when the golden path's T7 put `VERSION` after
    /// alpha's test module (find-5bcba7, 2026-10-18 run 1).
    const CLIPPY_ITEMS_AFTER_TESTS: &str = "    Checking alpha v0.1.0 (/scratch/repo/.roko/state/regression-checkout/crates/alpha)
error: items after a test module
  --> crates/alpha/src/lib.rs:56:1
   |
56 | mod tests {
   | ^^^^^^^^^
...
70 | pub const VERSION: &str = env!(\"CARGO_PKG_VERSION\");
   | ^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.96.0/index.html#items_after_test_module
   = note: `-D clippy::items-after-test-module` implied by `-D warnings`
   = help: to override `-D warnings` add `#[allow(clippy::items_after_test_module)]`
   = help: move the items to before the test module was defined

    Checking beta v0.1.0 (/scratch/repo/.roko/state/regression-checkout/crates/beta)
error: could not compile `alpha` (lib test) due to 1 previous error
warning: build failed, waiting for other jobs to finish...
";

    fn owners(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(path, task)| ((*path).to_string(), (*task).to_string()))
            .collect()
    }

    /// find-5bcba7: clippy's error names alpha's `lib.rs`, so it goes to the
    /// task that changed it last; alpha's test file and beta's crate, whose
    /// directory the progress lines name, are not named.
    #[test]
    fn a_clippy_error_names_the_file_it_points_at() {
        let owners = owners(&[
            ("crates/alpha/src/lib.rs", "T7"),
            ("crates/alpha/tests/accept_version.rs", "T7"),
            ("crates/beta/src/lib.rs", "T6"),
            ("py/textutil.py", "T8"),
        ]);

        let named = named_files(CLIPPY_ITEMS_AFTER_TESTS, &owners);

        assert_eq!(named, owners_of(&[("crates/alpha/src/lib.rs", "T7")]));
    }

    fn owners_of(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        owners(pairs)
    }

    /// A path names a file alone, at the end of an absolute path, or before
    /// a line number; never as part of a longer name.
    #[test]
    fn a_path_is_named_only_as_a_whole_path() {
        for (output, named) in [
            ("--> crates/beta/src/lib.rs:6:12", true),
            (
                "Diff in /tmp/x/regression-checkout/crates/beta/src/lib.rs:12:",
                true,
            ),
            ("File \"/tmp/x/py/textutil.py\", line 3, in slugify", true),
            ("at file:///tmp/x/ts/strings.ts:5:3", true),
            (
                "test crates/beta/src/lib.rs - farewell (line 7) ... FAILED",
                true,
            ),
            ("crates/beta/src/lib.rs.orig", false),
            ("xcrates/beta/src/lib.rs", false),
            ("Checking beta v0.1.0 (/tmp/x/crates/beta)", false),
            (
                "     Running unittests src/lib.rs (target/debug/deps/beta-1)",
                false,
            ),
        ] {
            assert_eq!(
                names_path(output, "crates/beta/src/lib.rs")
                    || names_path(output, "py/textutil.py")
                    || names_path(output, "ts/strings.ts"),
                named,
                "{output}"
            );
        }
    }

    /// The shared working tree has no commits per task: a file belongs to
    /// the last task in plan order that declares it.
    #[test]
    fn declared_files_belong_to_the_last_task_that_declares_them() {
        let tasks = crate::task_parser::TasksFile::parse_str(
            "[meta]\nplan = \"demo\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"One\"\ndescription = \"One.\"\n\
             role = \"implementer\"\nstatus = \"ready\"\ntier = \"focused\"\n\
             files = [\"src/lib.rs\", \"a.txt\"]\n\n\
             [[task]]\nid = \"T2\"\ntitle = \"Two\"\ndescription = \"Two.\"\n\
             role = \"implementer\"\nstatus = \"ready\"\ntier = \"focused\"\n\
             depends_on = [\"T1\"]\nfiles = [\"./src/lib.rs\"]\n",
        )
        .expect("the plan parses");
        let plan = Plan {
            id: "demo".to_string(),
            dir: std::path::PathBuf::from("plans/demo"),
            tasks,
        };

        let owners = declared_owners(&plan);

        assert_eq!(owners, owners_of(&[("a.txt", "T1"), ("src/lib.rs", "T2")]));
        let named = named_files("error: src/lib.rs:3:1: broken", &owners);
        assert_eq!(handed_tasks(&plan, &named), ["T2"]);
    }

    /// find-5bcba7: the next attempt's feedback says why it runs, names the
    /// step, and quotes the output from the error that names its file, which
    /// the retry prompt shows as raw output, help lines included.
    #[test]
    fn the_feedback_quotes_the_error_that_names_the_file() {
        let failure = FailedPlanCheck::new(
            "cargo clippy --workspace --all-targets -- -D warnings",
            CLIPPY_ITEMS_AFTER_TESTS,
        );

        let feedback = handback_feedback(&failure, &["crates/alpha/src/lib.rs"]).expect("feedback");

        let diagnosis = feedback.diagnosis.as_deref().expect("a diagnosis");
        assert!(diagnosis.contains("whole-plan check"), "{diagnosis}");
        assert!(diagnosis.contains("crates/alpha/src/lib.rs"), "{diagnosis}");
        assert!(feedback.compile_errors.is_empty(), "{feedback:?}");
        let raw = &feedback.raw_output;
        assert!(
            raw.starts_with("$ cargo clippy --workspace --all-targets -- -D warnings\n"),
            "{raw}"
        );
        assert!(raw.contains("error: items after a test module"), "{raw}");
        assert!(
            raw.contains("move the items to before the test module was defined"),
            "{raw}"
        );
        assert!(!raw.contains("Checking alpha"), "{raw}");
    }

    /// The delivery's check log holds each step it ran; the failed step is
    /// the last, with everything it wrote.
    #[test]
    fn the_failed_step_comes_from_the_check_log() {
        let log = "$ cargo fmt --all -- --check\n[exit 0 after 120 ms]\n\n\
                   $ cargo clippy --workspace -- -D warnings\nerror: boom\n  --> crates/a/src/lib.rs:1:1\n\
                   [exit 101 after 900 ms]\n\n";

        let failed =
            FailedPlanCheck::from_check_log(log, "cargo clippy --workspace -- -D warnings")
                .expect("the failed step");

        assert!(failed.output.starts_with("error: boom\n"), "{failed:?}");
        assert!(failed.output.contains("crates/a/src/lib.rs"), "{failed:?}");
        assert_eq!(FailedPlanCheck::from_check_log(log, "cargo test"), None);
    }
}
