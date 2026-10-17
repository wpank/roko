+++
id = "bug-1c0a66"
kind = "bug"
title = "/api/models served no cache-read rate for a model its profile prices, though dispatch charges 0.1x input, and served a zero-priced profile that dispatch passes over for built-in pricing"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "plugin-runs, round v6 (the Hermes plugin's cost check of 7a5dd6046's /api/models)"
discovered_from = "gap-a5ebd4"
anchors = ["crates/roko-core/src/config/model_registry.rs::effective_rates", "crates/roko-serve/src/routes/providers.rs::list_models"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-a5ebd4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn effective_rates_price_calls_as_dispatch_does' crates/roko-cli/src && cargo test -p roko-cli --lib effective_rates_price_calls_as_dispatch_does"
+++

## Problem

`GET /api/models` serves the rates roko prices each model's calls at (gap-a5ebd4, `effective_rates`). For a model
priced by its `[models.*]` profile it differs from dispatch in two ways:

- **No cache-read rate.** When neither the profile nor its provider names one, `/api/models` serves no
  `cost_cache_read_per_m`, and `EffectiveRates`' doc says such a read "costs as input". Dispatch charges it at
  `DEFAULT_CACHE_READ_MULTIPLIER` (0.1) times the input rate (`Usage::fill_cost_from_pricing`, bug-0c0747).
  plugin-runs found this while pricing calls in the Hermes plugin. The plugin follows dispatch.
- **A profile that charges nothing.** Dispatch moves on to roko's built-in pricing when a profile's prices come to
  $0 (`dispatch_v2::fill_usage_cost_from_pricing`). Only a model with no built-in price is free (backlog 2109).
  `/api/models` serves the profile's zeros, with `priced_by = "profile"`.

## Why it matters

A host that prices calls from `/api/models` gets a different cost than the run's own accounting, so the two can't
be compared.

## Where

- `crates/roko-core/src/config/model_registry.rs::effective_rates` and `EffectiveRates`.
- `crates/roko-serve/src/routes/providers.rs::list_models`.
- Dispatch, which these follow: `crates/roko-cli/src/dispatch_v2.rs::fill_usage_cost_from_pricing` and
  `crates/roko-core/src/chat_types.rs::fill_cost_from_pricing`.

## Plan

1. When the profile and its provider name no cache-read rate, `effective_rates` uses input times
   `DEFAULT_CACHE_READ_MULTIPLIER`. Every model with a price then has a cache-read rate, so `cache_read_per_m` is no
   longer optional.
2. A profile whose rates are all zero gives way to the built-in pricing when the model has one.
3. Fix the doc comments.

## Done when

`effective_rates_price_calls_as_dispatch_does` passes. For each source, the rates price a call with cache reads at
what `fill_usage_cost_from_pricing` charges it:
- a snapshot row;
- a profile with and without a cache-read rate;
- a zero-priced profile, with a built-in price and without one;
- built-in pricing.

`list_models_serves_the_rates_calls_are_priced_at` serves the profile's model 0.1 times its input rate.
