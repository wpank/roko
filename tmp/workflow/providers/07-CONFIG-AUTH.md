# 07 — Configuration & Auth Detection

## The Problem

Roko has a comprehensive config system (`~/.roko/config.toml` + `roko.toml`) with
providers, models, and agent defaults, but the chat/dispatch paths bypass it entirely
and use env-var-based auth detection instead.

---

## Auth Detection Chain
**File**: `crates/roko-cli/src/auth_detect.rs`

```
1. ZAI_API_KEY set?
   → OpenAiCompat { key, base="https://open.bigmodel.cn/api/paas/v4", model=ZAI_MODEL }

2. ANTHROPIC_API_KEY set?
   → AnthropicApi { key, model=None }

3. OPENAI_API_KEY set?
   → OpenAiCompat { key, base=OPENAI_API_BASE|OPENAI_BASE_URL|"https://api.openai.com/v1", model=None }

4. `claude --version` succeeds?
   → ClaudeCli

5. Nothing found
   → NeedsSetup
```

### What Auth Detection Ignores

- `~/.roko/config.toml` `[agent].default_backend` — never read
- `~/.roko/config.toml` `[agent].default_model` — never read
- `~/.roko/config.toml` `[providers.*]` — all provider definitions ignored
- `~/.roko/config.toml` `[models.*]` — all model definitions ignored
- `roko.toml` `[agent]` section — ignored
- API key env vars from provider definitions (`api_key_env` field) — not consulted

The user has `default_backend = "zai"` and `default_model = "glm-5.1"` in their
global config, but auth detection still checks ANTHROPIC_API_KEY first (if set)
and would use that instead.

---

## Config Merge Chain
**File**: `crates/roko-cli/src/config.rs`

```
1. Load project roko.toml (if exists)
2. Load ~/.roko/config.toml (global)
3. merge_global_providers():
   - Merge providers from global into project (project wins on conflict)
   - Merge models from global into project
   - If project.agent.default_model empty → use global.agent.default_model
   - If project.agent.default_backend empty → use global.agent.default_backend
4. config.apply_process_env() — env var overrides
```

This merge works correctly but the result is only used by:
- `run.rs dispatch_agent()` Path 1 (routing config path)
- `orchestrate.rs` (plan execution)

Not used by:
- `dispatch_direct.rs` (chat dispatch) — uses AuthMethod instead
- `chat_inline.rs` (inline chat) — uses AuthMethod
- `unified.rs` (unified entry point) — loads config but passes AuthMethod to chat

---

## The Disconnect

```
User's config says:
  [agent]
  default_model = "glm-5.1"
  default_backend = "zai"
  [providers.zai]
  kind = "openai_compat"
  base_url = "https://api.z.ai/api/paas/v4"
  api_key_env = "ZAI_API_KEY"

But when user runs `roko chat`:
  1. unified.rs loads config (correctly)
  2. unified.rs calls detect_auth() (ignores config)
  3. detect_auth() finds ZAI_API_KEY set → OpenAiCompat
  4. Auth method hardcodes base_url from env detection, not from config
  5. Model defaults to whatever the dispatch path hardcodes

Result: Config is loaded, merged, and thrown away.
```

---

## Two Config Systems

### roko-core: RokoConfig
**File**: `crates/roko-core/src/config/schema.rs`
- Full schema with providers, models, tools, budget, prompt
- Used by orchestrate.rs and agent dispatch
- Not used by CLI entry points

### roko-cli: Config
**File**: `crates/roko-cli/src/config.rs`
- CLI-specific config with layered loading
- Has `ConfigLayer`, `load_layered`, source tracking
- Separate struct from `RokoConfig` — different fields, different defaults

Two config models, neither is the single authoritative runtime config.

---

## What Needs to Change

### 1. Auth detection should consult config
`detect_auth()` should accept the loaded config and use `[agent].default_backend`
to determine the provider, then look up the provider's `api_key_env` to find the key.

### 2. dispatch_direct should use provider config
Instead of hardcoding API URLs and model names, `dispatch_prompt()` should accept
a resolved provider config from the loaded roko.toml.

### 3. One config resolution path
All entry points (run, chat, serve, plan) should go through the same config
resolution that produces a `ResolvedRuntimeConfig` with the final provider,
model, and token limits.

### 4. Config actually used for dispatch
The merged config's providers and models should be the source of truth for
model selection, API URLs, token limits, and timeouts — not env vars and
hardcoded strings.
