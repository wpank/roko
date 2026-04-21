# 22 - Config Schema Redesign

Scope: `roko.toml`, `crates/roko-cli/src/config.rs`, `crates/roko-core/src/config/schema.rs`, `crates/roko-agent/src/provider/mod.rs`, `crates/roko-agent/src/model_call_service.rs`, `crates/roko-cli/src/model_selection.rs`

Many of the provider, safety, and runner regressions trace back to config being both a schema and a migration shim. The code accepts old fields, synthesizes new providers, guesses defaults from commands and environment, and then individual surfaces add their own interpretation. This makes bad config easy to run and hard to diagnose.

## Findings

### CRITICAL: this repo still ships dangerous runner permissions in root config

`roko.toml:1019-1021` sets `[runner] dangerously_skip_permissions = true`. Earlier audits called this out as a direct safety regression. The redesign problem is broader: dangerous local overrides are ordinary config values with no scoped justification, expiry, or environment guard.

Expected design: dangerous flags should require an explicit local-only override file, reason, and possibly an environment acknowledgment. They should not live in the shared root config.

### HIGH: CLI config and core config are overlapping schemas

`crates/roko-cli/src/config.rs:20-79` defines a top-level `Config` with agent, providers, models, runner, learning, serve, and more. `crates/roko-core/src/config/schema.rs:46-119` defines `RokoConfig` with its own agent, providers, models, gates, routing, pipeline, learning, serve, runner, and agents.

The overlap is not just duplication; surfaces copy fields between these types. `unified.rs:263-274` manually maps CLI config into `RokoConfig` for one-shot chat. That is how provider semantics drift between commands.

Expected design: use one versioned config domain model. CLI can have a thin input layer, but every command should resolve into the same validated `RokoConfig`/runtime config object.

### HIGH: version migration warns but still runs old semantics

`schema.rs:38-42` defines current schema/config versions. `schema.rs:172-185` warns when `config_version == 1`, but still accepts the config. The root `roko.toml:1-2` is `config_version = 1` and `schema_version = 2` while also using provider/model tables.

Expected design: config loading should migrate to the current version or fail with actionable diagnostics. Warnings are not enough when config drives provider dispatch and safety policy.

### HIGH: provider identity is mixed with command inference

`roko.toml:40-45` names `[providers.anthropic]` but sets `kind = "claude_cli"`. Separately, `provider/mod.rs:164-210` synthesizes provider config from `agent.command`, and `provider/mod.rs:434-447` maps executable names like `claude`, `codex`, and `cursor-agent` to provider kinds.

This makes provider names, provider kinds, command names, and API families interchangeable in practice. The ACP regression where `ProviderKind::ClaudeCli` was treated like Anthropic API is a symptom of this blurry boundary.

Expected design: provider id, provider kind, transport, command, API base URL, and auth method should be separate validated fields. A provider named `anthropic` should not be a Claude CLI provider unless that is an explicit, validated alias with clear display text.

### MEDIUM: effective providers/models are synthesized from environment during runtime

`schema.rs:220-283` builds default providers from agent command and `ANTHROPIC_API_KEY`/`ANTHROPIC_BASE_URL`. `schema.rs:286-309` synthesizes model profiles from tier/default models. `model_call_service.rs:321-380` also inserts providers/models based on `openai_base_url`, model prefix, and env availability.

Synthesis is sometimes useful, but doing it in multiple layers makes config provenance opaque.

Expected design: config resolution should produce a `ResolvedConfig` with provenance for every provider/model: file, default, migration, env, command inference. Runtime execution should not mutate it.

### MEDIUM: alias/default fields make model ownership ambiguous

`roko.toml:12-15` includes both `default_model = "claude-sonnet"` and `default_backend = "anthropic"`. `config.rs:1619` accepts both `agent.model` and `agent.default_model`. `model_selection.rs` then layers CLI override, provider override, task model, role config, cascade router, project default, and built-in default.

Those are legitimate precedence levels, but they need a typed precedence contract and diagnostics. Today some paths print a reason string while others re-resolve or override later.

Expected design: centralize precedence in a config/model resolver that returns both the selected value and a machine-readable trace. No caller should manually copy or reinterpret model fields.

## Redesign Direction

1. Collapse CLI/core config into one validated domain model with a thin parse layer for legacy compatibility.
2. Require migration from config version 1 to the current version before runtime dispatch.
3. Make provider transport/auth explicit and separate from provider names and executable names.
4. Resolve all synthetic providers/models once at config load with provenance.
5. Move dangerous safety/permission flags into local scoped overrides with explicit acknowledgement.
6. Add config diagnostics that fail on ambiguous provider names, missing default model/provider links, and deprecated aliases used by production commands.
