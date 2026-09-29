//! Spend / token budget configuration section.
//!
//! ## Ceiling semantics
//!
//! `max_plan_usd`, `max_turn_usd`, and `max_task_retry_usd` use **0.0 = unlimited**:
//!
//! * When the field is absent from `roko.toml` the default is `0.0` (no cap),
//!   except for `max_task_retry_usd` which defaults to `5.0` ($5 per task).
//! * When set to a positive value the runner enforces that ceiling.
//! * Negative, `NaN`, and `Inf` values are rejected at pre-flight validation
//!   (see `event_loop::validate_budget_ceilings`).
//!
//! Previous defaults were 25.0 / 3.0 which silently capped every run even
//! when the user never configured a budget.

use serde::{Deserialize, Serialize};

use crate::task::TaskTier;

// ---- [budget] ------------------------------------------------------------

/// Spend / token budget settings.
///
/// A ceiling of `0.0` means **unlimited** — the runner will not enforce
/// any spend cap for that dimension.
#[allow(clippy::derive_partial_eq_without_eq)] // contains f32
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetConfig {
    /// Per-plan cost ceiling in USD. `0.0` means unlimited.
    #[serde(default)]
    pub max_plan_usd: f32,
    /// Base per-task cost ceiling in USD. `0.0` means unlimited.
    #[serde(default)]
    pub max_task_usd: f32,
    /// Per-turn cost ceiling in USD. `0.0` means unlimited.
    #[serde(default)]
    pub max_turn_usd: f32,
    /// Maximum cumulative cost across all retry attempts for a single task,
    /// in USD. `0.0` means unlimited. When the total cost of all attempts
    /// for one task exceeds this value, the next retry is suppressed and the
    /// task is marked failed with reason "cumulative cost cap exceeded".
    ///
    /// Default: `5.0` ($5.00 per task across all retries).
    #[serde(default = "default_max_task_retry_usd")]
    pub max_task_retry_usd: f32,
    /// Per-calendar-day cost ceiling in USD, enforced across all plan runs.
    /// `0.0` means unlimited. When the day's total spend (read from the
    /// costs log) reaches this ceiling, new dispatches are blocked.
    #[serde(default)]
    pub max_daily_usd: f32,
    /// Token budget for prompt composition.
    #[serde(default = "default_prompt_token_budget")]
    pub prompt_token_budget: usize,
    /// Complexity multipliers applied to [`Self::max_task_usd`].
    #[serde(default)]
    pub tier_multipliers: TaskBudgetMultipliers,
    /// P3-34: Per-agent cumulative lifetime cost ceiling in USD.
    ///
    /// `0.0` means unlimited (default). When a single agent's cumulative
    /// cost exceeds this limit, an `AgentBudgetExhausted` event is emitted
    /// and the agent is drained.
    #[serde(default)]
    pub max_agent_lifetime_usd: f32,
}

/// Per-task budget multipliers for the four canonical plan tiers.
#[allow(clippy::derive_partial_eq_without_eq)] // contains f32
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskBudgetMultipliers {
    /// Mechanical tasks normally use a cheap/fast model.
    #[serde(default = "default_mechanical_multiplier")]
    pub mechanical: f32,
    /// Standard/focused tasks use the base task budget.
    #[serde(default = "default_standard_multiplier", alias = "focused")]
    pub standard: f32,
    /// Complex/integrative tasks may span several components.
    #[serde(default = "default_complex_multiplier", alias = "integrative")]
    pub complex: f32,
    /// Expert/architectural tasks may require the deepest model tier.
    #[serde(default = "default_expert_multiplier", alias = "architectural")]
    pub expert: f32,
}

const fn default_mechanical_multiplier() -> f32 {
    0.2
}

const fn default_standard_multiplier() -> f32 {
    1.0
}

const fn default_complex_multiplier() -> f32 {
    3.0
}

const fn default_expert_multiplier() -> f32 {
    5.0
}

impl Default for TaskBudgetMultipliers {
    fn default() -> Self {
        Self {
            mechanical: default_mechanical_multiplier(),
            standard: default_standard_multiplier(),
            complex: default_complex_multiplier(),
            expert: default_expert_multiplier(),
        }
    }
}

const fn default_max_task_retry_usd() -> f32 {
    5.0
}

const fn default_prompt_token_budget() -> usize {
    10_000
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_plan_usd: 0.0,
            max_task_usd: 0.0,
            max_turn_usd: 0.0,
            max_task_retry_usd: default_max_task_retry_usd(),
            max_daily_usd: 0.0,
            prompt_token_budget: default_prompt_token_budget(),
            tier_multipliers: TaskBudgetMultipliers::default(),
            max_agent_lifetime_usd: 0.0,
        }
    }
}

impl TaskBudgetMultipliers {
    /// Multiplier for a task tier.
    #[must_use]
    pub const fn for_tier(&self, tier: TaskTier) -> f32 {
        match tier {
            TaskTier::Mechanical => self.mechanical,
            TaskTier::Focused => self.standard,
            TaskTier::Integrative => self.complex,
            TaskTier::Architectural => self.expert,
        }
    }
}

impl BudgetConfig {
    /// Return the effective task ceiling for a plan tier/model hint.
    ///
    /// The tier is read by [`TaskTier::parse`]. An omitted tier (empty or
    /// `"unknown"`) falls back to the model family; any other unknown tier
    /// uses the focused (standard) multiplier. A zero base remains unlimited.
    #[must_use]
    pub fn task_limit_usd(&self, tier: &str, model_hint: Option<&str>) -> f64 {
        if self.max_task_usd <= 0.0 {
            return 0.0;
        }
        let tier = TaskTier::parse(tier).unwrap_or_else(|| {
            let omitted = tier.trim().is_empty() || tier.trim().eq_ignore_ascii_case("unknown");
            if !omitted {
                return TaskTier::Focused;
            }
            let model = model_hint.unwrap_or_default().to_ascii_lowercase();
            if model.contains("haiku") || model.contains("mini") {
                TaskTier::Mechanical
            } else if model.contains("opus") {
                TaskTier::Integrative
            } else {
                TaskTier::Focused
            }
        });
        f64::from(self.max_task_usd) * f64::from(self.tier_multipliers.for_tier(tier))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_limits_scale_by_tier_and_model_fallback() {
        let budget = BudgetConfig {
            max_task_usd: 2.0,
            ..BudgetConfig::default()
        };

        assert!((budget.task_limit_usd("mechanical", None) - 0.4).abs() < 1e-6);
        assert_eq!(budget.task_limit_usd("focused", None), 2.0);
        assert_eq!(budget.task_limit_usd("integrative", None), 6.0);
        assert_eq!(budget.task_limit_usd("architectural", None), 10.0);
        assert!((budget.task_limit_usd("unknown", Some("claude-haiku-4-5")) - 0.4).abs() < 1e-6);
        assert_eq!(
            budget.task_limit_usd("unknown", Some("claude-opus-4-6")),
            6.0
        );
        assert_eq!(
            budget.task_limit_usd("unknown", Some("claude-sonnet-4-6")),
            2.0
        );

        // Tier aliases read through `TaskTier::parse`; a misspelt tier is
        // focused, whatever the model hint.
        assert!((budget.task_limit_usd(" T0 ", None) - 0.4).abs() < 1e-6);
        assert_eq!(budget.task_limit_usd("complex", None), 6.0);
        assert_eq!(budget.task_limit_usd("Premium", None), 10.0);
        assert_eq!(
            budget.task_limit_usd("mechancial", Some("claude-haiku-4-5")),
            2.0
        );
    }

    #[test]
    fn zero_task_budget_is_unlimited_for_every_tier() {
        assert_eq!(
            BudgetConfig::default().task_limit_usd("architectural", None),
            0.0
        );
    }
}
