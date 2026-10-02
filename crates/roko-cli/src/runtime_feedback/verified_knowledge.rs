//! Verified-knowledge sink — grows durable knowledge from task attempts
//! whose authored verify steps all passed.
//!
//! Each [`FeedbackEvent::TaskVerified`] becomes one
//! [`RuntimeEpisodeObservation`] for [`RuntimeKnowledgeLifecycle`], which:
//!
//! - admits a strategy fragment describing the verified attempt when it is
//!   novel; a repeat of a stored entry (the same task verified again)
//!   confirms that entry instead, and other close matches go to the
//!   evidence-based admission store, so the durable store does not fill with
//!   copies,
//! - reinforces the knowledge entries the attempt's prompt surfaced, and
//!   records the gate confirmation and context once on each of them and on
//!   the learned entry, so `TierProgression` can promote them
//!   (Working → Consolidated → ...),
//! - appends an auditable receipt to `.roko/neuro/knowledge-lifecycle.jsonl`.
//!
//! Tasks without verify steps never emit the event: a provider's own claim
//! of success is not evidence.
//!
//! An attempt that failed through the agent's own work (learning label 0,
//! blame `agent`) counts one contradiction against each entry its prompt
//! surfaced, and weakens none of them (S02 L5, decision 4).

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use roko_learn::episode_logger::EpisodeGateVerdict;
use roko_learn::telemetry::Blame;
use roko_neuro::{RuntimeEpisodeObservation, RuntimeKnowledgeLifecycle, SourceChannel};

use super::{FeedbackEvent, FeedbackSink};

/// Longest agent output kept on an observation, in bytes.
const MAX_AGENT_OUTPUT_BYTES: usize = 2_000;

/// Most declared files kept as knowledge tags.
const MAX_FILE_TAGS: usize = 16;

/// A task attempt whose every authored verify step passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAttempt {
    pub plan_id: String,
    pub task_id: String,
    /// Attempt identity, unique across runs; knowledge provenance
    /// (`source_episodes`) cites it.
    pub attempt_id: String,
    pub title: String,
    /// Tier or task type (`"focused"`, ...).
    pub task_type: String,
    pub role: String,
    /// Model that produced the verified output.
    pub model: String,
    /// Files the task declares.
    pub files: Vec<String>,
    /// `(label, command)` of each passed verify step, such as
    /// `("verify[0:test]", "cargo test")`.
    pub verify_steps: Vec<(String, String)>,
    /// Knowledge entry ids the attempt's prompt surfaced.
    pub knowledge_ids: Vec<String>,
    /// The agent's final output.
    pub agent_output: String,
}

impl VerifiedAttempt {
    /// The knowledge-lifecycle observation for this attempt.
    #[must_use]
    pub fn observation(&self) -> RuntimeEpisodeObservation {
        let non_empty = |value: &str| (!value.trim().is_empty()).then(|| value.to_string());
        let mut task_tags: Vec<String> = self.files.iter().take(MAX_FILE_TAGS).cloned().collect();
        task_tags.push(format!("plan:{}", self.plan_id));
        let gate_output = self
            .verify_steps
            .iter()
            .map(|(label, command)| format!("{label} `{command}` passed"))
            .collect::<Vec<_>>()
            .join("\n");
        let agent_output = bounded(
            &format!("Verified `{}`: {}", self.title, self.agent_output.trim()),
            MAX_AGENT_OUTPUT_BYTES,
        );
        RuntimeEpisodeObservation {
            episode_id: self.attempt_id.clone(),
            task_id: self.task_id.clone(),
            plan_id: non_empty(&self.plan_id),
            task_type: non_empty(&self.task_type),
            model: non_empty(&self.model),
            agent_id: non_empty(&self.role),
            gate_passed: true,
            gate_verdicts: self
                .verify_steps
                .iter()
                .map(|(label, _)| EpisodeGateVerdict::new(label, true))
                .collect(),
            gate_output,
            agent_output,
            context_entry_ids: self.knowledge_ids.clone(),
            task_tags,
            source_channel: SourceChannel::GateVerdict,
            observed_at: chrono::Utc::now(),
        }
    }
}

/// The first `max` bytes of `text`, cut at a character boundary.
fn bounded(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

/// Sink that feeds [`FeedbackEvent::TaskVerified`] into the durable
/// knowledge lifecycle.
#[derive(Debug)]
pub struct VerifiedKnowledgeSink {
    lifecycle: RuntimeKnowledgeLifecycle,
    /// One ingestion at a time: each rewrites the knowledge store several
    /// times, and parallel tasks can finish together.
    serial: Arc<Mutex<()>>,
}

impl VerifiedKnowledgeSink {
    /// Sink over the lifecycle rooted at `<workdir>/.roko`.
    #[must_use]
    pub fn for_workdir(workdir: impl AsRef<Path>) -> Self {
        Self::new(RuntimeKnowledgeLifecycle::for_workdir(workdir))
    }

    /// Sink over an explicit lifecycle.
    #[must_use]
    pub fn new(lifecycle: RuntimeKnowledgeLifecycle) -> Self {
        Self {
            lifecycle,
            serial: Arc::new(Mutex::new(())),
        }
    }
}

#[async_trait]
impl FeedbackSink for VerifiedKnowledgeSink {
    fn name(&self) -> &'static str {
        "verified_knowledge"
    }

    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(event, FeedbackEvent::TaskVerified(_)) || agent_blamed_failure(event).is_some()
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        if let Some((attempt_key, knowledge_ids)) = agent_blamed_failure(event) {
            return self.record_contradictions(attempt_key, knowledge_ids).await;
        }
        let FeedbackEvent::TaskVerified(attempt) = event else {
            return Ok(());
        };
        let observation = attempt.observation();
        let lifecycle = self.lifecycle.clone();
        let serial = Arc::clone(&self.serial);
        let record = tokio::task::spawn_blocking(move || {
            let _serial = serial.lock().unwrap_or_else(PoisonError::into_inner);
            lifecycle.ingest_observation(observation)
        })
        .await
        .map_err(|error| anyhow::anyhow!("verified knowledge ingest task join: {error}"))??;
        tracing::debug!(
            plan_id = %attempt.plan_id,
            task_id = %attempt.task_id,
            admission = ?record.admission_path,
            entry = ?record.candidate_entry_id,
            reinforced = record.gated_reinforcements,
            promoted = record.promotion_updates,
            "verified attempt ingested into durable knowledge"
        );
        Ok(())
    }
}

impl VerifiedKnowledgeSink {
    /// Count the failed attempt `attempt_key` against each entry of
    /// `knowledge_ids` its prompt surfaced, without weakening any.
    async fn record_contradictions(
        &self,
        attempt_key: &str,
        knowledge_ids: &[String],
    ) -> Result<(), anyhow::Error> {
        if knowledge_ids.is_empty() {
            return Ok(());
        }
        let store = self.lifecycle.knowledge_store().clone();
        let ids = knowledge_ids.to_vec();
        let key = attempt_key.to_string();
        let serial = Arc::clone(&self.serial);
        let counted = tokio::task::spawn_blocking(move || {
            let _serial = serial.lock().unwrap_or_else(PoisonError::into_inner);
            let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
            store.record_contradiction(&ids, &key)
        })
        .await
        .map_err(|error| anyhow::anyhow!("knowledge contradiction task join: {error}"))??;
        tracing::debug!(
            attempt_key,
            counted,
            "an agent-blamed failure counted against the knowledge it surfaced"
        );
        Ok(())
    }
}

/// The attempt key and surfaced knowledge ids of `event` when it settles an
/// attempt that failed through the agent's own work: learning label 0,
/// blame `agent` (S01 §4.3). Infra and harness failures, and attempts
/// without a label, are no evidence about the knowledge.
fn agent_blamed_failure(event: &FeedbackEvent) -> Option<(&str, &[String])> {
    let FeedbackEvent::TaskCompleted {
        settled: Some(settled),
        knowledge_ids,
        ..
    } = event
    else {
        return None;
    };
    (settled.learning_label == Some(0) && settled.blame == Blame::Agent)
        .then(|| (settled.identity.attempt_key.as_str(), knowledge_ids.as_slice()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_learn::telemetry::AttemptOutcome;
    use roko_neuro::{
        KnowledgeEntry, KnowledgeKind, KnowledgeStore, KnowledgeTier, RuntimeAdmissionPath,
    };
    use tempfile::tempdir;

    fn attempt(attempt_id: &str, knowledge_ids: Vec<String>) -> VerifiedAttempt {
        VerifiedAttempt {
            plan_id: "hello-plan".into(),
            task_id: "T01".into(),
            attempt_id: attempt_id.into(),
            title: "Write the hello world program".into(),
            task_type: "focused".into(),
            role: "implementer".into(),
            model: "claude-sonnet-4-6".into(),
            files: vec!["hello/main.rs".into()],
            verify_steps: vec![(
                "verify[0:build]".into(),
                "rustc hello/main.rs -o hello/hello-bin".into(),
            )],
            knowledge_ids,
            agent_output: "Created hello/main.rs printing hello world.".into(),
        }
    }

    #[test]
    fn observation_is_a_bounded_gate_verdict_pass() {
        let mut long = attempt("run:hello-plan/T01/a0", vec![]);
        long.agent_output = "é".repeat(MAX_AGENT_OUTPUT_BYTES);
        let observation = long.observation();

        assert!(observation.gate_passed);
        assert_eq!(observation.source_channel, SourceChannel::GateVerdict);
        assert_eq!(observation.episode_id, "run:hello-plan/T01/a0");
        assert_eq!(
            observation.gate_verdicts,
            [EpisodeGateVerdict::new("verify[0:build]", true)]
        );
        assert!(observation.gate_output.contains("`rustc hello/main.rs"));
        assert!(
            observation
                .agent_output
                .starts_with("Verified `Write the hello world program`: ")
        );
        assert!(observation.agent_output.len() <= MAX_AGENT_OUTPUT_BYTES);
        assert!(observation.task_tags.contains(&"hello/main.rs".to_string()));
        assert!(
            observation
                .task_tags
                .contains(&"plan:hello-plan".to_string())
        );
    }

    #[tokio::test]
    async fn verified_attempt_becomes_durable_knowledge_and_promotes_context() {
        let dir = tempdir().unwrap();
        let store = KnowledgeStore::for_workdir(dir.path());
        store
            .add(KnowledgeEntry {
                id: "prior-hint".into(),
                kind: KnowledgeKind::Insight,
                content: "Rust hello world programs compile with a plain rustc call".into(),
                confidence: 0.8,
                confidence_weight: 0.8,
                tags: vec!["rust".into()],
                ..KnowledgeEntry::default()
            })
            .unwrap();
        let sink = VerifiedKnowledgeSink::for_workdir(dir.path());

        sink.on_event(&FeedbackEvent::TaskVerified(attempt(
            "run-1:hello-plan/T01/a0",
            vec!["prior-hint".into()],
        )))
        .await
        .unwrap();

        let entries = store.read_all().unwrap();
        let learned = entries
            .iter()
            .find(|entry| entry.source.as_deref() == Some("runtime:gate_verdict"))
            .expect("the verified attempt is admitted as durable knowledge");
        assert_eq!(learned.kind, KnowledgeKind::StrategyFragment);
        assert!(learned.content.contains("Write the hello world program"));
        assert_eq!(learned.source_episodes, ["run-1:hello-plan/T01/a0"]);
        assert!(learned.confirmation_count >= 1);
        let hint = entries
            .iter()
            .find(|entry| entry.id == "prior-hint")
            .unwrap();
        assert!(
            hint.confirmation_count >= 1,
            "the surfaced entry gains the gate confirmation"
        );
        assert!(
            hint.distinct_contexts
                .iter()
                .any(|context| context.contains("run-1:hello-plan/T01/a0"))
        );
        assert_ne!(hint.tier, KnowledgeTier::Transient);

        let receipts = sink.lifecycle.read_records().unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(
            receipts[0].admission_path,
            RuntimeAdmissionPath::LightAdmitted
        );
        assert_eq!(receipts[0].gated_reinforcements, 1);
    }

    #[tokio::test]
    async fn repeated_verified_attempts_confirm_one_entry() {
        let dir = tempdir().unwrap();
        let sink = VerifiedKnowledgeSink::for_workdir(dir.path());
        for run in 0..3 {
            sink.on_event(&FeedbackEvent::TaskVerified(attempt(
                &format!("run-{run}:hello-plan/T01/a0"),
                vec![],
            )))
            .await
            .unwrap();
        }

        let durable = KnowledgeStore::for_workdir(dir.path())
            .read_all()
            .unwrap()
            .into_iter()
            .filter(|entry| entry.source.as_deref() == Some("runtime:gate_verdict"))
            .collect::<Vec<_>>();
        let [learned] = durable.as_slice() else {
            panic!("the same attempt verified again must not add entries: {durable:#?}");
        };
        assert_eq!(learned.confirmation_count, 3, "one confirmation per run");
        assert_eq!(learned.tier, KnowledgeTier::Consolidated);
        let paths = sink
            .lifecycle
            .read_records()
            .unwrap()
            .into_iter()
            .map(|record| record.admission_path)
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            [
                RuntimeAdmissionPath::LightAdmitted,
                RuntimeAdmissionPath::Duplicate,
                RuntimeAdmissionPath::Duplicate
            ]
        );
    }

    /// The `TaskCompleted` event of an attempt that settled `outcome` after
    /// its prompt surfaced `knowledge_ids`.
    fn completed(outcome: AttemptOutcome, knowledge_ids: Vec<String>) -> FeedbackEvent {
        FeedbackEvent::TaskCompleted {
            plan_id: "p".into(),
            task_id: "t".into(),
            outcome: crate::dispatch::AgentOutcome {
                task_id: "t".into(),
                plan_id: "p".into(),
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
            model_source: crate::dispatch::ModelChoiceSource::Router,
            succeeded: false,
            routing_context: None,
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids,
            playbook_ids: vec![],
            initial_model: String::new(),
            turns: 0,
            failure_reason: None,
            settled: crate::runtime_feedback::settled_as(outcome, true),
        }
    }

    /// S02 L5, decision 4: an attempt that failed through the agent's own
    /// work counts one contradiction against each entry its prompt surfaced
    /// and weakens none of them; an infra failure, or an attempt without a
    /// learning label, records nothing.
    #[tokio::test]
    async fn agent_blamed_failure_counts_a_contradiction_without_weakening() {
        let dir = tempdir().unwrap();
        let store = KnowledgeStore::for_workdir(dir.path());
        store
            .add(KnowledgeEntry {
                id: "prior-hint".into(),
                kind: KnowledgeKind::Insight,
                content: "Rust hello world programs compile with a plain rustc call".into(),
                confidence: 0.8,
                confidence_weight: 0.8,
                ..KnowledgeEntry::default()
            })
            .unwrap();
        let before = store.read_all().unwrap().remove(0);
        let sink = VerifiedKnowledgeSink::for_workdir(dir.path());
        let surfaced = vec!["prior-hint".to_string()];

        for outcome in [AttemptOutcome::ProviderError, AttemptOutcome::Unverified] {
            let event = completed(outcome, surfaced.clone());
            assert!(!sink.interested(&event), "{outcome:?}");
            sink.on_event(&event).await.unwrap();
        }
        assert_eq!(store.read_all().unwrap()[0].contradiction_count, 0);

        let failure = completed(AttemptOutcome::GateFailed, surfaced);
        assert!(sink.interested(&failure));
        sink.on_event(&failure).await.unwrap();
        let after = store.read_all().unwrap().remove(0);
        assert_eq!(after.contradiction_count, 1);
        assert_eq!(after.confidence, before.confidence);
        assert_eq!(after.confidence_weight, before.confidence_weight);
        assert_eq!(after.balance, before.balance);
        assert_eq!(after.tier, before.tier);
        assert_eq!(after.confirmation_count, before.confirmation_count);
    }

    #[tokio::test]
    async fn sink_ignores_other_events() {
        let dir = tempdir().unwrap();
        let sink = VerifiedKnowledgeSink::for_workdir(dir.path());
        let event = FeedbackEvent::IdleTick {
            ticks_since_last_work: 1,
        };
        assert!(!sink.interested(&event));
        sink.on_event(&event).await.unwrap();
        assert!(!dir.path().join(".roko").exists());
    }
}
