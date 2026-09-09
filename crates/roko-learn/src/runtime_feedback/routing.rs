//! Routing observation and latency helpers.
//!
//! Helpers used by [`super::LearningRuntime`] to compute reward signals for the
//! cascade router and to maintain prompt section effectiveness tracking.

use std::io;
use std::path::Path;

use crate::efficiency::AgentEfficiencyEvent;
use crate::latency::LatencyRegistry;
use crate::prompt_experiment::ExperimentStore;
use crate::model_router::compute_routing_reward_v2;

use super::records::LearningRuntimeError;
use super::LearningRuntime;

// ── Latency-aware reward computation ──────────────────────────────────

pub(crate) fn compute_reward_with_latency(
    gate_passed: bool,
    cost_usd: f64,
    wall_time_ms: u64,
    latency_stats: &LatencyRegistry,
    model: &str,
    provider: &str,
) -> f64 {
    let pass_rate = if gate_passed { 1.0 } else { 0.0 };
    let max_cost = 5.0;
    let normalized_cost = (cost_usd / max_cost).min(1.0);
    let historical_p50_ms = latency_stats
        .get(model, provider)
        .map(|stats| stats.p50_ms());
    let observed_latency_ms = if wall_time_ms > 0 {
        wall_time_ms as f64
    } else {
        historical_p50_ms.unwrap_or(30_000.0)
    };
    let sla_ms = 120_000.0;
    compute_routing_reward_v2(pass_rate, normalized_cost, observed_latency_ms, sla_ms)
}

/// Flush the experiment winner artifact to disk.
pub(crate) fn sync_experiment_winner_artifact(
    path: &Path,
    store: &ExperimentStore,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let winners = store.winner_summaries();
    let json = serde_json::to_vec_pretty(&winners)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

// ── Latency helpers ───────────────────────────────────────────────────

pub(crate) fn latency_model_slug(event: &AgentEfficiencyEvent) -> &str {
    let model = event.model_used.trim();
    if model.is_empty() {
        event.model.trim()
    } else {
        model
    }
}

pub(crate) fn latency_provider_id(event: &AgentEfficiencyEvent) -> &str {
    event.backend.trim()
}

pub(crate) fn latency_total_ms(event: &AgentEfficiencyEvent) -> f64 {
    if event.wall_time_ms > 0 {
        event.wall_time_ms as f64
    } else {
        event.duration_ms as f64
    }
}

// ── LearningRuntime extensions ────────────────────────────────────────

impl LearningRuntime {
    pub(crate) fn record_section_effectiveness_from_efficiency_event(
        &self,
        event: &AgentEfficiencyEvent,
    ) -> Result<(), LearningRuntimeError> {
        if event.role.trim().is_empty() || event.prompt_sections.is_empty() {
            return Ok(());
        }

        let mut registry = self.section_effectiveness.lock();
        for section in &event.prompt_sections {
            registry.record_outcome(
                section.name.clone(),
                event.role.trim(),
                !section.was_dropped,
                event.gate_passed.unwrap_or(false),
            );
        }
        registry.save(&self.paths.section_effects_json)?;
        Ok(())
    }

    pub(crate) fn record_latency_from_efficiency_event(
        &self,
        event: &AgentEfficiencyEvent,
    ) -> Result<(), LearningRuntimeError> {
        let model = latency_model_slug(event);
        let provider = latency_provider_id(event);
        if model.is_empty() || provider.is_empty() {
            return Ok(());
        }

        let total_ms = latency_total_ms(event);
        self.latency_registry.record(
            model,
            provider,
            event.time_to_first_token_ms as f64,
            total_ms,
            event.output_tokens,
        );
        self.latency_registry.save(&self.paths.latency_stats_json)?;
        Ok(())
    }
}
