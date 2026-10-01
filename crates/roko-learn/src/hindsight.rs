//! Append-only hindsight relabeling for recent episode outcomes.

use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::episode_logger::{Episode, EpisodeLogger, LEARNING_LABEL_KEY, LoggerError};
use crate::playbook_rules::Rule;

/// Episode `extra` key listing the tasks a failed gate blamed, as
/// `"{plan_id}/{task_id}"` keys.
pub const BLAMED_TASKS_KEY: &str = "blamed_tasks";

/// Default file name for durable adjustments under `.roko/learn/`.
pub const DEFAULT_ADJUSTMENTS_FILE: &str = "episode-adjustments.jsonl";

/// Episode `extra` key holding the hindsight correction applied to the
/// episode as it was read ([`apply_adjustments`]).
pub const HINDSIGHT_ADJUSTMENT_KEY: &str = "hindsight_adjustment";

/// Why a previous episode assessment changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjustmentKind {
    /// A later gate regression invalidated an earlier success.
    Regression,
    /// A later successful playbook reused an approach from a failed episode.
    SuccessfulReuse,
    /// A rule sourced from the episode was subsequently contradicted.
    HeuristicFalsified,
}

/// An immutable correction referring back to an original episode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodeAdjustment {
    /// Episode being corrected.
    pub original_episode_id: String,
    /// Correction category.
    pub adjustment_kind: AdjustmentKind,
    /// Original outcome or confidence value.
    pub old_value: Value,
    /// Corrected outcome or confidence value.
    pub new_value: Value,
    /// Auditable explanation.
    pub reason: String,
    /// Time the correction was inferred.
    pub timestamp: DateTime<Utc>,
}

/// Scans a bounded recent window and produces append-only corrections.
#[derive(Debug, Clone)]
pub struct HindsightRelabeler {
    max_age: Duration,
    attributed_only: bool,
}

impl Default for HindsightRelabeler {
    fn default() -> Self {
        Self {
            max_age: Duration::days(30),
            attributed_only: false,
        }
    }
}

impl HindsightRelabeler {
    /// Create a relabeler with the mandatory 30-day staleness limit.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Relabel a success as a regression only when a later failed gate
    /// blames its task by name ([`BLAMED_TASKS_KEY`]), and only the task's
    /// latest episode before that failure.
    ///
    /// Where every episode is a fresh attempt that edits files, a later
    /// failure of the same task, or of a task declaring the same files, does
    /// not show that the earlier work regressed; the new attempt's own edits
    /// explain it just as well.
    #[must_use]
    pub fn attributed_only(mut self) -> Self {
        self.attributed_only = true;
        self
    }

    /// Cross-reference episode outcomes and current rule evidence.
    ///
    /// `episodes` must be in write order. Episodes of different plans
    /// (`extra.plan_id`) never relabel each other.
    #[must_use]
    pub fn scan(&self, episodes: &[Episode], rules: &[Rule]) -> Vec<EpisodeAdjustment> {
        let cutoff = Utc::now() - self.max_age;
        let mut adjustments = Vec::new();

        for (index, episode) in episodes.iter().enumerate() {
            if episode.timestamp < cutoff || episode.kind == "episode_adjustment" {
                continue;
            }

            // Only learning labels count (S01 §4.1): an unverified success
            // or a provider failure is neither relabeled nor evidence.
            let learned = episode.learning_success();
            if learned == Some(true) {
                let files = episode_files(episode);
                let later_start = index.saturating_add(1);
                let regression = episodes[later_start..]
                    .iter()
                    .enumerate()
                    .find(|(offset, later)| {
                        if later.learning_success() != Some(false)
                            || later.timestamp < episode.timestamp
                            || !same_plan(episode, later)
                            || !later.gate_verdicts.iter().any(|verdict| !verdict.passed)
                        {
                            return false;
                        }
                        let attributed = blames(later, episode)
                            && is_latest_before(episodes, index, later_start + offset);
                        attributed
                            || (!self.attributed_only
                                && (!shared_files(&files, &episode_files(later)).is_empty()
                                    || later.task_id == episode.task_id))
                    })
                    .map(|(_, later)| later);
                if let Some(later) = regression {
                    let reason = if blames(later, episode) {
                        format!(
                            "later gate failure in episode {} was attributed to this task's files",
                            later.id
                        )
                    } else {
                        format!(
                            "later gate failure in episode {} affected the same task or files",
                            later.id
                        )
                    };
                    adjustments.push(EpisodeAdjustment {
                        original_episode_id: episode.id.clone(),
                        adjustment_kind: AdjustmentKind::Regression,
                        old_value: Value::Bool(true),
                        new_value: Value::Bool(false),
                        reason,
                        timestamp: Utc::now(),
                    });
                    continue;
                }
            } else if learned == Some(false)
                && episodes[index.saturating_add(1)..].iter().any(|later| {
                    later.learning_success() == Some(true)
                        && later.timestamp >= episode.timestamp
                        && reused_episode(later, &episode.id)
                })
            {
                adjustments.push(EpisodeAdjustment {
                    original_episode_id: episode.id.clone(),
                    adjustment_kind: AdjustmentKind::SuccessfulReuse,
                    old_value: Value::Bool(false),
                    new_value: Value::Bool(true),
                    reason: "a later successful playbook reused this episode's approach".into(),
                    timestamp: Utc::now(),
                });
                continue;
            }

            if let Some(rule) = rules.iter().find(|rule| {
                rule.contradictions > 0 && rule.source_episodes.iter().any(|id| id == &episode.id)
            }) {
                let old = rule.confidence.clamp(0.0, 1.0);
                adjustments.push(EpisodeAdjustment {
                    original_episode_id: episode.id.clone(),
                    adjustment_kind: AdjustmentKind::HeuristicFalsified,
                    old_value: Value::from(old),
                    new_value: Value::from((old - 0.1).max(0.0)),
                    reason: format!("playbook rule {} was contradicted", rule.rule_id),
                    timestamp: Utc::now(),
                });
            }
        }
        adjustments
    }

    /// Append adjustments to the same episode log as typed extension records.
    ///
    /// # Errors
    ///
    /// Returns the first logger or serialization error.
    pub async fn append(
        &self,
        logger: &EpisodeLogger,
        adjustments: &[EpisodeAdjustment],
    ) -> Result<(), LoggerError> {
        for adjustment in adjustments {
            let mut record = Episode::new("hindsight", &adjustment.original_episode_id);
            record.kind = "episode_adjustment".into();
            record.timestamp = adjustment.timestamp;
            record.extra.insert(
                "adjustment".into(),
                serde_json::to_value(adjustment).unwrap_or(Value::Null),
            );
            logger.append(&record).await?;
        }
        Ok(())
    }
}

/// Read the adjustments recorded at `path`, in write order.
///
/// A missing file reads as empty; malformed lines are skipped.
///
/// # Errors
///
/// Returns an error if the file exists but cannot be read.
pub fn read_adjustments(path: &Path) -> io::Result<Vec<EpisodeAdjustment>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    Ok(text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect())
}

/// Append to the JSONL log at `path` each adjustment it does not already
/// hold for the same original episode and kind, so repeated scans of one
/// history record every correction once. Returns the adjustments appended,
/// so a caller can act on each correction once.
///
/// Unlike [`HindsightRelabeler::append`], the log is kept apart from the
/// episode log, whose readers would count correction records as episodes.
/// Callers serialize concurrent appends to one path.
///
/// # Errors
///
/// Returns an error if the log cannot be read or written.
pub fn append_new_adjustments(
    path: &Path,
    adjustments: &[EpisodeAdjustment],
) -> io::Result<Vec<EpisodeAdjustment>> {
    if adjustments.is_empty() {
        return Ok(Vec::new());
    }
    let mut recorded: HashSet<(String, AdjustmentKind)> = read_adjustments(path)?
        .into_iter()
        .map(|adjustment| (adjustment.original_episode_id, adjustment.adjustment_kind))
        .collect();
    let mut lines = String::new();
    let mut appended = Vec::new();
    for adjustment in adjustments {
        if !recorded.insert((
            adjustment.original_episode_id.clone(),
            adjustment.adjustment_kind,
        )) {
            continue;
        }
        lines.push_str(&serde_json::to_string(adjustment).map_err(io::Error::other)?);
        lines.push('\n');
        appended.push(adjustment.clone());
    }
    if appended.is_empty() {
        return Ok(appended);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(lines.as_bytes())?;
    file.flush()?;
    Ok(appended)
}

/// Apply recorded corrections to `episodes`, matched by episode id, and
/// return how many episodes changed.
///
/// A [`AdjustmentKind::Regression`] marks its episode failed: `success` and
/// any learning label become a failure, and the correction is kept under
/// [`HINDSIGHT_ADJUSTMENT_KEY`]. The other kinds are audit records and
/// change nothing: a later reuse of a failed attempt's approach does not
/// make that attempt pass. The log is never rewritten; readers apply the
/// corrections as they load episodes.
pub fn apply_adjustments(episodes: &mut [Episode], adjustments: &[EpisodeAdjustment]) -> usize {
    let regressions: HashMap<&str, &EpisodeAdjustment> = adjustments
        .iter()
        .filter(|adjustment| adjustment.adjustment_kind == AdjustmentKind::Regression)
        .map(|adjustment| (adjustment.original_episode_id.as_str(), adjustment))
        .collect();
    if regressions.is_empty() {
        return 0;
    }
    let mut applied = 0;
    for episode in episodes {
        let Some(adjustment) = regressions.get(episode.id.as_str()) else {
            continue;
        };
        episode.success = false;
        if episode.extra.contains_key(LEARNING_LABEL_KEY) {
            episode
                .extra
                .insert(LEARNING_LABEL_KEY.to_string(), Value::from(0));
        }
        episode.extra.insert(
            HINDSIGHT_ADJUSTMENT_KEY.to_string(),
            serde_json::json!({
                "kind": adjustment.adjustment_kind,
                "reason": adjustment.reason,
                "timestamp": adjustment.timestamp,
            }),
        );
        applied += 1;
    }
    applied
}

/// The adjustments log of the workspace at `workdir`:
/// `.roko/learn/episode-adjustments.jsonl`.
#[must_use]
pub fn workspace_adjustments_path(workdir: &Path) -> PathBuf {
    roko_fs::RokoLayout::for_project(workdir)
        .learn_dir()
        .join(DEFAULT_ADJUSTMENTS_FILE)
}

/// [`apply_adjustments`] with the corrections recorded for the workspace at
/// `workdir`. An unreadable log applies none.
pub fn apply_workspace_adjustments(episodes: &mut [Episode], workdir: &Path) -> usize {
    apply_adjustments_from(episodes, &workspace_adjustments_path(workdir))
}

/// [`apply_adjustments`] with the corrections recorded in the log at `path`.
/// An unreadable log applies none.
pub fn apply_adjustments_from(episodes: &mut [Episode], path: &Path) -> usize {
    match read_adjustments(path) {
        Ok(adjustments) => apply_adjustments(episodes, &adjustments),
        Err(error) => {
            tracing::warn!(
                path = %path.display(),
                %error,
                "hindsight adjustments unreadable; episodes read as logged"
            );
            0
        }
    }
}

fn episode_files(episode: &Episode) -> HashSet<String> {
    episode
        .extra
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn shared_files(left: &HashSet<String>, right: &HashSet<String>) -> Vec<String> {
    left.intersection(right).cloned().collect()
}

fn episode_plan(episode: &Episode) -> Option<&str> {
    episode.extra.get("plan_id").and_then(Value::as_str)
}

/// Whether two episodes may relabel each other: unscoped episodes match
/// anything, scoped ones only their own plan.
fn same_plan(left: &Episode, right: &Episode) -> bool {
    match (episode_plan(left), episode_plan(right)) {
        (Some(left), Some(right)) => left == right,
        _ => true,
    }
}

/// Whether `failure` blames `episode`'s task in [`BLAMED_TASKS_KEY`].
fn blames(failure: &Episode, episode: &Episode) -> bool {
    let key = match episode_plan(episode) {
        Some(plan) => format!("{plan}/{}", episode.task_id),
        None => episode.task_id.clone(),
    };
    failure
        .extra
        .get(BLAMED_TASKS_KEY)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|blamed| blamed == key)
}

/// Whether no other episode of `episodes[original]`'s task (in its plan)
/// lies between it and `episodes[later]`.
fn is_latest_before(episodes: &[Episode], original: usize, later: usize) -> bool {
    let episode = &episodes[original];
    !episodes[original.saturating_add(1)..later]
        .iter()
        .any(|between| {
            between.kind != "episode_adjustment"
                && between.task_id == episode.task_id
                && same_plan(episode, between)
        })
}

fn reused_episode(episode: &Episode, source_id: &str) -> bool {
    episode
        .extra
        .get("source_episode_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|id| id == source_id)
        || episode
            .extra
            .get("reused_episode_id")
            .and_then(Value::as_str)
            == Some(source_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episode_logger::EpisodeGateVerdict;

    #[test]
    fn hindsight_relabels_a_later_regression() {
        let mut original = Episode::new("a", "task");
        original.success = true;
        original
            .extra
            .insert("files".into(), serde_json::json!(["src/lib.rs"]));
        let mut regression = Episode::new("b", "other");
        regression.timestamp = original.timestamp + Duration::minutes(1);
        regression.success = false;
        regression
            .extra
            .insert("files".into(), serde_json::json!(["src/lib.rs"]));
        regression.gate_verdicts = vec![EpisodeGateVerdict::new("test", false)];

        let found = HindsightRelabeler::new().scan(&[original.clone(), regression], &[]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].original_episode_id, original.id);
        assert_eq!(found[0].adjustment_kind, AdjustmentKind::Regression);
    }

    /// An episode of `task` in `plan`, `minutes` after `base`.
    fn plan_episode(plan: &str, task: &str, success: bool, minutes: i64) -> Episode {
        let mut episode = Episode::new(task, task);
        episode.id = format!("ep-{plan}-{task}-{minutes}");
        episode.timestamp = Utc::now() - Duration::hours(1) + Duration::minutes(minutes);
        episode.success = success;
        episode
            .extra
            .insert("plan_id".into(), Value::String(plan.into()));
        if !success {
            episode.gate_verdicts = vec![EpisodeGateVerdict::new("verify", false)];
        }
        episode
    }

    fn blaming(mut episode: Episode, blamed: &[&str]) -> Episode {
        episode
            .extra
            .insert(BLAMED_TASKS_KEY.into(), serde_json::json!(blamed));
        episode
    }

    #[test]
    fn attributed_failure_relabels_the_blamed_tasks_latest_success() {
        let older = plan_episode("p", "T12", true, 0);
        let latest = plan_episode("p", "T12", true, 5);
        let failure = blaming(plan_episode("p", "T2", false, 6), &["p/T12"]);

        let found = HindsightRelabeler::new()
            .attributed_only()
            .scan(&[older, latest.clone(), failure.clone()], &[]);

        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].original_episode_id, latest.id);
        assert_eq!(found[0].adjustment_kind, AdjustmentKind::Regression);
        assert_eq!(found[0].old_value, Value::Bool(true));
        assert_eq!(found[0].new_value, Value::Bool(false));
        assert!(found[0].reason.contains(&failure.id), "{}", found[0].reason);
    }

    #[test]
    fn attributed_only_ignores_same_task_file_and_other_plan_overlaps() {
        let success = plan_episode("p", "T1", true, 0);
        // A new attempt of the same task failing proves nothing about the old one.
        let same_task = plan_episode("p", "T1", false, 1);
        let mut shared = plan_episode("p", "T3", false, 2);
        shared
            .extra
            .insert("files".into(), serde_json::json!(["src/lib.rs"]));
        // Blame of a same-named task in another plan does not count either.
        let other_plan = blaming(plan_episode("q", "T2", false, 3), &["q/T1"]);
        let mut success_with_files = success.clone();
        success_with_files
            .extra
            .insert("files".into(), serde_json::json!(["src/lib.rs"]));

        let relabeler = HindsightRelabeler::new().attributed_only();
        assert!(
            relabeler
                .scan(&[success_with_files, same_task, shared, other_plan], &[])
                .is_empty()
        );
    }

    #[test]
    fn a_failed_retry_between_success_and_blame_supersedes_the_success() {
        let success = plan_episode("p", "T12", true, 0);
        let failed_retry = plan_episode("p", "T12", false, 1);
        let blame = blaming(plan_episode("p", "T2", false, 2), &["p/T12"]);

        let found = HindsightRelabeler::new()
            .attributed_only()
            .scan(&[success, failed_retry, blame], &[]);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_regression_relabels_the_episode_it_names_on_read() {
        let mut labelled = plan_episode("p", "T12", true, 0);
        labelled
            .extra
            .insert(LEARNING_LABEL_KEY.into(), Value::from(1));
        let other = plan_episode("p", "T3", true, 1);
        let blame = blaming(plan_episode("p", "T2", false, 2), &["p/T12"]);
        let adjustments = HindsightRelabeler::new()
            .attributed_only()
            .scan(&[labelled.clone(), other.clone(), blame], &[]);

        let mut episodes = vec![labelled, other];
        assert_eq!(apply_adjustments(&mut episodes, &adjustments), 1);
        assert!(!episodes[0].success);
        assert_eq!(episodes[0].learning_success(), Some(false));
        assert!(episodes[0].extra.contains_key(HINDSIGHT_ADJUSTMENT_KEY));
        assert!(episodes[1].success);
    }

    #[test]
    fn durable_adjustments_are_recorded_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("learn").join(DEFAULT_ADJUSTMENTS_FILE);
        let success = plan_episode("p", "T12", true, 0);
        let blame = blaming(plan_episode("p", "T2", false, 1), &["p/T12"]);
        let relabeler = HindsightRelabeler::new().attributed_only();

        let first = relabeler.scan(&[success.clone(), blame.clone()], &[]);
        let appended = append_new_adjustments(&path, &first).expect("append");
        assert_eq!(appended, first);
        let again = relabeler.scan(&[success.clone(), blame], &[]);
        let appended_again = append_new_adjustments(&path, &again).expect("append");
        assert!(appended_again.is_empty(), "{appended_again:?}");

        let recorded = read_adjustments(&path).expect("read");
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].original_episode_id, success.id);
        assert!(
            read_adjustments(&dir.path().join("missing.jsonl"))
                .expect("read")
                .is_empty()
        );
    }
}
