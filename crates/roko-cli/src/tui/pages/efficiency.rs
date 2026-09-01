//! Efficiency dashboard scaffold pages (§18).

use std::collections::{BTreeMap, BTreeSet};

use roko_core::agent::ModelTier;
use roko_core::config::model_registry::model_meta;
use roko_learn::efficiency::AgentEfficiencyEvent;

use super::{PageId, PageScaffold, WidgetScaffold};
use crate::tui::dashboard::DashboardData;

/// Derived efficiency summary used by the live TUI widgets.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EfficiencySnapshot {
    /// Total recorded efficiency events.
    pub event_count: usize,
    /// Distinct task attempts represented in the efficiency log.
    pub task_count: usize,
    /// Total input + output tokens.
    pub total_tokens: u64,
    /// Total input tokens.
    pub total_input_tokens: u64,
    /// Total output tokens.
    pub total_output_tokens: u64,
    /// Total spend in USD.
    pub total_cost_usd: f64,
    /// Average tokens per task.
    pub average_tokens_per_task: f64,
    /// Average spend per task.
    pub average_cost_per_task: f64,
    /// Success ratio from the persisted efficiency summary.
    pub success_rate: f64,
    /// Tokens emitted per model tier.
    pub tier_counts: BTreeMap<&'static str, u64>,
    /// Short bar label per tier, taken from the dominant model family/slug
    /// in that tier (see [`tier_bar_labels`]). Empty tiers have no entry.
    pub tier_labels: BTreeMap<&'static str, String>,
    /// Token usage history for sparkline rendering.
    pub token_series: Vec<u64>,
}

/// Build a live efficiency snapshot from dashboard data.
#[must_use]
pub fn build_efficiency_snapshot(data: &DashboardData) -> EfficiencySnapshot {
    let event_count = data.efficiency_events.len();
    let total_input_tokens = data.efficiency.total_input_tokens;
    let total_output_tokens = data.efficiency.total_output_tokens;
    let total_tokens = total_input_tokens + total_output_tokens;
    let total_cost_usd = data.efficiency.total_cost_usd;

    let task_count = data
        .efficiency_events
        .iter()
        .map(task_key)
        .collect::<BTreeSet<_>>()
        .len();
    let average_tokens_per_task = if task_count == 0 {
        0.0
    } else {
        total_tokens as f64 / task_count as f64
    };
    let average_cost_per_task = if task_count == 0 {
        0.0
    } else {
        total_cost_usd / task_count as f64
    };
    let success_rate = if event_count == 0 {
        0.0
    } else {
        data.efficiency.passed_count as f64 / event_count as f64
    };

    let mut tier_counts: BTreeMap<&'static str, u64> = BTreeMap::new();
    for tier in ["T0", "T1", "T2"] {
        tier_counts.insert(tier, 0);
    }
    for event in &data.efficiency_events {
        *tier_counts
            .entry(model_tier_label(&event.model))
            .or_default() += 1;
    }
    let tier_labels = tier_bar_labels(data.efficiency_events.iter().map(|e| e.model.as_str()));

    let token_series = data
        .efficiency_events
        .iter()
        .map(|event| event.input_tokens + event.output_tokens)
        .collect();

    EfficiencySnapshot {
        event_count,
        task_count,
        total_tokens,
        total_input_tokens,
        total_output_tokens,
        total_cost_usd,
        average_tokens_per_task,
        average_cost_per_task,
        success_rate,
        tier_counts,
        tier_labels,
        token_series,
    }
}

fn task_key(event: &AgentEfficiencyEvent) -> (String, String) {
    (event.plan_id.clone(), event.task_id.clone())
}

/// Map a model slug to its tier key via the shared registry resolver
/// ([`model_meta`]), not ad-hoc substring guesses.
pub(crate) fn model_tier_label(model: &str) -> &'static str {
    match model_meta(model).tier {
        ModelTier::Fast => "T0",
        ModelTier::Premium => "T2",
        // Standard and any future non-exhaustive variants land in the middle.
        _ => "T1",
    }
}

/// Short display label for a model slug on tier bars: the registry family
/// (`claude`, `gpt`, `codex`, `glm`, …) when known, otherwise a truncated
/// slug so unregistered models still render a sensible, non-empty label.
pub(crate) fn model_bar_label(model: &str) -> String {
    const MAX_LEN: usize = 8;
    let meta = model_meta(model);
    if meta.family != "unknown" {
        meta.family.to_string()
    } else {
        let trimmed = model.trim();
        if trimmed.is_empty() {
            "unknown".to_string()
        } else {
            trimmed.chars().take(MAX_LEN).collect()
        }
    }
}

/// Dominant [`model_bar_label`] per tier across the given model slugs.
/// Ties break alphabetically for determinism; tiers with no slugs are absent.
pub(crate) fn tier_bar_labels<'a>(
    models: impl Iterator<Item = &'a str>,
) -> BTreeMap<&'static str, String> {
    let mut counts: BTreeMap<&'static str, BTreeMap<String, u64>> = BTreeMap::new();
    for model in models {
        *counts
            .entry(model_tier_label(model))
            .or_default()
            .entry(model_bar_label(model))
            .or_default() += 1;
    }
    counts
        .into_iter()
        .filter_map(|(tier, labels)| {
            labels
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
                .map(|(label, _)| (tier, label))
        })
        .collect()
}

/// All six efficiency pages from plan 09 as placeholder scaffolds.
#[must_use]
pub fn scaffold_pages() -> Vec<PageScaffold> {
    vec![
        PageScaffold::new(
            PageId::Health,
            "Health",
            "Top-line efficiency gauges for current runs.",
            vec![
                WidgetScaffold::new("pass_rate", "Pass Rate", "Rolling gate pass rate."),
                WidgetScaffold::new("cost_per_task", "Cost / Task", "Average spend per task."),
                WidgetScaffold::new(
                    "prompt_size",
                    "Prompt Size",
                    "Median prompt token footprint.",
                ),
            ],
        ),
        PageScaffold::new(
            PageId::Trends,
            "Trends",
            "Time-series drift and learning-velocity trends.",
            vec![
                WidgetScaffold::new(
                    "latency_sparkline",
                    "Latency Sparkline",
                    "Turn latency over time.",
                ),
                WidgetScaffold::new(
                    "learning_velocity",
                    "Learning Velocity",
                    "New useful rules/skills over time.",
                ),
            ],
        ),
        PageScaffold::new(
            PageId::Correlations,
            "Correlations",
            "Relationship views between interventions and outcomes.",
            vec![
                WidgetScaffold::new(
                    "context_vs_pass",
                    "Context vs Pass Rate",
                    "Impact of context packing on gates.",
                ),
                WidgetScaffold::new(
                    "strategy_vs_cost",
                    "Strategy vs Cost",
                    "Cost profile by strategy selection.",
                ),
            ],
        ),
        PageScaffold::new(
            PageId::GateResults,
            "Verify Results",
            "Verify pass rates, adaptive thresholds, and recent failures.",
            vec![
                WidgetScaffold::new(
                    "gate_summary",
                    "Verify Summary",
                    "Verify name, runs, pass rate, average duration, and last run.",
                ),
                WidgetScaffold::new(
                    "adaptive_thresholds",
                    "Adaptive Thresholds",
                    "Current rung thresholds, EMA values, and trend arrows.",
                ),
                WidgetScaffold::new(
                    "recent_failures",
                    "Recent Failures",
                    "Last ten gate failures with task ID, gate name, and excerpt.",
                ),
            ],
        ),
        PageScaffold::new(
            PageId::Learning,
            "Learning",
            "Learning subsystem interactions, update counts, and feedback loop status.",
            vec![
                WidgetScaffold::new(
                    "learning_system_status",
                    "Learning System Status",
                    "Stage transitions, subsystem freshness, and missing feedback loops.",
                ),
                WidgetScaffold::new(
                    "active_experiments",
                    "Active Experiments",
                    "Experiment names, variants, samples, winners, and significance.",
                ),
                WidgetScaffold::new(
                    "efficiency_trends",
                    "Efficiency Trends",
                    "7-day sparklines for cost, tokens, success, and first-try rate.",
                ),
            ],
        ),
        PageScaffold::new(
            PageId::Parameters,
            "Parameters",
            "Runtime tunables and predicted impact metadata.",
            vec![
                WidgetScaffold::new(
                    "knobs",
                    "Knobs",
                    "Editable runtime and learning parameters.",
                ),
                WidgetScaffold::new(
                    "impact_scores",
                    "Impact Scores",
                    "Estimated sensitivity per knob.",
                ),
            ],
        ),
        PageScaffold::new(
            PageId::Experiments,
            "Experiments",
            "A/B experiment outcomes and statistical summaries.",
            vec![
                WidgetScaffold::new("ab_runs", "A/B Runs", "Recent active/finished experiments."),
                WidgetScaffold::new(
                    "significance",
                    "Significance",
                    "Simple significance verdicts per experiment.",
                ),
            ],
        ),
        PageScaffold::new(
            PageId::Optimizer,
            "Optimizer",
            "Closed-loop optimization state and confidence bars.",
            vec![
                WidgetScaffold::new(
                    "loop_state",
                    "Loop State",
                    "Current optimization cycle stage.",
                ),
                WidgetScaffold::new(
                    "confidence_bars",
                    "Confidence Bars",
                    "Confidence by optimization candidate.",
                ),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(model: &str, task_id: &str, input: u64, output: u64) -> AgentEfficiencyEvent {
        let mut event = AgentEfficiencyEvent::default_event();
        event.model = model.to_string();
        event.plan_id = "plan-1".to_string();
        event.task_id = task_id.to_string();
        event.input_tokens = input;
        event.output_tokens = output;
        event
    }

    #[test]
    fn model_tier_label_uses_registry_tiers() {
        assert_eq!(model_tier_label("claude-haiku-4-5"), "T0");
        assert_eq!(model_tier_label("claude-sonnet-4-6"), "T1");
        assert_eq!(model_tier_label("claude-opus-4-6"), "T2");
        // Non-claude slugs resolve via the registry, not haiku/opus substrings.
        assert_eq!(model_tier_label("codex-mini"), "T0");
        assert_eq!(model_tier_label("gpt-5.6-sol"), "T2");
        assert_eq!(model_tier_label("glm-5.1"), "T1");
        // Unknown slugs degrade to the middle tier instead of panicking.
        assert_eq!(model_tier_label("my-finetune-v2"), "T1");
        assert_eq!(model_tier_label(""), "T1");
    }

    #[test]
    fn model_bar_label_uses_family_not_claude_names() {
        // Codex/gpt slugs must not be labeled "sonnet".
        let codex = model_bar_label("gpt-5.6-sol");
        assert_eq!(codex, "gpt");
        assert_ne!(codex, "sonnet");
        assert_eq!(model_bar_label("codex-mini"), "codex");
        // glm/kimi families are labeled by family.
        assert_eq!(model_bar_label("glm-5.1"), "glm");
        assert_eq!(model_bar_label("kimi-k2"), "kimi");
        assert_eq!(model_bar_label("claude-sonnet-4-6"), "claude");
        // Unknown slugs fall back to truncated slug text; empty is safe.
        assert_eq!(model_bar_label("my-finetune-v2"), "my-finet");
        assert_eq!(model_bar_label(""), "unknown");
        assert_eq!(model_bar_label("  "), "unknown");
    }

    #[test]
    fn tier_bar_labels_picks_dominant_family() {
        let models = ["glm-5.1", "glm-5.1", "claude-sonnet-4-6", "codex-mini"];
        let labels = tier_bar_labels(models.into_iter());
        // T1 has glm x2 + claude x1 -> glm dominates; T0 has codex only.
        assert_eq!(labels.get("T1").map(String::as_str), Some("glm"));
        assert_eq!(labels.get("T0").map(String::as_str), Some("codex"));
        assert!(!labels.contains_key("T2"));
    }

    #[test]
    fn build_efficiency_snapshot_empty_is_zeroed() {
        let data = DashboardData::default();
        let snap = build_efficiency_snapshot(&data);
        assert_eq!(snap.event_count, 0);
        assert_eq!(snap.task_count, 0);
        assert_eq!(snap.total_tokens, 0);
        assert_eq!(snap.success_rate, 0.0);
        assert_eq!(snap.tier_counts.values().sum::<u64>(), 0);
        assert!(snap.tier_labels.is_empty());
        assert!(snap.token_series.is_empty());
    }

    #[test]
    fn build_efficiency_snapshot_buckets_non_claude_models() {
        let mut data = DashboardData::default();
        data.efficiency_events = vec![
            event("gpt-5.6-sol", "t1", 1_000, 200),
            event("glm-5.1", "t2", 500, 100),
            event("kimi-k2", "t3", 400, 100),
            event("my-finetune-v2", "t4", 100, 50),
        ];
        data.efficiency.total_input_tokens = 2_000;
        data.efficiency.total_output_tokens = 450;
        data.efficiency.total_cost_usd = 0.25;
        data.efficiency.passed_count = 3;

        let snap = build_efficiency_snapshot(&data);
        assert_eq!(snap.event_count, 4);
        assert_eq!(snap.task_count, 4);
        assert_eq!(snap.total_tokens, 2_450);
        assert!((snap.success_rate - 0.75).abs() < f64::EPSILON);
        // gpt-5.6-sol -> T2 (Premium), the rest -> T1 (Standard); nothing
        // collapses into a silent "sonnet" bucket.
        assert_eq!(snap.tier_counts.get("T2").copied(), Some(1));
        assert_eq!(snap.tier_counts.get("T1").copied(), Some(3));
        assert_eq!(snap.tier_counts.get("T0").copied(), Some(0));
        assert_eq!(snap.tier_labels.get("T2").map(String::as_str), Some("gpt"));
        assert!(snap.tier_labels.values().all(|l| l != "sonnet"));
        // Unknown slug still gets a label on its tier bar.
        assert!(snap.tier_labels.contains_key("T1"));
        assert_eq!(snap.token_series.len(), 4);
    }
}
