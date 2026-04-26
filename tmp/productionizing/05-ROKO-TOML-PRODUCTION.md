# Production `roko.toml` — Only Real Providers

## The Problem

Stock `roko.toml` lists many `[providers.*]` entries and `[models.*]` profiles. Routing (`[routing]`, cascade, roles) can reference **backend slugs** (the `slug = "..."` field inside each `[models.*]` block). If the model’s provider has no API key, dispatch fails late with a confusing error.

## Schema version (important)

This repo uses **`config_version = 1`** and **`schema_version = 2`**:

- Providers are **`[providers.<id>]`** tables with a `kind` field (not the legacy **`[[providers]]`** array).
- Models are **`[models.<alias>]`** tables with `provider = "<id>"` and `slug = "<backend-model-name>"`.

Any guide that shows `[[providers]]` / `[[models]]` is **out of date** for this tree. Copy shapes from the **root `roko.toml`** and delete what you do not need.

## Workaround (no code changes)

1. **Delete** entire `[providers.…]` sections for providers you do not use.
2. **Delete** every `[models.…]` whose `provider = "…"` points at a removed provider.
3. Set **`[agent].default_model`** to the **`slug`** of a model you kept (same string as that model’s `slug =` field), and **`default_backend`** to the provider id if your branch still uses it.
4. Update **`[routing]`** so `fast_task_model`, `standard_task_model`, and `complex_task_model` are **backend slugs** that still exist on remaining `[models.*]` rows.

After stripping, search for dangling references:

```bash
rg 'provider = "(moonshot|zhipu|cerebras|openrouter)"' roko.toml
rg 'fast_task_model|standard_task_model|complex_task_model' -n roko.toml
```

## Example: keeping one API provider (shape only)

Below is a **structural** example — preserve full flag fields from stock `roko.toml` when you copy, so you do not drop required serde fields.

```toml
# Illustration only — copy real blocks from stock roko.toml and trim.

[providers.openai]
kind = "openai_compat"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
timeout_ms = 120000
ttft_timeout_ms = 15000
connect_timeout_ms = 5000

[models.gpt54-mini]
provider = "openai"
slug = "gpt-5.4-mini"
context_window = 128000
supports_tools = true
supports_thinking = false
supports_vision = true
supports_web_search = false
supports_mcp_tools = false
supports_partial = false
supports_grounding = false
supports_code_execution = false
supports_caching = false
tool_format = "openai_json"
supports_search = false
supports_citations = false
supports_async = false
is_embedding_model = false

[agent]
default_model = "gpt-5.4-mini"  # must match `slug` above
default_backend = "openai"
```

**Anthropic in stock config** is often `kind = "claude_cli"` (local `claude` binary). For **headless Docker**, you typically need an **HTTP-style** provider configuration consistent with how your image runs; do not copy `claude_cli` blindly unless the CLI is installed in the container.

## Routing section after a strip

```toml
[routing]
mode = "auto_override"
algorithm = "linucb"
discount_factor = 0.99
fast_task_model = "<slug-you-kept>"
standard_task_model = "<slug-you-kept>"
complex_task_model = "<slug-you-kept>"
context_strategy = "mcp_first"
```

Every value above must match some **`slug =`** line on a retained `[models.*]` block.

## Providers you are likely to remove when keys are absent

Delete matching **both** `[providers.X]` and all `[models.*]` with `provider = "X"`:

- `moonshot`, `zhipu`, `cerebras`, `openrouter`, `zai`, etc., unless you set their `*_API_KEY` env vars.

## Validating

```bash
roko config check-secrets
# When health subcommand exists on your branch:
roko config providers health 2>/dev/null || true
roko run "echo ok" --model <your-default-slug>
```

## See also

- **02-MODEL-ROUTING-FIX.md** — code-level gating (preferred long term).
- **07-ANTI-PATTERNS.md** §1 — do not add providers “for completeness.”
