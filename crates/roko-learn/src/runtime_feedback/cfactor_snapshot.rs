//! C-Factor snapshot computation.
//!
//! Reads episode, context-attribution, and knowledge-confirmation records to
//! compute the social perceptiveness, knowledge integration rate, and
//! convergence velocity sub-scores that feed the composite C-Factor.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::cfactor::{CFactor, compute_cfactor};
use crate::episode_logger::{Episode, EpisodeLogger};

use super::episode_helpers::{episode_agent_label, episode_source_id};
use super::persistence::append_cfactor_snapshot;
use super::records::{LearningPaths, LearningRuntimeError};

// ── Public entry point ────────────────────────────────────────────────

/// Compute the current C-Factor snapshot for `learn_root` and append it to the
/// history log.
///
/// Returns the snapshot that was persisted.
///
/// # Errors
///
/// Returns an error if the snapshot cannot be computed or if the history log
/// cannot be updated.
pub async fn refresh_cfactor_snapshot(
    learn_root: impl AsRef<Path>,
) -> Result<CFactor, LearningRuntimeError> {
    let learn_root = learn_root.as_ref();
    let paths = LearningPaths::for_runtime_root(learn_root.to_path_buf());
    let snapshot = compute_cfactor_snapshot(learn_root).await?;
    append_cfactor_snapshot(&paths.cfactor_jsonl, &snapshot).await?;
    Ok(snapshot)
}

// ── Internal computation ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct ContextAttributionRecord {
    #[serde(default = "default_now")]
    pub(crate) ts: DateTime<Utc>,
    #[serde(default)]
    pub(crate) source_type: String,
    #[serde(default)]
    pub(crate) referenced: bool,
}

pub(crate) async fn compute_cfactor_snapshot(
    learn_root: &Path,
) -> Result<CFactor, LearningRuntimeError> {
    let paths = LearningPaths::for_runtime_root(learn_root.to_path_buf());
    let episodes = EpisodeLogger::read_all_lossy(&paths.episodes_jsonl).await?;
    let attribution_path = learn_root
        .parent()
        .unwrap_or(learn_root)
        .join("context-attribution.jsonl");
    let knowledge_path = learn_root
        .parent()
        .unwrap_or(learn_root)
        .join("neuro")
        .join("knowledge.jsonl");
    // Dedicated confirmation records emitted by KnowledgeStore on ingest.
    let confirmations_path = learn_root
        .parent()
        .unwrap_or(learn_root)
        .join("neuro")
        .join("knowledge-confirmations.jsonl");
    let attribution_records = read_context_attribution_records(&attribution_path).await?;
    // Read from both legacy knowledge entries and the dedicated
    // confirmation records file, then merge.
    let mut knowledge_records = read_knowledge_records(&knowledge_path).await?;
    let confirmation_records = read_knowledge_records(&confirmations_path).await?;
    knowledge_records.extend(confirmation_records);
    let social_perceptiveness =
        social_perceptiveness_from_attribution(&attribution_records, Duration::from_hours(168));
    let knowledge_integration_rate =
        knowledge_integration_rate(&knowledge_records, &episodes, Duration::from_hours(168));
    let convergence_velocity = convergence_velocity_from_agreement(
        &knowledge_records,
        &episodes,
        Duration::from_hours(168),
    );
    Ok(compute_cfactor(
        &episodes,
        Duration::from_hours(168),
        social_perceptiveness,
        knowledge_integration_rate,
        convergence_velocity,
    ))
}

// ── Context attribution IO ────────────────────────────────────────────

async fn read_context_attribution_records(
    path: &Path,
) -> Result<Vec<ContextAttributionRecord>, LearningRuntimeError> {
    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(LearningRuntimeError::Io(err)),
    };

    let mut lines = BufReader::new(file).lines();
    let mut out = Vec::new();
    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(record) = serde_json::from_str::<ContextAttributionRecord>(trimmed) {
            out.push(record);
        }
    }
    Ok(out)
}

pub(crate) fn social_perceptiveness_from_attribution(
    records: &[ContextAttributionRecord],
    window: Duration,
) -> f64 {
    let cutoff = match chrono::Duration::from_std(window) {
        Ok(delta) => Utc::now() - delta,
        Err(_) => DateTime::<Utc>::MIN_UTC,
    };

    let mut referenced = 0usize;
    let mut total = 0usize;
    for record in records.iter().filter(|record| record.ts >= cutoff) {
        if record.source_type != "prior_output" {
            continue;
        }
        total += 1;
        if record.referenced {
            referenced += 1;
        }
    }

    if total == 0 {
        0.0
    } else {
        referenced as f64 / total as f64
    }
}

fn default_now() -> DateTime<Utc> {
    Utc::now()
}

// ── Knowledge confirmation IO ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct KnowledgeConfirmationRecord {
    #[serde(default = "default_now")]
    pub(crate) created_at: DateTime<Utc>,
    #[serde(default)]
    pub(crate) source_episodes: Vec<String>,
}

async fn read_knowledge_records(
    path: &Path,
) -> Result<Vec<KnowledgeConfirmationRecord>, LearningRuntimeError> {
    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(LearningRuntimeError::Io(err)),
    };

    let mut lines = BufReader::new(file).lines();
    let mut out = Vec::new();
    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(record) = serde_json::from_str::<KnowledgeConfirmationRecord>(trimmed) {
            out.push(record);
        }
    }
    Ok(out)
}

// ── Sub-score computations ────────────────────────────────────────────

pub(crate) fn knowledge_integration_rate(
    records: &[KnowledgeConfirmationRecord],
    episodes: &[Episode],
    window: Duration,
) -> f64 {
    let cutoff = match chrono::Duration::from_std(window) {
        Ok(delta) => Utc::now() - delta,
        Err(_) => DateTime::<Utc>::MIN_UTC,
    };

    let mut episode_timestamps: HashMap<String, DateTime<Utc>> = HashMap::new();
    for episode in episodes {
        let source_id = episode_source_id(episode).to_string();
        episode_timestamps
            .entry(source_id)
            .and_modify(|current| {
                if episode.timestamp < *current {
                    *current = episode.timestamp;
                }
            })
            .or_insert(episode.timestamp);
    }

    let mut weighted_speed_sum = 0.0;
    let mut total_weight = 0.0;

    for record in records.iter().filter(|record| record.created_at >= cutoff) {
        let mut source_ids = record.source_episodes.iter().cloned().collect::<Vec<_>>();
        source_ids.sort();
        source_ids.dedup();

        let mut timestamps: Vec<DateTime<Utc>> = source_ids
            .iter()
            .filter_map(|source| episode_timestamps.get(source).copied())
            .collect();
        timestamps.sort();
        if timestamps.len() < 2 {
            continue;
        }

        let confirmations = source_ids.len().saturating_sub(1);
        let span = timestamps
            .last()
            .copied()
            .unwrap_or(record.created_at)
            .signed_duration_since(timestamps.first().copied().unwrap_or(record.created_at));
        let span_hours = span
            .to_std()
            .map(|duration| duration.as_secs_f64() / 3_600.0)
            .unwrap_or(0.0);
        let normalized_speed =
            ((confirmations as f64) / span_hours.max(1.0 / 60.0) / 4.0).clamp(0.0, 1.0);
        let weight = confirmations as f64;
        weighted_speed_sum += normalized_speed * weight;
        total_weight += weight;
    }

    if total_weight == 0.0 {
        0.0
    } else {
        (weighted_speed_sum / total_weight).clamp(0.0, 1.0)
    }
}

pub(crate) fn convergence_velocity_from_agreement(
    records: &[KnowledgeConfirmationRecord],
    episodes: &[Episode],
    window: Duration,
) -> f64 {
    let cutoff = match chrono::Duration::from_std(window) {
        Ok(delta) => Utc::now() - delta,
        Err(_) => DateTime::<Utc>::MIN_UTC,
    };

    let mut episode_agents: HashMap<String, (DateTime<Utc>, String)> = HashMap::new();
    for episode in episodes {
        let source_id = episode_source_id(episode).to_string();
        let agent_id = episode_agent_label(episode);
        episode_agents
            .entry(source_id)
            .and_modify(|current| {
                if episode.timestamp < current.0 {
                    current.0 = episode.timestamp;
                    current.1 = agent_id.clone();
                }
            })
            .or_insert((episode.timestamp, agent_id));
    }

    let mut weighted_speed_sum = 0.0;
    let mut total_weight = 0.0;

    for record in records.iter().filter(|record| record.created_at >= cutoff) {
        let mut source_ids = record.source_episodes.iter().cloned().collect::<Vec<_>>();
        source_ids.sort();
        source_ids.dedup();

        let mut agent_timestamps: HashMap<String, DateTime<Utc>> = HashMap::new();
        for source_id in source_ids {
            let Some((timestamp, agent_id)) = episode_agents.get(&source_id).cloned() else {
                continue;
            };
            agent_timestamps
                .entry(agent_id)
                .and_modify(|current| {
                    if timestamp < *current {
                        *current = timestamp;
                    }
                })
                .or_insert(timestamp);
        }

        if agent_timestamps.len() < 2 {
            continue;
        }

        let mut timestamps: Vec<DateTime<Utc>> = agent_timestamps.values().copied().collect();
        timestamps.sort();
        let span = timestamps
            .last()
            .copied()
            .unwrap_or(record.created_at)
            .signed_duration_since(timestamps.first().copied().unwrap_or(record.created_at));
        let span_hours = span
            .to_std()
            .map(|duration| duration.as_secs_f64() / 3_600.0)
            .unwrap_or(0.0);
        let agreements = agent_timestamps.len().saturating_sub(1);
        let normalized_velocity =
            ((agreements as f64) / span_hours.max(1.0 / 60.0) / 4.0).clamp(0.0, 1.0);
        let weight = agreements as f64;
        weighted_speed_sum += normalized_velocity * weight;
        total_weight += weight;
    }

    if total_weight == 0.0 {
        0.0
    } else {
        (weighted_speed_sum / total_weight).clamp(0.0, 1.0)
    }
}
