# Provider System, Agent Lifecycle, and Multi-Agent Coordination Audit

> Comprehensive audit of 7 provider adapters, the tool dispatch system, tool loop
> architecture, agent composition operators, multi-agent pools, model routing,
> and the concrete gaps between current state and full self-hosting.

---

## 1. Provider Architecture

### 1.1 Core Trait

Central `ProviderAdapter` trait in `crates/roko-agent/src/provider/mod.rs`:

```rust
trait ProviderAdapter {
    fn kind(&self) -> ProviderKind;
    fn create_agent(&self, provider: &ProviderConfig, model: &ModelProfile, options: &AgentOptions) -> Result<Box<dyn Agent>, AgentCreationError>;
    fn classify_error(&self, status: u16, body: &str) -> ErrorCategory;
}
```

**Entry point:** `create_agent_for_model(config, model_key, options)` resolves the model from
roko.toml, finds the provider adapter, and calls `create_agent()`. This is the canonical way
to create an agent -- all other paths should eventually call this.

### 1.2 Agent Trait

The `Agent` trait at `crates/roko-agent/src/agent.rs` is the async executor interface:

```rust
#[async_trait]
trait Agent: Send + Sync {
    async fn run(&self, prompt: &str, context: &Context) -> AgentResult;
    fn name(&self) -> &str;
    fn model(&self) -> &str;
}
```

`AgentResult` contains the output `Engram` (content-hashed signal), usage metrics, and any
errors. The `Context` carries working directory, environment, tool permissions, and metadata.

### 1.3 Agent Options

`AgentOptions` configures agent creation:
- `name: String` -- agent display name
- `system_prompt: Option<String>` -- system prompt text
- `allowed_tools: Option<Vec<String>>` -- tool allowlist
- `mcp_config: Option<PathBuf>` -- MCP server config path
- `timeout_ms: Option<u64>` -- per-request timeout
- `max_turns: Option<u32>` -- tool loop iteration limit
- `workdir: Option<PathBuf>` -- working directory
- `env: Vec<(String, String)>` -- environment variables
- `resume: Option<String>` -- session ID for resume
- `supports_tools: bool` -- whether to wire tool loop

---

## 2. Provider Implementations (7 Adapters)

### 2.1 Claude CLI (`ClaudeCliAdapter`)

**File:** `crates/roko-agent/src/provider/claude_cli.rs`
**Implementation:** `crates/roko-agent/src/claude_cli_agent.rs`

The most mature adapter. Spawns the `claude` CLI binary as a subprocess with full flag
configuration:

| Flag | Source | Purpose |
|------|--------|---------|
| `--model` | `ModelProfile.slug` | Model selection |
| `--append-system-prompt` | `AgentOptions.system_prompt` | System prompt injection |
| `--tools` | `AgentOptions.allowed_tools` | Tool allowlist |
| `--mcp-config` | `AgentOptions.mcp_config` | MCP server config |
| `--effort` | Effort setting | Reasoning effort |
| `--bare` | `bare_mode` field | No interactive UI |
| `--dangerously-skip-permissions` | Default true in worktrees | Skip permission prompts |
| `--settings` | `build_settings_json()` | Safety hooks |
| `--max-turns` | `OperatingFrequency.turn_limit()` | Turn limit |
| `--output-format stream-json` | Always | Streaming output |

**Stream parsing:** Output is parsed line-by-line as `ClaudeStreamEvent` variants (System,
Assistant, Tool, Result) defined in `crates/roko-acp/src/bridge_events.rs`.

**Safety:** `build_settings_json()` generates PreToolUse hooks blocking destructive git and
filesystem operations. Seven hooks covering checkout, switch, branch rename, push, and
recursive delete.

**Error classification:** Stderr pattern-matching categorizes errors as auth failures, rate
limits, context overflow, model not found, or server errors. This drives the retry policy.

**Usage tracking:** Claude CLI reports `total_cost_usd`, `input_tokens`, `output_tokens`,
`cache_creation_input_tokens`, `cache_read_input_tokens` in the Result event.

**Fallback model:** Default fallback is `claude-haiku-4-5`. If the primary model fails, the
agent retries with the fallback.

**Turn limit:** Configurable via `OperatingFrequency` enum (Theta/Alpha/Beta/Gamma/Delta)
from `roko-core`. Each frequency has a different turn limit appropriate for the operating
mode.

### 2.2 Anthropic API (`AnthropicApiAdapter`)

**File:** `crates/roko-agent/src/provider/anthropic_api.rs`

Direct HTTP adapter for the Anthropic Messages API. Two code paths:

1. **Tool-capable models:** Creates a full tool loop with:
   - `AnthropicMessagesBackend` for HTTP transport
   - `AnthropicTranslator` for message format conversion
   - `ToolLoop` for iterative tool execution
   - `ToolDispatcher` for tool permission and dispatch

2. **Non-tool models:** Simple `ClaudeAgent` (direct HTTP, no tool loop)

**Key features:**
- Posts to `/v1/messages` with native Anthropic tool schema format
- Cache markers via `inject_cache_markers()` for prompt caching
- Provider semaphore support for concurrency limits
- Native `tool_use` / `tool_result` content blocks (no translation overhead)

### 2.3 OpenAI-Compatible (`OpenAiCompatAdapter`)

**File:** `crates/roko-agent/src/provider/openai_compat.rs`
**Backend:** `crates/roko-agent/src/openai_compat_backend.rs`

**The most feature-rich adapter.** Handles OpenRouter, Kimi, DeepSeek, xAI Grok, and any
OpenAI-compatible API.

**Two code paths:**
1. `supports_tools = true` -> Full tool loop with `OpenAiTranslator` + `OpenAiCompatBackend`
2. `supports_tools = false` -> `CodexAgent` (simple prompt-response)

**MCP tool discovery:** If the model supports MCP and an `mcp_config` is provided, discovers
and merges MCP tools into the tool registry before creating the tool loop.

**Kimi-K2.5 special handling:**
- Thinking mode locks temperature and top_p
- `reasoning_content` carried in history
- Different content block structure

**Per-model tool limits:**
- Model-specific `max_tools` from profile
- Temperament-based limits: cautious=64, balanced=128, exploratory=unlimited

**Backend features:**
- `OpenAiCompatLlmBackend` with rate limiting via `ProviderRateLimiter`
- `skip_session_fields` for strict providers (Cerebras rejects unknown fields)
- `disable_parallel_tool_calls` for small models
- `normalize_tool_call_content` for providers that reject empty-string content
- Extra headers and body params from provider config

### 2.4 Cerebras (`CerebrasAdapter`)

**File:** `crates/roko-agent/src/provider/cerebras.rs`

Ultra-fast inference with small models (Llama 3.1 8B, 70B, 4-Scout).

**Key innovation: `StrictOpenAiTranslator`**

Small models need help with tool calling. Cerebras uses three techniques:

1. **Strict schemas:** `strict: true` + `additionalProperties: false` on all tool schemas.
   This enables constrained decoding -- the model can only produce valid JSON.

2. **Few-shot tool-call examples:** 8 messages injected between system prompt and user message
   showing `cargo init` + `write_file` round-trips. Generated by `coding_few_shot_examples()`.

3. **System prompt preamble:** Explicit instruction to use the tool-call interface rather
   than emitting tool calls as text.

**Configuration:**
- Default max iterations: 50 (higher than other providers because small models need more turns)
- Context token limit from `model.context_window`
- Timeout from provider config or 120s default

**Reliability assessment:** Works well for simple coding tasks (bash, file I/O). May struggle
with complex multi-step planning. The strict schemas + few-shot examples are specifically
engineered for small models that can't reliably follow tool-calling conventions.

### 2.5 Gemini (`GeminiAdapter`)

**File:** `crates/roko-agent/src/gemini/adapter.rs`

**Five code paths** (the most complex adapter):

1. **Embedding models** -> `GeminiEmbedAgent`
2. **Grounding/code-exec** -> `GeminiNativeAgent`
3. **`tool_format = "gemini_native"`** -> Native tool loop with `GeminiTranslator` +
   `GeminiNativeBackend`
4. **Default tool-capable** -> OpenAI-compat via Gemini's `/v1beta/openai/v1`
5. **Other** -> `GeminiCompatAgent`

**Native backend:** `GeminiNativeBackend` at `crates/roko-agent/src/tool_loop/backends/gemini_native.rs`
implements direct Gemini API with native `FunctionCall` / `FunctionResponse` parts.

**Cache module:** `crates/roko-agent/src/gemini/cache.rs` implements Gemini's context caching
API via `GeminiCacheClient`.

### 2.6 Cursor ACP (`CursorAcpAdapter`)

**File:** `crates/roko-agent/src/provider/cursor_acp.rs`

HTTP fallback for Cursor's ACP protocol:
- Posts to `/v1/prompt` with `X-Cursor-Protocol: acp/1` header
- No tool loop -- Cursor handles tool use internally
- Maps Cursor's response format back to roko's `AgentResult`

### 2.7 Perplexity (`PerplexityAdapter`)

**File:** `crates/roko-agent/src/perplexity/adapter.rs`

Research-oriented adapter with:
- Search capabilities
- Deep research mode
- Citation extraction
- Used by `roko research` CLI commands

---

## 3. Shared Infrastructure

### 3.1 HTTP Client Pool

`SHARED_HTTP_CLIENT`: Global reqwest client with:
- 10 idle connections per host
- 90s idle timeout
- 30s keepalive
- Connection pooling across all providers

### 3.2 Provider Semaphores

`ProviderSemaphores`: Per-provider concurrency limiting:
- Default: 10 concurrent requests per provider
- Configurable per-provider in roko.toml
- Prevents rate limit storms

### 3.3 Rate Limiter

`ProviderRateLimiter` in `crates/roko-agent/src/rate_limit.rs`:
- Per-provider RPM (requests per minute) enforcement
- Default: 60 RPM
- Token-bucket implementation

### 3.4 Safety Layer

Thread-local `ACTIVE_SAFETY_LAYER` + `ACTIVE_TEMPERAMENT`:
- Safety layer applies tool permission checks
- Temperament affects model selection, tool limits, exploration
- Set before agent dispatch, cleared after

### 3.5 Mock Agent

`ROKO_MOCK_AGENT_REPLY` environment variable -> all calls return `MockAgent` with the
specified reply. Used for testing without real API calls.

---

## 4. Tool Dispatch System

### 4.1 ToolDispatcher (`dispatcher/mod.rs`)

Per-call pipeline:

```
Validate args (JSON schema) -> Resolve ToolDef -> Authorize (role permissions)
-> Resolve handler (HandlerResolver) -> Race handler vs timeout+cancel
-> Truncate oversized results (UTF-8 boundary safe)
```

**Key design:** The dispatcher is stateless. Tool definitions and permissions come from the
`ToolRegistry`. Handler resolution is pluggable via `HandlerResolver` trait. This means the
same dispatcher works for any provider.

### 4.2 Batch Dispatch (`dispatcher/parallel.rs`)

- `Parallel` tools: `futures::future::join_all`
- `Serial` tools: sequential execution (preserves shell state, avoids write-write races)
- Unknown defaults to **serial** (safer)

### 4.3 Supporting Modules

| Module | File | Purpose |
|--------|------|---------|
| Cancel | `cancel.rs` | Cancellation propagation |
| Validate | `validate.rs` | JSON schema validation |
| Truncate | `truncate.rs` | Output truncation (UTF-8 safe) |
| Dedup cache | `dedup_cache.rs` | Identical call deduplication |
| Result cache | `result_cache.rs` | Result caching |
| Hook chain | `hook_chain.rs` | Pre/post execution hooks |
| Timeout | `timeout.rs` | Per-tool timeouts |
| Tool selector | `tool_selector.rs` | Tool subset selection |

---

## 5. Tool Loop (Agentic Tool Use)

### 5.1 Core Loop (`tool_loop/`)

```
Send messages + tool definitions to LLM
-> Parse tool calls from response
-> Dispatch via ToolDispatcher
-> Append results as new messages
-> Repeat until end_turn/stop or max iterations
```

### 5.2 Four Translators

Each translator converts between roko's internal message format and the provider's wire format:

| Translator | Provider | Key Feature | File |
|-----------|----------|-------------|------|
| `AnthropicTranslator` | Anthropic API | Native `tool_use`/`tool_result` blocks | `translate/anthropic.rs` |
| `OpenAiTranslator` | OpenAI-compat | `function` calls in `tool_calls` array | `translate/openai.rs` |
| `StrictOpenAiTranslator` | Cerebras | `strict: true` + `additionalProperties: false` | `translate/openai.rs` |
| `GeminiTranslator` | Gemini | Native Gemini `FunctionCall`/`FunctionResponse` | `gemini/translator.rs` |

### 5.3 Context Management

**Pruning (`prune.rs`):** Drops oldest non-head/tail messages when tokens exceed limit.
Keeps first 2 + last 3 messages. This ensures the system prompt and most recent context
are always preserved.

**Compaction (`compaction.rs`):** Truncates verbose old tool results (>500 chars -> 200 char
preview), preserves `tool_call_id`s for conversation coherence, keeps 2 most recent groups
in full.

**Checkpointing (`checkpoint.rs`):** Serializable state (iteration, tool calls, messages,
session) for crash recovery at `.roko/state/tool-loop-{task_id}.json`.

### 5.4 Built-in Tools (30 total)

16 standard tools + 14 chain-domain tools:

**Standard tools:**

| Tool | Category | Permission | Description |
|------|----------|------------|-------------|
| `read_file` | Read | Read | File contents |
| `write_file` | Write | Write | Write file |
| `edit_file` | Write | Write | Text edit |
| `multi_edit` | Write | Write | Multiple edits |
| `glob` | Read | Read | Pattern matching |
| `grep` | Read | Read | Content search |
| `bash` | Exec | Exec | Shell command |
| `ls` | Read | Read | Directory listing |
| `web_fetch` | Read | Read | HTTP fetch |
| `web_search` | Read | Read | Web search (via `crates/roko-std/src/tool/builtin/web_search.rs`) |
| `notebook_edit` | Write | Write | Jupyter editing |
| `todo_write` | Meta | None | Task tracking |
| `task` | Meta | None | Sub-agent delegation |
| `apply_patch` | Write | Write | Unified diff |
| `run_tests` | Exec | Exec | Test suite |
| `exit_plan_mode` | Meta | None | Exit plan mode |

**Chain-domain tools (14):** Blockchain-specific tools for contract deployment, transaction
signing, balance queries, etc.

### 5.5 Hedged Backend

`crates/roko-agent/src/tool_loop/backends/hedged.rs` implements tail-latency hedging:
- Sends request to primary backend
- If primary doesn't respond within P99 latency, sends duplicate to secondary
- Returns whichever responds first
- Cancels the slower request

This reduces tail latency at the cost of ~10% more API calls.

---

## 6. Agent Resumption and Retry

### 6.1 Retry Policy (`retry.rs`)

AWS-style full-jitter exponential backoff:

| Error Category | Retryable? |
|---------------|-----------|
| RateLimit | Yes |
| Timeout | Yes |
| ServerError | Yes |
| Unknown (first 2 attempts) | Yes |
| AuthFailure | No |
| ContentPolicy | No |
| ContextOverflow | No |
| ModelNotFound | No |

- Provider `retry_after_ms` preferred over computed jitter
- Default: 3 attempts, 1s base delay, 60s max delay

### 6.2 Session Resumption (`session.rs`)

`AgentInvocationSession` captures:
- `invocation_id` -- unique per attempt
- `provider_session_id` -- from provider response
- `backend_id`, `model`, `role` -- identity
- `plan_id`, `task_id` -- scoping
- `prompt_fingerprint` -- BLAKE3 hash of prompt
- `context_fingerprint` -- BLAKE3 hash of context
- `reuse_policy` -- scope (Disabled/Task/Plan/Session), max idle, carryover
- `state` -- Created/InProgress/Completed/Failed/TimedOut/Cancelled

**Fail-closed validation (`validate_resume_request()`):** Backend, model, role, scope, prompt
fingerprint, and context fingerprint must ALL match before resumption. Any mismatch -> reject.

**Resumable states:** InProgress, TimedOut, Cancelled. Completed and Failed are terminal.

### 6.3 Executor Snapshot

Full executor state persisted to `.roko/state/executor.json`. The `--resume` flag re-reads
and continues, skipping completed tasks. The snapshot includes:
- All task states (pending/running/completed/failed)
- Completed task outputs
- Routing decisions made
- Budget consumed

---

## 7. Multi-Agent Coordination

### 7.1 AgentPool (Sequential) -- `pool.rs`

Queue of `AgentTask` for a single `AgentRole`, executed one at a time:

- **Primary + fallback:** If primary agent fails, retries with different model
- **Task lifecycle:** Pending -> Active -> Done/Failed/Cancelled
- **Operations:** `submit()`, `execute_next()`, `execute_all()`, `cancel()`, `drain_completed()`
- **Use case:** Sequential task execution where order matters

### 7.2 MultiAgentPool (Parallel) -- `multi_pool.rs`

Multiple agent instances across roles with full lifecycle management:

**Warm Pool:**
- `pre_spawn_warm(role, count, agent_fn)` -- pre-spawn N warm agents
- `promote_warm()` -- move warm -> active
- `evict_warm(role, max_idle)` -- garbage collect idle agents

**Active Instances:**
- `run_task(task)` -- execute with primary+fallback
- `ensure_active_instance(role, instance, agent_fn)`:
  1. Reuse already-active matching instance
  2. Promote named warm instance
  3. Promote any warm instance for role
  4. Spawn fresh (most expensive)

**Lifecycle:**
- `recycle_terminal_to_warm(id)` -- move Done/Failed back to warm (disabled reuse by default)
- `reap_terminal_active()` -- GC terminal instances
- Per-role concurrency limits (default: 4)
- Kill operations: `kill_all(deadline)`, `kill_plan_agents(plan_id)`, `kill_role(role)`

### 7.3 Warm Reuse Policy System

Prevents accidental context bleed between tasks:

```rust
struct WarmReusePolicy {
    scope: ReuseScope,       // Disabled/Task/Plan/Session
    max_idle: Duration,       // Max time warm before eviction
    fingerprints: Vec<Hash>,  // Context fingerprints for matching
    allow_context_carryover: bool,
}
```

**Validation:** Scope match, fingerprint match, carryover allowed, idle time within max.

**Key safety property:** Recycled agents get `ReuseScope::Disabled` -- cannot be reused via
the checked path. This prevents context bleed from terminated agents.

### 7.4 Agent Composition Operators (`composition.rs`)

Four composition patterns at `crates/roko-agent/src/composition.rs`:

| Pattern | What | Use case |
|---------|------|----------|
| Pipeline | Sequential agent chain, output -> input | Strategy -> Implement -> Review |
| Parallel | Fan-out, merge results | Multiple reviewers |
| Conditional | Route by task properties | Simple vs complex dispatch |
| Mixture-of-Agents | Aggregate N outputs | Best-of-N, voting |

**Merge strategies:**
- `Concatenate` -- join outputs in order
- `Aggregate` -- JSON array of outputs
- `Vote` -- majority vote over normalized text
- `BestOfN` -- heuristic quality selection

**SkillSelector:** Routes tasks by 5 dimensions:
- `TaskCategory` (code, research, docs, etc.)
- `TaskComplexityBand` (trivial, simple, standard, complex)
- `TaskReasoningLevel` (low, medium, high)
- `TaskSpeedPriority` (fast, balanced, thorough)
- `TaskQualityProfile` (draft, production, critical)

---

## 8. Model Routing / CascadeRouter

### 8.1 Three-Stage Cascade

Located at `crates/roko-learn/src/cascade_router.rs`:

| Stage | Observations | Strategy | Characteristics |
|-------|-------------|----------|----------------|
| Static | < 50 | Hardcoded role -> model table | Simple, reliable, no data needed |
| Confidence | 50-200 | Empirical pass rates + confidence interval | Conservative, prefers proven models |
| UCB1 | > 200 | Full LinUCB contextual bandit | Optimal exploration-exploitation |

### 8.2 17-Dimensional Routing Context

The routing context vector includes:

| Dimension | Source | Range |
|-----------|--------|-------|
| Task tier (one-hot, 4 dims) | Task metadata | 0 or 1 |
| Complexity band | Task metadata | 0-3 |
| Iteration count | Pipeline state | 0-N |
| Role hash features (4 dims) | Hash of role name | 0-1 |
| Crate familiarity | Learning data | 0-1 |
| Prior failure flag | Pipeline state | 0 or 1 |
| Conductor load | Runtime metrics | 0-1 |
| Active agents | Runtime metrics | 0-1 |
| Ready queue depth | Runtime metrics | 0-1 |
| Daimon policy | Affect engine | 0-1 |
| Thinking level | Session config | 0-3 |
| Temperament | Session config | 0-3 |
| Previous model | Pipeline state | hash |
| Plan context tokens | Prompt assembly | 0-1 |
| Tier thresholds | Adaptive | 0-1 |

### 8.3 Advanced Features

- **Pareto frontier:** Tracks cost-quality trade-offs across models. Eliminates dominated
  models (higher cost AND lower quality).

- **Shadow model evaluation:** Runs free-tier Gemini in parallel to evaluate model quality
  without additional cost. Results used to calibrate the bandit.

- **Hysteresis:** Prevents model thrashing by requiring a significant quality improvement
  before switching. Minimum improvement threshold from config.

- **Cache affinity bonuses:** Prefers models that have warm caches for the current context.
  Reduces latency for consecutive turns in a session.

- **Temperament-based exploration:** Higher exploration multiplier for exploratory temperament,
  lower for conservative. Controls the UCB1 exploration term.

- **Budget pressure cost factor:** As budget usage increases, the router increasingly favors
  cheaper models. Linear interpolation from budget fraction.

- **Provider health registry:** Integrates with `ProviderSemaphores` to avoid routing to
  providers with high error rates.

- **Override learning (UX34):** When the user manually selects a model via `force_backend`,
  the router records the outcome to learn from manual overrides. **Status: partially wired.**
  The `ForceBackendOverrideRecorder` trait exists on `ModelCallService` but the cascade router
  observation is not always recorded.

---

## 9. Usage Tracking

### 9.1 Two Usage Types

**Legacy `Usage` (from roko-core):**
```rust
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
    cache_read_tokens: u32,
    cache_create_tokens: u32,
    cost_usd: f32,
    wall_ms: u64,
}
```

**Canonical `UsageObservation` (from roko-agent):**
```rust
struct UsageObservation {
    input_tokens: Option<u64>,   // None = unknown, not zero
    output_tokens: Option<u64>,
    cache_creation_tokens: Option<u64>,
    cache_read_tokens: Option<u64>,
    cost_usd: Option<f64>,
    source: UsageSource,         // ProviderReported / Estimated / Unknown
    model: Option<String>,
    wall_ms: u64,
}
```

**Key design decision:** `Option<u64>` vs `u32`. The canonical type uses `Option` so that
unknown values stay unknown rather than collapsing to zero. Zero means "free" -- unknown
means "we don't know." The `UsageSource` enum tracks provenance.

**Conversion:** `From<Usage> for UsageObservation` and `From<UsageObservation> for Usage`
implementations exist with clamping to u32::MAX for overflow.

### 9.2 Cost Table

`crates/roko-agent/src/task_runner.rs` has `CostTable` mapping model slugs to per-token
pricing. Used by `ModelCallService` to compute `cost_usd` from raw token counts when the
provider doesn't report cost directly.

---

## 10. Current Config State

### 10.1 Providers in roko.toml

12 providers configured: anthropic, openai, perplexity, moonshot, zai, zhipu, cerebras,
ollama, gemini, openrouter, claude_cli (plus the cursor_acp adapter).

### 10.2 Default Config

```toml
default_model = "glm51"         # ZhiPu GLM-5.1
default_backend = "zhipu"
bare_mode = true
command = "claude"
mode = "ephemeral"
```

### 10.3 Critical Issue: Config Not Used at Runtime

The user's configured model/backend is loaded, merged, and then **thrown away by most dispatch
paths** due to the auth_detect bypass. Nine fragmented dispatch paths with inconsistent behavior:

| Path | Description | Uses config? |
|------|-------------|-------------|
| 1 | Routing config | Yes |
| 2 | `run.rs` direct dispatch | No (hardcoded "claude-sonnet-4-6") |
| 3 | `dispatch_direct.rs` primary | No (hardcoded "claude-sonnet-4-6-20250514") |
| 4 | `dispatch_direct.rs` fallback | No (hardcoded "gpt-4o") |
| 5 | `auth_detect.rs` | No (hardcoded "claude-sonnet-4-6") |
| 6 | ACP runner | Yes (partial) |
| 7 | Runner v2 | Yes (partial) |
| 8 | Agent sidecar | Yes |
| 9 | orchestrate.rs | Yes |

Only paths 1, 6-9 consistently use config. Paths 2-5 have hardcoded model strings.

### 10.4 Eight Hardcoded Model Strings

| File | Line | Hardcoded Model |
|------|------|----------------|
| `run.rs` | 530 | "claude-sonnet-4-6" |
| `run.rs` | 657 | "llama3.1:8b" |
| `dispatch_direct.rs` | 208 | "claude-sonnet-4-6-20250514" |
| `dispatch_direct.rs` | 291 | "gpt-4o" |
| `auth_detect.rs` | 42 | "claude-sonnet-4-6" |
| `orchestrate.rs` | various | Multiple hardcoded strings |

**Impact:** Changing `default_model` in roko.toml has no effect on most dispatch paths.
The user's model preference is silently ignored.

### 10.5 Inconsistent Max Tokens

| Path | Max Tokens |
|------|-----------|
| dispatch_direct.rs | 8192 |
| anthropic adapter | 4096 |
| gateway | 1024 |
| demo | 512 |

Same model, different max tokens depending on entry point. This causes inconsistent behavior
and can lead to truncated responses.

---

## 11. What's Working Well

### 11.1 The Tool Loop Architecture

The tool loop at `crates/roko-agent/src/tool_loop/` is well-designed:
- Clean separation between LlmBackend, Translator, and ToolDispatcher
- Pluggable backends for different providers
- Context management (pruning, compaction, checkpointing)
- Cancellation support
- Configurable iteration limits

### 11.2 Provider Adapter Pattern

The `ProviderAdapter` trait is the right abstraction. Adding a new provider requires:
1. Implement `ProviderAdapter`
2. Add to the provider registry
3. Add config section to roko.toml

### 11.3 Cerebras Small-Model Engineering

The `StrictOpenAiTranslator` + few-shot examples + preamble pattern is a good template for
making small models work reliably. This pattern should be generalized for other fast/cheap
providers.

### 11.4 Multi-Agent Pool Warm Reuse

The warm pool + reuse policy system is well-engineered. The fail-closed validation (all
fingerprints must match) prevents context bleed.

### 11.5 ModelCallService Centralization

`ModelCallService` at `crates/roko-agent/src/model_call_service.rs` is the right design:
one service handling model resolution, cost tracking, event emission, feedback recording,
caching, budget enforcement, convergence detection, and thinking caps. Getting all dispatch
paths to use this service is the key convergence task.

---

## 12. What's Missing

### 12.1 Unified Dispatch Path

The highest-priority gap. Nine dispatch paths should become one. All model calls should go
through `ModelCallService`. This eliminates:
- Hardcoded model strings (8 instances)
- Inconsistent max tokens (4 values)
- Missing cost tracking (most paths)
- Missing episode recording (most paths)

**Files to modify:**
- `crates/roko-cli/src/run.rs` -- replace direct dispatch with ModelCallService
- `crates/roko-cli/src/dispatch_direct.rs` -- delete or reduce to thin wrapper
- `crates/roko-cli/src/auth_detect.rs` -- remove hardcoded model

### 12.2 Knowledge-Informed Model Routing

The `CascadeRouter` does not query the neuro store for model selection. It should use
historical knowledge about which models performed well for similar tasks.

**Implementation path:**
1. Add `knowledge_store` field to `CascadeRouter` (similar to `ModelCallService`)
2. Query knowledge store for task-similar episodes
3. Use model performance from those episodes as prior for bandit

### 12.3 Provider Health Dashboard

No runtime visibility into provider health. Should expose:
- Per-provider error rates
- Per-provider latency percentiles
- Rate limit utilization
- Cost accumulation

**Where to wire:** `crates/roko-serve/src/routes/providers.rs` already has route stubs.

### 12.4 Agent Memory Across Sessions

Agents currently have no memory across sessions. Each invocation starts fresh. The
`KnowledgeStore` from `roko-neuro` has the infrastructure (tiers, decay, consolidation)
but is only queried at dispatch time, not updated by agent outputs.

**Implementation path:**
1. After each successful agent run, extract key decisions/patterns
2. Store as `KnowledgeEntry` with `KnowledgeKind::Heuristic` or `StrategyFragment`
3. Query and inject at dispatch time (already partially wired in `crates/roko-acp/src/knowledge.rs`)

### 12.5 Streaming Cost Updates

Most providers report cost only at the end of a turn. For long-running agents, cost tracking
is unavailable during execution. The `UsageUpdate` ACP event supports streaming cost, but
most providers don't emit it.

**Workaround:** Use `CostEstimate` from `ModelCallService` to predict cost before execution
and show estimated running total in the UI.

### 12.6 Cold Substrate Archival

Built in `roko-fs` but not instantiated at runtime. No cron trigger for periodic archival
of cold signals. The infrastructure exists; it needs a trigger from the daemon.

**File to modify:** `crates/roko-cli/src/daemon.rs` -- add periodic archival task.

---

## 13. Mega-Parity Runner Lessons Applied to Agent System

### 13.1 No-Build Mode for Agent Tasks

The mega-parity runner's biggest win -- disabling compilation during agent work -- should be
applied to the agent dispatch system. When dispatching implementation agents:
1. Tell the agent not to compile
2. Run gates at phase boundaries only
3. This reduces per-agent wall time by 10x

**Where to implement:** `crates/roko-acp/src/runner.rs` `run_implementer()` should inject
"do not compile" instruction into the system prompt for Express/Standard workflows.

### 13.2 Context Handoff Between Agents

The cumulative context sections that reduced merge conflicts by 40% in the runner should
be formalized as an inter-agent context protocol:
1. Each agent records what files it changed and why
2. The next agent receives a summary of prior changes
3. This prevents duplicate work and conflicting changes

**Where to implement:** The `DispatchKnowledge` struct in `crates/roko-acp/src/knowledge.rs`
already queries knowledge and playbooks. Add a "recent changes" section from the pipeline
state.

### 13.3 Anti-Pattern Checks as Pre-Gates

The grep-based anti-pattern checks from the runner (milliseconds per batch) should run as
pre-gates before the full compile/test cycle. This catches common LLM mistakes instantly
without waiting for cargo:
- Stub functions returning default values
- `block_on` in async context
- Duplicate trait definitions
- Raw `Command::new("claude")` instead of going through provider system

**Where to implement:** Add `AntiPatternGate` to `crates/roko-gate/src/` using the 10
patterns from the runner.

### 13.4 Result Files as Agent Coordination

The `.result` file pattern (simple JSON files on disk) proved more reliable than message
passing or shared memory for inter-agent coordination. The ACP runner should adopt this for
multi-step workflows:
1. Each phase writes a result file with status, outputs, and metadata
2. The runner reads result files to determine next action
3. Manual intervention is possible by editing result files
4. Kill and restart is safe because state is on disk

**Where to implement:** `crates/roko-acp/src/workflow.rs` `WorkflowRun` should persist after
each phase transition.
