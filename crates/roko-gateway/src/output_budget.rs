//! Learned per-model output caps.

use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

const EMA_ALPHA: f64 = 0.05;
const MIN_SAMPLES: u64 = 20;
const MIN_CAP: u32 = 1_024;

/// Per-model exponential output statistics.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ModelOutputStats {
    /// EMA of output tokens.
    pub ema: f64,
    /// EMA of squared output tokens.
    pub ema_sq: f64,
    /// Largest observed response.
    pub max_seen: u64,
    /// Observation count.
    pub count: u64,
}

impl ModelOutputStats {
    fn record(&mut self, output_tokens: u64) {
        let sample = output_tokens as f64;
        if self.count == 0 {
            self.ema = sample;
            self.ema_sq = sample * sample;
        } else {
            self.ema = EMA_ALPHA.mul_add(sample, (1.0 - EMA_ALPHA) * self.ema);
            self.ema_sq = EMA_ALPHA.mul_add(sample * sample, (1.0 - EMA_ALPHA) * self.ema_sq);
        }
        self.max_seen = self.max_seen.max(output_tokens);
        self.count = self.count.saturating_add(1);
    }

    /// Trusted p95-derived cap once the minimum sample count is met.
    #[must_use]
    pub fn cap(&self) -> Option<u32> {
        if self.count < MIN_SAMPLES {
            return None;
        }
        let variance = (self.ema_sq - self.ema * self.ema).max(0.0);
        let p95 = self.ema + 2.0 * variance.sqrt();
        let cap = (p95 * 1.5).ceil().max(f64::from(MIN_CAP));
        Some(cap.min(f64::from(u32::MAX)) as u32)
    }
}

/// Output-budget telemetry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct OutputBudgetStats {
    /// Requests whose maximum was inserted or reduced.
    pub output_budgets_applied: u64,
    /// Requested tokens removed by high-cap reductions.
    pub output_tokens_bounded: u64,
}

/// Thread-safe per-model output budget learner.
#[derive(Default)]
pub struct OutputBudgeter {
    models: RwLock<HashMap<String, ModelOutputStats>>,
    output_budgets_applied: AtomicU64,
    output_tokens_bounded: AtomicU64,
}

impl OutputBudgeter {
    /// Update a model's exponential output statistics.
    pub fn record_output(&self, model: &str, output_tokens: u64) {
        if let Ok(mut models) = self.models.write() {
            models
                .entry(model.to_string())
                .or_default()
                .record(output_tokens);
        }
    }

    /// Return a learned cap only when the request should be changed.
    #[must_use]
    pub fn apply_budget(&self, model: &str, current_max_tokens: Option<u32>) -> Option<u32> {
        let cap = self.models.read().ok()?.get(model)?.cap()?;
        match current_max_tokens {
            None => {
                self.output_budgets_applied.fetch_add(1, Ordering::Relaxed);
                Some(cap)
            }
            Some(current) if u64::from(current) > u64::from(cap).saturating_mul(2) => {
                self.output_budgets_applied.fetch_add(1, Ordering::Relaxed);
                self.output_tokens_bounded
                    .fetch_add(u64::from(current - cap), Ordering::Relaxed);
                Some(cap)
            }
            Some(_) => None,
        }
    }

    /// Current statistics for a model.
    #[must_use]
    pub fn model_stats(&self, model: &str) -> Option<ModelOutputStats> {
        self.models.read().ok()?.get(model).copied()
    }

    /// Aggregate budget counters.
    #[must_use]
    pub fn stats(&self) -> OutputBudgetStats {
        OutputBudgetStats {
            output_budgets_applied: self.output_budgets_applied.load(Ordering::Relaxed),
            output_tokens_bounded: self.output_tokens_bounded.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_budget_waits_for_twenty_samples_and_applies_floor() {
        let budgeter = OutputBudgeter::default();
        for _ in 0..19 {
            budgeter.record_output("model", 100);
        }
        assert_eq!(budgeter.apply_budget("model", None), None);
        budgeter.record_output("model", 100);
        assert_eq!(budgeter.apply_budget("model", None), Some(1_024));
    }

    #[test]
    fn output_budget_caps_only_absent_or_unreasonably_high_values() {
        let budgeter = OutputBudgeter::default();
        for sample in [1_000_u64, 1_100, 900, 1_050, 950]
            .into_iter()
            .cycle()
            .take(20)
        {
            budgeter.record_output("model", sample);
        }
        let cap = budgeter.model_stats("model").unwrap().cap().unwrap();
        assert_eq!(budgeter.apply_budget("model", Some(cap)), None);
        assert_eq!(budgeter.apply_budget("model", Some(cap / 2)), None);
        assert_eq!(budgeter.apply_budget("model", Some(cap * 2)), None);
        assert_eq!(budgeter.apply_budget("model", Some(cap * 2 + 1)), Some(cap));
        assert_eq!(budgeter.stats().output_budgets_applied, 1);
    }
}
