+++
id = "bug-12153c"
kind = "bug"
title = "The config loader silently drops agent.fallback_model, agent.tier_models, serve.port and project.default_domain from roko.toml"
status = "done"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "b11ca807d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-planner's report on gap-853b31)"
anchors = ["crates/roko-core/src/config/loader.rs::build_schema_tree", "crates/roko-core/src/config/loader.rs::strip_unknown_fields", "crates/roko-core/src/config/loader.rs::validate_known_config_paths"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["gap-7a3527", "gap-853b31", "bug-477ede"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn every_optional_config_key_survives_a_load' crates/roko-core/src/ && cargo test -p roko-core --lib every_optional_config_key_survives_a_load"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Loader schema tree gets sentinels for agent.fallback_model, serve.port, project.default_domain and a dynamic agent.tier_models section, so they survive load (f96efeeb9, merged b351d2be5). About 100 more stripped keys filed separately. Batch 6 gate (work/rust-batch-5 tree plus fmt-only and unused-import fixes fdb2a9b72, 579ffd0e6, a8dd7f09f, 2aa55ab1f): cargo check --workspace --tests clean; clippy -p roko-cli -p roko-core -p roko-agent -p roko-serve -p roko-learn -p roko-gateway --no-deps -D warnings clean; lib tests roko-cli 3087, roko-agent 2249, roko-core 1922, roko-learn 1177, roko-serve 955, roko-gate 685, roko-gateway 41, 0 failed; merged MAIN tree re-checked (cargo check --workspace --tests clean)."
+++

## Problem

Loading `roko.toml` parses it to a TOML value, removes every key that is not in `build_schema_tree()` (`strip_unknown_fields`), and only then deserializes. The schema tree is a serialized `RokoConfig::default()` plus sentinel values for fields a default serialization skips. Four fields have no sentinel, so they are missing from the tree:

- `agent.fallback_model`: `Option<String>`, and `None` is not serialized;
- `agent.tier_models`: a `HashMap` with `skip_serializing_if = "HashMap::is_empty"`;
- `serve.port`: `Option<u16>`;
- `project.default_domain`: `Option<TaskDomain>` with `skip_serializing_if`.

A user's `fallback_model = "…"`, `[agent.tier_models]`, `[serve] port = …` or `default_domain = "…"` is removed on load. The only trace is a `tracing::warn!` that calls the key unknown (`validate_known_config_paths` uses the same tree), and roko prints tracing on stderr only with `--verbose` or `ROKO_LOG`/`RUST_LOG`.

## Why it matters

Release blocker (epic spec-ae5f94): documented settings don't take effect, and nothing tells the user. `agent.fallback_model` has 28 readers (among them model fallback in roko-agent `model_call_service.rs`, `dispatch_v2.rs` and roko-cli `config.rs`), `agent.tier_models` 16 (among them `prd plan`'s escalation), and `serve.port` feeds `roko-serve/src/terminal.rs:177`. Every new optional key meets the same fate unless someone remembers to add a sentinel.

## Where

- `crates/roko-core/src/config/loader.rs`: `build_schema_tree` (:1431), `strip_unknown_fields` (:2046), the load path (:600-615) and `deserialize_migrated_toml` (:2020).
- The fields: `config/agent.rs` (:75, :78), `config/serve.rs` (:88), `config/project.rs` (:24).

## Current state

At BASE the sentinels cover `agent.command`, `args`, `timeout_ms`, `env`, `env_passthrough`, `data_llm`, `extensions`, `mcp_config`, `default_agent_id` and `disabled_providers`, the routing and gate lists, and GitHub's owner and repo, but not the four fields above. No test loads any of them from TOML text; the e2e tests that write `default_domain` are `#[ignore]`d.

## Plan

1. Add sentinels for the four fields in `build_schema_tree`.
2. Guard against the next one: add `every_optional_config_key_survives_a_load`. It loads a fixture `roko.toml` that sets every optional and skip-if-empty field (these four and the existing sentinel fields) and checks each value on the loaded config. A structural alternative is to derive the schema tree from the types (for example with `schemars`) instead of from a serialized default.
3. Consider printing the "unknown config key" diagnostic on stderr by default, so a stripped key is at least visible.

## Done when

- [ ] Each of the four keys set in `roko.toml` reaches the loaded config.
- [ ] The `[[verify]]` command passes.

## Notes

- `project.default_domain` has no runtime reader today, so dropping it has no effect yet. It still belongs in the test.
- gap-853b31's new `[authoring] planner_model` is a `String` with a default, so it serializes and survives the strip.
- bug-477ede (plan escalation) reads `agent.tier_models`, which this bug removes on load.
- Premise confirmed at `407ce30d5` with `target/debug/roko` (built at `33e107da1`; `build_schema_tree` has not changed since): `roko config validate` calls all four keys unknown, and loading strips them.
- The four now have sentinels. `agent.tier_models` also became a dynamic map section (user-defined keys, string values), so `config set agent.tier_models.<tier>` is typed as well. `every_optional_config_key_survives_a_load` loads the four and the keys that already had sentinels. Plan step 3 (print the unknown-key diagnostic on stderr by default) is not done: while the gaps below are open it would fire on keys serde accepts, so it belongs with them.
- Same bug, not fixed here (for a follow-up item, probably p1): validating a `roko.toml` that sets every field reachable from `RokoConfig` (generated from the struct definitions) reports about 100 more keys missing from the tree, so loading strips them too. A few may be artifacts of the generator. Among them: `[agent.roles.<name>]` overrides (`model`, `effort`, `tools`, ...; the template is `RoleOverride::default()`), `[profiles.<name>]` fields, `[providers.<name>.extra_headers]` entries and `limits.*`, `models.<name>.tier`, `use_max_completion_tokens` and `provider_routing.*`, `serve.auth.api_keys`, `jwks_providers` and `privy_*`, `serve.deploy.webhooks`, `server.auth_token`, `serve.event_ingest_allowlist`, `serve.tracing.otlp_endpoint`, `timeouts.*`, `conductor.watchers.*`, `routing.weights.*`, `retrieval.role_token_budgets.*`, `dreams.scheduled_cron`, `gates.domain_gates` and `max_rung`, `runner.max_concurrent_*`, `scheduler.cron`, `resources.per_plan_disk_budget_mb`, `agent.defaults.*_model`, `agent.data_llm.output_schema`, `learning.override_learning_dampening`, and the optional keys of `deploy`, `relay`, `chain`, `gemini` and `perplexity`. A sentinel list will keep drifting; a guard test can read the fields each `deny_unknown_fields` section accepts from serde's unknown-field error and fail when the tree lacks one.
- Implemented on `work/bug-12153c` at `f96efeeb9`; cargo verification deferred to the batch check.
