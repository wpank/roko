//! Learning-file synchronisation and efficiency-rate computation.
//!
//! Contains `sync_connected_learning_files` (efficiency JSONL tail,
//! experiment store reload, safety incident ingestion) and the
//! per-frame `update_efficiency_rates` EMA tracker.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use chrono::{DateTime, Utc};

use super::super::dashboard::{
    CascadeRouterState, DashboardData, ExperimentSummary, PlaybookSummary,
};
use super::{
    AgentRow, MAX_TOKEN_SAMPLES, ProviderStatus, RouteMetrics, SafetyIncident, SmoothedValue,
    TuiState,
};
use roko_core::OperatingFrequency;

// ---------------------------------------------------------------------------
// TuiState impl -- connected learning file sync
// ---------------------------------------------------------------------------

impl TuiState {
    /// Tail local learning files to pick up events that the push-based
    /// snapshot protocol cannot carry (per-event payloads, experiment stores).
    pub(super) fn sync_connected_learning_files(&mut self) {
        if self.workdir.as_os_str().is_empty() {
            return;
        }
        let learn_dir = self.workdir.join(".roko").join("learn");

        // -- efficiency events: incremental JSONL tail, parse only new bytes --
        let efficiency_path = learn_dir.join("efficiency.jsonl");
        let efficiency_len = std::fs::metadata(&efficiency_path).map_or(0, |meta| meta.len());
        if efficiency_len != self.connected_efficiency_len {
            self.connected_efficiency_len = efficiency_len;
            if self
                .connected_efficiency_tailer
                .path()
                .as_os_str()
                .is_empty()
            {
                self.connected_efficiency_tailer =
                    super::super::jsonl_tailer::efficiency_tailer(&efficiency_path);
            }
            let _ = self.connected_efficiency_tailer.tick();
            self.efficiency_events = self.connected_efficiency_tailer.items().to_vec();
            self.rev_efficiency.bump();
            if !self.efficiency_events.is_empty() {
                // Event-derived summary has real pass counts and latencies;
                // prefer it over the approximation from pushed trend buckets.
                self.efficiency_summary = super::super::dashboard::efficiency_summary_from_events(
                    &self.efficiency_events,
                );
            }
            if self.efficiency_trend.is_empty() {
                self.efficiency_trend = roko_learn::aggregate::efficiency_trend(
                    &efficiency_path,
                    chrono::Duration::hours(1),
                    24,
                )
                .unwrap_or_default();
            }
        }

        // -- experiment store: stamp-gated whole-file reload --
        let experiments_path = learn_dir.join("experiments.json");
        let experiments_stamp = std::fs::metadata(&experiments_path).map_or((0, 0), |meta| {
            let mtime_ms = meta
                .modified()
                .ok()
                .and_then(|mtime| mtime.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |duration| duration.as_millis() as i64);
            (meta.len(), mtime_ms)
        });
        if experiments_stamp != self.connected_experiments_stamp {
            self.connected_experiments_stamp = experiments_stamp;
            if let Ok(text) = std::fs::read_to_string(&experiments_path) {
                if let Ok(store) =
                    serde_json::from_str::<roko_learn::prompt_experiment::ExperimentStore>(&text)
                {
                    let mut experiments = store
                        .iter()
                        .map(ExperimentSummary::from_experiment)
                        .collect::<Vec<_>>();
                    experiments.sort_by(|a, b| a.experiment_id.cmp(&b.experiment_id));
                    self.experiments = experiments;
                    self.experiment_winners = store.winner_summaries();
                }
            }
        }

        // P2-06: Load safety incidents from `.roko/immune/` directory.
        let immune_dir = self.workdir.join(".roko").join("immune");
        if immune_dir.is_dir() {
            let mut incidents = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&immune_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path
                        .extension()
                        .is_some_and(|ext| ext == "json" || ext == "jsonl")
                    {
                        if let Ok(text) = std::fs::read_to_string(&path) {
                            for line in text.lines() {
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                                    let incident = SafetyIncident {
                                        timestamp_ms: val
                                            .get("timestamp_ms")
                                            .or_else(|| val.get("ts"))
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0),
                                        event_type: val
                                            .get("event_type")
                                            .or_else(|| val.get("kind"))
                                            .or_else(|| val.get("type"))
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown")
                                            .to_string(),
                                        severity: val
                                            .get("severity")
                                            .or_else(|| val.get("level"))
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("info")
                                            .to_string(),
                                        description: val
                                            .get("description")
                                            .or_else(|| val.get("message"))
                                            .or_else(|| val.get("summary"))
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                    };
                                    incidents.push(incident);
                                }
                            }
                        }
                    }
                }
            }
            incidents.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
            self.safety_incidents = incidents;
        }
    }

    /// Update smoothed token and cost rates from the last sample interval.
    pub(super) fn update_efficiency_rates(&mut self) {
        let now = Instant::now();
        let token_total = self.token_total;
        let cost_dollars = self.cost_dollars;

        if token_total < self.last_token_total_sample
            || cost_dollars < self.last_cost_dollars_sample
        {
            const METRIC_EMA_ALPHA: f64 = 0.25;

            self.token_rate = 0.0;
            self.cost_rate = 0.0;
            self.token_rate_smoothed = SmoothedValue::new(METRIC_EMA_ALPHA);
            self.cost_rate_smoothed = SmoothedValue::new(METRIC_EMA_ALPHA);
        } else if let Some(last_sample_at) = self.last_rate_sample_at {
            let elapsed_secs = now.duration_since(last_sample_at).as_secs_f64();
            if elapsed_secs > 0.0 {
                let token_delta = token_total.saturating_sub(self.last_token_total_sample) as f64;
                let cost_delta = (cost_dollars - self.last_cost_dollars_sample).max(0.0);

                self.token_rate = self
                    .token_rate_smoothed
                    .update(token_delta * 60.0 / elapsed_secs);
                self.cost_rate = self
                    .cost_rate_smoothed
                    .update(cost_delta * 60.0 / elapsed_secs);
            }
        }

        self.last_rate_sample_at = Some(now);
        self.last_token_total_sample = token_total;
        self.last_cost_dollars_sample = cost_dollars;
    }
}

// ---------------------------------------------------------------------------
// Free helpers -- provider status population
// ---------------------------------------------------------------------------

/// Build `Vec<ProviderStatus>` from the on-disk provider-health registry
/// and efficiency events already loaded into `TuiState`.
///
/// The function merges three data sources:
///
/// 1. **Provider health registry** (`provider-health.json`) -- circuit state,
///    request/failure counts, cooldown timers, and failure window.
/// 2. **Efficiency events** (`efficiency.jsonl`) -- per-turn cost, latency,
///    and model information already parsed into `TuiState::efficiency_events`.
/// 3. **Config providers** -- the keys of `[providers.*]` in `roko.toml`
///    ensure every configured provider appears even if it has never been used.
#[allow(clippy::cast_precision_loss)]
pub(super) fn populate_provider_statuses(
    workdir: &Path,
    efficiency_events: &[roko_learn::efficiency::AgentEfficiencyEvent],
) -> Vec<ProviderStatus> {
    use super::{CreditStatus, ProviderHealth};
    use roko_learn::provider_health::{CircuitState, ErrorClass};

    // --- 1. Load provider health from disk ---
    let health_path = workdir
        .join(".roko")
        .join("learn")
        .join("provider-health.json");
    let health_map: HashMap<String, roko_learn::provider_health::ProviderHealth> =
        std::fs::read_to_string(&health_path)
            .ok()
            .and_then(|text| {
                #[derive(serde::Deserialize)]
                struct Snap {
                    #[serde(default)]
                    providers: HashMap<String, roko_learn::provider_health::ProviderHealth>,
                }
                serde_json::from_str::<Snap>(&text).ok()
            })
            .map(|snap| snap.providers)
            .unwrap_or_default();

    // --- 2. Aggregate efficiency events per provider ---
    struct EffAgg {
        total_cost: f64,
        total_latency_ms: u64,
        call_count: u64,
        models: HashSet<String>,
        cost_samples: Vec<f64>,
        latency_samples: Vec<u64>,
    }
    impl Default for EffAgg {
        fn default() -> Self {
            Self {
                total_cost: 0.0,
                total_latency_ms: 0,
                call_count: 0,
                models: HashSet::new(),
                cost_samples: Vec::new(),
                latency_samples: Vec::new(),
            }
        }
    }

    let mut eff_by_provider: HashMap<String, EffAgg> = HashMap::new();
    for event in efficiency_events {
        let provider_key = if event.backend.is_empty() {
            infer_provider_name(&event.model)
        } else {
            normalize_provider_display(&event.backend)
        };
        let agg = eff_by_provider.entry(provider_key).or_default();
        agg.call_count += 1;
        agg.total_cost += event.cost_usd;
        agg.total_latency_ms += event.wall_time_ms;
        if !event.model.is_empty() {
            agg.models.insert(event.model.clone());
        }
        agg.cost_samples.push(event.cost_usd);
        agg.latency_samples.push(event.wall_time_ms);
    }

    // --- 3. Load configured providers from roko.toml ---
    let config_providers: Vec<(String, String)> =
        roko_core::config::loader::load_config_unified(workdir)
            .map(|cfg| {
                cfg.providers
                    .iter()
                    .map(|(name, pc)| (name.clone(), format!("{:?}", pc.kind)))
                    .collect()
            })
            .unwrap_or_default();

    // --- 4. Merge all provider names ---
    let mut all_providers: BTreeMap<String, Option<String>> = BTreeMap::new();
    for (name, kind) in &config_providers {
        all_providers.insert(name.clone(), Some(kind.clone()));
    }
    for key in health_map.keys() {
        all_providers.entry(key.clone()).or_insert(None);
    }
    for key in eff_by_provider.keys() {
        all_providers.entry(key.clone()).or_insert(None);
    }

    // --- 5. Build ProviderStatus for each ---
    let now_ms = Utc::now().timestamp_millis();
    let mut statuses: Vec<ProviderStatus> = Vec::with_capacity(all_providers.len());

    for (name, config_kind) in &all_providers {
        let health_entry = health_map.get(name);
        let eff_entry = eff_by_provider.get(name);

        // Circuit state from health registry.
        let (circuit_state, total_requests, total_failures, cooldown_remaining_secs) =
            if let Some(h) = health_entry {
                let circuit_label = match h.state {
                    CircuitState::Closed => "closed",
                    CircuitState::Open => "open",
                    CircuitState::HalfOpen => "half_open",
                };
                let cooldown = h.cooldown_until.and_then(|until| {
                    let remaining_ms = until - now_ms;
                    if remaining_ms > 0 {
                        Some((remaining_ms / 1000) as u64)
                    } else {
                        None
                    }
                });
                (
                    circuit_label.to_string(),
                    h.total_requests,
                    h.total_failures,
                    cooldown,
                )
            } else {
                ("closed".to_string(), 0, 0, None)
            };

        // Success rate: prefer health registry (it sees all requests including
        // failures that don't produce efficiency events).
        let success_rate = if total_requests > 0 {
            (total_requests - total_failures) as f64 / total_requests as f64
        } else {
            // No health data yet; assume healthy.
            1.0
        };

        // Health classification.
        let health = if let Some(h) = health_entry {
            match h.state {
                CircuitState::Open => {
                    // Check if it's a billing issue.
                    let is_billing = h
                        .failure_window
                        .back()
                        .is_some_and(|f| f.error_class == ErrorClass::Billing);
                    if is_billing {
                        ProviderHealth::Billing
                    } else {
                        ProviderHealth::Failed
                    }
                }
                CircuitState::HalfOpen => ProviderHealth::Degraded,
                CircuitState::Closed => {
                    if success_rate < 0.7 && total_requests >= 3 {
                        ProviderHealth::Degraded
                    } else {
                        ProviderHealth::Healthy
                    }
                }
            }
        } else {
            ProviderHealth::Healthy
        };

        // Cost and latency from efficiency events.
        let (total_cost_usd, avg_latency_ms, active_models, cost_history, latency_history) =
            if let Some(agg) = eff_entry {
                let avg_lat = if agg.call_count > 0 {
                    agg.total_latency_ms / agg.call_count
                } else {
                    0
                };
                let models: Vec<String> = agg.models.iter().cloned().collect();
                // Keep only the last 60 samples for sparklines.
                let cost_hist: Vec<f64> = agg
                    .cost_samples
                    .iter()
                    .rev()
                    .take(60)
                    .rev()
                    .copied()
                    .collect();
                let lat_hist: Vec<u64> = agg
                    .latency_samples
                    .iter()
                    .rev()
                    .take(60)
                    .rev()
                    .copied()
                    .collect();
                (agg.total_cost, avg_lat, models, cost_hist, lat_hist)
            } else {
                (0.0, 0, Vec::new(), Vec::new(), Vec::new())
            };

        // Last error from health registry.
        let last_error = health_entry.and_then(|h| {
            h.failure_window
                .back()
                .map(|f| format!("{:?}", f.error_class))
        });

        // Kind from config, or "unknown".
        let kind = config_kind.clone().unwrap_or_else(|| "unknown".to_string());

        statuses.push(ProviderStatus {
            name: name.clone(),
            kind,
            health,
            credit_status: CreditStatus::Unknown,
            total_cost_usd,
            total_requests,
            total_failures,
            success_rate,
            avg_latency_ms,
            active_models,
            cost_history,
            latency_history,
            last_error,
            circuit_state,
            cooldown_remaining_secs,
        });
    }

    statuses
}

/// Infer a provider name from a model slug when the efficiency event has
/// no explicit backend field.
pub(super) fn infer_provider_name(model: &str) -> String {
    let lower = model.to_ascii_lowercase();
    if lower.contains("claude") || lower.contains("anthropic") {
        "anthropic".to_string()
    } else if lower.contains("gpt")
        || lower.contains("openai")
        || lower.contains("o1")
        || lower.contains("o3")
        || lower.contains("o4")
    {
        "openai".to_string()
    } else if lower.contains("gemini") || lower.contains("google") {
        "google".to_string()
    } else if lower.contains("llama") || lower.contains("cerebras") {
        "cerebras".to_string()
    } else if lower.contains("perplexity") || lower.contains("sonar") {
        "perplexity".to_string()
    } else if model.contains('/') {
        model.split('/').next().unwrap_or("unknown").to_string()
    } else {
        "unknown".to_string()
    }
}

/// Normalize a provider backend key for display (e.g. "anthropic_api" -> "anthropic").
pub(super) fn normalize_provider_display(backend: &str) -> String {
    let lower = backend.to_ascii_lowercase().replace('-', "_");
    match lower.as_str() {
        "anthropic_api" | "anthropicapi" => "anthropic".to_string(),
        "openai_compat" | "openaicompat" => "openai".to_string(),
        "gemini_api" | "geminiapi" => "google".to_string(),
        "cerebras_api" | "cerebrasapi" => "cerebras".to_string(),
        "perplexity_api" | "perplexityapi" => "perplexity".to_string(),
        "claude_cli" | "claudecli" => "claude_cli".to_string(),
        "gemini_cli" | "geminicli" => "gemini_cli".to_string(),
        "cursor_cli" | "cursorcli" => "cursor_cli".to_string(),
        "cursor_acp" | "cursoracp" => "cursor_acp".to_string(),
        "codex_cli" | "codexcli" => "codex_cli".to_string(),
        _ => lower,
    }
}

// ---------------------------------------------------------------------------
// Free helpers -- token / cost / agent-event aggregation
// ---------------------------------------------------------------------------

pub(super) struct LatestAgentEvent {
    pub role: String,
    pub status: String,
    pub model: String,
    pub plan_id: Option<String>,
    pub task_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub timestamp: Option<DateTime<Utc>>,
}

pub(super) fn latest_agent_events(
    events: &[roko_learn::efficiency::AgentEfficiencyEvent],
) -> HashMap<String, LatestAgentEvent> {
    let mut latest = HashMap::new();

    for event in events {
        let timestamp = parse_efficiency_timestamp(&event.timestamp);
        let candidate = LatestAgentEvent {
            role: event.role.clone(),
            status: if event.gate_passed == Some(true) {
                "done".to_string()
            } else {
                "active".to_string()
            },
            model: event.model.clone(),
            plan_id: Some(event.plan_id.clone()),
            task_id: event.task_id.clone(),
            input_tokens: event.input_tokens,
            output_tokens: event.output_tokens,
            timestamp,
        };

        let should_replace = latest
            .get(&event.agent_id)
            .map(
                |existing: &LatestAgentEvent| match (existing.timestamp, candidate.timestamp) {
                    (Some(lhs), Some(rhs)) => rhs >= lhs,
                    (None, Some(_)) => true,
                    _ => false,
                },
            )
            .unwrap_or(true);
        if should_replace {
            latest.insert(event.agent_id.clone(), candidate);
        }
    }

    latest
}

pub(super) fn latest_route_metrics(
    events: &[roko_learn::efficiency::AgentEfficiencyEvent],
    data: &DashboardData,
) -> HashMap<String, RouteMetrics> {
    let mut latest: HashMap<String, (Option<DateTime<Utc>>, RouteMetrics)> = HashMap::new();

    for event in events {
        let timestamp = parse_efficiency_timestamp(&event.timestamp);
        let should_replace = latest
            .get(&event.agent_id)
            .map(|(existing, _)| match (*existing, timestamp) {
                (Some(lhs), Some(rhs)) => rhs >= lhs,
                (None, Some(_)) => true,
                (None, None) => true,
                _ => false,
            })
            .unwrap_or(true);
        if should_replace {
            latest.insert(
                event.agent_id.clone(),
                (timestamp, route_metrics_from_event(event, data)),
            );
        }
    }

    latest
        .into_iter()
        .map(|(agent_id, (_, metrics))| (agent_id, metrics))
        .collect()
}

pub(super) fn compute_token_rate(events: &[roko_learn::efficiency::AgentEfficiencyEvent]) -> f64 {
    let mut first_seen: Option<DateTime<Utc>> = None;
    let mut last_seen: Option<DateTime<Utc>> = None;
    let mut total_tokens = 0_u64;

    for event in events {
        let Some(timestamp) = parse_efficiency_timestamp(&event.timestamp) else {
            continue;
        };
        first_seen = Some(match first_seen {
            Some(current) => current.min(timestamp),
            None => timestamp,
        });
        last_seen = Some(match last_seen {
            Some(current) => current.max(timestamp),
            None => timestamp,
        });
        total_tokens = total_tokens.saturating_add(event.total_tokens());
    }

    let Some(first_seen) = first_seen else {
        return 0.0;
    };
    let Some(last_seen) = last_seen else {
        return 0.0;
    };

    let elapsed_seconds = last_seen.signed_duration_since(first_seen).num_seconds();
    if elapsed_seconds <= 0 {
        return 0.0;
    }

    total_tokens as f64 / (elapsed_seconds as f64 / 60.0)
}

pub(super) fn build_token_samples(
    data: &DashboardData,
) -> HashMap<String, std::collections::VecDeque<(DateTime<Utc>, u64)>> {
    use std::collections::VecDeque;

    let max_samples = MAX_TOKEN_SAMPLES;

    let mut per_agent: HashMap<String, Vec<(DateTime<Utc>, u64)>> = HashMap::new();

    if data.efficiency_events.is_empty() {
        for episode in data.episodes() {
            let total_tokens = episode
                .usage
                .input_tokens
                .saturating_add(episode.usage.output_tokens);
            per_agent
                .entry(episode.agent_id.clone())
                .or_default()
                .push((episode.timestamp, total_tokens));
        }
    } else {
        for event in &data.efficiency_events {
            let Some(timestamp) = parse_efficiency_timestamp(&event.timestamp) else {
                continue;
            };
            per_agent
                .entry(event.agent_id.clone())
                .or_default()
                .push((timestamp, event.total_tokens()));
        }
    }

    let mut histories = HashMap::new();
    for (agent_id, mut samples) in per_agent {
        samples.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));

        let mut cumulative_total = 0u64;
        let mut history = VecDeque::new();
        for (timestamp, total_tokens) in samples {
            cumulative_total = cumulative_total.saturating_add(total_tokens);
            history.push_back((timestamp, cumulative_total));
            if history.len() > max_samples {
                history.pop_front();
            }
        }

        histories.insert(agent_id, history);
    }

    histories
}

pub(super) fn compute_windowed_token_rate(
    samples: &std::collections::VecDeque<(DateTime<Utc>, u64)>,
) -> f64 {
    const TOKEN_RATE_WINDOW_SAMPLES: usize = 60;

    if samples.len() < 2 {
        return 0.0;
    }

    let start_idx = samples.len().saturating_sub(TOKEN_RATE_WINDOW_SAMPLES);
    let Some((start_time, start_total)) = samples.get(start_idx) else {
        return 0.0;
    };
    let Some((end_time, end_total)) = samples.back() else {
        return 0.0;
    };

    let elapsed_secs = end_time
        .signed_duration_since(*start_time)
        .num_milliseconds() as f64
        / 1_000.0;
    if elapsed_secs <= 0.0 {
        return 0.0;
    }

    end_total.saturating_sub(*start_total) as f64 * 60.0 / elapsed_secs
}

pub(super) fn parse_efficiency_timestamp(timestamp: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

pub(super) fn sum_costs(
    data: &DashboardData,
    plans: &mut HashMap<String, f64>,
    tasks: &mut HashMap<String, f64>,
) {
    plans.clear();
    tasks.clear();
    for e in &data.efficiency_events {
        if !e.plan_id.is_empty() {
            *plans.entry(e.plan_id.clone()).or_default() += e.cost_usd;
        }
        if !e.task_id.is_empty() {
            *tasks
                .entry(format!("{}:{}", e.plan_id, e.task_id))
                .or_default() += e.cost_usd;
        }
    }
}

pub(super) fn plan_is_active(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "active"
            | "running"
            | "executing"
            | "in_progress"
            | "implementing"
            | "gating"
            | "verifying"
            | "reviewing"
            | "strategist"
            | "implementer"
            | "preflight"
    )
}

/// Extract output text from an episode's extra fields.
pub(super) fn extract_episode_output(episode: &roko_learn::episode_logger::Episode) -> String {
    for key in [
        "stderr",
        "agent_stderr",
        "output",
        "stdout",
        "agent_output",
        "output_tail",
        "detail",
        "text",
    ] {
        if let Some(serde_json::Value::String(text)) = episode.extra.get(key) {
            if !text.trim().is_empty() {
                return text.clone();
            }
        }
    }
    episode.failure_reason.as_deref().unwrap_or("").to_string()
}

// ---------------------------------------------------------------------------
// Route metrics helpers (used by snapshot.rs too)
// ---------------------------------------------------------------------------

pub(super) fn route_tier_label_for_frequency(frequency: OperatingFrequency) -> &'static str {
    match frequency {
        OperatingFrequency::Gamma => "fast",
        OperatingFrequency::Theta => "balanced",
        OperatingFrequency::Delta => "deep",
    }
}

pub(super) fn route_tier_label_for_model(model: &str) -> &'static str {
    let lower = model.to_ascii_lowercase();
    if lower.contains("haiku") || lower.contains("flash") || lower.contains("mini") {
        "fast"
    } else if lower.contains("sonnet")
        || lower.contains("gpt-4o")
        || lower.contains("gemini-2")
        || lower.contains("pro")
    {
        "standard"
    } else if lower.contains("opus") || lower.contains("o1") || lower.contains("o3") {
        "deep"
    } else {
        "standard"
    }
}

pub(super) fn prompt_focus_score(event: &roko_learn::efficiency::AgentEfficiencyEvent) -> f64 {
    if event.prompt_sections.is_empty() {
        return if event.total_prompt_tokens > 0 {
            1.0
        } else {
            0.0
        };
    }

    let mut max_weighted = 0.0;
    let mut retained_weighted = 0.0;
    for section in &event.prompt_sections {
        let priority_weight = 1.0 / (1.0 + f64::from(section.priority));
        let weighted_tokens = section.tokens as f64 * priority_weight;
        max_weighted += weighted_tokens;

        let retention = if section.was_dropped {
            0.0
        } else if section.was_truncated {
            0.5
        } else {
            1.0
        };
        retained_weighted += weighted_tokens * retention;
    }

    if max_weighted > 0.0 {
        (retained_weighted / max_weighted).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[must_use]
pub(super) fn route_focus_score(
    event: &roko_learn::efficiency::AgentEfficiencyEvent,
    data: &DashboardData,
    model: &str,
) -> f64 {
    if let Some(stats) = data.cascade_router.confidence_stats.get(model) {
        if stats.trials > 0 {
            return (stats.successes as f64 / stats.trials as f64).clamp(0.0, 1.0);
        }
    }
    prompt_focus_score(event)
}

#[must_use]
pub(super) fn route_metrics_from_event(
    event: &roko_learn::efficiency::AgentEfficiencyEvent,
    data: &DashboardData,
) -> RouteMetrics {
    let model = crate::tui::display_utils::event_model_slug(event);
    let context_limit = super::model_context_limit(&model);
    let focus_score = route_focus_score(event, data, &model);
    RouteMetrics {
        tier: if model.is_empty() {
            route_tier_label_for_frequency(event.frequency).to_string()
        } else {
            route_tier_label_for_model(&model).to_string()
        },
        model,
        context_used: event.total_tokens(),
        context_limit,
        focus_score,
    }
}

#[must_use]
pub(super) fn fallback_route_metrics_for_agent(agent: &AgentRow) -> RouteMetrics {
    let context_limit = agent
        .context_limit
        .max(super::model_context_limit(agent.model.as_str()));
    RouteMetrics {
        model: agent.model.clone(),
        tier: route_tier_label_for_model(&agent.model).to_string(),
        context_used: agent.input_tokens.saturating_add(agent.output_tokens),
        context_limit,
        focus_score: 0.0,
    }
}

/// Load playbook rule count from disk.
#[allow(dead_code)] // pre-wired TUI learning view helper; P2-TUI-7
pub(super) fn load_playbook_rule_count(learn_dir: &Path) -> usize {
    let playbooks_path = learn_dir.join("playbooks.json");
    std::fs::read_to_string(&playbooks_path)
        .ok()
        .and_then(|text| {
            #[derive(serde::Deserialize)]
            struct Stub {
                rules: Vec<serde_json::Value>,
            }
            serde_json::from_str::<Stub>(&text)
                .ok()
                .map(|stub| stub.rules.len())
        })
        .unwrap_or(0)
}

/// Load playbook summaries from disk.
#[allow(dead_code)] // pre-wired TUI learning view helper; P2-TUI-7
pub(super) fn load_playbook_summaries(learn_dir: &Path) -> Vec<PlaybookSummary> {
    let playbooks_dir = learn_dir.join("playbooks");
    let entries = match std::fs::read_dir(&playbooks_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut summaries = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(pb) = serde_json::from_str::<serde_json::Value>(&contents) else {
            continue;
        };
        let id = pb
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = pb
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();
        let goal = pb
            .get("goal")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let step_count = pb
            .get("steps")
            .and_then(|v| v.as_array())
            .map_or(0, |a| a.len());
        let success_count = pb
            .get("success_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let failure_count = pb
            .get("failure_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let total = success_count + failure_count;
        let success_rate_pct = if total > 0 {
            Some(success_count as f64 / total as f64 * 100.0)
        } else {
            None
        };
        summaries.push(PlaybookSummary {
            id,
            name,
            goal,
            step_count,
            success_count,
            failure_count,
            success_rate_pct,
        });
    }
    // Sort by success_count descending.
    summaries.sort_by(|a, b| b.success_count.cmp(&a.success_count));
    summaries
}

/// Compute routing coverage from cascade router state.
#[allow(dead_code)] // pre-wired TUI learning view helper; P2-TUI-7
pub(super) fn compute_routing_coverage(router: &CascadeRouterState) -> f64 {
    if router.model_slugs.is_empty() {
        return 0.0;
    }
    let with_data = router
        .confidence_stats
        .values()
        .filter(|s| s.trials > 0)
        .count();
    with_data as f64 / router.model_slugs.len() as f64 * 100.0
}

/// Load gate thresholds summary from disk.
#[allow(dead_code)] // pre-wired TUI learning view helper; P2-TUI-7
pub(super) fn load_gate_thresholds_summary(learn_dir: &Path) -> Vec<(String, f64)> {
    let path = learn_dir.join("gate-thresholds.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| {
            let val: serde_json::Value = serde_json::from_str(&text).ok()?;
            let rungs = val.get("rungs")?.as_object()?;
            let mut summary = Vec::new();
            for (rung, data) in rungs {
                if let Some(ema) = data.get("ema_pass_rate").and_then(|v| v.as_f64()) {
                    summary.push((rung.clone(), ema));
                }
            }
            summary.sort_by(|a, b| a.0.cmp(&b.0));
            Some(summary)
        })
        .unwrap_or_default()
}

/// Aggregate prompt section stats from efficiency events.
///
/// Returns `(tokens_per_role, context_utilization)` where each role maps to
/// an average token count and an estimated context utilisation ratio.
#[allow(dead_code)] // pre-wired TUI learning view helper; P2-TUI-7
pub(super) fn aggregate_prompt_stats(
    events: &[roko_learn::efficiency::AgentEfficiencyEvent],
) -> (Vec<(String, u64)>, Vec<(String, f64)>) {
    use std::collections::BTreeMap;
    let mut role_tokens: BTreeMap<String, (u64, u64)> = BTreeMap::new(); // (total, count)
    for ev in events {
        let entry = role_tokens.entry(ev.role.clone()).or_default();
        entry.0 += ev.input_tokens + ev.output_tokens;
        entry.1 += 1;
    }
    let tokens_per_role: Vec<(String, u64)> = role_tokens
        .iter()
        .map(|(role, (total, count))| {
            let avg = if *count > 0 { total / count } else { 0 };
            (role.clone(), avg)
        })
        .collect();
    // Context utilization: assume 200K context window for now.
    const DEFAULT_CONTEXT_WINDOW: u64 = 200_000;
    let utilization: Vec<(String, f64)> = tokens_per_role
        .iter()
        .map(|(role, avg)| (role.clone(), *avg as f64 / DEFAULT_CONTEXT_WINDOW as f64))
        .collect();
    (tokens_per_role, utilization)
}

pub(super) fn current_epoch_ms() -> u64 {
    Utc::now().timestamp_millis().max(0) as u64
}
