//! Handing a failed whole-plan check back to the tasks it names
//! (find-5bcba7).
//!
//! Every task of a plan can pass its own checks while the plan's whole-plan
//! check ([`super::plan_verify`]) fails on their integrated result, such as a
//! clippy lint or a doc example that no task's check runs. Rather than end the
//! plan there, the run hands the failure back: one more attempt, from the
//! plan's accepted work, with the failed step's output as its feedback, and
//! then the plan is checked again.
//!
//! A file belongs to the task that changed it last. Under
//! `--worktree-per-task` that is the task whose accepted commit on the plan
//! branch last changed it; in the shared working tree, the last task in plan
//! order that declares it in its `files`. A file no task of the plan changed,
//! such as one the run's base already had, belongs to none.
//!
//! The failure goes back to the tasks that own the files its output names,
//! or, when it names none, to every task that changed a file. Fixing it can
//! take more than one task's files, as in Undertow's phase 4 (bug-fc05a3: two
//! tasks each drew a colour the check forbids, and fixing either alone still
//! failed), so the hand-back is one attempt, not one per task. One of them
//! runs it ([`lead_task`]): a task whose own steps run the test that failed,
//! else the one of the highest tier. It may change all their files, and its
//! own verify steps, theirs and the plan's check judge it
//! ([`handback_task`]). An attempt that changes nothing while the check fails
//! is a failure, never "already satisfied".
//!
//! The hand-backs are bounded: a plan's check is handed back at most
//! [`MAX_HANDBACKS`] times in a run, and each try is retried at most
//! [`HANDBACK_RETRIES`] times, and not at all once it changed nothing. A try
//! that does not pass, and a failed step that printed nothing to go on, end
//! the plan as a failed check did before.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Serialize;

use roko_learn::failure_text::{failure_excerpt, is_failure_line, is_progress_line};

use super::batch::PlanWork;
use crate::dispatch::GateFeedback;
use crate::runner::merge::git_output;
use crate::runner::plan_loader::Plan;
use crate::task_parser::{TaskDef, VerifyStep};

/// The most times one plan's failed whole-plan check is handed back to its
/// tasks in a run.
pub const MAX_HANDBACKS: usize = 2;

/// The most retries a hand-back's try gets, within its task's own budget;
/// none once it changed nothing, since the next would start from the same
/// tree (bug-fc05a3).
pub const HANDBACK_RETRIES: u32 = 1;

/// Receipt extension holding the hand-backs ([`Handback`]) that came before a
/// plan's last delivery into its run's batch branch.
pub const HANDBACKS_EXTENSION: &str = "roko.plan_check.handbacks@1";

/// The most bytes of a failed step's output a hand-back keeps to find the
/// files it names; the start of a longer output is dropped.
const MAX_OUTPUT_BYTES: usize = 256 * 1024;

/// The most characters of output the next attempt's feedback quotes.
const MAX_EXCERPT_CHARS: usize = 4_000;

/// Lines of output kept above the first line that names a handed-back file,
/// where an excerpt with no failure line starts.
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

    /// Whether the step printed nothing a hand-back's try could go on: no
    /// line but blank ones and the check log's closing `[exit 1 after 9
    /// ms]`.
    #[must_use]
    pub fn printed_nothing(&self) -> bool {
        self.output.lines().map(str::trim).all(|line| {
            line.is_empty()
                || (line.starts_with('[') && line.ends_with(" ms]") && line.contains(" after "))
        })
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
    /// The files its output named, each with the task that changed it last.
    pub files: BTreeMap<String, String>,
    /// The tasks whose files its try could change, in plan order: the owners
    /// of `files`, or every task that changed a file when it named none.
    pub tasks: Vec<String>,
    /// The task whose attempt was the try ([`lead_task`]).
    pub lead: String,
    /// How the try ended: `passed` (the plan is checked again), `unchanged`
    /// (it changed nothing, and the check still failed on its tree),
    /// `failed`, `stopped` or `cancelled`.
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

/// The tasks a failed check goes back to, in `plan`'s order: the owners of
/// the files its output names, `named`, or, when it names none, every task
/// that owns a file of `owners`, since nothing in the output points at one of
/// them (bug-fc05a3).
#[must_use]
pub fn implicated_tasks(
    plan: &Plan,
    owners: &BTreeMap<String, String>,
    named: &BTreeMap<String, String>,
) -> Vec<String> {
    handed_tasks(plan, if named.is_empty() { owners } else { named })
}

/// The task of `tasks` whose attempt is a hand-back's one try for `failure`
/// (bug-fc05a3). A task whose own verify steps run a test that failed leads
/// ([`runs_failing_test`]), since its task says what that test wants: phase
/// 4's T6 ran P4.06 (`tools/spec.sh 4 display 'P4.06*,P4.08*,P4.09*'`).
/// Else, and among several, the one of the highest tier leads, the last in
/// `plan`'s order among equals, since the try may have to change every one of
/// their files.
#[must_use]
pub fn lead_task(plan: &Plan, tasks: &[String], failure: &FailedPlanCheck) -> Option<String> {
    let failing = failing_words(&failure.output);
    let handed: Vec<&TaskDef> = plan
        .tasks
        .tasks
        .iter()
        .filter(|task| tasks.contains(&task.id))
        .collect();
    let running: Vec<&TaskDef> = handed
        .iter()
        .copied()
        .filter(|task| runs_failing_test(task, &failing))
        .collect();
    let pool = if running.is_empty() { handed } else { running };
    pool.into_iter()
        .max_by_key(|task| task.tier_class())
        .map(|task| task.id.clone())
}

/// The words of `output`'s failure: of its lines from the first that says
/// something failed to the last ([`is_failure_line`]), split at anything
/// but letters, digits and `_.-`, without a closing `.`. Empty when no line
/// says something failed.
fn failing_words(output: &str) -> BTreeSet<String> {
    let lines: Vec<&str> = output.lines().collect();
    let (Some(first), Some(last)) = (
        lines.iter().position(|line| is_failure_line(line)),
        lines.iter().rposition(|line| is_failure_line(line)),
    ) else {
        return BTreeSet::new();
    };
    lines[first..=last]
        .iter()
        .flat_map(|line| {
            line.split(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '.' | '-')))
        })
        .map(|word| word.trim_end_matches('.'))
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

/// Whether one of `task`'s own verify steps runs a test named in `failing`
/// ([`failing_words`]): a word of its command after the program, split at
/// `/`, `:`, `=`, commas and quotes, with a digit, `_`, `.` or `*` in it as a
/// test's name has (`P4.06*`, `a_try_passes`, `test_text.py`), equals a
/// failing word or, with `*` or `?` in it, matches one. Plain words
/// (`display`, `--lib`) select too much to say whose test failed.
fn runs_failing_test(task: &TaskDef, failing: &BTreeSet<String>) -> bool {
    task.verify.iter().any(|step| {
        step.command
            .split_whitespace()
            .skip(1)
            .flat_map(|word| word.split(['/', ':', '=', ',', '\'', '"']))
            .filter(|word| !word.starts_with('-'))
            .filter(|word| word.chars().any(char::is_alphabetic))
            .filter(|word| {
                word.contains(|c: char| c.is_ascii_digit() || matches!(c, '_' | '.' | '*'))
            })
            .any(|selector| {
                if selector.contains(['*', '?']) {
                    glob::Pattern::new(selector)
                        .is_ok_and(|pattern| failing.iter().any(|word| pattern.matches(word)))
                } else {
                    failing.contains(selector)
                }
            })
    })
}

/// Task `lead` as a hand-back's try runs it (bug-fc05a3). Its description
/// leads with why it runs again and which files it may change, `scope`, each
/// with the task that changed it last, and those files join its own. Its own
/// verify steps judge it, then those of `others`, the other tasks whose files
/// it may change, then the plan's whole-plan check `checks`, each command
/// once, so an attempt that changes nothing while the check fails fails. A
/// borrowed step keeps what it runs: what it covers and expects belongs to
/// its task.
#[must_use]
pub fn handback_task(
    lead: &TaskDef,
    others: &[&TaskDef],
    checks: &[VerifyStep],
    failure: &FailedPlanCheck,
    scope: &BTreeMap<String, String>,
) -> TaskDef {
    let mut task = lead.clone();
    for path in scope.keys() {
        if !task
            .files
            .iter()
            .any(|file| file.trim().trim_start_matches("./") == path)
        {
            task.files.push(path.clone());
        }
    }
    let borrowed: Vec<VerifyStep> = others
        .iter()
        .flat_map(|other| &other.verify)
        .chain(checks)
        .map(|step| VerifyStep {
            covers: Vec::new(),
            expect: None,
            ..step.clone()
        })
        .collect();
    for step in borrowed {
        let command = step.command.trim();
        if !task.verify.iter().any(|own| own.command.trim() == command) {
            task.verify.push(step);
        }
    }
    let files = scope
        .iter()
        .map(|(path, owner)| format!("{path} ({owner})"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut description = format!(
        "This attempt is a hand-back of the plan's whole-plan check. Every task of the plan \
         passed its own checks, and their work was accepted, but the plan's check fails on \
         their integrated work: its step `{command}` failed. The attempt starts from that work, \
         every task's included, and passes only when the plan's check passes on its tree, \
         together with the checks of the tasks whose files it may change. It may change these \
         files, each named with the task that changed it last: {files}. Find what makes the \
         check fail, fix it there, keep every task's work, and change nothing else.",
        command = failure.command,
    );
    if let Some(own) = lead
        .description
        .as_deref()
        .filter(|own| !own.trim().is_empty())
    {
        description.push_str("\n\nThe task's own description: ");
        description.push_str(own);
    }
    task.description = Some(description);
    task
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

/// The feedback a hand-back's try gets for `failure` (bug-fc05a3): why it
/// runs again, the files the output names, `named`, each with the task that
/// changed it last, the tasks whose files it may change, `tasks`, and the
/// step's output from where it fails. `None` when there is nothing to say.
#[must_use]
pub fn handback_feedback(
    failure: &FailedPlanCheck,
    named: &BTreeMap<String, String>,
    tasks: &[String],
) -> Option<GateFeedback> {
    let names = if named.is_empty() {
        "It names no file a task of the plan changed.".to_string()
    } else {
        let files = named
            .iter()
            .map(|(path, owner)| format!("{path} ({owner})"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("It names {files}.")
    };
    let diagnosis = format!(
        "Every task of the plan passed its own checks and its work was accepted. Then the \
         plan's whole-plan check failed on the plan's integrated work: its step `{command}` \
         failed, and its output follows, from where it fails. {names} This attempt starts from \
         the plan's accepted work, every task's included. It may change the files of task(s) \
         {tasks}, which the task's description lists, and passes only when the plan's check \
         passes on its tree.",
        command = failure.command,
        tasks = tasks.join(", "),
    );
    let files: Vec<&str> = named.keys().map(String::as_str).collect();
    let raw = format!(
        "$ {}\n{}",
        failure.command,
        excerpt(&failure.output, &files)
    );
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

/// The part of `output` the next attempt needs, at most about
/// [`MAX_EXCERPT_CHARS`] long: failure first
/// ([`roko_learn::failure_text::failure_excerpt`]), each failing test case
/// with its first failure and why (bug-3e3d08), else from a few lines above
/// the first line that says something failed. A step's errors can follow
/// lines that name a file only to warn about it, as its standard error,
/// written after its test output, did in Undertow's phase 4 (bug-fc05a3).
/// Without a line that says something failed, it runs from the first line
/// that names one of `files`, build progress left out.
fn excerpt(output: &str, files: &[&str]) -> String {
    if let Some(excerpt) = failure_excerpt(output, MAX_EXCERPT_CHARS) {
        return excerpt;
    }
    let lines: Vec<&str> = output
        .lines()
        .filter(|line| !is_progress_line(line))
        .collect();
    let start = lines
        .iter()
        .position(|line| files.iter().any(|file| names_path(line, file)))
        .unwrap_or(0);
    let mut excerpt = String::new();
    for line in &lines[start.saturating_sub(EXCERPT_LEAD_LINES)..] {
        if excerpt.chars().count() + line.chars().count() + 1 > MAX_EXCERPT_CHARS {
            excerpt.push_str("[…]\n");
            break;
        }
        excerpt.push_str(line);
        excerpt.push('\n');
    }
    excerpt
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
        let named = owners_of(&[("crates/alpha/src/lib.rs", "T7")]);

        let feedback = handback_feedback(&failure, &named, &["T7".to_string()]).expect("feedback");

        let diagnosis = feedback.diagnosis.as_deref().expect("a diagnosis");
        assert!(diagnosis.contains("whole-plan check"), "{diagnosis}");
        assert!(
            diagnosis.contains("crates/alpha/src/lib.rs (T7)"),
            "{diagnosis}"
        );
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

    /// What `tools/check.sh --plugin` wrote when Undertow's phase 4 failed
    /// P4.06 (run c4fa953f), cut short: the test output, then the build's
    /// standard error, whose warnings name the tasks' files.
    const PHASE_4_CHECK: &str = "[ 15%] Building CXX object plugin/CMakeFiles/Undertow.dir/ui/Look.cpp.o
100% tests passed, 0 tests failed out of 22
      Start 23: spec_p4_budget
23/28 Test #23: spec_p4_budget ...................   Passed    9.80 sec
      Start 24: spec_p4_display
24/28 Test #24: spec_p4_display ..................***Failed    2.14 sec
TEST CASE:  P4.06 the one number: the pitch in bone while a note plays, in one place in the CHEST
/x/regression-checkout/tests/spec/p4/display_test.cpp:57: ERROR: CHECK( idle.bone <= idle.pixels / 10000 ) is NOT correct!
  logged: before any note the panel shows 1145 bone pixels; bone appears only when there is a pitch to show
The following tests FAILED:
	 24 - spec_p4_display (Failed)
/x/regression-checkout/plugin/ui/Look.cpp:89:48: warning: comparing floating point with == or != is unsafe [-Wfloat-equal]
   89 |     if (cachedLayer_.isValid() && cachedScale_ == scale && cachedSize_ == size)
1 warning generated.
Errors while running CTest
";

    /// bug-fc05a3: the excerpt leads with the failure, not with the first
    /// line that names a handed-back file, which was a warning written after
    /// it: phase 4's T5 got only warnings and never saw P4.06. Since
    /// bug-3e3d08 it keeps the failing doctest case whole, and the warnings
    /// after it drop out.
    #[test]
    fn the_excerpt_leads_with_the_failure_not_a_warning() {
        let excerpt = excerpt(PHASE_4_CHECK, &["plugin/ui/Look.cpp"]);

        assert!(
            excerpt.starts_with("24/28 Test #24: spec_p4_display"),
            "{excerpt}"
        );
        assert!(
            excerpt.contains("TEST CASE:  P4.06 the one number"),
            "{excerpt}"
        );
        assert!(excerpt.contains("1145 bone pixels"), "{excerpt}");
        assert!(!excerpt.contains("warning"), "{excerpt}");
        // Without a line that says something failed, the first line that
        // names a file leads, as before.
        let quiet = "building\nlinking\nsrc/a.rs:3: unused\nlast line\n";
        assert_eq!(
            excerpt_lines(quiet, &["src/a.rs"]),
            ["building", "linking", "src/a.rs:3: unused", "last line"]
        );
        for (line, failed) in [
            ("error: could not compile `alpha`", true),
            ("error[E0425]: cannot find value `x`", true),
            ("src/a.cpp:3:1: error: expected ';'", true),
            ("src/a.cpp:1:10: fatal error: 'b.h' file not found", true),
            ("x_test.cpp:57: ERROR: CHECK( a ) is NOT correct!", true),
            ("test result: FAILED. 1 passed; 1 failed", true),
            (
                "24/28 Test #24: spec_p4_display ...***Failed    2.14 sec",
                true,
            ),
            ("FAIL: test_slug (tests.test_text.TextTest)", true),
            ("thread 't' panicked at src/lib.rs:3:5:", true),
            ("AssertionError: 1 != 2", true),
            ("100% tests passed, 0 tests failed out of 22", false),
            (
                "warning: build failed, waiting for other jobs to finish...",
                false,
            ),
            ("x.cpp:9:17: warning: unused variable 'kPi'", false),
            ("test result: ok. 5 passed; 0 failed", false),
        ] {
            assert_eq!(is_failure_line(line), failed, "{line}");
        }
    }

    /// The lines of [`excerpt`]`(output, files)`.
    fn excerpt_lines(output: &str, files: &[&str]) -> Vec<String> {
        excerpt(output, files).lines().map(str::to_string).collect()
    }

    /// A plan of tasks, each `(id, tier, files, its verify step's command)`,
    /// in that order.
    fn plan_of(tasks: &[(&str, &str, &[&str], &str)]) -> Plan {
        let toml = tasks
            .iter()
            .map(|(id, tier, files, command)| {
                format!(
                    "[[task]]\nid = \"{id}\"\ntitle = \"Task {id}\"\ndescription = \"Do {id}.\"\n\
                     role = \"implementer\"\nstatus = \"ready\"\ntier = \"{tier}\"\nfiles = {files:?}\n\n\
                     [[task.verify]]\nphase = \"test\"\ncommand = {command:?}\ncovers = [\"AC1\"]\n"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let tasks =
            crate::task_parser::TasksFile::parse_str(&format!("[meta]\nplan = \"demo\"\n\n{toml}"))
                .expect("the plan parses");
        Plan {
            id: "demo".to_string(),
            dir: std::path::PathBuf::from("plans/demo"),
            tasks,
        }
    }

    /// bug-fc05a3: a failure goes back to the owners of the files it names,
    /// else to every task that changed a file. One try runs it: a task whose
    /// own steps run the test that failed, P4.06 as phase 4's T6 did, else
    /// the task of the highest tier, the last in plan order among equals.
    #[test]
    fn the_task_that_runs_the_failing_test_leads_the_hand_back() {
        let plan = plan_of(&[
            (
                "T1",
                "integrative",
                &["plugin/PluginProcessor.cpp"],
                "tools/spec.sh 4 scope",
            ),
            (
                "T2",
                "architectural",
                &["plugin/PluginEditor.cpp"],
                "tools/spec.sh 4 editor 'P4.03*'",
            ),
            (
                "T5",
                "integrative",
                &["plugin/ui/Look.cpp"],
                "tools/spec.sh 4 display 'P4.05*'",
            ),
            (
                "T6",
                "integrative",
                &["plugin/ui/Displays.cpp"],
                "tools/spec.sh 4 display 'P4.06*,P4.08*,P4.09*'",
            ),
            (
                "T7",
                "architectural",
                &["plugin/ui/Status.cpp"],
                "tools/spec.sh 4 display 'P4.07*'",
            ),
        ]);
        let owners = declared_owners(&plan);
        let failure = FailedPlanCheck::new("tools/check.sh --plugin", PHASE_4_CHECK);

        // The warning names T5's file alone.
        let named = named_files(&failure.output, &owners);
        assert_eq!(named, owners_of(&[("plugin/ui/Look.cpp", "T5")]));
        assert_eq!(implicated_tasks(&plan, &owners, &named), ["T5"]);
        // P4.06's failure names no task's file: every task may be at fault,
        // and T6, whose step runs P4.06, leads, above T7's tier.
        let silent = FailedPlanCheck::new(
            "tools/check.sh --plugin",
            &PHASE_4_CHECK.replace("/x/regression-checkout/plugin/ui/Look.cpp", "Look.cpp"),
        );
        let named = named_files(&silent.output, &owners);
        let tasks = implicated_tasks(&plan, &owners, &named);
        assert_eq!(tasks, ["T1", "T2", "T5", "T6", "T7"]);
        assert_eq!(lead_task(&plan, &tasks, &silent).as_deref(), Some("T6"));
        // Without a task that runs it, the highest tier leads, the last
        // among equals.
        let elsewhere = FailedPlanCheck::new("tools/check.sh", "error: the build broke\n");
        assert_eq!(lead_task(&plan, &tasks, &elsewhere).as_deref(), Some("T7"));
        let tasks = ["T1".to_string(), "T5".to_string()];
        assert_eq!(lead_task(&plan, &tasks, &elsewhere).as_deref(), Some("T5"));
        assert_eq!(lead_task(&plan, &[], &silent), None);
        // A plain word selects too much; a test's name, or a pattern of one,
        // selects it.
        let words = failing_words(PHASE_4_CHECK);
        for (command, runs) in [
            ("tools/spec.sh 4 display", false),
            ("tools/spec.sh 4 display 'P4.06*'", true),
            ("cd build && ctest -R spec_p4_display", true),
            ("ctest -R spec_p4_budget", false),
            ("cargo test -p roko-cli --lib -- --nocapture", false),
        ] {
            let mut task = plan.tasks.tasks[0].clone();
            task.verify[0].command = command.to_string();
            assert_eq!(runs_failing_test(&task, &words), runs, "{command}");
        }
    }

    /// bug-fc05a3: the try may change every file of the tasks it was handed
    /// for, and their verify steps and the plan's check judge it, each
    /// command once; what a borrowed step covers is its own task's.
    #[test]
    fn the_try_borrows_the_files_and_checks_it_needs() {
        let plan = plan_of(&[
            ("T5", "integrative", &["look.cpp"], "test -f T5.txt"),
            ("T6", "architectural", &["displays.cpp"], "test -f T6.txt"),
            ("T7", "integrative", &["status.cpp"], "test -f T7.txt"),
        ]);
        let task = |id: &str| {
            plan.tasks
                .tasks
                .iter()
                .find(|task| task.id == id)
                .expect("the task")
        };
        let step = |command: &str, timeout_ms| VerifyStep {
            phase: "test".to_string(),
            command: command.to_string(),
            fail_msg: None,
            timeout_ms,
            scope: Vec::new(),
            covers: Vec::new(),
            expect: None,
        };
        // A step a task runs already runs once.
        let checks = [
            step("tools/check.sh --plugin", 900_000),
            step("test -f T5.txt", 1_000),
        ];
        let failure = FailedPlanCheck::new("tools/check.sh --plugin", PHASE_4_CHECK);
        let scope = owners_of(&[
            ("displays.cpp", "T6"),
            ("look.cpp", "T5"),
            ("status.cpp", "T7"),
        ]);

        let tried = handback_task(
            task("T6"),
            &[task("T5"), task("T7")],
            &checks,
            &failure,
            &scope,
        );

        assert_eq!(tried.files, ["displays.cpp", "look.cpp", "status.cpp"]);
        let commands: Vec<&str> = tried
            .verify
            .iter()
            .map(|step| step.command.as_str())
            .collect();
        assert_eq!(
            commands,
            [
                "test -f T6.txt",
                "test -f T5.txt",
                "test -f T7.txt",
                "tools/check.sh --plugin"
            ]
        );
        assert_eq!(
            tried.verify[0].covers,
            ["AC1"],
            "its own step keeps what it covers"
        );
        assert!(tried.verify[1].covers.is_empty(), "{:?}", tried.verify[1]);
        assert_eq!(tried.verify[3].timeout_ms, 900_000);
        let description = tried.description.as_deref().expect("a description");
        assert!(
            description.contains("hand-back of the plan's whole-plan check"),
            "{description}"
        );
        assert!(
            description.contains("`tools/check.sh --plugin`"),
            "{description}"
        );
        assert!(
            description.contains("displays.cpp (T6), look.cpp (T5), status.cpp (T7)"),
            "{description}"
        );
        assert!(
            description.ends_with("The task's own description: Do T6."),
            "{description}"
        );
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
        assert!(!failed.printed_nothing(), "{failed:?}");
        assert_eq!(FailedPlanCheck::from_check_log(log, "cargo test"), None);
        // A step that wrote nothing leaves a hand-back nothing to go on.
        let silent = FailedPlanCheck::from_check_log(
            "$ test -f together.txt\n[exit 1 after 3 ms]\n\n",
            "test -f together.txt",
        )
        .expect("the failed step");
        assert!(silent.printed_nothing(), "{silent:?}");
        assert!(FailedPlanCheck::new("exit 1", "").printed_nothing());
    }
}
