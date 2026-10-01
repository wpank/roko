//! Verify steps that wait for siblings' edits, and edits that wait for
//! siblings' verify steps (gap-1920ba).
//!
//! Tasks that share a working tree read each other's half-written files.
//! Before a verify step runs, it waits until no sibling that writes where it
//! reads ([`StepScope`]) is mid-edit, and while it runs, a sibling that would
//! write there waits before it starts editing. A task running its verify
//! steps edits nothing, so it never counts as an editor, and two verifying
//! tasks never wait on each other. Both waits are bounded by
//! `[gates] sibling_settle_secs` (`0` turns them off). When the bound runs
//! out the step or the edit goes ahead, and a failure caused by a sibling's
//! edit is left to `settle_failed_step`.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;

use super::{InFlightAttempt, InFlightGuard, InFlightTasks, StepScope};

/// A task's verify phase: until it drops, the task edits nothing.
pub(crate) struct VerifyingGuard<'a> {
    tasks: &'a InFlightTasks,
    key: String,
}

/// One verify step while it runs: until it drops, siblings that would write
/// where the step reads wait before they start editing.
pub(crate) struct ReadingGuard<'a> {
    tasks: &'a InFlightTasks,
    key: String,
}

/// A verify step about to run, and how long it may wait for siblings.
pub(crate) struct StepRead<'a> {
    pub plan_id: &'a str,
    pub task_id: &'a str,
    /// Step label for logs, such as `verify[0:typecheck]`.
    pub label: &'a str,
    pub workdir: &'a Path,
    /// What the step reads.
    pub scope: &'a StepScope,
    /// `[gates] sibling_settle_secs`; zero waits for nothing.
    pub limit: Duration,
}

impl Drop for VerifyingGuard<'_> {
    fn drop(&mut self) {
        self.tasks.update(&self.key, |attempt| {
            attempt.verifying = false;
            attempt.reading = None;
        });
    }
}

impl Drop for ReadingGuard<'_> {
    fn drop(&mut self) {
        self.tasks
            .update(&self.key, |attempt| attempt.reading = None);
    }
}

impl InFlightTasks {
    /// Start the verify phase of `key` (`"{plan_id}/{task_id}"`): until the
    /// guard drops, its attempts edit nothing, so no sibling's verify step
    /// waits for them.
    pub(crate) fn begin_verify(&self, key: &str) -> VerifyingGuard<'_> {
        self.update(key, |attempt| attempt.verifying = true);
        VerifyingGuard {
            tasks: self,
            key: key.to_string(),
        }
    }

    /// Before a verify step runs: record what it reads, so that siblings
    /// that would write there wait, then wait up to the step's limit until
    /// no sibling that writes there is mid-edit. The step counts as reading
    /// until the guard drops.
    pub(crate) async fn begin_step(&self, step: &StepRead<'_>) -> ReadingGuard<'_> {
        let key = format!("{}/{}", step.plan_id, step.task_id);
        self.update(&key, |attempt| attempt.reading = Some(step.scope.clone()));
        let guard = ReadingGuard {
            tasks: self,
            key: key.clone(),
        };
        if step.limit.is_zero() {
            return guard;
        }
        let editors = self.editors(&key, step.workdir, step.scope);
        if editors.is_empty() {
            return guard;
        }
        tracing::info!(
            plan_id = step.plan_id,
            task_id = step.task_id,
            step = step.label,
            siblings = %editors.join(", "),
            "verify step waits for sibling tasks editing what it reads"
        );
        let edited = self
            .wait_until(step.limit, |tasks| {
                tasks.editors(&key, step.workdir, step.scope).is_empty()
            })
            .await;
        if !edited {
            tracing::warn!(
                plan_id = step.plan_id,
                task_id = step.task_id,
                step = step.label,
                "sibling tasks still editing after [gates] sibling_settle_secs; \
                 running the verify step anyway"
            );
        }
        guard
    }

    /// Record an attempt of `key` that may edit `files` in `workdir`, as
    /// [`Self::register`] does, once no sibling there runs a verify step that
    /// reads those files, waiting at most `limit` for them.
    pub(crate) async fn register_when_unread(
        &self,
        key: &str,
        workdir: &Path,
        files: &[String],
        limit: Duration,
    ) -> InFlightGuard<'_> {
        if limit.is_zero() {
            return self.register(key, workdir, files);
        }
        let mut changed = self.changed.subscribe();
        let mut readers = Vec::new();
        let registered = tokio::time::timeout(limit, async {
            loop {
                match self.register_unread(key, workdir, files) {
                    Ok(guard) => return guard,
                    Err(reading) if readers.is_empty() => {
                        tracing::info!(
                            task = key,
                            siblings = %reading.join(", "),
                            "attempt waits for sibling verify steps that read its files"
                        );
                        readers = reading;
                    }
                    Err(reading) => readers = reading,
                }
                if changed.changed().await.is_err() {
                    return self.register(key, workdir, files);
                }
            }
        })
        .await;
        registered.unwrap_or_else(|_| {
            tracing::warn!(
                task = key,
                siblings = %readers.join(", "),
                "sibling verify steps still read this task's files after \
                 [gates] sibling_settle_secs; starting the attempt anyway"
            );
            self.register(key, workdir, files)
        })
    }

    /// Register as [`Self::register`] does unless a sibling's verify step in
    /// `workdir` reads `files`, and otherwise name those siblings. The check
    /// and the registration share one lock, so a step that starts meanwhile
    /// sees the attempt and waits for it instead.
    fn register_unread(
        &self,
        key: &str,
        workdir: &Path,
        files: &[String],
    ) -> Result<InFlightGuard<'_>, Vec<String>> {
        let mut attempts = self.attempts.lock();
        let readers: Vec<String> = attempts
            .values()
            .filter(|attempt| {
                attempt.key != key
                    && attempt.workdir == workdir
                    && attempt
                        .reading
                        .as_ref()
                        .is_some_and(|scope| scope.covers(files))
            })
            .map(|attempt| attempt.key.clone())
            .collect();
        if !readers.is_empty() {
            return Err(readers);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        attempts.insert(
            id,
            InFlightAttempt {
                key: key.to_string(),
                workdir: workdir.to_path_buf(),
                files: files.to_vec(),
                settling: false,
                verifying: false,
                reading: None,
            },
        );
        Ok(InFlightGuard { tasks: self, id })
    }

    /// Siblings of `key` in `workdir` that are mid-edit on what `scope`
    /// reads: they declare files there and are neither settling nor running
    /// their verify steps.
    fn editors(&self, key: &str, workdir: &Path, scope: &StepScope) -> Vec<String> {
        self.attempts
            .lock()
            .values()
            .filter(|attempt| {
                attempt.key != key
                    && attempt.workdir == workdir
                    && !attempt.settling
                    && !attempt.verifying
                    && scope.covers(&attempt.files)
            })
            .map(|attempt| attempt.key.clone())
            .collect()
    }

    /// Wait up to `limit` until `done` holds, checking again whenever an
    /// attempt ends or changes state. Returns whether it held in time.
    async fn wait_until(&self, limit: Duration, done: impl Fn(&Self) -> bool) -> bool {
        let mut changed = self.changed.subscribe();
        tokio::time::timeout(limit, async {
            while !done(self) {
                if changed.changed().await.is_err() {
                    break;
                }
            }
        })
        .await
        .is_ok()
    }

    /// Apply `change` to every attempt of `key`, then wake the waiters.
    fn update(&self, key: &str, change: impl Fn(&mut InFlightAttempt)) {
        for attempt in self
            .attempts
            .lock()
            .values_mut()
            .filter(|attempt| attempt.key == key)
        {
            change(attempt);
        }
        self.bump();
    }

    /// Whether a verify step of `key` is running or waiting to run, for
    /// tests that act once a step has reached its wait.
    #[cfg(test)]
    pub(crate) fn reading(&self, key: &str) -> bool {
        self.attempts
            .lock()
            .values()
            .any(|attempt| attempt.key == key && attempt.reading.is_some())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::*;

    const WORKDIR: &str = "/repo";

    fn files(paths: &[&str]) -> Vec<String> {
        paths.iter().map(ToString::to_string).collect()
    }

    fn step<'a>(task_id: &'a str, scope: &'a StepScope) -> StepRead<'a> {
        StepRead {
            plan_id: "plan",
            task_id,
            label: "verify[0:check]",
            workdir: Path::new(WORKDIR),
            scope,
            limit: Duration::from_secs(30),
        }
    }

    /// Time a wait that should end at once gets: far below the steps'
    /// limit, so a wait that ran to the limit fails the test.
    const PROMPTLY: Duration = Duration::from_secs(5);

    /// Let spawned tasks run up to the point where they wait.
    async fn let_run() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    /// A whole-project step waits until the sibling mid-edit has finished
    /// its attempt; a step that reads elsewhere runs at once.
    #[tokio::test]
    async fn a_step_waits_only_for_editors_of_what_it_reads() {
        let tasks = Arc::new(InFlightTasks::default());
        let editor = tasks.register("plan/T12", Path::new(WORKDIR), &files(&["web/src/a.ts"]));

        let elsewhere = StepScope::Paths(vec![PathBuf::from("crates/core")]);
        let step_elsewhere = step("T08", &elsewhere);
        let reading = tokio::time::timeout(PROMPTLY, tasks.begin_step(&step_elsewhere)).await;
        drop(reading.expect("a step reading elsewhere runs at once"));

        let waiting = {
            let tasks = Arc::clone(&tasks);
            tokio::spawn(async move {
                drop(tasks.begin_step(&step("T08", &StepScope::Whole)).await);
            })
        };
        let_run().await;
        assert!(
            !waiting.is_finished(),
            "the step ran while its sibling edited"
        );
        drop(editor);
        let ran = tokio::time::timeout(PROMPTLY, waiting).await;
        ran.expect("the step runs once its sibling is done")
            .expect("step");
    }

    /// A sibling that is running its verify steps edits nothing: no step
    /// waits for it, and neither does a failed step that settles.
    #[tokio::test]
    async fn a_verifying_sibling_is_not_an_editor() {
        let tasks = InFlightTasks::default();
        let _sibling = tasks.register("plan/T12", Path::new(WORKDIR), &files(&["web/src/a.ts"]));
        let _verifying = tasks.begin_verify("plan/T12");
        let _own = tasks.register("plan/T08", Path::new(WORKDIR), &files(&["web/src/b.ts"]));

        let whole = StepScope::Whole;
        let reading = tokio::time::timeout(PROMPTLY, tasks.begin_step(&step("T08", &whole))).await;
        drop(reading.expect("no sibling edits"));
        assert!(
            tasks
                .begin_settle("plan/T08", Path::new(WORKDIR))
                .is_empty()
        );
    }

    /// While a whole-project step runs, a sibling that would edit the tree
    /// waits for it before it registers; one in another worktree does not.
    #[tokio::test]
    async fn an_edit_waits_for_a_step_that_reads_its_files() {
        let tasks = Arc::new(InFlightTasks::default());
        let _own = tasks.register("plan/T08", Path::new(WORKDIR), &files(&["web/src/b.ts"]));
        let verifying = tasks.begin_verify("plan/T08");
        let reading = tasks.begin_step(&step("T08", &StepScope::Whole)).await;

        let isolated = tokio::time::timeout(
            PROMPTLY,
            tasks.register_when_unread(
                "plan/T13",
                Path::new("/repo/.roko/worktrees/T13"),
                &files(&["web/src/a.ts"]),
                Duration::from_secs(30),
            ),
        )
        .await;
        drop(isolated.expect("an edit in another worktree starts at once"));

        let editing = {
            let tasks = Arc::clone(&tasks);
            tokio::spawn(async move {
                let guard = tasks
                    .register_when_unread(
                        "plan/T12",
                        Path::new(WORKDIR),
                        &files(&["web/src/a.ts"]),
                        Duration::from_secs(30),
                    )
                    .await;
                drop(guard);
            })
        };
        let_run().await;
        assert!(
            !editing.is_finished(),
            "the edit started while the step read its files"
        );
        drop(reading);
        let started = tokio::time::timeout(PROMPTLY, editing).await;
        started
            .expect("the edit starts once the step is done")
            .expect("edit");
        drop(verifying);
    }
}
