# 39 -- Config Schema Phantom Fields Audit

**Date**: 2026-05-01
**Scope**: `crates/roko-core/src/config/`, `roko.toml`, `crates/roko-serve/src/routes/config.rs`
**Method**: Static analysis of struct definitions against grep for runtime read sites

---

## 1. Phantom Config Sections (Defined but Never Consumed at Runtime)

### FINDING 1.1 -- `[oneirography]` is fully phantom

**Severity: MEDIUM**

The `OneirographyConfig` struct is defined in `crates/roko-core/src/config/tools.rs:124-135`
and included as `pub oneirography: OneirographyConfig` on `RokoConfig` (schema.rs:110).
It is present in `roko.toml` (lines 1002-1007) with all fields set.

Zero runtime reads exist. Grepping `config.oneirography`, `cfg.oneirography`, and
`.oneirography.` across all `.rs` files returns **no matches**. A separate
`OneirographyConfig` struct exists in `crates/roko-dreams/src/phase2/oneirography.rs:335`
-- this is a distinct type that is never connected to the config version.

The section is serialized/deserialized but no code path ever reads its values.

**Files**:
- `crates/roko-core/src/config/tools.rs:124-135` (struct definition)
- `crates/roko-core/src/config/schema.rs:110` (field on RokoConfig)

### FINDING 1.2 -- `[demurrage]` is phantom at the roko-core config level

**Severity: LOW**

`DemurrageConfig` in `crates/roko-core/src/config/learning.rs:116-145` is defined with
8 fields (rate_per_hour, min_balance, freeze_threshold, thaw_balance, max_balance,
gc_interval_secs, kind_rate_multipliers, freeze_before_delete, death_threshold).

Runtime reads of `config.demurrage` across all of `crates/` return only:
- `crates/roko-core/src/config/hot_reload.rs:153-156` -- diffing old vs new for reload detection
- `crates/roko-core/src/config/schema.rs:770-781` -- example TOML generation

No runtime code reads any field from `config.demurrage` to actually apply demurrage
decay. The `roko-chain` crate has its own `demurrage_rate` field on `KoraiConfig` that is
independent of this config section.

**Impact**: Users can set `[demurrage]` values in roko.toml but they have zero effect.

### FINDING 1.3 -- `[attention]` is phantom

**Severity: LOW**

`AttentionConfig` in `crates/roko-core/src/config/learning.rs:194-207` has 4 fields.
The only reads are in `schema.rs` example TOML generation (lines 789-800). No runtime
code reads `config.attention.max_tokens_per_layer` or any other field.

### FINDING 1.4 -- `[immune]` is phantom

**Severity: LOW**

`ImmuneConfig` in `crates/roko-core/src/config/learning.rs:239-252` has 4 fields.
Only referenced in `schema.rs` example generation (lines 809-813). No runtime reads.

### FINDING 1.5 -- `[temporal]` is phantom

**Severity: LOW**

`TemporalConfig` in `crates/roko-core/src/config/learning.rs:284-294` has 3 fields.
Only referenced in `schema.rs` example generation (lines 818-823). No runtime reads.

### FINDING 1.6 -- `[goals]` section -- config struct is phantom

**Severity: LOW**

`GoalsConfig` in `crates/roko-core/src/config/learning.rs:320-334` has 4 fields.
Only referenced in `schema.rs` example generation (lines 829-836). The serve and
hot_reload code references `doc.goals` but that is a different field (a `Vec<String>` on
a config document struct, not the `GoalsConfig` struct).

### FINDING 1.7 -- `[energy]` is phantom

**Severity: LOW**

`EnergyConfig` in `crates/roko-core/src/config/budget.rs:53-66` has 4 fields
(pool_usd, per_task_cap_usd, tier_caps, metabolism_rate). Only referenced in `schema.rs`
example generation (lines 841-843). No runtime reads.

---

## 2. Phantom Fields Within Used Sections

### FINDING 2.1 -- `[conductor]` has 6+ fields never read by orchestration

**Severity: MEDIUM**

`ConductorConfig` (schema.rs:1051-1079) has 12 fields. The following are **never read**
by orchestrate.rs, event_loop.rs, or the runner:

| Field | Read by runtime? | Read by anything? |
|---|---|---|
| `auto_advance_batch` | No | TUI config_meta.rs (display only) |
| `auto_merge_on_complete` | No | TUI config_meta.rs (display only) |
| `pre_plan` | No | example TOML only |
| `conductor_model` | No | compat.rs migration only |
| `warm_implementers_per_plan` | No | example TOML only |
| `enabled_roles` | No | TUI config_meta.rs, compat.rs |

These fields exist in roko.toml (lines 862-879) and users can configure them, but
the runtime ignores them entirely. The fields that *are* read: `max_agents`,
`max_parallel_plans`, `parallel_enabled`, `express_mode`, `max_auto_fix_attempts`,
`auto_fix_model`, `watchers`.

### FINDING 2.2 -- `agent.data_llm` is defined but never wired at dispatch

**Severity: MEDIUM**

`DataLlmConfig` in `crates/roko-core/src/config/agent.rs:219-272` is a well-documented
CaMeL dual-LLM isolation config. The `DataLlmRouter` exists in
`crates/roko-agent/src/safety/data_llm.rs` with full logic. However:

- `crates/roko-cli/src/orchestrate.rs` never reads `.agent.data_llm`
- No code path constructs a `DataLlmRouter` from the config
- The orchestrate.rs TODO at line 4481 explicitly says:
  `// TODO(M-future): Load extensions from config.agent.extensions and`

### FINDING 2.3 -- `agent.policy_manifests` is never loaded

**Severity: LOW**

Defined at `crates/roko-core/src/config/agent.rs:70`, documented as
"RoleProfile/PromptPolicy manifest paths loaded before agent dispatch."
No runtime code ever opens or reads these manifest files. Only referenced in
schema.rs example TOML as a comment.

### FINDING 2.4 -- `agent.domain` is never read

**Severity: LOW**

Defined at `crates/roko-core/src/config/agent.rs:92`. No runtime code reads
`config.agent.domain` or `roko_config.agent.domain`. The field exists only in the
struct definition and Default impl.

---

## 3. Config Validation

### FINDING 3.1 -- `validate_strict_config_toml()` is never called in production

**Severity: HIGH**

`validate_strict_config_toml()` at `crates/roko-core/src/config/validation.rs:62-85`
checks whether `runner.dangerously_skip_permissions = true` appears in shared config.
This is a safety-critical check.

However, the function is:
- Exported from `crates/roko-core/src/config/mod.rs:46`
- Called **only** in `validation.rs` unit tests (lines 176, 190, 196, 205, 211)
- **Never called** from `load_config()` or any production code path

The production config load path at `crates/roko-core/src/config/mod.rs:97-118` is:
1. Read file as string
2. `toml::from_str()` -- pure deserialization, no validation
3. `interpolate_env_vars()` -- secret substitution
4. `resolve_file_secrets()` -- file-based secrets
5. Return

There is no schema validation, no field range checking, no cross-reference validation.
A user can set `runner.dangerously_skip_permissions = true` in `roko.toml` and it will be
silently accepted. The validation function that was built to prevent this is dead code.

### FINDING 3.2 -- No semantic validation on config load

**Severity: MEDIUM**

Beyond the strict check, there is no validation at all:
- No check that referenced model slugs exist in `[models.*]`
- No check that referenced provider names exist in `[providers.*]`
- No range validation (e.g., `budget.max_plan_usd > 0`)
- No check for obviously wrong context_window values
- No duplicate detection for model definitions with the same slug

---

## 4. Duplicate and Conflicting Model Definitions in roko.toml

### FINDING 4.1 -- Duplicate model slugs with conflicting context_window values

**Severity: HIGH**

The following model aliases in `roko.toml` map to the same backend slug but with
different `context_window` values:

| Alias | Slug | context_window | Line |
|---|---|---|---|
| `gemini-2-5-pro` | `gemini-2.5-pro` | **1048576** | 336 |
| `gemini-pro` | `gemini-2.5-pro` | **128000** | 393 |

| `kimi-k25` | `kimi-k2.5` | **128000** | 108 |
| `kimi-k2-5` | `kimi-k2.5` | **262144** | 545 |

| `kimi-k2-6` | `kimi-k2.6` | **262144** | 127 |
| `kimi-k26` | `kimi-k2.6` | **128000** | 678 |

| `sonnet` | `claude-sonnet-4-6` | **128000** | 203 |
| `claude-sonnet` | `claude-sonnet-4-6` | **200000** | 583 |

| `opus` | `claude-opus-4-6` | **128000** | 621 |
| `claude-opus` | `claude-opus-4-6` | **200000** | 697 |

| `sonar` | `sonar-pro` | **128000** | 469 |
| `sonar-pro` | `sonar-pro` | **127000** | 659 |

**Impact**: Which alias is used determines the context_window the runtime sees. This
causes inconsistent prompt truncation behavior depending on whether code references
`"sonnet"` or `"claude-sonnet"`, for instance. The Anthropic API context windows should
be 200000, so the `sonnet` and `opus` aliases are **wrong**.

### FINDING 4.2 -- `[providers.anthropic]` uses `kind = "claude_cli"`

**Severity: MEDIUM**

At roko.toml line 40-45:
```toml
[providers.anthropic]
kind = "claude_cli"
```

A provider named "anthropic" with kind `claude_cli` is confusing. The `effective_providers()`
synthesis in schema.rs (line 207-228) auto-creates an "anthropic" provider with kind
`AnthropicApi` when `ANTHROPIC_API_KEY` is set. Having an explicit "anthropic" entry as
`claude_cli` may shadow or conflict with the auto-synthesis depending on load order.

### FINDING 4.3 -- `[providers.zhipu]` and `[providers.zai]` share `ZAI_API_KEY`

**Severity: LOW**

`providers.zhipu` (line 84) and `providers.zai` (line 55) both use
`api_key_env = "ZAI_API_KEY"`. This is likely intentional (same parent company) but
could cause confusion. Models referencing provider "zhipu" (e.g., `glm4`, `glm51`) will
fail if `ZAI_API_KEY` is not set, with no hint that "zhipu" API keys are sourced from
`ZAI_API_KEY`.

---

## 5. config_version

### FINDING 5.1 -- config_version is 1 in the live roko.toml despite providers existing

**Severity: MEDIUM**

`roko.toml` line 1: `config_version = 1`

But `config_version = 1` is the "legacy" format (no `[providers]` section). The file
has a full `[providers.*]` section. The runtime behavior when `config_version == 1`:

1. `RokoConfig::from_toml()` (schema.rs:176-183) logs a warning via `tracing::warn!`:
   `"roko.toml uses config version 1 (no [providers] section)"`
2. `legacy_layout_warning()` (config_cmd.rs:1088) returns a hint to run `roko config migrate`

But `config_version` does **not** change any runtime behavior. There is no migration
logic that transforms the config based on version. The warning fires even though
providers are fully configured.

### FINDING 5.2 -- schema_version is checked but never enforced

**Severity: LOW**

`RokoConfig::is_stale()` (schema.rs:199-201) returns true when
`schema_version < CURRENT_SCHEMA_VERSION`. This method is available but a grep shows
it is only called from `config_cmd.rs` display output, never as a gate or enforcement
mechanism.

---

## 6. Secret Handling

### FINDING 6.1 -- `/api/config/toml` exposes ALL secrets including private keys

**Severity: CRITICAL**

The `GET /api/config/toml` endpoint at `crates/roko-serve/src/routes/config.rs:48-58`
serializes the full `RokoConfig` to TOML and returns it as `text/toml`:

```rust
async fn get_config_toml(...) -> ... {
    let cfg = state.load_roko_config();
    let toml_str = toml::to_string_pretty(cfg.as_ref())
        .map_err(|e| ApiError::internal(format!("serialize toml: {e}")))?;
    Ok(([(axum::http::header::CONTENT_TYPE, "application/toml")], toml_str))
}
```

**No masking is applied.** The raw config includes:
- `chain.wallet_key` -- a literal Ethereum private key (roko.toml line 1012:
  `wallet_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"`)
- `webhooks.github.secret` -- webhook HMAC secret
- `serve.auth.api_key` -- API authentication key
- `server.auth_token` -- server auth token
- `deploy.railway_api_token` -- deployment credentials

The JSON endpoint (`GET /api/config`) applies `mask_secret_fields()`, but the TOML
endpoint bypasses masking entirely.

### FINDING 6.2 -- `mask_secret_fields()` has incomplete coverage

**Severity: HIGH**

`mask_secret_fields()` at `crates/roko-serve/src/routes/config.rs:245-259` only masks
3 fields:
1. `serve.auth.api_key`
2. `server.auth_token`
3. `deploy.railway_api_token`

**Not masked** even on the JSON endpoint:
- `chain.wallet_key` -- Ethereum private key
- `webhooks.github.secret` -- webhook signing secret
- `serve.auth.privy_app_id` -- Privy app ID (less sensitive but still auth material)
- Any `api_key_env` values that leaked into the config (though these are env var *names*,
  not values, so lower risk)

### FINDING 6.3 -- Hardcoded Anvil private key in roko.toml

**Severity: LOW** (dev-only key, but documenting for completeness)

`roko.toml` line 1012 contains the well-known Anvil/Hardhat test private key
`0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80`. This is a
standard development key with no real funds. However, this file is committed to the repo,
and the pattern of storing private keys in config files is dangerous if a user copies this
pattern with a real key.

---

## 7. GAPS.md Review

`/Users/will/dev/nunchi/roko/roko/.roko/GAPS.md` was reviewed. **No config-related gaps
are documented.** The file covers dispatch, runtime_feedback, projection, and resume
cycle wiring gaps, but does not mention:
- Phantom config sections
- Missing config validation
- Secret exposure in API endpoints
- Duplicate model definitions

---

## Summary Table

| # | Finding | Severity | Category |
|---|---|---|---|
| 6.1 | `/api/config/toml` exposes all secrets unmasked | **CRITICAL** | Secret handling |
| 3.1 | `validate_strict_config_toml()` never called in production | **HIGH** | Validation |
| 4.1 | 6 duplicate model slugs with conflicting context_windows | **HIGH** | Model definitions |
| 6.2 | `mask_secret_fields()` missing chain.wallet_key, webhooks.github.secret | **HIGH** | Secret handling |
| 1.1 | `[oneirography]` section fully phantom | **MEDIUM** | Phantom section |
| 2.1 | 6 conductor fields never read by runtime | **MEDIUM** | Phantom fields |
| 2.2 | `agent.data_llm` config never wired to dispatch | **MEDIUM** | Phantom fields |
| 3.2 | No semantic validation on config load | **MEDIUM** | Validation |
| 4.2 | `providers.anthropic` uses `kind = "claude_cli"` (confusing) | **MEDIUM** | Model definitions |
| 5.1 | config_version=1 in roko.toml despite providers existing | **MEDIUM** | Versioning |
| 1.2 | `[demurrage]` section phantom at roko-core level | **LOW** | Phantom section |
| 1.3 | `[attention]` section phantom | **LOW** | Phantom section |
| 1.4 | `[immune]` section phantom | **LOW** | Phantom section |
| 1.5 | `[temporal]` section phantom | **LOW** | Phantom section |
| 1.6 | `[goals]` config struct phantom | **LOW** | Phantom section |
| 1.7 | `[energy]` section phantom | **LOW** | Phantom section |
| 2.3 | `agent.policy_manifests` never loaded | **LOW** | Phantom fields |
| 2.4 | `agent.domain` never read | **LOW** | Phantom fields |
| 4.3 | zhipu/zai providers share ZAI_API_KEY | **LOW** | Model definitions |
| 5.2 | schema_version checked but never enforced | **LOW** | Versioning |
| 6.3 | Hardcoded Anvil private key in roko.toml | **LOW** | Secret handling |

---

## Recommended Actions

### Immediate (CRITICAL/HIGH)

1. **Fix `/api/config/toml` secret exposure**: Either apply masking to the TOML output
   (regex-replace secret fields) or remove the endpoint entirely. The endpoint comment
   says it's for "Builder workspaces to copy the live config" -- this should use a
   separate mechanism that excludes secrets.

2. **Extend `mask_secret_fields()`**: Add masking for `chain.wallet_key` and
   `webhooks.github.secret` on the JSON endpoint.

3. **Wire `validate_strict_config_toml()`**: Call it from `load_config()` with the
   appropriate source classification.

4. **Deduplicate model definitions**: Consolidate to canonical aliases with correct
   context_window values. Anthropic models should use 200000.

### Short-term (MEDIUM)

5. **Remove or document phantom sections**: Either wire demurrage/attention/immune/
   temporal/goals/energy to runtime code, or remove them from the config schema and
   roko.toml to reduce confusion. If kept for future use, add a comment marking them
   as reserved.

6. **Wire or remove phantom conductor fields**: auto_advance_batch,
   auto_merge_on_complete, pre_plan, conductor_model, warm_implementers_per_plan,
   enabled_roles should either influence orchestration behavior or be removed.

7. **Update config_version** to 2 in roko.toml to suppress the stale-version warning.

### Long-term (LOW)

8. Add semantic config validation (model/provider cross-references, range checks).
9. Wire `agent.data_llm` to dispatch for CaMeL dual-LLM isolation.
10. Consider moving private keys to environment variables rather than config files.
