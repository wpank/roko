+++
id = "bug-12153c"
kind = "bug"
title = "The config loader silently drops agent.fallback_model, agent.tier_models, serve.port and project.default_domain from roko.toml"
status = "open"
triage = "unverified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-planner's report on gap-853b31)"
anchors = ["crates/roko-core/src/config/loader.rs::build_schema_tree", "crates/roko-core/src/config/loader.rs::strip_unknown_fields", "crates/roko-core/src/config/loader.rs::validate_known_config_paths"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["gap-7a3527", "gap-853b31", "bug-477ede"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn every_optional_config_key_survives_a_load' crates/roko-core/src/ && cargo test -p roko-core --lib every_optional_config_key_survives_a_load"
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
