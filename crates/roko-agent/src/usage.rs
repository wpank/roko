//! Token usage and cost tracking for agent runs.

use serde::{Deserialize, Serialize};

/// Usage metrics from a single agent invocation.
///
/// Populated by agents that can count tokens and/or track cost (e.g. via
/// LLM API responses). Mock agents and simple exec agents may leave these
/// fields at their defaults.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    /// Input (prompt) tokens consumed.
    pub input_tokens: u32,
    /// Output (completion) tokens produced.
    pub output_tokens: u32,
    /// Cache-read tokens (from prompt caching, if supported).
    pub cache_read_tokens: u32,
    /// Cache-creation tokens (wrote to prompt cache).
    pub cache_create_tokens: u32,
    /// Estimated cost in USD.
    pub cost_usd: f32,
    /// Wall-clock duration in milliseconds.
    pub wall_ms: u64,
}

impl Usage {
    /// An empty usage record.
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_create_tokens: 0,
            cost_usd: 0.0,
            wall_ms: 0,
        }
    }

    /// Total tokens consumed (input + output + cache-create; excludes cache reads).
    #[must_use]
    pub const fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens + self.cache_create_tokens
    }

    /// Add another usage record into this one (for aggregating multi-turn runs).
    pub fn add(&mut self, other: &Self) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.cache_create_tokens += other.cache_create_tokens;
        self.cost_usd += other.cost_usd;
        self.wall_ms += other.wall_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_default() {
        assert_eq!(Usage::zero(), Usage::default());
    }

    #[test]
    fn add_aggregates() {
        let mut a = Usage {
            input_tokens: 10,
            output_tokens: 20,
            cost_usd: 0.01,
            wall_ms: 500,
            ..Default::default()
        };
        let b = Usage {
            input_tokens: 5,
            output_tokens: 10,
            cost_usd: 0.005,
            wall_ms: 200,
            ..Default::default()
        };
        a.add(&b);
        assert_eq!(a.input_tokens, 15);
        assert_eq!(a.output_tokens, 30);
        assert!((a.cost_usd - 0.015).abs() < 1e-6);
        assert_eq!(a.wall_ms, 700);
    }

    #[test]
    fn total_tokens_excludes_cache_reads() {
        let u = Usage {
            input_tokens: 100,
            output_tokens: 200,
            cache_read_tokens: 1000, // big cache reads shouldn't pad the total
            cache_create_tokens: 50,
            ..Default::default()
        };
        assert_eq!(u.total_tokens(), 350);
    }
}
