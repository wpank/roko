# 00-ARCH -- Configuration Schema

> **Parent**: [00-ARCHITECTURE](../../00-ARCHITECTURE.md)
>
> Roko has 70+ configurable parameters spread across 30+ config structs. This depth
> file catalogs every section, its defaults, valid ranges, interdependencies, and
> runtime override mechanisms. Updated for E42 (Config Evolution, 8/8) with
> priority/provenance, seven invariants, migrations, profiles, transactional reload,
> and freshness diagnostics.
>
> **Note**: The `[chain]` section is deprecated. Chain-related configuration should
> use the chain crate's local state machines directly. The section remains in the
> schema for backwards compatibility but is not actively developed.
>
> Canonical source: `crates/roko-core/src/config/schema.rs` (~2,600 lines)

---

## 1. Schema Structure

The `RokoConfig` struct is the root. Every section is a separate struct with serde
defaults, so a bare `roko.toml` produces a fully populated config.

```rust
pub struct RokoConfig {
    pub config_version: u32,              // Migration tracking (current: 2)
    pub schema_version: u32,              // Semantic version (current: 2)
    pub project: ProjectConfig,           // Project metadata
    pub agent: AgentConfig,               // Agent/model settings
    pub providers: IndexMap<String, ProviderConfig>,  // Provider registry
    pub models: IndexMap<String, ModelProfile>,       // Model registry
    pub profiles: HashMap<String, DomainProfile>,     // Domain-specific overlays
    pub gates: GatesConfig,               // Verification gates
    pub graduation: GraduationConfig,     // Bus -> Store promotion policies
    pub routing: RoutingConfig,           // Model routing
    pub pipeline: PipelineConfig,         // Complexity-to-pipeline mapping
    pub budget: BudgetConfig,             // Spend/token budgets
    pub conductor: ConductorConfig,       // Meta-orchestrator
    pub watcher: WatcherConfig,           // File-system watcher
    pub learning: LearningConfig,         // Learning subsystem
    pub tui: TuiConfig,                   // Terminal UI
    pub timeouts: TimeoutConfig,          // Global timeout defaults
    pub statehub: StateHubConfig,         // Push-based dashboard state
    pub serve: ServeConfig,               // HTTP API / control plane
    pub scheduler: SchedulerConfig,       // Cron scheduler
    pub webhooks: WebhooksConfig,         // Webhook ingress
    pub github: GitHubConfig,             // GitHub repo identity and workflows
    pub subscriptions: Vec<SubscriptionConfig>,  // Event subscriptions
    pub server: ServerConfig,             // HTTP server/gateway
    pub deploy: DeployConfig,             // Cloud deployment
    pub perplexity: PerplexityConfig,     // Perplexity-specific
    pub gemini: GeminiConfig,             // Gemini-specific
    pub tools: ToolsConfig,              // Tool configuration
    pub chain: ChainConfig,              // [DEPRECATED] Chain settings
    pub relay: RelayConfig,              // Relay connectivity
    pub feed_agents: FeedAgentsConfig,    // Feed agent configuration
    pub runner: CoreRunnerConfig,         // Core runner settings
    pub agents: Vec<AgentDefinition>,     // Persistent agent definitions
    pub groups: Vec<GroupDefinition>,     // Persistent group definitions
    pub validation: ValidationConfig,     // Validation settings
    pub cold_storage: ColdStorageConfig,  // Cold-storage archival
    pub prompt: PromptConfig,             // Composition strategy, VCG warmup
    pub resources: ResourcesConfig,       // Disk budget, GC policy
    pub dreams: DreamScheduleConfig,      // Dream-cycle scheduling
    pub daimon: DaimonConfig,            // Affect-engine configuration
    pub repos: Vec<RepoConfig>,          // Per-repository blocks
    pub retrieval: RetrievalConfig,       // RAG retrieval pipeline
}
```

---

## 2. Parameter Catalog

### 2.1 Project (`[project]`)

| Parameter | Type | Default | Description |
|---|---|---|---|
| `name` | String | `"roko-project"` | Human-readable project name |
| `root` | String | `"."` | Project root directory |
| `fresh_base_branch` | String | `"main"` | Base branch for fresh checkouts |

### 2.2 PRD (`[prd]`, removed)

The `[prd]` section configured the PRD pipeline, which was removed: plans come straight
from a prompt (`roko run --plan`, `roko plan generate`). An old `roko.toml` that still has
a `[prd]` section, `auto_plan` included, still loads; the section is dropped with a
warning.

### 2.3 Agent (`[agent]`)

| Parameter | Type | Default | Description |
|---|---|---|---|
| `default_model` | String | `"claude-sonnet-4-6"` | Default model for all roles |
| `default_backend` | String | `"claude"` | Default backend/provider family |
| `default_effort` | String | `"medium"` | Reasoning effort (low/medium/high/max) |
| `context_limit_k` | u32 | `200` | Context window limit (thousands of tokens) |
| `roles` | HashMap | empty | Per-role model/parameter overrides |

Per-role overrides (`[agent.roles.<name>]`):

| Parameter | Type | Default | Description |
|---|---|---|---|
| `role` | Option\<String\> | None (use section name) | Override the runtime role label |
| `model` | Option\<String\> | None | Model override for this role |
| `backend` | Option\<String\> | None | Backend override for this role |
| `effort` | Option\<String\> | None | Reasoning effort override |
| `context_limit_k` | Option\<u32\> | None | Context window override |
| `tools` | Option\<Vec\<String\>\> | None | Role-local tool whitelist |
| `budget` | Option\<AgentBudget\> | None | Per-turn token and cost caps |
| `thresholds` | Option\<AgentThresholds\> | None | Adaptive gate-threshold overrides |
| `routing_overrides` | Option\<RoutingOverrides\> | None | Force backend/tier during routing |

### 2.4 Providers (`[providers.<name>]`)

| Parameter | Type | Default | Range | Description |
|---|---|---|---|---|
| `kind` | ProviderKind | required | anthropic/openai/openrouter/ollama/gemini/perplexity/cerebras | Protocol family |
| `api_key_env` | Option\<String\> | None | env var name | Environment variable for API key |
| `base_url` | Option\<String\> | None | valid URL | Custom API endpoint |
| `max_retries` | u32 | `3` | 0-10 | Max retry attempts |
| `timeout_secs` | u64 | `300` | 10-3,600 | Request timeout |
| `rate_limit_rpm` | Option\<u32\> | None | 1-100,000 | Requests per minute limit |

### 2.5 Gates (`[gates]`)

| Parameter | Type | Default | Description |
|---|---|---|---|
| `clippy` | bool | `true` | Enable clippy gate |
| `test` | bool | `true` | Enable test gate |
| `compile` | bool | `true` | Enable compile gate |
| `fmt` | bool | `false` | Enable format check gate |
| `diff` | bool | `true` | Enable diff review gate |
| `max_iterations` | u32 | `5` | Max retry iterations per task |
| `timeout_secs` | u64 | `300` | Gate execution timeout |

### 2.6 Routing (`[routing]`)

| Parameter | Type | Default | Range | Description |
|---|---|---|---|---|
| `mode` | String | `"auto_override"` | auto_override/static/bandit | Routing strategy |
| `exploration_rate` | f32 | `0.1` | 0.0-1.0 | Thompson sampling exploration |
| `min_samples` | u32 | `10` | 1-100 | Min samples before bandit arms trusted |
| `cost_weight` | f32 | `0.3` | 0.0-1.0 | Weight of cost in reward signal |
| `latency_weight` | f32 | `0.1` | 0.0-1.0 | Weight of latency in reward signal |
| `quality_weight` | f32 | `0.6` | 0.0-1.0 | Weight of pass rate in reward signal |

**Constraint**: `cost_weight + latency_weight + quality_weight` must equal 1.0 (within
0.01 tolerance).

### 2.7 Budget (`[budget]`)

| Parameter | Type | Default | Range | Description |
|---|---|---|---|---|
| `max_plan_usd` | f32 | `50.0` | 0.01-10,000 | Max spend per plan |
| `max_session_usd` | f32 | `100.0` | 0.01-50,000 | Max spend per session |
| `max_task_usd` | f32 | `10.0` | 0.01-1,000 | Max spend per task |
| `warn_threshold` | f32 | `0.8` | 0.1-0.99 | Fraction triggering warning |
| `block_threshold` | f32 | `0.95` | 0.5-1.0 | Fraction blocking new tasks |

**Constraint**: `warn_threshold < block_threshold`.

### 2.8 Learning (`[learning]`)

| Parameter | Type | Default | Description |
|---|---|---|---|
| `auto_refresh_playbook` | bool | `true` | Refresh playbook after successful tasks |
| `auto_extract_skills` | bool | `true` | Extract skills from successful episodes |
| `episode_retention_days` | u32 | `30` | Days to retain raw episode logs |
| `pattern_min_frequency` | u32 | `3` | Min occurrences for trigram pattern |
| `experiment_min_samples` | u32 | `25` | Min samples per experiment variant |
| `adaptive_thresholds` | bool | `true` | Enable EMA-based gate threshold adaptation |
| `cascade_router_persistence` | bool | `true` | Persist cascade router state |
| `replan_on_gate_failure` | bool | `true` | Trigger replanning on gate failure |

### 2.9 Conductor (`[conductor]`)

| Parameter | Type | Default | Range | Description |
|---|---|---|---|---|
| `max_agents` | u32 | `4` | 1-32 | Max concurrent agents |
| `circuit_breaker_threshold` | u32 | `5` | 1-50 | Failures before circuit opens |
| `circuit_breaker_reset_secs` | u64 | `300` | 30-3,600 | Seconds before half-open retry |
| `health_check_interval_secs` | u64 | `60` | 10-600 | Seconds between health checks |
| `enable_watchers` | bool | `true` | Enable file system watchers |
| `enable_scheduler` | bool | `true` | Enable cron scheduler |

### 2.10 Prompt Composition (`[prompt]`)

| Parameter | Type | Default | Description |
|---|---|---|---|
| `strategy` | String | `"greedy"` | greedy/vcg/auto -- composition strategy |
| `vcg_warmup_threshold` | u32 | `10` | Rounds before auto switches from greedy to VCG |

### 2.11 Additional Sections

| Section | Key parameters | Description |
|---|---|---|
| `[tui]` | `refresh_rate_ms` (250), `theme` ("rosedust") | Terminal UI configuration |
| `[serve]` | `bind` ("127.0.0.1:6677") | HTTP control plane |
| `[server]` | `bind`, `workers` (4), `request_timeout_secs` (30) | HTTP server config |
| `[resources]` | `disk_warn_mb` (500), `disk_block_mb` (100) | Disk budget, GC policy |
| `[dreams]` | `enabled` (true) | Dream-cycle scheduling |
| `[daimon]` | `enabled` (true) | Affect-engine toggle |
| `[cold_storage]` | age-based archival policy | Cold-tier Signal archival |
| `[relay]` | relay connectivity settings | E29 relay transport config |
| `[github]` | repo identity, workflow settings | E46 GitHub integration |

---

## 3. E42 Config Evolution Features

### 3.1 Priority and Provenance

Parameters can be overridden at runtime via four mechanisms, in priority order:

| Priority | Mechanism | Scope | Persistence |
|---|---|---|---|
| 1 (highest) | CLI flags | Single invocation | None |
| 2 | Environment variables | Session | None |
| 3 | Profile overlay | Persistent | On disk |
| 4 (lowest) | `roko.toml` | Persistent | On disk |

### 3.2 Seven Config Invariants

E42 guarantees seven structural invariants:

1. **Complete defaults**: `RokoConfig::default()` is always valid
2. **Forward compatibility**: Unknown TOML fields warn but do not fail
3. **Migration chain**: Schema version changes have sequential migration functions
4. **Transactional reload**: Config reloads are atomic; partial loads are rejected
5. **Profile merge**: Named profiles overlay the base config with clear precedence
6. **Freshness tracking**: `roko config doctor` detects stale configs
7. **Secret isolation**: Secrets are stored separately, never in plain TOML

### 3.3 Profiles (`[profiles.<name>]`)

Named domain-specific overlays that merge on top of the base config:

```toml
[profiles.chain]
agent.default_model = "claude-opus-4-6"
budget.max_task_usd = 25.0
gates.test = false
```

### 3.4 Transactional Reload

Config reloads are atomic:

1. Parse the new TOML into a fresh `RokoConfig`
2. Validate all constraints
3. If valid, atomically swap the active config
4. If invalid, keep the previous config and log the error

### 3.5 Migration

```rust
fn migrate_v1_to_v2(old: &toml::Value) -> Result<toml::Value, MigrationError> {
    // Rename [agent.model] -> [agent.default_model]
    // Move [agent.mcp] -> [agent.mcp_config]
    // Remove deprecated [chain.rate_oracle]
    // ...
}
```

---

## 4. Interdependencies

Some parameters constrain each other. The config loader validates these on startup:

| Constraint | Parameters | Validation |
|---|---|---|
| Budget ordering | `warn_threshold`, `block_threshold` | `warn < block` |
| Reward weights | `cost_weight`, `latency_weight`, `quality_weight` | Sum == 1.0 (within 0.01 tolerance) |
| Max agents vs budget | `max_agents`, `max_session_usd` | `max_agents * max_task_usd <= max_session_usd` (warning) |
| Gate iterations vs pipeline | `gates.max_iterations`, `pipeline.*.max_iterations` | Pipeline value overrides gate default |
| Model availability | `agent.default_model`, provider entries | Default model's provider must exist |

---

## 5. Validation Rules

```rust
impl RokoConfig {
    pub fn validate(&self) -> Vec<ConfigWarning> {
        let mut warnings = Vec::new();

        // Budget ordering
        if self.budget.warn_threshold >= self.budget.block_threshold {
            warnings.push(ConfigWarning::BudgetThresholdOrder);
        }

        // Reward weight sum
        let sum = self.routing.cost_weight
            + self.routing.latency_weight
            + self.routing.quality_weight;
        if (sum - 1.0).abs() > 0.01 {
            warnings.push(ConfigWarning::RewardWeightSum { actual: sum });
        }

        // Default model has a provider
        if !self.providers.is_empty() {
            let has_provider = self.providers.values()
                .any(|p| p.models_include(&self.agent.default_model));
            if !has_provider {
                warnings.push(ConfigWarning::OrphanDefaultModel {
                    model: self.agent.default_model.clone(),
                });
            }
        }

        warnings
    }
}
```

---

## 6. Full TOML Schema Example

```toml
config_version = 2
schema_version = 2

[project]
name = "roko"
root = "."
fresh_base_branch = "main"

[agent]
default_model = "claude-sonnet-4-6"
default_backend = "claude"
default_effort = "medium"
context_limit_k = 200

[agent.roles.implementer]
model = "claude-opus-4-6"
effort = "high"

[agent.roles.reviewer]
model = "claude-sonnet-4-6"

[providers.anthropic]
kind = "anthropic"
api_key_env = "ANTHROPIC_API_KEY"
max_retries = 3
timeout_secs = 300

[providers.openai]
kind = "openai"
api_key_env = "OPENAI_API_KEY"

[gates]
clippy = true
test = true
compile = true
diff = true
fmt = false
max_iterations = 5
timeout_secs = 300

[routing]
mode = "auto_override"
exploration_rate = 0.1
cost_weight = 0.3
latency_weight = 0.1
quality_weight = 0.6

[budget]
max_plan_usd = 50.0
max_session_usd = 100.0
max_task_usd = 10.0
warn_threshold = 0.8
block_threshold = 0.95

[conductor]
max_agents = 4
circuit_breaker_threshold = 5
circuit_breaker_reset_secs = 300

[learning]
auto_refresh_playbook = true
auto_extract_skills = true
episode_retention_days = 30
adaptive_thresholds = true
cascade_router_persistence = true
replan_on_gate_failure = true

[tui]
refresh_rate_ms = 250
theme = "rosedust"

[serve]
bind = "127.0.0.1:6677"

[prompt]
strategy = "greedy"
vcg_warmup_threshold = 10

[resources]
disk_warn_mb = 500
disk_block_mb = 100

[dreams]
enabled = true

[daimon]
enabled = true

# [chain]  -- DEPRECATED: see roko-chain local state machines
```

---

## 7. CLI Commands

| Command | What it does |
|---|---|
| `roko config init` | Create default roko.toml |
| `roko config show` | Print resolved config |
| `roko config path` | Print config file path |
| `roko config edit` | Open config in $EDITOR |
| `roko config set <key> <value>` | Set a config value |
| `roko config validate` | Validate without modifying |
| `roko config migrate` | Run migration chain |
| `roko config doctor` | Print config health (freshness, missing secrets) |
| `roko config set-secret` | Store a secret separately |
| `roko config check-secrets` | Verify secret availability |
| `roko config export` | Export as env vars for deployment |
| `roko config env` | List all recognized env vars |
| `roko config providers list/health/test` | Provider inspection |
| `roko config models list/route` | Model inspection and routing |
| `roko config preset gates/routing/budget/model` | Apply validated presets (--dry-run, --yes) |
| `roko config profiles list/add/remove` | Profile management |
| `roko config secrets set/get/list/rotate` | Profile-aware secrets |
| `roko config mcp list/test/add` | MCP server configuration |

---

## 8. Error Handling

| Condition | Response |
|---|---|
| Missing `roko.toml` | Use `RokoConfig::default()` with all serde defaults |
| Malformed TOML | Return parse error with line number and column |
| Unknown field | Warn but do not fail (forwards compatibility) |
| Wrong type for field | Return type mismatch error with expected and actual |
| Schema version mismatch | Run migration chain; fail if no migration path |
| Validation warning | Print warning, continue with config as-is |
| Validation error (e.g., negative budget) | Refuse to start; print error |

---

## 9. Test Criteria

1. `RokoConfig::default()` passes `validate()` with zero warnings
2. Minimal TOML (just `[project] name = "x"`) deserializes and validates
3. Full TOML with all sections deserializes and validates
4. Budget threshold violation (`warn >= block`) produces a validation warning
5. Reward weight sum != 1.0 produces a validation warning
6. CLI flag overrides TOML value for the same parameter
7. Environment variable overrides TOML value
8. CLI flag overrides environment variable
9. Unknown TOML field does not cause deserialization failure
10. Schema version migration from v1 to v2 produces a valid v2 config
11. Profile merge applies overlays in correct precedence order
12. Transactional reload rejects partial/invalid configs atomically

---

## Cross-References

- [00-ARCHITECTURE](../../00-ARCHITECTURE.md) -- Parent chapter
- [21-01 Schema Sections](../21-config/01-schema-sections.md) -- Section-level depth
- [21-02 Provenance Internals](../21-config/02-provenance-internals.md) -- Priority/merge
- [21-03 Migration Mechanics](../21-config/03-migration-mechanics.md) -- Migration chain
- [21-04 Hot Reload](../21-config/04-hot-reload-protocol.md) -- Transactional reload
- [21-05 Presets and Profiles](../21-config/05-presets-and-profiles.md) -- Profile system
- `crates/roko-core/src/config/schema.rs` -- RokoConfig struct (~2,600 lines)
- `crates/roko-core/src/config/loader.rs` -- Config loading and validation
- `crates/roko-core/src/config/compat.rs` -- Legacy compatibility
