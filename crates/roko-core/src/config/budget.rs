//! Spend / token budget configuration section.
//!
//! ## Ceiling semantics
//!
//! `max_plan_usd` and `max_turn_usd` use **0.0 = unlimited**:
//!
//! * When the field is absent from `roko.toml` the default is `0.0` (no cap).
//! * When set to a positive value the runner enforces that ceiling.
//! * Negative, `NaN`, and `Inf` values are rejected at pre-flight validation
//!   (see `event_loop::validate_budget_ceilings`).
//!
//! Previous defaults were 25.0 / 3.0 which silently capped every run even
//! when the user never configured a budget.

use serde::{Deserialize, Serialize};

// ---- [budget] ------------------------------------------------------------

/// Spend / token budget settings.
///
/// A ceiling of `0.0` means **unlimited** — the runner will not enforce
/// any spend cap for that dimension.
#[allow(clippy::derive_partial_eq_without_eq)] // contains f32
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Per-plan cost ceiling in USD. `0.0` means unlimited.
    #[serde(default)]
    pub max_plan_usd: f32,
    /// Per-turn cost ceiling in USD. `0.0` means unlimited.
    #[serde(default)]
    pub max_turn_usd: f32,
    /// Token budget for prompt composition.
    #[serde(default = "default_prompt_token_budget")]
    pub prompt_token_budget: usize,
}

const fn default_prompt_token_budget() -> usize {
    10_000
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_plan_usd: 0.0,
            max_turn_usd: 0.0,
            prompt_token_budget: default_prompt_token_budget(),
        }
    }
}
