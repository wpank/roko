+++
id = "bug-647249"
kind = "bug"
title = "About 100 more roko.toml keys are missing from the loader's schema tree, so loading strips them"
status = "open"
triage = "unverified"
severity = "p1"
goal = "release"
size = "M"
subsystem = ["roko-core/config"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-onboard's report on bug-12153c, branch work/bug-12153c)"
anchors = ["crates/roko-core/src/config/loader.rs::build_schema_tree", "crates/roko-core/src/config/loader.rs::strip_unknown_fields", "crates/roko-core/src/config/serve.rs", "crates/roko-core/src/config/provider.rs", "crates/roko-core/src/config/agent.rs"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = ["bug-12153c"], blocks = [], related = ["bug-12153c", "bug-9434c4", "gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn every_accepted_config_field_is_in_the_schema_tree' crates/roko-core/src/ && cargo test -p roko-core --lib every_accepted_config_field_is_in_the_schema_tree"
+++

## Problem

bug-12153c covers four keys that `strip_unknown_fields` removes from `roko.toml` on load, because `build_schema_tree` (a serialized `RokoConfig::default()` plus sentinels) lacks them. Its branch (`work/bug-12153c`, not merged at ad391f99a) adds those four sentinels and the test `every_optional_config_key_survives_a_load`.

While doing that, wk-onboard validated a `roko.toml` that sets every field reachable from `RokoConfig`, generated from the struct definitions. About 100 more keys are missing from the tree, so loading strips them too. A few may be artifacts of the generator. Among them:

- `[agent.roles.<name>]` overrides (`model`, `effort`, `tools`, …; the template is `RoleOverride::default()`);
- `[profiles.<name>]` fields;
- `[providers.<name>.extra_headers]` entries and `limits.*`;
- `models.<name>.tier`, `use_max_completion_tokens` and `provider_routing.*`;
- `serve.auth.api_keys`, `jwks_providers` and `privy_*`, `serve.deploy.webhooks`, `server.auth_token`, `serve.event_ingest_allowlist`, `serve.tracing.otlp_endpoint`;
- `timeouts.*`, `conductor.watchers.*`, `routing.weights.*`, `retrieval.role_token_budgets.*`, `dreams.scheduled_cron`;
- `gates.domain_gates` and `max_rung`, `runner.max_concurrent_*`, `scheduler.cron`, `resources.per_plan_disk_budget_mb`;
- `agent.defaults.*_model`, `agent.data_llm.output_schema`, `learning.override_learning_dampening`;
- the optional keys of `deploy`, `relay`, `chain`, `gemini` and `perplexity`.

Spot-checked at ad391f99a, these are skipped by a default serialization, so they are absent from the tree unless a sentinel adds them:

- `serve.auth.api_keys`, `jwks_providers` and `serve.deploy.webhooks` (`skip_serializing_if = "Vec::is_empty"`, serve.rs:210-223, :315);
- `providers.*.extra_headers` (`Option`, provider.rs:310);
- `use_max_completion_tokens` (`is_false`, provider.rs:672).

## Why it matters

Release blocker (epic spec-ae5f94), p1: documented settings silently do nothing, including security settings. A `serve.auth.api_keys` list or JWKS providers removed on load change how serve authenticates. The only trace is a `tracing::warn!` that needs `--verbose`.

## Where

`build_schema_tree` and `strip_unknown_fields` in `crates/roko-core/src/config/loader.rs`, and the config structs in `crates/roko-core/src/config/`.

## Current state

At ad391f99a no sentinel covers any of these keys. bug-12153c's branch covers its four keys only.

## Plan

1. Don't extend the sentinel list by hand; it will keep drifting. Build the guard wk-onboard suggests: for each `deny_unknown_fields` section, read the fields it accepts (for example from serde's unknown-field error, which lists the expected names), and fail when the tree lacks one. Call it `every_accepted_config_field_is_in_the_schema_tree`.
2. Fix what the guard finds. Dynamic maps (`agent.roles.<name>`, `profiles.<name>`, `extra_headers`) need a template child in the tree, as bug-12153c did for `agent.tier_models`.
3. Alternatively, derive the tree from the types (for example with `schemars`) instead of from a serialized default.

## Done when

- [ ] Every field `RokoConfig` accepts survives a load, and the guard keeps it that way.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on bug-12153c's branch, whose test and sentinels this extends.
