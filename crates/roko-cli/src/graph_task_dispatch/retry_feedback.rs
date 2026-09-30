//! Gate feedback carried from a failed verify to a task's next attempt.
//!
//! A failed verify leaves [`GateFeedback`] (the failing steps' output and a
//! cheap model's diagnosis) for the task's next attempt prompt. The book keeps
//! it per plan in `retry-feedback.json` beside the plan's Graph checkpoint,
//! rewritten atomically on every change, so it outlives the process: the
//! attempt a `--resume-plan` run starts, after an interrupt or after the retry
//! budget ran out, still gets it. A passing verify clears it. Feedback left
//! under another checkpoint run (`--fresh` starts one) is ignored.
//!
//! Attempt numbers count a task's dispatches from 0. Within a process they
//! follow the Graph engine's retry counter (attempt `k` is retry `k` of the
//! task's `max_retries`, whatever failed before it); a resumed run continues
//! from the attempt the persisted feedback was left for.
//!
//! The same file keeps each task's [`LadderStanding`] (gap-460230): the rungs
//! it climbed on the model ladder and its agent-blamed failures on the
//! current rung, so a resumed run routes where the last one stopped.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::dispatch::GateFeedback;

/// On-disk schema of `retry-feedback.json`.
const SCHEMA_VERSION: u32 = 1;

/// Feedback left for a task's next attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PendingFeedback {
    /// Attempt number of the dispatch that receives the feedback.
    pub(crate) next_attempt: u32,
    /// When the failed verify left it (RFC 3339).
    pub(crate) recorded_at: String,
    pub(crate) feedback: GateFeedback,
}

/// A task's standing on the model ladder (gap-460230): the rungs it climbed
/// above its start rung, and its agent-blamed failures on the current one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LadderStanding {
    /// Rungs climbed above the start rung.
    pub(crate) escalations: u32,
    /// Agent-blamed failures on the current rung.
    pub(crate) failures_on_rung: u32,
}

/// `retry-feedback.json`: one plan's pending feedback and ladder standings,
/// keyed by task id.
#[derive(Debug, Serialize, Deserialize)]
struct RetryFeedbackFile {
    schema_version: u32,
    plan_id: String,
    /// Graph checkpoint run the feedback belongs to.
    run_id: String,
    tasks: BTreeMap<String, PendingFeedback>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    ladder: BTreeMap<String, LadderStanding>,
}

/// A task's next dispatch: its attempt number and the feedback it starts with.
#[derive(Debug)]
pub(crate) struct NextAttempt {
    pub(crate) attempt: u32,
    pub(crate) feedback: Option<GateFeedback>,
}

/// `(plan_id, task_id)`.
type TaskKey = (String, String);

#[derive(Debug, Default)]
struct BookState {
    pending: HashMap<TaskKey, PendingFeedback>,
    /// Attempts earlier processes of a resumed run made.
    earlier_attempts: HashMap<TaskKey, u32>,
    /// Standings on the model ladder.
    ladder: HashMap<TaskKey, LadderStanding>,
    /// Per plan: the file its feedback is kept in, and the checkpoint run.
    files: HashMap<String, (PathBuf, String)>,
}

/// Pending gate feedback of every task one dispatcher runs.
#[derive(Debug, Default)]
pub(crate) struct RetryFeedbackBook {
    state: parking_lot::Mutex<BookState>,
}

impl RetryFeedbackBook {
    /// Keep `plan_id`'s feedback in `path` for Graph checkpoint run `run_id`,
    /// restoring what an earlier process of that run left there. Returns the
    /// ids of the tasks whose feedback was restored.
    pub(crate) fn attach(&self, plan_id: &str, path: PathBuf, run_id: &str) -> Vec<String> {
        let (restored, standings) = read_feedback_file(&path, plan_id, run_id);
        let mut state = self.state.lock();
        state.pending.retain(|(plan, _), _| plan != plan_id);
        state
            .earlier_attempts
            .retain(|(plan, _), _| plan != plan_id);
        state.ladder.retain(|(plan, _), _| plan != plan_id);
        for (task_id, standing) in standings {
            state
                .ladder
                .insert((plan_id.to_string(), task_id), standing);
        }
        let mut task_ids = Vec::with_capacity(restored.len());
        for (task_id, entry) in restored {
            let key = (plan_id.to_string(), task_id.clone());
            state
                .earlier_attempts
                .insert(key.clone(), entry.next_attempt);
            state.pending.insert(key, entry);
            task_ids.push(task_id);
        }
        state
            .files
            .insert(plan_id.to_string(), (path, run_id.to_string()));
        task_ids
    }

    /// The Graph checkpoint run `plan_id`'s feedback is attached to.
    pub(crate) fn run_id(&self, plan_id: &str) -> Option<String> {
        let state = self.state.lock();
        state.files.get(plan_id).map(|(_, run_id)| run_id.clone())
    }

    /// Number the dispatch of `task_id` that is attempt `attempt_in_run` of
    /// this process, and return the feedback it starts with.
    pub(crate) fn next_attempt(
        &self,
        plan_id: &str,
        task_id: &str,
        attempt_in_run: u32,
    ) -> NextAttempt {
        let key = (plan_id.to_string(), task_id.to_string());
        let state = self.state.lock();
        let earlier = state.earlier_attempts.get(&key).copied().unwrap_or(0);
        NextAttempt {
            attempt: earlier.saturating_add(attempt_in_run),
            feedback: state.pending.get(&key).map(|entry| entry.feedback.clone()),
        }
    }

    /// Leave `feedback` for attempt `next_attempt` of `task_id`. Returns the
    /// file it is kept in, or `None` when it is kept in memory only (the plan
    /// has no checkpoint, or the write failed).
    pub(crate) fn record(
        &self,
        plan_id: &str,
        task_id: &str,
        feedback: GateFeedback,
        next_attempt: u32,
    ) -> Option<PathBuf> {
        let mut state = self.state.lock();
        state.pending.insert(
            (plan_id.to_string(), task_id.to_string()),
            PendingFeedback {
                next_attempt,
                recorded_at: chrono::Utc::now().to_rfc3339(),
                feedback,
            },
        );
        persist(&state, plan_id)
    }

    /// `task_id`'s standing on the model ladder.
    pub(crate) fn ladder_standing(&self, plan_id: &str, task_id: &str) -> LadderStanding {
        let key = (plan_id.to_string(), task_id.to_string());
        self.state
            .lock()
            .ladder
            .get(&key)
            .copied()
            .unwrap_or_default()
    }

    /// Keep `standing` for `task_id`, on disk as well when the plan has a
    /// checkpoint, so a resumed run climbs from where this one stopped.
    pub(crate) fn set_ladder_standing(
        &self,
        plan_id: &str,
        task_id: &str,
        standing: LadderStanding,
    ) {
        let mut state = self.state.lock();
        state
            .ladder
            .insert((plan_id.to_string(), task_id.to_string()), standing);
        persist(&state, plan_id);
    }

    /// Drop `task_id`'s ladder standing once it passes.
    pub(crate) fn clear_ladder(&self, plan_id: &str, task_id: &str) {
        let key = (plan_id.to_string(), task_id.to_string());
        let mut state = self.state.lock();
        if state.ladder.remove(&key).is_some() {
            persist(&state, plan_id);
        }
    }

    /// Drop `task_id`'s feedback once it passes.
    pub(crate) fn clear(&self, plan_id: &str, task_id: &str) {
        let key = (plan_id.to_string(), task_id.to_string());
        let mut state = self.state.lock();
        state.earlier_attempts.remove(&key);
        if state.pending.remove(&key).is_some() {
            persist(&state, plan_id);
        }
    }
}

/// Rewrite `plan_id`'s file from `state`, removing it once nothing is
/// pending and no task stands above its start rung. Returns the file on
/// success.
fn persist(state: &BookState, plan_id: &str) -> Option<PathBuf> {
    let (path, run_id) = state.files.get(plan_id)?;
    let tasks: BTreeMap<String, PendingFeedback> = state
        .pending
        .iter()
        .filter(|((plan, _), _)| plan == plan_id)
        .map(|((_, task_id), entry)| (task_id.clone(), entry.clone()))
        .collect();
    let ladder: BTreeMap<String, LadderStanding> = state
        .ladder
        .iter()
        .filter(|((plan, _), _)| plan == plan_id)
        .map(|((_, task_id), standing)| (task_id.clone(), *standing))
        .collect();
    let written = if tasks.is_empty() && ladder.is_empty() {
        match std::fs::remove_file(path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    } else {
        let file = RetryFeedbackFile {
            schema_version: SCHEMA_VERSION,
            plan_id: plan_id.to_string(),
            run_id: run_id.clone(),
            tasks,
            ladder,
        };
        serde_json::to_vec_pretty(&file)
            .map_err(std::io::Error::other)
            .and_then(|bytes| roko_core::io::atomic_write(path, &bytes))
    };
    match written {
        Ok(()) => Some(path.clone()),
        Err(error) => {
            tracing::warn!(
                plan_id,
                path = %path.display(),
                %error,
                "retry feedback write failed; the next attempt of this run still gets it, \
                 a resumed run does not"
            );
            None
        }
    }
}

/// Pending feedback and ladder standings `path` holds for `plan_id`'s
/// checkpoint run `run_id`, keyed by task id. A missing, unreadable, or
/// foreign file holds none.
fn read_feedback_file(
    path: &Path,
    plan_id: &str,
    run_id: &str,
) -> (
    BTreeMap<String, PendingFeedback>,
    BTreeMap<String, LadderStanding>,
) {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(path = %path.display(), %error, "retry feedback unreadable; ignored");
            }
            return (BTreeMap::new(), BTreeMap::new());
        }
    };
    let file: RetryFeedbackFile = match serde_json::from_slice(&bytes) {
        Ok(file) => file,
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "retry feedback unparsable; ignored");
            return (BTreeMap::new(), BTreeMap::new());
        }
    };
    if file.schema_version != SCHEMA_VERSION || file.plan_id != plan_id {
        tracing::warn!(
            path = %path.display(),
            schema_version = file.schema_version,
            file_plan_id = %file.plan_id,
            plan_id,
            "retry feedback has another schema or plan; ignored"
        );
        return (BTreeMap::new(), BTreeMap::new());
    }
    if file.run_id != run_id {
        tracing::debug!(
            path = %path.display(),
            file_run_id = %file.run_id,
            run_id,
            "retry feedback belongs to an earlier checkpoint run; ignored"
        );
        return (BTreeMap::new(), BTreeMap::new());
    }
    (file.tasks, file.ladder)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn feedback(text: &str) -> GateFeedback {
        GateFeedback::from_raw(text)
            .expect("non-empty gate output")
            .with_diagnosis("the fixture file is missing")
    }

    #[test]
    fn a_later_process_of_the_same_run_restores_feedback_and_attempt_numbers() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("plan-a/retry-feedback.json");

        let first = RetryFeedbackBook::default();
        assert!(first.attach("plan-a", path.clone(), "run-1").is_empty());
        // Two provider failures, then the third dispatch fails its verify.
        assert_eq!(first.next_attempt("plan-a", "T05", 2).attempt, 2);
        let kept_in = first.record("plan-a", "T05", feedback("verify[0:test]: boom"), 3);
        assert_eq!(kept_in.as_deref(), Some(path.as_path()));

        let resumed = RetryFeedbackBook::default();
        assert_eq!(
            resumed.attach("plan-a", path.clone(), "run-1"),
            ["T05".to_string()]
        );
        let next = resumed.next_attempt("plan-a", "T05", 0);
        assert_eq!(next.attempt, 3, "a resumed run continues the numbering");
        let restored = next.feedback.expect("restored feedback");
        assert_eq!(restored.raw_output, "verify[0:test]: boom");
        assert_eq!(
            restored.diagnosis.as_deref(),
            Some("the fixture file is missing")
        );
        // The engine retries within the resumed run: numbering keeps going.
        assert_eq!(resumed.next_attempt("plan-a", "T05", 1).attempt, 4);
    }

    #[test]
    fn feedback_of_another_checkpoint_run_is_ignored() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("retry-feedback.json");
        let first = RetryFeedbackBook::default();
        first.attach("plan-a", path.clone(), "run-1");
        first.record("plan-a", "T01", feedback("error: boom"), 1);

        let fresh = RetryFeedbackBook::default();
        assert!(fresh.attach("plan-a", path, "run-2").is_empty());
        let next = fresh.next_attempt("plan-a", "T01", 0);
        assert_eq!(next.attempt, 0);
        assert!(next.feedback.is_none());
    }

    #[test]
    fn a_pass_clears_the_feedback_and_its_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("retry-feedback.json");
        let book = RetryFeedbackBook::default();
        book.attach("plan-a", path.clone(), "run-1");
        book.record("plan-a", "T01", feedback("error: one"), 1);
        book.record("plan-a", "T02", feedback("error: two"), 1);

        book.clear("plan-a", "T01");
        assert!(book.next_attempt("plan-a", "T01", 1).feedback.is_none());
        let later = RetryFeedbackBook::default();
        assert_eq!(
            later.attach("plan-a", path.clone(), "run-1"),
            ["T02".to_string()]
        );

        book.clear("plan-a", "T02");
        assert!(!path.exists(), "nothing pending leaves no file");
    }

    #[test]
    fn plans_keep_separate_files() {
        let dir = tempdir().expect("tempdir");
        let (a, b) = (dir.path().join("a.json"), dir.path().join("b.json"));
        let book = RetryFeedbackBook::default();
        book.attach("plan-a", a.clone(), "run-a");
        book.attach("plan-b", b.clone(), "run-b");
        book.record("plan-a", "T01", feedback("error: a"), 1);
        book.record("plan-b", "T01", feedback("error: b"), 1);
        book.clear("plan-a", "T01");

        assert!(!a.exists());
        let later = RetryFeedbackBook::default();
        assert_eq!(later.attach("plan-b", b, "run-b"), ["T01".to_string()]);
    }

    #[test]
    fn an_unreadable_file_is_ignored_then_replaced() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("retry-feedback.json");
        std::fs::write(&path, "{not json").expect("write garbage");
        let book = RetryFeedbackBook::default();
        assert!(book.attach("plan-a", path.clone(), "run-1").is_empty());

        book.record("plan-a", "T01", feedback("error: boom"), 1);
        let later = RetryFeedbackBook::default();
        assert_eq!(later.attach("plan-a", path, "run-1"), ["T01".to_string()]);
    }

    /// gap-460230: a task's ladder standing outlives the process like its
    /// feedback does. A resumed run of the same checkpoint climbs from where
    /// the last one stopped, a fresh run starts over, and a pass forgets it.
    #[test]
    fn escalation_rung_survives_resume() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("plan-a/retry-feedback.json");
        let first = RetryFeedbackBook::default();
        first.attach("plan-a", path.clone(), "run-1");
        let climbed = LadderStanding {
            escalations: 1,
            failures_on_rung: 1,
        };
        first.set_ladder_standing("plan-a", "T01", climbed);
        assert!(path.is_file(), "a standing alone keeps the file");

        let resumed = RetryFeedbackBook::default();
        assert!(
            resumed.attach("plan-a", path.clone(), "run-1").is_empty(),
            "no feedback is pending"
        );
        assert_eq!(resumed.ladder_standing("plan-a", "T01"), climbed);
        assert_eq!(
            resumed.ladder_standing("plan-a", "T02"),
            LadderStanding::default()
        );

        let fresh = RetryFeedbackBook::default();
        fresh.attach("plan-a", path.clone(), "run-2");
        assert_eq!(
            fresh.ladder_standing("plan-a", "T01"),
            LadderStanding::default()
        );

        resumed.clear_ladder("plan-a", "T01");
        assert!(!path.exists(), "nothing is left to keep");
    }

    #[test]
    fn without_a_checkpoint_feedback_stays_in_memory() {
        let book = RetryFeedbackBook::default();
        assert!(
            book.record("plan-a", "T01", feedback("error: boom"), 1)
                .is_none()
        );
        let next = book.next_attempt("plan-a", "T01", 1);
        assert_eq!(next.attempt, 1);
        assert!(next.feedback.is_some());
    }
}
