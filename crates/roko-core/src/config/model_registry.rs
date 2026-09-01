//! Static registry of well-known built-in models.
//!
//! Provides [`BUILTIN_MODELS`] — a compile-time table of popular models across
//! Anthropic, OpenAI, Gemini, and Perplexity — and a [`builtin_model`] lookup
//! function that resolves by exact slug or common alias.
//!
//! On top of that table sits the shared [`model_meta`] resolver: one place
//! that classifies any model slug into family / tier / context window /
//! pricing, so the TUI and the learning layer stop re-implementing substring
//! matchers. Pricing rows come from [`BUILTIN_PRICING`].

use crate::agent::{ModelTier, ProviderKind};

/// A single entry in the built-in model registry.
#[derive(Debug, Clone)]
pub struct BuiltinModel {
    /// Wire slug sent to the provider API (e.g. `"claude-opus-4-6"`).
    pub slug: &'static str,
    /// Provider protocol family.
    pub provider_kind: ProviderKind,
    /// Context window in tokens.
    pub context_window: u64,
    /// Maximum output tokens.
    pub max_output: u64,
    /// Whether the model supports tool/function calling.
    pub supports_tools: bool,
    /// Whether the model supports image/vision inputs.
    pub supports_vision: bool,
    /// Whether the model supports thinking/reasoning output.
    pub supports_thinking: bool,
    /// Whether the model uses `max_completion_tokens` instead of `max_tokens`.
    pub use_max_completion_tokens: bool,
    /// Environment variable name holding the API key.
    pub api_key_env: &'static str,
}

/// All well-known built-in models.
pub static BUILTIN_MODELS: &[BuiltinModel] = &[
    // ── Anthropic ────────────────────────────────────────────────────────
    BuiltinModel {
        slug: "claude-opus-4-6",
        provider_kind: ProviderKind::AnthropicApi,
        context_window: 200_000,
        max_output: 32_000,
        supports_tools: true,
        supports_vision: true,
        supports_thinking: true,
        use_max_completion_tokens: false,
        api_key_env: "ANTHROPIC_API_KEY",
    },
    BuiltinModel {
        slug: "claude-sonnet-4-6",
        provider_kind: ProviderKind::AnthropicApi,
        context_window: 200_000,
        max_output: 16_384,
        supports_tools: true,
        supports_vision: true,
        supports_thinking: true,
        use_max_completion_tokens: false,
        api_key_env: "ANTHROPIC_API_KEY",
    },
    BuiltinModel {
        slug: "claude-haiku-4-5",
        provider_kind: ProviderKind::AnthropicApi,
        context_window: 200_000,
        max_output: 8_192,
        supports_tools: true,
        supports_vision: true,
        supports_thinking: false,
        use_max_completion_tokens: false,
        api_key_env: "ANTHROPIC_API_KEY",
    },
    // ── OpenAI ───────────────────────────────────────────────────────────
    BuiltinModel {
        slug: "gpt-5.5",
        provider_kind: ProviderKind::OpenAiCompat,
        context_window: 200_000,
        max_output: 32_768,
        supports_tools: true,
        supports_vision: true,
        supports_thinking: true,
        use_max_completion_tokens: true,
        api_key_env: "OPENAI_API_KEY",
    },
    BuiltinModel {
        slug: "gpt-5.4-mini",
        provider_kind: ProviderKind::OpenAiCompat,
        context_window: 200_000,
        max_output: 16_384,
        supports_tools: true,
        supports_vision: true,
        supports_thinking: false,
        use_max_completion_tokens: true,
        api_key_env: "OPENAI_API_KEY",
    },
    BuiltinModel {
        slug: "o3",
        provider_kind: ProviderKind::OpenAiCompat,
        context_window: 200_000,
        max_output: 100_000,
        supports_tools: true,
        supports_vision: true,
        supports_thinking: true,
        use_max_completion_tokens: true,
        api_key_env: "OPENAI_API_KEY",
    },
    BuiltinModel {
        slug: "o4-mini",
        provider_kind: ProviderKind::OpenAiCompat,
        context_window: 200_000,
        max_output: 100_000,
        supports_tools: true,
        supports_vision: true,
        supports_thinking: true,
        use_max_completion_tokens: true,
        api_key_env: "OPENAI_API_KEY",
    },
    BuiltinModel {
        slug: "gpt-4o",
        provider_kind: ProviderKind::OpenAiCompat,
        context_window: 128_000,
        max_output: 16_384,
        supports_tools: true,
        supports_vision: true,
        supports_thinking: false,
        use_max_completion_tokens: true,
        api_key_env: "OPENAI_API_KEY",
    },
    BuiltinModel {
        slug: "codex-mini",
        provider_kind: ProviderKind::OpenAiCompat,
        context_window: 200_000,
        max_output: 16_384,
        supports_tools: true,
        supports_vision: false,
        supports_thinking: true,
        use_max_completion_tokens: true,
        api_key_env: "OPENAI_API_KEY",
    },
    // ── Gemini ───────────────────────────────────────────────────────────
    BuiltinModel {
        slug: "gemini-2.5-pro",
        provider_kind: ProviderKind::GeminiApi,
        context_window: 1_048_576,
        max_output: 65_536,
        supports_tools: true,
        supports_vision: true,
        supports_thinking: true,
        use_max_completion_tokens: false,
        api_key_env: "GEMINI_API_KEY",
    },
    BuiltinModel {
        slug: "gemini-2.5-flash",
        provider_kind: ProviderKind::GeminiApi,
        context_window: 1_048_576,
        max_output: 65_536,
        supports_tools: true,
        supports_vision: true,
        supports_thinking: true,
        use_max_completion_tokens: false,
        api_key_env: "GEMINI_API_KEY",
    },
    // ── Perplexity ───────────────────────────────────────────────────────
    BuiltinModel {
        slug: "sonar-pro",
        provider_kind: ProviderKind::PerplexityApi,
        context_window: 200_000,
        max_output: 8_000,
        supports_tools: false,
        supports_vision: false,
        supports_thinking: false,
        use_max_completion_tokens: false,
        api_key_env: "PERPLEXITY_API_KEY",
    },
    BuiltinModel {
        slug: "sonar",
        provider_kind: ProviderKind::PerplexityApi,
        context_window: 128_000,
        max_output: 8_000,
        supports_tools: false,
        supports_vision: false,
        supports_thinking: false,
        use_max_completion_tokens: false,
        api_key_env: "PERPLEXITY_API_KEY",
    },
];

/// Common aliases mapped to their canonical slug.
pub static ALIASES: &[(&str, &str)] = &[
    // Anthropic short names
    ("opus", "claude-opus-4-6"),
    ("claude-opus", "claude-opus-4-6"),
    ("sonnet", "claude-sonnet-4-6"),
    ("claude-sonnet", "claude-sonnet-4-6"),
    ("haiku", "claude-haiku-4-5"),
    ("claude-haiku", "claude-haiku-4-5"),
    // OpenAI short names
    ("gpt5", "gpt-5.5"),
    ("gpt-5", "gpt-5.5"),
    ("4o", "gpt-4o"),
    ("codex", "codex-mini"),
    // Gemini short names
    ("gemini-pro", "gemini-2.5-pro"),
    ("gemini-flash", "gemini-2.5-flash"),
    ("flash", "gemini-2.5-flash"),
];

/// Look up a built-in model by exact slug or common alias.
///
/// Returns `None` if the slug is not recognized.
#[must_use]
pub fn builtin_model(slug: &str) -> Option<&'static BuiltinModel> {
    // Exact match first.
    if let Some(m) = BUILTIN_MODELS.iter().find(|m| m.slug == slug) {
        return Some(m);
    }
    // Alias resolution.
    let canonical = ALIASES.iter().find(|(alias, _)| *alias == slug)?.1;
    BUILTIN_MODELS.iter().find(|m| m.slug == canonical)
}

// ── Shared model metadata resolver ──────────────────────────────────────────

/// Pricing for a well-known model, in USD per million tokens.
///
/// Rates are seeded from the cost tables already present in the workspace
/// (`roko-learn/src/cost_table.rs`, `roko-agent/src/task_runner.rs`,
/// `roko-agent/src/provider/codex_cli/stream.rs`) and from
/// `examples/roko-perplexity.toml` for Sonar. Cache-write rows marked
/// "derived" follow the `CostTable::from_config` convention
/// (`input * 1.25`) where no explicit rate exists in-repo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPricing {
    /// Cost in USD per million input tokens.
    pub input_per_m: f64,
    /// Cost in USD per million output tokens.
    pub output_per_m: f64,
    /// Cost in USD per million cache-read tokens.
    pub cache_read_per_m: f64,
    /// Cost in USD per million cache-write tokens.
    pub cache_write_per_m: f64,
    /// Tokenizer size ratio relative to OpenAI `o200k_base`.
    pub tokenizer_ratio: f64,
}

/// Pricing rows for well-known models, keyed by canonical slug.
///
/// Lookup uses the same exact-then-longest-prefix rule as
/// `CostTable::lookup`, so date- or variant-suffixed slugs
/// (`claude-sonnet-4-6-20250514`) resolve to their base model's rates.
pub static BUILTIN_PRICING: &[(&str, ModelPricing)] = &[
    // Anthropic — rates match cost_table.rs / task_runner.rs.
    (
        "claude-opus-4-6",
        ModelPricing {
            input_per_m: 15.00,
            output_per_m: 75.00,
            cache_read_per_m: 3.75,
            cache_write_per_m: 18.75,
            tokenizer_ratio: 1.0,
        },
    ),
    (
        "claude-sonnet-4-6",
        ModelPricing {
            input_per_m: 3.00,
            output_per_m: 15.00,
            cache_read_per_m: 0.30,
            cache_write_per_m: 3.75,
            tokenizer_ratio: 1.0,
        },
    ),
    (
        "claude-haiku-4-5",
        ModelPricing {
            input_per_m: 0.80,
            output_per_m: 4.00,
            cache_read_per_m: 0.08,
            cache_write_per_m: 1.00,
            tokenizer_ratio: 1.0,
        },
    ),
    // Z.AI GLM — rates match cost_table.rs / task_runner.rs.
    (
        "glm-5.1",
        ModelPricing {
            input_per_m: 1.40,
            output_per_m: 4.40,
            cache_read_per_m: 0.26,
            cache_write_per_m: 1.75,
            tokenizer_ratio: 1.05,
        },
    ),
    (
        "glm-5",
        ModelPricing {
            input_per_m: 1.00,
            output_per_m: 3.20,
            cache_read_per_m: 0.50,
            cache_write_per_m: 1.25,
            tokenizer_ratio: 1.05,
        },
    ),
    // Moonshot Kimi — rates match cost_table.rs / task_runner.rs.
    (
        "kimi-k2.5",
        ModelPricing {
            input_per_m: 0.60,
            output_per_m: 3.00,
            cache_read_per_m: 0.10,
            cache_write_per_m: 0.75,
            tokenizer_ratio: 0.98,
        },
    ),
    // OpenAI — gpt-5.2/5.4 rates match cost_table.rs; gpt-5.5 aligns with the
    // gpt-5 rate in roko-compose/src/enrichment/estimate.rs.
    (
        "gpt-5.2",
        ModelPricing {
            input_per_m: 2.00,
            output_per_m: 8.00,
            cache_read_per_m: 0.50,
            cache_write_per_m: 2.50,
            tokenizer_ratio: 1.0,
        },
    ),
    (
        "gpt-5.4",
        ModelPricing {
            input_per_m: 2.50,
            output_per_m: 10.00,
            cache_read_per_m: 0.63,
            cache_write_per_m: 3.13,
            tokenizer_ratio: 1.0,
        },
    ),
    (
        "gpt-5.4-mini",
        ModelPricing {
            input_per_m: 0.40,
            output_per_m: 1.60,
            cache_read_per_m: 0.10,
            cache_write_per_m: 0.50,
            tokenizer_ratio: 1.0,
        },
    ),
    (
        "gpt-5.5",
        ModelPricing {
            input_per_m: 2.50,
            output_per_m: 10.00,
            cache_read_per_m: 0.63,
            cache_write_per_m: 3.13,
            tokenizer_ratio: 1.0,
        },
    ),
    // Codex — gpt-5.6-sol rates from codex_cli/stream.rs ($2/$0.50 cached/$8);
    // cache-write derived (input * 1.25). codex-mini mirrors the only codex
    // rates available in-repo until provider-specific rows exist.
    (
        "gpt-5.6-sol",
        ModelPricing {
            input_per_m: 2.00,
            output_per_m: 8.00,
            cache_read_per_m: 0.50,
            cache_write_per_m: 2.50,
            tokenizer_ratio: 1.0,
        },
    ),
    (
        "codex-mini",
        ModelPricing {
            input_per_m: 2.00,
            output_per_m: 8.00,
            cache_read_per_m: 0.50,
            cache_write_per_m: 2.50,
            tokenizer_ratio: 1.0,
        },
    ),
    // Perplexity — token rates from examples/roko-perplexity.toml (Sonar also
    // bills per request; that fee lives on `ModelProfile::cost_per_request`).
    // Cache rows derived; Sonar does not support prompt caching.
    (
        "sonar",
        ModelPricing {
            input_per_m: 1.00,
            output_per_m: 1.00,
            cache_read_per_m: 0.50,
            cache_write_per_m: 1.25,
            tokenizer_ratio: 1.0,
        },
    ),
    (
        "sonar-pro",
        ModelPricing {
            input_per_m: 3.00,
            output_per_m: 15.00,
            cache_read_per_m: 1.50,
            cache_write_per_m: 3.75,
            tokenizer_ratio: 1.0,
        },
    ),
];

/// Look up pricing for a model slug.
///
/// Tries an exact match first, then any table key that is a prefix of the
/// slug separated by `-` or `.` (longest prefix wins). Matching is
/// case-insensitive. Returns `None` for unknown slugs.
#[must_use]
pub fn builtin_pricing(slug: &str) -> Option<ModelPricing> {
    let lower = slug.to_ascii_lowercase();
    if let Some((_, pricing)) = BUILTIN_PRICING.iter().find(|(key, _)| *key == lower) {
        return Some(*pricing);
    }
    BUILTIN_PRICING
        .iter()
        .filter(|(key, _)| {
            lower.len() > key.len()
                && lower.starts_with(key)
                && matches!(lower.as_bytes().get(key.len()), Some(b'-' | b'.'))
        })
        .max_by_key(|(key, _)| key.len())
        .map(|(_, pricing)| *pricing)
}

/// Resolved metadata for a model slug: the single source of truth shared by
/// the TUI and the learning layer for family / tier / context / pricing.
///
/// Built by [`model_meta`]. Fields that cannot be resolved (unknown slugs)
/// are `None` except `family` and `tier`, which fall back to heuristics
/// (`"unknown"` / [`ModelTier::Standard`]).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelMeta {
    /// Coarse family: `"claude"`, `"gpt"`, `"codex"`, `"glm"`, `"kimi"`,
    /// `"gemini"`, `"sonar"`, `"deepseek"`, or `"unknown"`.
    pub family: &'static str,
    /// Capability tier for routing/display (Fast/Standard/Premium).
    pub tier: ModelTier,
    /// Context window in tokens, when the slug is in [`BUILTIN_MODELS`].
    pub context_window: Option<u64>,
    /// Maximum output tokens, when the slug is in [`BUILTIN_MODELS`].
    pub max_output: Option<u64>,
    /// Pricing from [`BUILTIN_PRICING`] (exact or longest-prefix match).
    pub pricing: Option<ModelPricing>,
    /// Canonical registry slug when resolved via [`BUILTIN_MODELS`] or
    /// [`ALIASES`]; `None` for unregistered slugs.
    pub canonical_slug: Option<&'static str>,
}

/// Classify a (lowercased) slug into a coarse model family.
fn family_for_slug(slug: &str) -> &'static str {
    if slug.starts_with("claude")
        || slug.contains("haiku")
        || slug.contains("sonnet")
        || slug.contains("opus")
    {
        "claude"
    } else if slug.contains("codex") {
        "codex"
    } else if slug.starts_with("gpt")
        || slug.starts_with("o1")
        || slug.starts_with("o3")
        || slug.starts_with("o4")
    {
        "gpt"
    } else if slug.contains("glm") {
        "glm"
    } else if slug.starts_with("kimi") {
        "kimi"
    } else if slug.contains("gemini") {
        "gemini"
    } else if slug.starts_with("sonar") {
        "sonar"
    } else if slug.starts_with("deepseek") {
        "deepseek"
    } else {
        "unknown"
    }
}

/// Classify a (lowercased) slug into a capability tier.
///
/// Fast: haiku / mini / nano / flash-lite class. Premium: opus, `-pro`
/// variants, o1/o3 reasoning, and gpt-5 mainline. Everything else Standard.
fn tier_for_slug(slug: &str) -> ModelTier {
    if slug.contains("haiku")
        || slug.contains("mini")
        || slug.contains("nano")
        || slug.contains("flash-lite")
    {
        ModelTier::Fast
    } else if slug.contains("opus")
        || slug.contains("-pro")
        || slug.starts_with("o1")
        || slug.starts_with("o3")
        || slug.starts_with("gpt-5")
        || slug.starts_with("gpt5")
    {
        ModelTier::Premium
    } else {
        ModelTier::Standard
    }
}

/// Resolve family, tier, context window, and pricing for any model slug.
///
/// Resolution order: registry exact/alias ([`builtin_model`]) supplies the
/// canonical slug and context sizes; [`builtin_pricing`] supplies pricing
/// (exact then longest-prefix, so dated variants resolve), retried against
/// the canonical slug when the input was an alias; family and tier come from
/// substring heuristics that also cover unregistered slugs. Matching is
/// case-insensitive.
#[must_use]
pub fn model_meta(slug: &str) -> ModelMeta {
    let lower = slug.to_ascii_lowercase();
    let registered = builtin_model(&lower);
    let pricing =
        builtin_pricing(&lower).or_else(|| registered.and_then(|m| builtin_pricing(m.slug)));
    ModelMeta {
        family: family_for_slug(&lower),
        tier: tier_for_slug(&lower),
        context_window: registered.map(|m| m.context_window),
        max_output: registered.map(|m| m.max_output),
        pricing,
        canonical_slug: registered.map(|m| m.slug),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_slug_lookup() {
        let m = builtin_model("claude-opus-4-6").expect("opus by slug");
        assert_eq!(m.slug, "claude-opus-4-6");
        assert_eq!(m.provider_kind, ProviderKind::AnthropicApi);
        assert!(m.supports_thinking);
    }

    #[test]
    fn alias_lookup() {
        let m = builtin_model("opus").expect("opus alias");
        assert_eq!(m.slug, "claude-opus-4-6");

        let m = builtin_model("sonnet").expect("sonnet alias");
        assert_eq!(m.slug, "claude-sonnet-4-6");

        let m = builtin_model("haiku").expect("haiku alias");
        assert_eq!(m.slug, "claude-haiku-4-5");

        let m = builtin_model("flash").expect("flash alias");
        assert_eq!(m.slug, "gemini-2.5-flash");
    }

    #[test]
    fn unknown_returns_none() {
        assert!(builtin_model("nonexistent-model").is_none());
    }

    #[test]
    fn all_models_present() {
        assert_eq!(BUILTIN_MODELS.len(), 13);
    }

    #[test]
    fn openai_models_use_max_completion_tokens() {
        for m in BUILTIN_MODELS {
            if m.provider_kind == ProviderKind::OpenAiCompat {
                assert!(
                    m.use_max_completion_tokens,
                    "{} should use max_completion_tokens",
                    m.slug
                );
            }
        }
    }

    #[test]
    fn vision_capable_models_have_supports_vision() {
        let expected = [
            "claude-opus-4-6",
            "claude-sonnet-4-6",
            "claude-haiku-4-5",
            "gpt-5.5",
            "gpt-5.4-mini",
            "o3",
            "o4-mini",
            "gpt-4o",
            "gemini-2.5-pro",
            "gemini-2.5-flash",
        ];
        for m in BUILTIN_MODELS {
            assert_eq!(
                m.supports_vision,
                expected.contains(&m.slug),
                "unexpected vision capability for {}",
                m.slug
            );
            if m.supports_vision {
                assert!(
                    m.provider_kind.supports_inline_images(),
                    "{} advertises vision but provider {} lacks an inline-image wire path",
                    m.slug,
                    m.provider_kind
                );
            }
        }
    }

    #[test]
    fn builtin_pricing_exact_and_longest_prefix() {
        let sonnet = builtin_pricing("claude-sonnet-4-6").expect("sonnet pricing");
        assert!((sonnet.input_per_m - 3.00).abs() < 1e-12);
        assert!((sonnet.output_per_m - 15.00).abs() < 1e-12);

        // Date-suffixed variants resolve to the base model's rates.
        let dated = builtin_pricing("claude-sonnet-4-6-20250514").expect("dated variant");
        assert_eq!(dated, sonnet);

        // Longest prefix wins: gpt-5.4-mini must not resolve to gpt-5.4 rates.
        let mini = builtin_pricing("gpt-5.4-mini").expect("mini pricing");
        assert!((mini.input_per_m - 0.40).abs() < 1e-12);
        let base = builtin_pricing("gpt-5.4").expect("base pricing");
        assert!((base.input_per_m - 2.50).abs() < 1e-12);

        // sonar is a prefix of sonar-pro; each keeps its own rates.
        let sonar = builtin_pricing("sonar").expect("sonar pricing");
        assert!((sonar.input_per_m - 1.00).abs() < 1e-12);
        let sonar_pro = builtin_pricing("sonar-pro").expect("sonar-pro pricing");
        assert!((sonar_pro.input_per_m - 3.00).abs() < 1e-12);

        // No partial-word matches; unknown slugs yield None.
        assert!(builtin_pricing("glmx").is_none());
        assert!(builtin_pricing("my-fine-tuned-model").is_none());
    }

    #[test]
    fn builtin_pricing_case_insensitive() {
        let pricing = builtin_pricing("CLAUDE-HAIKU-4-5").expect("uppercase slug");
        assert!((pricing.input_per_m - 0.80).abs() < 1e-12);
    }

    #[test]
    fn model_meta_registered_slug() {
        let meta = model_meta("claude-opus-4-6");
        assert_eq!(meta.family, "claude");
        assert_eq!(meta.tier, ModelTier::Premium);
        assert_eq!(meta.context_window, Some(200_000));
        assert_eq!(meta.canonical_slug, Some("claude-opus-4-6"));
        let pricing = meta.pricing.expect("opus pricing");
        assert!((pricing.input_per_m - 15.00).abs() < 1e-12);
        assert!((pricing.output_per_m - 75.00).abs() < 1e-12);
    }

    #[test]
    fn model_meta_alias_resolves_canonical() {
        let meta = model_meta("sonnet");
        assert_eq!(meta.canonical_slug, Some("claude-sonnet-4-6"));
        assert_eq!(meta.family, "claude");
        assert_eq!(meta.tier, ModelTier::Standard);
        // Aliases resolve pricing through their canonical slug.
        let pricing = meta.pricing.expect("alias pricing");
        assert!((pricing.input_per_m - 3.00).abs() < 1e-12);
        assert!((pricing.output_per_m - 15.00).abs() < 1e-12);
    }

    #[test]
    fn model_meta_codex_family() {
        let meta = model_meta("codex-mini");
        assert_eq!(meta.family, "codex");
        assert_eq!(meta.tier, ModelTier::Fast);
        assert_eq!(meta.canonical_slug, Some("codex-mini"));
        let pricing = meta.pricing.expect("codex-mini pricing");
        assert!((pricing.input_per_m - 2.00).abs() < 1e-12);
        assert!((pricing.output_per_m - 8.00).abs() < 1e-12);

        // Codex CLI default slug: gpt family, gpt-5-class tier, priced.
        let meta = model_meta("gpt-5.6-sol");
        assert_eq!(meta.family, "gpt");
        assert_eq!(meta.tier, ModelTier::Premium);
        assert_eq!(meta.canonical_slug, None);
        let pricing = meta.pricing.expect("gpt-5.6-sol pricing");
        assert!((pricing.input_per_m - 2.00).abs() < 1e-12);

        // gpt-*-codex slugs classify as codex family.
        assert_eq!(model_meta("gpt-5-codex").family, "codex");

        // The "codex" alias resolves to codex-mini and its pricing.
        let meta = model_meta("codex");
        assert_eq!(meta.canonical_slug, Some("codex-mini"));
        assert_eq!(meta.family, "codex");
        assert!(meta.pricing.is_some());
    }

    #[test]
    fn model_meta_glm_kimi_sonar() {
        let glm = model_meta("glm-5.1");
        assert_eq!(glm.family, "glm");
        assert_eq!(glm.tier, ModelTier::Standard);
        let pricing = glm.pricing.expect("glm pricing");
        assert!((pricing.input_per_m - 1.40).abs() < 1e-12);
        assert!((pricing.output_per_m - 4.40).abs() < 1e-12);

        let kimi = model_meta("kimi-k2.5");
        assert_eq!(kimi.family, "kimi");
        let pricing = kimi.pricing.expect("kimi pricing");
        assert!((pricing.input_per_m - 0.60).abs() < 1e-12);
        assert!((pricing.cache_read_per_m - 0.10).abs() < 1e-12);

        let sonar = model_meta("sonar");
        assert_eq!(sonar.family, "sonar");
        assert_eq!(sonar.context_window, Some(128_000));
        assert!(sonar.pricing.is_some());
    }

    #[test]
    fn model_meta_unknown_slug() {
        let meta = model_meta("some-mystery-model-v3");
        assert_eq!(meta.family, "unknown");
        assert_eq!(meta.tier, ModelTier::Standard);
        assert_eq!(meta.context_window, None);
        assert_eq!(meta.max_output, None);
        assert_eq!(meta.pricing, None);
        assert_eq!(meta.canonical_slug, None);
    }

    #[test]
    fn model_meta_case_insensitive() {
        let meta = model_meta("Claude-Haiku-4-5");
        assert_eq!(meta.family, "claude");
        assert_eq!(meta.tier, ModelTier::Fast);
        assert_eq!(meta.canonical_slug, Some("claude-haiku-4-5"));
    }

    #[test]
    fn pricing_rows_cover_learning_cost_table_models() {
        // Rows the learning-layer cost tables already knew about, plus the
        // codex / sonar rows added for the efficiency pipeline.
        for slug in [
            "glm-5.1",
            "glm-5",
            "kimi-k2.5",
            "gpt-5.2",
            "gpt-5.4",
            "gpt-5.4-mini",
            "gpt-5.5",
            "gpt-5.6-sol",
            "codex-mini",
            "sonar",
            "sonar-pro",
        ] {
            assert!(
                builtin_pricing(slug).is_some(),
                "{slug} missing from BUILTIN_PRICING"
            );
        }
    }
}
