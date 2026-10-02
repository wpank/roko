//! Verified-knowledge sink — grows durable knowledge from task attempts
//! whose authored verify steps all passed.
//!
//! Each [`FeedbackEvent::TaskVerified`] becomes one
//! [`RuntimeEpisodeObservation`] for [`RuntimeKnowledgeLifecycle`], which:
//!
//! - admits the lesson the agent stated, its `Lesson:` line, when it is
//!   novel, and nothing for a pass that states none (decision 4201, backlog
//!   4216); a repeat of a stored lesson confirms that entry instead, and
//!   other close matches go to the evidence-based admission store, so the
//!   durable store does not fill with copies,
//! - reinforces the knowledge entries the attempt's prompt surfaced, and
//!   records the gate confirmation and context once on each of them and on
//!   the learned entry, so `TierProgression` can promote them
//!   (Working → Consolidated → ...),
//! - appends an auditable receipt to `.roko/neuro/knowledge-lifecycle.jsonl`.
//!
//! Tasks without verify steps never emit the event: a provider's own claim
//! of success is not evidence.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use roko_learn::episode_logger::EpisodeGateVerdict;
use roko_neuro::{RuntimeEpisodeObservation, RuntimeKnowledgeLifecycle, SourceChannel};

use super::{FeedbackEvent, FeedbackSink};

/// Longest agent output kept on an observation, in bytes.
const MAX_AGENT_OUTPUT_BYTES: usize = 2_000;

/// Most declared files kept as knowledge tags.
const MAX_FILE_TAGS: usize = 16;
/// The longest lesson a verified pass stores, in characters (decision 4201).
const MAX_LESSON_CHARS: usize = 300;

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
            lesson: stated_lesson(&self.agent_output),
            context_entry_ids: self.knowledge_ids.clone(),
            task_tags,
            source_channel: SourceChannel::GateVerdict,
            observed_at: chrono::Utc::now(),
        }
    }
}

/// The lesson the agent stated: the text after `Lesson:` on the last line of
/// its output that starts with it (backlog 4216). `none`, an empty lesson or
/// one longer than [`MAX_LESSON_CHARS`] is no lesson.
fn stated_lesson(agent_output: &str) -> Option<String> {
    let lesson = agent_output
        .lines()
        .rev()
        .find_map(|line| line.trim().strip_prefix("Lesson:"))?
        .trim();
    let none = lesson.is_empty()
        || lesson.trim_end_matches('.').eq_ignore_ascii_case("none")
        || lesson.chars().count() > MAX_LESSON_CHARS;
    (!none).then(|| lesson.to_string())
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
        matches!(event, FeedbackEvent::TaskVerified(_))
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use roko_neuro::{
        KnowledgeEntry, KnowledgeKind, KnowledgeStore, KnowledgeTier, RuntimeAdmissionPath,
    };
    use tempfile::tempdir;

    /// The lesson the fixture's agent states.
    const LESSON: &str = "Keep hello/main.rs free of external crates.";

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
            agent_output: format!("Created hello/main.rs printing hello world.\nLesson: {LESSON}"),
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
        assert_eq!(learned.content, LESSON);
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

    /// backlog 4216 (decision 4201): a verified pass stores the lesson its
    /// agent stated, not a success note, and a pass that states none adds
    /// no entry. The last `Lesson:` line counts; an over-long one is none.
    #[tokio::test]
    async fn verified_pass_stores_stated_lesson() {
        let dir = tempdir().expect("tempdir");
        let store = KnowledgeStore::for_workdir(dir.path());
        let sink = VerifiedKnowledgeSink::for_workdir(dir.path());
        let mut silent = attempt("run-1:hello-plan/T01/a0", vec![]);
        silent.agent_output = "Created hello/main.rs.\nLesson: none".into();
        sink.on_event(&FeedbackEvent::TaskVerified(silent))
            .await
            .expect("ingest the silent pass");
        assert!(store.read_all().expect("read").is_empty());

        let stated = attempt("run-2:hello-plan/T01/a0", vec![]);
        sink.on_event(&FeedbackEvent::TaskVerified(stated))
            .await
            .expect("ingest the stated lesson");
        let entries = store.read_all().expect("read");
        let [learned] = entries.as_slice() else {
            panic!("one lesson, one entry: {entries:#?}");
        };
        assert_eq!(learned.content, LESSON);
        assert!(learned.tags.contains(&"lesson".to_string()), "{:?}", learned.tags);
        assert!(learned.tags.contains(&"hello".to_string()), "{:?}", learned.tags);

        let long = format!("Lesson: {}", "x".repeat(MAX_LESSON_CHARS + 1));
        assert_eq!(stated_lesson(&long), None);
        let twice = "Lesson: first\nmore work\n  Lesson: second";
        assert_eq!(stated_lesson(twice).as_deref(), Some("second"));
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
