+++
id = "gap-a5ebd4"
kind = "gap"
title = "GET /api/models serves only [models.*] prices, so a model roko prices from its snapshot or built-in table (the top rung claude-sonnet-4-6) shows none, and no model shows a cache-read rate"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "79fbd4cb1"
source = "team-lead, round v6 (spec-v3 §10 P1, P3: the Receipt's top-rung comparison)"
anchors = ["crates/roko-serve/src/routes/providers.rs::list_models", "crates/roko-core/src/config/model_registry.rs::model_has_price"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-8548e3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn list_models_serves_the_rates_calls_are_priced_at' crates/roko-serve/src && cargo test -p roko-serve --lib list_models_serves_the_rates_calls_are_priced_at"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T12:27:35Z"
commit = "79fbd4cb1"
forced = false
evidence = "79fbd4cb1: roko_core::config::model_registry::effective_rates returns a model's input, output and cache-read rates as a run prices its calls, in dispatch's order (the price snapshot's row, then the profile's prices with its provider's cache-read rate, then builtin_pricing), with the source; GET /api/models serves them as cost_input_per_m, cost_output_per_m and cost_cache_read_per_m, plus priced_by (snapshot, profile or builtin) and price_snapshot (the snapshot id) when the snapshot priced the model, and no prices for a model nothing prices. list_models_serves_the_rates_calls_are_priced_at passes: gpt-oss-120b from the snapshot over its [models.*] prices, a profile-priced model, claude-sonnet-4-6 from the built-in table (3.0, 15.0, cache read 0.3), an unpriced model; the older list_models test (glm-5.1, profile prices) still passes. Checks at 225c30b61 (79fbd4cb1 plus bug-9b65dc, on main 37088dc12): fmt --check, clippy -D warnings (roko-core, roko-cli, roko-serve), lib tests roko-core 2038 passed, roko-serve 1053 passed, roko-cli 3774 passed; the [[verify]] passes."
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
