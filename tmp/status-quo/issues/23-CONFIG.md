# Configuration Issues

## High

### `cold_storage.enabled` defaults to `true`
- `schema.rs:1593,1617`: Any project with 7-day-old engrams will have them silently migrated to `.roko/cold/` after first plan run, even if user never configured cold storage.

### `serve.auth.enabled = true` + empty keys = complete lockout
- `serve.rs:94-105`: Default has `enabled: true`, `api_key: String::new()`. Fresh `roko serve` without `roko init` → all API calls return 401. No startup validation error.

### `chain.wallet_key` is plaintext in roko.toml
- `chain.rs:28`: Hex-encoded private key stored directly. No `api_key_env` indirection, no `#[serde(skip_serializing)]`. Round-trips through `config show` and config endpoint (not redacted at `routes/config.rs:317`).

## Medium

### Fields parsed but never used at runtime
- `routing.mode` and `routing.context_strategy`: Only TUI metadata display. No runtime branching.
- `[graduation]` section: `GraduationCell::default()` always uses defaults. Config ignored.
- `PerplexityConfig.default_research_model/reasoning_model/embed_model`: Only `default_search_model` ever read.
- `GeminiConfig.use_free_tier`: Zero runtime consumers.
- `routing.discount_factor` and `routing.algorithm`: Never passed to CascadeRouter.

### Demo config uses non-existent field names (silently dropped)
- `max_cost_per_task`, `max_cost_per_session`: Not in `BudgetConfig`. Serde drops them silently.
- `[routing] fast = "claude-haiku"`: Actual fields are `fast_task_model`, etc. No `deny_unknown_fields`.

### No env var for `serve.auth.api_key`
- No `ROKO_API_KEY` or hierarchical `ROKO__SERVE__AUTH__API_KEY`. Container environments can't supply via env.

### `config_version` serde default diverges from `CURRENT_CONFIG_VERSION`
- `schema.rs:181`: Default function returns 1; current is 2. Spurious stale-version warning on every fresh project.

### Routing model defaults reference slugs not in `[models]`
- `routing.rs:154-164`: Defaults are `"claude-haiku-4-5"`, `"claude-sonnet-4-6"`, `"claude-opus-4-6"`. Without matching `[models]` entries → `ValidationWarning::UnknownModel` on every load.
