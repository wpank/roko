# 15 — Config And Safety Regression

Scope: `roko.toml`, `crates/roko-cli/src/config.rs`, `crates/roko-core/src/agent.rs`

This pass reviewed configuration changes against the runner safety rules. The most serious issue is that the current root `roko.toml` reintroduces an explicit dangerous permission bypass while earlier audit work already identified permissive defaults as a release blocker.

## Findings

### CRITICAL: Root config sets `dangerously_skip_permissions = true`

`roko.toml:1019-1021` adds:

```toml
[runner]
plan_timeout_secs = 3600
dangerously_skip_permissions = true
```

This directly violates the post-parity safety rule: `dangerously_skip_permissions` defaults to false and must require explicit user opt-in. Because this is the repository root config, local demos, runner scripts, and development commands can inherit the bypass as if it were normal project policy.

Expected design: the root config should either omit the field or set it false. Any bypass should be a local, uncommitted override or a command-line flag with an explicit warning path.

### HIGH: Config contains modern-looking model IDs without a source of truth

The current `roko.toml` contains model IDs such as `claude-sonnet-4-6` (`roko.toml:13`, `roko.toml:821`), `gpt-5.4` (`roko.toml:220-222`), and `gpt-5.4-mini` (`roko.toml:258-260`). This audit did not verify those against provider metadata. More importantly, the codebase still treats the static TOML table as truth.

That is fragile for model selection. If a model slug is wrong, deprecated, unavailable to the account, or provider-specific, the resolver can still select it and send requests that fail late. Runner work appears to have expanded the static table rather than designing provider metadata discovery, validation, or a tested alias policy.

Expected design: separate stable local aliases from provider backend slugs. Validate slugs through provider metadata or a health check path, and make stale aliases fail with actionable diagnostics.

### HIGH: Provider merge fix is local but not backed by schema-level invariants

`config.rs:1351-1388` changes provider merging so kind-specific fields are not inherited when provider `kind` changes. This is a good symptom fix, but it still relies on merge behavior to preserve validity after the fact.

Provider config needs schema-level invariants: a `claude_cli` provider should not carry API-only fields; an `openai_compat` provider should have a base URL and an API key policy; a provider with `api_key_env = ""` should be explicitly local/no-auth and not treated as missing or empty auth.

Expected design: parse into typed provider variants, validate each variant, then merge typed values. Avoid `Option<String>` bags where invalid combinations can survive until dispatch.

### MEDIUM: `default_model` alias was patched into the parser but schema naming is still split

`config.rs:1596-1600` treats `agent.default_model` as `agent.model`, and `config.rs:2291-2312` adds a serde alias. This makes the current `roko.toml:13` parse, but it also cements two names for the same field.

Expected design: choose the v2 schema field name, migrate old configs through a migration step, and warn on deprecated names. Silent aliases hide config drift.

### MEDIUM: Backend classification relies on string prefixes and short aliases

`agent.rs:125-132` classifies `sonnet`, `opus`, and `haiku` as Claude by matching literal short names. That is a convenience patch, not a model registry. It does not prove that the short name exists in `roko.toml`, maps to a configured provider, or has a valid backend slug.

Expected design: backend classification should come from resolved model metadata, not independent slug-prefix heuristics.

## Root Cause

Configuration has become a dumping ground for runner convenience: add aliases, add model IDs, reorder providers, add bypass flags. The core problem is the absence of typed provider/model config and validation. Without that, every command can patch around a different symptom.

## Fix Direction

1. Remove or set `roko.toml:1021` to false before any production-like run.
2. Add config validation that rejects dangerous permission bypass in checked-in/default configs.
3. Replace provider `Option` bags with typed provider variants or equivalent validation.
4. Establish one model alias registry with provider-validated backend slugs.
5. Migrate `default_model`/`model` naming instead of silently accepting both forever.
