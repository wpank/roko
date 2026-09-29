# 21-config / 01 -- Schema Sections Reference

> Full section-by-section reference for all `RokoConfig` sections. The canonical
> source is `crates/roko-core/src/config/schema.rs`. All types derive
> `Serialize + Deserialize` and use `#[serde(default)]` so a bare config
> produces a fully-populated `RokoConfig`.

**Parent**: [21-CONFIG](../../21-CONFIG.md)

---

## Top-Level Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `config_version` | u32 | 1 (serde default; loader migrates to 2) | Layout version for migration tooling |
| `schema_version` | u32 | 2 | Semantic version for the parameter set |

---

## `[project]` -- ProjectConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | String | `"roko-project"` | Workspace name |
| `root` | String | `"."` | Workspace root path |
| `fresh_base_branch` | String | `"main"` | Base branch for worktree operations |
| `default_domain` | Option\<String\> | None | Default task domain |

---

## `[agent]` -- AgentConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default_model` | String | `"claude-sonnet-4-6"` | Default LLM model |
| `default_backend` | String | `"claude"` | Default provider backend |
| `default_effort` | String | `"medium"` | Task effort level |
| `context_limit_k` | u32 | `200` | Context window limit (K tokens) |
| `bare_mode` | bool | `true` | Replace CLI built-in prompt with Roko's canonical prompt |
| `fallback_model` | Option\<String\> | None | Fallback when primary unavailable |
| `extensions` | Vec\<String\> | `[]` | Default extension chain |
| `domain` | Option\<String\> | None | Default domain profile |
| `mode` | AgentMode | `Ephemeral` | `ephemeral` / `persistent` / `reactive` |

### `[agent.roles.<name>]` -- per-role overrides

```toml
[agent.roles.reviewer]
model = "claude-haiku-4-5"
effort = "low"
turn_budget_usd = 0.5
```

Override fields: `model`, `backend`, `effort`, `temperament`, `context_limit_k`,
`tools`, `budget`, `thresholds`, `routing_overrides`, `turn_budget_usd`.

### `[agent.data_llm]` -- DataLlmConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | String | `"claude-haiku-3-5"` | Model for data extraction |
| `max_tokens` | u64 | 4096 | Output token limit |
| `temperature` | f64 | 0.0 | Temperature (0 = deterministic) |
| `strip_tool_calls` | bool | true | Remove tool calls from output |
| `sanitize_input` | bool | true | Sanitize inputs before sending |

---

## `[runner]` -- CoreRunnerConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sandbox_level` | RunnerSandboxLevel | `"restrict"` | Live enforcement: `none`, `observe`, `restrict`, `isolate`, or `quarantine` |
| `dangerously_skip_permissions` | bool | false | Skip permission checks (rejected in strict/shared config) |

---

## `[providers.<name>]` -- ProviderConfig

```toml
[providers.anthropic]
kind = "anthropic_api"
api_key_env = "ANTHROPIC_API_KEY"
max_concurrent = 50
timeout_ms = 120000
ttft_timeout_ms = 30000
connect_timeout_ms = 5000
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `kind` | ProviderKind | *required* | Provider type (see below) |
| `base_url` | Option\<String\> | None | API base URL |
| `api_key_env` | Option\<String\> | None | Environment variable holding the API key |
| `command` | Option\<String\> | None | CLI binary name (for CLI providers) |
| `args` | Option\<Vec\<String\>\> | None | CLI arguments |
| `timeout_ms` | Option\<u64\> | None | Request timeout |
| `ttft_timeout_ms` | Option\<u64\> | 30000 | Time-to-first-token timeout |
| `connect_timeout_ms` | Option\<u64\> | None | Connection timeout |
| `extra_headers` | Option\<HashMap\> | None | Extra HTTP headers (supports `*_file` secrets) |
| `max_concurrent` | Option\<usize\> | None | Concurrency limit |
| `limits` | Option\<ProviderLimits\> | None | Rate limits and resource constraints |
| `require_confirmation` | bool | false | Require user confirmation before dispatch |

Provider kinds: `anthropic_api`, `claude_cli`, `codex_cli`, `openai_compat`,
`cursor_acp`, `cursor_cli`, `perplexity_api`, `gemini_api`, `gemini_cli`,
`cerebras_api`, `hermes`, `open_claw`, `ollama`.

---

## `[models.<name>]` -- ModelProfile

```toml
[models.sonnet]
provider = "anthropic"
slug = "claude-sonnet-4-6"
context_window = 200000
supports_tools = true
tool_format = "anthropic"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | String | *required* | Must reference a key in `[providers]` |
| `slug` | String | `""` | Provider-facing model identifier |
| `context_window` | u32 | 200000 | Context window size (tokens) |
| `supports_tools` | bool | false | Whether the model supports tool use |
| `tool_format` | String | `""` | Tool format: `anthropic`, `openai_json`, etc. |

---

## `[profiles.<name>]` -- DomainProfile

See [05-presets-and-profiles.md](05-presets-and-profiles.md) for inheritance
semantics and built-in profiles.

---

## `[gates]` -- GatesConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `clippy_enabled` | bool | false | Enable clippy gate |
| `skip_tests` | bool | false | Skip test gate |
| `max_iterations` | u32 | 3 | Global gate retry ceiling |

---

## `[routing]` -- RoutingConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fast_task_model` | String | MODEL_FAST | Model for fast/simple tasks |
| `standard_task_model` | String | MODEL_FOCUSED | Model for standard tasks |
| `complex_task_model` | String | MODEL_DEEP | Model for complex tasks |

---

## `[budget]` -- BudgetConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_plan_usd` | f64 | 0.0 | Plan spending ceiling (0 = unlimited) |
| `max_task_usd` | f64 | 0.0 | Per-task ceiling |
| `max_turn_usd` | f32 | 0.0 | Per-turn ceiling |
| `prompt_token_budget` | u32 | 8000 | Token budget for prompt composition |

---

## `[conductor]` -- ConductorConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_agents` | usize | 8 | Maximum concurrent agents |
| `max_parallel_plans` | usize | 2 | Maximum parallel plan executions |
| `plan_failure_policy` | `skip_failed` or `fail_fast` | `skip_failed` | When a plan task fails: skip only its dependants and run the rest, or start no further task. A plan's `[meta] failure_policy` overrides it |
| `parallel_enabled` | bool | false | Enable parallel execution |
| `express_mode` | bool | false | Skip non-essential steps |
| `max_auto_fix_attempts` | u32 | 3 | Auto-fix retry limit |
| `auto_fix_model` | String | MODEL_FOCUSED | Model for auto-fix attempts |

---

## `[learning]` -- LearningConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `auto_playbook_refresh` | bool | false | Refresh playbooks automatically |
| `knowledge_file_intel` | bool | false | File intelligence enrichment |
| `knowledge_warnings` | bool | false | Warning pattern enrichment |
| `knowledge_wave_context` | bool | false | Wave context enrichment |
| `knowledge_error_patterns` | bool | false | Error pattern enrichment |
| `replan_on_gate_failure` | bool | false | Trigger replan on gate failures |
| `replan_max_per_plan` | u32 | 1 | Maximum replans per plan |
| `dream_on_completion` | bool | false | Opt in to dream consolidation on plan completion; otherwise dreams run on demand via `roko knowledge dream run` |
| `use_lookahead_router` | bool | false | Enable lookahead routing |
| `gate_threshold_flush_interval` | u64 | 300 | Adaptive threshold flush cadence (seconds) |

---

## `[pipeline]` -- PipelineConfig

Four complexity bands with per-band retry overrides:

```toml
[pipeline.mechanical]
max_iterations = 1

[pipeline.focused]
max_iterations = 2

[pipeline.integrative]
max_iterations = 3

[pipeline.architectural]
max_iterations = 5
```

---

## `[serve]` -- ServeConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `port` | Option\<u16\> | None | Override port (falls back to `server.port`) |
| `auto_orchestrate` | bool | true | Auto-start orchestration on plan execution |

### `[serve.auth]` -- ServeAuthConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | false | Enable authentication middleware |
| `api_key` | String | `""` | Legacy single API key |
| `api_keys` | Vec\<ApiKeyEntry\> | `[]` | Named scoped API keys |

---

## `[server]` -- ServerConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `bind` | String | `"127.0.0.1"` | Bind address |
| `port` | u16 | 6677 | HTTP port |
| `cors_origins` | Vec\<String\> | `[]` | Allowed CORS origins |

---

## Other Sections

| Section | Struct | Purpose |
|---------|--------|---------|
| `[prd]` | `PrdConfig` | PRD lifecycle settings |
| `[graduation]` | `GraduationConfig` | Bus-to-Store promotion policies |
| `[watcher]` | `WatcherConfig` | Filesystem watcher settings |
| `[tui]` | `TuiConfig` | TUI display preferences |
| `[timeouts]` | `TimeoutConfig` | Global timeout defaults |
| `[statehub]` | `StateHubConfig` | StateHub projection history retention |
| `[scheduler]` | `SchedulerConfig` | Cron scheduling |
| `[webhooks]` | `WebhooksConfig` | Webhook configuration |
| `[github]` | `GitHubConfig` | GitHub repository identity and workflow settings |
| `[subscriptions]` | `Vec<SubscriptionConfig>` | Event subscriptions |
| `[deploy]` | `DeployConfig` | Cloud deployment settings |
| `[perplexity]` | `PerplexityConfig` | Perplexity API settings |
| `[gemini]` | `GeminiConfig` | Gemini API settings |
| `[tools]` | `ToolsConfig` | Tool profiles and allow/deny lists |
| `[chain]` | `ChainConfig` | Chain client config (deprecated) |
| `[relay]` | `RelayConfig` | Relay transport config (to be relocated) |
| `[feed_agents]` | `FeedAgentsConfig` | Feed agent configuration |
| `[validation]` | `ValidationConfig` | Validation strictness settings |
| `[cold_storage]` | `ColdStorageConfig` | Cold-storage archival settings |
| `[prompt]` | `PromptConfig` | Prompt composition knobs |
| `[resources]` | `ResourcesConfig` | Disk budget and GC policy |
| `[dreams]` | `DreamScheduleConfig` | Dream cycle scheduling |
| `[daimon]` | `DaimonConfig` | Affect engine configuration |
| `[retrieval]` | `RetrievalConfig` | RAG retrieval pipeline settings |
| `[[agents]]` | `Vec<AgentDefinition>` | Named agent definitions |
| `[[groups]]` | `Vec<GroupDefinition>` | Persistent group definitions |
| `[[repos]]` | `Vec<RepoConfig>` | Per-repository configuration blocks |
