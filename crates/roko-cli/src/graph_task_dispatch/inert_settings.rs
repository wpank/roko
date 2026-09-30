//! Config keys set to non-default values that the Graph engine never reads.

use super::*;

/// A config key set to a non-default value that `plan run` (Graph engine)
/// never reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InertGraphSetting {
    /// Dotted config key.
    pub key: &'static str,
    /// Why the key has no effect on `plan run`.
    pub reason: &'static str,
}

/// Config keys set to non-default values that have no effect on `plan run`.
///
/// Reported once per process when a Graph dispatcher is built and by
/// `roko config doctor`, so operators stop relying on them.
#[must_use]
pub fn graph_engine_inert_settings(config: &RokoConfig) -> Vec<InertGraphSetting> {
    const LEGACY_GATES: &str = "only the legacy Runner-v2 gate pipeline (--engine legacy) reads it";
    const ADAPTIVE: &str = "of the adaptive-threshold settings the Graph engine reads only \
                            adaptive_min_retries and adaptive_max_retries (task retry budgets); \
                            its gate EMA uses a fixed alpha";
    const NOT_ENFORCED: &str = "not enforced by the Graph engine";
    const NO_READER: &str = "no production code reads it";
    const DISPLAY_ONLY: &str = "shown by config views; no routing decision reads it";
    const LADDER: &str = "deprecated: no routing decision reads it; [routing.ladder] picks \
                          plan-task models";
    const NO_WARM_POOL: &str = "no dispatch path pre-spawns or reuses agents";
    const PIPELINE_BAND: &str = "only `max_turns` in [pipeline.<tier>] affects plan run";
    const NOT_WIRED: &str = "dispatch does not read the learned tier limits yet (gap-5a6e01)";

    let defaults = RokoConfig::default();
    let (gates, default_gates) = (&config.gates, &defaults.gates);
    let (routing, default_routing) = (&config.routing, &defaults.routing);
    let mut checks = vec![
        (gates.mode != default_gates.mode, "gates.mode", LEGACY_GATES),
        (
            gates.clippy_enabled != default_gates.clippy_enabled,
            "gates.clippy_enabled",
            LEGACY_GATES,
        ),
        (
            gates.skip_tests != default_gates.skip_tests,
            "gates.skip_tests",
            LEGACY_GATES,
        ),
        (
            gates.max_iterations != default_gates.max_iterations,
            "gates.max_iterations",
            LEGACY_GATES,
        ),
        (
            gates.impact_timeout_ms != default_gates.impact_timeout_ms,
            "gates.impact_timeout_ms",
            LEGACY_GATES,
        ),
        (
            gates.impact_max_reverse_dependents != default_gates.impact_max_reverse_dependents,
            "gates.impact_max_reverse_dependents",
            LEGACY_GATES,
        ),
        (
            gates.impact_max_targets != default_gates.impact_max_targets,
            "gates.impact_max_targets",
            LEGACY_GATES,
        ),
        (
            gates.max_rung != default_gates.max_rung,
            "gates.max_rung",
            LEGACY_GATES,
        ),
        (
            gates.domain_gates != default_gates.domain_gates,
            "gates.domain_gates",
            NO_READER,
        ),
        (
            gates.ema_alpha.to_bits() != default_gates.ema_alpha.to_bits(),
            "gates.ema_alpha",
            ADAPTIVE,
        ),
        (
            gates.skip_streak_threshold != default_gates.skip_streak_threshold,
            "gates.skip_streak_threshold",
            ADAPTIVE,
        ),
        (
            gates.convergence_min_observations != default_gates.convergence_min_observations,
            "gates.convergence_min_observations",
            ADAPTIVE,
        ),
        (
            config.budget.max_daily_usd.to_bits() != defaults.budget.max_daily_usd.to_bits(),
            "budget.max_daily_usd",
            NOT_ENFORCED,
        ),
        (
            config.budget.max_agent_lifetime_usd.to_bits()
                != defaults.budget.max_agent_lifetime_usd.to_bits(),
            "budget.max_agent_lifetime_usd",
            NOT_ENFORCED,
        ),
        (
            config.learning.replan_max_per_plan != defaults.learning.replan_max_per_plan,
            "learning.replan_max_per_plan",
            NO_READER,
        ),
        (
            config.learning.replan_gate_attempts != defaults.learning.replan_gate_attempts,
            "learning.replan_gate_attempts",
            NO_READER,
        ),
        (config.agent.data_llm.is_some(), "agent.data_llm", NO_READER),
        (
            routing.algorithm != default_routing.algorithm,
            "routing.algorithm",
            DISPLAY_ONLY,
        ),
        (
            routing.discount_factor.to_bits() != default_routing.discount_factor.to_bits(),
            "routing.discount_factor",
            DISPLAY_ONLY,
        ),
        (
            routing.standard_task_model != default_routing.standard_task_model,
            "routing.standard_task_model",
            LADDER,
        ),
        (
            routing.complex_task_model != default_routing.complex_task_model,
            "routing.complex_task_model",
            LADDER,
        ),
        (
            routing.context_strategy != default_routing.context_strategy,
            "routing.context_strategy",
            DISPLAY_ONLY,
        ),
        (
            routing.weights != default_routing.weights,
            "routing.weights",
            DISPLAY_ONLY,
        ),
        (
            config.runner.warm_pool_size != defaults.runner.warm_pool_size,
            "runner.warm_pool_size",
            NO_WARM_POOL,
        ),
        (
            config.runner.warm_pool_idle_timeout_secs
                != defaults.runner.warm_pool_idle_timeout_secs,
            "runner.warm_pool_idle_timeout_secs",
            NO_WARM_POOL,
        ),
        (
            config.pipeline.learned_limits != defaults.pipeline.learned_limits,
            "pipeline.learned_limits",
            NOT_WIRED,
        ),
    ];
    for (key, band, default_band) in [
        (
            "pipeline.mechanical",
            config.pipeline.mechanical,
            defaults.pipeline.mechanical,
        ),
        (
            "pipeline.focused",
            config.pipeline.focused,
            defaults.pipeline.focused,
        ),
        (
            "pipeline.integrative",
            config.pipeline.integrative,
            defaults.pipeline.integrative,
        ),
        (
            "pipeline.architectural",
            config.pipeline.architectural,
            defaults.pipeline.architectural,
        ),
    ] {
        let band_without_turns = |band: roko_core::config::PipelineBandConfig| {
            (
                band.strategist,
                band.reviewers,
                band.reviewer_mode,
                band.max_iterations,
            )
        };
        checks.push((
            band_without_turns(band) != band_without_turns(default_band),
            key,
            PIPELINE_BAND,
        ));
    }
    checks
        .into_iter()
        .filter(|(changed, _, _)| *changed)
        .map(|(_, key, reason)| InertGraphSetting { key, reason })
        .collect()
}

/// Warn once per process about [`graph_engine_inert_settings`].
pub(super) fn warn_inert_graph_settings_once(config: &RokoConfig) {
    static WARNED: std::sync::Once = std::sync::Once::new();
    let inert = graph_engine_inert_settings(config);
    if inert.is_empty() {
        return;
    }
    WARNED.call_once(|| {
        let keys = inert
            .iter()
            .map(|setting| setting.key)
            .collect::<Vec<_>>()
            .join(", ");
        tracing::warn!(
            keys = %keys,
            "config keys set to non-default values have no effect on `plan run` \
             (Graph engine); run `roko config doctor` for details"
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inert_settings_list_only_changed_keys_the_graph_engine_ignores() {
        assert!(graph_engine_inert_settings(&RokoConfig::default()).is_empty());

        let mut config = RokoConfig::default();
        config
            .gates
            .domain_gates
            .insert("docs".to_string(), vec!["shell:true".to_string()]);
        config.runner.warm_pool_size = 4;
        // Wired keys are never reported.
        config.pipeline.focused.max_turns = 50;
        config.budget.max_task_usd = 2.0;
        config.gates.write_eval_artifacts = true;
        config.gates.adaptive_max_retries = 8;
        // Every plan task runs the workspace's required rungs.
        config.gates.custom_rungs = vec![roko_core::config::GateRungConfig {
            name: "test".to_string(),
            command: "cargo test".to_string(),
            timeout_secs: 300,
            required: true,
            parallel_with: Vec::new(),
        }];
        let keys = graph_engine_inert_settings(&config)
            .iter()
            .map(|setting| setting.key)
            .collect::<Vec<_>>();
        assert_eq!(keys, ["gates.domain_gates", "runner.warm_pool_size"]);

        config.pipeline.focused.strategist = true;
        assert!(
            graph_engine_inert_settings(&config)
                .iter()
                .any(|setting| setting.key == "pipeline.focused")
        );
    }
}
