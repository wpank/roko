# 18 — Structural Cleanup: Dual Config, ToolDef Extension, Hot Reload, Plugin Points

> **Priority**: 🟡 P1 — Prevents architectural debt from accumulating during the refactor
> **Status**: Not started
> **Depends on**: 02 (provider registry)

## Problem Statement

The provider refactor introduces new config types (`ProviderConfig`, `ModelProfile`) that must live in a codebase with two separate config schemas, a `ToolDef` struct that can't easily accommodate new tool types, no hot-reload mechanism for daemon mode, and no plugin story for external developers.

## Discoveries

### Dual Config
- `roko-cli/src/config.rs` has `Config` (agent, tools, prompt, repos, gates, executor, budget, serve)
- `roko-core/src/config/schema.rs` has `RokoConfig` (project, prd, agent, gates, routing, budget, conductor, watcher, learning, tui, serve, scheduler, webhooks, subscriptions, server, deploy)
- Both have `agent`, `gates`, `budget`, `serve` but with **different field names/types**
- Both loaded independently in daemon; no sync mechanism
- Existing layered loading (`ConfigLayer` + `merge()`) is well-designed — matches Cargo's pattern

### ToolDef
- `ToolDef` is a struct with fixed fields at `roko-core/src/tool/def.rs`
- `ToolCategory` is `#[non_exhaustive]` — forward compatible
- `ToolPermission` is a struct with 5 bool fields — doesn't scale
- `DynamicToolRegistry` in `roko-agent/src/mcp/dynamic_registry.rs` already composes static + MCP tools
- No config-driven tool definition mechanism

### Hot Reload
- Daemon has no SIGHUP handler, no file watch, no config reload
- `DynamicToolRegistry` requires `&mut self` for `add_mcp_tools()`
- MCP config not reloaded without daemon restart

### Plugin Architecture
- rig-rs, genai use compile-time trait extension (no runtime plugins)
- MCP is already an IPC-based plugin protocol (tools, context)
- WASM plugins via Extism are production-proven (moonrepo) but heavy
- For roko, MCP-as-plugin + trait-based compile-time extension is the right blend

---

## A. ToolDef Extension Strategy

### 2P.01 — Add ToolSource field to ToolDef (non-breaking)

**File**: `crates/roko-core/src/tool/def.rs`
**What**: Add a discriminated source field to track where a tool came from:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolSource {
    #[default]
    Builtin,
    Mcp { server: String },
    WebSearch { provider: String, config: serde_json::Value },
    Retrieval { knowledge_id: String },
    Plugin { name: String },
}

pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: ToolSchema,
    pub category: ToolCategory,
    pub permission: ToolPermission,
    pub timeout_ms: u64,
    pub concurrency: ToolConcurrency,
    pub idempotent: bool,
    // NEW — non-breaking, serde(default):
    #[serde(default)]
    pub source: ToolSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}
```

**Context**: All existing serialized ToolDefs deserialize cleanly because `source` defaults to `Builtin` and `metadata` defaults to `None`. The 16 builtin handlers continue to work unchanged. GLM's web_search/retrieval/mcp tools use the new `ToolSource` variants. The `metadata` bag handles future extensibility without struct changes.

**Acceptance**: Existing tool definitions parse unchanged. New tool with `source: WebSearch` serializes correctly.
**Verification**: `cargo test -p roko-core -- tool_def_source`

---

### 2P.02 — Update OpenAiTranslator to render ToolSource-aware tools

**File**: `crates/roko-agent/src/translate/openai.rs`
**What**: When `render_tools()` encounters a tool with `source: WebSearch`, render it as GLM's native format instead of an OpenAI function:

```rust
fn render_tools(&self, tools: &[ToolDef]) -> RenderedTools {
    let mut json_tools = Vec::new();
    for tool in tools {
        match &tool.source {
            ToolSource::Builtin | ToolSource::Mcp { .. } | ToolSource::Plugin { .. } => {
                // Standard OpenAI function format
                json_tools.push(json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters.as_value(),
                    }
                }));
            },
            ToolSource::WebSearch { provider, config } => {
                // GLM native web_search format
                json_tools.push(json!({
                    "type": "web_search",
                    "web_search": config,
                }));
            },
            ToolSource::Retrieval { knowledge_id } => {
                // GLM native retrieval format
                json_tools.push(json!({
                    "type": "retrieval",
                    "retrieval": { "knowledge_id": knowledge_id },
                }));
            },
        }
    }
    RenderedTools::JsonArray(Value::Array(json_tools))
}
```

**Context**: This replaces doc 04 task 2C.04's proposal to make ToolDef an enum. Instead, the existing struct gains a `source` field and the translator renders based on source type.

**Acceptance**: Mix of builtin + web_search tools renders correctly.
**Verification**: `cargo test -p roko-agent -- render_tools_with_source`

---

## B. Config Unification

### 2P.03 — Add providers and models to BOTH config schemas

**File**: `crates/roko-core/src/config/schema.rs` AND `crates/roko-cli/src/config.rs`
**What**: The new `[providers.*]` and `[models.*]` sections must be in both config schemas since both are loaded independently:

```rust
// In roko-core RokoConfig:
pub struct RokoConfig {
    // ... existing fields ...
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub models: HashMap<String, ModelProfile>,
}

// In roko-cli Config (via ConfigLayer):
pub struct ProviderLayer {
    pub kind: Option<String>,
    pub base_url: Option<String>,
    pub api_key_env: Option<String>,
    // ... all fields as Option<T> for layered merge
}
```

The CLI's `ConfigLayer` needs `providers: Option<HashMap<String, ProviderLayer>>` and `models: Option<HashMap<String, ModelProfileLayer>>` for the layered merge to work.

**Context**: This is the biggest integration risk in the whole plan. If only roko-core gets the new types, the CLI's config loading path won't see them. If only roko-cli gets them, the daemon's roko-core path won't.

**Acceptance**: Both `roko config show` (CLI path) and roko-serve's `/api/config` (core path) show providers/models.
**Verification**: `cargo run -p roko-cli -- config show | grep providers`

---

### 2P.04 — Add per-field env var overrides (ROKO__* pattern)

**File**: `crates/roko-cli/src/config.rs`
**What**: Add environment variable overrides that map `ROKO__SECTION__FIELD` to config fields:

```rust
fn apply_env_overrides(config: &mut Config) {
    for (key, value) in std::env::vars() {
        if !key.starts_with("ROKO__") { continue; }
        let path = key[6..].to_lowercase().replace("__", ".");
        // Map "ROKO__AGENT__MODEL" → "agent.model"
        // Map "ROKO__PROVIDERS__ZAI__BASE_URL" → "providers.zai.base_url"
        config.set_by_path(&path, &value);
    }
}
```

**Context**: Current env overrides are file-level only (`ROKO_CONFIG` overrides the whole file). Per-field overrides enable `ROKO__AGENT__MODEL=glm-5.1` without editing TOML, matching Cargo's `CARGO_*` and config-rs's `APP__*` conventions.

**Acceptance**: `ROKO__AGENT__MODEL=glm-5.1 roko run "test"` uses glm-5.1.
**Verification**: `ROKO__AGENT__MODEL=test cargo run -p roko-cli -- config show | grep model`

---

## C. Hot Reload for Daemon Mode

### 2P.05 — Add ArcSwap-based config to daemon AppState

**File**: `crates/roko-serve/src/state.rs`
**What**: Replace `Arc<Config>` with `arc_swap::ArcSwap<Config>` for lock-free config reads:

```rust
use arc_swap::ArcSwap;

pub struct AppState {
    pub config: ArcSwap<RokoConfig>,  // Was: Arc<RokoConfig> or similar
    // ... other fields
}
```

**Deps**: Add `arc-swap` to `roko-serve/Cargo.toml`.

**Context**: `ArcSwap` allows lock-free reads (zero overhead for HTTP request handlers reading config) with atomic swaps for updates. This is the standard Rust pattern for hot-reloadable config in servers.

**Acceptance**: Multiple concurrent HTTP handlers read config without blocking.
**Verification**: `cargo test -p roko-serve -- arcswap_config`

---

### 2P.06 — Add POST /api/config/reload endpoint

**File**: `crates/roko-serve/src/routes/config.rs`
**What**: Endpoint to trigger config reload from disk:

```rust
pub async fn reload_config(State(state): State<AppState>) -> Result<Json<ReloadResponse>> {
    let new_config = load_config(&state.workdir)?;

    // Validate before applying
    let warnings = validate_references(&new_config)?;

    // Atomic swap
    state.config.store(Arc::new(new_config));

    Ok(Json(ReloadResponse {
        success: true,
        warnings,
        timestamp: Utc::now().to_rfc3339(),
    }))
}
```

**Acceptance**: `POST /api/config/reload` reloads config from disk without restart.
**Verification**: `curl -X POST http://localhost:9090/api/config/reload | jq`

---

### 2P.07 — Add SIGHUP handler for daemon config reload

**File**: `crates/roko-cli/src/daemon.rs`
**What**: Handle SIGHUP to trigger config reload (Unix convention):

```rust
use tokio::signal::unix::{signal, SignalKind};

async fn setup_signal_handlers(state: Arc<AppState>) {
    let mut sighup = signal(SignalKind::hangup()).expect("failed to register SIGHUP");

    tokio::spawn(async move {
        loop {
            sighup.recv().await;
            tracing::info!("SIGHUP received, reloading config...");
            match reload_config_from_disk(&state).await {
                Ok(warnings) => {
                    if warnings.is_empty() {
                        tracing::info!("config reloaded successfully");
                    } else {
                        tracing::warn!(warnings = ?warnings, "config reloaded with warnings");
                    }
                },
                Err(e) => {
                    tracing::error!(error = %e, "config reload failed — keeping previous config");
                },
            }
        }
    });
}
```

**Context**: `roko daemon reload` already exists as a CLI command — it sends SIGHUP. This task adds the handler that actually processes it.

**Acceptance**: `kill -HUP $(pidof roko)` reloads config. Invalid config logs error and keeps old config.
**Verification**: `roko daemon start & sleep 2 && roko daemon reload && roko daemon logs -n 5 | grep reload`

---

## D. Extension Points Documentation

### 2P.08 — Document how to add a new provider (zero-code path)

**File**: `examples/adding-a-provider.md` (new)
**What**: Step-by-step guide for adding a new OpenAI-compatible provider with zero code changes:

```markdown
# Adding a New Provider

## Step 1: Add provider to roko.toml

[providers.my_provider]
kind = "openai_compat"
base_url = "https://api.my-provider.com/v1"
api_key_env = "MY_PROVIDER_API_KEY"

## Step 2: Add model(s)

[models.my-model]
provider = "my_provider"
slug = "my-model-v1"
context_window = 128000
supports_tools = true
cost_input_per_m = 1.00
cost_output_per_m = 5.00

## Step 3: Set the API key

export MY_PROVIDER_API_KEY="sk-..."

## Step 4: Verify

roko provider test my_provider
roko model list

## Step 5: (Optional) Set as default

[agent]
default_model = "my-model"
```

**Acceptance**: A developer can follow the guide to add a new provider without reading source code.
**Verification**: Manual walkthrough produces working provider.

---

### 2P.09 — Document how to add a new provider (code path)

**File**: `examples/adding-a-custom-protocol.md` (new)
**What**: Guide for adding a non-OpenAI-compatible provider that requires a new `ProviderKind`:

```markdown
# Adding a Custom Protocol Provider

## When to use this guide
- Your provider uses a non-OpenAI API format (not chat completions)
- Your provider needs a custom request/response translation

## Step 1: Add ProviderKind variant
File: crates/roko-core/src/agent.rs

## Step 2: Implement LlmBackend
File: crates/roko-agent/src/tool_loop/backends/your_backend.rs

## Step 3: Register in create_backend factory
File: crates/roko-agent/src/tool_loop/backends/mod.rs

## Step 4: (Optional) Add a Translator
File: crates/roko-agent/src/translate/your_translator.rs
```

**Acceptance**: Guide covers all 4 steps with code examples.
**Verification**: Manual walkthrough.

---

### 2P.10 — Document how to add custom tools via MCP

**File**: `examples/adding-custom-tools.md` (new)
**What**: Guide for adding custom tools to roko agents via MCP servers:

```markdown
# Adding Custom Tools via MCP

MCP servers provide tools that roko agents can call during execution.

## Step 1: Create an MCP server (any language)
## Step 2: Add to .mcp.json in your project root
## Step 3: Tools are auto-discovered and available to all agents
## Step 4: For HTTP backends, tools are converted to function definitions automatically
```

**Acceptance**: Guide covers the full path from MCP server to agent tool use.
**Verification**: Manual walkthrough with a sample MCP server.

---

## Summary

| Section | Tasks | IDs | What |
|---|---|---|---|
| **A. ToolDef extension** | 2 | 2P.01–2P.02 | Non-breaking ToolSource field + translator update |
| **B. Config unification** | 2 | 2P.03–2P.04 | Both schemas get providers/models + env overrides |
| **C. Hot reload** | 3 | 2P.05–2P.07 | ArcSwap + reload endpoint + SIGHUP handler |
| **D. Extension docs** | 3 | 2P.08–2P.10 | Zero-code, code, and MCP extension guides |
| **Total** | **10** | **2P.01–2P.10** | |

## E. Model-Aware Prompt Adaptation (Phase 3+)

### 2P.11 — Add optional model hint to RoleSystemPromptSpec

**File**: `crates/roko-compose/src/role_prompts.rs`
**What**: Add an optional model slug so the prompt builder can adapt formatting:

```rust
pub struct RoleSystemPromptSpec {
    pub role: AgentRole,
    pub task_context: TaskContext,
    pub tool_allowlist_csv: String,
    // NEW:
    pub model_hint: Option<String>,  // e.g., "glm-5.1", "claude-opus-4-6"
}
```

The builder doesn't need to do anything with this in Phase 1 — just pass it through. In Phase 3+, it can use it for:
- Model-specific instruction phrasing ("Use XML tags" for Claude vs plain text for GLM)
- Different tool description verbosity per model
- Model-specific anti-patterns ("Do not use <think> tags" for non-thinking models)

**Context**: `roko-compose` depends on `roko-core` only. The `model_hint` is a plain string, not a type from `roko-agent`. This keeps the dependency chain clean.

**Acceptance**: `RoleSystemPromptSpec` accepts model_hint. Prompt output is unchanged when hint is None.
**Verification**: `cargo test -p roko-compose -- model_hint_passthrough`

---

### 2P.12 — Add ExecAgent fallback to create_agent_for_model

**File**: `crates/roko-agent/src/provider/mod.rs`
**What**: When no provider matches the model key AND the command is not a known protocol, fall back to ExecAgent:

```rust
pub fn create_agent_for_model(
    config: &RokoConfig,
    model_key: &str,
    options: AgentOptions,
) -> Result<Box<dyn Agent>, AgentCreationError> {
    let resolved = resolve_model(config, model_key);

    if let Some(ref provider_config) = resolved.provider_config {
        let adapter = adapter_for_kind(resolved.provider_kind);
        adapter.create_agent(provider_config, &resolved.profile.unwrap_or_default(), &options)
    } else {
        // Legacy fallback: ExecAgent for unrecognized commands
        tracing::warn!(
            model_key = model_key,
            command = %options.command.as_deref().unwrap_or("unknown"),
            "no provider found — falling back to ExecAgent (no tool support)"
        );
        let mut agent = ExecAgent::new(
            options.command.as_deref().unwrap_or("cat"),
            options.extra_args.clone(),
        ).with_timeout_ms(options.timeout_ms.unwrap_or(120_000));
        Ok(Box::new(agent))
    }
}
```

**Context**: ExecAgent is NOT a ProviderKind. It's a subprocess wrapper with no tool support, no protocol, no streaming. It exists as the absolute last resort for testing (`cat`, `echo`) and legacy CLIs. The warning log ensures developers know they're in fallback mode.

**Acceptance**: Unknown model key with no matching provider produces ExecAgent with warning.
**Verification**: `cargo test -p roko-agent -- exec_agent_fallback`

---

## Supersedes

- Doc 04 task 2C.04 (make ToolDef an enum) → replaced by 2P.01 (add ToolSource field)
- Doc 02 task 2A.04 (add to RokoConfig only) → extended by 2P.03 (add to BOTH configs)
