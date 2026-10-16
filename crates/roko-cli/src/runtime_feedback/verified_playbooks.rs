//! Verified-playbook sink: plan runs write L-play's playbooks (gap-89f985).
//!
//! Each [`FeedbackEvent::TaskVerified`], a gate-backed pass, offers the
//! attempt's tool calls as a playbook ([`extract_playbook_from_episode`]):
//! one step per call that did not fail, in order, keyed by the task's topic
//! terms, the words prompt assembly matches playbooks against (backlog 4212).
//! Admission:
//!
//! - only a verified pass, whose task has at least [`MIN_TOPIC_OVERLAP`]
//!   topic terms (a playbook with fewer could never reach a prompt) and whose
//!   attempt made a tool call that did not fail;
//! - a stored playbook whose topic terms and steps both match the
//!   candidate's ([`MATCH_THRESHOLD`]) gains the success instead, unless the
//!   attempt's prompt included it: dispatch credits those with the attempt's
//!   label already (W07);
//! - at most [`MAX_NEW_PLAYBOOKS_PER_RUN`] new playbooks per plan run.
//!
//! The producer makes no model call. A frozen run (decision 2218) registers
//! no such sink, so it writes no playbook. What the sink writes is published
//! on the run's hub as `playbook_written`, which the workspace event log
//! keeps.

use std::collections::HashSet;
use std::hash::Hash;
use std::path::PathBuf;

use async_trait::async_trait;
use roko_learn::playbook::{Playbook, PlaybookStore, extract_playbook_from_episode};

use super::{AttemptToolCall, FeedbackEvent, FeedbackSink, VerifiedAttempt};
use crate::dispatch::prompt_builder::{MIN_TOPIC_OVERLAP, topic_terms};

/// Most new playbooks one plan run creates.
pub const MAX_NEW_PLAYBOOKS_PER_RUN: usize = 3;

/// Most steps a playbook keeps: its attempt's first tool calls.
const MAX_PLAYBOOK_STEPS: usize = 12;

/// Least overlap (Jaccard) of topic terms, and of steps, at which a stored
/// playbook is the one a verified attempt repeats; the playbook store merges
/// similar playbooks at the same bar.
const MATCH_THRESHOLD: f64 = 0.8;

/// The longest goal a `playbook_written` event carries, in characters.
const EVENT_GOAL_CHARS: usize = 160;

/// Sink that turns [`FeedbackEvent::TaskVerified`] into playbooks under
/// `.roko/learn/playbooks/`.
pub struct VerifiedPlaybookSink {
    store: PlaybookStore,
    /// Where what the sink writes is published: the run's hub.
    publisher: Option<crate::state_hub::StateHubSender>,
    /// The new playbooks this run created. Each admission holds it from the
    /// store's read to its write, so two tasks that pass together cannot both
    /// create the same playbook.
    created: tokio::sync::Mutex<usize>,
}

impl std::fmt::Debug for VerifiedPlaybookSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifiedPlaybookSink")
            .field("store", &self.store.root())
            .field("publishes", &self.publisher.is_some())
            .finish_non_exhaustive()
    }
}

impl VerifiedPlaybookSink {
    /// Sink over the playbook store at `playbook_dir`.
    #[must_use]
    pub fn new(playbook_dir: impl Into<PathBuf>) -> Self {
        Self {
            store: PlaybookStore::new(playbook_dir),
            publisher: None,
            created: tokio::sync::Mutex::new(0),
        }
    }

    /// Publish what the sink writes on `publisher`, the run's hub, as a
    /// `playbook_written` event.
    #[must_use]
    pub fn with_publisher(mut self, publisher: crate::state_hub::StateHubSender) -> Self {
        self.publisher = Some(publisher);
        self
    }

    /// Admit the playbook `attempt` teaches, or credit the stored one it
    /// repeats. Returns what it wrote: `created` or `credited`, and the
    /// playbook.
    async fn admit(
        &self,
        attempt: &VerifiedAttempt,
    ) -> Result<Option<(&'static str, Playbook)>, anyhow::Error> {
        let Some(candidate) = candidate_playbook(attempt) else {
            tracing::debug!(
                plan_id = %attempt.plan_id,
                task_id = %attempt.task_id,
                "the verified attempt teaches no playbook: too few topic terms or no tool call"
            );
            return Ok(None);
        };
        let mut created = self.created.lock().await;
        let stored = self.store.list().await?;
        if let Some(repeated) = repeated_playbook(&candidate, &stored) {
            if attempt.playbook_ids.contains(&repeated.id) {
                // Its prompt included the playbook: dispatch credits it.
                return Ok(None);
            }
            let credited = self.store.record_outcome(&repeated.id, true).await?;
            return Ok(credited.then(|| ("credited", repeated.clone())));
        }
        if *created >= MAX_NEW_PLAYBOOKS_PER_RUN {
            tracing::debug!(
                plan_id = %attempt.plan_id,
                task_id = %attempt.task_id,
                cap = MAX_NEW_PLAYBOOKS_PER_RUN,
                "the run created its new playbooks; this one is left out"
            );
            return Ok(None);
        }
        self.store.save(&candidate).await?;
        *created += 1;
        Ok(Some(("created", candidate)))
    }
}

#[async_trait]
impl FeedbackSink for VerifiedPlaybookSink {
    fn name(&self) -> &'static str {
        "verified_playbooks"
    }

    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(event, FeedbackEvent::TaskVerified(_))
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        let FeedbackEvent::TaskVerified(attempt) = event else {
            return Ok(());
        };
        let Some((admission, playbook)) = self.admit(attempt).await? else {
            return Ok(());
        };
        tracing::debug!(
            plan_id = %attempt.plan_id,
            task_id = %attempt.task_id,
            admission,
            playbook_id = %playbook.id,
            steps = playbook.steps.len(),
            "a verified attempt wrote a playbook"
        );
        if let Some(publisher) = &self.publisher {
            publisher.publish(roko_core::DashboardEvent::PlaybookWritten {
                plan_id: attempt.plan_id.clone(),
                task_id: attempt.task_id.clone(),
                admission: admission.to_string(),
                playbook_id: playbook.id.clone(),
                goal: playbook.goal.chars().take(EVENT_GOAL_CHARS).collect(),
                steps: u32::try_from(playbook.steps.len()).unwrap_or(u32::MAX),
            });
        }
        Ok(())
    }
}

/// The playbook `attempt` teaches: its goal the task's title and
/// description, its steps the attempt's tool calls ([`playbook_steps`]), its
/// `when_pattern` the task's topic terms. `None` for a task with fewer than
/// [`MIN_TOPIC_OVERLAP`] topic terms, or an attempt without a step.
fn candidate_playbook(attempt: &VerifiedAttempt) -> Option<Playbook> {
    if attempt.topic_terms.len() < MIN_TOPIC_OVERLAP {
        return None;
    }
    let description = attempt.description.trim();
    let goal = if description.is_empty() {
        attempt.title.clone()
    } else {
        format!("{}: {description}", attempt.title.trim())
    };
    let goal = goal.split_whitespace().collect::<Vec<_>>().join(" ");
    let source = format!("{}-{}", attempt.plan_id, attempt.task_id);
    let mut playbook =
        extract_playbook_from_episode(&source, &goal, &playbook_steps(&attempt.tool_calls))?;
    playbook.when_pattern = Some(attempt.topic_terms.join(" "));
    Some(playbook)
}

/// The steps `calls` teach, as `(tool, what it acted on)`: each call that did
/// not fail, in order, its tool's name standing in for a target it did not
/// show. A call that repeats the step before it adds none, and the steps stop
/// at [`MAX_PLAYBOOK_STEPS`].
fn playbook_steps(calls: &[AttemptToolCall]) -> Vec<(String, String)> {
    let mut steps: Vec<(String, String)> = Vec::new();
    for call in calls {
        let name = call.name.trim();
        if name.is_empty() || call.succeeded == Some(false) {
            continue;
        }
        let target = call.target.trim();
        let target = if target.is_empty() { name } else { target };
        let step = (name.to_string(), target.to_string());
        if steps.last() != Some(&step) {
            steps.push(step);
        }
        if steps.len() == MAX_PLAYBOOK_STEPS {
            break;
        }
    }
    steps
}

/// The stored playbook `candidate` repeats: its topic terms and its steps
/// both overlap the candidate's by at least [`MATCH_THRESHOLD`]. The closest
/// one wins, then the lowest id.
fn repeated_playbook<'a>(candidate: &Playbook, stored: &'a [Playbook]) -> Option<&'a Playbook> {
    let topic = playbook_topic(candidate);
    let steps = step_signatures(candidate);
    stored
        .iter()
        .filter_map(|playbook| {
            let topic_overlap = jaccard(&topic, &playbook_topic(playbook));
            let step_overlap = jaccard(&steps, &step_signatures(playbook));
            (topic_overlap >= MATCH_THRESHOLD && step_overlap >= MATCH_THRESHOLD)
                .then_some((topic_overlap + step_overlap, playbook))
        })
        .max_by(|(a, x), (b, y)| a.total_cmp(b).then_with(|| y.id.cmp(&x.id)))
        .map(|(_, playbook)| playbook)
}

/// A playbook's topic terms: those of its `when_pattern`, which a verified
/// pass's playbook keys by its task's topic terms, else of its goal.
fn playbook_topic(playbook: &Playbook) -> HashSet<String> {
    topic_terms(playbook.when_pattern.as_deref().unwrap_or(&playbook.goal))
}

/// A playbook's steps, as `(tool, what it acted on)`.
fn step_signatures(playbook: &Playbook) -> HashSet<(String, String)> {
    playbook
        .steps
        .iter()
        .map(|step| {
            (
                step.action_kind.trim().to_ascii_lowercase(),
                step.description.trim().to_string(),
            )
        })
        .collect()
}

/// The Jaccard overlap of `a` and `b`; 0 when both are empty.
fn jaccard<T: Eq + Hash>(a: &HashSet<T>, b: &HashSet<T>) -> f64 {
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    a.intersection(b).count() as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// A tool call that succeeded.
    fn call(name: &str, target: &str) -> AttemptToolCall {
        AttemptToolCall {
            name: name.into(),
            target: target.into(),
            succeeded: Some(true),
        }
    }

    /// A verified pass of task `task_id` titled `title`, whose agent wrote
    /// `greeting.txt` and printed it.
    fn verified(task_id: &str, title: &str) -> VerifiedAttempt {
        let mut terms: Vec<String> = topic_terms(&format!("{title} Print a friendly banner."))
            .into_iter()
            .collect();
        terms.sort_unstable();
        VerifiedAttempt {
            plan_id: "greet".into(),
            task_id: task_id.into(),
            attempt_id: format!("run-1:greet/{task_id}/a1"),
            title: title.into(),
            description: "Print a friendly banner.".into(),
            topic_terms: terms,
            tool_calls: vec![
                call("Write", "greeting.txt"),
                call("Bash", "cat greeting.txt"),
            ],
            ..VerifiedAttempt::default()
        }
    }

    /// The `playbook_written` events published on `hub`, as `(task,
    /// admission, playbook id, steps)`.
    fn written(hub: &crate::state_hub::SharedStateHub) -> Vec<(String, String, String, u32)> {
        hub.subscribe_events_from(0)
            .replay
            .into_iter()
            .filter_map(|envelope| match envelope.payload {
                roko_core::DashboardEvent::PlaybookWritten {
                    task_id,
                    admission,
                    playbook_id,
                    steps,
                    ..
                } => Some((task_id, admission, playbook_id, steps)),
                _ => None,
            })
            .collect()
    }

    /// gap-89f985: a verified pass's tool calls become one playbook, keyed
    /// by its task's topic terms, and the run's hub hears it. The same
    /// procedure on the same topic credits that playbook instead of adding
    /// another, unless the attempt's prompt included it: dispatch credits
    /// those.
    #[tokio::test]
    async fn verified_pass_creates_a_playbook_then_credits_its_repeat() {
        let dir = tempdir().expect("tempdir");
        let hub = crate::state_hub::shared_state_hub();
        let sink = VerifiedPlaybookSink::new(dir.path()).with_publisher(hub.sender());
        let store = PlaybookStore::new(dir.path());

        let first = verified("T1", "Greeting banner");
        sink.on_event(&FeedbackEvent::TaskVerified(first))
            .await
            .expect("create");
        let playbooks = store.list().await.expect("list");
        let [playbook] = playbooks.as_slice() else {
            panic!("one playbook: {playbooks:#?}");
        };
        assert_eq!(playbook.goal, "Greeting banner: Print a friendly banner.");
        assert_eq!(
            playbook.when_pattern.as_deref(),
            Some("banner friendly greeting print")
        );
        let steps: Vec<(&str, &str)> = playbook
            .steps
            .iter()
            .map(|step| (step.action_kind.as_str(), step.description.as_str()))
            .collect();
        assert_eq!(
            steps,
            [("Write", "greeting.txt"), ("Bash", "cat greeting.txt")]
        );
        assert_eq!((playbook.success_count, playbook.failure_count), (1, 0));
        let id = playbook.id.clone();

        sink.on_event(&FeedbackEvent::TaskVerified(verified(
            "T1",
            "Greeting banner",
        )))
        .await
        .expect("credit");
        let mut included = verified("T1", "Greeting banner");
        included.playbook_ids = vec![id.clone()];
        sink.on_event(&FeedbackEvent::TaskVerified(included))
            .await
            .expect("left to dispatch");

        let playbooks = store.list().await.expect("list");
        assert_eq!(playbooks.len(), 1, "{playbooks:#?}");
        assert_eq!(playbooks[0].success_count, 2, "{playbooks:#?}");
        let created = ("T1".to_string(), "created".to_string(), id.clone(), 2);
        let credited = ("T1".to_string(), "credited".to_string(), id, 2);
        assert_eq!(written(&hub), [created, credited]);
    }

    /// Admission: a task with fewer than two topic terms, or an attempt whose
    /// only tool call failed, teaches nothing; another topic, or other steps,
    /// is a new playbook; failed and repeated calls are no steps; and a run
    /// creates at most three playbooks.
    #[tokio::test]
    async fn admission_needs_a_topic_and_a_step_and_stops_at_the_cap() {
        let dir = tempdir().expect("tempdir");
        let sink = VerifiedPlaybookSink::new(dir.path());
        let store = PlaybookStore::new(dir.path());

        let mut untopical = verified("T1", "Banner");
        untopical.topic_terms = vec!["banner".into()];
        let mut failed = verified("T2", "Greeting banner");
        failed.tool_calls = vec![AttemptToolCall {
            succeeded: Some(false),
            ..call("Bash", "false")
        }];
        for attempt in [untopical, failed] {
            sink.on_event(&FeedbackEvent::TaskVerified(attempt))
                .await
                .expect("nothing to admit");
        }
        assert!(store.list().await.expect("list").is_empty());

        let mut retried = verified("T3", "Greeting banner");
        retried.tool_calls = vec![
            call("Read", "greeting.txt"),
            call("Read", "greeting.txt"),
            AttemptToolCall {
                succeeded: Some(false),
                ..call("Bash", "false")
            },
            call("Edit", ""),
        ];
        let mut farewell = verified("T4", "Farewell message");
        farewell.description = "Print a parting message.".into();
        farewell.topic_terms = vec!["farewell".into(), "message".into(), "parting".into()];
        let fourth = verified("T5", "Weather report");
        for attempt in [verified("T1", "Greeting banner"), retried, farewell, fourth] {
            sink.on_event(&FeedbackEvent::TaskVerified(attempt))
                .await
                .expect("admit");
        }

        let playbooks = store.list().await.expect("list");
        assert_eq!(playbooks.len(), MAX_NEW_PLAYBOOKS_PER_RUN, "{playbooks:#?}");
        let retried = playbooks
            .iter()
            .find(|playbook| playbook.id.starts_with("ep-greet-T3-"))
            .expect("T3's playbook: the same topic with other steps");
        let steps: Vec<(&str, &str)> = retried
            .steps
            .iter()
            .map(|step| (step.action_kind.as_str(), step.description.as_str()))
            .collect();
        assert_eq!(steps, [("Read", "greeting.txt"), ("Edit", "Edit")]);
        assert!(
            !playbooks
                .iter()
                .any(|playbook| playbook.id.starts_with("ep-greet-T5-")),
            "the cap left T5 out: {playbooks:#?}"
        );
    }

    #[tokio::test]
    async fn sink_ignores_other_events() {
        let dir = tempdir().expect("tempdir");
        let sink = VerifiedPlaybookSink::new(dir.path().join("playbooks"));
        let event = FeedbackEvent::IdleTick {
            ticks_since_last_work: 1,
        };
        assert!(!sink.interested(&event));
        sink.on_event(&event).await.expect("ignored");
        assert!(!dir.path().join("playbooks").exists());
    }
}
