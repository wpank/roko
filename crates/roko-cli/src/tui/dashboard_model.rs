//! The `TuiDashboardModel` learning snapshot and its rendering helpers.
//!
//! This module owns the best-effort learning snapshot that backs the
//! dashboard health, trends, efficiency, and operations pages. It was
//! extracted from `dashboard.rs` to reduce that file to a manageable size.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use roko_core::metric::{Headlines, TaskMetric, compute_headlines};
use roko_gate::adaptive_threshold::AdaptiveThresholds;
use roko_learn::aggregate::EfficiencyBucket;
use roko_learn::cascade_router::{CascadeStage, StageTransition};
use roko_learn::efficiency::AgentEfficiencyEvent;
use roko_learn::episode_logger::{Episode, EpisodeLogger};
use roko_learn::pattern_discovery::CrossEpisodeConsolidator;
use roko_learn::prompt_experiment::ExperimentStore;
use roko_learn::provider_health::{CircuitState, ProviderHealth};
use roko_learn::skill_library::Skill;
use roko_runtime::load_durable_runner_projection;

use super::dashboard::{
    build_agent_activity_snapshot, build_gate_results_page_data, file_stamp, format_duration_ms,
    format_elapsed_ms, load_efficiency_trend, load_gate_signal_summaries, load_recent_signals,
    now_ms, resolve_episodes_path, runner_task_outcomes_for_plan, runner_terminal_task_outcome,
    CASCADE_ROUTER_FILE, EFFICIENCY_FILE, EXPERIMENTS_FILE, GATE_THRESHOLDS_FILE,
    KNOWLEDGE_CONFIRMATIONS_FILE, KNOWLEDGE_FILE, LATENCY_STATS_FILE, LEARN_DIR, MEMORY_DIR,
    NEURO_DIR, PROVIDER_HEALTH_FILE, SKILLS_FILE, TASK_METRICS_FILE, FileStamp,
};
use super::dashboard_types::{
    AgentSummary, GateResultsPageData, GateTrend, KnowledgeBrowseEntry, SignalSummary,
    parse_efficiency_timestamp,
};
use super::display_utils::truncate as truncate_str;
use super::pages::PageScaffold;

/// Best-effort learning snapshot for dashboard rendering.
#[derive(Debug, Clone)]
pub struct TuiDashboardModel {
    pub(super) root: PathBuf,
    pub(super) episode_count: usize,
    pub(super) success_rate: Option<f64>,
    pub(super) average_cost_usd: Option<f64>,
    pub(super) average_wall_time_ms: Option<f64>,
    pub(super) task_metric_count: usize,
    pub(super) haiku_share: Option<f64>,
    pub(super) cache_hit_rate: Option<f64>,
    pub(super) headlines: Headlines,
    /// Raw efficiency events from `.roko/learn/efficiency.jsonl`.
    pub(super) efficiency_events: Vec<AgentEfficiencyEvent>,
    /// Hourly efficiency trend over the last 24 hours.
    pub(super) efficiency_trend: Vec<EfficiencyBucket>,
    /// Prompt experiment store from `.roko/learn/experiments.json`.
    pub(super) experiments: Option<ExperimentStore>,
    /// Adaptive gate thresholds from `.roko/learn/gate-thresholds.json`.
    pub(super) adaptive_thresholds: Option<AdaptiveThresholds>,
    /// Verify-results page data derived from signals and thresholds.
    pub(super) gate_results_page: GateResultsPageData,
    /// Most recent signals from `.roko/engrams.jsonl`.
    pub(super) recent_signals: Vec<SignalSummary>,
    /// Cascade router snapshot from `.roko/learn/cascade-router.json` (raw JSON).
    cascade_snapshot: Option<CascadeSnapshotData>,
    /// Last observed cascade-router file metadata.
    cascade_snapshot_stamp: FileStamp,
    /// Last observed experiments file metadata.
    experiments_stamp: FileStamp,
    /// Last observed gate-thresholds file metadata.
    adaptive_thresholds_stamp: FileStamp,
    /// Persisted skill-library snapshot from `.roko/learn/skills.json`.
    skills: Vec<Skill>,
    /// Last observed skills file metadata.
    skills_stamp: FileStamp,
    /// Optional persisted provider-health snapshot from `.roko/learn/provider-health.json`.
    provider_health: Option<ProviderHealthRegistrySnapshotData>,
    /// Last observed provider-health file metadata.
    provider_health_stamp: FileStamp,
    /// Latency stats from `.roko/learn/latency-stats.json`.
    latency_stats: Option<LatencyStatsData>,
    /// Knowledge-store counters derived from `.roko/neuro/*.jsonl`.
    knowledge_store: KnowledgeStoreSnapshot,
    /// Raw episodes kept for per-agent analysis.
    pub(super) episodes: Vec<Episode>,
}

/// Deserialized cascade router snapshot matching the private `CascadeSnapshot`.
#[derive(Debug, Clone, serde::Deserialize)]
struct CascadeSnapshotData {
    #[serde(default)]
    model_slugs: Vec<String>,
    #[serde(default)]
    confidence_stats: HashMap<String, PersistedModelStatsData>,
    #[serde(default)]
    total_observations: u64,
    #[serde(default)]
    stage_transitions: Vec<StageTransition>,
}

/// Per-model stats from the cascade router JSON.
#[derive(Debug, Clone, serde::Deserialize)]
struct PersistedModelStatsData {
    trials: u64,
    successes: u64,
}

/// Deserialized latency stats from `.roko/learn/latency-stats.json`.
#[derive(Debug, Clone, Default, serde::Deserialize)]
struct LatencyStatsData {
    #[serde(default)]
    entries: Vec<LatencyEntryData>,
}

/// Per-provider latency entry.
#[derive(Debug, Clone, serde::Deserialize)]
struct LatencyEntryData {
    #[serde(default)]
    provider: String,
    #[serde(default)]
    stats: LatencyStatsEntryData,
}

/// Latency statistics for one provider.
#[derive(Debug, Clone, Default, serde::Deserialize)]
struct LatencyStatsEntryData {
    #[serde(default)]
    recent_latencies: Vec<f64>,
}

#[derive(Debug, Clone, Default)]
struct LearningArtifactsSnapshot {
    cascade_stamp: FileStamp,
    experiments_stamp: FileStamp,
    gate_thresholds_stamp: FileStamp,
    skills: Vec<Skill>,
    skills_stamp: FileStamp,
    provider_health: Option<ProviderHealthRegistrySnapshotData>,
    provider_health_stamp: FileStamp,
    latency_stats: Option<LatencyStatsData>,
    knowledge_store: KnowledgeStoreSnapshot,
}

#[derive(Debug, Clone, Default)]
struct KnowledgeStoreSnapshot {
    total_records: usize,
    last_updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ProviderHealthRegistrySnapshotData {
    #[serde(default)]
    providers: HashMap<String, ProviderHealth>,
}

#[derive(Debug, Clone)]
struct LearningSubsystemRow {
    subsystem: &'static str,
    updates: String,
    last: String,
    health: String,
}

impl TuiDashboardModel {
    /// Load the learning snapshot from a workspace root.
    pub async fn load(root: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let root = resolve_snapshot_root(root.as_ref());
        let memory_dir = root.join(MEMORY_DIR);
        let learn_dir = root.join(LEARN_DIR);
        let episodes_path = resolve_episodes_path(&root);
        let task_metrics_path = memory_dir.join(TASK_METRICS_FILE);
        let signals_path = root.join(".roko").join("engrams.jsonl");

        let episodes_logger = EpisodeLogger::new(&episodes_path);
        let episodes = EpisodeLogger::read_all_lossy(episodes_logger.path())
            .await
            .map_err(std::io::Error::other)?;
        let task_metrics = read_task_metrics(&task_metrics_path).await?;

        // Load learning subsystem data (best-effort).
        let efficiency_path = learn_dir.join(EFFICIENCY_FILE);
        let efficiency_events = read_efficiency_events(&efficiency_path).await;
        let efficiency_trend = load_efficiency_trend(&efficiency_path);
        let experiments = load_json_opt::<ExperimentStore>(&learn_dir.join(EXPERIMENTS_FILE));
        let adaptive_thresholds =
            load_json_opt::<AdaptiveThresholds>(&learn_dir.join(GATE_THRESHOLDS_FILE));
        let gate_signals = load_gate_signal_summaries(&signals_path);
        let gate_results_page =
            build_gate_results_page_data(&gate_signals, adaptive_thresholds.as_ref());
        let recent_signals = load_recent_signals(&signals_path, 100);
        let cascade_path = learn_dir.join(CASCADE_ROUTER_FILE);
        let experiments_path = learn_dir.join(EXPERIMENTS_FILE);
        let thresholds_path = learn_dir.join(GATE_THRESHOLDS_FILE);
        let skills_path = learn_dir.join(SKILLS_FILE);
        let provider_health_path = learn_dir.join(PROVIDER_HEALTH_FILE);
        let cascade_snapshot = load_json_opt::<CascadeSnapshotData>(&cascade_path);
        let learning_artifacts = LearningArtifactsSnapshot {
            cascade_stamp: file_stamp(&cascade_path),
            experiments_stamp: file_stamp(&experiments_path),
            gate_thresholds_stamp: file_stamp(&thresholds_path),
            skills: load_json_opt::<Vec<Skill>>(&skills_path).unwrap_or_default(),
            skills_stamp: file_stamp(&skills_path),
            provider_health: load_json_opt::<ProviderHealthRegistrySnapshotData>(
                &provider_health_path,
            ),
            provider_health_stamp: file_stamp(&provider_health_path),
            latency_stats: load_json_opt::<LatencyStatsData>(&learn_dir.join(LATENCY_STATS_FILE)),
            knowledge_store: load_knowledge_store_snapshot(&root),
        };

        Ok(Self::from_records(
            root,
            &episodes,
            &task_metrics,
            efficiency_events,
            efficiency_trend,
            experiments,
            adaptive_thresholds,
            gate_results_page,
            recent_signals,
            cascade_snapshot,
            learning_artifacts,
        ))
    }

    /// Build a render projection from the core [`roko_core::dashboard_snapshot::DashboardSnapshot`]
    /// delivered through StateHub in connected mode.
    ///
    /// Learning-specific fields (experiments, cascade router, efficiency events)
    /// are left at their defaults here because the core snapshot does not carry
    /// them as typed values. In connected mode
    /// `TuiState::update_from_dashboard_snapshot` fills them in: the pushed
    /// learning payloads (efficiency trend, cascade-router and gate-threshold
    /// JSON) are parsed off the snapshot, and the per-event efficiency log plus
    /// the experiment store are tailed incrementally from the local
    /// `.roko/learn/` files.
    pub fn from_core_snapshot(
        snap: &roko_core::dashboard_snapshot::DashboardSnapshot,
        root: PathBuf,
    ) -> Self {
        let gate_pass_rate = {
            let total = snap.stats.gates_passed + snap.stats.gates_failed;
            if total > 0 {
                Some(snap.stats.gates_passed as f64 / total as f64)
            } else {
                None
            }
        };
        let episode_count = snap.episodes.len();
        let headlines = compute_headlines(&[]);

        Self {
            root,
            episode_count,
            success_rate: gate_pass_rate,
            average_cost_usd: None,
            average_wall_time_ms: None,
            task_metric_count: 0,
            haiku_share: None,
            cache_hit_rate: None,
            headlines,
            efficiency_events: Vec::new(),
            efficiency_trend: Vec::new(),
            experiments: None,
            adaptive_thresholds: None,
            gate_results_page: GateResultsPageData::default(),
            recent_signals: Vec::new(),
            cascade_snapshot: None,
            cascade_snapshot_stamp: FileStamp::default(),
            experiments_stamp: FileStamp::default(),
            adaptive_thresholds_stamp: FileStamp::default(),
            skills: Vec::new(),
            skills_stamp: FileStamp::default(),
            provider_health: None,
            provider_health_stamp: FileStamp::default(),
            latency_stats: None,
            knowledge_store: KnowledgeStoreSnapshot::default(),
            episodes: Vec::new(),
        }
    }

    pub(super) fn empty(root: PathBuf) -> Self {
        Self::from_records(
            root,
            &[],
            &[],
            Vec::new(),
            Vec::new(),
            None,
            None,
            GateResultsPageData::default(),
            Vec::new(),
            None,
            LearningArtifactsSnapshot::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn from_records(
        root: PathBuf,
        episodes: &[Episode],
        task_metrics: &[TaskMetric],
        efficiency_events: Vec<AgentEfficiencyEvent>,
        efficiency_trend: Vec<EfficiencyBucket>,
        experiments: Option<ExperimentStore>,
        adaptive_thresholds: Option<AdaptiveThresholds>,
        gate_results_page: GateResultsPageData,
        recent_signals: Vec<SignalSummary>,
        cascade_snapshot: Option<CascadeSnapshotData>,
        learning_artifacts: LearningArtifactsSnapshot,
    ) -> Self {
        let episode_count = episodes.len();
        let success_rate = if episode_count == 0 {
            None
        } else {
            let successes = episodes.iter().filter(|episode| episode.success).count();
            Some(count_to_f64(successes) / count_to_f64(episode_count))
        };
        let average_cost_usd = if episode_count == 0 {
            None
        } else {
            Some(
                episodes
                    .iter()
                    .map(|episode| episode.usage.cost_usd)
                    .sum::<f64>()
                    / count_to_f64(episode_count),
            )
        };
        let average_wall_time_ms = if episode_count == 0 {
            None
        } else {
            Some(
                episodes
                    .iter()
                    .map(|episode| wall_ms_to_f64(episode.usage.wall_ms))
                    .sum::<f64>()
                    / count_to_f64(episode_count),
            )
        };

        let task_metric_count = task_metrics.len();
        let haiku_share = if task_metric_count == 0 {
            None
        } else {
            let haiku = task_metrics
                .iter()
                .filter(|metric| metric.model.to_ascii_lowercase().contains("haiku"))
                .count();
            Some(count_to_f64(haiku) / count_to_f64(task_metric_count))
        };
        let cache_hit_rate = if task_metric_count == 0 {
            None
        } else {
            Some(
                task_metrics
                    .iter()
                    .map(|metric| metric.cache_hit_rate)
                    .sum::<f64>()
                    / count_to_f64(task_metric_count),
            )
        };
        let headlines = compute_headlines(task_metrics);

        Self {
            root,
            episode_count,
            success_rate,
            average_cost_usd,
            average_wall_time_ms,
            task_metric_count,
            haiku_share,
            cache_hit_rate,
            headlines,
            efficiency_events,
            efficiency_trend,
            experiments,
            adaptive_thresholds,
            gate_results_page,
            recent_signals,
            cascade_snapshot,
            cascade_snapshot_stamp: learning_artifacts.cascade_stamp,
            experiments_stamp: learning_artifacts.experiments_stamp,
            adaptive_thresholds_stamp: learning_artifacts.gate_thresholds_stamp,
            skills: learning_artifacts.skills,
            skills_stamp: learning_artifacts.skills_stamp,
            provider_health: learning_artifacts.provider_health,
            provider_health_stamp: learning_artifacts.provider_health_stamp,
            latency_stats: learning_artifacts.latency_stats,
            knowledge_store: learning_artifacts.knowledge_store,
            episodes: episodes.to_vec(),
        }
    }

    pub(super) fn render_health_page(&self, page: &PageScaffold) -> Option<String> {
        if self.episode_count == 0 {
            return None;
        }

        let mut out = String::new();
        let _ = writeln!(out, "{} ({})", page.title, page.id.slug());
        let _ = writeln!(out, "group: {}", page.id.group());
        let _ = writeln!(out, "intent: {}", page.intent);
        let _ = writeln!(
            out,
            "source: {}",
            resolve_episodes_path(&self.root).display()
        );
        let _ = writeln!(out, "episodes: {}", self.episode_count);
        let _ = writeln!(
            out,
            "success rate: {}",
            format_pct(self.success_rate.unwrap_or(0.0))
        );
        let _ = writeln!(
            out,
            "average cost: {}",
            format_usd(self.average_cost_usd.unwrap_or(0.0))
        );
        let _ = writeln!(
            out,
            "average wall time: {}",
            format_ms(self.average_wall_time_ms.unwrap_or(0.0))
        );
        if let Some(hit_rate) = self.cache_hit_rate {
            let _ = writeln!(out, "cache hit rate: {}", format_pct(hit_rate));
        }
        if let Some(haiku_share) = self.haiku_share {
            let _ = writeln!(out, "haiku share: {}", format_pct(haiku_share));
        }
        if self.task_metric_count > 0 {
            let _ = writeln!(out, "task metrics: {}", self.task_metric_count);
        }
        out.push_str("widgets (scaffold):\n");
        for widget in &page.widgets {
            let _ = writeln!(out, "{}", widget.render_line());
        }
        Some(out)
    }

    pub(super) fn render_trends_page(&self, page: &PageScaffold) -> Option<String> {
        if self.task_metric_count == 0 {
            return None;
        }

        let mut out = String::new();
        let _ = writeln!(out, "{} ({})", page.title, page.id.slug());
        let _ = writeln!(out, "group: {}", page.id.group());
        let _ = writeln!(out, "intent: {}", page.intent);
        let _ = writeln!(
            out,
            "source: {}/{}",
            self.root.join(MEMORY_DIR).display(),
            TASK_METRICS_FILE
        );
        let _ = writeln!(out, "task metrics: {}", self.task_metric_count);
        let _ = writeln!(
            out,
            "first-attempt pass rate: {}",
            format_pct(self.headlines.first_attempt_pass_rate)
        );
        let _ = writeln!(
            out,
            "avg iterations per plan: {}",
            format_float(self.headlines.avg_iterations_per_plan)
        );
        let _ = writeln!(
            out,
            "avg cost per plan: {}",
            format_usd(self.headlines.avg_cost_per_plan)
        );
        let _ = writeln!(
            out,
            "avg input tokens per spawn: {}",
            format_float(self.headlines.avg_input_tokens_per_spawn)
        );
        let _ = writeln!(out, "plans: {}", self.headlines.n_plans);
        let _ = writeln!(out, "records: {}", self.headlines.n_records);
        if let Some(hit_rate) = self.cache_hit_rate {
            let _ = writeln!(out, "cache hit rate: {}", format_pct(hit_rate));
        }
        if let Some(haiku_share) = self.haiku_share {
            let _ = writeln!(out, "haiku share: {}", format_pct(haiku_share));
        }
        out.push_str("headlines:\n");
        let _ = writeln!(
            out,
            "- first_attempt_pass_rate: {}",
            format_pct(self.headlines.first_attempt_pass_rate)
        );
        let _ = writeln!(
            out,
            "- avg_iterations_per_plan: {}",
            format_float(self.headlines.avg_iterations_per_plan)
        );
        let _ = writeln!(
            out,
            "- avg_cost_per_plan: {}",
            format_usd(self.headlines.avg_cost_per_plan)
        );
        let _ = writeln!(
            out,
            "- avg_input_tokens_per_spawn: {}",
            format_float(self.headlines.avg_input_tokens_per_spawn)
        );
        out.push_str("widgets (scaffold):\n");
        for widget in &page.widgets {
            let _ = writeln!(out, "{}", widget.render_line());
        }
        Some(out)
    }

    // ── Efficiency pages ────────────────────────────────────────────

    pub(super) fn render_correlations_page(&self, page: &PageScaffold) -> Option<String> {
        if self.efficiency_events.is_empty() {
            return None;
        }
        let mut out = page_header(page);
        let _ = writeln!(out, "events: {}", self.efficiency_events.len());
        let _ = writeln!(out);

        // prompt_tokens vs gate_passed histogram.
        // Bucket by prompt token count in 1k increments.
        let mut buckets: BTreeMap<u64, (u32, u32)> = BTreeMap::new(); // bucket -> (total, passed)
        for ev in &self.efficiency_events {
            let bucket = ev.total_prompt_tokens / 1000 * 1000; // round down to nearest 1k
            let entry = buckets.entry(bucket).or_default();
            entry.0 += 1;
            if ev.gate_passed == Some(true) {
                entry.1 += 1;
            }
        }
        let _ = writeln!(out, "prompt tokens vs pass rate:");
        let _ = writeln!(
            out,
            "  {:>10}  {:>6}  {:>9}  bar",
            "tokens", "count", "pass rate"
        );
        for (bucket, (total, passed)) in &buckets {
            let rate = if *total > 0 {
                *passed as f64 / *total as f64
            } else {
                0.0
            };
            let bar_len = (rate * 20.0).round() as usize;
            let bar: String = std::iter::repeat_n('#', bar_len).collect();
            let _ = writeln!(
                out,
                "  {:>9}k  {:>6}  {:>8}  {}",
                bucket / 1000,
                total,
                format_pct(rate),
                bar
            );
        }

        // cost vs pass rate.
        let _ = writeln!(out);
        let _ = writeln!(out, "cost vs pass rate:");
        let cost_buckets: Vec<(f64, &str)> = vec![
            (0.001, "<$0.001"),
            (0.01, "<$0.01"),
            (0.1, "<$0.10"),
            (f64::MAX, ">=$0.10"),
        ];
        let _ = writeln!(out, "  {:>10}  {:>6}  {:>9}", "range", "count", "pass rate");
        let mut prev = 0.0_f64;
        for (threshold, label) in &cost_buckets {
            let matching: Vec<&AgentEfficiencyEvent> = self
                .efficiency_events
                .iter()
                .filter(|e| e.cost_usd >= prev && e.cost_usd < *threshold)
                .collect();
            let total = matching.len();
            let passed = matching
                .iter()
                .filter(|e| e.gate_passed == Some(true))
                .count();
            if total > 0 {
                let rate = count_to_f64(passed) / count_to_f64(total);
                let _ = writeln!(
                    out,
                    "  {:>10}  {:>6}  {:>9}",
                    label,
                    total,
                    format_pct(rate)
                );
            }
            prev = *threshold;
        }

        Some(out)
    }

    pub(super) fn render_parameters_page(&self, page: &PageScaffold) -> Option<String> {
        let has_thresholds = self.adaptive_thresholds.is_some();
        let has_cascade = self.cascade_snapshot.is_some();
        if !has_thresholds && !has_cascade {
            return None;
        }

        let mut out = page_header(page);

        // Cascade router model weights.
        if let Some(snap) = &self.cascade_snapshot {
            let _ = writeln!(out, "cascade router:");
            let _ = writeln!(out, "  registered models: {}", snap.model_slugs.len());
            for slug in &snap.model_slugs {
                let _ = writeln!(out, "    - {slug}");
            }
            if !snap.confidence_stats.is_empty() {
                let _ = writeln!(out, "  confidence-stage stats:");
                let _ = writeln!(
                    out,
                    "    {:>20}  {:>8}  {:>8}  {:>9}",
                    "model", "trials", "passes", "pass rate"
                );
                let mut stats: Vec<_> = snap.confidence_stats.iter().collect();
                stats.sort_by(|a, b| b.1.trials.cmp(&a.1.trials));
                for (model, s) in stats {
                    #[allow(clippy::cast_precision_loss)]
                    let rate = if s.trials > 0 {
                        s.successes as f64 / s.trials as f64
                    } else {
                        0.0
                    };
                    let _ = writeln!(
                        out,
                        "    {:>20}  {:>8}  {:>8}  {:>9}",
                        model,
                        s.trials,
                        s.successes,
                        format_pct(rate)
                    );
                }
            }
            let _ = writeln!(out);
        }

        // Adaptive gate thresholds.
        if let Some(at) = &self.adaptive_thresholds {
            let _ = writeln!(out, "adaptive gate thresholds:");
            let _ = writeln!(
                out,
                "  {:>5}  {:>12}  {:>6}  {:>12}  {:>4}",
                "rung", "ema pass rate", "obs", "consec pass", "skip"
            );
            let mut rungs: Vec<_> = at.all_rungs().collect();
            rungs.sort_by_key(|(r, _)| *r);
            for (rung, stats) in rungs {
                let skip = if at.should_skip_rung(*rung) {
                    "yes"
                } else {
                    "no"
                };
                let _ = writeln!(
                    out,
                    "  {:>5}  {:>12}  {:>6}  {:>12}  {:>4}",
                    rung,
                    format_pct(stats.ema_pass_rate),
                    stats.total_observations,
                    stats.consecutive_passes,
                    skip
                );
            }
        }

        Some(out)
    }

    pub(super) fn render_gate_results_page(&self, page: &PageScaffold) -> Option<String> {
        if self.gate_results_page.gate_rows.is_empty()
            && self.gate_results_page.threshold_rows.is_empty()
            && self.gate_results_page.failure_rows.is_empty()
        {
            return None;
        }

        let mut out = page_header(page);
        let _ = writeln!(
            out,
            "source: {}/engrams.jsonl",
            self.root.join(".roko").display()
        );
        let _ = writeln!(
            out,
            "source: {}/gate-thresholds.json",
            self.root.join(LEARN_DIR).display()
        );

        let _ = writeln!(out);
        let _ = writeln!(out, "gate summary:");
        let _ = writeln!(
            out,
            "  {:>18}  {:>10}  {:>9}  {:>12}  {:>12}",
            "gate", "runs", "pass rate", "avg duration", "last run"
        );
        for row in &self.gate_results_page.gate_rows {
            let _ = writeln!(
                out,
                "  {:>18}  {:>10}  {:>9}  {:>12}  {:>12}",
                truncate_str(&row.gate_name, 18),
                row.total_runs,
                format_pct(row.pass_rate),
                format_ms(row.avg_duration_ms),
                truncate_str(&row.last_run, 12)
            );
        }

        let _ = writeln!(out);
        let _ = writeln!(out, "adaptive thresholds:");
        let _ = writeln!(
            out,
            "  {:>6}  {:>10}  {:>9}  {:>8}",
            "rung", "threshold", "ema", "trend"
        );
        for row in &self.gate_results_page.threshold_rows {
            let trend = match row.trend {
                GateTrend::Up => "↑",
                GateTrend::Flat => "→",
                GateTrend::Down => "↓",
            };
            let _ = writeln!(
                out,
                "  {:>6}  {:>10}  {:>9}  {:>8}",
                row.rung,
                row.current_threshold,
                format_pct(row.ema_pass_rate),
                trend
            );
        }

        let _ = writeln!(out);
        let _ = writeln!(out, "recent gate failures:");
        if self.gate_results_page.failure_rows.is_empty() {
            let _ = writeln!(out, "  (none)");
        } else {
            for row in &self.gate_results_page.failure_rows {
                let _ = writeln!(
                    out,
                    "  {} | {} | {}",
                    row.task_id,
                    row.gate_name,
                    truncate_str(&row.error_excerpt, 120)
                );
            }
        }

        Some(out)
    }

    pub(super) fn render_experiments_page(&self, page: &PageScaffold) -> Option<String> {
        let store = self.experiments.as_ref()?;
        if store.running_count() == 0 && store.concluded_count() == 0 {
            return None;
        }

        let mut out = page_header(page);
        let _ = writeln!(
            out,
            "experiments: {} running, {} concluded",
            store.running_count(),
            store.concluded_count()
        );
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "  {:>20}  {:>10}  {:>8}  {:>10}  verdict",
            "section", "status", "trials", "arms"
        );
        for exp in store.iter() {
            let total_trials: u64 = exp.stats.values().map(|s| s.trials).sum();
            let verdict = exp
                .winner_id
                .as_ref()
                .map_or_else(|| "-".to_string(), |winner| format!("winner={winner}"));
            let _ = writeln!(
                out,
                "  {:>20}  {:>10}  {:>8}  {:>10}  {}",
                exp.section_name,
                format!("{:?}", exp.status),
                total_trials,
                exp.variants.len(),
                verdict
            );
        }

        Some(out)
    }

    pub(super) fn render_learning_page(&self, _page: &PageScaffold) -> Option<String> {
        let observations = cascade_observations_snapshot(self.cascade_snapshot.as_ref());
        let stage = cascade_stage_for_observations(observations);
        let last_transition = self
            .cascade_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.stage_transitions.last());
        let gate_threshold_updates: u64 = self
            .adaptive_thresholds
            .as_ref()
            .map(|thresholds| {
                thresholds
                    .all_rungs()
                    .map(|(_, stats)| stats.total_observations)
                    .sum()
            })
            .unwrap_or(0);
        let running_experiments = self
            .experiments
            .as_ref()
            .map(ExperimentStore::running_count)
            .unwrap_or(0);
        let pattern_count = CrossEpisodeConsolidator::default()
            .discover(&self.episodes)
            .meta_pattern_count;
        let provider_summary = learning_provider_health_summary(
            &self.provider_health,
            self.provider_health_stamp,
            &self.efficiency_events,
        );

        let rows = vec![
            LearningSubsystemRow {
                subsystem: "CascadeRouter",
                updates: observations.to_string(),
                last: format_file_age(self.cascade_snapshot_stamp),
                health: format!(
                    "● {}",
                    match stage {
                        CascadeStage::Static => "warming",
                        CascadeStage::Confidence => "calibrating",
                        CascadeStage::Ucb => "learning",
                    }
                ),
            },
            LearningSubsystemRow {
                subsystem: "GateThresholds",
                updates: gate_threshold_updates.to_string(),
                last: format_file_age(self.adaptive_thresholds_stamp),
                health: format!(
                    "● {}",
                    if gate_threshold_updates > 0 {
                        "stable"
                    } else {
                        "pending"
                    }
                ),
            },
            LearningSubsystemRow {
                subsystem: "Experiments",
                updates: format!("{running_experiments} running"),
                last: format_file_age(self.experiments_stamp),
                health: format!(
                    "● {}",
                    if running_experiments > 0 {
                        "active"
                    } else {
                        "idle"
                    }
                ),
            },
            LearningSubsystemRow {
                subsystem: "SkillLibrary",
                updates: format!("{} skills", self.skills.len()),
                last: learning_skills_last_updated(&self.skills, self.skills_stamp),
                health: format!(
                    "● {}",
                    if self.skills.is_empty() {
                        "empty"
                    } else {
                        "growing"
                    }
                ),
            },
            LearningSubsystemRow {
                subsystem: "PatternMiner",
                updates: format!("{pattern_count} patterns"),
                last: learning_patterns_last_updated(&self.episodes),
                health: format!("● {}", if pattern_count > 0 { "mining" } else { "idle" }),
            },
            LearningSubsystemRow {
                subsystem: "ProviderHealth",
                updates: format!("{} providers", provider_summary.provider_count),
                last: provider_summary.last_updated,
                health: format!("● {}", provider_summary.health),
            },
            LearningSubsystemRow {
                subsystem: "KnowledgeStore",
                updates: learning_knowledge_updates(&self.knowledge_store),
                last: format_relative_timestamp(self.knowledge_store.last_updated),
                health: if self.knowledge_store.total_records > 0 {
                    String::from("● learning")
                } else {
                    String::new()
                },
            },
        ];

        let subsystem_header = format!(
            "{:<14} {:<9} {:<8} {}",
            "Subsystem", "Updates", "Last", "Health"
        );
        let mut lines = vec![
            String::new(),
            format!(
                "  Stage: {} ({} observations)",
                cascade_stage_label(stage),
                observations
            ),
            format!(
                "  Last transition: {}",
                last_transition.map_or_else(
                    || String::from("none yet"),
                    |transition| format!(
                        "{} -> {} at obs {}",
                        cascade_stage_label(transition.from),
                        cascade_stage_label(transition.to),
                        transition.observations
                    )
                )
            ),
            String::new(),
            format!("  {subsystem_header}"),
        ];
        lines.extend(rows.into_iter().map(|row| {
            format!(
                "  {:<14} {:<9} {:<8} {}",
                row.subsystem, row.updates, row.last, row.health
            )
        }));
        lines.extend(render_learning_trend_lines(&self.efficiency_trend));
        lines.extend([
            String::new(),
            String::from("  Feedback Loops:  6/8 connected"),
            String::from("  Missing: GateFail->Replan, SectionEffect->Prompt"),
            String::new(),
        ]);

        Some(render_boxed_panel("Learning System Status", &lines))
    }

    pub(super) fn render_optimizer_page(&self, page: &PageScaffold) -> Option<String> {
        let at = self.adaptive_thresholds.as_ref()?;
        let mut out = page_header(page);

        // Show EMA confidence per rung.
        let _ = writeln!(out, "gate EMA confidence by rung:");
        let _ = writeln!(
            out,
            "  {:>5}  {:>12}  {:>12}  {:>6}",
            "rung", "ema pass", "observations", "retries"
        );
        let mut rungs: Vec<_> = at.all_rungs().collect();
        rungs.sort_by_key(|(r, _)| *r);
        for (rung, stats) in &rungs {
            let retries = at.suggested_max_retries(**rung);
            let _ = writeln!(
                out,
                "  {:>5}  {:>12}  {:>12}  {:>6}",
                rung,
                format_pct(stats.ema_pass_rate),
                stats.total_observations,
                retries
            );
        }

        // Overall optimization state.
        let _ = writeln!(out);
        let total_obs: u64 = rungs.iter().map(|(_, s)| s.total_observations).sum();
        let avg_ema: f64 = if rungs.is_empty() {
            0.0
        } else {
            rungs.iter().map(|(_, s)| s.ema_pass_rate).sum::<f64>() / count_to_f64(rungs.len())
        };
        let _ = writeln!(out, "optimization cycle:");
        let _ = writeln!(out, "  total observations: {total_obs}");
        let _ = writeln!(out, "  avg ema pass rate: {}", format_pct(avg_ema));
        let skippable: usize = rungs
            .iter()
            .filter(|(r, _)| at.should_skip_rung(**r))
            .count();
        let _ = writeln!(out, "  skippable rungs: {} / {}", skippable, rungs.len());

        // Experiment store summary if present.
        if let Some(store) = &self.experiments {
            let _ = writeln!(out, "  active experiments: {}", store.running_count());
            let _ = writeln!(out, "  concluded experiments: {}", store.concluded_count());
        }

        Some(out)
    }

    // ── Operations pages ────────────────────────────────────────────

    pub(super) fn render_agent_status_page(&self, page: &PageScaffold) -> Option<String> {
        let mut active_agents = Vec::new();
        let mut seen_agents = HashSet::new();
        for episode in &self.episodes {
            if seen_agents.insert(episode.agent_id.clone()) {
                active_agents.push(AgentSummary {
                    id: episode.agent_id.clone(),
                    label: episode.agent_id.clone(),
                    plan_id: None,
                    status: String::from("active"),
                });
            }
        }
        let snapshot = build_agent_activity_snapshot(&active_agents, &self.efficiency_events)?;

        let mut out = page_header(page);
        let _ = writeln!(out, "active agents:");
        let _ = writeln!(
            out,
            "  {:>20}  {:>14}  {:>16}  {:>12}  {:>5}  {:>12}  {:>10}  {:>10}",
            "agent id", "model", "task", "role", "turns", "tokens used", "cost", "uptime"
        );
        for row in &snapshot.active_agents {
            let _ = writeln!(
                out,
                "  {:>20}  {:>14}  {:>16}  {:>12}  {:>5}  {:>12}  {:>10}  {:>10}",
                truncate_str(&row.agent_id, 20),
                truncate_str(&row.model, 14),
                truncate_str(&row.task, 16),
                truncate_str(&row.role, 12),
                row.turns,
                row.tokens_used,
                format_usd(row.cost_usd),
                format_duration_ms(row.uptime_ms)
            );
        }

        let _ = writeln!(out);
        let _ = writeln!(out, "model distribution:");
        let _ = writeln!(out, "  {:>10}  {:>8}", "model", "count");
        for row in &snapshot.model_usage {
            let _ = writeln!(out, "  {:>10}  {:>8}", row.label, row.count);
        }

        let _ = writeln!(out);
        let _ = writeln!(out, "cost breakdown:");
        let _ = writeln!(
            out,
            "  {:>20}  {:>12}  {:>12}  {:>12}",
            "model", "input tokens", "output tokens", "cost"
        );
        for row in &snapshot.cost_rows {
            // Estimated costs are marked so an unknown cost never renders as
            // a bare "$0.00".
            let cost = if row.cost_estimated {
                format!("~{}", format_usd(row.cost_usd))
            } else {
                format_usd(row.cost_usd)
            };
            let _ = writeln!(
                out,
                "  {:>20}  {:>12}  {:>12}  {:>12}",
                truncate_str(&row.model, 20),
                row.input_tokens,
                row.output_tokens,
                cost
            );
        }
        if snapshot.cost_rows.iter().any(|row| row.cost_estimated) {
            let _ = writeln!(out, "  (~ cost estimated from registry rates)");
        }
        let _ = writeln!(
            out,
            "  total session cost: {}",
            format_usd(snapshot.total_session_cost)
        );

        Some(out)
    }

    pub(super) fn render_plan_view_page(&self, page: &PageScaffold) -> Option<String> {
        let mut out = page_header(page);
        let projection = match load_durable_runner_projection(&self.root) {
            Ok(Some(projection)) => projection,
            Ok(None) => {
                let _ = writeln!(out, "source: missing");
                let _ = writeln!(out, "(no durable Runner projection)");
                return Some(out);
            }
            Err(error) => {
                let _ = writeln!(out, "source: invalid");
                let _ = writeln!(out, "error: {error}");
                return Some(out);
            }
        };
        let state = projection.executor_projection;
        let _ = writeln!(
            out,
            "source: {} ({})",
            projection.source_path.display(),
            projection.source
        );
        let _ = writeln!(out, "generation: {}", projection.generation);

        // Canonical snapshots expose task lifecycle as an ID-keyed map.
        let canonical_tasks = state
            .pointer("/_runner_projection/lifecycle/tasks")
            .and_then(Value::as_object)
            .map(|tasks| {
                let runner = state
                    .get("_runner_projection")
                    .expect("lifecycle tasks require a Runner projection");
                let mut tasks = tasks
                    .values()
                    .filter_map(|task| {
                        let plan_id = task.get("plan_id")?.as_str()?;
                        let task_id = task.get("task_id")?.as_str()?;
                        let mut normalized = task.clone();
                        if let Some(status) = runner_terminal_task_outcome(runner, plan_id, task_id)
                        {
                            normalized["status"] = Value::String(status.to_string());
                        }
                        Some(normalized)
                    })
                    .collect::<Vec<_>>();
                tasks.sort_by(|left, right| {
                    left.get("plan_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .cmp(right.get("plan_id").and_then(Value::as_str).unwrap_or(""))
                        .then_with(|| {
                            left.get("task_id")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .cmp(right.get("task_id").and_then(Value::as_str).unwrap_or(""))
                        })
                });
                tasks
            })
            .or_else(|| {
                state.get("_runner_projection")?;
                let mut tasks = Vec::new();
                for plan_id in state
                    .get("plan_states")
                    .and_then(Value::as_object)
                    .into_iter()
                    .flat_map(|plans| plans.keys())
                {
                    tasks.extend(
                        runner_task_outcomes_for_plan(&state, plan_id)
                            .into_iter()
                            .flatten()
                            .map(|(task_id, status)| {
                                serde_json::json!({
                                    "plan_id": plan_id,
                                    "task_id": task_id,
                                    "status": status
                                })
                            }),
                    );
                }
                Some(tasks)
            });
        let legacy_tasks = state.get("tasks").and_then(Value::as_array).cloned();
        if let Some(tasks) = canonical_tasks.or(legacy_tasks) {
            let total = tasks.len();
            let done = tasks
                .iter()
                .filter(|task| {
                    matches!(
                        task.get("status").and_then(Value::as_str),
                        Some("done" | "passed")
                    )
                })
                .count();
            let failed = tasks
                .iter()
                .filter(|task| {
                    matches!(
                        task.get("status").and_then(Value::as_str),
                        Some("failed" | "exhausted" | "cancelled" | "timed_out")
                    )
                })
                .count();
            let skipped = tasks
                .iter()
                .filter(|task| task.get("status").and_then(Value::as_str) == Some("skipped"))
                .count();
            let running = tasks
                .iter()
                .filter(|task| {
                    matches!(
                        task.get("status").and_then(Value::as_str),
                        Some("started" | "running" | "retrying")
                    )
                })
                .count();
            let pending = total.saturating_sub(done + failed + skipped + running);
            let _ = writeln!(out);
            let _ = writeln!(
                out,
                "tasks: {total} total, {done} done, {failed} failed, {skipped} skipped, {running} running, {pending} pending"
            );
            let _ = writeln!(out);

            // Task table.
            let _ = writeln!(out, "  {:>4}  {:>10}  {:>30}", "idx", "status", "id");
            for (i, task) in tasks.iter().enumerate() {
                let status = task
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");
                let id = task
                    .get("task_id")
                    .or_else(|| task.get("id"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("-");
                let _ = writeln!(out, "  {:>4}  {:>10}  {:>30}", i, status, id);
            }
        } else {
            let _ = writeln!(out, "(no task data in executor state)");
        }

        Some(out)
    }

    pub(super) fn render_log_view_page(&self, page: &PageScaffold) -> Option<String> {
        let signals_path = self.root.join(".roko").join("engrams.jsonl");
        let episodes_path = resolve_episodes_path(&self.root);

        let signals_exist = signals_path.exists();
        let episodes_exist = episodes_path.exists();
        if !signals_exist && !episodes_exist && self.episodes.is_empty() {
            return None;
        }

        let mut out = page_header(page);

        // Show last N episodes.
        let tail_n = 20;
        let _ = writeln!(out, "recent episodes (last {tail_n}):");
        let start = self.episodes.len().saturating_sub(tail_n);
        if self.episodes.is_empty() {
            let _ = writeln!(out, "  (none)");
        } else {
            let _ = writeln!(
                out,
                "  {:>20}  {:>24}  {:>8}  {:>9}  {:>10}",
                "timestamp", "agent", "task", "success", "cost"
            );
            for ep in &self.episodes[start..] {
                let ts = ep.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
                let _ = writeln!(
                    out,
                    "  {:>20}  {:>24}  {:>8}  {:>9}  {:>10}",
                    ts,
                    truncate_str(&ep.agent_id, 24),
                    truncate_str(&ep.task_id, 8),
                    if ep.success { "pass" } else { "FAIL" },
                    format_usd(ep.usage.cost_usd)
                );
            }
        }

        // Show last N signals if the file exists.
        if signals_exist {
            let _ = writeln!(out);
            let _ = writeln!(out, "recent signals (last {tail_n}):");
            if let Ok(text) = std::fs::read_to_string(&signals_path) {
                let lines: Vec<&str> = text.lines().collect();
                let start = lines.len().saturating_sub(tail_n);
                for line in &lines[start..] {
                    let _ = writeln!(out, "  {line}");
                }
            }
        }

        Some(out)
    }

    pub(super) fn render_signals_page(&self, page: &PageScaffold) -> Option<String> {
        if self.recent_signals.is_empty() {
            return None;
        }

        let mut signals = self.recent_signals.clone();
        signals.sort_by(|a, b| {
            b.created_at_ms
                .cmp(&a.created_at_ms)
                .then_with(|| a.id.cmp(&b.id))
        });

        let mut out = page_header(page);
        let _ = writeln!(
            out,
            "source: {}/engrams.jsonl",
            self.root.join(".roko").display()
        );
        let _ = writeln!(out, "window: last {} signals", signals.len());

        let _ = writeln!(out);
        let _ = writeln!(out, "recent signals:");
        let _ = writeln!(
            out,
            "  {:>8}  {:>18}  {:>18}  {:>60}",
            "time", "kind", "plan/task", "payload preview"
        );
        for signal in signals.iter().take(20) {
            let time = signal_relative_age(signal.created_at_ms);
            let plan_task = signal
                .plan_id
                .as_deref()
                .or(signal.task_id.as_deref())
                .unwrap_or("-");
            let _ = writeln!(
                out,
                "  {:>8}  {:>18}  {:>18}  {:>60}",
                truncate_str(&time, 8),
                truncate_str(&signal.kind, 18),
                truncate_str(plan_task, 18),
                truncate_str(&signal.payload_preview, 60)
            );
        }

        let _ = writeln!(out);
        let _ = writeln!(out, "signal kind distribution:");
        let distribution = signal_kind_distribution(&signals);
        if distribution.is_empty() {
            let _ = writeln!(out, "  (none)");
        } else {
            for (kind, count) in distribution {
                let _ = writeln!(out, "  {:>18}  {:>6}", kind, count);
            }
        }

        let _ = writeln!(out);
        let _ = writeln!(out, "signal DAG explorer:");
        if let Some(selected) = signals.first() {
            let _ = writeln!(
                out,
                "  selected: {} ({})",
                truncate_str(&selected.kind, 24),
                truncate_str(&selected.id, 16)
            );
            for (depth, node) in signal_parent_chain(&signals, selected)
                .into_iter()
                .enumerate()
            {
                let indent = "  ".repeat(depth + 1);
                let label = match node.signal {
                    Some(signal) => format!(
                        "{} [{}] {}",
                        truncate_str(&signal.kind, 24),
                        truncate_str(&signal.id, 16),
                        signal_relative_age(signal.created_at_ms)
                    ),
                    None => truncate_str(&node.hash, 48),
                };
                let _ = writeln!(out, "{indent}- {label}");
            }
        }

        Some(out)
    }

    pub(super) fn render_config_view_page(&self, page: &PageScaffold) -> Option<String> {
        let config_path = self.root.join("roko.toml");
        let text = std::fs::read_to_string(&config_path).ok()?;

        let mut out = page_header(page);
        let _ = writeln!(out, "source: {}", config_path.display());
        let _ = writeln!(out);

        // Render with section annotations.
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let _ = writeln!(out, "## {line}");
            } else {
                let _ = writeln!(out, "  {line}");
            }
        }

        Some(out)
    }

    pub(super) fn render_provider_health_page(&self, page: &PageScaffold) -> Option<String> {
        let ph = self.provider_health.as_ref()?;
        if ph.providers.is_empty() {
            return None;
        }

        // Build a lookup of latency p50 per provider.
        let latency_p50: HashMap<&str, f64> = self
            .latency_stats
            .as_ref()
            .map(|ls| {
                ls.entries
                    .iter()
                    .filter_map(|entry| {
                        let p50 = percentile_ms(&entry.stats.recent_latencies, 50.0)?;
                        Some((entry.provider.as_str(), p50))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut out = page_header(page);

        // Sort providers alphabetically for stable output.
        let mut providers: Vec<_> = ph.providers.iter().collect();
        providers.sort_by_key(|(name, _)| name.as_str());

        // Aggregate summary totals.
        let total_requests: u64 = providers.iter().map(|(_, p)| p.total_requests).sum();
        let total_failures: u64 = providers.iter().map(|(_, p)| p.total_failures).sum();

        for (name, entry) in &providers {
            let state_symbol = match entry.state {
                CircuitState::Closed => "\u{25cf} CLOSED",
                CircuitState::HalfOpen => "\u{25d1} HALF-OPEN",
                CircuitState::Open => "\u{25cb} OPEN",
            };
            let _ = writeln!(out, "  {name}: {state_symbol}");
            if let Some(p50) = latency_p50.get(name.as_str()) {
                let _ = writeln!(out, "    p50: {}", format_latency_seconds(*p50));
            }
            let _ = writeln!(
                out,
                "    requests: {}, failures: {}",
                entry.total_requests, entry.total_failures
            );
        }

        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "summary: {} requests, {} failures",
            total_requests, total_failures
        );

        Some(out)
    }

    #[allow(clippy::cast_precision_loss)]
    pub(super) fn render_model_comparison_page(&self, page: &PageScaffold) -> Option<String> {
        let cascade = self.cascade_snapshot.as_ref()?;
        if cascade.confidence_stats.is_empty() {
            return None;
        }

        let mut out = page_header(page);
        let _ = writeln!(out, "models: {}", cascade.model_slugs.len());

        // Table of model stats.
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "  {:>24}  {:>8}  {:>8}  {:>9}",
            "model", "trials", "passes", "pass rate"
        );
        let mut stats: Vec<_> = cascade.confidence_stats.iter().collect();
        stats.sort_by(|a, b| b.1.trials.cmp(&a.1.trials));
        for (model, s) in &stats {
            let rate = if s.trials > 0 {
                s.successes as f64 / s.trials as f64
            } else {
                0.0
            };
            let _ = writeln!(
                out,
                "  {:>24}  {:>8}  {:>8}  {:>9}",
                model,
                s.trials,
                s.successes,
                format_pct(rate)
            );
        }

        // Pareto frontier: a model is dominated if another model has both a
        // higher (or equal) pass rate AND fewer (or equal) trials (proxy for
        // cost).  We report dominated models with the model that dominates them.
        let _ = writeln!(out);
        let _ = writeln!(out, "Pareto frontier:");
        let mut model_rates: Vec<(&String, f64, u64)> = cascade
            .confidence_stats
            .iter()
            .map(|(model, s)| {
                let rate = if s.trials > 0 {
                    s.successes as f64 / s.trials as f64
                } else {
                    0.0
                };
                (model, rate, s.trials)
            })
            .collect();
        // Sort by trials descending for deterministic output: when looking for
        // dominators, prefer the "closest" (most trials) model first.
        model_rates.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(b.0)));

        let mut any_dominated = false;
        for (model, rate, trials) in &model_rates {
            // Check if another model dominates this one.
            for (other_model, other_rate, other_trials) in &model_rates {
                if *model == *other_model {
                    continue;
                }
                // `other` dominates `model` if it has a strictly higher pass
                // rate with fewer or equal trials (cost proxy).
                if *other_rate > *rate && *other_trials <= *trials {
                    let _ = writeln!(out, "  {model} dominated by {other_model}");
                    any_dominated = true;
                    break;
                }
            }
        }
        if !any_dominated {
            let _ = writeln!(out, "  (no dominated models)");
        }

        Some(out)
    }

    pub(super) fn render_dreams_page(&self, page: &PageScaffold) -> Option<String> {
        let dream_dir = self.root.join(".roko").join("dreams");
        let journal_path = dream_dir.join("journal.jsonl");
        let archive_path = dream_dir.join("archive.jsonl");
        let journal_exists = journal_path.exists();
        let archive_exists = archive_path.exists();

        if !journal_exists && !archive_exists {
            return None;
        }

        let mut out = page_header(page);

        // Journal entries (most recent).
        if journal_exists {
            let _ = writeln!(out, "journal: {}", journal_path.display());
            if let Ok(content) = std::fs::read_to_string(&journal_path) {
                let lines: Vec<&str> = content.lines().collect();
                let total = lines.len();
                let _ = writeln!(out, "  entries: {total}");
                let _ = writeln!(out, "  recent:");
                for line in lines.iter().rev().take(5) {
                    if let Ok(val) = serde_json::from_str::<Value>(line) {
                        let cycle_id = val.get("cycle_id").and_then(|v| v.as_str()).unwrap_or("?");
                        let phase = val.get("phase").and_then(|v| v.as_str()).unwrap_or("?");
                        let summary = val.get("summary").and_then(|v| v.as_str()).unwrap_or("");
                        let _ = writeln!(out, "    [{cycle_id}] {phase}: {summary}");
                    }
                }
            }
        } else {
            let _ = writeln!(out, "journal: (no entries yet)");
        }

        let _ = writeln!(out);

        // Archive entries.
        if archive_exists {
            let _ = writeln!(out, "archive: {}", archive_path.display());
            if let Ok(content) = std::fs::read_to_string(&archive_path) {
                let lines: Vec<&str> = content.lines().collect();
                let total = lines.len();
                let _ = writeln!(out, "  entries: {total}");
                let _ = writeln!(out, "  recent:");
                for line in lines.iter().rev().take(5) {
                    if let Ok(val) = serde_json::from_str::<Value>(line) {
                        let kind = val.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
                        let quality = val
                            .get("quality_score")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        let summary = val.get("summary").and_then(|v| v.as_str()).unwrap_or("");
                        let _ = writeln!(out, "    [{kind}] q={quality:.2}: {summary}");
                    }
                }
            }
        } else {
            let _ = writeln!(out, "archive: (no entries yet)");
        }

        Some(out)
    }
}

/// Per-agent aggregated stats.
/// Render standard page header.
fn page_header(page: &PageScaffold) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{} ({})", page.title, page.id.slug());
    let _ = writeln!(out, "group: {}", page.id.group());
    let _ = writeln!(out, "intent: {}", page.intent);
    out
}

fn render_boxed_panel(title: &str, lines: &[String]) -> String {
    let width = 50_usize;
    let mut out = String::new();
    let _ = writeln!(out, "╔{}╗", "═".repeat(width));
    let _ = writeln!(out, "║{: <width$}║", format!("  {title}"), width = width);
    let _ = writeln!(out, "╠{}╣", "═".repeat(width));
    for line in lines {
        let truncated: String = line.chars().take(width).collect();
        let _ = writeln!(out, "║{: <width$}║", truncated, width = width);
    }
    let _ = write!(out, "╚{}╝", "═".repeat(width));
    out
}

fn render_learning_trend_lines(buckets: &[EfficiencyBucket]) -> Vec<String> {
    let tokens = buckets
        .iter()
        .map(|bucket| bucket.tokens_in.saturating_add(bucket.tokens_out))
        .collect::<Vec<_>>();
    let latency = buckets
        .iter()
        .map(|bucket| bucket.latency_ms_avg.round() as u64)
        .collect::<Vec<_>>();
    let cost = buckets
        .iter()
        .map(|bucket| bucket.cost_usd_cents)
        .collect::<Vec<_>>();
    let has_data = buckets.iter().any(|bucket| bucket.turns > 0);

    if !has_data {
        return vec![
            String::new(),
            String::from("  24h Efficiency Trends"),
            String::from("  tok/h   no efficiency events yet"),
            String::from("  lat/h   no efficiency events yet"),
            String::from("  cost/h  no efficiency events yet"),
        ];
    }

    vec![
        String::new(),
        String::from("  24h Efficiency Trends"),
        format!(
            "  tok/h   {} {}",
            learning_sparkline(&tokens),
            format_compact_count(tokens.iter().copied().max().unwrap_or(0))
        ),
        format!(
            "  lat/h   {} {}ms",
            learning_sparkline(&latency),
            latency.iter().copied().max().unwrap_or(0)
        ),
        format!(
            "  cost/h  {} {}",
            learning_sparkline(&cost),
            format_usd(cost.iter().copied().max().unwrap_or(0) as f64 / 100.0)
        ),
    ]
}

fn learning_sparkline(series: &[u64]) -> String {
    if series.is_empty() {
        return String::new();
    }

    let max = series.iter().copied().max().unwrap_or(0);
    series
        .iter()
        .map(|value| learning_sparkline_char(*value, max))
        .collect()
}

fn learning_sparkline_char(value: u64, max: u64) -> char {
    const LEVELS: &[char; 8] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if max == 0 {
        return '·';
    }

    let idx = ((value.saturating_mul((LEVELS.len() - 1) as u64)) + (max / 2)) / max;
    LEVELS[idx as usize]
}

fn format_compact_count(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value >= 10_000 {
        format!("{}k", value / 1_000)
    } else if value >= 1_000 {
        format!("{:.1}k", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

fn signal_relative_age(created_at_ms: i64) -> String {
    let created_at_ms = u64::try_from(created_at_ms).unwrap_or_default();
    let age_ms = now_ms().saturating_sub(created_at_ms);
    format_elapsed_ms(age_ms)
}

fn format_relative_timestamp(timestamp: Option<DateTime<Utc>>) -> String {
    let Some(timestamp) = timestamp else {
        return String::new();
    };
    let age_ms = Utc::now()
        .signed_duration_since(timestamp)
        .num_milliseconds()
        .max(0) as u64;
    format!("{} ago", format_elapsed_ms(age_ms))
}

fn format_file_age(stamp: FileStamp) -> String {
    format_relative_timestamp(stamp.modified.map(DateTime::<Utc>::from))
}

fn cascade_observations_snapshot(snapshot: Option<&CascadeSnapshotData>) -> u64 {
    snapshot.map_or(0, |snapshot| {
        if snapshot.total_observations > 0 {
            snapshot.total_observations
        } else {
            snapshot
                .confidence_stats
                .values()
                .map(|stats| stats.trials)
                .sum()
        }
    })
}

fn cascade_stage_for_observations(observations: u64) -> CascadeStage {
    if observations >= 200 {
        CascadeStage::Ucb
    } else if observations >= 50 {
        CascadeStage::Confidence
    } else {
        CascadeStage::Static
    }
}

fn cascade_stage_label(stage: CascadeStage) -> &'static str {
    match stage {
        CascadeStage::Static => "Static",
        CascadeStage::Confidence => "Confidence",
        CascadeStage::Ucb => "UCB",
    }
}

fn learning_skills_last_updated(skills: &[Skill], stamp: FileStamp) -> String {
    let latest = skills
        .iter()
        .filter_map(|skill| skill.last_matched.or(skill.first_seen))
        .max();
    format_relative_timestamp(latest.or_else(|| stamp.modified.map(DateTime::<Utc>::from)))
}

fn learning_patterns_last_updated(episodes: &[Episode]) -> String {
    format_relative_timestamp(episodes.iter().map(|episode| episode.timestamp).max())
}

struct ProviderHealthSummary {
    provider_count: usize,
    last_updated: String,
    health: &'static str,
}

fn learning_provider_health_summary(
    snapshot: &Option<ProviderHealthRegistrySnapshotData>,
    stamp: FileStamp,
    efficiency_events: &[AgentEfficiencyEvent],
) -> ProviderHealthSummary {
    if let Some(snapshot) = snapshot {
        let provider_count = snapshot.providers.len();
        let last_updated = snapshot
            .providers
            .values()
            .filter_map(|provider| provider.last_failure_at)
            .max()
            .and_then(DateTime::<Utc>::from_timestamp_millis)
            .or_else(|| stamp.modified.map(DateTime::<Utc>::from));
        let health = if snapshot
            .providers
            .values()
            .any(|provider| provider.state != CircuitState::Closed)
        {
            "degraded"
        } else if provider_count == 0 {
            "unknown"
        } else {
            "healthy"
        };
        return ProviderHealthSummary {
            provider_count,
            last_updated: format_relative_timestamp(last_updated),
            health,
        };
    }

    let mut providers = HashSet::new();
    let mut latest = None;
    for event in efficiency_events {
        if !event.backend.trim().is_empty() {
            providers.insert(event.backend.clone());
        }
        if let Some(timestamp) = parse_efficiency_timestamp(&event.timestamp) {
            latest = latest.max(Some(timestamp));
        }
    }

    ProviderHealthSummary {
        provider_count: providers.len(),
        last_updated: format_relative_timestamp(
            latest.or_else(|| stamp.modified.map(DateTime::<Utc>::from)),
        ),
        health: if providers.is_empty() {
            "unknown"
        } else {
            "healthy"
        },
    }
}

fn learning_knowledge_updates(snapshot: &KnowledgeStoreSnapshot) -> String {
    if snapshot.total_records == 0 {
        String::from("[WIP]")
    } else {
        format!("{} records", snapshot.total_records)
    }
}

fn signal_kind_prefix(kind: &str) -> String {
    kind.split(':').next().unwrap_or(kind).to_string()
}

fn signal_kind_distribution(signals: &[SignalSummary]) -> Vec<(String, u64)> {
    let mut counts = BTreeMap::<String, u64>::new();
    for signal in signals {
        *counts.entry(signal_kind_prefix(&signal.kind)).or_default() += 1;
    }

    let mut rows = counts.into_iter().collect::<Vec<_>>();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    rows
}

#[derive(Debug)]
struct SignalTreeEntry<'a> {
    hash: String,
    signal: Option<&'a SignalSummary>,
}

fn signal_parent_chain<'a>(
    signals: &'a [SignalSummary],
    selected: &'a SignalSummary,
) -> Vec<SignalTreeEntry<'a>> {
    let by_id = signals
        .iter()
        .map(|signal| (signal.id.as_str(), signal))
        .collect::<HashMap<_, _>>();

    let mut chain = Vec::new();
    chain.push(SignalTreeEntry {
        hash: selected.id.clone(),
        signal: Some(selected),
    });

    let ancestors: Vec<String> = if selected.lineage.is_empty() {
        selected.parent_hash.iter().cloned().collect()
    } else {
        selected.lineage.iter().rev().cloned().collect()
    };

    for hash in ancestors {
        let signal = by_id.get(hash.as_str()).copied();
        chain.push(SignalTreeEntry { hash, signal });
    }

    chain
}

pub(super) fn load_snapshot_best_effort(root: &Path) -> TuiDashboardModel {
    load_snapshot_blocking(root).unwrap_or_else(|_| TuiDashboardModel::empty(root.to_path_buf()))
}

pub(super) fn load_snapshot_blocking(root: &Path) -> Result<TuiDashboardModel, std::io::Error> {
    let root = root.to_path_buf();
    let load = move || -> Result<TuiDashboardModel, std::io::Error> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(std::io::Error::other)?;
        runtime.block_on(TuiDashboardModel::load(&root))
    };

    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::spawn(load)
            .join()
            .map_err(|_| std::io::Error::other("dashboard snapshot loader panicked"))?
    } else {
        load()
    }
}

pub(super) fn resolve_snapshot_root(start: &Path) -> PathBuf {
    let mut cursor = Some(start);
    while let Some(dir) = cursor {
        let memory_dir = dir.join(MEMORY_DIR);
        if resolve_episodes_path(dir).exists() || memory_dir.join(TASK_METRICS_FILE).exists() {
            return dir.to_path_buf();
        }
        cursor = dir.parent();
    }
    start.to_path_buf()
}

pub(super) fn load_knowledge_store_snapshot(root: &Path) -> KnowledgeStoreSnapshot {
    let neuro_dir = root.join(NEURO_DIR);
    let knowledge_path = neuro_dir.join(KNOWLEDGE_FILE);
    let confirmations_path = neuro_dir.join(KNOWLEDGE_CONFIRMATIONS_FILE);
    let knowledge_stamp = file_stamp(&knowledge_path);
    let confirmations_stamp = file_stamp(&confirmations_path);
    let last_updated = [knowledge_stamp.modified, confirmations_stamp.modified]
        .into_iter()
        .flatten()
        .max()
        .map(DateTime::<Utc>::from);

    KnowledgeStoreSnapshot {
        total_records: count_nonempty_lines(&knowledge_path)
            + count_nonempty_lines(&confirmations_path),
        last_updated,
    }
}

/// Load knowledge entries from `.roko/neuro/knowledge.jsonl`, projecting each
/// full `KnowledgeEntry` to the lightweight `KnowledgeBrowseEntry` for the TUI.
pub(super) fn load_knowledge_browse_entries(root: &Path) -> Vec<KnowledgeBrowseEntry> {
    let knowledge_path = root.join(NEURO_DIR).join(KNOWLEDGE_FILE);
    let text = match std::fs::read_to_string(&knowledge_path) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let mut entries: Vec<KnowledgeBrowseEntry> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<roko_neuro::KnowledgeEntry>(line).ok())
        .map(|entry| {
            let preview = if entry.content.len() > 120 {
                format!("{}...", &entry.content[..117])
            } else {
                entry.content.clone()
            };
            KnowledgeBrowseEntry {
                id: entry.id.clone(),
                kind: format!("{:?}", entry.kind).to_lowercase(),
                content_preview: preview,
                confidence: entry.confidence,
                tier: format!("{:?}", entry.tier).to_lowercase(),
                tags: entry.tags.clone(),
                created_at: entry.created_at,
                frozen: entry.frozen,
            }
        })
        .collect();
    // Most recent first
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    entries
}

pub(super) fn count_nonempty_lines(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|text| text.lines().filter(|line| !line.trim().is_empty()).count())
        .unwrap_or(0)
}

pub(super) async fn read_task_metrics(path: &Path) -> Result<Vec<TaskMetric>, std::io::Error> {
    let text = match tokio::fs::read_to_string(path).await {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };

    let mut metrics = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(metric) = TaskMetric::from_jsonl(line) {
            metrics.push(metric);
        }
    }
    Ok(metrics)
}

pub(super) fn format_pct(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

pub(super) fn format_float(value: f64) -> String {
    format!("{value:.2}")
}

pub(super) fn format_usd(value: f64) -> String {
    format!("${value:.4}")
}

pub(super) fn format_ms(value: f64) -> String {
    format!("{value:.0} ms")
}

pub(super) fn count_to_f64(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}

pub(super) fn wall_ms_to_f64(wall_ms: u64) -> f64 {
    f64::from(u32::try_from(wall_ms).unwrap_or(u32::MAX))
}

/// Compute the p-th percentile from a slice of millisecond latencies.
/// Returns `None` for empty slices.
pub(super) fn percentile_ms(values: &[f64], pct: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((pct / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    let idx = idx.min(sorted.len() - 1);
    Some(sorted[idx])
}

/// Format a millisecond value as seconds with one decimal place (e.g. "0.8s").
pub(super) fn format_latency_seconds(ms: f64) -> String {
    let secs = ms / 1000.0;
    format!("{secs:.1}s")
}

/// Read efficiency events from JSONL (best-effort, returns empty on error).
pub(super) async fn read_efficiency_events(path: &Path) -> Vec<AgentEfficiencyEvent> {
    let Ok(text) = tokio::fs::read_to_string(path).await else {
        return Vec::new();
    };
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// Best-effort JSON file loader. Returns `None` if missing or corrupt.
pub(super) fn load_json_opt<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

