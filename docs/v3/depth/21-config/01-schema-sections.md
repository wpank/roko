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

The CaMeL data-LLM boundary: untrusted tool output (MCP, plugin, web-search and network tool
results) goes to this separate, tool-less model, and the main model sees only its validated output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | String | `"claude-haiku-4-5"` | Model for data extraction: a `[models.*]` key or a builtin slug |
| `max_tokens` | u64 | 4096 | Output token limit |
| `temperature` | f64 | 0.0 | Temperature (0 = deterministic) |
| `strip_tool_calls` | bool | true | The data LLM gets no tools; `false` fails config loading |
| `output_schema` | JSON | none | Keys the data LLM's JSON output must have (`required`) |
| `sanitize_input` | bool | true | Strip known injection phrases before the call |
| `timeout_ms` | u64 | 30000 | Time limit for one data-LLM call; a slower call withholds the content |
| `max_input_bytes` | usize | 32768 | Most untrusted text one call is given; the rest is cut off |

Leaving the section out turns the boundary off. With it set, the tool loops roko runs itself send
the output of MCP, plugin, web-search, retrieval and network tools through the data LLM: those of
every agent roko builds for an API provider (Anthropic, OpenAI-compatible, Gemini, Perplexity,
Cerebras), and ACP's. The model sees only the extracted summary and facts, or a notice that they
were withheld. CLI providers run their own tool loops, so it cannot cover them. The data model
must be one roko calls over an API: if roko cannot build it, the agent fails to start, or the ACP
turn fails, rather than run without the boundary.

---

## `[authoring]` -- AuthoringConfig

Which model writes and revises plans. Frontier models plan and cheap models
execute, so the planner is chosen apart from the models that run tasks.

```toml
[authoring]
planner_model = "claude-opus-4-6"   # a key from [models], or a builtin slug
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `planner_model` | String | `""` (unset) | Model for every plan generate and revise path: `roko prd plan`, `roko plan generate` / `regenerate`, the plan-writing bands of `roko do`, and the serve runtime's generate and revise |

Precedence (`model_selection::resolve_planner_model`): `--model`, then
`[authoring] planner_model`, then `[agent.roles.strategist] model`, then
`[agent] model`. An empty value counts as unset. It is a string rather than an
optional value so the key survives the loader, which drops keys that the
serialized default config lacks.

---

## `[runner]` -- CoreRunnerConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sandbox_level` | RunnerSandboxLevel | `"restrict"` | Live enforcement: `none`, `observe`, `restrict`, `isolate`, or `quarantine` |
| `dangerously_skip_permissions` | bool | false | Run agents without the provider's own permission checks (Claude `--dangerously-skip-permissions`, the Codex and Gemini bypass modes). Off by default, and nothing turns it on by itself: `roko plan run`, the direct agent flows (`roko prd`, `plan generate`, `research`, `do`), `roko chat`, template dispatch in `roko serve` and the legacy ACP pipeline read this key, and the other spawn paths never skip. Rejected in strict/shared config |
| `worktree_per_task` | bool | true | `roko plan run` runs each task in its own git worktree and delivers finished plans into the run's batch branch, `roko/batch/<run-id>`, never the operator's checkout; the run ends with the `git merge` that takes the work (gap-4ec59f). A workdir that is not the top level of a git checkout with a commit runs its tasks in the shared working tree. `--worktree-per-task` and `--no-worktree-per-task` override it per run, and a server's runs follow the server's value |

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
| `clippy_enabled` | bool | true | Enable clippy gate |
| `skip_tests` | bool | false | Skip test gate |
| `max_iterations` | u32 | 3 | Global gate retry ceiling |
| `rungs` | array of tables (`name`, `command`, `timeout_secs`, `required`, `parallel_with`) | none | Declared gate rungs. The `required` ones are `roko run`'s verify steps, and every `roko plan run` task runs them after its own, skipping a rung whose command one of its steps already runs. A plan opts out with `[meta] workspace_rungs = false` |

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

Defaults come from `LearningConfig` (`crates/roko-core/src/config/learning.rs`), whose serde
defaults and `Default` impl agree. Checked at `7c556bc0a` (2026-09-29), most of these keys
change nothing: "No effect" marks a key that only the config tooling reads (loading,
`roko config set`, presets and config views). "No effect on Graph runs" marks a key that
`roko plan run`, and the plans that `roko run` and `roko serve` start, never read. The replan
limits `replan_max_per_plan` and `replan_gate_attempts` were removed (see [Removed keys](#removed-keys)).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `auto_playbook_refresh` | bool | true | Refresh playbook rules after successful tasks. No effect |
| `knowledge_file_intel` | bool | true | Inject file difficulty profiles into agent context. No effect |
| `knowledge_warnings` | bool | true | Inject knowledge-store warnings into agent context. No effect |
| `knowledge_wave_context` | bool | true | Pass wave context between tasks. No effect |
| `knowledge_error_patterns` | bool | true | Match error signatures against known patterns. No effect |
| `learning_min_occurrences` | usize | 2 | Occurrences before a learned rule is promoted. No effect |
| `file_intel_max_entries` | usize | 15 | Maximum file-intel entries injected per task. No effect |
| `warning_max_entries` | usize | 5 | Maximum warning entries injected per task. No effect |
| `replan_on_gate_failure` | bool | true | Graph runs never revise a plan: a failed task is retried up to its `max_retries`. When true and a cheap model is available, each failed verify also gets an LLM reflection, saved to `.roko/learn/post-gate-reflections.json` |
| `dream_on_completion` | bool | false | Opt in to dream consolidation on plan completion; otherwise dreams run on demand via `roko knowledge dream run`. No effect on Graph runs: nothing emits the plan-completion event (q-6b7cca) |
| `use_lookahead_router` | bool | false | Pass the cascade router's pick through `LookaheadRouter`, which may choose a cheaper tier. No effect |
| `lookahead_threshold` | f64 | 0.7 | Success probability at which the lookahead router accepts a cheaper tier. No effect |
| `override_learning_dampening` | Option\<f64\> | None | Weight of a manual model override's outcome in router learning. No effect: the router always uses 0.5 (`OVERRIDE_LEARNING_RATE`) |
| `gate_threshold_flush_interval` | u64 | 10 | Gate observations (a count, not seconds) between writes of `.roko/learn/gate-thresholds.json`; 0 is read as 1. Graph runs write the thresholds once this many observations have built up, before a plan's retry budgets are read, and when the run ends (reg-c7ecf6) |
| `t0_reflexes` | bool | false | Run the T0 reflex path in Graph task dispatch. Off by default until reflex rules are credited after verify (bug-94151f) |

The `dreams` and `knowledge` fields are the two sub-tables below.

### `[learning.dreams]` -- DreamsConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `trigger_on_plan_complete` | bool | true | Plan-completion dream trigger; fires only when `learning.dream_on_completion` is also true. No effect on Graph runs: nothing emits the plan-completion event (q-6b7cca) |
| `max_concurrent` | usize | 1 | Most dream consolidations that ACP sessions run at once in one process; an ACP turn that finds this many running starts none (0 is treated as 1). The plan-completion trigger keeps its own limit of one |
| `trigger_on_acp_episodes` | bool | false | Opt in to a dream consolidation from ACP sessions once `acp_episode_threshold` episodes accumulate since the last dream report; independent of the plan-completion switches |
| `acp_episode_threshold` | usize | 10 | Episodes since the last dream report before an ACP session starts a dream (0 is treated as 1) |

### `[learning.knowledge]` -- KnowledgeProgressionConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `transient_confirmations` | u32 | 2 | Confirmations for Transient -> Working promotion. No effect |
| `working_contexts` | u32 | 3 | Distinct contexts for Working -> Consolidated promotion. No effect |
| `consolidated_age_days` | u32 | 14 | Minimum age in days for Consolidated -> Persistent promotion. No effect |
| `demotion_balance_threshold` | f64 | 0.1 | Minimum balance before an entry is considered for demotion. No effect |

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
| `terminal_enabled` | bool | false | Expose the PTY terminal routes |
| `terminal_commands` | Vec\<String\> | `[]` | Command lines a terminal session may run instead of the login shell; any other `command` is refused |
| `terminal_max_sessions` | usize | 8 | Most PTY sessions open at once; `0` lifts the cap |
| `terminal_session_ttl_secs` | u64 | 28800 | Seconds a PTY session may live, attached or not; `0` lifts the limit |
| `revision_max_retries` | u32 | 1 | Times `POST /api/plans/{id}/revise` asks the planning agent again, with the validation diagnostics, after a revision that fails validation; `0` makes one attempt only |

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

## Removed keys

These keys were removed because nothing read them. Loading an old `roko.toml` that sets one
still works: the key is dropped with a warning that says why, and `roko config doctor` parses
the file the same way. `roko config validate` reports the key as removed, so delete the line, and
`roko config set` refuses it with the same reason.
The list is `REMOVED_CONFIG_KEYS` in `crates/roko-core/src/config/loader.rs`.

| Key | Why it went |
|-----|-------------|
| `runner.max_concurrent_plans` | No plan run read it. `conductor.max_parallel_plans` (or `roko plan run --max-parallel-plans`) sets how many plans run at once (gap-6bc156) |
| `gates.domain_gates` | No gate ran its commands. Give the plan tasks of that domain their own verify commands (gap-7a3527) |
| `learning.replan_max_per_plan` | No plan run revises a plan on gate failure, so it limited nothing (gap-7a3527) |
| `learning.replan_gate_attempts` | As for `replan_max_per_plan` (gap-7a3527) |
| `[executor]` (the whole section) | The CLI-only parallel executor it configured never ran in a plan run. `conductor.max_parallel_plans` sets how many plans run at once, and `runner.worktree_per_task` (on by default) runs each task in its own git worktree (gap-666ab3, gap-4ec59f) |
| `tools.prefer_mcp`, `tools.mcp_timeout_secs` | v1 keys of the CLI-only config that nothing read (bug-d5051e) |
| `tools.global_denied` | v1 key that nothing read; `tools.deny` is the current tool denylist (bug-d5051e) |
| `prompt.token_budget` | v1 key that nothing read from roko.toml; `budget.prompt_token_budget` is the current key (bug-d5051e) |
| `prompt.role` | v1 key that nothing read from roko.toml; `--role` chooses the agent role (bug-d5051e) |
| `prompt.files`, `prompt.budgets`, `prompt.context_budgets` | v1 keys that nothing read (bug-d5051e) |

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
