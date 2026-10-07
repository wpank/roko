//! Error-pattern sink: a failure of the agent's work on the Graph path goes
//! into the error-pattern store that prompt assembly reads.
//!
//! The Graph plan runner loads `.roko/learn/error-patterns.json` into the
//! dispatch factory's shared [`ErrorPatternStore`], and dispatch formats its
//! top patterns into every prompt. This sink records each failed attempt in
//! that same store, so the run's later attempts see the failure at once, and
//! saves the store, so the next run starts from it. Only a verify failure
//! whose settled learning label is a failure (S01 §4.1) counts. A turn-cap
//! stop or a timeout after output fails the agent too, but says nothing about
//! the code: the retry's turn policy, the episode and the router keep those
//! (backlog 4208). Provider and harness failures say nothing about the
//! agent's work.
//!
//! A verified pass of a task that failed earlier in the run records its fix
//! on the patterns the task failed with (backlog 4125), so the prompt shows
//! each failure with what fixed it. A pass no verify step checked records
//! nothing.
//!
//! It is the one error-pattern writer on the Graph path.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use async_trait::async_trait;
use roko_learn::error_pattern_store::{
    ErrorPatternStore, GateFailureObservation, GateFailureSource, normalize_error_digest,
};

use super::{FeedbackEvent, FeedbackSink, VerifiedAttempt};

/// How many lines of a verified attempt's final answer its fix quotes.
const RESOLUTION_ANSWER_LINES: usize = 3;

/// The pattern keys each `(plan, task)` failed with.
type FailedPatterns = HashMap<(String, String), BTreeSet<String>>;

/// Sink that records failed attempts in the shared error-pattern store, and
/// the fixes of verified retries, and saves it.
#[derive(Debug)]
pub struct ErrorPatternSink {
    store: Arc<RwLock<ErrorPatternStore>>,
    path: PathBuf,
    /// The pattern keys each `(plan, task)` failed with in this run, until a
    /// verified pass of the task records its fix on them.
    failed: Mutex<FailedPatterns>,
}

impl ErrorPatternSink {
    /// Sink recording into `store`, the dispatch factory's shared store, and
    /// saving it to `path`, `.roko/learn/error-patterns.json`.
    #[must_use]
    pub fn new(store: Arc<RwLock<ErrorPatternStore>>, path: impl Into<PathBuf>) -> Self {
        Self {
            store,
            path: path.into(),
            failed: Mutex::new(HashMap::new()),
        }
    }

    /// The pattern keys each task failed with in this run.
    fn failed(&self) -> Result<MutexGuard<'_, FailedPatterns>, anyhow::Error> {
        self.failed
            .lock()
            .map_err(|_| anyhow::anyhow!("failed-pattern map lock poisoned"))
    }

    /// Record the fix of `attempt`, a verified pass, on the patterns its task
    /// failed with earlier in this run, and save the store. A task that did
    /// not fail first records nothing.
    async fn record_fix(&self, attempt: &VerifiedAttempt) -> Result<(), anyhow::Error> {
        let task = (attempt.plan_id.clone(), attempt.task_id.clone());
        // The lock is released before the save is awaited.
        let keys = self.failed()?.remove(&task);
        let Some(keys) = keys else {
            return Ok(());
        };
        let resolution = resolution_summary(attempt);
        let resolved_by = attempt.attempt_id.clone();
        let store = Arc::clone(&self.store);
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let mut store = store
                .write()
                .map_err(|_| anyhow::anyhow!("error pattern store lock poisoned"))?;
            for key in &keys {
                store.record_resolution(key, &resolution, &resolved_by);
            }
            store.save(&path)?;
            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|error| anyhow::anyhow!("error pattern task join: {error}"))?
    }
}

#[async_trait]
impl FeedbackSink for ErrorPatternSink {
    fn name(&self) -> &'static str {
        "error_patterns"
    }

    /// A verify failure of the agent's work (learning label 0) with its
    /// reason, and a verified pass (label 1), which may fix one.
    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(event, FeedbackEvent::TaskVerified(_)) || is_verify_failure(event)
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        if let FeedbackEvent::TaskVerified(attempt) = event {
            return self.record_fix(attempt).await;
        }
        if !is_verify_failure(event) {
            return Ok(());
        }
        let FeedbackEvent::TaskCompleted {
            plan_id,
            task_id,
            failure_reason: Some(reason),
            ..
        } = event
        else {
            return Ok(());
        };
        let Some(observation) = observation(plan_id, task_id, reason) else {
            return Ok(());
        };
        self.failed()?
            .entry((plan_id.clone(), task_id.clone()))
            .or_default()
            .insert(observation.key.clone());
        let store = Arc::clone(&self.store);
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let mut store = store
                .write()
                .map_err(|_| anyhow::anyhow!("error pattern store lock poisoned"))?;
            store.observe_gate_failure(observation);
            store.save(&path)?;
            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|error| anyhow::anyhow!("error pattern task join: {error}"))?
    }
}

/// The `failure_reason` class of a verify failure.
const VERIFY_CLASS: &str = "verify";

/// Whether `event` is a verify failure of the agent's work (learning label
/// 0) with its reason.
fn is_verify_failure(event: &FeedbackEvent) -> bool {
    matches!(
        event,
        FeedbackEvent::TaskCompleted {
            failure_reason: Some(reason),
            ..
        } if failure_class(reason) == VERIFY_CLASS
    ) && event.learning_success() == Some(false)
}

/// The fix a verified attempt records: the first lines of its final answer
/// and the files its task declares. The store cuts it to
/// `MAX_RESOLUTION_CHARS`.
fn resolution_summary(attempt: &VerifiedAttempt) -> String {
    let answer = attempt
        .agent_output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(RESOLUTION_ANSWER_LINES)
        .collect::<Vec<_>>()
        .join(" ");
    let files = attempt.files.join(", ");
    match (answer.is_empty(), files.is_empty()) {
        (_, true) => answer,
        (true, false) => format!("changed {files}"),
        (false, false) => format!("{answer} (files: {files})"),
    }
}

/// The class of a class-prefixed `failure_reason`: `verify`, `turn_cap`,
/// `timeout`, …
fn failure_class(failure_reason: &str) -> &str {
    failure_reason
        .split_once(": ")
        .map_or("failure", |(class, _)| class)
}

/// The error-pattern observation of a failed attempt's class-prefixed
/// `failure_reason`, when it is a verify failure (`"verify: …"`). Its key is
/// the class and the failure's digest, so a failure that recurs across
/// attempts, tasks and plans merges into one pattern. Its gate is the failing
/// step's command, so prompts select it for the tasks that run that command
/// (backlog 4210); a failure that quotes none keeps the class.
fn observation(
    plan_id: &str,
    task_id: &str,
    failure_reason: &str,
) -> Option<GateFailureObservation> {
    let (class, detail) = failure_reason
        .split_once(": ")
        .unwrap_or(("failure", failure_reason));
    if class != VERIFY_CLASS {
        return None;
    }
    let digest = failure_digest(detail);
    if digest.is_empty() {
        return None;
    }
    Some(GateFailureObservation::new(
        format!("{class}::{digest}"),
        plan_id,
        Some(task_id.to_string()),
        failing_command(detail).unwrap_or(class),
        class,
        digest,
        GateFailureSource::GateClassification,
    ))
}

/// The command a verify failure's first step line quotes: the line's first
/// backtick-quoted span, as verification writes it,
/// ``verify[0:test] (`cargo test -p app`): exit code: 101``, or a pack
/// rung's ``rung[clippy] (`…`): …``. The older
/// ``verify[0:test] `cargo test -p app` failed: …`` reads the same.
fn failing_command(detail: &str) -> Option<&str> {
    detail.lines().find_map(|line| {
        let line = line.trim_start();
        if !(line.starts_with("verify[") || line.starts_with("rung[")) {
            return None;
        }
        let (_, quoted) = line.split_once('`')?;
        let (command, _) = quoted.split_once('`')?;
        (!command.trim().is_empty()).then_some(command)
    })
}

/// A failure's digest: the first line after its summary line (a verify
/// failure's first failing step), else the summary line, normalized.
fn failure_digest(detail: &str) -> String {
    let mut lines = detail.lines().filter(|line| !line.trim().is_empty());
    let summary = lines.next().unwrap_or_default();
    normalize_error_digest(lines.next().unwrap_or(summary))
}

#[cfg(test)]
mod tests {
    use roko_learn::telemetry::AttemptOutcome;
    use tempfile::tempdir;

    use super::*;
    use crate::dispatch::{AgentOutcome, ModelChoiceSource};
    use crate::runtime_feedback::settled_as;

    /// A completed Graph attempt of `task_id` that settled as `outcome`.
    fn completed(task_id: &str, outcome: AttemptOutcome, failure_reason: &str) -> FeedbackEvent {
        FeedbackEvent::TaskCompleted {
            plan_id: "plan-e".into(),
            task_id: task_id.into(),
            outcome: AgentOutcome {
                task_id: task_id.into(),
                plan_id: "plan-e".into(),
                model: "claude-sonnet-4-6".into(),
                provider: "claude_cli".into(),
                output: String::new(),
                tokens_in: 0,
                tokens_out: 0,
                cost_usd: 0.0,
                duration_ms: 0,
                exit_code: Some(1),
                is_error: true,
            },
            model_source: ModelChoiceSource::Router,
            succeeded: false,
            routing_context: None,
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: String::new(),
            turns: 1,
            failure_reason: Some(failure_reason.to_string()),
            settled: settled_as(outcome, true),
        }
    }

    /// gap-2ce86f: a Graph attempt that fails its verify step adds the
    /// failure to the shared error-pattern store, which prompt assembly
    /// reads, and saves it to `learn/error-patterns.json`. A recurring failure
    /// merges into one pattern; a provider failure adds nothing.
    #[tokio::test]
    async fn a_failed_graph_attempt_updates_the_error_pattern_store() {
        let temp = tempdir().unwrap();
        let path = temp.path().join(".roko/learn/error-patterns.json");
        let store = Arc::new(RwLock::new(ErrorPatternStore::empty()));
        let sink = ErrorPatternSink::new(Arc::clone(&store), &path);
        let verify_failure = |title: &str| {
            format!(
                "verify: 1/1 verify step(s) failed for task `{title}`:\n\n\
                 verify[0:test] (`cargo test -p app`): exit code: 101\n\
                 thread 'greets' panicked at src/lib.rs:4:5"
            )
        };

        for (task_id, title) in [("T1", "Greet"), ("T2", "Wave")] {
            let event = completed(task_id, AttemptOutcome::GateFailed, &verify_failure(title));
            assert!(sink.interested(&event));
            sink.on_event(&event).await.unwrap();
        }
        let provider_failure = completed(
            "T3",
            AttemptOutcome::ProviderError,
            "provider: exit 1: upstream connect error",
        );
        assert!(!sink.interested(&provider_failure));
        sink.on_event(&provider_failure).await.unwrap();

        let saved = ErrorPatternStore::load(&path);
        assert_eq!(saved.len(), 1, "one recurring failure is one pattern");
        let pattern = saved.top_patterns(1)[0];
        assert_eq!(pattern.occurrences, 2);
        assert_eq!(pattern.gate.as_deref(), Some("cargo test -p app"));
        assert_eq!(pattern.category, "verify");
        assert_eq!(pattern.task_ids.len(), 2);
        assert!(
            pattern.digest.contains("cargo test -p app"),
            "{}",
            pattern.digest
        );
        let prompt = store.read().unwrap().format_for_prompt(5);
        assert!(prompt.contains("cargo test -p app"), "{prompt}");
    }

    /// The failing command is the step line's first quoted span: in the form
    /// verification writes, ``verify[i:phase] (`cmd`): …``, in a pack rung's
    /// ``rung[name] (`cmd`): …``, and in the older ``verify[i:phase] `cmd`
    /// failed: …``. A failure without a step line quotes none.
    #[test]
    fn failing_command_reads_the_step_lines_quoted_command() {
        let cases = [
            (
                "1/1 failed:\n\nverify[0:test] (`cargo test -p app`): exit code: 101",
                "cargo test -p app",
            ),
            (
                "1/1 failed:\n\nrung[clippy] (`cargo clippy -- -D warnings`): failed",
                "cargo clippy -- -D warnings",
            ),
            (
                "1/1 failed:\n\nverify[0:test] `cargo test -p app` failed: exit code: 101",
                "cargo test -p app",
            ),
        ];
        for (detail, command) in cases {
            assert_eq!(failing_command(detail), Some(command), "{detail}");
        }
        assert_eq!(failing_command("1/1 failed:\n\nno step line"), None);
    }

    /// backlog 4208: a turn-cap stop or a timeout after output fails the
    /// agent, but says nothing about the code, so it adds no error pattern
    /// and the store is not written.
    #[tokio::test]
    async fn turn_cap_failure_records_no_error_pattern() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join(".roko/learn/error-patterns.json");
        let store = Arc::new(RwLock::new(ErrorPatternStore::empty()));
        let sink = ErrorPatternSink::new(Arc::clone(&store), &path);
        let cases = [
            (
                AttemptOutcome::TurnCap,
                "turn_cap: agent turn cap reached (turns=12, cap=12)",
            ),
            (
                AttemptOutcome::Timeout,
                "timeout: attempt timed out after 600s",
            ),
        ];
        for (outcome, reason) in cases {
            let event = completed("T1", outcome, reason);
            assert_eq!(event.learning_success(), Some(false), "{reason}");
            assert!(!sink.interested(&event), "{reason}");
            sink.on_event(&event).await.expect("on_event");
        }
        assert_eq!(store.read().expect("store").len(), 0);
        assert!(!path.exists(), "nothing was saved");
    }

    /// A verified attempt of `task_id` whose final answer is `answer`.
    fn verified(task_id: &str, answer: &str) -> FeedbackEvent {
        FeedbackEvent::TaskVerified(VerifiedAttempt {
            plan_id: "plan-e".into(),
            task_id: task_id.into(),
            attempt_id: format!("gr-e:plan-e:{task_id}:2"),
            title: "Greet".into(),
            task_type: "focused".into(),
            role: "implementer".into(),
            model: "claude-sonnet-4-6".into(),
            files: vec!["src/lib.rs".into()],
            verify_steps: vec![("verify[0:test]".into(), "cargo test -p app".into())],
            knowledge_ids: vec![],
            agent_output: answer.into(),
        })
    }

    /// backlog 4125: a task that fails its verify step and then passes it
    /// records the fix on its failure's pattern: the first lines of the
    /// verified answer with the task's files, and the attempt's key. The
    /// pattern stays unresolved, so prompts keep showing it, now with its
    /// fix. A pass no verify step checked, and a verified pass of a task that
    /// never failed, record nothing.
    #[tokio::test]
    async fn verified_retry_records_resolution_on_its_failure_pattern() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join(".roko/learn/error-patterns.json");
        let store = Arc::new(RwLock::new(ErrorPatternStore::empty()));
        let sink = ErrorPatternSink::new(Arc::clone(&store), &path);
        for (task_id, crate_name) in [("T1", "app"), ("T2", "web")] {
            let reason = format!(
                "verify: 1/1 verify step(s) failed for task `{task_id}`:\n\n\
                 verify[0:test] (`cargo test -p {crate_name}`): exit code: 101"
            );
            let failed = completed(task_id, AttemptOutcome::GateFailed, &reason);
            sink.on_event(&failed).await.expect("record the failure");
        }

        // T1 passes its verify step on its retry, T2 passes with nothing to
        // verify it, and T3 passes without having failed.
        let fixed = verified(
            "T1",
            "Added `greet`.\n\nIt returns a greeting.\nDone.\nMore.",
        );
        assert!(sink.interested(&fixed));
        sink.on_event(&fixed).await.expect("record the fix");
        let unverified = completed("T2", AttemptOutcome::Unverified, "unverified");
        assert!(!sink.interested(&unverified));
        sink.on_event(&unverified).await.expect("ignore the pass");
        let never_failed = verified("T3", "Nothing to fix.");
        sink.on_event(&never_failed)
            .await
            .expect("nothing to record");

        let saved = ErrorPatternStore::load(&path);
        assert_eq!(saved.len(), 2);
        let pattern = |gate: &str| {
            saved
                .top_patterns(10)
                .into_iter()
                .find(|pattern| pattern.gate.as_deref() == Some(gate))
                .cloned()
                .expect("the failure's pattern")
        };
        let app = pattern("cargo test -p app");
        assert_eq!(
            app.resolution.as_deref(),
            Some("Added `greet`. It returns a greeting. Done. (files: src/lib.rs)")
        );
        assert_eq!(app.resolved_by.as_deref(), Some("gr-e:plan-e:T1:2"));
        assert!(!app.resolved, "a pattern with a fix stays in prompts");
        let web = pattern("cargo test -p web");
        assert_eq!((web.resolution, web.resolved_by), (None, None));
        let prompt = store.read().expect("store").format_for_prompt(5);
        assert!(prompt.contains("Fix: Added `greet`."), "{prompt}");
    }
}
