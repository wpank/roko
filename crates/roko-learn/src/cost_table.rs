//! Per-model pricing tables and cost normalization utilities.

use std::collections::HashMap;

use indexmap::IndexMap;
use roko_agent::Usage;
use roko_core::config::model_registry::{
    DEFAULT_CACHE_READ_MULTIPLIER, DEFAULT_CACHE_WRITE_MULTIPLIER, is_snapshot_of,
};
use roko_core::config::schema::ModelProfile;
use serde::{Deserialize, Serialize};

pub use roko_core::config::model_registry::warn_unpriced_model;

/// Pricing for a single model slug.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Per-model pricing table keyed by canonical model slug.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostTable {
    /// Pricing entries keyed by model slug.
    pub models: HashMap<String, ModelPricing>,
}

impl CostTable {
    /// Look up pricing for a model slug.
    ///
    /// Tries exact match first, then a table key the slug is a dated or
    /// versioned snapshot of ([`is_snapshot_of`]; the longest such key wins),
    /// so `o3-mini` never takes `o3`'s rates (bug-1f81ab).
    #[must_use]
    pub fn lookup(&self, slug: &str) -> Option<&ModelPricing> {
        if let Some(pricing) = self.models.get(slug) {
            return Some(pricing);
        }
        self.models
            .iter()
            .filter(|(key, _)| is_snapshot_of(slug, key))
            .max_by_key(|(key, _)| key.len())
            .map(|(_, pricing)| pricing)
    }

    /// Price `usage` on `model_slug`, using [`lookup`](Self::lookup) for
    /// prefix matching. `None` when the slug has no row but tokens were used:
    /// an unknown model is unpriced, not priced at another model's rates
    /// (gap-ad0d39).
    #[must_use]
    pub fn price(&self, model_slug: &str, usage: &Usage) -> Option<f64> {
        let total_tokens = usage.input_tokens
            + usage.output_tokens
            + usage.cache_read_tokens
            + usage.cache_create_tokens;

        let Some(pricing) = self.lookup(model_slug) else {
            return (total_tokens == 0).then_some(0.0);
        };

        Some(
            (usage.input_tokens as f64 * pricing.input_per_m / 1_000_000.0)
                + (usage.output_tokens as f64 * pricing.output_per_m / 1_000_000.0)
                + (usage.cache_read_tokens as f64 * pricing.cache_read_per_m / 1_000_000.0)
                + (usage.cache_create_tokens as f64 * pricing.cache_write_per_m / 1_000_000.0),
        )
    }

    /// Calculate request cost from raw token counts: [`price`](Self::price),
    /// with an unpriced model at `0.0`, which `Usage::has_known_cost` reads
    /// as an unknown cost rather than a free one. Such a model is logged once;
    /// it used to be priced at Sonnet's rates (gap-ad0d39).
    #[must_use]
    pub fn calculate(&self, model_slug: &str, usage: &Usage) -> f64 {
        self.price(model_slug, usage).unwrap_or_else(|| {
            warn_unpriced_model(model_slug);
            0.0
        })
    }

    /// Normalize a token count to OpenAI-equivalent tokens for cross-provider comparison.
    #[must_use]
    pub fn normalize_tokens(&self, model_slug: &str, tokens: u64) -> u64 {
        let ratio = self
            .lookup(model_slug)
            .map(|pricing| pricing.tokenizer_ratio)
            .unwrap_or(1.0);

        (tokens as f64 * ratio) as u64
    }

    /// Return the blended per-million-token cost normalized for tokenizer size.
    ///
    /// The blended value uses a 3:1 input/output weighting, matching the
    /// Artificial Analysis methodology described in the routing plan.
    #[must_use]
    pub fn blended_cost_per_m(&self, model_slug: &str) -> f64 {
        let pricing = match self.lookup(model_slug) {
            Some(pricing) => pricing,
            None => return 0.0,
        };

        ((3.0 * pricing.input_per_m + pricing.output_per_m) / 4.0) * pricing.tokenizer_ratio
    }

    /// Load pricing rows from config model profiles. A profile with no cache
    /// prices gets the shared default multiples of its input price
    /// (bug-0c0747).
    #[must_use]
    pub fn from_config(models: &IndexMap<String, ModelProfile>) -> Self {
        let mut table = HashMap::new();

        for profile in models.values() {
            if let (Some(input), Some(output)) =
                (profile.cost_input_per_m, profile.cost_output_per_m)
            {
                table.insert(
                    profile.slug.clone(),
                    ModelPricing {
                        input_per_m: input,
                        output_per_m: output,
                        cache_read_per_m: profile
                            .cost_cache_read_per_m
                            .unwrap_or(input * DEFAULT_CACHE_READ_MULTIPLIER),
                        cache_write_per_m: profile
                            .cost_cache_write_per_m
                            .unwrap_or(input * DEFAULT_CACHE_WRITE_MULTIPLIER),
                        tokenizer_ratio: profile.tokenizer_ratio.unwrap_or(1.0),
                    },
                );
            }
        }

        Self { models: table }
    }

    /// Fill in fallback pricing rows for known models without overriding config.
    ///
    /// Rows come from the shared registry
    /// ([`roko_core::config::model_registry::BUILTIN_PRICING`]) so this table,
    /// the TUI, and the enrichment estimator all price known slugs
    /// (including codex / gpt-5.x) from the same source.
    #[must_use]
    pub fn with_defaults(mut self) -> Self {
        for (slug, pricing) in roko_core::config::model_registry::BUILTIN_PRICING {
            self.models
                .entry((*slug).to_string())
                .or_insert(ModelPricing {
                    input_per_m: pricing.input_per_m,
                    output_per_m: pricing.output_per_m,
                    cache_read_per_m: pricing.cache_read_per_m,
                    cache_write_per_m: pricing.cache_write_per_m,
                    tokenizer_ratio: pricing.tokenizer_ratio,
                });
        }

        self
    }

    /// P3-33: Refresh pricing from an updated config without restart.
    ///
    /// Reads model profiles from the given config and updates any pricing
    /// that has changed. Returns the number of models updated.
    pub fn refresh_from_config(&mut self, config: &roko_core::config::schema::RokoConfig) -> usize {
        let mut updated = 0;
        for (slug, profile) in &config.models {
            let input = profile.cost_input_per_m.unwrap_or(0.0);
            let pricing = ModelPricing {
                input_per_m: input,
                output_per_m: profile.cost_output_per_m.unwrap_or(0.0),
                cache_read_per_m: profile
                    .cost_cache_read_per_m
                    .unwrap_or(input * DEFAULT_CACHE_READ_MULTIPLIER),
                cache_write_per_m: profile
                    .cost_cache_write_per_m
                    .unwrap_or(input * DEFAULT_CACHE_WRITE_MULTIPLIER),
                tokenizer_ratio: profile.tokenizer_ratio.unwrap_or(1.0),
            };
            let entry = self.models.entry(slug.clone());
            match entry {
                std::collections::hash_map::Entry::Occupied(mut occ) => {
                    if *occ.get() != pricing {
                        occ.insert(pricing);
                        updated += 1;
                    }
                }
                std::collections::hash_map::Entry::Vacant(vac) => {
                    vac.insert(pricing);
                    updated += 1;
                }
            }
        }
        if updated > 0 {
            tracing::info!(updated, "P3-33: cost table refreshed from config");
        }
        updated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glm_5_1_profile() -> ModelProfile {
        ModelProfile {
            provider: "zai".into(),
            slug: "glm-5.1".into(),
            context_window: 200_000,
            max_output: Some(131_072),
            supports_tools: true,
            supports_thinking: true,
            supports_vision: false,
            supports_web_search: false,
            supports_mcp_tools: false,
            supports_partial: false,
            provider_routing: None,
            tool_format: "openai_json".into(),
            cost_input_per_m: Some(1.40),
            cost_output_per_m: Some(4.40),
            cost_cache_read_per_m: Some(0.26),
            cost_cache_write_per_m: Some(1.75),
            max_tools: None,
            tokenizer_ratio: Some(1.05),
            ..Default::default()
        }
    }

    #[test]
    fn cost_table_calculate() {
        let mut models = HashMap::new();
        models.insert(
            "glm-5.1".into(),
            ModelPricing {
                input_per_m: 1.40,
                output_per_m: 4.40,
                cache_read_per_m: 0.26,
                cache_write_per_m: 1.75,
                tokenizer_ratio: 1.05,
            },
        );

        let table = CostTable { models };
        let usage = Usage {
            input_tokens: 1_000,
            output_tokens: 200,
            cache_read_tokens: 100,
            cache_create_tokens: 50,
            ..Usage::default()
        };

        let cost = table.calculate("glm-5.1", &usage);
        assert!((cost - 0.002_393_5).abs() < 1e-12);
    }

    #[test]
    fn token_normalization_uses_tokenizer_ratio() {
        let mut models = HashMap::new();
        models.insert(
            "glm-5.1".into(),
            ModelPricing {
                input_per_m: 1.40,
                output_per_m: 4.40,
                cache_read_per_m: 0.26,
                cache_write_per_m: 1.75,
                tokenizer_ratio: 1.05,
            },
        );

        let table = CostTable { models };
        assert_eq!(table.normalize_tokens("glm-5.1", 1_000), 1_050);
    }

    #[test]
    fn blended_cost_uses_tokenizer_ratio() {
        let mut models = HashMap::new();
        models.insert(
            "glm-5.1".into(),
            ModelPricing {
                input_per_m: 1.40,
                output_per_m: 4.40,
                cache_read_per_m: 0.26,
                cache_write_per_m: 1.75,
                tokenizer_ratio: 1.05,
            },
        );

        let table = CostTable { models };
        let blended = table.blended_cost_per_m("glm-5.1");
        let expected = ((3.0 * 1.40 + 4.40) / 4.0) * 1.05;
        assert!((blended - expected).abs() < 1e-12);
    }

    #[test]
    fn from_config_loads_pricing_rows() {
        let mut profiles = indexmap::IndexMap::new();
        profiles.insert("glm-5.1".into(), glm_5_1_profile());

        let table = CostTable::from_config(&profiles);
        let pricing = table.models.get("glm-5.1").expect("pricing row");

        assert!((pricing.input_per_m - 1.40).abs() < 1e-12);
        assert!((pricing.output_per_m - 4.40).abs() < 1e-12);
        assert!((pricing.cache_read_per_m - 0.26).abs() < 1e-12);
        assert!((pricing.cache_write_per_m - 1.75).abs() < 1e-12);
        assert!((pricing.tokenizer_ratio - 1.05).abs() < 1e-12);
    }

    #[test]
    fn cost_defaults() {
        let mut models = HashMap::new();
        models.insert(
            "custom-model".into(),
            ModelPricing {
                input_per_m: 9.99,
                output_per_m: 8.88,
                cache_read_per_m: 7.77,
                cache_write_per_m: 6.66,
                tokenizer_ratio: 1.23,
            },
        );

        let table = CostTable { models }.with_defaults();

        assert!(table.models.len() >= 9);

        let claude_opus = table
            .models
            .get("claude-opus-4-6")
            .expect("claude-opus-4-6");
        assert!((claude_opus.input_per_m - 5.00).abs() < 1e-12);
        assert!((claude_opus.output_per_m - 25.00).abs() < 1e-12);
        assert!((claude_opus.cache_read_per_m - 0.50).abs() < 1e-12);
        assert!((claude_opus.cache_write_per_m - 6.25).abs() < 1e-12);
        assert!((claude_opus.tokenizer_ratio - 1.0).abs() < 1e-12);

        let custom = table.models.get("custom-model").expect("custom-model");
        assert!((custom.input_per_m - 9.99).abs() < 1e-12);
        assert!((custom.output_per_m - 8.88).abs() < 1e-12);
        assert!((custom.cache_read_per_m - 7.77).abs() < 1e-12);
        assert!((custom.cache_write_per_m - 6.66).abs() < 1e-12);
        assert!((custom.tokenizer_ratio - 1.23).abs() < 1e-12);
    }

    #[test]
    fn lookup_prefix_match_with_date_suffix() {
        let table = CostTable {
            models: HashMap::new(),
        }
        .with_defaults();
        assert!(table.lookup("claude-sonnet-4-6").is_some());
        let pricing = table
            .lookup("claude-sonnet-4-6-20250514")
            .expect("prefix match");
        assert!((pricing.input_per_m - 3.00).abs() < 1e-12);
    }

    #[test]
    fn lookup_no_partial_word_match() {
        let mut models = HashMap::new();
        models.insert(
            "glm-5.1".into(),
            ModelPricing {
                input_per_m: 1.0,
                output_per_m: 1.0,
                cache_read_per_m: 0.5,
                cache_write_per_m: 0.5,
                tokenizer_ratio: 1.0,
            },
        );
        let table = CostTable { models };
        assert!(table.lookup("glm-5.1-20260101").is_some());
        assert!(table.lookup("glm-5.1x").is_none());
        // A suffix that names another model is no snapshot (bug-1f81ab).
        assert!(table.lookup("glm-5.1-air").is_none());
    }

    /// gap-ad0d39: an unknown model is unpriced, not priced at Sonnet's
    /// rates: `price` says so, and `calculate` records the unknown `0.0`.
    #[test]
    fn an_unknown_model_is_unpriced_rather_than_priced_as_sonnet() {
        let table = CostTable {
            models: HashMap::new(),
        };
        let zero = Usage::default();
        assert_eq!(table.price("unknown-model", &zero), Some(0.0));
        let usage = Usage {
            input_tokens: 1_000_000,
            output_tokens: 0,
            ..Usage::default()
        };
        assert_eq!(table.price("unknown-model", &usage), None);
        assert!(table.calculate("unknown-model", &usage).abs() < 1e-12);
    }

    /// bug-3de629: a cached Opus 4.6 token costs a tenth of an input token;
    /// the built-in row used to charge a quarter.
    #[test]
    fn cache_read_rates_price_cached_opus_tokens_at_a_tenth_of_input() {
        let table = CostTable {
            models: HashMap::new(),
        }
        .with_defaults();
        let usage = Usage {
            cache_read_tokens: 1_000_000,
            ..Usage::default()
        };
        let cost = table.calculate("claude-opus-4-6", &usage);
        assert!((cost - 0.50).abs() < 1e-12, "1M cached tokens cost {cost}");
    }

    /// bug-3de629: cache reads and writes as multiples of the input rate, as
    /// each provider's price page gave them on 2026-10-01 (the pages are
    /// cited beside `BUILTIN_PRICING`). A built-in row is pinned here or
    /// listed in `UNVERIFIED_PRICING`.
    #[test]
    fn cache_read_rates_match_each_providers_price_page() {
        // (slug, cache read / input, cache write / input)
        let multipliers = [
            // Anthropic: reads 0.1x, 5-minute writes 1.25x.
            ("claude-opus-4-6", 0.1, 1.25),
            ("claude-sonnet-4-6", 0.1, 1.25),
            ("claude-haiku-4-5", 0.1, 1.25),
            // Z.AI: $0.26 cached against $1.40, and $0.20 against $1.00.
            ("glm-5.1", 0.26 / 1.40, 1.0),
            ("glm-5", 0.2, 1.0),
            // OpenAI: half for gpt-4o, gpt-4o-mini and o3-mini, a quarter
            // for o3 and o4-mini, a tenth for gpt-5.x; only gpt-5.6-sol
            // charges for a write.
            ("gpt-4o", 0.5, 1.0),
            ("gpt-4o-mini", 0.5, 1.0),
            ("o3", 0.25, 1.0),
            ("o3-mini", 0.5, 1.0),
            ("o4-mini", 0.25, 1.0),
            ("gpt-5.2", 0.1, 1.0),
            ("gpt-5.4", 0.1, 1.0),
            ("gpt-5.4-mini", 0.1, 1.0),
            ("gpt-5.5", 0.1, 1.0),
            ("gpt-5.6-sol", 0.1, 1.25),
            // Perplexity: Sonar caches at $0.0625 against $1; Sonar Pro
            // and Sonar Reasoning Pro have no cache price.
            ("sonar", 0.0625, 1.0),
            ("sonar-pro", 1.0, 1.0),
            ("sonar-reasoning-pro", 1.0, 1.0),
            // Gemini 2.5: context caching at a tenth of input.
            ("gemini-2.5-pro", 0.1, 1.0),
            ("gemini-2.5-flash", 0.1, 1.0),
            ("gemini-2.5-flash-lite", 0.1, 1.0),
        ];
        let table = CostTable {
            models: HashMap::new(),
        }
        .with_defaults();
        for &(slug, read, write) in &multipliers {
            let pricing = table.lookup(slug).expect("a built-in row");
            assert!(
                (pricing.cache_read_per_m - read * pricing.input_per_m).abs() < 1e-9,
                "{slug}: cache read {} against input {}",
                pricing.cache_read_per_m,
                pricing.input_per_m
            );
            assert!(
                (pricing.cache_write_per_m - write * pricing.input_per_m).abs() < 1e-9,
                "{slug}: cache write {} against input {}",
                pricing.cache_write_per_m,
                pricing.input_per_m
            );
        }
        for (slug, _) in roko_core::config::model_registry::BUILTIN_PRICING {
            assert!(
                multipliers.iter().any(|(pinned, _, _)| pinned == slug)
                    || roko_core::config::model_registry::UNVERIFIED_PRICING.contains(slug),
                "{slug}: pin its cache multipliers here or list it as unverified"
            );
        }
    }

    /// bug-0c0747: the provider catalog lists no price for a model the
    /// shared registry prices, so `roko config providers add` leaves its
    /// rates to the registry, and lists one for every other model.
    #[test]
    fn price_tables_agree_provider_catalog_with_builtin_pricing() {
        for entry in roko_core::provider_catalog::catalog() {
            for model in entry.models {
                let in_registry = roko_core::config::model_registry::BUILTIN_PRICING
                    .iter()
                    .any(|(slug, _)| *slug == model.slug);
                assert_eq!(
                    model.cost_input_per_m.is_none(),
                    in_registry,
                    "{}",
                    model.slug
                );
                assert_eq!(
                    model.cost_output_per_m.is_none(),
                    in_registry,
                    "{}",
                    model.slug
                );
            }
        }
    }

    /// bug-0c0747: a Codex turn's cost estimate prices its model at the
    /// shared registry's rates, and a model the registry does not know at
    /// gpt-5.6-sol's, the Codex CLI's default.
    #[test]
    fn price_tables_agree_codex_turn_estimate_with_builtin_pricing() {
        use roko_agent::AgentRuntimeEvent;
        use roko_agent::provider::codex_cli::stream::parse_stream_line_with_model;
        use roko_core::config::model_registry::builtin_pricing;

        let line = r#"{"type":"turn.completed","usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":100}}"#;
        for (model, priced_as) in [
            (Some("gpt-5.6-sol"), "gpt-5.6-sol"),
            (Some("gpt-5.4-mini"), "gpt-5.4-mini"),
            (Some("codex-mini"), "codex-mini"),
            (Some("gpt-9-future"), "gpt-5.6-sol"),
            (None, "gpt-5.6-sol"),
        ] {
            let cost = parse_stream_line_with_model(line, model)
                .into_iter()
                .find_map(|event| match event {
                    AgentRuntimeEvent::TurnCompleted { total_cost_usd, .. } => total_cost_usd,
                    _ => None,
                })
                .expect("a turn cost");
            let pricing = builtin_pricing(priced_as).expect("a registry row");
            let expected = (600.0 * pricing.input_per_m
                + 400.0 * pricing.cache_read_per_m
                + 100.0 * pricing.output_per_m)
                / 1e6;
            assert!(
                (cost - expected).abs() < 1e-12,
                "{model:?}: {cost} against {expected}"
            );
        }
    }

    /// bug-0c0747: a configured model with no cache prices gets the same
    /// cache rates on every cost path: this table, the task runner's table
    /// and a usage's own cost fill.
    #[test]
    fn price_tables_agree_on_default_cache_prices() {
        let mut profiles = IndexMap::new();
        profiles.insert(
            "custom".to_string(),
            ModelProfile {
                slug: "custom-model".into(),
                cost_input_per_m: Some(2.0),
                cost_output_per_m: Some(8.0),
                ..Default::default()
            },
        );
        let read = 2.0 * DEFAULT_CACHE_READ_MULTIPLIER;
        let write = 2.0 * DEFAULT_CACHE_WRITE_MULTIPLIER;

        let learn = CostTable::from_config(&profiles);
        let learn = &learn.models["custom-model"];
        assert!((learn.cache_read_per_m - read).abs() < 1e-12, "{learn:?}");
        assert!((learn.cache_write_per_m - write).abs() < 1e-12, "{learn:?}");

        let agent = roko_agent::CostTable::from_config_with_defaults(&profiles);
        let agent = &agent.models["custom-model"];
        assert!((agent.cache_read_per_m - read).abs() < 1e-12, "{agent:?}");
        assert!((agent.cache_write_per_m - write).abs() < 1e-12, "{agent:?}");

        // What a usage priced with no cache prices pays for a million tokens.
        let filled = |cache_read_tokens, cache_create_tokens| {
            let mut usage = Usage {
                cache_read_tokens,
                cache_create_tokens,
                ..Usage::default()
            };
            usage.fill_cost_from_pricing(Some(2.0), Some(8.0), None, None);
            f64::from(usage.cost_usd)
        };
        assert!((filled(1_000_000, 0) - read).abs() < 1e-6);
        assert!((filled(0, 1_000_000) - write).abs() < 1e-6);
    }

    #[test]
    fn codex_and_gpt5x_price_from_registry_not_sonnet_fallback() {
        let table = CostTable {
            models: HashMap::new(),
        }
        .with_defaults();

        // Codex slugs resolve to their own rows (gpt-5.6-sol $4/$20,
        // codex-mini $2/$8), not the $3/$15 sonnet fallback they silently
        // got before.
        let usage = Usage {
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            ..Usage::default()
        };
        let cost = table.calculate("gpt-5.6-sol", &usage);
        assert!((cost - 24.00).abs() < 1e-12, "gpt-5.6-sol cost {cost}");
        let cost = table.calculate("codex-mini", &usage);
        assert!((cost - 10.00).abs() < 1e-12, "codex-mini cost {cost}");

        // Date-/variant-suffixed codex slugs still prefix-match.
        let pricing = table.lookup("gpt-5.6-sol-2026").expect("prefix match");
        assert!((pricing.input_per_m - 4.00).abs() < 1e-12);

        // Sonar rows exist now too.
        let pricing = table.lookup("sonar").expect("sonar pricing");
        assert!((pricing.input_per_m - 1.00).abs() < 1e-12);

        // Truly unknown models are unpriced, not priced at Sonnet's rates
        // (gap-ad0d39).
        assert_eq!(table.price("totally-unknown-llm", &usage), None);
        let cost = table.calculate("totally-unknown-llm", &usage);
        assert!(cost.abs() < 1e-12, "unknown cost {cost}");
    }
}
