//! Model-family defaults for explicitly enabled extended thinking.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

use crate::{ThinkingConfig, ThinkingMode};

/// Thinking-cap telemetry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ThinkingCapStats {
    /// Requests receiving a default budget.
    pub thinking_budgets_applied: u64,
    /// Sum of default token budgets inserted.
    pub thinking_tokens_capped_estimate: u64,
}

/// Stateful cap application with aggregate counters.
#[derive(Default)]
pub struct ThinkingCapper {
    thinking_budgets_applied: AtomicU64,
    thinking_tokens_capped_estimate: AtomicU64,
}

impl ThinkingCapper {
    /// Apply a family default only to enabled thinking with no explicit budget.
    pub fn apply(&self, model: &str, thinking: &mut Option<ThinkingConfig>) -> Option<u32> {
        let budget = apply_thinking_cap(model, thinking)?;
        self.thinking_budgets_applied
            .fetch_add(1, Ordering::Relaxed);
        self.thinking_tokens_capped_estimate
            .fetch_add(u64::from(budget), Ordering::Relaxed);
        Some(budget)
    }

    /// Current cap counters.
    #[must_use]
    pub fn stats(&self) -> ThinkingCapStats {
        ThinkingCapStats {
            thinking_budgets_applied: self.thinking_budgets_applied.load(Ordering::Relaxed),
            thinking_tokens_capped_estimate: self
                .thinking_tokens_capped_estimate
                .load(Ordering::Relaxed),
        }
    }
}

/// Apply a model-family default and return the inserted budget.
pub fn apply_thinking_cap(model: &str, thinking: &mut Option<ThinkingConfig>) -> Option<u32> {
    let config = thinking.as_mut()?;
    if config.kind != ThinkingMode::Enabled || config.budget_tokens.is_some() {
        return None;
    }
    let budget = default_budget(model);
    config.budget_tokens = Some(budget);
    Some(budget)
}

/// Default budget by model-family substring.
#[must_use]
pub fn default_budget(model: &str) -> u32 {
    let model = model.to_ascii_lowercase();
    if model.contains("opus") {
        32_768
    } else if model.contains("haiku") {
        4_096
    } else {
        16_384
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinking_cap_uses_model_defaults_only_when_enabled_and_absent() {
        for (model, expected) in [
            ("claude-opus-4-6", 32_768),
            ("claude-sonnet-4-6", 16_384),
            ("claude-haiku-4-5", 4_096),
        ] {
            let mut thinking = Some(ThinkingConfig {
                kind: ThinkingMode::Enabled,
                budget_tokens: None,
            });
            assert_eq!(apply_thinking_cap(model, &mut thinking), Some(expected));
            assert_eq!(thinking.unwrap().budget_tokens, Some(expected));
        }
    }

    #[test]
    fn thinking_cap_never_forces_or_overrides() {
        let mut absent = None;
        assert_eq!(apply_thinking_cap("opus", &mut absent), None);
        let mut disabled = Some(ThinkingConfig::default());
        assert_eq!(apply_thinking_cap("opus", &mut disabled), None);
        let mut explicit = Some(ThinkingConfig {
            kind: ThinkingMode::Enabled,
            budget_tokens: Some(8_192),
        });
        assert_eq!(apply_thinking_cap("opus", &mut explicit), None);
        assert_eq!(explicit.unwrap().budget_tokens, Some(8_192));
    }
}
