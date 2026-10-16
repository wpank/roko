+++
id = "find-c3da60"
kind = "finding"
title = "A model priced without a cache rate counts cache reads at a tenth of input, and the tool loop's profile pricing pre-empts the price snapshot"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["roko-core/pricing", "roko-agent/tool-loop"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v5/findings/roko-caps.md (roko-caps, 2026-10-16)"
discovered_from = "bug-4fabd0: checking the cache-read pricing of the $0.753 overrun"
anchors = ["crates/roko-core/src/chat_types.rs::fill_cost_from_pricing", "crates/roko-cli/src/dispatch_v2.rs::fill_usage_cost_from_pricing"]
links = { depends_on = [], blocks = [], related = ["bug-4fabd0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cerebras_cache_reads_cost_the_input_rate' crates/roko-cli/src && cargo test -p roko-cli --lib cerebras_cache_reads_cost_the_input_rate"
+++

## Problem

Two pricing paths can count a cache-heavy call at a fraction of what the provider bills, which lets spend past a
run's cap:

1. A `[models.*]` profile with `cost_input_per_m` and `cost_output_per_m` but no `cost_cache_read_per_m` prices
   cache reads at 0.1x the input rate (`roko_core::Usage::fill_cost_from_pricing`,
   `DEFAULT_CACHE_READ_MULTIPLIER = 0.1`). That is OpenAI's and Anthropic's discount. Cerebras bills cached input at
   the full input rate ("Input tokens, whether served from the cache or processed fresh, are billed at the standard
   input token rate", https://inference-docs.cerebras.ai/capabilities/prompt-caching); Z.ai charges 0.11 of 0.60,
   Moonshot 0.16 of 0.95.
2. roko's own tool loop prices each turn from the model profile when the profile has rates
   (`ToolLoop::run_inner`, `with_model_profile`), and dispatch then treats the summed cost as reported, so the dated
   price snapshot (`dispatch_v2::fill_usage_cost_from_pricing`, "snapshot first") is never consulted for those
   models. For the Cerebras adapter, which sets no profile on its loop, the snapshot applies.

## Why it matters

For a Cerebras model priced in roko.toml without a cache rate (and no snapshot row), a loop that re-sends its
context each turn is counted at about a tenth of its cost, so a cap lets about ten times its value through. Not hit
today: `roko init` writes `cost_cache_read_per_m` for its Cerebras, Z.ai, Moonshot and OpenAI models, and the
snapshot rows match the providers (checked 2026-10-16). Its OpenRouter rows have no cache rate.

## Where

- `crates/roko-core/src/chat_types.rs::fill_cost_from_pricing` and `crates/roko-core/src/config/model_registry.rs`
  (`DEFAULT_CACHE_READ_MULTIPLIER`).
- `crates/roko-agent/src/tool_loop/mod.rs::run_inner` (per-turn profile pricing).
- `crates/roko-cli/src/dispatch_v2.rs::fill_usage_cost_from_pricing` (snapshot, profile, registry).

## Plan

Decide: (a) price a turn with no cache rate at the input rate (never undercount a money control), or per provider
(Cerebras at 1.0); (b) let the snapshot win over the profile inside the tool loop too, as dispatch documents; (c)
have `roko doctor` warn about an API model priced without a cache rate.

## Notes

Found by roko-caps (bug-4fabd0) while checking the $0.753 overrun's pricing: that run was priced right.

## Decision (roko-caps, 2026-10-16)

- (a), per provider: the 0.1 default stays, since it is right for OpenAI, Anthropic and Gemini and every cost path
  shares it (bug-0c0747). A provider that bills cached input at its input rate (Cerebras, by kind or by its API's
  address, as `roko init` writes it) gives a profile that names no cache-read rate its input rate
  (`ProviderConfig::bills_cached_input_at_input_rate`, `ModelProfile::with_provider_cache_rate`), in roko's tool loop
  (openai_compat adapter) and in dispatch's pricing.
- (b): dispatch prices every call through `dispatch_v2::price_target_usage`. For an agent roko's tool loop runs, the
  cost a call carries is the loop's estimate from the profile, so the dated snapshot's row prices it instead when
  there is one (decision 2113); a CLI agent's own reported cost stands.
- (c) not done: with (a) the known undercount is gone; a warning for every profile without a cache rate would flag
  `roko init`'s OpenRouter rows, whose discount OpenRouter passes through from the serving provider.
