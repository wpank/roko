# 19 -- Configuration

> Configuration is a Signal. `Kind::Config` carries content-addressed, versioned, lineage-tracked, demurrage-decayed configuration state. Runtime overrides resolve through a Compose Cell. Schema validation runs as a Verify Cell. Hot reload fires from a Trigger Cell. The same five primitives that govern every other subsystem govern configuration.

> **Implementation status:** **E42 manifest COMPLETE (8/8); broader spec PARTIAL.** The runtime configuration layer is implemented: unified TOML loading, schema v1 -> v2 migration before deserialization, layered per-field resolution with provenance, post-merge invariant validation, selective transactional reload, inheritable domain-profile overlays, and per-section freshness warnings in `roko doctor`. Config-as-Signal (`Kind::Config`) and the `ConfigComposeCell`, `ConfigVerifyCell`, `ConfigMigrateCell`, and `ConfigWatchTrigger` protocol Cells remain aspirational. There is a synchronous reload API, but no config filesystem watcher or reload Graph yet.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal, Kind, content addressing, demurrage), [02-CELL](02-CELL.md) (9 protocols, Compose, Verify, Trigger), [03-GRAPH](03-GRAPH.md) (Graph composition), [13-TRIGGERS](13-TRIGGERS.md) (Trigger Cell, file watcher)

---

## 1. Config as Signal

> **Implementation status:** Aspirational. No `Kind::Config` exists in code. Configuration is loaded from TOML files, not wrapped in Signals.

Configuration is not special-cased infrastructure. It is data that:
- has a content hash (identical configs produce the same SHA-256),
- has a version (`schema_version` tracks breaking changes),
- has lineage (each config derives from its predecessor plus an override source),
- decays (stale config is worse than no config -- a six-month-old routing weight is probably wrong).

Configuration is a Signal with `Kind::Config`. It participates in the same Store, Bus, and protocol system as every other Signal. Config Signals are content-addressed, carry lineage, can be queried by HDC similarity, and are subject to demurrage.

```rust
/// Configuration as a Signal.
///
/// A config Signal carries the full or partial configuration state.
/// Content-addressed: the same config values produce the same hash.
/// Versioned: schema_version tracks breaking changes.
/// Lineage: parent_hashes point to the config(s) it was derived from.
pub fn config_signal(config: &RokoConfig, source: ConfigSource) -> Signal {
    Signal {
        kind: Kind::Config,
        payload: serde_json::to_value(config).expect("config is serializable"),
        metadata: SignalMetadata {
            schema_version: config.schema_version,
            source: source.to_string(),
            hash: ContentHash::compute(&canonical_bytes(config)),
            parent_hashes: vec![],  // set by the Compose protocol
            demurrage_balance: 1.0,
            ..Default::default()
        },
    }
}
```

---

## 2. ConfigSource and Priority

> **Implementation status:** Implemented for the runtime loader. `ValidatedConfig::merge_context` retains runtime-only `FieldProvenance` entries for explicit project-file fields, changed global-file values, and named or hierarchical environment overrides. Migration steps are retained in the broader provenance trail. The loader records a default sentinel when no config file exists rather than materializing provenance for every compiled default.

Every config value can have provenance. Sources are ordered by priority -- higher-priority sources override lower ones for the same field.

```rust
/// Where a config value came from, in priority order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    File,
    Migration,
    Default,
    Env,
    LocalOverride,
    CliOverride,
    Evolved,
    Composed,
    ApiOverride,
}

impl ConfigSource {
    pub const fn priority(&self) -> u8 {
        match self {
            Self::ApiOverride => 5,
            Self::CliOverride => 4,
            Self::Env => 3,
            Self::File | Self::LocalOverride => 2,
            Self::Evolved | Self::Migration => 1,
            Self::Default | Self::Composed => 0,
        }
    }
}
```

Declared resolution is `API (5) > CLI (4) > Env (3) > File/LocalOverride (2) > Evolved/Migration (1) > Default/Composed (0)`. File sources at equal priority are applied in specificity order, with the project file winning over the global file. The unified loader currently materializes the default, global/project file, migration, and environment layers; CLI, API, and evolved categories are available for their owning integrations. API overrides and all provenance metadata are runtime-only and do not persist to `roko.toml`.

### Environment Variable Convention

```
ROKO__SECTION__FIELD  ->  section.field

ROKO__CONDUCTOR__MAX_AGENTS=8       -> conductor.max_agents = 8
ROKO__BUDGET__MAX_PLAN_USD=200      -> budget.max_plan_usd = 200.0
ROKO__ROUTING__COST_WEIGHT=0.4      -> routing.cost_weight = 0.4
ROKO__AGENT__DEFAULT_MODEL=opus     -> agent.default_model = "opus"
```

The generic convention uses the `ROKO__` prefix and double underscores between path components. Named compatibility variables such as `ROKO_MODEL`, `ROKO_BACKEND`, `ROKO_CONTEXT_LIMIT_K`, `ROKO_MAX_AGENTS`, and `ROKO_BUDGET_USD` are also supported. `${VAR}` expansion inside TOML string values is supported, for example `rpc_url = "${ETH_RPC_URL}"`.

---

## 3. Config Compose Cell

> **Implementation status:** The `ConfigComposeCell` and Signal lineage shown below are aspirational. The non-Signal runtime equivalent is implemented in `roko-core/src/config/loader.rs`: defaults, global config, the more-specific project file, named environment variables, and hierarchical `ROKO__*` variables resolve field by field, with provenance retained in `MergeContext`.

In the target architecture, API, CLI, environment, file, evolved, and default layers form a **Compose protocol** (02-CELL.md) that merges config Signals by priority. The Compose Cell assembles a composite config from multiple partial inputs.

```rust
/// Compose Cell: merges config Signals by priority.
///
/// Input: up to 4 config Signals (evolved, toml, env, cli), each partial.
/// Output: 1 merged config Signal with full lineage.
///
/// Priority: API > CLI > Env > TOML > Evolved > Default.
/// Each field resolves independently: the highest-priority source
/// that provides a value for that field wins.
pub struct ConfigComposeCell;

impl Cell for ConfigComposeCell {
    fn protocols(&self) -> &[ProtocolId] { &[ProtocolId::Compose] }
    fn name(&self) -> &str { "config-compose" }

    async fn execute(
        &self,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        let mut sources = input.into_iter()
            .map(|s| {
                let source: ConfigSource = extract_source(&s)?;
                let priority = source.priority();
                Ok((priority, s))
            })
            .collect::<Result<Vec<_>, CellError>>()?;
        sources.sort_by(|a, b| b.0.cmp(&a.0));

        // Start with compiled defaults
        let mut merged = RokoConfig::default();

        // Apply in reverse priority order (lowest first, highest wins)
        let mut parent_hashes = Vec::new();
        for (_, signal) in sources.iter().rev() {
            apply_partial_config(&mut merged, &signal.payload)?;
            parent_hashes.push(signal.hash());
        }

        // Build merged config Signal with lineage
        let mut result = config_signal(&merged, ConfigSource::Composed);
        result.metadata.parent_hashes = parent_hashes;

        Ok(vec![result])
    }
}
```

### Resolution Algorithm

```rust
fn resolve_field(field: &str, sources: &[ConfigSignal]) -> Value {
    // Sources sorted by priority (highest first)
    for source in sources {
        if let Some(value) = source.get_field(field) {
            return value;
        }
    }
    // Fall through to compiled default
    RokoConfig::default().get_field(field)
}
```

The merged config Signal records all parent hashes. To trace where a value came from: walk the lineage. To see what changed: diff the current config Signal against its parent. The same lineage system used for every other Signal.

---

## 4. Config Verify Cell

> **Implementation status:** The `ConfigVerifyCell` and Verdict Signal shown below are aspirational. The equivalent runtime checks are implemented by `validate_invariants` and run unconditionally after all file, environment, interpolation, and secret-resolution layers. Error-severity failures reject the load; warning-severity failures are logged and included in validated-config diagnostics.

Configuration validation is a **Verify protocol Cell** (02-CELL.md). It takes a config Signal as input and emits a Verdict Signal.

```rust
/// Verify Cell: config schema validation.
///
/// Checks 7 invariants plus type correctness, provider existence,
/// schema version compatibility, and unknown top-level field detection.
pub struct ConfigVerifyCell;

impl Cell for ConfigVerifyCell {
    fn protocols(&self) -> &[ProtocolId] { &[ProtocolId::Verify] }
    fn name(&self) -> &str { "config-verify" }

    async fn execute(
        &self,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        let config: RokoConfig = serde_json::from_value(input[0].payload.clone())?;
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // Check all 7 invariants (see section 4.1)
        validate_invariants(&config, &mut errors, &mut warnings);

        // Schema version migration check
        if config.schema_version < CURRENT_SCHEMA_VERSION {
            warnings.push(format!(
                "Config schema version {} is behind current {}; migration available",
                config.schema_version, CURRENT_SCHEMA_VERSION
            ));
        }

        let passed = errors.is_empty();
        let verdict = Signal::new(
            Kind::Verdict,
            ConfigVerdict { passed, errors, warnings },
        );
        Ok(vec![verdict])
    }
}
```

### 4.1 The 7 Invariants

| # | Invariant | Parameters | Rule |
|---|---|---|---|
| 1 | Budget ordering | `budget.max_turn_usd`, `budget.max_plan_usd` | Turn ceiling must not exceed a finite plan ceiling; error. Zero retains its unlimited-budget semantics. |
| 2 | Gate iteration hierarchy | `gates.max_iterations`, `pipeline.*.max_iterations` | Each pipeline band must remain within the global gate ceiling, and retries must not decrease from mechanical through architectural; warning. |
| 3 | Provider existence | `models.*.provider`, `providers` | Every configured model must reference an existing provider; error. |
| 4 | Agent capacity vs budget | `conductor.max_agents * budget.max_turn_usd * 10` | Heuristic estimated parallel cost should not exceed a finite plan ceiling; warning. |
| 5 | Context bounds | `agent.context_limit_k` | Must be within `4..=1000`; warning. |
| 6 | Conductor parallelism | `conductor.max_agents` | Must be at least one; error. |
| 7 | Learning consistency | `learning.replan_on_gate_failure`, test/clippy gates | Warn when replanning is enabled while both test and clippy gates are disabled. |

### 4.2 Error Handling

| Condition | Response |
|---|---|
| Missing `roko.toml` | Use `RokoConfig::default()` -- system is functional with just defaults |
| Malformed TOML | Parse error with line/column. Refuse to start. |
| Unknown top-level field | Add a diagnostic, log a warning, and continue (forward compatibility) |
| Unknown nested field | Continue silently under serde compatibility; nested unknown-field diagnostics are not implemented |
| Wrong type | Type mismatch error with expected vs actual |
| Schema version mismatch | Run migration chain; fail if no migration path |
| Validation warning | Log warning, continue |
| Validation error | Refuse to start, print error |

---

## 5. Config Watch Trigger

> **Implementation status:** The `ConfigWatchTrigger`, automatic filesystem watch, and Trigger/Compose/Verify reload Graph remain aspirational. The core does implement `ConfigWatchCallback`, `ConfigReloadRequest`, `ReloadPolicy` (500 ms debounce, auto-reload off, reject invariant errors), and synchronous `try_reload`. `try_reload` loads through the unified migration/merge/validation path, leaves the current config unchanged on failure, applies hot-reloadable sections, and reports changes that still require restart.

The hot-reloadable sections are budget, tools, learning, gates/pipeline, conductor, and routing. Agent, provider, model, serve, scheduler, watcher, profile, and server changes are detected but require restart. The CLI owns any future filesystem watcher; `roko-core` deliberately does not depend on `notify`.

The target architecture is a **Trigger Cell** ([13-TRIGGERS](13-TRIGGERS.md)) watching `roko.toml`:

```rust
/// Trigger Cell: watches roko.toml for changes and triggers reload.
///
/// When the file changes:
/// 1. Read the new config from disk.
/// 2. Create a new Config Signal with TomlFile source.
/// 3. Route through ConfigComposeCell (merge with env and CLI).
/// 4. Route through ConfigVerifyCell.
/// 5. If verification passes: publish on "config.reloaded" topic.
/// 6. If verification fails: publish "config.reload_failed" with errors.
///    The old config remains active.
pub struct ConfigWatchTrigger {
    watch_path: PathBuf,
    debounce_ms: u64,  // default: 500ms
}

impl Cell for ConfigWatchTrigger {
    fn protocols(&self) -> &[ProtocolId] { &[ProtocolId::Trigger] }
    fn name(&self) -> &str { "config-watch-trigger" }

    async fn execute(
        &self,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        let event = extract_file_event(&input[0])?;
        if event.path != self.watch_path {
            return Ok(vec![]);
        }

        // Debounce: skip if last reload was too recent
        if event.elapsed_since_last < Duration::from_millis(self.debounce_ms) {
            return Ok(vec![]);
        }

        // Read new config
        let toml_content = tokio::fs::read_to_string(&self.watch_path).await?;
        let new_config: RokoConfig = toml::from_str(&toml_content)?;
        let config_signal = config_signal(
            &new_config,
            ConfigSource::TomlFile(self.watch_path.clone()),
        );

        // The Graph routes this through Compose -> Verify -> publish
        Ok(vec![config_signal])
    }
}
```

### 5.1 The Config Reload Graph

```toml
# Graph: config hot reload pipeline
[graph]
name = "config-reload"
description = "Watch roko.toml, merge overrides, validate, publish"

[[graph.node]]
id   = "watch"
kind = "block"
block = "config-watch-trigger"

[[graph.node]]
id   = "compose"
kind = "block"
block = "config-compose"

[[graph.node]]
id   = "verify"
kind = "block"
block = "config-verify"

[[graph.edge]]
from = "watch"
to   = "compose"
[[graph.edge.maps]]
from = "out"
to   = "toml_input"

[[graph.edge]]
from = "compose"
to   = "verify"

# After verification, the config Signal is published on the Bus.
# All Cells that depend on config subscribe to "config.reloaded".
```

---

## 6. Schema Versioning and Migration

> **Implementation status:** Implemented in the unified loader as `ConfigMigrator`, `MigrationFn`, `MigrationReport`, and `MigrationStep`. TOML is migrated as a `toml::Value` before serde deserialization. The built-in v1 -> v2 step renames legacy nested agent and budget keys and sets both version fields to 2. Missing schema versions default to v1; live loading fails if the chain cannot reach the current schema. The Signal-based `ConfigMigrateCell` below remains aspirational, and the separate flat Mori converter remains available for that legacy format.

Config carries two version numbers:

| Field | Purpose | Current |
|---|---|---|
| `config_version` | Layout version for migration tooling | 2 |
| `schema_version` | Semantic version for the parameter set | 2 |

When a breaking change is introduced (renamed field, changed default, removed parameter), bump `schema_version` and add a migration function.

```rust
/// Config migration as a Cell.
///
/// Takes a config Signal at schema version N and produces a config Signal
/// at schema version N+1. Migrations are chained: v1 -> v2 -> v3.
pub struct ConfigMigrateCell {
    migrations: BTreeMap<u32, Box<dyn Fn(&toml::Value) -> Result<toml::Value, MigrationError>>>,
}

impl Cell for ConfigMigrateCell {
    fn name(&self) -> &str { "config-migrate" }

    async fn execute(
        &self,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        let signal = &input[0];
        let mut config_value: toml::Value = extract_toml(&signal)?;
        let mut current_version = extract_schema_version(&config_value);

        while current_version < CURRENT_SCHEMA_VERSION {
            let migrator = self.migrations.get(&current_version)
                .ok_or(CellError::NoMigrationPath {
                    from: current_version,
                    to: CURRENT_SCHEMA_VERSION,
                })?;
            config_value = migrator(&config_value)?;
            current_version += 1;
        }

        // Migrated config is a new Signal with lineage to the old one
        let new_config: RokoConfig = toml::from_str(&config_value.to_string())?;
        let mut result = config_signal(&new_config, ConfigSource::Migration);
        result.metadata.parent_hashes = vec![signal.hash()];

        Ok(vec![result])
    }
}
```

| Version | Format | Notes |
|---|---|---|
| `schema_version = 1` or omitted | Previous unified schema | Automatically migrates nested `agent.model/backend/effort` and `budget.max_session_usd/max_agent_usd` names before deserialization |
| `schema_version = 2` | Current unified schema | No migration step is applied |

`config_version` is also set to 2 by the built-in migration. Flat Mori-format conversion is handled separately by `from_mori_toml`; it is not the schema migration chain described here.

---

## 7. Minimal Config

A bare-minimum config Signal:

```toml
config_version = 2
schema_version = 2

[project]
name = "my-project"
```

Every other section uses `#[serde(default)]` defaults. The system is fully functional with just a project name. Today serde and `RokoConfig::default()` fill unspecified fields; a future Config Compose Cell would express that operation in the Signal graph.

---

## 8. Full Section Reference

The canonical source is `crates/roko-core/src/config/schema.rs`. All types derive `Serialize + Deserialize`.

### 8.1 `[project]` -- ProjectConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `name` | String | `"roko-project"` | Workspace name |
| `root` | String | `"."` | Workspace root path |
| `fresh_base_branch` | String | `"main"` | Base branch for worktree operations |
| `default_domain` | Option\<String\> | None | Default task domain |

### 8.2 `[server]` -- ServerConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `bind` | String | `"127.0.0.1"` | Bind address |
| `port` | u16 | `6677` | HTTP port |
| `cors_origins` | Vec\<String\> | `[]` | Allowed CORS origins (empty = permissive) |
| `auth_token` | Option\<String\> | None | Legacy single auth token |

### 8.2a `[statehub]` -- StateHubConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `history_retention` | String duration | `"7d"` | Maximum age of projection-history records; enforced during restore, live updates, and queries |

### 8.3 `[serve]` -- ServeConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `port` | Option\<u16\> | None | Override port (falls back to `server.port`) |
| `auto_orchestrate` | bool | `true` | Auto-start orchestration on plan execution |

#### `[serve.auth]` -- ServeAuthConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Enable authentication middleware |
| `api_key` | String | `""` | Legacy single API key (use `api_keys` instead) |
| `api_keys` | Vec\<ApiKeyEntry\> | `[]` | Named scoped API keys |
| `privy_app_id` | Option\<String\> | None | Privy app ID for JWT validation |

```toml
[[serve.auth.api_keys]]
name = "dashboard"
key_hash = "sha256:..."    # SHA-256 hex of plaintext key
scope = "admin"            # "read" | "agent:write" | "plan:write" | "admin"
created_at = "2026-04-20T00:00:00Z"
expires_at = "2027-04-20T00:00:00Z"  # optional
```

### 8.4 `[agent]` -- AgentConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `default_model` | String | `"claude-sonnet-4-6"` | Default LLM model |
| `default_backend` | String | `"claude"` | Default provider backend |
| `default_effort` | String | `"medium"` | Task effort level |
| `context_limit_k` | u32 | `200` | Context window limit (K tokens) |
| `bare_mode` | bool | `true` | For Claude CLI, replace its built-in system prompt with Roko's canonical prompt instead of appending to it; MCP/tool policy remains independently configured |
| `fallback_model` | Option\<String\> | None | Fallback when primary unavailable |
| `extensions` | Vec\<String\> | `[]` | Default extension chain |
| `domain` | Option\<String\> | None | Default domain profile |
| `mode` | AgentMode | `Ephemeral` | `ephemeral` / `persistent` / `reactive` |

#### `[agent.roles.<name>]` -- per-role overrides

```toml
[agent.roles.reviewer]
model = "claude-haiku-4-5"
effort = "low"
turn_budget_usd = 0.5
```

Available override fields: `model`, `backend`, `effort`, `temperament`, `context_limit_k`, `tools`, `budget`, `thresholds`, `routing_overrides`, `turn_budget_usd`.

#### `[runner]` -- CoreRunnerConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `sandbox_level` | RunnerSandboxLevel | `"restrict"` | Live Runner/ACP enforcement level: `none`, `observe`, `restrict`, `isolate`, or `quarantine`; unknown values fail config parsing |

#### `[agent.data_llm]` -- DataLlmConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `model` | String | `"claude-haiku-3-5"` | Model for data extraction |
| `max_tokens` | u64 | `4096` | Output token limit |
| `temperature` | f64 | `0.0` | Temperature (0 = deterministic) |
| `strip_tool_calls` | bool | `true` | Remove tool calls from output |
| `sanitize_input` | bool | `true` | Sanitize inputs before sending |

### 8.5 `[[agents]]` -- agent definitions

```toml
[[agents]]
name = "coder-1"
domain = "coding"
prompt = "Implement features and fix bugs"
model = "claude-sonnet-4-6"
enabled = true
```

| Field | Type | Default | Description |
|---|---|---|---|
| `name` | String | *required* | Unique agent name |
| `domain` | String | *required* | `"coding"` / `"research"` / `"chain"` / `"general"` |
| `prompt` | String | `""` | Agent purpose description |
| `model` | Option\<String\> | None | Override model |
| `chain_rpc` | Option\<String\> | None | Chain RPC for chain agents |
| `enabled` | bool | `true` | Enable/disable |

### 8.6 `[providers]` -- LLM provider backends

```toml
[providers.anthropic]
kind = "anthropic_api"
api_key_env = "ANTHROPIC_API_KEY"
max_concurrent = 50

[providers.anthropic.limits]
rpm = 50
tpm = 40000
max_cpu_seconds = 120
max_rss_bytes = 2147483648
max_processes = 8
network = "allow"

[providers.ollama]
kind = "ollama"
base_url = "http://localhost:11434"
```

| Field | Type | Default | Description |
|---|---|---|---|
| `kind` | ProviderKind | *required* | `anthropic_api` / `claude_cli` / `openai_compat` / `cursor_acp` / `gemini_api` / `perplexity_api` / `ollama` / `codex` / `openai` |
| `base_url` | Option\<String\> | None | API endpoint |
| `api_key_env` | Option\<String\> | None | Env var for API key |
| `command` | Option\<String\> | None | CLI binary (subprocess providers) |
| `timeout_ms` | Option\<u64\> | `120_000` | Request timeout |
| `ttft_timeout_ms` | Option\<u64\> | `15_000` | Time-to-first-token timeout |
| `connect_timeout_ms` | Option\<u64\> | `5_000` | TCP connection timeout |
| `max_concurrent` | Option\<u32\> | None | Concurrency limit |

`[providers.<id>.limits]` applies shared request/token budgets and OS-backed guarantees
to subprocess providers:

| Field | Type | Default | Description |
|---|---|---|---|
| `rpm` | u32 | `0` | Requests-per-minute ceiling; zero disables this ceiling |
| `tpm` | u64 | `0` | Tokens-per-minute ceiling; zero disables this ceiling |
| `max_cpu_seconds` | Option\<u64\> | None | Per-process CPU-time limit; unsupported requests fail closed |
| `max_rss_bytes` | Option\<u64\> | None | Address-space/memory ceiling where the host can enforce it |
| `max_processes` | Option\<u64\> | None | Unix `RLIMIT_NPROC` ceiling (accounted per real user ID) |
| `network` | `"allow"` or `"deny"` | `"allow"` | Deny all subprocess network access through macOS Seatbelt or Linux firejail+seccomp; unsupported hosts fail closed |

Network denial covers provider subprocesses, not in-process HTTP transports, and is
currently all-or-nothing rather than domain/port scoped. `timeout_ms` bounds provider
workloads and the complete Hermes/OpenClaw readiness/version probe sequence.

### 8.7 `[models]` -- model profiles

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
|---|---|---|---|
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

`supports_vision = true` enables inline base64 image input only on the supported
Anthropic, OpenAI-compatible, and Gemini API transports. It also controls the ACP image
capability advertisement. Unsupported transports and non-vision models fail closed;
remote image URLs and audio are not accepted.

### 8.8 `[routing]` -- model routing

Controls the CascadeRouter (LinUCB bandit) for automatic model selection:

| Field | Type | Default | Description |
|---|---|---|---|
| `mode` | String | `"auto_override"` | Routing mode |
| `algorithm` | String | `"linucb"` | `linucb` / `thompson` |
| `discount_factor` | f64 | `0.99` | Temporal discount |
| `fast_task_model` | String | `"claude-haiku-4-5"` | T0 reflex model |
| `standard_task_model` | String | `"claude-sonnet-4-6"` | T1 reflective model |
| `complex_task_model` | String | `"claude-opus-4-6"` | T2 deliberate model |

#### `[routing.weights]`

| Field | Type | Default | Description |
|---|---|---|---|
| `quality` | f64 | `0.5` | Weight for gate pass rate |
| `cost` | f64 | `0.3` | Weight for cost efficiency |
| `latency` | f64 | `0.2` | Weight for response speed |

Per-complexity overrides: `[routing.weights.mechanical]`, `[routing.weights.focused]`, `[routing.weights.integrative]`, `[routing.weights.architectural]`.

### 8.9 `[gates]` -- gate pipeline

| Field | Type | Default | Description |
|---|---|---|---|
| `mode` | enum | `full` | Verification breadth: `none`, `structural`, `focused`, `full` |
| `clippy_enabled` | bool | `true` | Run clippy gate |
| `skip_tests` | bool | `false` | Skip test gate |
| `max_iterations` | u32 | `3` | Max retry iterations on gate failure |
| `cargo_fix_enabled` | bool | `true` | Attempt `cargo fix --allow-dirty` before agent retry |
| `impact_timeout_ms` | u64 | `5000` | Timeout for changed-target analysis |
| `compile_concurrency` | usize | `1` | Per-repository Cargo command ownership limit |
| `domain_gates` | HashMap | `{}` | Per-domain custom gate lists |
| `rungs` | Vec\<GateRungConfig\> | `[]` | Custom gate rungs (alias: `custom_rungs`) |

Custom rungs replace the built-in compile/lint/test defaults. Each rung is a `{ name, command, timeout_secs, required, parallel_with }` table. Legacy `[[gate]]` syntax is migrated to `[[gates.rungs]]` by `roko config migrate`.

### 8.10 `[pipeline]` -- execution pipeline per complexity

| Tier | strategist | reviewers | reviewer_mode | max_iterations |
|---|---|---|---|---|
| mechanical | false | false | quick | 1 |
| focused | false | false | quick | 2 |
| integrative | true | true | quick | 2 |
| architectural | true | true | full | 3 |

### 8.11 `[budget]` -- cost limits

| Field | Type | Default | Description |
|---|---|---|---|
| `max_plan_usd` | f32 | `0.0` | Max cost per plan execution; **zero means unlimited** |
| `max_task_usd` | f32 | `0.0` | Base task ceiling; **zero means unlimited** |
| `max_turn_usd` | f32 | `0.0` | Max cost per agent turn; **zero means unlimited** |
| `max_task_retry_usd` | f32 | `0.0` | Max cumulative cost across retry attempts for one task; **zero means unlimited** |
| `prompt_token_budget` | usize | `10_000` | Max prompt tokens |
| `tier_multipliers.mechanical` | f32 | `0.2` | Mechanical/Haiku task multiplier |
| `tier_multipliers.standard` | f32 | `1.0` | Standard/Sonnet task multiplier |
| `tier_multipliers.complex` | f32 | `3.0` | Complex/Opus task multiplier |
| `tier_multipliers.expert` | f32 | `5.0` | Explicit expert/architectural task multiplier |

> **`0.0` = unlimited semantics.** The library contract is that a ceiling of `0.0` means "no cap". The runner does not enforce any spend limit for that dimension. Negative, `NaN`, and `Inf` values are rejected by pre-flight validation. The checked-in `roko.toml` sets `max_plan_usd = 10.0` and `max_turn_usd = 0.50` as explicit operator policy; these are *not* library defaults.

### 8.12 `[conductor]` -- orchestration control

| Field | Type | Default | Description |
|---|---|---|---|
| `max_agents` | usize | `8` | Max concurrent agents |
| `max_parallel_plans` | usize | `1` | Max parallel plan executions |
| `parallel_enabled` | bool | `false` | Enable parallel task execution |
| `express_mode` | bool | `false` | Skip strategist for quick fixes |
| `max_auto_fix_attempts` | u32 | `3` | Auto-fix retries before replan |
| `auto_fix_model` | String | `"claude-haiku-4-5"` | Model for auto-fix attempts |
| `warm_implementers_per_plan` | usize | `1` | Pre-spawned warm agents |

**Live in runner-v2.** Conductor watcher config (`[conductor.watchers.*]`) is
fully live in the runner-v2 event loop. Watchers run every 5 seconds against
the signal ring buffer and can trigger Restart or Fail decisions. When
watchers detect repeated failures on a model or resource pressure (cost,
context window, time), the conductor emits a **routing bias** that feeds
into model selection: deprioritized models are filtered from the candidate
set and `prefer_cheaper` shifts scoring toward cheaper tiers. This routing
bias only applies when no `force_backend` override or task `model_hint` is
set -- operator and author intent always take precedence.

### 8.13 `[learning]` -- learning and feedback

| Field | Type | Default | Description |
|---|---|---|---|
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
| `gate_threshold_flush_interval` | u64 | `10` | Gate observations between adaptive-threshold writes; zero normalizes to one |

### 8.14 `[demurrage]` -- signal decay

| Field | Type | Default | Description |
|---|---|---|---|
| `rate_per_hour` | f64 | `0.01` | Decay rate per hour |
| `min_balance` | f64 | `0.1` | Minimum signal balance |
| `freeze_threshold` | f64 | `0.05` | Balance below which signal freezes |
| `freeze_before_delete` | bool | `true` | Freeze before garbage collection |

### 8.15 `[runner]` -- CoreRunnerConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `plan_timeout_secs` | u64 | `3600` | Wall-clock timeout for the entire plan execution |
| `dangerously_skip_permissions` | bool | `false` | Skip agent permission prompts (rejected in shared config) |
| `sandbox_level` | enum | `restrict` | Sandbox enforcement: `none`, `observe`, `restrict`, `isolate`, `quarantine` |
| `dispatch_max_retries` | u32 | `5` | Max dispatch retry attempts for transient errors |
| `warm_pool_size` | usize | `2` | Pre-spawned warm agent slots per role |
| `warm_pool_idle_timeout_secs` | u64 | `300` | Idle timeout before a warm agent slot is reclaimed |

**Restart required.** Runner configuration is read at plan start; changes take effect on the next plan execution.

### 8.16 `[resources]` -- ResourcesConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `min_free_disk_mb` | u64 | `2048` | Minimum free disk space (MB) to start a plan run |
| `warn_disk_mb` | u64 | `5120` | Warning threshold (MB); execution continues |
| `max_plan_disk_mb` | u64 | `0` | Max disk growth per plan (MB); zero is unlimited |
| `gc_on_plan_start` | bool | `true` | Run filesystem GC before plan execution |
| `gc_on_plan_end` | bool | `true` | Run filesystem GC after plan completion |
| `gc_on_failure` | bool | `true` | Run filesystem GC after plan failure |
| `target_cleanup_enabled` | bool | `true` | Auto-remove stale Rust `target/` directories |
| `target_max_age_days` | u64 | `3` | Max age before a target directory is cleaned |
| `log_rotation_max_mb` | u64 | `100` | Max size of JSONL logs before rotation |
| `worktree_cleanup_on_complete` | bool | `true` | Clean up worktrees on successful plan completion |
| `worktree_cleanup_on_failure` | bool | `true` | Clean up worktrees on plan failure |
| `worktree_max_age_secs` | u64 | `86400` | Max worktree age before forced cleanup |
| `auto_cleanup_on_complete` | bool | `true` | Run full cleanup pass on plan completion |

**Restart required** for disk thresholds; GC policy changes are picked up on the next plan run.

### 8.17 `[prompt]` -- PromptConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `composition_strategy` | enum | `auto` | Prompt budget allocation: `auto`, `density_greedy`, `weighted_sum`, `vcg` |
| `vcg_warmup_observations` | u32 | `10` | Minimum bidder observations before `auto` enables VCG allocation |

**Hot-reloadable.** Changes take effect on the next prompt composition.

### 8.18 `[validation]` -- ValidationConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `strict_validation` | bool | `false` | When true, dangling provider references become hard errors |

**Restart required.** Validation mode is evaluated at config load time.

### 8.19 `[feed_agents]` -- FeedAgentsConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Whether the 10 built-in feed agents are spawned at serve startup |

**Restart required.** Feed agents are spawned once during `roko serve` initialization.

### 8.20 `[[groups]]` -- GroupDefinition

| Field | Type | Default | Description |
|---|---|---|---|
| `name` | String | required | Group name |
| `description` | String | `""` | Human-readable description |
| `coordination` | String | required | Coordination strategy (e.g. `"consensus"`, `"leader"`) |
| `members` | Vec\<String\> | `[]` | Initial member agent names |
| `leader` | Option\<String\> | `None` | Designated group leader |
| `assignment_strategy` | Option\<String\> | `None` | Task assignment Cell or algorithm |
| `public` | bool | `false` | Whether the group is publicly discoverable |
| `max_members` | Option\<usize\> | `None` | Maximum group size |
| `knowledge_policy` | Option\<String\> | `None` | Knowledge sharing policy |
| `pheromone_decay_rate` | Option\<f64\> | `None` | Pheromone signal decay rate |

**Restart required.** Groups are reconciled into the serve runtime at startup.

### 8.21 Additional Sections

| Section | Key Fields | Notes |
|---|---|---|
| `[chain]` | `enabled`, `profile`, `auto_deploy_contracts` | Blockchain integration |
| `[relay]` | `heartbeat_interval_secs` | Relay connection |
| `[cold_storage]` | `enabled`, `max_age_days`, `batch_size`, `interval_secs` | Signal archival |
| `[tui]` | `refresh_rate_ms`, `effects.preset` | Terminal UI |
| `[deploy]` | `backend`, `worker_image` | Cloud deployment |
| `[prd]` | `auto_plan` | PRD lifecycle |
| `[tools]` | `allow`, `deny`, `profiles.<name>` | Tool permissions |
| `[dreams]` | `auto_dream`, `idle_threshold_mins`, `scheduled_cron` | Dream scheduling |
| `[daimon]` | `strategy_space.domain`, `strategy_space.dimensions` | Affect engine |
| `[[subscriptions]]` | `template`, `trigger`, `concurrency_limit`, `cooldown_secs` | Event subscriptions |
| `[[scheduler.cron]]` | `name`, `expression`, `signal_kind` | Scheduled events |
| `[[repos]]` | `name`, `path`, `branch`, `subscriptions` | Per-repository config |
| `[webhooks.github]` | `secret` | GitHub webhook verification |
| `[github]` | `auto_pr`, `default_branch`, `merge_method` | GitHub integration |
| `[gemini]` | `thinking_level`, `safety_settings`, `use_free_tier` | Gemini provider settings |
| `[perplexity]` | `search_recency_filter`, `academic_mode` | Perplexity search tuning |
| `[timeouts]` | `llm_call_secs`, `gate_*_secs`, `plan_total_secs` | Per-subsystem timeouts |

### 8.22 Hot-reload vs restart classification

The following table summarizes which config sections take effect without a process restart and which require re-launching `roko serve` or starting a new plan run.

| Reload behavior | Sections |
|---|---|
| **Hot-reloadable** (takes effect on next operation) | `budget`, `tools`, `learning`, `gates`, `pipeline`, `conductor`, `routing`, `prompt` |
| **Restart required** (read once at startup or plan start) | `agent`, `providers`, `models`, `serve`, `server`, `scheduler`, `watcher`, `profiles`, `runner`, `resources`, `validation`, `feed_agents`, `groups`, `agents`, `deploy`, `chain`, `relay`, `cold_storage`, `dreams`, `daimon`, `repos`, `webhooks`, `github`, `gemini`, `perplexity`, `timeouts` |
| **Detected by `roko doctor`** | `conductor.context_pressure_enabled` (deprecated; parseable for compatibility but runtime-dead) |

> **Environment variable reference.** The canonical inventory of `ROKO_*` and `ROKO__SECTION__FIELD` environment overrides is maintained by #339 in `docs/v2/ENVIRONMENT.md`. This chapter documents TOML sections only.

---

## 9. Domain Profiles as Cognitive Postures

> **Implementation status:** Inheritable config overlays are implemented in `config/schema.rs`, including cycle detection, a five-edge depth limit, field-wise gate merging, an extensible `extra` map, and `coding`, `research`, and `review` built-in definitions. Automatic selection and application of a profile to an agent runtime is not wired yet. The broader clock, extension, wakeup, context-weight, and infrastructure posture described by earlier versions of this chapter remains aspirational.

A domain profile is currently a named, optional overlay for model, effort, context, iteration, tool, and gate settings.

### 9.1 Profile Schema

```rust
pub struct DomainProfile {
    pub name: String,
    pub base: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub context_limit_k: Option<u32>,
    pub max_iterations: Option<u32>,
    pub tool_profile: Option<String>,
    pub gate_config: Option<GateProfileConfig>,
    pub extra: HashMap<String, toml::Value>,
}

pub struct GateProfileConfig {
    pub skip_tests: Option<bool>,
    pub clippy_enabled: Option<bool>,
    pub max_rung: Option<u32>,
}
```

### 9.2 Profile Comparison

`builtin_profiles()` supplies these definitions to callers; `RokoConfig::default()` keeps its `profiles` map empty.

| Profile | Implemented overlay values |
|---|---|
| `coding` | `effort = "high"`, `tool_profile = "full"`, `max_iterations = 3` |
| `research` | `effort = "medium"`, `context_limit_k = 200`, `gate_config.skip_tests = true` |
| `review` | `effort = "low"`, `max_iterations = 1` |

### 9.3 Profile Inheritance

Profiles in `RokoConfig.profiles` extend other profiles via `base`. Resolution follows at most five inheritance edges and rejects cycles or missing parents. Optional scalar values from the child override the parent, gate options merge field by field, and child `extra` entries replace parent entries with the same key.

```toml
[profiles.base-review]
name = "base-review"
effort = "low"
max_iterations = 1

[profiles.security-review]
name = "security-review"
base = "base-review"
model = "claude-opus-4-6"
tool_profile = "security"

[profiles.security-review.gate_config]
skip_tests = false
clippy_enabled = true
```

---

## 10. Config Evolution in L4

> **Implementation status:** `ConfigSource::Evolved`, its priority, and provenance constructors are implemented. The L4 proposal, approval, persistence, and Signal flow below is not wired.

Configuration is intended to be evolvable. In L4 structural adaptation (07-LEARNING.md), the system proposes config changes based on observed outcomes.

```rust
/// L4 config evolution: the system proposes config changes.
///
/// Flow:
/// 1. L4 StructuralAdaptation observes suboptimal parameters.
/// 2. Generates a ConfigProposal Signal.
/// 3. Proposal goes through ConfigVerifyCell.
/// 4. If valid, enters human approval queue.
/// 5. If approved, new Config Signal emitted with source Evolved.
/// 6. Evolved is below file/env/CLI/API inputs but above compiled defaults.
pub struct ConfigProposal {
    pub changes: BTreeMap<String, serde_json::Value>,
    pub rationale: String,
    pub evidence: Vec<ContentHash>,
    pub expected_improvement: String,
}
```

### 10.1 Config Demurrage

> **Implementation status:** A pragmatic freshness layer is implemented. `ConfigFreshness` persists per-section review timestamps at `.roko/state/config-freshness.json`, `touch_changed` updates sections from a config diff, and `config_freshness_diagnostics` warns when a tracked timestamp is older than the default 30-day threshold. `roko doctor` displays those warnings. Only sections present in the freshness file are evaluated; never-tracked sections are silent. Signal balance decay and warning Pulses remain aspirational.

The target Signal architecture would make stale config lose balance and emit a warning Pulse:

```rust
pub fn config_demurrage_check(config_signal: &Signal, now: Instant) -> Option<Signal> {
    let age_days = (now - config_signal.created_at).as_secs_f64() / 86400.0;
    let balance = config_signal.metadata.demurrage_balance;

    if balance < CONFIG_STALE_THRESHOLD && age_days > 30.0 {
        Some(Signal::pulse(
            Kind::Alert,
            topic!("config.stale_warning"),
            ConfigStaleWarning {
                section: extract_section(&config_signal),
                age_days,
                balance,
                recommendation: "Review this config section or re-validate".into(),
            },
        ))
    } else {
        None
    }
}
```

### 10.2 Feedback Loops per Learning Level

| Level | What happens to config |
|---|---|
| **L1** | Adaptive threshold tuning adjusts gate parameters within declared ranges |
| **L2** | CascadeRouter adjusts routing weights based on outcomes; config provides initial values |
| **L3** | Delta consolidation reviews which config values have been overridden most often, proposes permanent changes |
| **L4** | Full config evolution proposals based on KPI trends |

---

## 11. Configuration Hierarchy (Three Layers)

Three layers, deep-merged:

1. **Workspace**: `<workspace>/workspace.toml` or `roko.toml` -- top precedence
2. **User**: `~/.roko/config.toml` -- middle
3. **Built-in defaults** -- bottom

CLI flags override config. Environment variables (`ROKO_*`) override config but are overridden by flags.

### 11.1 Workspace Scoping

Roko supports multi-workspace operation. A single daemon can serve multiple workspaces, each with its own capability grants, knowledge scope, and resource limits.

```toml
# ~/.roko/daemon.toml
[daemon]
port = 6677
workspaces = [
  { path = "/Users/will/dev/nunchi/roko/roko",   name = "roko" },
  { path = "/Users/will/dev/nunchi/dashboard",    name = "dashboard" },
]

[daemon.limits]
max_agents_per_workspace = 20
max_total_agents = 50
max_budget_per_workspace_usd = 100.0
```

### 11.2 Cross-Workspace Knowledge Sharing

Knowledge Signals are scoped to their workspace by default. Cross-workspace sharing is explicit:

```toml
# roko workspace shares coding heuristics
[space.knowledge]
share_with = ["tag:nunchi"]
share_kinds = ["Heuristic", "Insight"]

# dashboard workspace imports from roko
[space.knowledge]
import_from = ["roko"]
import_filter = { min_tier = "Consolidated" }
```

Shared Signals carry their origin workspace tag in CaMeL provenance. The receiving workspace can query but not modify the original.

---

## 12. Secret Management

Secrets are **never stored in roko.toml**. Three mechanisms:

1. **Environment variables**: `api_key_env = "ANTHROPIC_API_KEY"` in provider config
2. **Secrets store**: `roko config secrets set <key> <value>` stores encrypted at `~/.roko/secrets/`
3. **`${VAR}` expansion**: Any string value can reference env vars: `rpc_url = "${ETH_RPC_URL}"`

**Secret rotation**: `roko config secrets rotate <key>` updates the secret and signals roko-serve to reload (hot-swap, no restart required).

---

## 13. Full Working Example

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
gate_threshold_flush_interval = 10

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

---

## 14. Acceptance Criteria

This table describes the chapter-level target. E42 covers the non-Signal migration, resolution/provenance, invariant, pull-reload, profile-overlay, and tracked-section freshness paths. Criteria involving `Kind::Config`, protocol Cells, a filesystem watch trigger, the reload Graph, automatic profile application, or L4 proposals remain open.

| Criterion | Verification |
|---|---|
| Config Signal round-trips through serialize -> parse -> reserialize with content hash preserved | Unit test on representative config |
| ConfigComposeCell merges CLI > Env > TOML > Evolved correctly per field | Multi-source merge test |
| ConfigVerifyCell catches all 7 invariant violations | One negative test per invariant |
| ConfigWatchTrigger fires on roko.toml change, debounces within 500ms | File modification trigger test |
| Config reload Graph: watch -> compose -> verify -> publish pipeline works end-to-end | Integration test with file change |
| Config migration chain: v1 -> v2 preserves all values | Round-trip migration test |
| Schema version mismatch triggers migration automatically | Test with old-version config |
| Malformed TOML refuses to start with line/column error | Parse error test |
| Unknown top-level fields warn but do not fail | Forward compatibility test; nested unknown fields currently remain serde-compatible and silent |
| Missing roko.toml uses defaults and is fully functional | Default config boot test |
| Minimal config (just project.name) produces functional system | Minimal config test |
| Domain profile overlay configures the implemented model/effort/context/tool/gate dimensions | Resolve a named profile and inspect the overlay |
| Profile inheritance: child overrides parent, with field-wise gate and `extra` merging | Profile with `base`, verify merge semantics and cycle/depth rejection |
| Environment variable convention: `ROKO__SECTION__FIELD` maps correctly | Env var resolution test |
| Config freshness: a tracked stale section emits a doctor warning after 30 days | Freshness diagnostic and doctor-display tests |
| L4 config proposal: evolved values remain below file/env/CLI/API inputs | Compare `ConfigSource` priorities; proposal flow remains open |
| Secret management: secrets never appear in config Signal payload | Lineage inspection test |
| Multi-workspace daemon isolates config per workspace | Config isolation test |
