+++
id = "gap-a5ebd4"
kind = "gap"
title = "GET /api/models serves only [models.*] prices, so a model roko prices from its snapshot or built-in table (the top rung claude-sonnet-4-6) shows none, and no model shows a cache-read rate"
status = "open"
triage = "unverified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-10-17
updated = 2026-10-17
source = "team-lead, round v6 (spec-v3 §10 P1, P3: the Receipt's top-rung comparison)"
anchors = ["crates/roko-serve/src/routes/providers.rs::list_models", "crates/roko-core/src/config/model_registry.rs::model_has_price"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-8548e3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn list_models_serves_the_rates_calls_are_priced_at' crates/roko-serve/src && cargo test -p roko-serve --lib list_models_serves_the_rates_calls_are_priced_at"
+++

## Problem

`GET /api/models` (roko serve, `crates/roko-serve/src/routes/providers.rs::list_models`) serves only the cost fields of
each model's `[models.*]` entry. A model with no prices there comes back with none, even though roko prices its calls
from the price snapshot or its built-in table, and no model shows a cache-read rate. On the pocket-synth showcase the
top rung, claude-sonnet-4-6, has no `[models.*]` prices, so the Hermes plugin's Receipt cannot compute "≈ the same work
on the top rung" (spec-v3 §10 P3), and its Ladder shows no price for that rung (P1).

## Why it matters

The Receipt's comparison with the top rung is part of the pitch: cheap models first, and here is what they saved.
The page must show the prices roko actually charges calls at.

## Where

- `crates/roko-serve/src/routes/providers.rs::list_models` and `ModelInfo`.
- Dispatch prices a call in `crates/roko-cli/src/dispatch_v2.rs::fill_usage_cost_from_pricing`: the snapshot's row for
  the slug, then the profile's input and output prices (with its provider's cache-read rate,
  `ModelProfile::with_provider_cache_rate`), then `roko_core::config::model_registry::builtin_pricing`.
- `crates/roko-core/src/config/model_registry.rs::model_has_price` asks the same question as a yes or no.

## Plan

1. `model_registry::effective_rates(snapshot, profile, slug)`: the input, output and cache-read rates a run prices
   the model at, and which source (`snapshot`, `profile`, `builtin`) supplied them, in dispatch's order.
2. `list_models` serves them: `cost_input_per_m` and `cost_output_per_m` become those rates, plus
   `cost_cache_read_per_m`, `priced_by`, and `price_snapshot` (the snapshot id) when the snapshot supplied them.

## Done when

`list_models_serves_the_rates_calls_are_priced_at` shows a snapshot-priced model (its row beating `[models.*]`
prices), a profile-priced one, a built-in-priced one (claude-sonnet-4-6) and an unpriced one; the `[[verify]]`
command passes; plugin-runs has the shape.
