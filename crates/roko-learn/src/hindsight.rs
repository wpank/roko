//! Append-only hindsight relabeling for recent episode outcomes.

use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::episode_logger::{Episode, EpisodeLogger, LoggerError};
use crate::playbook_rules::Rule;

/// Why a previous episode assessment changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl Default for HindsightRelabeler {
    fn default() -> Self {
        Self {
            max_age: Duration::days(30),
        }
    }
}

impl HindsightRelabeler {
    /// Create a relabeler with the mandatory 30-day staleness limit.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Cross-reference episode outcomes and current rule evidence.
    #[must_use]
    pub fn scan(&self, episodes: &[Episode], rules: &[Rule]) -> Vec<EpisodeAdjustment> {
        let cutoff = Utc::now() - self.max_age;
        let mut adjustments = Vec::new();

        for (index, episode) in episodes.iter().enumerate() {
            if episode.timestamp < cutoff || episode.kind == "episode_adjustment" {
                continue;
            }

            if episode.success {
                let files = episode_files(episode);
                let regression = episodes[index.saturating_add(1)..].iter().find(|later| {
                    !later.success
                        && later.timestamp >= episode.timestamp
                        && (!shared_files(&files, &episode_files(later)).is_empty()
                            || later.task_id == episode.task_id)
                        && later.gate_verdicts.iter().any(|verdict| !verdict.passed)
                });
                if let Some(later) = regression {
                    adjustments.push(EpisodeAdjustment {
                        original_episode_id: episode.id.clone(),
                        adjustment_kind: AdjustmentKind::Regression,
                        old_value: Value::Bool(true),
                        new_value: Value::Bool(false),
                        reason: format!(
                            "later gate failure in episode {} affected the same task or files",
                            later.id
                        ),
                        timestamp: Utc::now(),
                    });
                    continue;
                }
            } else if episodes[index.saturating_add(1)..].iter().any(|later| {
                later.success
                    && later.timestamp >= episode.timestamp
                    && reused_episode(later, &episode.id)
            }) {
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
}
