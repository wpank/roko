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
//! It is the one error-pattern writer on the Graph path.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use roko_learn::error_pattern_store::{
    ErrorPatternStore, GateFailureObservation, GateFailureSource, normalize_error_digest,
};

use super::{FeedbackEvent, FeedbackSink};

/// Sink that records failed attempts in the shared error-pattern store and
/// saves it.
#[derive(Debug)]
pub struct ErrorPatternSink {
    store: Arc<RwLock<ErrorPatternStore>>,
    path: PathBuf,
}

impl ErrorPatternSink {
    /// Sink recording into `store`, the dispatch factory's shared store, and
    /// saving it to `path`, `.roko/learn/error-patterns.json`.
    #[must_use]
    pub fn new(store: Arc<RwLock<ErrorPatternStore>>, path: impl Into<PathBuf>) -> Self {
        Self {
            store,
            path: path.into(),
        }
    }
}

#[async_trait]
impl FeedbackSink for ErrorPatternSink {
    fn name(&self) -> &'static str {
        "error_patterns"
    }

    /// A verify failure of the agent's work (learning label 0) with its
    /// reason.
    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(
            event,
            FeedbackEvent::TaskCompleted {
                failure_reason: Some(reason),
                ..
            } if failure_class(reason) == VERIFY_CLASS
        ) && event.learning_success() == Some(false)
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        if !self.interested(event) {
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

/// The command a verify failure's first step line quotes, as in
/// ``verify[0:test] `cargo test -p app` failed: exit code: 101``, or a
/// workspace rung's ``rung[clippy] `…` failed: …``.
fn failing_command(detail: &str) -> Option<&str> {
    detail.lines().find_map(|line| {
        let line = line.trim_start();
        if !(line.starts_with("verify[") || line.starts_with("rung[")) {
            return None;
        }
        let (_, quoted) = line.split_once(" `")?;
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
                 verify[0:test] `cargo test -p app` failed: exit code: 101\n\
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
}
