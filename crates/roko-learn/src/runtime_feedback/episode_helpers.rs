//! Episode metadata extraction helpers.
//!
//! Small, pure functions that pull typed values out of the `episode.extra`
//! JSON map.  Shared across multiple sub-modules of `runtime_feedback`.

use std::collections::HashMap;
use std::path::Path;

use crate::costs_db::CostRecord;
use crate::episode_logger::Episode;
use roko_core::agent::AgentRole;

// ── Extra-field accessors ─────────────────────────────────────────────

/// Read optional string value from `episode.extra`.
pub(crate) fn extra_string(episode: &Episode, key: &str) -> Option<String> {
    episode
        .extra
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}

/// Read optional floating-point value from `episode.extra`.
pub(crate) fn extra_f64(episode: &Episode, key: &str) -> Option<f64> {
    episode.extra.get(key).and_then(serde_json::Value::as_f64)
}

pub(crate) fn extra_u64(episode: &Episode, key: &str) -> Option<u64> {
    episode.extra.get(key).and_then(serde_json::Value::as_u64)
}

pub(crate) fn extra_bool(episode: &Episode, key: &str) -> Option<bool> {
    episode.extra.get(key).and_then(serde_json::Value::as_bool)
}

pub(crate) fn extra_string_vec(episode: &Episode, key: &str) -> Option<Vec<String>> {
    let values = episode.extra.get(key)?.as_array()?;
    let out = values
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    (!out.is_empty()).then_some(out)
}

// ── String utilities ──────────────────────────────────────────────────

pub(crate) fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(crate) fn nonzero_u64(value: u64) -> Option<u64> {
    (value > 0).then_some(value)
}

pub(crate) fn ratio_u64(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

// ── Episode field accessors ───────────────────────────────────────────

pub(crate) fn episode_model(episode: &Episode) -> String {
    non_empty_string(&episode.model)
        .or_else(|| extra_string(episode, "model"))
        .or_else(|| extra_string(episode, "model_used"))
        .unwrap_or_default()
}

pub(crate) fn episode_provider(episode: &Episode) -> String {
    non_empty_string(&episode.backend)
        .or_else(|| extra_string(episode, "provider"))
        .or_else(|| extra_string(episode, "backend"))
        .unwrap_or_else(|| "unknown-provider".to_string())
}

pub(crate) fn episode_role(episode: &Episode) -> String {
    extra_string(episode, "role")
        .or_else(|| extra_string(episode, "role_id"))
        .or_else(|| non_empty_string(&episode.agent_template))
        .unwrap_or_else(|| "unknown-role".to_string())
}

pub(crate) fn episode_run_id(episode: &Episode) -> Option<String> {
    extra_string(episode, "run_id")
        .or_else(|| extra_string(episode, "session_id"))
        .or_else(|| non_empty_string(&episode.episode_id))
}

pub(crate) fn prompt_section_count_from_episode(episode: &Episode) -> u32 {
    episode
        .prompt_composition
        .as_ref()
        .and_then(|value| value.get("sections"))
        .and_then(serde_json::Value::as_array)
        .map_or(0, |sections| sections.len().min(u32::MAX as usize) as u32)
}

pub(crate) fn episode_source_id(episode: &Episode) -> &str {
    if episode.episode_id.trim().is_empty() {
        &episode.id
    } else {
        &episode.episode_id
    }
}

pub(crate) fn episode_agent_label(episode: &Episode) -> String {
    let agent_id = episode.agent_id.trim();
    if !agent_id.is_empty() {
        return agent_id.to_string();
    }

    let template = episode.agent_template.trim();
    if !template.is_empty() {
        return template.to_string();
    }

    episode.id.clone()
}

/// Parse an [`AgentRole`] from either the persisted kebab-case label or the
/// debug-style variant name used by `format!("{role:?}")` in orchestration.
pub(crate) fn parse_agent_role(raw: &str) -> Option<AgentRole> {
    if let Ok(role) = serde_json::from_str::<AgentRole>(&format!("\"{raw}\"")) {
        return Some(role);
    }

    std::iter::once(AgentRole::Conductor)
        .chain(AgentRole::ALL_AGENTS.iter().copied())
        .find(|role| raw == format!("{role:?}"))
}

// ── Gate counts ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct GateCounts {
    pub(crate) passed: u64,
    pub(crate) failed: u64,
    pub(crate) skipped: u64,
}

impl GateCounts {
    pub(crate) fn executed(self) -> u64 {
        self.passed + self.failed
    }

    pub(crate) fn pass_rate(self) -> f64 {
        let executed = self.executed();
        if executed == 0 {
            0.0
        } else {
            self.passed as f64 / executed as f64
        }
    }

    pub(crate) fn summary(self) -> String {
        format!(
            "{} passed, {} failed, {} skipped",
            self.passed, self.failed, self.skipped
        )
    }

    pub(crate) fn has_only_skipped(self) -> bool {
        self.executed() == 0 && self.skipped > 0
    }
}

pub(crate) fn gate_counts_from_episode(episode: &Episode) -> Option<GateCounts> {
    let gate_counts = episode
        .extra
        .get("gate_counts")
        .and_then(serde_json::Value::as_object);

    let passed = gate_counts
        .and_then(|counts| counts.get("passed"))
        .and_then(serde_json::Value::as_u64)
        .or_else(|| extra_u64(episode, "gates_passed"));
    let failed = gate_counts
        .and_then(|counts| counts.get("failed"))
        .and_then(serde_json::Value::as_u64)
        .or_else(|| extra_u64(episode, "gates_failed"));
    let skipped = gate_counts
        .and_then(|counts| counts.get("skipped"))
        .and_then(serde_json::Value::as_u64)
        .or_else(|| extra_u64(episode, "gates_skipped"));

    if passed.is_none() && failed.is_none() && skipped.is_none() && episode.gate_verdicts.is_empty()
    {
        return None;
    }

    Some(GateCounts {
        passed: passed.unwrap_or_else(|| {
            episode
                .gate_verdicts
                .iter()
                .filter(|verdict| verdict.passed)
                .count() as u64
        }),
        failed: failed.unwrap_or_else(|| {
            episode
                .gate_verdicts
                .iter()
                .filter(|verdict| !verdict.passed)
                .count() as u64
        }),
        skipped: skipped.unwrap_or(0),
    })
}

pub(crate) fn backfill_gate_counts(episode: &mut Episode, counts: GateCounts) {
    episode
        .extra
        .entry("gates_passed".to_string())
        .or_insert_with(|| serde_json::json!(counts.passed));
    episode
        .extra
        .entry("gates_failed".to_string())
        .or_insert_with(|| serde_json::json!(counts.failed));
    episode
        .extra
        .entry("gates_skipped".to_string())
        .or_insert_with(|| serde_json::json!(counts.skipped));
    episode
        .extra
        .entry("gates_executed".to_string())
        .or_insert_with(|| serde_json::json!(counts.executed()));
    episode
        .extra
        .entry("gate_summary".to_string())
        .or_insert_with(|| serde_json::json!(counts.summary()));
    episode
        .extra
        .entry("gate_pass_rate".to_string())
        .or_insert_with(|| serde_json::json!(counts.pass_rate()));
    episode
        .extra
        .entry("gate_counts".to_string())
        .or_insert_with(|| {
            serde_json::json!({
                "passed": counts.passed,
                "failed": counts.failed,
                "skipped": counts.skipped,
                "executed": counts.executed(),
                "summary": counts.summary(),
                "pass_rate": counts.pass_rate(),
            })
        });
}

// ── Retry status ──────────────────────────────────────────────────────

use super::records::RetryOutcomeStatus;

pub(crate) fn retry_status_from_episode(episode: &Episode) -> Option<RetryOutcomeStatus> {
    if let Some(raw) =
        extra_string(episode, "retry_status").or_else(|| extra_string(episode, "retry_outcome"))
    {
        return parse_retry_status(&raw);
    }
    if extra_bool(episode, "retry_scheduled") == Some(true) {
        return Some(RetryOutcomeStatus::Scheduled);
    }
    if extra_bool(episode, "retry_started") == Some(true) {
        return Some(RetryOutcomeStatus::Started);
    }
    if extra_bool(episode, "retry_exhausted") == Some(true) {
        return Some(RetryOutcomeStatus::Exhausted);
    }
    if extra_bool(episode, "retry_not_retryable") == Some(true) {
        return Some(RetryOutcomeStatus::NotRetryable);
    }
    None
}

pub(crate) fn parse_retry_status(raw: &str) -> Option<RetryOutcomeStatus> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "scheduled" => Some(RetryOutcomeStatus::Scheduled),
        "started" => Some(RetryOutcomeStatus::Started),
        "succeeded" | "success" | "passed" => Some(RetryOutcomeStatus::Succeeded),
        "exhausted" | "retries_exhausted" => Some(RetryOutcomeStatus::Exhausted),
        "not_retryable" | "non_retryable" => Some(RetryOutcomeStatus::NotRetryable),
        "cancelled" | "canceled" => Some(RetryOutcomeStatus::Cancelled),
        _ => None,
    }
}

// ── Hashing ───────────────────────────────────────────────────────────

pub(crate) fn stable_hash_hex(parts: &[&str]) -> String {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    for part in parts {
        for byte in part.as_bytes().iter().copied().chain([0xff]) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    format!("{hash:016x}")
}

// ── Cost derivation ───────────────────────────────────────────────────

/// Build a [`CostRecord`] from an [`Episode`] and optional provider override.
///
/// An episode whose cost nobody measured ([`Episode::cost_known`]) gets no
/// record: a $0 row in the cost ledger would read as a free run.
pub(crate) fn derive_cost_record(
    episode: &Episode,
    provider_override: Option<&str>,
) -> Option<CostRecord> {
    if episode.agent_id.is_empty() && episode.task_id.is_empty() {
        return None;
    }
    if !episode.cost_known() {
        return None;
    }

    let provider = provider_override
        .map(ToOwned::to_owned)
        .or_else(|| extra_string(episode, "provider"))
        .unwrap_or_else(|| "unknown-provider".to_string());

    Some(CostRecord {
        timestamp: episode.timestamp.to_rfc3339(),
        model: episode_model(episode),
        provider,
        role: extra_string(episode, "role").unwrap_or_else(|| "unknown-role".to_string()),
        plan_id: extra_string(episode, "plan_id").unwrap_or_default(),
        task_id: if episode.task_id.is_empty() {
            extra_string(episode, "task_id").unwrap_or_default()
        } else {
            episode.task_id.clone()
        },
        complexity_band: extra_string(episode, "complexity_band")
            .unwrap_or_else(|| "standard".to_string()),
        input_tokens: episode.usage.input_tokens,
        output_tokens: episode.usage.output_tokens,
        cached_tokens: episode.usage.cache_read_tokens,
        cost_usd: episode.usage.cost_usd,
        duration_ms: episode.usage.wall_ms,
        success: episode.success,
        session_id: extra_string(episode, "session_id").unwrap_or_default(),
    })
}

/// Load persisted local reward functions, or return an empty map.
pub(crate) fn load_local_rewards(
    path: &Path,
) -> HashMap<String, crate::local_reward::LocalRewardFunction> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}
