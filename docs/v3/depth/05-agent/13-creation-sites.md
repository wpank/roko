# 05-13 -- Agent Creation Sites

> **Implementation status (2026-09):** The `create_agent_for_model`
> factory function is the canonical agent construction path. Six
> production call sites route through it. Two intentional exceptions
> remain: test code and known-protocol subprocess fallbacks.

---

## The Problem (Historical)

Agent construction was originally split between a shared factory and a
handful of specialized fallbacks. Each call site manually matched on the
command string to decide whether to construct a `ClaudeCliAgent`,
`ExecAgent`, or `OllamaAgent`. This meant:

1. **Inconsistent behavior** -- direct paths could miss shared defaults,
   options, or safety settings.
2. **Hard to add providers** -- new backends required changes in every
   call site.
3. **No single point for routing** -- the CascadeRouter could only
   intercept model selection on the factory path.

The refactoring PRD identified "8 creation sites" as a Tier 1 priority
for consolidation.

---

## The Factory Function

All production agent construction now flows through
`create_agent_for_model` in `crates/roko-agent/src/provider/mod.rs`:

```rust
pub fn create_agent_for_model(
    config: &RokoConfig,
    model_key: &str,
    options: AgentOptions,
) -> Result<Box<dyn Agent>, AgentCreationError>
```

The function:

1. Resolves the model via `resolve_model(config, model_key)`.
2. Looks up the provider via the model's `provider` field.
3. Determines the `ProviderKind` from the provider config.
4. Calls `adapter_for_kind(kind)` to get the static adapter.
5. Invokes `adapter.create_agent(provider, profile, options)`.
6. Returns a configured `Box<dyn Agent>`.

---

## The Adapter Dispatch Table

All 12 provider adapters are registered as static references:

```rust
static ANTHROPIC_API_ADAPTER: AnthropicApiAdapter = AnthropicApiAdapter;
static CEREBRAS_ADAPTER: CerebrasAdapter = CerebrasAdapter;
static CLAUDE_CLI_ADAPTER: ClaudeCliAdapter = ClaudeCliAdapter;
static CODEX_CLI_ADAPTER: CodexCliAdapter = CodexCliAdapter;
static CURSOR_ACP_ADAPTER: CursorAcpAdapter = CursorAcpAdapter;
static CURSOR_CLI_ADAPTER: CursorCliAdapter = CursorCliAdapter;
static GEMINI_CLI_ADAPTER: GeminiCliAdapter = GeminiCliAdapter;
static HERMES_ADAPTER: HermesProviderAdapter = HermesProviderAdapter;
static OPENAI_COMPAT_ADAPTER: OpenAiCompatAdapter = OpenAiCompatAdapter;
static OPENCLAW_ADAPTER: OpenClawProviderAdapter = OpenClawProviderAdapter;
static PERPLEXITY_ADAPTER: PerplexityAdapter = PerplexityAdapter;
static GEMINI_ADAPTER: GeminiAdapter = GeminiAdapter;
```

The `adapter_for_kind` function uses an exhaustive `match` on
`ProviderKind`, ensuring the compiler catches any unregistered variant.

---

## The Call Sites

### 1. Plan runner (`runner/event_loop.rs`)

The primary agent call site for plan execution. Constructs agents for
each task in the plan DAG.

**Status:** Migrated. Uses `create_agent_for_model` for all routed and
no-routing paths. Only known-protocol subprocess commands (Claude CLI,
Codex CLI) retain manual construction when no routing config is present,
preserving their existing behavior.

### 2. `run.rs` -- single-prompt execution

The `roko run "<prompt>"` command constructs an agent for one-shot
execution.

**Status:** Migrated for routed and no-routing paths. Uses the shared
factory.

### 3. `plan_generate/` -- plan generation

`roko plan generate` and the plan-writing path of `roko run` (`--plan`, or a
prompt sized as standard or complex) construct an agent that writes the plan.

**Status:** Partially migrated. The main path uses
`create_agent_for_model`.

### 4. `research.rs` -- research agent

The `roko research` commands construct agents for deep research tasks.
Gemini grounding and Perplexity search-grounded paths now use the
shared factory.

**Status:** Partially migrated. Specialty endpoints such as Perplexity
deep research still diverge.

### 5. `agent_exec.rs` -- agent execution helper

Internal helper for background tasks.

**Status:** Migrated. Background task creation goes through
`create_agent_for_model`.

### 6. Agent sidecar (`roko-agent-server`)

The per-agent HTTP sidecar constructs agents for real LLM dispatch.

**Status:** Migrated. Uses `create_agent_for_model` for the dispatch
path (T9).

### 7. Test code

Tests construct agents directly (MockAgent, ExecAgent) for specific
test scenarios.

**Status:** Intentionally exempt. Tests should construct specific agent
types directly for determinism.

### 8. Examples and benchmarks

Example code and benchmark harnesses construct agents directly.

**Status:** Acceptable for examples. Benchmarks use
`create_agent_for_model` to exercise the full pipeline.

---

## Target Architecture

After consolidation, agent construction follows one path:

```
Call site (any production path)
    |
    v
create_agent_for_model(config, model_key, options)
    |
    +-- resolve_model(config, model_key) -> ResolvedModel
    |   +-- Config registry lookup
    |   +-- Fallback to slug heuristic
    |
    +-- CascadeRouter may override model_key -> different tier
    |
    +-- adapter_for_kind(provider_kind) -> &dyn ProviderAdapter
    |
    +-- adapter.create_agent(provider, profile, options)
            -> Box<dyn Agent>
```

### Benefits

1. **One place to add providers** -- new providers are registered in the
   adapter dispatch table and config, not in call sites.
2. **CascadeRouter intercepts all model selection** -- the router can
   override any model choice, enabling tier routing.
3. **Consistent configuration** -- all agents get the same treatment:
   timeout, system prompt, tools, MCP, safety.
4. **Easy auditing** -- one function to review for security and
   correctness.

---

## Model Resolution

The two-phase resolution in `resolve_model` bridges the config-driven
and heuristic worlds:

```rust
pub fn resolve_model(config: &RokoConfig, model_key: &str)
    -> Option<ResolvedModel>
{
    // Phase 1: Try the config registry
    if let Some(profile) = config.models.get(model_key) {
        let provider = config.providers.get(&profile.provider)?;
        return Some(ResolvedModel { profile, provider });
    }

    // Phase 2: Fall back to slug heuristic
    AgentBackend::from_model(model_key).map(|backend| {
        // Construct a synthetic profile from the heuristic
        // ...
    })
}
```

This means bare model slugs like `"claude-opus-4-6"` continue to work
via heuristic, while `[providers.*]` and `[models.*]` entries give full
control.

---

## Shared HTTP Client

All HTTP-based provider adapters share a process-wide `reqwest::Client`
via the `SHARED_HTTP_CLIENT` lazy static:

```rust
static SHARED_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .user_agent(concat!("roko-agent/", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});
```

This keeps TCP and TLS connections warm across all provider adapters,
avoiding redundant handshakes when new backends are constructed for the
same process.

---

## Provider Rate Limiting

The `ProviderRateLimiter` manages per-provider concurrency via
`tokio::sync::Semaphore`. The `max_concurrent` field in
`ProviderConfig` sets the semaphore permit count. The default is
`DEFAULT_PROVIDER_MAX_CONCURRENT` from `roko-core::defaults`.

---

## Immune Boundary Wrapping

After construction, production agents are wrapped through the immune
boundary via `wrap_provider_agent` from
`crates/roko-agent/src/immune_boundary.rs`. This ensures all provider
output traverses the five-stage immune Graph before reaching the
calling code.

---

## Implementation Sources

| File | Purpose |
|------|---------|
| `crates/roko-agent/src/provider/mod.rs` | `create_agent_for_model`, adapter dispatch table, shared HTTP client |
| `crates/roko-core/src/agent.rs` | `resolve_model`, ProviderKind, AgentBackend |
| `crates/roko-agent/src/immune_boundary.rs` | `wrap_provider_agent` |
| `crates/roko-cli/src/runner/event_loop.rs` | Primary plan runner call site |

---

## Citations

1. `crates/roko-agent/src/provider/mod.rs` -- Factory function and
   adapter dispatch table.
2. `crates/roko-core/src/agent.rs` -- Model resolution and ProviderKind.
3. `crates/roko-agent/src/immune_boundary.rs` -- Immune boundary
   wrapping for provider agents.
