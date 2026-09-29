//! Sibling-aware settlement of failed verify steps.
//!
//! Tasks that run side by side in one working tree see each other's
//! half-written edits, so a whole-project check (`tsc --noEmit`,
//! `cargo check`) run by one task can fail on a file a sibling is still
//! editing. When a verify step fails while such siblings are mid-attempt,
//! the dispatcher waits, bounded by `[gates] sibling_settle_secs`, for them
//! to finish their current attempt and re-runs the step once; only that
//! result counts. A failure that persists with every located error in a
//! sibling's `files` names it: `blocked_by_sibling = <task>`.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use regex::Regex;
use roko_core::Verdict;
use tokio::sync::watch;

/// Task attempts in flight, so a failed verify step can tell whether a
/// sibling editing the same working tree may have caused it.
pub(crate) struct InFlightTasks {
    attempts: parking_lot::Mutex<BTreeMap<u64, InFlightAttempt>>,
    /// Attempts that have ended, by id, so the siblings that overlapped a
    /// task's attempts can still be named afterwards
    /// ([`Self::sibling_files_since`]).
    ended: parking_lot::Mutex<Vec<(u64, InFlightAttempt)>>,
    next_id: AtomicU64,
    /// Bumped whenever an attempt ends or starts settling.
    changed: watch::Sender<u64>,
}

/// Where a task's window of edits to its working tree starts: the attempts
/// in flight then, and the id the next attempt to register gets. The
/// siblings that overlap the window are those attempts and every later one.
#[derive(Debug, Clone, Default)]
pub(crate) struct WriterMark {
    next_id: u64,
    in_flight: BTreeSet<u64>,
}

struct InFlightAttempt {
    /// `"{plan_id}/{task_id}"`.
    key: String,
    workdir: PathBuf,
    files: Vec<String>,
    /// Waiting for its own siblings to settle: it writes nothing meanwhile,
    /// and waiting on it could deadlock.
    settling: bool,
}

/// A sibling attempt that may be editing the working tree.
#[derive(Debug, Clone)]
struct SiblingWriter {
    id: u64,
    /// `"{plan_id}/{task_id}"`.
    key: String,
    files: Vec<String>,
}

/// One registered attempt; dropping it ends the attempt.
pub(crate) struct InFlightGuard<'a> {
    tasks: &'a InFlightTasks,
    id: u64,
}

/// The verify step that failed, and how long it may wait for siblings.
pub(crate) struct FailedStep<'a> {
    pub plan_id: &'a str,
    pub task_id: &'a str,
    /// The task's own `files`.
    pub files: &'a [String],
    /// Step label for logs, such as `verify[0:typecheck]`.
    pub label: &'a str,
    pub workdir: &'a Path,
    /// `[gates] sibling_settle_secs`; zero keeps every failure as is.
    pub settle_limit: Duration,
}

impl Default for InFlightTasks {
    fn default() -> Self {
        Self {
            attempts: parking_lot::Mutex::default(),
            ended: parking_lot::Mutex::default(),
            next_id: AtomicU64::new(0),
            changed: watch::channel(0).0,
        }
    }
}

impl Drop for InFlightGuard<'_> {
    fn drop(&mut self) {
        let ended = self.tasks.attempts.lock().remove(&self.id);
        if let Some(attempt) = ended {
            self.tasks.ended.lock().push((self.id, attempt));
        }
        self.tasks.bump();
    }
}

impl InFlightTasks {
    /// Record an attempt of `key` (`"{plan_id}/{task_id}"`) that may edit
    /// `files` in `workdir` until the returned guard drops.
    pub(crate) fn register(
        &self,
        key: &str,
        workdir: &Path,
        files: &[String],
    ) -> InFlightGuard<'_> {
        // The id is taken under the lock, so a [`WriterMark`] never sees an
        // id handed out whose attempt is not in the map yet.
        let mut attempts = self.attempts.lock();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        attempts.insert(
            id,
            InFlightAttempt {
                key: key.to_string(),
                workdir: workdir.to_path_buf(),
                files: files.to_vec(),
                settling: false,
            },
        );
        InFlightGuard { tasks: self, id }
    }

    /// Start a window from which [`Self::sibling_files_since`] names the
    /// sibling attempts that may edit a task's working tree.
    pub(crate) fn mark(&self) -> WriterMark {
        let attempts = self.attempts.lock();
        WriterMark {
            next_id: self.next_id.load(Ordering::Relaxed),
            in_flight: attempts.keys().copied().collect(),
        }
    }

    /// The `files` declared by attempts of tasks other than `key` in
    /// `workdir` that were in flight at `mark` or started since, ended or
    /// not: what they edited there is theirs, not `key`'s.
    pub(crate) fn sibling_files_since(
        &self,
        mark: &WriterMark,
        key: &str,
        workdir: &Path,
    ) -> Vec<String> {
        let overlaps = |id: u64, attempt: &InFlightAttempt| {
            attempt.key != key
                && attempt.workdir == workdir
                && (id >= mark.next_id || mark.in_flight.contains(&id))
        };
        let mut files = BTreeSet::new();
        for (id, attempt) in self.attempts.lock().iter() {
            if overlaps(*id, attempt) {
                files.extend(attempt.files.iter().cloned());
            }
        }
        for (id, attempt) in self.ended.lock().iter() {
            if overlaps(*id, attempt) {
                files.extend(attempt.files.iter().cloned());
            }
        }
        files.into_iter().collect()
    }

    /// Settle a verify step that failed while siblings may be editing its
    /// working tree.
    ///
    /// Without such siblings, or with a zero limit, the failure stands and
    /// nothing waits. Otherwise the failure is deferred until the siblings
    /// finish their current attempt (bounded by the limit), `rerun` runs the
    /// step once, and its verdict is the step's result. A re-run that still
    /// fails with every located error in siblings' files also returns their
    /// task ids, the `blocked_by_sibling` of the failure.
    pub(crate) async fn settle_failed_step<Fut>(
        &self,
        step: &FailedStep<'_>,
        failed: Verdict,
        rerun: impl FnOnce() -> Fut,
    ) -> (Verdict, Option<String>)
    where
        Fut: Future<Output = Verdict>,
    {
        if step.settle_limit.is_zero() {
            return (failed, None);
        }
        let key = format!("{}/{}", step.plan_id, step.task_id);
        let writers = self.begin_settle(&key, step.workdir);
        if writers.is_empty() {
            return (failed, None);
        }
        let siblings = writers
            .iter()
            .map(|writer| sibling_label(step.plan_id, &writer.key))
            .collect::<Vec<_>>()
            .join(", ");
        tracing::warn!(
            plan_id = step.plan_id,
            task_id = step.task_id,
            step = step.label,
            siblings = %siblings,
            blocked_by_sibling = blame(step, &failed, &writers)
                .as_ref()
                .map_or("none", |blame| blame.siblings.as_str()),
            settle_secs = step.settle_limit.as_secs(),
            "verify step failed while sibling tasks edit the working tree; \
             deferring the failure until they settle"
        );
        let settled = self.wait_settled(&writers, step.settle_limit).await;
        self.end_settle(&key);
        if !settled {
            tracing::warn!(
                plan_id = step.plan_id,
                task_id = step.task_id,
                step = step.label,
                siblings = %siblings,
                "sibling tasks did not settle within [gates] sibling_settle_secs; \
                 re-running the verify step anyway"
            );
        }

        let mut verdict = rerun().await;
        if verdict.passed {
            tracing::info!(
                plan_id = step.plan_id,
                task_id = step.task_id,
                step = step.label,
                siblings = %siblings,
                "verify step passed once sibling tasks settled; the deferred failure \
                 came from their in-progress edits"
            );
            return (verdict, None);
        }
        let blame = blame(step, &verdict, &writers);
        tracing::warn!(
            plan_id = step.plan_id,
            task_id = step.task_id,
            step = step.label,
            siblings = %siblings,
            blocked_by_sibling = blame.as_ref().map_or("none", |blame| blame.siblings.as_str()),
            "verify step still fails after sibling tasks settled"
        );
        let Some(blame) = blame else {
            return (verdict, None);
        };
        let note = format!(
            "blocked_by_sibling = {}: every located error, first at {}, \
             is in a file it declares",
            blame.siblings, blame.first
        );
        verdict.detail = Some(match verdict.detail.take() {
            Some(detail) => format!("{detail}\n{note}"),
            None => note,
        });
        (verdict, Some(blame.siblings))
    }

    /// Siblings that may be editing `workdir` beside `key`: other attempts
    /// there that declare files and are not settling themselves. When any
    /// exist, `key` starts settling under the same lock, so two failing
    /// tasks never wait on each other.
    fn begin_settle(&self, key: &str, workdir: &Path) -> Vec<SiblingWriter> {
        let mut attempts = self.attempts.lock();
        let writers: Vec<SiblingWriter> = attempts
            .iter()
            .filter(|(_, attempt)| {
                attempt.key != key
                    && attempt.workdir == workdir
                    && !attempt.files.is_empty()
                    && !attempt.settling
            })
            .map(|(id, attempt)| SiblingWriter {
                id: *id,
                key: attempt.key.clone(),
                files: attempt.files.clone(),
            })
            .collect();
        if !writers.is_empty() {
            for attempt in attempts.values_mut().filter(|attempt| attempt.key == key) {
                attempt.settling = true;
            }
            drop(attempts);
            self.bump();
        }
        writers
    }

    fn end_settle(&self, key: &str) {
        for attempt in self
            .attempts
            .lock()
            .values_mut()
            .filter(|attempt| attempt.key == key)
        {
            attempt.settling = false;
        }
    }

    /// Wait up to `limit` until every writer has finished its attempt or is
    /// settling itself. Returns whether they did in time.
    async fn wait_settled(&self, writers: &[SiblingWriter], limit: Duration) -> bool {
        let mut changed = self.changed.subscribe();
        tokio::time::timeout(limit, async {
            while !self.settled(writers) {
                if changed.changed().await.is_err() {
                    break;
                }
            }
        })
        .await
        .is_ok()
    }

    fn settled(&self, writers: &[SiblingWriter]) -> bool {
        let attempts = self.attempts.lock();
        writers.iter().all(|writer| {
            attempts
                .get(&writer.id)
                .is_none_or(|attempt| attempt.settling)
        })
    }

    fn bump(&self) {
        self.changed
            .send_modify(|generation| *generation = generation.wrapping_add(1));
    }
}

/// Siblings blamed for a failure: whose files hold every located error.
struct Blame {
    siblings: String,
    first: ErrorLocation,
}

/// Blame for `verdict`'s failure when every error located in its output is
/// in a sibling's files and none is in the step's own.
fn blame(step: &FailedStep<'_>, verdict: &Verdict, writers: &[SiblingWriter]) -> Option<Blame> {
    let output = verdict.detail.as_deref().unwrap_or(&verdict.reason);
    let locations = parse_error_locations(output);
    let owners = blocking_siblings(&locations, step.workdir, step.files, writers);
    if owners.is_empty() {
        return None;
    }
    let siblings = owners
        .iter()
        .map(|key| sibling_label(step.plan_id, key))
        .collect::<Vec<_>>()
        .join(", ");
    let first = locations.into_iter().next()?;
    Some(Blame { siblings, first })
}

/// A sibling's task id, qualified by its plan when that differs.
fn sibling_label<'k>(plan_id: &str, key: &'k str) -> &'k str {
    key.strip_prefix(plan_id)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(key)
}

/// Keys of the siblings whose files hold every located error. Empty when
/// nothing was located, or when any error lies in the task's own files or
/// in no sibling's files.
fn blocking_siblings<'w>(
    locations: &[ErrorLocation],
    workdir: &Path,
    own_files: &[String],
    writers: &'w [SiblingWriter],
) -> Vec<&'w str> {
    let mut owners: Vec<&str> = Vec::new();
    for location in locations {
        let path = workspace_path(workdir, &location.path);
        if own_files.iter().any(|file| declares(file, &path)) {
            return Vec::new();
        }
        let mut owned = false;
        for writer in writers
            .iter()
            .filter(|writer| writer.files.iter().any(|file| declares(file, &path)))
        {
            owned = true;
            if !owners.contains(&writer.key.as_str()) {
                owners.push(&writer.key);
            }
        }
        if !owned {
            return Vec::new();
        }
    }
    owners
}

/// Where a compiler, linter, or test runner reported an error.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ErrorLocation {
    path: String,
    line: u32,
    column: u32,
}

impl std::fmt::Display for ErrorLocation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:{}:{}", self.path, self.line, self.column)
    }
}

/// Error locations in verify output: tsc `path(line,col): error`,
/// rustc/cargo `--> path:line:col` under an `error` (not a `warning`)
/// header, eslint's default file-then-`line:col error` blocks, and the
/// `path:line:col` of eslint, vitest, and panics.
fn parse_error_locations(output: &str) -> Vec<ErrorLocation> {
    static ANSI: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("valid ANSI regex"));
    static TSC: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(\S+?)\((\d+),(\d+)\): error\b").expect("valid tsc regex")
    });
    static RUSTC_HEADER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(error|warning)(\[\w+\])?:").expect("valid rustc header regex")
    });
    static RUSTC_SPAN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*--> (.+?):(\d+):(\d+)\s*$").expect("valid rustc span regex")
    });
    static STYLISH_FILE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^([^\s:]+\.[A-Za-z]\w*)$").expect("valid eslint file regex"));
    static STYLISH_ERROR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s+(\d+):(\d+)\s+error\s").expect("valid eslint entry regex")
    });
    static PATH_LINE_COL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w./@~+-])([\w./@~+-]*\.[A-Za-z]\w*):(\d+):(\d+)")
            .expect("valid path:line:col regex")
    });

    let output = ANSI.replace_all(output, "");
    let mut locations = Vec::new();
    let mut rustc_warning = false;
    let mut stylish_file: Option<&str> = None;
    for line in output.lines() {
        if let Some(header) = RUSTC_HEADER.captures(line) {
            rustc_warning = &header[1] == "warning";
        } else if let Some(span) = RUSTC_SPAN.captures(line) {
            if !rustc_warning {
                push_location(&mut locations, &span[1], &span[2], &span[3]);
            }
        } else if line.trim_start().starts_with(":::") {
            // rustc's secondary spans point at related code, not the error.
        } else if let Some(tsc) = TSC.captures(line) {
            push_location(&mut locations, &tsc[1], &tsc[2], &tsc[3]);
        } else if let Some((file, entry)) = stylish_file.zip(STYLISH_ERROR.captures(line)) {
            push_location(&mut locations, file, &entry[1], &entry[2]);
        } else if let Some(file) = STYLISH_FILE.captures(line).and_then(|file| file.get(1)) {
            stylish_file = Some(file.as_str());
        } else {
            for found in PATH_LINE_COL.captures_iter(line) {
                push_location(&mut locations, &found[1], &found[2], &found[3]);
            }
        }
    }
    locations
}

fn push_location(locations: &mut Vec<ErrorLocation>, path: &str, line: &str, column: &str) {
    let (Ok(line), Ok(column)) = (line.parse(), column.parse()) else {
        return;
    };
    let location = ErrorLocation {
        path: path.to_string(),
        line,
        column,
    };
    if !locations.contains(&location) {
        locations.push(location);
    }
}

/// `raw` relative to the workspace root when inside it, with `.` and `..`
/// resolved lexically.
fn workspace_path(workdir: &Path, raw: &str) -> PathBuf {
    let raw = Path::new(raw.strip_prefix("file://").unwrap_or(raw));
    lexical(raw.strip_prefix(workdir).unwrap_or(raw))
}

fn lexical(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    normalized
}

/// Whether the task `files` entry `declared` covers `located`: the same file
/// or a directory holding it, or either path a suffix of the other, since a
/// tool run in a subproject (`cd web && tsc`) reports paths relative to it.
pub(super) fn declares(declared: &str, located: &Path) -> bool {
    let declared = lexical(Path::new(declared.trim()));
    !declared.as_os_str().is_empty()
        && !located.as_os_str().is_empty()
        && (located.starts_with(&declared)
            || located.ends_with(&declared)
            || declared.ends_with(located))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use std::time::Instant;

    const WORKDIR: &str = "/repo";
    const PLAN_VIEW_ERROR: &str = "src/components/stage/PlanView.tsx(448,24): error TS2304: \
                                   Cannot find name 'formatDuration'.";

    fn location(path: &str, line: u32, column: u32) -> ErrorLocation {
        ErrorLocation {
            path: path.to_string(),
            line,
            column,
        }
    }

    fn files(paths: &[&str]) -> Vec<String> {
        paths.iter().map(ToString::to_string).collect()
    }

    fn writer(id: u64, key: &str, paths: &[&str]) -> SiblingWriter {
        SiblingWriter {
            id,
            key: key.to_string(),
            files: files(paths),
        }
    }

    fn failed_step<'a>(task_id: &'a str, files: &'a [String], limit: Duration) -> FailedStep<'a> {
        FailedStep {
            plan_id: "plan",
            task_id,
            files,
            label: "verify[0:typecheck]",
            workdir: Path::new(WORKDIR),
            settle_limit: limit,
        }
    }

    fn failing(output: &str) -> Verdict {
        Verdict::fail("verify[0:typecheck]", "exit code: 2").with_detail(output)
    }

    #[test]
    fn parses_tsc_error_locations() {
        let output = format!(
            "{PLAN_VIEW_ERROR}\n\
             app/(dashboard)/page.tsx(3,1): error TS1005: ';' expected.\n\
             \n\
             Found 2 errors in 2 files.\n\
             \n\
             Errors  Files\n     \
             1  src/components/stage/PlanView.tsx:448\n"
        );

        assert_eq!(
            parse_error_locations(&output),
            vec![
                location("src/components/stage/PlanView.tsx", 448, 24),
                location("app/(dashboard)/page.tsx", 3, 1),
            ]
        );
    }

    #[test]
    fn parses_rustc_error_spans_but_not_warnings_or_secondary_spans() {
        let output = "warning: unused import: `std::fmt`\n \
                      --> crates/roko-cli/src/own.rs:3:5\n  \
                      |\n\
                      error[E0425]: cannot find function `format_duration` in this scope\n  \
                      --> crates/roko-cli/src/sibling.rs:12:9\n   \
                      |\n\
                      12 |         format_duration(elapsed)\n   \
                      |         ^^^^^^^^^^^^^^^ not found in this scope\n  \
                      ::: crates/roko-core/src/time.rs:40:1\n\
                      \x1b[1m\x1b[31merror\x1b[0m: aborting due to 1 previous error\n";

        assert_eq!(
            parse_error_locations(output),
            vec![location("crates/roko-cli/src/sibling.rs", 12, 9)]
        );
    }

    #[test]
    fn parses_eslint_and_vitest_locations() {
        let output = "/repo/web/src/a.ts:3:7: 'unused' is assigned but never used [Error/no-unused-vars]\n\
                      /repo/web/src/b.ts\n  \
                      12:5  error    'x' is not defined     no-undef\n  \
                      13:1  warning  Unexpected console     no-console\n\
                      \x20FAIL  src/utils/format.test.ts > formatDuration > formats minutes\n\
                      \x20\u{276f} src/utils/format.test.ts:14:22\n\
                      \x20  Duration  12:34:56\n";

        assert_eq!(
            parse_error_locations(output),
            vec![
                location("/repo/web/src/a.ts", 3, 7),
                location("/repo/web/src/b.ts", 12, 5),
                location("src/utils/format.test.ts", 14, 22),
            ]
        );
    }

    #[test]
    fn attributes_a_failure_only_when_every_error_is_in_a_sibling_file() {
        let workdir = Path::new(WORKDIR);
        let own = files(&["web/src/components/Header.tsx"]);
        let writers = [
            writer(1, "plan/T12", &["web/src/components/stage/PlanView.tsx"]),
            writer(2, "other/T3", &["crates/roko-cli/src/"]),
        ];
        // tsc run from `web/` reports paths relative to it.
        let in_t12 = location("src/components/stage/PlanView.tsx", 448, 24);
        assert_eq!(
            blocking_siblings(std::slice::from_ref(&in_t12), workdir, &own, &writers),
            vec!["plan/T12"]
        );
        // Absolute paths and directory entries resolve too.
        let in_both = [
            location("/repo/web/src/components/stage/PlanView.tsx", 1, 1),
            location("crates/roko-cli/src/lib.rs", 2, 2),
        ];
        assert_eq!(
            blocking_siblings(&in_both, workdir, &own, &writers),
            vec!["plan/T12", "other/T3"]
        );
        // One error in the task's own files, or in no sibling's, keeps the
        // failure the task's own.
        let with_own = [in_t12.clone(), location("src/components/Header.tsx", 3, 1)];
        assert!(blocking_siblings(&with_own, workdir, &own, &writers).is_empty());
        let unowned = [in_t12, location("web/src/main.tsx", 1, 1)];
        assert!(blocking_siblings(&unowned, workdir, &own, &writers).is_empty());
        assert!(blocking_siblings(&[], workdir, &own, &writers).is_empty());
        // Siblings in the same plan are named by task id alone.
        assert_eq!(sibling_label("plan", "plan/T12"), "T12");
        assert_eq!(sibling_label("plan", "other/T3"), "other/T3");
    }

    #[tokio::test]
    async fn a_failure_without_writers_beside_it_stands_without_waiting() {
        let tasks = InFlightTasks::default();
        let own = files(&["web/src/a.ts"]);
        let _own = tasks.register("plan/T08", Path::new(WORKDIR), &own);
        // An isolated worktree, or a task declaring no files, writes nothing here.
        let _isolated = tasks.register("plan/T12", Path::new("/repo/.roko/worktrees/T12"), &own);
        let _reader = tasks.register("plan/T13", Path::new(WORKDIR), &[]);
        let reruns = AtomicUsize::new(0);
        let step = failed_step("T08", &own, Duration::from_secs(60));

        let (verdict, _) = tokio::time::timeout(
            Duration::from_secs(5),
            tasks.settle_failed_step(&step, failing(PLAN_VIEW_ERROR), || async {
                reruns.fetch_add(1, Ordering::SeqCst);
                Verdict::pass("verify[0:typecheck]")
            }),
        )
        .await
        .expect("no wait without writers");

        assert!(!verdict.passed);
        assert_eq!(reruns.load(Ordering::SeqCst), 0);

        // A zero limit disables settling even beside a writer.
        let _writer = tasks.register("plan/T14", Path::new(WORKDIR), &files(&["web/src/b.ts"]));
        let disabled = failed_step("T08", &own, Duration::ZERO);
        let (verdict, _) = tasks
            .settle_failed_step(&disabled, failing(PLAN_VIEW_ERROR), || async {
                reruns.fetch_add(1, Ordering::SeqCst);
                Verdict::pass("verify[0:typecheck]")
            })
            .await;
        assert!(!verdict.passed);
        assert_eq!(reruns.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_failure_beside_a_writer_waits_for_it_then_reruns_once() {
        let tasks = InFlightTasks::default();
        let own = files(&["web/src/components/Header.tsx"]);
        let _own = tasks.register("plan/T08", Path::new(WORKDIR), &own);
        let sibling = tasks.register(
            "plan/T12",
            Path::new(WORKDIR),
            &files(&["web/src/components/stage/PlanView.tsx"]),
        );
        let sibling_done = AtomicBool::new(false);
        let reruns = AtomicUsize::new(0);
        let step = failed_step("T08", &own, Duration::from_secs(30));

        let settle = tasks.settle_failed_step(&step, failing(PLAN_VIEW_ERROR), || async {
            assert!(
                sibling_done.load(Ordering::SeqCst),
                "re-ran before the sibling finished"
            );
            reruns.fetch_add(1, Ordering::SeqCst);
            Verdict::pass("verify[0:typecheck]")
        });
        let finish_sibling = async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            sibling_done.store(true, Ordering::SeqCst);
            drop(sibling);
        };
        let ((verdict, blocked_by), ()) = tokio::time::timeout(Duration::from_secs(10), async {
            tokio::join!(settle, finish_sibling)
        })
        .await
        .expect("settles once the sibling finishes");

        assert!(verdict.passed);
        assert_eq!(blocked_by, None);
        assert_eq!(reruns.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_failure_that_persists_in_a_sibling_file_names_the_sibling() {
        let tasks = InFlightTasks::default();
        let own = files(&["web/src/components/Header.tsx"]);
        let _own = tasks.register("plan/T08", Path::new(WORKDIR), &own);
        let sibling = tasks.register(
            "plan/T12",
            Path::new(WORKDIR),
            &files(&["web/src/components/stage/PlanView.tsx"]),
        );
        let step = failed_step("T08", &own, Duration::from_secs(30));

        let settle = tasks.settle_failed_step(&step, failing(PLAN_VIEW_ERROR), || async {
            failing(PLAN_VIEW_ERROR)
        });
        let finish_sibling = async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            drop(sibling);
        };
        let ((verdict, blocked_by), ()) = tokio::join!(settle, finish_sibling);

        assert!(!verdict.passed);
        assert_eq!(blocked_by.as_deref(), Some("T12"));
        let detail = verdict.detail.unwrap_or_default();
        assert!(detail.starts_with(PLAN_VIEW_ERROR), "{detail}");
        assert!(
            detail.ends_with(
                "blocked_by_sibling = T12: every located error, first at \
                 src/components/stage/PlanView.tsx:448:24, is in a file it declares"
            ),
            "{detail}"
        );
    }

    #[tokio::test]
    async fn a_writer_that_never_settles_is_waited_for_only_up_to_the_limit() {
        let tasks = InFlightTasks::default();
        let own = files(&["web/src/a.ts"]);
        let _own = tasks.register("plan/T08", Path::new(WORKDIR), &own);
        let _stuck = tasks.register("plan/T12", Path::new(WORKDIR), &files(&["web/src/b.ts"]));
        let reruns = AtomicUsize::new(0);
        let step = failed_step("T08", &own, Duration::from_millis(50));
        let started = Instant::now();

        let (verdict, blocked_by) = tokio::time::timeout(
            Duration::from_secs(5),
            tasks.settle_failed_step(&step, failing("boom"), || async {
                reruns.fetch_add(1, Ordering::SeqCst);
                failing("boom")
            }),
        )
        .await
        .expect("the wait is bounded");

        assert!(started.elapsed() >= Duration::from_millis(50));
        assert!(!verdict.passed);
        assert_eq!(reruns.load(Ordering::SeqCst), 1);
        // Nothing located, so nothing is attributed.
        assert_eq!(blocked_by, None);
        assert!(
            !verdict
                .detail
                .unwrap_or_default()
                .contains("blocked_by_sibling")
        );
    }

    #[tokio::test]
    async fn two_tasks_failing_together_do_not_wait_on_each_other() {
        let tasks = InFlightTasks::default();
        let a_files = files(&["web/src/a.ts"]);
        let b_files = files(&["web/src/b.ts"]);
        let a = tasks.register("plan/A", Path::new(WORKDIR), &a_files);
        let b = tasks.register("plan/B", Path::new(WORKDIR), &b_files);
        let a_step = failed_step("A", &a_files, Duration::from_secs(30));
        let b_step = failed_step("B", &b_files, Duration::from_secs(30));
        let settle_a = async {
            let (verdict, _) = tasks
                .settle_failed_step(&a_step, failing("a"), || async { Verdict::pass("a") })
                .await;
            drop(a);
            verdict
        };
        let settle_b = async {
            let (verdict, _) = tasks
                .settle_failed_step(&b_step, failing("b"), || async { Verdict::pass("b") })
                .await;
            drop(b);
            verdict
        };

        let (a_verdict, b_verdict) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(settle_a, settle_b)
        })
        .await
        .expect("no deadlock");

        // The first to fail waits for the other, which sees it settling and
        // keeps its own failure.
        assert_ne!(a_verdict.passed, b_verdict.passed);
    }

    #[test]
    fn sibling_files_since_names_every_attempt_that_overlapped_the_window() {
        let tasks = InFlightTasks::default();
        let workdir = Path::new(WORKDIR);
        let before = tasks.register("plan/BEFORE", workdir, &files(&["done.rs"]));
        drop(before);
        let running = tasks.register("plan/RUNNING", workdir, &files(&["running.rs"]));
        let own = tasks.register("plan/OWN", workdir, &files(&["own.rs"]));

        let mark = tasks.mark();
        // A sibling that starts and ends inside the window still counts.
        drop(tasks.register("plan/BRIEF", workdir, &files(&["brief.rs"])));
        drop(running);
        let _elsewhere = tasks.register("plan/OTHER", Path::new("/other"), &files(&["x.rs"]));

        assert_eq!(
            tasks.sibling_files_since(&mark, "plan/OWN", workdir),
            ["brief.rs", "running.rs"]
        );
        drop(own);
    }
}
