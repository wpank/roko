# Configuration

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> Updated 2026-04-25: Complete schema reference derived from `roko-core/src/config/schema.rs`.

---

## Configuration file: `roko.toml`

Workspace-level configuration lives at the project root. All sections are optional — missing sections use defaults.

**Canonical source**: `crates/roko-core/src/config/schema.rs` — all types derive `Serialize + Deserialize`.

### Load precedence

1. Read `workdir/roko.toml` from disk
2. Missing file → `RokoConfig::default()` (all defaults applied)
3. Environment variable expansion: `${VAR}` in string values resolved from env
4. `*_file` keys in `extra_headers` resolved to file path contents

### Config versions

| Version | Format | Notes |
|---------|--------|-------|
| `config_version = 1` | Legacy Mori format | Warns on load, suggests `roko config migrate` |
| `config_version = 2` | Current unified schema | Default for new workspaces |

---

## Section reference

### `[project]` — ProjectConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | String | `"roko-project"` | Workspace name |
| `root` | String | `"."` | Workspace root path |
| `fresh_base_branch` | String | `"main"` | Base branch for worktree operations |
| `default_domain` | Option | None | Default task domain |

### `[server]` — ServerConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `bind` | String | `"127.0.0.1"` | Bind address |
| `port` | u16 | `6677` | HTTP port |
| `cors_origins` | Vec\<String\> | `[]` | Allowed CORS origins (empty = permissive) |
| `auth_token` | Option\<String\> | None | Legacy single auth token |

### `[serve]` — ServeConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `port` | Option\<u16\> | None | Override port (falls back to `server.port`) |
| `auto_orchestrate` | bool | `true` | Auto-start orchestration on plan execution |

#### `[serve.auth]` — ServeAuthConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Enable authentication middleware |
| `api_key` | String | `""` | Legacy single API key (use `api_keys` instead) |
| `api_keys` | Vec\<ApiKeyEntry\> | `[]` | Named scoped API keys |
| `privy_app_id` | Option\<String\> | None | Privy app ID for JWT validation |

**ApiKeyEntry**:
```toml
[[serve.auth.api_keys]]
name = "dashboard"
key_hash = "sha256:..."    # SHA-256 hex of plaintext key
scope = "admin"            # "read" | "agent:write" | "plan:write" | "admin"
created_at = "2026-04-20T00:00:00Z"
expires_at = "2027-04-20T00:00:00Z"  # optional
```

#### `[serve.deploy]` — ServeDeployConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | String | `"railway"` | Deploy target |
| `environment` | Vec\<String\> | `["GITHUB_TOKEN", ...]` | Env vars forwarded to deployments |

### `[agent]` — AgentConfig

Top-level agent defaults. Per-agent overrides go in `[[agents]]`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default_model` | String | `"claude-sonnet-4-6"` | Default LLM model |
| `default_backend` | String | `"claude"` | Default provider backend |
| `default_effort` | String | `"medium"` | Task effort level |
| `context_limit_k` | u32 | `200` | Context window limit (K tokens) |
| `bare_mode` | bool | `true` | Run agents in bare mode (no MCP) |
| `fallback_model` | Option\<String\> | None | Fallback when primary unavailable |
| `extensions` | Vec\<String\> | `[]` | Default extension chain |
| `domain` | Option\<String\> | None | Default domain profile |
| `mode` | AgentMode | `Ephemeral` | `ephemeral` / `persistent` / `reactive` |

#### `[agent.roles.<name>]` — per-role overrides

Override any agent field for a specific role (implementer, reviewer, strategist, etc.):

```toml
[agent.roles.reviewer]
model = "claude-haiku-4-5"
effort = "low"
turn_budget_usd = 0.5
```

Available override fields: `model`, `backend`, `effort`, `temperament`, `context_limit_k`, `tools`, `budget`, `thresholds`, `routing_overrides`, `turn_budget_usd`.

#### `[agent.data_llm]` — DataLlmConfig

Dedicated model for structured data extraction (non-creative tasks):

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | String | `"claude-haiku-3-5"` | Model for data extraction |
| `max_tokens` | u64 | `4096` | Output token limit |
| `temperature` | f64 | `0.0` | Temperature (0 = deterministic) |
| `strip_tool_calls` | bool | `true` | Remove tool calls from output |
| `sanitize_input` | bool | `true` | Sanitize inputs before sending |

### `[[agents]]` — agent definitions

Each `[[agents]]` entry defines a named agent:

```toml
[[agents]]
name = "coder-1"
domain = "coding"
prompt = "Implement features and fix bugs"
model = "claude-sonnet-4-6"    # override default
enabled = true
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | String | *required* | Unique agent name |
| `domain` | String | *required* | `"coding"` / `"research"` / `"chain"` / `"general"` |
| `prompt` | String | `""` | Agent purpose description |
| `model` | Option\<String\> | None | Override model |
| `chain_rpc` | Option\<String\> | None | Chain RPC for chain agents |
| `enabled` | bool | `true` | Enable/disable |

### `[providers]` — LLM provider backends

Each provider maps to an LLM API or CLI subprocess:

```toml
[providers.anthropic]
kind = "anthropic_api"
api_key_env = "ANTHROPIC_API_KEY"
max_concurrent = 50

[providers.ollama]
kind = "ollama"
base_url = "http://localhost:11434"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `kind` | ProviderKind | *required* | `anthropic_api` / `claude_cli` / `openai_compat` / `cursor_acp` / `gemini_api` / `perplexity_api` / `ollama` / `codex` / `openai` |
| `base_url` | Option\<String\> | None | API endpoint |
| `api_key_env` | Option\<String\> | None | Env var for API key |
| `command` | Option\<String\> | None | CLI binary (subprocess providers) |
| `timeout_ms` | Option\<u64\> | `120_000` | Request timeout |
| `ttft_timeout_ms` | Option\<u64\> | `15_000` | Time-to-first-token timeout |
| `connect_timeout_ms` | Option\<u64\> | `5_000` | TCP connection timeout |
| `max_concurrent` | Option\<u32\> | None | Concurrency limit |

### `[models]` — model profiles

Map model names to providers with capability flags:

```toml
[models.claude-sonnet]
provider = "anthropic"
slug = "claude-sonnet-4-6-20250514"
context_window = 200000
supports_tools = true
supports_thinking = true
supports_caching = true
cost_input_per_m = 3.0
cost_output_per_m = 15.0
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | String | *required* | Key into `[providers.*]` |
| `slug` | String | *required* | Model ID for API calls |
| `context_window` | u64 | `128_000` | Max context tokens |
| `max_output` | Option\<u64\> | None | Max output tokens |
| `supports_tools` | bool | `true` | Tool/function calling |
| `supports_thinking` | bool | `false` | Extended reasoning |
| `supports_vision` | bool | `false` | Image inputs |
| `supports_caching` | bool | `false` | Provider-side caching |
| `cost_input_per_m` | Option\<f64\> | None | $/M input tokens |
| `cost_output_per_m` | Option\<f64\> | None | $/M output tokens |
| `cost_cache_read_per_m` | Option\<f64\> | None | $/M cached read |

### `[routing]` — model routing

Controls the CascadeRouter (LinUCB bandit) for automatic model selection:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | String | `"auto_override"` | Routing mode |
| `algorithm` | String | `"linucb"` | `linucb` / `thompson` |
| `discount_factor` | f64 | `0.99` | Temporal discount |
| `fast_task_model` | String | `"claude-haiku-4-5"` | T0 reflex model |
| `standard_task_model` | String | `"claude-sonnet-4-6"` | T1 reflective model |
| `complex_task_model` | String | `"claude-opus-4-6"` | T2 deliberate model |

#### `[routing.weights]` — reward weights

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `quality` | f64 | `0.5` | Weight for gate pass rate |
| `cost` | f64 | `0.3` | Weight for cost efficiency |
| `latency` | f64 | `0.2` | Weight for response speed |

Per-complexity overrides: `[routing.weights.mechanical]`, `[routing.weights.focused]`, `[routing.weights.integrative]`, `[routing.weights.architectural]`.

### `[gates]` — gate pipeline

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `clippy_enabled` | bool | `true` | Run clippy gate |
| `skip_tests` | bool | `false` | Skip test gate |
| `max_iterations` | u32 | `3` | Max retry iterations on gate failure |
| `domain_gates` | HashMap | `{}` | Per-domain custom gate lists |

### `[pipeline]` — execution pipeline per complexity

```toml
[pipeline.mechanical]
strategist = false
reviewers = false
max_iterations = 1

[pipeline.architectural]
strategist = true
reviewers = true
reviewer_mode = "full"
max_iterations = 3
```

| Tier | strategist | reviewers | reviewer_mode | max_iterations |
|------|-----------|-----------|---------------|----------------|
| mechanical | false | false | quick | 1 |
| focused | false | false | quick | 2 |
| integrative | true | true | quick | 2 |
| architectural | true | true | full | 3 |

### `[budget]` — cost limits

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_plan_usd` | f32 | `25.0` | Max cost per plan execution |
| `max_turn_usd` | f32 | `3.0` | Max cost per agent turn |
| `prompt_token_budget` | usize | `10_000` | Max prompt tokens |

### `[conductor]` — orchestration control

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_agents` | usize | `8` | Max concurrent agents |
| `max_parallel_plans` | usize | `1` | Max parallel plan executions |
| `parallel_enabled` | bool | `false` | Enable parallel task execution |
| `express_mode` | bool | `false` | Skip strategist for quick fixes |
| `max_auto_fix_attempts` | u32 | `3` | Auto-fix retries before replan |
| `auto_fix_model` | String | `"claude-haiku-4-5"` | Model for auto-fix attempts |
| `warm_implementers_per_plan` | usize | `1` | Pre-spawned warm agents |

### `[learning]` — learning and feedback

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `auto_playbook_refresh` | bool | `true` | Auto-update playbook rules |
| `knowledge_file_intel` | bool | `true` | Include file intel in context |
| `knowledge_warnings` | bool | `true` | Include warnings in context |
| `knowledge_wave_context` | bool | `true` | Include sibling task context |
| `knowledge_error_patterns` | bool | `true` | Include error patterns in context |
| `file_intel_max_entries` | usize | `15` | Max file intel entries per prompt |
| `warning_max_entries` | usize | `5` | Max warning entries per prompt |
| `replan_on_gate_failure` | bool | `true` | Trigger replan on gate failure |
| `replan_max_per_plan` | u32 | `2` | Max replans per plan |
| `replan_gate_attempts` | u32 | `3` | Gate attempts before replan |

### `[chain]` — blockchain

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rpc_url` | Option\<String\> | None | Chain RPC endpoint |
| `chain_id` | Option\<u64\> | None | Chain ID |
| `wallet_key` | Option\<String\> | None | Hex private key (use secrets store) |
| `agent_registry` | Option\<String\> | None | ERC-8004 contract address |
| `bounty_market` | Option\<String\> | None | Bounty market contract address |

### `[relay]` — relay connection

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | Option\<String\> | None | Relay WebSocket URL |
| `workspace_name` | Option\<String\> | None | Workspace name (defaults to hostname) |
| `heartbeat_interval_secs` | u64 | `30` | Heartbeat interval |

### `[energy]` — cognitive energy model

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pool_usd` | f64 | `50.0` | Energy pool in USD |
| `per_task_cap_usd` | f64 | `0.0` | Per-task cap (0 = no cap) |
| `metabolism_rate` | f64 | `0.1` | Base energy consumption rate |

### `[attention]` — attention/context budget

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_tokens_per_layer` | usize | `4096` | Max tokens per context layer |
| `utilization_target` | f64 | `0.85` | Target context utilization |
| `auction_enabled` | bool | `false` | Enable VCG attention auction |

### `[demurrage]` — signal decay

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rate_per_hour` | f64 | `0.01` | Decay rate per hour |
| `min_balance` | f64 | `0.1` | Minimum signal balance |
| `freeze_threshold` | f64 | `0.05` | Balance below which signal freezes |
| `freeze_before_delete` | bool | `true` | Freeze before garbage collection |

### `[tui]` — terminal UI

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `refresh_rate_ms` | u64 | `250` | TUI refresh interval |

### `[deploy]` — deployment

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | String | `"manual"` | `"manual"` / `"railway"` / `"fly"` |
| `railway_api_token` | Option\<String\> | None | Railway API token |
| `project_id` | Option\<String\> | None | Railway project ID |
| `worker_image` | Option\<String\> | `"ghcr.io/nunchi-trade/roko-worker:latest"` | Docker image |

### `[prd]` — PRD lifecycle

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `auto_plan` | bool | `false` | Auto-generate plan on PRD publish |

### `[tools]` — tool permissions

```toml
[tools]
allow = ["bash", "file_read", "file_write"]
deny = ["rm_rf"]

[tools.profiles.coding]
extra_tools = ["cargo", "git"]

[tools.profiles.research]
extra_tools = ["web_search", "pdf_read"]
excluded_tools = ["bash"]
```

### `[[subscriptions]]` — event subscriptions

```toml
[[subscriptions]]
template = "auto-review"
trigger = "signal.gate_failure"
concurrency_limit = 1
cooldown_secs = 60
enabled = true
```

### `[[scheduler.cron]]` — scheduled events

```toml
[[scheduler.cron]]
name = "daily-dream"
expression = "0 3 * * *"
signal_kind = "dream.trigger"
```

---

## Secret management

Secrets are **never stored in roko.toml**. Instead:

1. **Environment variables**: `api_key_env = "ANTHROPIC_API_KEY"` in provider config
2. **Secrets store**: `roko config secrets set <key> <value>` stores encrypted at `~/.roko/secrets/`
3. **`${VAR}` expansion**: Any string value can reference env vars: `rpc_url = "${ETH_RPC_URL}"`

**Secret rotation**: `roko config secrets rotate <key>` updates the secret and signals roko-serve to reload (hot-swap, no restart required).

---

## Full working example

```toml
config_version = 2

[project]
name = "my-workspace"
fresh_base_branch = "main"

[server]
bind = "0.0.0.0"
port = 6677

[serve.auth]
enabled = true
privy_app_id = "cmhw01vut003tjx0d5lmqc8zs"

[agent]
default_model = "claude-sonnet-4-6"
context_limit_k = 200

[routing]
fast_task_model = "claude-haiku-4-5"
standard_task_model = "claude-sonnet-4-6"
complex_task_model = "claude-opus-4-6"

[budget]
max_plan_usd = 25.0
max_turn_usd = 3.0

[conductor]
max_agents = 8
express_mode = false

[learning]
replan_on_gate_failure = true
file_intel_max_entries = 15

[gates]
clippy_enabled = true
skip_tests = false

[[agents]]
name = "coder-1"
domain = "coding"
prompt = "Implement features and fix bugs in Rust"

[[agents]]
name = "pr-reviewer"
domain = "coding"
model = "claude-haiku-4-5"

[[agents]]
name = "researcher"
domain = "research"
```
