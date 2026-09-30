//! Planner-written acceptance tests, pinned out of the agent's reach
//! (`[task.accept]`, gap-d14a43).
//!
//! A task names the tests its planner wrote and proved red on the tree the
//! task starts from:
//!
//! ```toml
//! [task.accept]
//! files = [
//!     { src = "accept/x.test.ts", dest = "apps/portal/src/x.test.ts", runner = "cd apps/portal && node scripts/vitest-min.mjs {dest} {count}", count = 16 },
//! ]
//! ```
//!
//! `src` is relative to the plan directory and `dest` to the task's working
//! tree. When a run loads its plans, [`pin_plans`] copies every `src` into a
//! store outside any working tree, `~/.roko/accept/<workspace>/<plan>/<task>/<src>`,
//! and puts one verify step per entry in front of the task's own. That step
//! checks the pinned copy against the sha256 compiled into it, copies the
//! pinned copy over `dest`, runs `runner`, and requires exactly `count`
//! passing tests. Whatever the agent did to `src` or `dest`, the pinned test is
//! what runs, and a pinned copy that changed fails the step with a tamper
//! message.
//!
//! The generated steps travel with the task definition, so they run through
//! the ordinary verify path and the checkpoint's graph identity covers their
//! pinned hashes. A source that differs from its pinned copy when a later run
//! loads the plan is rejected rather than re-pinned: the agent may have edited
//! it. Without an OS sandbox the store is tamper-evident, not tamper-proof.

use std::io::Write as _;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::runner::plan_loader::Plan;
use crate::task_parser::{TaskDef, VerifyStep, default_verify_timeout};

/// First line of every generated acceptance step.
const STEP_HEADER: &str = "# roko accept:";

/// Sums the passing tests a runner reported: libtest (`test result: ok. 7
/// passed; …`), vitest and jest (`Tests  16 passed (16)`), pytest (`16 passed
/// in 0.12s`), TAP and `node --test` (`# pass 16`), and unittest (`Ran 16
/// tests` then `OK`). Prints `none` when the output has no such summary.
/// Contains no single quote, so it can sit inside `awk '…'`.
const PASSED_COUNT_AWK: &str = r#"BEGIN { esc = sprintf("%c", 27); total = 0; seen = 0; ran = "" }
{ line = $0; gsub(esc "\\[[0-9;]*[A-Za-z]", "", line); gsub(/\r/, "", line) }
line ~ /^[ \t]*Test (Files|Suites):?[ \t]/ { next }
line ~ /test result: / || line ~ /^[ \t]*Tests:?[ \t]/ || line ~ /[0-9]+ passed.* in [0-9.]+m?s/ {
  n = split(line, w, /[^A-Za-z0-9]+/)
  for (i = 2; i <= n; i++) if (w[i] == "passed" && w[i - 1] ~ /^[0-9]+$/) { total += w[i - 1]; seen = 1 }
  next
}
line ~ /^[^A-Za-z0-9]*pass [0-9]+[ \t]*$/ { n = split(line, w, /[^A-Za-z0-9]+/); total += (w[n] == "" ? w[n - 1] : w[n]); seen = 1; next }
line ~ /^Ran [0-9]+ tests? in / { split(line, w, " "); ran = w[2]; next }
line ~ /^OK( |$)/ && ran != "" {
  skipped = 0
  if (match(line, /skipped=[0-9]+/)) skipped = substr(line, RSTART + 8, RLENGTH - 8)
  total += ran - skipped; seen = 1; ran = ""
}
END { if (seen) print total; else print "none" }"#;

/// `[task.accept]`: the acceptance tests a task's planner wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskAccept {
    /// Pinned tests, run in this order before the task's own verify steps.
    #[serde(default)]
    pub files: Vec<AcceptFile>,
}

/// One planner-written acceptance test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptFile {
    /// The test as the planner wrote it, relative to the plan directory.
    pub src: String,
    /// Where the verify step copies the pinned test, relative to the task's
    /// working tree.
    pub dest: String,
    /// Shell command that runs the test from the working tree root. `{dest}`
    /// expands to the copied test's absolute path and `{count}` to `count`.
    pub runner: String,
    /// Exactly how many passing tests the runner must report.
    pub count: u32,
    /// Timeout of the generated verify step in milliseconds; the verify-step
    /// default when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// A pinned acceptance test: the copy in the store and its sha256.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedAccept {
    /// The copy the generated verify step runs.
    pub stored: PathBuf,
    /// Hex sha256 of the pinned bytes.
    pub sha256: String,
}

/// The directory that holds pinned acceptance tests, outside every working
/// tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptStore {
    root: PathBuf,
}

impl AcceptStore {
    /// A store rooted at `root`.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The user's store, `~/.roko/accept`.
    pub fn user_default() -> Result<Self> {
        let home = dirs::home_dir()
            .context("no home directory for the acceptance-test store ~/.roko/accept")?;
        Ok(Self::at(home.join(".roko").join("accept")))
    }

    /// The store's root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where `task_id`'s pinned tests live: `<root>/<workspace>/<plan>/<task>/`.
    #[must_use]
    pub fn task_dir(&self, workdir: &Path, plan_id: &str, task_id: &str) -> PathBuf {
        self.root
            .join(workspace_key(workdir))
            .join(path_component(plan_id))
            .join(path_component(task_id))
    }

    /// Pin `entry`'s source, read from `plan_dir`, and return the pinned copy.
    ///
    /// The first pin copies the source into the store. Later pins require the
    /// source to be byte-identical to the pinned copy: a test that changed
    /// after it was pinned is rejected, not re-pinned, because the agent may
    /// have edited it.
    pub fn pin(
        &self,
        workdir: &Path,
        plan_id: &str,
        plan_dir: &Path,
        task_id: &str,
        entry: &AcceptFile,
    ) -> Result<PinnedAccept> {
        let src = contained_path(&entry.src).map_err(|problem| {
            anyhow::anyhow!("task {task_id}: accept src `{}` {problem}", entry.src)
        })?;
        let src_path = plan_dir.join(&src);
        let bytes = std::fs::read(&src_path).with_context(|| {
            format!(
                "task {task_id}: read acceptance test {}",
                src_path.display()
            )
        })?;
        let sha256 = sha256_hex(&bytes);
        let task_dir = self.task_dir(workdir, plan_id, task_id);
        let stored = task_dir.join(&src);
        match std::fs::read(&stored) {
            Ok(pinned) => {
                let pinned_sha256 = sha256_hex(&pinned);
                if pinned_sha256 != sha256 {
                    bail!(
                        "task {task_id}: acceptance test {} (sha256 {sha256}) changed after it was \
                         pinned at {} (sha256 {pinned_sha256}); a planner-written test that changes \
                         after pinning is treated as tampering. If the change is intended, remove {} \
                         and run again to re-pin it",
                        src_path.display(),
                        stored.display(),
                        task_dir.display()
                    );
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                write_pinned(&stored, &bytes)?;
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("read pinned acceptance test {}", stored.display()));
            }
        }
        Ok(PinnedAccept { stored, sha256 })
    }
}

/// The verify step that runs one pinned acceptance test.
///
/// It fails with a `TAMPERED` message when the pinned copy no longer has the
/// sha256 compiled into it, when `runner` exits non-zero, and when the runner
/// reports anything but exactly `count` passing tests.
#[must_use]
pub fn pinned_verify_step(task_id: &str, entry: &AcceptFile, pinned: &PinnedAccept) -> VerifyStep {
    let count = entry.count;
    let runner = entry
        .runner
        .replace("{dest}", "\"$roko_dest\"")
        .replace("{count}", &count.to_string());
    let header = format!(
        "{STEP_HEADER} {} {} -> {} (exactly {count} passing tests)",
        printable(task_id),
        printable(&entry.src),
        printable(&entry.dest)
    );
    let lines = [
        header,
        format!("roko_src={}", shell_quote(&entry.src)),
        format!(
            "roko_stored={}",
            shell_quote(&pinned.stored.to_string_lossy())
        ),
        format!("roko_pinned={}", shell_quote(&pinned.sha256)),
        format!("roko_dest=\"$PWD\"/{}", shell_quote(&entry.dest)),
        "roko_actual=$({ sha256sum \"$roko_stored\" || shasum -a 256 \"$roko_stored\"; } 2>/dev/null | cut -d' ' -f1)".to_string(),
        "if [ \"$roko_actual\" != \"$roko_pinned\" ]; then".to_string(),
        "  echo \"roko accept: TAMPERED: the pinned copy of $roko_src at $roko_stored has sha256 ${roko_actual:-<missing>}, not the $roko_pinned pinned when the plan loaded; it was not run\" >&2".to_string(),
        "  exit 1".to_string(),
        "fi".to_string(),
        "mkdir -p \"$(dirname \"$roko_dest\")\" && cp -f \"$roko_stored\" \"$roko_dest\" || exit 1".to_string(),
        "roko_log=$(mktemp) || exit 1".to_string(),
        format!("{{ {runner}"),
        "} 2>&1 | tee \"$roko_log\"".to_string(),
        "roko_rc=$?".to_string(),
        format!("roko_passed=$(awk '{PASSED_COUNT_AWK}' \"$roko_log\")"),
        "rm -f \"$roko_log\"".to_string(),
        "if [ \"$roko_rc\" -ne 0 ]; then".to_string(),
        "  echo \"roko accept: $roko_src failed: its runner exited with status $roko_rc\" >&2"
            .to_string(),
        "  exit 1".to_string(),
        "fi".to_string(),
        format!("if [ \"$roko_passed\" != \"{count}\" ]; then"),
        format!(
            "  echo \"roko accept: $roko_src must pass exactly {count} tests; the runner reported $roko_passed\" >&2"
        ),
        "  exit 1".to_string(),
        "fi".to_string(),
    ];
    VerifyStep {
        phase: "test".to_string(),
        command: lines.join("\n"),
        fail_msg: Some(format!(
            "the pinned acceptance test {} must pass exactly {count} tests",
            entry.src
        )),
        timeout_ms: entry.timeout_ms.unwrap_or_else(default_verify_timeout),
    }
}

/// Whether `step` was generated by [`pinned_verify_step`].
#[must_use]
pub fn is_pinned_step(step: &VerifyStep) -> bool {
    is_pinned_command(&step.command)
}

/// Whether `command` is a generated acceptance step: it starts with the step's header line.
#[must_use]
pub fn is_pinned_command(command: &str) -> bool {
    command.starts_with(STEP_HEADER)
}

/// A verify command as a prompt shows it (gap-1b5636). A generated acceptance step shows only its
/// header line, `# roko accept: <task> <src> -> <dest> (exactly <count> passing tests)`; the rest
/// is the harness's hash check and copy. Any other command shows in full.
#[must_use]
pub fn prompt_command(command: &str) -> &str {
    if is_pinned_command(command) {
        command.lines().next().unwrap_or(command)
    } else {
        command
    }
}

/// Pin `task`'s acceptance tests and put the verify steps that run them in
/// front of its own. Returns how many steps the task gained; a task already
/// pinned gains none.
pub fn pin_task(
    store: &AcceptStore,
    workdir: &Path,
    plan_id: &str,
    plan_dir: &Path,
    task: &mut TaskDef,
) -> Result<usize> {
    let Some(accept) = task
        .accept
        .as_ref()
        .filter(|accept| !accept.files.is_empty())
    else {
        return Ok(0);
    };
    if task.verify.first().is_some_and(is_pinned_step) {
        return Ok(0);
    }
    let mut steps = Vec::with_capacity(accept.files.len());
    for entry in &accept.files {
        let problems = entry_problems(entry);
        if !problems.is_empty() {
            bail!(
                "task {}: [task.accept] entry `{}`: {}",
                task.id,
                entry.src,
                problems.join("; ")
            );
        }
        let pinned = store.pin(workdir, plan_id, plan_dir, &task.id, entry)?;
        steps.push(pinned_verify_step(&task.id, entry, &pinned));
    }
    let added = steps.len();
    steps.append(&mut task.verify);
    task.verify = steps;
    Ok(added)
}

/// Pin the acceptance tests of every task in `plans`, for a run from
/// `workdir`, in the user's store. Fails before anything is dispatched when a
/// test is missing or malformed, or changed after it was pinned.
pub fn pin_plans(plans: &mut [Plan], workdir: &Path) -> Result<()> {
    let any_accept = plans
        .iter()
        .any(|plan| plan.tasks.tasks.iter().any(TaskDef::has_accept_tests));
    if !any_accept {
        return Ok(());
    }
    pin_plans_in(&AcceptStore::user_default()?, plans, workdir)
}

/// [`pin_plans`] with an explicit store.
pub fn pin_plans_in(store: &AcceptStore, plans: &mut [Plan], workdir: &Path) -> Result<()> {
    for plan in plans {
        let plan_dir = if plan.dir.is_dir() {
            plan.dir.clone()
        } else {
            workdir.join(&plan.dir)
        };
        let mut pinned = 0;
        for task in &mut plan.tasks.tasks {
            pinned += pin_task(store, workdir, &plan.id, &plan_dir, task)?;
        }
        if pinned > 0 {
            tracing::info!(
                plan_id = %plan.id,
                pinned,
                store = %store.root().display(),
                "pinned planner-written acceptance tests; their verify steps run first"
            );
        }
    }
    Ok(())
}

/// A `plan validate` finding about a task's acceptance tests (PLAN_038).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptIssue {
    /// An error (the plan would fail to load for a run) rather than a warning.
    pub blocking: bool,
    /// What is wrong.
    pub message: String,
}

/// Problems with `task`'s `[task.accept]` entries, whose sources resolve
/// against `plan_dir`, and verify steps that still copy a test out of
/// `accept/` by hand.
#[must_use]
pub fn accept_issues(task: &TaskDef, plan_dir: &Path) -> Vec<AcceptIssue> {
    let mut issues = Vec::new();
    for entry in task.accept.iter().flat_map(|accept| &accept.files) {
        for problem in entry_problems(entry) {
            issues.push(AcceptIssue {
                blocking: true,
                message: format!("task '{}' [task.accept] {problem}", task.id),
            });
        }
        if let Ok(src) = contained_path(&entry.src)
            && !plan_dir.join(&src).is_file()
        {
            issues.push(AcceptIssue {
                blocking: true,
                message: format!(
                    "task '{}' [task.accept] src `{}` is missing from {}",
                    task.id,
                    entry.src,
                    plan_dir.display()
                ),
            });
        }
    }
    for (index, step) in task.verify.iter().enumerate() {
        if copies_accept_test_by_hand(&step.command) {
            issues.push(AcceptIssue {
                blocking: false,
                message: format!(
                    "task '{}' verify step {} copies a test out of accept/ by hand; declare it \
                     in [task.accept] so the harness pins it outside the agent's reach and \
                     checks its passing count",
                    task.id,
                    index + 1
                ),
            });
        }
    }
    issues
}

/// What is wrong with `entry` before its source is read.
fn entry_problems(entry: &AcceptFile) -> Vec<String> {
    let mut problems = Vec::new();
    if let Err(problem) = contained_path(&entry.src) {
        problems.push(format!("src `{}` {problem}", entry.src));
    }
    if let Err(problem) = contained_path(&entry.dest) {
        problems.push(format!("dest `{}` {problem}", entry.dest));
    }
    if entry.runner.trim().is_empty() {
        problems.push(format!("runner of `{}` is empty", entry.src));
    }
    if entry.count == 0 {
        problems.push(format!(
            "count of `{}` is 0; a pinned test must pass at least one test",
            entry.src
        ));
    }
    problems
}

/// Whether `command` copies a file out of an `accept/` directory with `cp`,
/// the hand convention `[task.accept]` replaces.
fn copies_accept_test_by_hand(command: &str) -> bool {
    command.split(['&', '|', ';', '\n']).any(|segment| {
        let mut words = segment.split_whitespace();
        words.next() == Some("cp")
            && words.any(|word| word.starts_with("accept/") || word.contains("/accept/"))
    })
}

/// `raw` as a relative path that stays inside its base directory.
fn contained_path(raw: &str) -> std::result::Result<PathBuf, &'static str> {
    if raw.trim().is_empty() {
        return Err("is empty");
    }
    if raw.chars().any(char::is_control) {
        return Err("contains a control character");
    }
    let mut path = PathBuf::new();
    for component in Path::new(raw).components() {
        match component {
            Component::Normal(part) => path.push(part),
            Component::CurDir => {}
            Component::ParentDir => return Err("leaves its directory through `..`"),
            Component::RootDir | Component::Prefix(_) => return Err("is absolute"),
        }
    }
    if path.as_os_str().is_empty() {
        return Err("names no file");
    }
    Ok(path)
}

/// Write `bytes` to `stored` through a temporary sibling, so a crash never
/// leaves a partial pin.
fn write_pinned(stored: &Path, bytes: &[u8]) -> Result<()> {
    let parent = stored
        .parent()
        .context("pinned acceptance test path has no parent")?;
    std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("create a temporary file in {}", parent.display()))?;
    tmp.write_all(bytes)
        .with_context(|| format!("write the pin for {}", stored.display()))?;
    tmp.persist(stored)
        .map_err(|error| error.error)
        .with_context(|| format!("pin acceptance test {}", stored.display()))?;
    Ok(())
}

/// `<dir name>-<12 hex digits of the sha256 of its canonical path>`: one
/// store namespace per checkout.
fn workspace_key(workdir: &Path) -> String {
    let canonical = workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf());
    let digest = sha256_hex(canonical.to_string_lossy().as_bytes());
    let name = canonical.file_name().map_or_else(
        || "workspace".to_string(),
        |name| path_component(&name.to_string_lossy()),
    );
    format!("{name}-{}", &digest[..12])
}

/// `raw` as one path component: characters other than ASCII letters, digits,
/// `.`, `-` and `_` become `_`, and `.` or `..` becomes `_`.
fn path_component(raw: &str) -> String {
    let component: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    match component.as_str() {
        "" | "." | ".." => "_".to_string(),
        _ => component,
    }
}

/// `raw` with control characters replaced, for the step's comment line.
fn printable(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

/// `raw` as one single-quoted shell word.
fn shell_quote(raw: &str) -> String {
    format!("'{}'", raw.replace('\'', "'\\''"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_parser::TasksFile;

    /// The pinned test "reports" its own passing count: the runner prints the
    /// file the step copied over `dest`, so the step passes only when that is
    /// the pinned copy.
    const PINNED_TEST: &str = "test result: ok. 2 passed; 0 failed\n";

    const PLAN: &str = r#"
[meta]
plan = "accept-demo"
total = 1

[[task]]
id = "T01"
title = "Make the pinned test pass"
description = "Make the planner-written acceptance test pass."
role = "implementer"
files = ["src/x.test.txt"]

[task.accept]
files = [
    { src = "accept/x.test.txt", dest = "src/x.test.txt", runner = "cat {dest}", count = 2 },
]

[[task.verify]]
phase = "compile"
command = "true"
"#;

    struct Fixture {
        _dir: tempfile::TempDir,
        workdir: PathBuf,
        plan_dir: PathBuf,
        store: AcceptStore,
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().expect("tempdir");
        let workdir = dir.path().join("ws");
        let plan_dir = workdir.join("plans").join("accept-demo");
        std::fs::create_dir_all(plan_dir.join("accept")).expect("create plan dir");
        std::fs::write(plan_dir.join("tasks.toml"), PLAN).expect("write plan");
        std::fs::write(plan_dir.join("accept").join("x.test.txt"), PINNED_TEST)
            .expect("write accept test");
        let store = AcceptStore::at(dir.path().join("store"));
        Fixture {
            _dir: dir,
            workdir,
            plan_dir,
            store,
        }
    }

    fn demo_task() -> TaskDef {
        TasksFile::parse_str(PLAN)
            .expect("parse plan")
            .tasks
            .remove(0)
    }

    /// Run `step` the way Graph verification does: `bash -o pipefail -c`
    /// in the task's working tree.
    fn run_step(step: &VerifyStep, workdir: &Path) -> (bool, String) {
        let output = std::process::Command::new("bash")
            .args(["-o", "pipefail", "-c", &step.command])
            .current_dir(workdir)
            .output()
            .expect("run bash");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status.success(), text)
    }

    #[test]
    fn accept_table_compiles_to_a_pinned_verify_step() {
        let fx = fixture();
        let mut task = demo_task();
        assert!(task.has_accept_tests());
        assert_eq!(
            task.accept.as_ref().map(|accept| accept.files.clone()),
            Some(vec![AcceptFile {
                src: "accept/x.test.txt".to_string(),
                dest: "src/x.test.txt".to_string(),
                runner: "cat {dest}".to_string(),
                count: 2,
                timeout_ms: None,
            }])
        );

        let added = pin_task(
            &fx.store,
            &fx.workdir,
            "accept-demo",
            &fx.plan_dir,
            &mut task,
        )
        .expect("pin");
        assert_eq!(added, 1);
        assert_eq!(
            task.verify.len(),
            2,
            "the pinned step runs before the authored one"
        );
        let step = task.verify[0].clone();
        assert!(is_pinned_step(&step));
        assert_eq!(step.phase, "test");
        assert_eq!(task.verify[1].command, "true");

        // The step is pinned: it names the stored copy and its hash, never the
        // agent-writable source.
        let stored = fx
            .store
            .task_dir(&fx.workdir, "accept-demo", "T01")
            .join("accept")
            .join("x.test.txt");
        assert!(stored.starts_with(fx.store.root()));
        assert!(
            !stored.starts_with(&fx.workdir),
            "the store is outside the working tree"
        );
        assert_eq!(
            std::fs::read_to_string(&stored).expect("read pin"),
            PINNED_TEST
        );
        assert!(step.command.contains(&sha256_hex(PINNED_TEST.as_bytes())));
        assert!(step.command.contains(&*stored.to_string_lossy()));
        assert!(!copies_accept_test_by_hand(&step.command));
        assert!(!PASSED_COUNT_AWK.contains('\''));

        // Pinning again adds nothing, and the steps survive the JSON round
        // trip the Graph engine makes without being compiled twice.
        assert_eq!(
            pin_task(
                &fx.store,
                &fx.workdir,
                "accept-demo",
                &fx.plan_dir,
                &mut task
            )
            .expect("re-pin"),
            0
        );
        let json = serde_json::to_string(&task).expect("serialize");
        let decoded: TaskDef = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.verify.len(), 2);
        assert_eq!(decoded.verify[0].command, step.command);
        assert_eq!(decoded.accept, task.accept);

        // The agent rewrote both its copy and the plan's source: the pinned
        // test is what runs, and it lands on `dest`.
        std::fs::create_dir_all(fx.workdir.join("src")).expect("create src");
        std::fs::write(
            fx.workdir.join("src").join("x.test.txt"),
            "test result: ok. 9 passed; 0 failed\n",
        )
        .expect("agent edits dest");
        std::fs::write(fx.plan_dir.join("accept").join("x.test.txt"), "gutted\n")
            .expect("agent edits src");
        let (passed, output) = run_step(&step, &fx.workdir);
        assert!(
            passed,
            "the pinned test passes with exactly 2 tests:\n{output}"
        );
        assert_eq!(
            std::fs::read_to_string(fx.workdir.join("src").join("x.test.txt")).expect("read dest"),
            PINNED_TEST
        );

        // Any other passing count fails the step.
        let entry = &task.accept.as_ref().expect("accept").files[0];
        let pinned = PinnedAccept {
            stored,
            sha256: sha256_hex(PINNED_TEST.as_bytes()),
        };
        let three = AcceptFile {
            count: 3,
            ..entry.clone()
        };
        let (passed, output) = run_step(&pinned_verify_step("T01", &three, &pinned), &fx.workdir);
        assert!(!passed);
        assert!(
            output.contains("must pass exactly 3 tests; the runner reported 2"),
            "{output}"
        );
    }

    #[test]
    fn accept_store_rejects_a_changed_source() {
        let fx = fixture();
        let mut task = demo_task();
        pin_task(
            &fx.store,
            &fx.workdir,
            "accept-demo",
            &fx.plan_dir,
            &mut task,
        )
        .expect("pin");
        let step = task.verify[0].clone();
        let stored = fx
            .store
            .task_dir(&fx.workdir, "accept-demo", "T01")
            .join("accept")
            .join("x.test.txt");

        // A changed pinned copy fails the step with a tamper message, and the
        // changed test never runs.
        std::fs::write(&stored, "test result: ok. 2 passed; 0 failed\n# weakened\n")
            .expect("tamper with the pin");
        let (passed, output) = run_step(&step, &fx.workdir);
        assert!(!passed);
        assert!(output.contains("TAMPERED"), "{output}");
        assert!(!fx.workdir.join("src").join("x.test.txt").exists());

        // So does a pinned copy that disappeared.
        std::fs::remove_file(&stored).expect("remove the pin");
        let (passed, output) = run_step(&step, &fx.workdir);
        assert!(!passed);
        assert!(output.contains("TAMPERED"), "{output}");

        // A later run that finds the source changed since it was pinned
        // refuses to re-pin it.
        std::fs::write(&stored, PINNED_TEST).expect("restore the pin");
        std::fs::write(fx.plan_dir.join("accept").join("x.test.txt"), "gutted\n")
            .expect("edit the source");
        let error = pin_task(
            &fx.store,
            &fx.workdir,
            "accept-demo",
            &fx.plan_dir,
            &mut demo_task(),
        )
        .expect_err("a changed source is rejected");
        let message = format!("{error:#}");
        assert!(message.contains("changed after it was pinned"), "{message}");
        assert!(message.contains("tampering"), "{message}");
        assert_eq!(
            std::fs::read_to_string(&stored).expect("read pin"),
            PINNED_TEST,
            "the pin is kept"
        );

        // A missing source is an error, not an empty pin.
        std::fs::remove_file(fx.plan_dir.join("accept").join("x.test.txt"))
            .expect("remove the source");
        let other = AcceptStore::at(fx.store.root().join("fresh"));
        assert!(
            pin_task(
                &other,
                &fx.workdir,
                "accept-demo",
                &fx.plan_dir,
                &mut demo_task()
            )
            .is_err()
        );
    }

    #[test]
    fn accept_issues_flag_bad_entries_and_hand_copies() {
        let fx = fixture();
        let mut task = demo_task();
        task.accept = Some(TaskAccept {
            files: vec![
                AcceptFile {
                    src: "accept/missing.test.txt".to_string(),
                    dest: "src/missing.test.txt".to_string(),
                    runner: "cat {dest}".to_string(),
                    count: 0,
                    timeout_ms: None,
                },
                AcceptFile {
                    src: "../outside.txt".to_string(),
                    dest: "/etc/passwd".to_string(),
                    runner: " ".to_string(),
                    count: 1,
                    timeout_ms: None,
                },
            ],
        });
        task.verify.push(VerifyStep {
            phase: "test".to_string(),
            command:
                "cd apps/portal && cp ../../plans/p/accept/x.test.ts src/x.test.ts && npm test"
                    .to_string(),
            fail_msg: None,
            timeout_ms: 1_000,
        });
        let issues = accept_issues(&task, &fx.plan_dir);
        let errors: Vec<&str> = issues
            .iter()
            .filter(|issue| issue.blocking)
            .map(|issue| issue.message.as_str())
            .collect();
        assert!(
            errors
                .iter()
                .any(|m| m.contains("count of `accept/missing.test.txt` is 0"))
        );
        assert!(
            errors
                .iter()
                .any(|m| m.contains("src `accept/missing.test.txt` is missing"))
        );
        assert!(
            errors
                .iter()
                .any(|m| m.contains("src `../outside.txt` leaves its directory"))
        );
        assert!(
            errors
                .iter()
                .any(|m| m.contains("dest `/etc/passwd` is absolute"))
        );
        assert!(
            errors
                .iter()
                .any(|m| m.contains("runner of `../outside.txt` is empty"))
        );
        let warnings: Vec<&AcceptIssue> = issues.iter().filter(|issue| !issue.blocking).collect();
        assert_eq!(warnings.len(), 1, "{issues:?}");
        assert!(
            warnings[0]
                .message
                .contains("verify step 2 copies a test out of accept/")
        );

        // The fixture's own entry is clean.
        assert!(accept_issues(&demo_task(), &fx.plan_dir).is_empty());
        // A malformed entry never pins.
        assert!(
            pin_task(
                &fx.store,
                &fx.workdir,
                "accept-demo",
                &fx.plan_dir,
                &mut task
            )
            .is_err()
        );
    }

    #[test]
    fn passed_count_reads_common_runner_summaries() {
        let cases = [
            (
                "test result: ok. 5 passed; 0 failed\ntest result: ok. 2 passed; 0 failed\n",
                "7",
            ),
            (
                " Test Files  1 passed (1)\n      Tests  16 passed (16)\n",
                "16",
            ),
            (
                "Test Suites: 1 passed, 1 total\nTests:       15 passed, 15 total\n",
                "15",
            ),
            (
                "=========== 16 passed, 2 warnings in 0.31s ===========\n",
                "16",
            ),
            ("# tests 4\n# pass 4\n# fail 0\n", "4"),
            ("Ran 4 tests in 0.001s\n\nOK (skipped=1)\n", "3"),
            ("all good\n", "none"),
        ];
        let dir = tempfile::tempdir().expect("tempdir");
        for (output, expected) in cases {
            let log = dir.path().join("log");
            std::fs::write(&log, output).expect("write log");
            let counted = std::process::Command::new("awk")
                .arg(PASSED_COUNT_AWK)
                .arg(&log)
                .output()
                .expect("run awk");
            assert_eq!(
                String::from_utf8_lossy(&counted.stdout).trim(),
                expected,
                "{output:?}"
            );
        }
    }
}
