//! Pre-dispatch cost projection utilities.
//!
//! Given an estimated prompt token count and a model slug, [`project_task_cost`]
//! returns a [`CostProjection`] with predicted input and output token counts and
//! the resulting cost in USD.
//!
//! The pricing table used here is intentionally separate from the live
//! [`CostTable`] that reads `roko.toml` model profiles; it provides hardcoded
//! conservative defaults for use in contexts where a full config is not
//! available (e.g. pre-flight CLI checks, TUI estimates).
//!
//! # Default output token assumption
//!
//! Output token counts are unknown before a request is issued.  We assume
//! **512 output tokens** as a conservative minimum that keeps projections
//! usable without being wildly optimistic.  Callers with a specific
//! `max_tokens` value should multiply `estimated_output_tokens` accordingly.

use serde::{Deserialize, Serialize};

/// Projected cost for a single LLM task dispatch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostProjection {
    /// Estimated number of input (prompt) tokens.
    pub estimated_input_tokens: u64,
    /// Estimated number of output (completion) tokens.
    pub estimated_output_tokens: u64,
    /// Predicted cost in USD for the estimated token counts.
    pub estimated_cost_usd: f64,
}

/// Default assumed output token count when `max_tokens` is unknown.
const DEFAULT_OUTPUT_TOKENS: u64 = 512;

/// Fallback pricing used when the model slug is not in the pricing table.
///
/// Uses conservative Sonnet-equivalent rates as a safe over-estimate:
/// - $0.003 / 1K input tokens
/// - $0.015 / 1K output tokens
const FALLBACK_INPUT_PER_K: f64 = 0.003;
const FALLBACK_OUTPUT_PER_K: f64 = 0.015;

/// Per-model pricing entry used by the local projection table.
struct Pricing {
    /// Cost in USD per 1 000 input tokens.
    input_per_k: f64,
    /// Cost in USD per 1 000 output tokens.
    output_per_k: f64,
}

/// Project the cost of a single task dispatch given a prompt token estimate and
/// a model slug.
///
/// # Parameters
///
/// - `prompt_tokens`: estimated input token count (e.g. from
///   `roko_agent::estimate_prompt_tokens`).
/// - `model_slug`: canonical model slug such as `"claude-sonnet-4-6"` or
///   `"gpt-5.4-mini"`.  Prefix matching is performed so
///   `"claude-sonnet-4-6-20250514"` resolves to the `claude-sonnet-4-6` entry.
///
/// # Returns
///
/// A [`CostProjection`] with the estimated input tokens, a conservative output
/// token assumption of [`DEFAULT_OUTPUT_TOKENS`], and the projected USD cost.
#[must_use]
pub fn project_task_cost(prompt_tokens: u64, model_slug: &str) -> CostProjection {
    let output_tokens = DEFAULT_OUTPUT_TOKENS;

    let pricing = resolve_pricing(model_slug);

    let estimated_cost_usd = (prompt_tokens as f64 * pricing.input_per_k / 1_000.0)
        + (output_tokens as f64 * pricing.output_per_k / 1_000.0);

    CostProjection {
        estimated_input_tokens: prompt_tokens,
        estimated_output_tokens: output_tokens,
        estimated_cost_usd,
    }
}

/// Project the cost with an explicit output token count.
///
/// Useful when the caller knows `max_tokens` from the model profile or task
/// config and wants a tighter estimate than the [`DEFAULT_OUTPUT_TOKENS`]
/// assumption.
#[must_use]
pub fn project_task_cost_with_output(
    prompt_tokens: u64,
    output_tokens: u64,
    model_slug: &str,
) -> CostProjection {
    let pricing = resolve_pricing(model_slug);

    let estimated_cost_usd = (prompt_tokens as f64 * pricing.input_per_k / 1_000.0)
        + (output_tokens as f64 * pricing.output_per_k / 1_000.0);

    CostProjection {
        estimated_input_tokens: prompt_tokens,
        estimated_output_tokens: output_tokens,
        estimated_cost_usd,
    }
}

// ── pricing table ─────────────────────────────────────────────────────────────

/// Per-model pricing table.
///
/// Entries are `(slug_prefix, input_per_k_usd, output_per_k_usd)`.
/// Resolution uses prefix matching so slugs with date suffixes such as
/// `"claude-sonnet-4-6-20250514"` still resolve to the `"claude-sonnet-4-6"`
/// entry.
///
/// Prices are expressed per-1K tokens (not per-million) to keep the arithmetic
/// readable at typical task scale.
static PRICING_TABLE: &[(&str, f64, f64)] = &[
    // Anthropic Claude models
    ("claude-opus-4-6", 0.015, 0.075),
    ("claude-sonnet-4-6", 0.003, 0.015),
    ("claude-haiku-4-5", 0.0008, 0.004),
    // GLM / ZhipuAI
    ("glm-5.1", 0.0014, 0.0044),
    ("glm-5", 0.001, 0.0032),
    // Kimi
    ("kimi-k2.5", 0.0006, 0.003),
    // OpenAI GPT-5 series
    ("gpt-5.4-mini", 0.0004, 0.0016),
    ("gpt-5.4", 0.0025, 0.010),
    ("gpt-5.2", 0.002, 0.008),
    // Perplexity / Ollama (typically negligible or zero cost)
    ("llama", 0.0, 0.0),
    ("ollama", 0.0, 0.0),
    ("mistral", 0.001, 0.003),
];

/// Resolve pricing for a model slug, using prefix matching.
///
/// Falls back to [`FALLBACK_INPUT_PER_K`] / [`FALLBACK_OUTPUT_PER_K`] when
/// no entry matches.
fn resolve_pricing(slug: &str) -> Pricing {
    // Try exact match first.
    for &(prefix, inp, out) in PRICING_TABLE {
        if slug == prefix {
            return Pricing {
                input_per_k: inp,
                output_per_k: out,
            };
        }
    }

    // Prefix match: the table key must be a proper prefix of the slug,
    // separated by `-` or `.` at the boundary.
    let mut best: Option<(usize, f64, f64)> = None;
    for &(prefix, inp, out) in PRICING_TABLE {
        if slug.len() > prefix.len()
            && slug.starts_with(prefix)
            && matches!(slug.as_bytes().get(prefix.len()), Some(b'-' | b'.'))
        {
            if best.map_or(true, |(len, _, _)| prefix.len() > len) {
                best = Some((prefix.len(), inp, out));
            }
        }
    }

    if let Some((_, inp, out)) = best {
        return Pricing {
            input_per_k: inp,
            output_per_k: out,
        };
    }

    // Unknown model — use conservative fallback.
    Pricing {
        input_per_k: FALLBACK_INPUT_PER_K,
        output_per_k: FALLBACK_OUTPUT_PER_K,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_model_sonnet() {
        let proj = project_task_cost(1_000, "claude-sonnet-4-6");
        assert_eq!(proj.estimated_input_tokens, 1_000);
        assert_eq!(proj.estimated_output_tokens, DEFAULT_OUTPUT_TOKENS);
        // 1_000 * 0.003/1_000 + 512 * 0.015/1_000
        let expected = 1.0 * 0.003 + 512.0 * 0.015 / 1_000.0;
        assert!((proj.estimated_cost_usd - expected).abs() < 1e-12);
    }

    #[test]
    fn known_model_haiku() {
        let proj = project_task_cost(2_000, "claude-haiku-4-5");
        // 2_000 * 0.0008/1_000 + 512 * 0.004/1_000
        let expected = 2.0 * 0.0008 + 512.0 * 0.004 / 1_000.0;
        assert!((proj.estimated_cost_usd - expected).abs() < 1e-12);
    }

    #[test]
    fn prefix_match_with_date_suffix() {
        let proj_base = project_task_cost(1_000, "claude-sonnet-4-6");
        let proj_dated = project_task_cost(1_000, "claude-sonnet-4-6-20250514");
        // Both should resolve to the same pricing
        assert!((proj_base.estimated_cost_usd - proj_dated.estimated_cost_usd).abs() < 1e-12);
    }

    #[test]
    fn unknown_model_uses_fallback() {
        let proj = project_task_cost(1_000, "some-unknown-llm-v99");
        // 1_000 * FALLBACK_INPUT_PER_K/1_000 + 512 * FALLBACK_OUTPUT_PER_K/1_000
        let expected = 1.0 * FALLBACK_INPUT_PER_K + 512.0 * FALLBACK_OUTPUT_PER_K / 1_000.0;
        assert!((proj.estimated_cost_usd - expected).abs() < 1e-12);
    }

    #[test]
    fn zero_tokens_produces_zero_cost() {
        let proj = project_task_cost(0, "claude-sonnet-4-6");
        // 0 input + 512 output at sonnet rates
        let expected = 512.0 * 0.015 / 1_000.0;
        assert!((proj.estimated_cost_usd - expected).abs() < 1e-12);
    }

    #[test]
    fn explicit_output_tokens_override() {
        let proj = project_task_cost_with_output(1_000, 2_000, "claude-sonnet-4-6");
        assert_eq!(proj.estimated_output_tokens, 2_000);
        // 1_000 * 0.003/1_000 + 2_000 * 0.015/1_000
        let expected = 1.0 * 0.003 + 2.0 * 0.015;
        assert!((proj.estimated_cost_usd - expected).abs() < 1e-12);
    }

    #[test]
    fn opus_pricing() {
        let proj = project_task_cost(1_000, "claude-opus-4-6");
        // 1_000 * 0.015/1_000 + 512 * 0.075/1_000
        let expected = 1.0 * 0.015 + 512.0 * 0.075 / 1_000.0;
        assert!((proj.estimated_cost_usd - expected).abs() < 1e-12);
    }

    #[test]
    fn local_model_zero_cost() {
        let proj = project_task_cost(10_000, "ollama-llama3");
        // ollama prefix → 0.0, 0.0
        assert!((proj.estimated_cost_usd).abs() < 1e-12);
    }
}
