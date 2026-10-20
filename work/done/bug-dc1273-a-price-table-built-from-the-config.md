+++
id = "bug-dc1273"
kind = "bug"
title = "A price table built from the config counts a Cerebras model's cache reads at a tenth of its input rate (ModelCallService's default since bug-4e5731)"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/model_call_service"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "6fdeca256"
source = "roko-caps, 2026-10-16, on bug-4e5731's path (find-c3da60's provider cache rule)"
anchors = ["crates/roko-agent/src/task_runner.rs::CostTable", "crates/roko-agent/src/model_call_service.rs::with_config", "crates/roko-core/src/config/provider.rs::with_provider_cache_rate"]
links = { depends_on = [], blocks = [], related = ["bug-4e5731", "find-c3da60"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_service_prices_cerebras_cache_reads_at_the_input_rate' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_service_prices_cerebras_cache_reads_at_the_input_rate"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T13:45:28Z"
commit = "ac853c7b8"
by = "roko-planner"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-16T12:36:39Z"
model = "claude-opus-5-5"
forced = false
evidence = "ac853c7b8: RokoConfig::billed_models maps each effective [models.*] profile through ModelProfile::with_provider_cache_rate with its effective provider (find-c3da60: Cerebras, by kind or by its URL behind openai_compat, bills cached input at the input rate), and every price table built from the config uses it: ModelCallService::with_config's default, dispatch_v2's and serve_runtime's service tables, serve's service factory (x2), serve's gateway table, roko chat's table and roko config models route. Checked: [[verify]] a_service_prices_cerebras_cache_reads_at_the_input_rate passes on the merged tree 6fdeca256 (main f7dbaba72 merged in), with billed_models_price_cerebras_cache_reads_at_the_input_rate (roko-core); lib suites roko-agent 2356, roko-cli 3720, roko-core 2035, roko-serve 1040 pass; clippy -D warnings; fmt; integration failures only bug-2a31bc, bug-c63cca, bug-ee2752. No live run (no paid call needed). Reported by roko-caps."
+++

## Problem

A price table built from the config counts a Cerebras model's cache reads at a tenth of its input rate, while
Cerebras bills cached input at the full input rate.

`CostTable::from_config_with_defaults(&config.models)` prices each `[models.*]` profile with its own rates, and
a profile that names no `cost_cache_read_per_m` gets `DEFAULT_CACHE_READ_MULTIPLIER` (0.1) × its input rate. It
sees only the profiles, not their providers, so the provider rule that find-c3da60 added,
`ModelProfile::with_provider_cache_rate(&ProviderConfig)` (a provider that `bills_cached_input_at_input_rate`, i.e.
Cerebras by kind or by its base URL, prices cache reads at the input rate), never applies there.

Since bug-4e5731 (7a68db4c5), `ModelCallService::with_config` builds its table this way whenever the caller sets
none, so the services that pass no table now price a Cerebras model's cache reads low: the episode distiller,
`roko chat`, ACP's dispatch and distiller, serve's provider test route and one gateway path. The callers that pass
their own table built the same way (`dispatch_v2.rs`, `serve_runtime.rs`, `service_factory.rs`, `gateway.rs`)
have the same gap in that table. Reported by roko-caps (2026-10-16).

## Why it matters

Cache reads are most of a long agent call's input on Cerebras, so the cost rows, the plan and daily ceilings and a
host's spend undercount real money (goal `truth`).

## Where

- `crates/roko-agent/src/task_runner.rs::CostTable`: `from_config_with_defaults` takes only the profiles.
- `crates/roko-agent/src/model_call_service.rs::with_config`: the default table since bug-4e5731.
- `crates/roko-core/src/config/provider.rs::with_provider_cache_rate`: the rule to apply.

## Plan

1. Give `CostTable` a constructor from the whole config that maps each profile through
   `with_provider_cache_rate(config.providers.get(&profile.provider))` before pricing it (or a second argument to
   `from_config_with_defaults`).
2. Use it in `ModelCallService::with_config`, and move the other callers to it (coordinate with whoever holds
   `dispatch_v2.rs`).
3. Test: a service whose config has a Cerebras-kind provider and a profile with input and output rates but no
   cache-read rate prices cache reads at the input rate.

## Done when

`[[verify]]` passes: a service built from such a config prices a Cerebras model's cache reads at its input rate.
