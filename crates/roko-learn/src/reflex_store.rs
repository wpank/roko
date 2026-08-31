//! T0 reflex store — condition-action pairs learned from successful T2 episodes.
//!
//! Before an LLM is selected, callers can check this store for an already
//! learned deterministic action. Rules are held in insertion order for fast,
//! predictable matching and persisted as a JSONL snapshot.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Maximum number of rules held by a reflex store.
pub const MAX_RULES: usize = 200;

const PROMOTE_MIN_HITS: u32 = 3;
const PROMOTE_MIN_CONFIDENCE: f64 = 0.90;
const DEMOTE_MULTIPLIER: f64 = 0.5;
const DEMOTE_DELETE_THRESHOLD: f64 = 0.50;

/// Observation pattern that must match for a rule to fire.
///
/// Every populated field must match. Unpopulated fields are wildcards.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ReflexCondition {
    /// Tool name that must be present in the observation (for example, `bash`).
    pub tool: Option<String>,
    /// Substring that must appear in the tool arguments.
    pub args_pattern: Option<String>,
    /// Substring that must appear in the observation context.
    pub context: Option<String>,
    /// Message type tag (for example, `user` or `tool_result`).
    pub message_type: Option<String>,
    /// File extension that must be visible in the context (for example, `.rs`).
    pub file_ext: Option<String>,
}

impl ReflexCondition {
    /// Return `true` when every populated field matches `observation`.
    ///
    /// This predicate is side-effect free; unlike
    /// [`ReflexStore::match_observation`], it does not update rule statistics.
    #[must_use]
    pub fn matches(&self, observation: &ReflexObservation) -> bool {
        condition_matches(self, observation)
    }
}

/// Deterministic action taken when a reflex condition matches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ReflexAction {
    /// Tool to invoke.
    pub tool: String,
    /// Arguments to pass to the tool.
    pub args: String,
}

/// Exact result of a reflex match, including feedback provenance.
///
/// Callers that may execute reflexes concurrently should retain `rule_id` and
/// use [`ReflexStore::record_gate_pass_for`] or
/// [`ReflexStore::record_gate_fail_for`] when the gate completes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflexMatch {
    /// Identifier of the rule that matched.
    pub rule_id: Uuid,
    /// Condition that matched, retained for exact feedback and demotion provenance.
    pub condition: ReflexCondition,
    /// Action selected by the matching rule.
    pub action: ReflexAction,
}

/// A condition-action rule in the reflex store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReflexRule {
    /// Stable rule identifier.
    pub id: Uuid,
    /// Pattern that must match for this rule to fire.
    pub condition: ReflexCondition,
    /// Action to take when the condition matches.
    pub action: ReflexAction,
    /// Ratio of successful gate outcomes to hits, adjusted by demotions.
    pub confidence: f64,
    /// Episode that produced this rule's first promotion.
    pub source_episode: String,
    /// Time at which the rule was created.
    pub promoted_at: DateTime<Utc>,
    /// Most recent time at which the rule fired.
    pub last_fired_at: Option<DateTime<Utc>>,
    /// Total number of times this rule matched an observation.
    pub hit_count: u32,
    /// Number of matched executions whose subsequent gate passed.
    pub success_count: u32,
}

/// Observation summary passed to [`ReflexStore::match_observation`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReflexObservation {
    /// Active tool name, if any.
    pub tool: Option<String>,
    /// Tool arguments, if any.
    pub args: Option<String>,
    /// Surrounding context text, if any.
    pub context: Option<String>,
    /// Message type, if any.
    pub message_type: Option<String>,
    /// File extensions visible in the context.
    pub file_exts: Vec<String>,
}

/// Stable T2 decision that may be promoted to a reflex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionCandidate {
    /// Episode that produced this candidate.
    pub episode_id: String,
    /// Condition observed by the episode.
    pub condition: ReflexCondition,
    /// Action selected by the episode.
    pub action: ReflexAction,
}

#[derive(Debug, Default)]
struct StoreState {
    rules: IndexMap<Uuid, ReflexRule>,
    last_fired_by_action: HashMap<ReflexAction, Uuid>,
    dirty: bool,
}

#[derive(Debug)]
struct StoreInner {
    path: PathBuf,
    state: Mutex<StoreState>,
    persist_lock: Mutex<()>,
}

impl Drop for StoreInner {
    fn drop(&mut self) {
        let state = self.state.get_mut();
        if state.dirty
            && let Err(error) = persist_rules(&self.path, &state.rules)
        {
            tracing::warn!(
                path = %self.path.display(),
                %error,
                "failed to persist T0 reflex store while closing"
            );
        }
    }
}

/// Thread-safe JSONL-backed store of [`ReflexRule`] values.
///
/// Clones share the same in-memory rules. Mutations that receive gate feedback
/// are persisted immediately; match counters are also persisted when the last
/// clone is dropped or when [`Self::flush`] is called.
#[derive(Debug, Clone)]
pub struct ReflexStore {
    inner: Arc<StoreInner>,
}

impl ReflexStore {
    /// Open or create the logical reflex store at `path`.
    ///
    /// A missing file produces an empty store without creating it. Unreadable
    /// files are logged and treated as empty, while malformed individual JSONL
    /// records are logged and skipped so valid records remain available.
    #[must_use]
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut rules = match load_from_jsonl(&path) {
            Ok(rules) => rules,
            Err(error) if error.kind() == io::ErrorKind::NotFound => IndexMap::new(),
            Err(error) => {
                tracing::warn!(
                    path = %path.display(),
                    %error,
                    "failed to load T0 reflex store; starting empty"
                );
                IndexMap::new()
            }
        };

        let mut trimmed = false;
        while rules.len() > MAX_RULES {
            if let Some(id) = least_recently_used_id(&rules) {
                rules.shift_remove(&id);
                trimmed = true;
            } else {
                break;
            }
        }
        let dirty = if trimmed {
            if let Err(error) = persist_rules(&path, &rules) {
                tracing::warn!(
                    path = %path.display(),
                    %error,
                    "failed to persist capacity-trimmed T0 reflex store"
                );
                true
            } else {
                false
            }
        } else {
            false
        };

        Self {
            inner: Arc::new(StoreInner {
                path,
                state: Mutex::new(StoreState {
                    rules,
                    dirty,
                    ..StoreState::default()
                }),
                persist_lock: Mutex::new(()),
            }),
        }
    }

    /// Check rules in insertion order and return the first matching action.
    ///
    /// A match increments the rule's hit counter and updates its last-fired
    /// timestamp. This hot-path operation does not perform disk I/O.
    #[must_use]
    pub fn match_observation(&self, observation: &ReflexObservation) -> Option<ReflexAction> {
        self.match_observation_with_id(observation)
            .map(|matched| matched.action)
    }

    /// Return whether any rule matches without updating counters or cloning a rule.
    ///
    /// Runners can use this pure locked scan before reserving scarce execution
    /// capacity, then call [`Self::match_observation_with_id`] only after the
    /// reservation succeeds.
    #[must_use]
    pub fn has_match(&self, observation: &ReflexObservation) -> bool {
        self.inner
            .state
            .lock()
            .rules
            .values()
            .any(|rule| rule.condition.matches(observation))
    }

    /// Check rules in insertion order and return the exact matching rule.
    ///
    /// This is the concurrency-safe form of [`Self::match_observation`]. Its
    /// rule identifier binds later gate feedback to the rule that actually
    /// fired even when several rules produce identical actions.
    #[must_use]
    pub fn match_observation_with_id(
        &self,
        observation: &ReflexObservation,
    ) -> Option<ReflexMatch> {
        let mut state = self.inner.state.lock();
        let matched = state
            .rules
            .iter()
            .find_map(|(id, rule)| rule.condition.matches(observation).then_some(*id));
        let id = matched?;

        let (condition, action) = {
            let rule = state
                .rules
                .get_mut(&id)
                .expect("matched reflex rule must remain in the locked store");
            rule.hit_count = rule.hit_count.saturating_add(1);
            rule.last_fired_at = Some(Utc::now());
            (rule.condition.clone(), rule.action.clone())
        };
        state.last_fired_by_action.insert(action.clone(), id);
        state.dirty = true;
        Some(ReflexMatch {
            rule_id: id,
            condition,
            action,
        })
    }

    /// Record a passing gate for the rule that most recently fired `action`.
    ///
    /// The success count and confidence ratio are updated and the store is
    /// persisted. If no rule has fired yet, the earliest rule with the same
    /// action is used, which also makes explicit administrative feedback
    /// possible.
    pub fn record_gate_pass(&self, action: &ReflexAction) {
        let updated = {
            let mut state = self.inner.state.lock();
            let Some(id) = feedback_rule_id(&mut state, action) else {
                return;
            };
            record_gate_pass_for_id(&mut state, id)
        };
        if updated {
            self.flush_best_effort();
        }
    }

    /// Record a passing gate for an exact rule identifier.
    ///
    /// Unknown or already-evicted rule identifiers are ignored.
    pub fn record_gate_pass_for(&self, rule_id: Uuid) {
        let updated = record_gate_pass_for_id(&mut self.inner.state.lock(), rule_id);
        if updated {
            self.flush_best_effort();
        }
    }

    /// Record a failing gate for the rule that most recently fired `action`.
    ///
    /// Confidence is halved. If it falls below `0.50`, the rule is deleted and
    /// this method returns `true`; otherwise it returns `false`.
    #[must_use]
    pub fn record_gate_fail(&self, action: &ReflexAction) -> bool {
        let deleted = {
            let mut state = self.inner.state.lock();
            let Some(id) = feedback_rule_id(&mut state, action) else {
                return false;
            };
            record_gate_fail_for_id(&mut state, id)
        };
        if deleted.is_some() {
            self.flush_best_effort();
        }
        deleted.unwrap_or(false)
    }

    /// Record a failing gate for an exact rule identifier.
    ///
    /// Returns `true` only when this failure deletes the rule. Unknown or
    /// already-evicted identifiers return `false` without changing the store.
    #[must_use]
    pub fn record_gate_fail_for(&self, rule_id: Uuid) -> bool {
        let deleted = record_gate_fail_for_id(&mut self.inner.state.lock(), rule_id);
        if deleted.is_some() {
            self.flush_best_effort();
        }
        deleted.unwrap_or(false)
    }

    /// Promote a stable T2 decision into the T0 store.
    ///
    /// At least three successful identical fires are required. An identical
    /// condition/action rule is never duplicated. When at capacity, the
    /// least-recently-fired rule is evicted before insertion.
    #[must_use]
    pub fn try_promote(&self, candidate: &PromotionCandidate, fires: u32) -> bool {
        let initial_confidence = 1.0_f64;
        if fires < PROMOTE_MIN_HITS || initial_confidence < PROMOTE_MIN_CONFIDENCE {
            return false;
        }

        {
            let mut state = self.inner.state.lock();
            if state.rules.values().any(|rule| {
                rule.condition == candidate.condition && rule.action == candidate.action
            }) {
                return false;
            }

            if state.rules.len() >= MAX_RULES
                && let Some(id) = least_recently_used_id(&state.rules)
            {
                state.rules.shift_remove(&id);
                state
                    .last_fired_by_action
                    .retain(|_, rule_id| *rule_id != id);
            }

            let rule = ReflexRule {
                id: Uuid::new_v4(),
                condition: candidate.condition.clone(),
                action: candidate.action.clone(),
                confidence: initial_confidence,
                source_episode: candidate.episode_id.clone(),
                promoted_at: Utc::now(),
                last_fired_at: None,
                hit_count: fires,
                success_count: fires,
            };
            state.rules.insert(rule.id, rule);
            state.dirty = true;
        }
        self.flush_best_effort();
        true
    }

    /// Return the number of rules currently in the store.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.state.lock().rules.len()
    }

    /// Return `true` when the store contains no rules.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return all rules ordered by descending hit count.
    ///
    /// Rules with equal hit counts retain their insertion order.
    #[must_use]
    pub fn snapshot(&self) -> Vec<ReflexRule> {
        let state = self.inner.state.lock();
        let mut rules: Vec<_> = state.rules.values().cloned().collect();
        rules.sort_by_key(|rule| std::cmp::Reverse(rule.hit_count));
        rules
    }

    /// Persist all in-memory rule changes immediately.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if serialization, directory creation, writing,
    /// syncing, or atomic publication fails.
    pub fn flush(&self) -> io::Result<()> {
        let _persist_guard = self.inner.persist_lock.lock();
        let rules = {
            let mut state = self.inner.state.lock();
            if !state.dirty {
                return Ok(());
            }
            let rules = state.rules.clone();
            state.dirty = false;
            rules
        };
        if let Err(error) = persist_rules(&self.inner.path, &rules) {
            self.inner.state.lock().dirty = true;
            return Err(error);
        }
        Ok(())
    }

    fn flush_best_effort(&self) {
        if let Err(error) = self.flush() {
            tracing::warn!(
                path = %self.inner.path.display(),
                %error,
                "failed to persist T0 reflex store"
            );
        }
    }
}

fn feedback_rule_id(state: &mut StoreState, action: &ReflexAction) -> Option<Uuid> {
    state.last_fired_by_action.remove(action).or_else(|| {
        state
            .rules
            .iter()
            .filter(|(_, rule)| rule.action == *action)
            .reduce(|current, candidate| {
                if candidate.1.last_fired_at > current.1.last_fired_at {
                    candidate
                } else {
                    current
                }
            })
            .map(|(id, _)| *id)
    })
}

fn record_gate_pass_for_id(state: &mut StoreState, id: Uuid) -> bool {
    let Some(rule) = state.rules.get_mut(&id) else {
        return false;
    };
    rule.success_count = rule
        .success_count
        .saturating_add(1)
        .min(rule.hit_count.max(1));
    rule.confidence =
        (f64::from(rule.success_count) / f64::from(rule.hit_count.max(1))).clamp(0.0, 1.0);
    state
        .last_fired_by_action
        .retain(|_, rule_id| *rule_id != id);
    state.dirty = true;
    true
}

fn record_gate_fail_for_id(state: &mut StoreState, id: Uuid) -> Option<bool> {
    let Some(rule) = state.rules.get_mut(&id) else {
        return None;
    };
    rule.confidence = (rule.confidence * DEMOTE_MULTIPLIER).clamp(0.0, 1.0);
    let should_delete = rule.confidence < DEMOTE_DELETE_THRESHOLD;

    if should_delete {
        state.rules.shift_remove(&id);
    }
    state
        .last_fired_by_action
        .retain(|_, rule_id| *rule_id != id);
    state.dirty = true;
    Some(should_delete)
}

fn persist_rules(path: &Path, rules: &IndexMap<Uuid, ReflexRule>) -> io::Result<()> {
    let mut bytes = Vec::new();
    for rule in rules.values() {
        serde_json::to_writer(&mut bytes, rule)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        bytes.push(b'\n');
    }
    roko_fs::atomic_write_bytes(path, &bytes)
}

fn load_from_jsonl(path: &Path) -> io::Result<IndexMap<Uuid, ReflexRule>> {
    use std::io::BufRead as _;

    let file = std::fs::File::open(path)?;
    let mut rules = IndexMap::new();
    for (line_number, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<ReflexRule>(line) {
            Ok(rule) => {
                rules.insert(rule.id, rule);
            }
            Err(error) => tracing::warn!(
                path = %path.display(),
                line = line_number + 1,
                %error,
                "skipping malformed T0 reflex record"
            ),
        }
    }
    Ok(rules)
}

fn least_recently_used_id(rules: &IndexMap<Uuid, ReflexRule>) -> Option<Uuid> {
    rules
        .iter()
        .min_by_key(|(_, rule)| {
            (
                rule.last_fired_at.is_some(),
                rule.last_fired_at.unwrap_or(rule.promoted_at),
                rule.promoted_at,
            )
        })
        .map(|(id, _)| *id)
}

fn condition_matches(condition: &ReflexCondition, observation: &ReflexObservation) -> bool {
    condition
        .tool
        .as_deref()
        .is_none_or(|tool| observation.tool.as_deref() == Some(tool))
        && condition.args_pattern.as_deref().is_none_or(|pattern| {
            observation
                .args
                .as_deref()
                .is_some_and(|args| args.contains(pattern))
        })
        && condition.context.as_deref().is_none_or(|expected| {
            observation
                .context
                .as_deref()
                .is_some_and(|context| context.contains(expected))
        })
        && condition
            .message_type
            .as_deref()
            .is_none_or(|kind| observation.message_type.as_deref() == Some(kind))
        && condition.file_ext.as_deref().is_none_or(|extension| {
            observation
                .file_exts
                .iter()
                .any(|observed| observed == extension)
        })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use tempfile::TempDir;

    fn condition(label: &str) -> ReflexCondition {
        ReflexCondition {
            tool: Some("bash".to_owned()),
            args_pattern: Some(label.to_owned()),
            context: Some("edited source".to_owned()),
            message_type: Some("tool_result".to_owned()),
            file_ext: Some(".rs".to_owned()),
        }
    }

    fn action(label: &str) -> ReflexAction {
        ReflexAction {
            tool: "bash".to_owned(),
            args: format!("cargo test -p {label}"),
        }
    }

    fn candidate(label: &str) -> PromotionCandidate {
        PromotionCandidate {
            episode_id: format!("episode-{label}"),
            condition: condition(label),
            action: action(label),
        }
    }

    fn observation(label: &str) -> ReflexObservation {
        ReflexObservation {
            tool: Some("bash".to_owned()),
            args: Some(format!("prefix {label} suffix")),
            context: Some("recently edited source files".to_owned()),
            message_type: Some("tool_result".to_owned()),
            file_exts: vec![".toml".to_owned(), ".rs".to_owned()],
        }
    }

    #[test]
    fn promotion_requires_three_fires_and_deduplicates_rules() {
        let directory = TempDir::new().expect("temporary directory");
        let path = directory.path().join("nested/reflexes.jsonl");
        let store = ReflexStore::open(&path);
        let candidate = candidate("learn");

        assert!(!store.try_promote(&candidate, 2));
        assert!(store.try_promote(&candidate, 3));
        assert!(!store.try_promote(&candidate, 4));
        assert_eq!(store.len(), 1);

        let rule = store.snapshot().pop().expect("promoted rule");
        assert_eq!(rule.confidence, 1.0);
        assert_eq!(rule.hit_count, 3);
        assert_eq!(rule.success_count, 3);
        assert_eq!(rule.source_episode, "episode-learn");

        let persisted = std::fs::read_to_string(path).expect("persisted JSONL");
        assert_eq!(persisted.lines().count(), 1);
        assert_eq!(
            serde_json::from_str::<ReflexRule>(persisted.trim()).expect("valid rule"),
            rule
        );
    }

    #[test]
    fn matching_requires_every_populated_field_and_uses_insertion_order() {
        let directory = TempDir::new().expect("temporary directory");
        let store = ReflexStore::open(directory.path().join("reflexes.jsonl"));
        let first = candidate("test");
        let mut second = candidate("test");
        second.episode_id = "episode-second".to_owned();
        second.condition.context = None;
        second.action.args = "fallback".to_owned();
        assert!(store.try_promote(&first, 3));
        assert!(store.try_promote(&second, 3));

        assert_eq!(
            store.match_observation(&observation("test")),
            Some(first.action)
        );

        let mut mismatch = observation("test");
        mismatch.message_type = Some("user".to_owned());
        assert_eq!(store.match_observation(&mismatch), None);
        mismatch.message_type = Some("tool_result".to_owned());
        mismatch.file_exts.clear();
        assert_eq!(store.match_observation(&mismatch), None);

        let snapshot = store.snapshot();
        let fired = snapshot
            .iter()
            .find(|rule| rule.source_episode == "episode-test")
            .expect("first rule");
        assert_eq!(fired.hit_count, 4);
        assert!(fired.last_fired_at.is_some());
    }

    #[test]
    fn wildcard_condition_matches_an_empty_observation() {
        let directory = TempDir::new().expect("temporary directory");
        let store = ReflexStore::open(directory.path().join("reflexes.jsonl"));
        let mut candidate = candidate("wildcard");
        candidate.condition = ReflexCondition::default();
        assert!(store.try_promote(&candidate, 3));

        assert_eq!(
            store.match_observation(&ReflexObservation::default()),
            Some(candidate.action)
        );
    }

    #[test]
    fn id_specific_feedback_targets_the_rule_that_fired() {
        let directory = TempDir::new().expect("temporary directory");
        let store = ReflexStore::open(directory.path().join("reflexes.jsonl"));
        let first = candidate("first");
        let mut second = candidate("second");
        second.action = first.action.clone();
        assert!(store.try_promote(&first, 3));
        assert!(store.try_promote(&second, 3));

        let matched = store
            .match_observation_with_id(&observation("second"))
            .expect("second rule should match");
        assert_eq!(matched.action, second.action);
        assert!(!store.record_gate_fail_for(matched.rule_id));

        let rules = store.snapshot();
        assert_eq!(
            rules
                .iter()
                .find(|rule| rule.source_episode == "episode-first")
                .expect("first rule")
                .confidence,
            1.0
        );
        assert_eq!(
            rules
                .iter()
                .find(|rule| rule.source_episode == "episode-second")
                .expect("second rule")
                .confidence,
            0.5
        );
    }

    #[test]
    fn gate_pass_updates_confidence_and_persists_match_counters() {
        let directory = TempDir::new().expect("temporary directory");
        let path = directory.path().join("reflexes.jsonl");
        let store = ReflexStore::open(&path);
        let candidate = candidate("pass");
        assert!(store.try_promote(&candidate, 3));
        assert_eq!(
            store.match_observation(&observation("pass")),
            Some(candidate.action.clone())
        );

        store.record_gate_pass(&candidate.action);
        drop(store);

        let rule = ReflexStore::open(path)
            .snapshot()
            .pop()
            .expect("reloaded rule");
        assert_eq!(rule.hit_count, 4);
        assert_eq!(rule.success_count, 4);
        assert_eq!(rule.confidence, 1.0);
        assert!(rule.last_fired_at.is_some());
    }

    #[test]
    fn two_gate_failures_demote_then_delete_a_rule() {
        let directory = TempDir::new().expect("temporary directory");
        let store = ReflexStore::open(directory.path().join("reflexes.jsonl"));
        let candidate = candidate("fail");
        assert!(store.try_promote(&candidate, 3));

        assert!(!store.record_gate_fail(&candidate.action));
        assert_eq!(store.snapshot()[0].confidence, 0.5);
        assert!(store.record_gate_fail(&candidate.action));
        assert!(store.is_empty());
    }

    #[test]
    fn rules_survive_restart_with_all_fields_intact() {
        let directory = TempDir::new().expect("temporary directory");
        let path = directory.path().join("reflexes.jsonl");
        let candidate = candidate("restart");
        let expected = {
            let store = ReflexStore::open(&path);
            assert!(store.try_promote(&candidate, 7));
            store.snapshot().pop().expect("rule")
        };

        let reopened = ReflexStore::open(path);
        assert_eq!(reopened.len(), 1);
        assert_eq!(reopened.snapshot(), vec![expected]);
    }

    #[test]
    fn malformed_records_are_skipped_without_losing_valid_rules() {
        let directory = TempDir::new().expect("temporary directory");
        let path = directory.path().join("reflexes.jsonl");
        let valid = ReflexRule {
            id: Uuid::new_v4(),
            condition: condition("valid"),
            action: action("valid"),
            confidence: 1.0,
            source_episode: "episode-valid".to_owned(),
            promoted_at: Utc::now(),
            last_fired_at: None,
            hit_count: 3,
            success_count: 3,
        };
        let text = format!(
            "not json\n{}\n{{\"truncated\":\n",
            serde_json::to_string(&valid).expect("serialize")
        );
        std::fs::write(&path, text).expect("fixture");

        assert_eq!(ReflexStore::open(path).snapshot(), vec![valid]);
    }

    #[test]
    fn capacity_evicts_the_least_recently_used_rule() {
        let directory = TempDir::new().expect("temporary directory");
        let store = ReflexStore::open(directory.path().join("reflexes.jsonl"));
        let mut oldest_id = None;
        {
            let mut state = store.inner.state.lock();
            let now = Utc::now();
            for index in 0..MAX_RULES {
                let id = Uuid::new_v4();
                if index == 0 {
                    oldest_id = Some(id);
                }
                state.rules.insert(
                    id,
                    ReflexRule {
                        id,
                        condition: condition(&format!("existing-{index}")),
                        action: action(&format!("existing-{index}")),
                        confidence: 1.0,
                        source_episode: format!("episode-{index}"),
                        promoted_at: now + chrono::Duration::seconds(index as i64),
                        last_fired_at: Some(now + chrono::Duration::seconds(index as i64)),
                        hit_count: 3,
                        success_count: 3,
                    },
                );
            }
        }

        let replacement = candidate("replacement");
        assert!(store.try_promote(&replacement, 3));
        let rules = store.snapshot();
        assert_eq!(rules.len(), MAX_RULES);
        let oldest_id = oldest_id.expect("at least one fixture rule");
        assert!(rules.iter().all(|rule| rule.id != oldest_id));
        assert!(
            rules
                .iter()
                .any(|rule| rule.source_episode == "episode-replacement")
        );
    }

    #[test]
    fn matching_two_hundred_rules_averages_under_one_hundred_microseconds() {
        let directory = TempDir::new().expect("temporary directory");
        let store = ReflexStore::open(directory.path().join("reflexes.jsonl"));
        {
            let mut state = store.inner.state.lock();
            for index in 0..MAX_RULES {
                let id = Uuid::new_v4();
                let label = format!("rule-{index:03}-only");
                state.rules.insert(
                    id,
                    ReflexRule {
                        id,
                        condition: condition(&label),
                        action: action(&label),
                        confidence: 1.0,
                        source_episode: format!("episode-{index}"),
                        promoted_at: Utc::now(),
                        last_fired_at: None,
                        hit_count: 3,
                        success_count: 3,
                    },
                );
            }
        }
        let observation = observation("rule-199-only");
        assert!(store.match_observation(&observation).is_some());

        const ITERATIONS: u32 = 2_000;
        let started = Instant::now();
        for _ in 0..ITERATIONS {
            assert!(store.match_observation(&observation).is_some());
        }
        let average = started.elapsed() / ITERATIONS;
        assert!(
            average < Duration::from_micros(100),
            "200-rule lookup averaged {average:?}"
        );
    }
}
