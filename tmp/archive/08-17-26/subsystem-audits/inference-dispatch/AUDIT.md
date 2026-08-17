# Inference & Provider Dispatch Subsystem Audit

Every path in roko that calls an LLM -- who calls what, how responses are parsed, what
feedback is recorded, and where the duplication lives.

### Architecture Runner Status (2026-04-29)

**ModelCallService exists and implements `ModelCaller`.** Phase 1.1 completed:
- `ModelCallService` (`roko-agent/src/model_call_service.rs`, 2143 LOC) provides single
  dispatch for all workflow-engine inference with cache, budget, convergence detection,
  knowledge-informed routing, and gateway event emission
- Implements `ModelCaller` trait from `roko-core/src/foundation.rs`
- `dispatch_via_model_call_service()` in `dispatch_v2.rs` is the v2 entry point for
  one-shot CLI prompts
- `resolve_effective_model()` in `model_selection.rs` provides unified 6-tier precedence
  chain: CLI override > task model > role config > cascade router > project default >
  built-in default
- Legacy `dispatch_direct.rs` gated behind `#[cfg(feature = "legacy-orchestrate")]`

**Remaining gaps:**
- CascadeRouter not consulted from most live paths (only passed when caller provides it)
- One-shot paths still skip episode/efficiency logging
- ACP bridge still has independent Claude CLI spawn path
- Several specialized call sites still read env vars directly

---

## 1. Provider Registry (7 Backends)

### 1A. Registered Provider Adapters

The provider system lives in `crates/roko-agent/src/provider/` and currently registers
7 backend adapters via `adapter_for_kind()`:

| # | Kind | Adapter | File | Protocol | Auth |
|---|---|---|---|---|---|
| 1 | `AnthropicApi` | `AnthropicApiAdapter` | `provider/anthropic_api.rs` | HTTP POST `/v1/messages` | `x-api-key` header |
| 2 | `ClaudeCli` | `ClaudeCliAdapter` | `provider/claude_cli.rs` | subprocess `claude --output-format stream-json` | CLI session |
| 3 | `OpenAiCompat` | `OpenAiCompatAdapter` | `provider/openai_compat.rs` | HTTP POST `/chat/completions` | `Bearer` header |
| 4 | `CursorAcp` | `CursorAcpAdapter` | `provider/cursor_acp.rs` | HTTP POST `/v1/prompt` (ACP/1) | `x-cursor-api-key` |
| 5 | `PerplexityApi` | `PerplexityApiAdapter` | `provider/openai_compat.rs` (shared) | HTTP POST `/chat/completions` (Sonar ext) | `Bearer` header |
| 6 | `GeminiApi` | `GeminiApiAdapter` | `provider/openai_compat.rs` (shared) | HTTP POST `/chat/completions` (Gemini compat) | `Bearer` header |
| 7 | `CerebrasApi` | `CerebrasAdapter` | `provider/cerebras.rs` | HTTP POST `/chat/completions` (strict mode) | `Bearer` header |

Each adapter implements the `ProviderAdapter` trait:
```rust
pub trait ProviderAdapter: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn create_agent(&self, provider: &ProviderConfig, model: &ModelProfile,
                    options: &AgentOptions) -> Result<Box<dyn Agent>, AgentCreationError>;
    fn classify_error(&self, status: u16, body: &Value) -> ProviderError;
}
```

### 1B. Provider-Specific Features

| Provider | Tools | Streaming | Thinking | Vision | Web Search | Caching | MCP |
|---|---|---|---|---|---|---|---|
| Anthropic API | Yes (blocks) | Yes (SSE) | Yes | Yes | No | Yes (prompt) | No |
| Claude CLI | Yes (--tools) | Yes (stream-json) | Yes (--effort) | No | No | No | Yes (--mcp-config) |
| OpenAI Compat | Yes (functions) | Yes (SSE) | Yes (reasoning) | Depends | Depends | No | No |
| Cerebras | Yes (strict) | Yes (SSE) | No | No | No | No | No |
| Cursor ACP | Yes | Yes (SSE) | No | No | No | No | No |
| Perplexity | No | No | No | No | Yes (Sonar) | No | No |
| Gemini | Yes | Yes | Yes | Yes | Yes (grounding) | No | No |

### 1C. Cerebras Adapter -- Small Model Specialization

The `CerebrasAdapter` (`provider/cerebras.rs`) demonstrates provider-specific tuning:
- Uses `StrictOpenAiTranslator` for constrained JSON decoding (`strict: true`,
  `additionalProperties: false`) on tool schemas
- Injects few-shot tool-call examples (2 complete round-trips) between system prompt
  and user message to teach small models (Llama 3.1 8B) the tool protocol
- Prepends a tool-call instruction preamble to prevent models from emitting tool
  invocations as plain text
- Sets `disable_parallel_tool_calls = true` and `normalize_tool_call_content = true`
- This is the template for how future small-model adapters should work

### 1D. OpenRouter Metadata Helper

`provider/openrouter_meta.rs` provides `fetch_model_metadata()` which queries the
OpenRouter catalog API and converts entries to roko `ModelProfile`. This enables:
- Auto-discovery of model capabilities (tools, thinking, vision, web search)
- Cost discovery (input/output/cache pricing per million tokens)
- Context window and max output token detection
- Architecture metadata (input/output modalities, tokenizer)

This is read-only and does not yet auto-populate config -- callers must explicitly
request a profile and merge it.

---

## 2. Invocation Paths (Every Way We Call An LLM)

### 2A. Provider System Paths (Correct)

| # | File | Entry Point | Through Provider? | Feedback? | Status |
|---|---|---|---|---|---|
| 1 | `provider/mod.rs` | `create_agent_for_model()` | Yes | Via caller | Factory |
| 2 | `model_call_service.rs` | `ModelCallService::call()` | Yes | Yes (full) | Canonical |
| 3 | `dispatch_v2.rs` | `dispatch_via_model_call_service()` | Yes | Yes (via MCS) | v2 entry |
| 4 | `tool_loop/mod.rs` | `ToolLoop::run()` / `run_streaming()` | Yes (via backend) | Via caller | Multi-turn |
| 5 | `dreams/runner.rs` | Dream consolidation | Yes | No | Correct |

### 2B. Claude CLI Subprocess Paths (Mixed)

| # | File | Function | Args | Provider? | Feedback? |
|---|---|---|---|---|---|
| 6 | `runner/agent_stream.rs` | `spawn_agent()` | Via `CliProviderConfig::build_invocation()` | Partial | Episode + efficiency |
| 7 | `dispatch_direct.rs` | `dispatch_claude_cli()` | `--print --output-format stream-json` | No | CostMeter only |
| 8 | `roko-acp/runner.rs` | `run_claude_cli()` | `--print --dangerously-skip-permissions` | No | None |
| 9 | `roko-acp/bridge_events.rs` | `run_claude_cognitive_task()` | `--print --output-format stream-json --model <m>` | No | None |

**What's wrong:** #7 is gated behind `legacy-orchestrate` feature flag (good), but #8 and
#9 still bypass the provider system entirely. #6 uses `CliProviderConfig` which is partial
provider integration (model/args from config, but no CascadeRouter or feedback service).

### 2C. Direct HTTP Paths (Bypassing Provider System)

| # | File | Function | Reads Env? | Provider? | Feedback? |
|---|---|---|---|---|---|
| 10 | `dispatch_direct.rs` | `dispatch_anthropic_api()` | Yes (`ANTHROPIC_API_KEY`) | No | CostMeter |
| 11 | `dispatch_direct.rs` | `dispatch_openai_compat()` | Yes (`OPENAI_API_KEY`) | No | CostMeter |
| 12 | `neuro/episode_completion.rs` | Distillation calls | Yes (`ANTHROPIC_API_KEY`) | No | None |
| 13 | `std/tool/builtin/web_search.rs` | Perplexity search | Yes (`PERPLEXITY_API_KEY`) | No | None |

All gated behind `legacy-orchestrate` except #12 and #13 which remain live bypasses.

### 2D. Dead Code Paths

| # | File | Function | Status | Why Dead |
|---|---|---|---|---|
| 14 | `orchestrate.rs` | `dispatch_agent_with()` | Dead | `PlanRunner` never instantiated |
| 15 | `orchestrate.rs` | `PlanRunner::run_task_plans()` | Dead | No callers from CLI |

The `PlanRunner` in `orchestrate.rs` (21,577 LOC) contains the most sophisticated dispatch
logic (CascadeRouter, 9-layer prompts, HDC fingerprints, daimon affect, conductor
intervention, budget guardrails, anomaly detection) -- all unreachable from any live CLI
path. Some helper functions (`save_snapshot_atomic`) are still imported in tests.

---

## 3. ModelCallService Architecture (2,143 LOC)

### 3A. Core Design

`ModelCallService` (`crates/roko-agent/src/model_call_service.rs`) is the canonical
single-dispatch abstraction. It wraps `create_agent_for_model()` with:

| Feature | Implementation | Status |
|---|---|---|
| Model resolution | `resolve_model()` with router fallback | Wired |
| Cost prediction | `cost_predict()` with `CostTable` | Wired |
| Cost budget | `BudgetCell` with cumulative tracking | Wired |
| L1 response cache | `CacheCell` (128 entries, exact match) | Wired |
| Thinking cap | `ThinkingCapCell` (default 16K tokens) | Wired |
| Convergence detection | `ConvergenceDetectionCell` (5-window, 0.85 sim, 3 trigger) | Wired |
| Gateway events | `GatewayEventWriter` | Wired |
| Feedback recording | `FeedbackSink` for episodes | Wired |
| Knowledge routing | `KnowledgeStoreQuery` adapter | Wired (erased) |
| Force-backend learning | `ForceBackendOverrideRecorder` for cascade router | Wired |
| Fallback models | Auto-derived from workspace config | Wired |
| Provider auto-config | Auto-injects Anthropic/OpenAI providers from env | Wired |

### 3B. Model Resolution Flow

```
ModelCallRequest.model (explicit)
       |
       v  (empty?)
model_router callback  (CascadeRouter or role-based)
       |
       v  (none?)
default_model (from config)
       |
       v
config_for_model()  -- auto-inject provider config if needed
       |
       v
create_agent_for_model()  -- provider adapter factory
       |
       v
Agent::run()  -- concrete backend execution
```

### 3C. Cost Prediction

Before executing, `cost_predict()` estimates cost using:
- Prompt length heuristic: total chars / 4 = estimated input tokens
- Output: `max_tokens` from request or 2048 default
- Price: `CostTable.calculate(model, usage)` using per-model pricing
- Returns `CostEstimate { model, estimated_input_tokens, max_output_tokens, predicted_cost_usd }`

### 3D. Convergence Detection

The `ConvergenceDetectionCell` detects when an agent is producing near-identical outputs:
- Maintains a sliding window of recent output hashes
- Jaccard similarity threshold (default 0.85)
- Triggers after N consecutive similar outputs (default 3)
- Returns early with convergence warning to prevent infinite loops

---

## 4. Model Selection & Routing

### 4A. resolve_effective_model() -- 6-Tier Precedence

`crates/roko-cli/src/model_selection.rs` implements a clean precedence chain:

```
1. CLI override (--model flag)           → SelectionSource::CliOverride
2. Task model hint (from task definition) → SelectionSource::TaskModel
3. Role config (from roko.toml roles)     → SelectionSource::RoleConfig
4. CascadeRouter (LinUCB bandit)          → SelectionSource::CascadeRouter
5. Project default (roko.toml default)    → SelectionSource::ProjectDefault
6. Built-in fallback                      → SelectionSource::BuiltInDefault
```

The function returns `EffectiveModelSelection` with full provenance:
- `requested_model`: what was originally asked for
- `effective_model_key`: resolved config key
- `backend_slug`: concrete slug sent to provider
- `provider_key` / `provider_kind`: which provider handles it
- `source`: which precedence tier won
- `reason`: human-readable explanation

### 4B. CascadeRouter (LinUCB Bandit)

`crates/roko-learn/src/cascade_router.rs` implements a multi-stage routing pipeline:

| Stage | Strategy | What It Does |
|---|---|---|
| 1 | Role table | Static role -> model mapping (e.g. architect -> opus) |
| 2 | Confidence | Per-model stats (mean reward, variance, trial count) |
| 3 | LinUCB | 17-dimensional contextual bandit with UCB exploration |
| 4 | Pareto frontier | Down-weight dominated models (cost vs quality) |

Features:
- Persists to `.roko/learn/cascade-router.json`
- Per-model reward tracking with Bayesian updates
- Task requirement scoring (complexity, domain, category)
- Cost spike detection and model filtering
- Free-tier shadow runner for evaluation (optional Gemini)
- Routing decision explanation for observability

**Live callers:** `resolve_effective_model()` accepts `Option<&CascadeRouter>`, but most
callers pass `None`. Only `resolve_effective_model_key()` convenience wrapper is called from
CLI commands, and it always passes `None` for cascade_router. The dead `PlanRunner` in
orchestrate.rs was the only path that constructed and passed a CascadeRouter.

### 4C. Model Resolution in Provider System

`roko_core::agent::resolve_model(config, model_key)` returns a `ResolvedModel`:
- `model_key`: canonical config key
- `slug`: concrete backend slug (e.g. "claude-sonnet-4-6-20250514")
- `profile`: optional `ModelProfile` reference from config
- `provider_kind`: inferred `ProviderKind` from model name patterns

Pattern matching for kind inference:
- `claude-*` -> `ClaudeCli` (or `AnthropicApi` if profile says so)
- `gpt-*`, `o1-*`, `o3-*` -> `OpenAiCompat`
- `gemini-*` -> `GeminiApi`
- `sonar-*`, `perplexity-*` -> `PerplexityApi`
- `cursor-*` -> `CursorAcp`
- Explicit profile overrides all pattern matching

---

## 5. Response Parsing

### 5A. Claude CLI stream-json Parsing (Duplicated)

There are still **4 copies** of the stream-json parsing logic:

| # | File:Function | What It Does | Truncation? |
|---|---|---|---|
| 1 | `dispatch_direct.rs:dispatch_claude_cli()` | Parses tool/result/assistant events | 4096 + char_boundary |
| 2 | `translate/mod.rs:extract_text()` | Parses same events, inline format | 4096 + char_boundary |
| 3 | `translate/mod.rs:extract_tool_outputs()` | Returns `Vec<(Option<String>, String)>` | 4096 + char_boundary |
| 4 | `chat.rs:extract_clean_text()` | JSONL multi-line parser (10 formats) | 4096 + char_boundary |

**Mitigation:** #1 is gated behind `legacy-orchestrate` feature flag, reducing live
duplication to #2, #3, #4.

### 5B. Canonical Stream Parser

`provider/claude_cli/stream.rs` exports a proper typed parser:
```rust
pub enum ClaudeStreamEvent {
    System(ClaudeSystemEvent),
    Assistant(ClaudeAssistantEvent),
    Tool(ClaudeToolEvent),
    Result(ClaudeResultEvent),
    ContentBlockDelta { delta: ClaudeContentBlock },
    Unknown(Value),
}

pub fn parse_stream_line(line: &str) -> Option<ClaudeStreamEvent>;
```

This should be the ONE parser all paths use. The remaining duplicates should call through
this parser.

### 5C. OpenAI-Compatible Response Parsing

The `OpenAiCompatLlmBackend` handles both synchronous and streaming responses:
- Synchronous: `send_turn()` returns `BackendResponse::Json(Value)`
- Streaming: `send_turn_streaming()` accumulates SSE events via `StreamAccumulator`,
  emits `StreamChunk` events through `mpsc::UnboundedSender`, then synthesizes a final
  JSON response

The `StreamAccumulator` properly handles:
- Content deltas (`choices[0].delta.content`)
- Tool call deltas (`choices[0].delta.tool_calls`)
- Reasoning content (`choices[0].delta.reasoning_content`)
- Usage chunks (prompt/completion token counts)
- Finish reasons (stop, tool_calls, length, content_filter)
- Session/thread/conversation IDs from response metadata

---

## 6. Feedback & Learning Recording

### 6A. What Gets Recorded Where

| Signal | Written To | Written By | Live Callers |
|---|---|---|---|
| Episode (full turn) | `.roko/episodes.jsonl` | `persist::append_jsonl()` | `runner/event_loop.rs` |
| Efficiency event | `.roko/learn/efficiency.jsonl` | `persist::append_jsonl()` | `runner/event_loop.rs` |
| Gateway event | `.roko/learn/gateway.jsonl` | `GatewayEventWriter` | `ModelCallService` |
| Cost record | `.roko/learn/costs.jsonl` | `CostsLog` | Dead (orchestrate.rs only) |
| Routing decision | `.roko/learn/routing-log.jsonl` | `RoutingDecisionLogStore` | Dead (orchestrate.rs only) |
| CostMeter | In-memory only | `chat_inline.rs` | `roko` / `roko "prompt"` |
| Gate verdict | Episode attachment | `run_gate_pipeline()` | `roko run`, `roko plan run` |
| Feedback events | `FeedbackSink` | `ModelCallService` | v2 dispatch path |

### 6B. FeedbackService Integration

`roko-learn/src/feedback_service.rs` provides a `FeedbackService` implementing
`FeedbackSink`. It records:
- `FeedbackEvent::ModelCall` -- per-call telemetry with model, provider, tokens, cost,
  latency, success flag, prompt section IDs, knowledge IDs
- Persists to `.roko/learn/` directory
- `ModelCallService` automatically records feedback when a sink is configured

### 6C. UsageObservation Telemetry

`roko-agent/src/usage.rs` defines `UsageObservation` as the canonical telemetry shape:
```rust
pub struct UsageObservation {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_creation_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
    pub source: UsageSource,  // ProviderReported | Estimated | Unknown
    pub model: Option<String>,
    pub wall_ms: u64,
}
```

Optional fields distinguish "not reported" from zero -- important for cost accounting.

---

## 7. Prompt Assembly

### 7A. Where Prompts Are Built

| Path | How | SystemPromptBuilder? | Templates? | Knowledge? |
|---|---|---|---|---|
| `ModelCallService` | `req.system` field passed through | No (caller decides) | No | Via knowledge routing |
| `dispatch_v2` | None (bare prompt) | No | No | No |
| `roko run` | `build_role_system_prompt_validated()` | Yes (9-layer) | Yes | Playbooks + skills |
| `roko plan run` (runner) | `--append-system-prompt` via task config | Partial | Partial | No |
| `roko acp runner.rs` | None (bare) | No | No | No |
| `roko acp bridge_events.rs` | `--system-prompt` from caller | No | No | No |
| orchestrate.rs `PlanRunner` (dead) | Full 9-layer + VCG + bidders + affect | Yes | Yes | Full |

### 7B. Template System

`roko-compose/src/templates/` has role templates:
- Strategist, Implementer, Reviewer, Architect, Auditor, Scribe, Critic
- Quick reviewer, researcher, dream reviewer

`roko-compose/src/system_prompt_builder.rs` has the 9-layer assembly:
1. Role definition (from template)
2. Convention rules
3. Domain context
4. Task description
5. Gate feedback (prior failures)
6. Tool descriptions
7. Skill library
8. Anti-patterns
9. Affect state

Only `roko run` uses the full builder from live code.

---

## 8. Anti-Patterns Remaining

| Anti-Pattern | Where | Severity | Fix Path |
|---|---|---|---|
| Direct env key reads | `episode_completion.rs`, `web_search.rs` | Medium | Route through provider adapters |
| 4 stream-json parsers | `dispatch_direct.rs`, `translate/`, `chat.rs` | Medium | Consolidate to `ClaudeStreamParser` |
| `extract_clean_text()` 130-line monster | `chat.rs:386-515` | Medium | Replace with typed deserialization |
| CascadeRouter zero live callers | `model_selection.rs` passes None | High | Thread router through all paths |
| orchestrate.rs God object | 21,577 LOC, dead `PlanRunner` | High | Extract live helpers, delete dead code |
| ACP bare subprocess spawns | `runner.rs`, `bridge_events.rs` | Medium | Route through `ClaudeCliAdapter` |

---

## 9. Grep Gates (Acceptance Criteria)

After full ModelCallService adoption, these should return zero results outside tests:

```bash
# No more bare claude spawns outside provider adapters
rg 'Command::new\("claude"\)' crates/ --type rust | grep -v test | grep -v 'provider/\|adapter'

# No more direct env key reads for providers outside provider adapters
rg 'std::env::var.*API_KEY' crates/ --type rust | grep -v test | grep -v 'provider/\|adapter'

# No more extract_clean_text (replaced by typed parsing)
rg 'extract_clean_text' crates/ --type rust | grep -v test

# No more inline 4096 truncation (one utility function)
rg '4096' crates/ --type rust | grep -v test | wc -l  # should be <= 1

# All one-shot dispatch paths go through ModelCallService
rg 'dispatch_direct::dispatch_prompt\|chat_inline::run_unified_inline' crates/ --type rust \
  | grep -v test  # should be 0
```

---

## 10. File Inventory

### Provider System (Correct Layer)

| File | LOC | What |
|---|---|---|
| `roko-agent/src/provider/mod.rs` | 1,148 | Provider adapter registry, `create_agent_for_model()` |
| `roko-agent/src/provider/anthropic_api.rs` | 441 | Anthropic Messages API adapter |
| `roko-agent/src/provider/claude_cli.rs` | 407 | Claude CLI subprocess adapter |
| `roko-agent/src/provider/openai_compat.rs` | ~1,300 | OpenAI/Perplexity/Gemini adapter |
| `roko-agent/src/provider/cerebras.rs` | 198 | Cerebras inference adapter (strict mode) |
| `roko-agent/src/provider/cursor_acp.rs` | 258 | Cursor ACP adapter |
| `roko-agent/src/provider/openrouter_meta.rs` | 388 | OpenRouter catalog metadata helper |
| `roko-agent/src/model_call_service.rs` | 2,143 | Canonical ModelCallService |
| `roko-agent/src/openai_compat_backend.rs` | 1,120 | OpenAI-compat LlmBackend impl |
| `roko-agent/src/tool_loop/mod.rs` | 1,837 | Multi-turn tool loop |
| `roko-agent/src/usage.rs` | 74 | UsageObservation telemetry type |

### CLI Dispatch Layer

| File | LOC | What | Live? |
|---|---|---|---|
| `roko-cli/src/model_selection.rs` | 581 | 6-tier model selection | Yes |
| `roko-cli/src/dispatch_v2.rs` | 946 | v2 dispatch, `CliProviderConfig` | Yes |
| `roko-cli/src/dispatch_direct.rs` | 404 | Legacy one-shot dispatch | Gated |
| `roko-cli/src/orchestrate.rs` | 21,577 | Dead PlanRunner God object | Dead |
| `roko-cli/src/run.rs` | 1,555 | `roko run` universal loop | Yes |

### Bypass Paths (Need Migration)

| File | LOC | What | Bypass Type |
|---|---|---|---|
| `roko-acp/src/runner.rs` | 969 | ACP bare subprocess | No provider |
| `roko-acp/src/bridge_events.rs` | 1,855 | ACP event bridge | Mixed |
| `roko-neuro/src/episode_completion.rs` | ~200 | Direct env key read | No provider |
| `roko-std/src/tool/builtin/web_search.rs` | ~200 | Direct env key read | No provider |

---

## Sources

All paths verified against actual source files as of 2026-04-29:

| File | Purpose |
|---|---|
| `crates/roko-agent/src/provider/mod.rs` | Provider adapter registry (1,148 LOC) |
| `crates/roko-agent/src/provider/anthropic_api.rs` | Anthropic API adapter (441 LOC) |
| `crates/roko-agent/src/provider/claude_cli.rs` | Claude CLI adapter (407 LOC) |
| `crates/roko-agent/src/provider/openai_compat.rs` | OpenAI-compat adapter (~1,300 LOC) |
| `crates/roko-agent/src/provider/cerebras.rs` | Cerebras adapter (198 LOC) |
| `crates/roko-agent/src/provider/cursor_acp.rs` | Cursor ACP adapter (258 LOC) |
| `crates/roko-agent/src/provider/openrouter_meta.rs` | OpenRouter metadata (388 LOC) |
| `crates/roko-agent/src/model_call_service.rs` | ModelCallService (2,143 LOC) |
| `crates/roko-agent/src/openai_compat_backend.rs` | OpenAI-compat backend (1,120 LOC) |
| `crates/roko-agent/src/usage.rs` | Usage telemetry types (74 LOC) |
| `crates/roko-cli/src/model_selection.rs` | Model selection precedence (581 LOC) |
| `crates/roko-cli/src/dispatch_v2.rs` | v2 dispatch entry point (946 LOC) |
| `crates/roko-cli/src/dispatch_direct.rs` | Legacy dispatch (404 LOC, feature-gated) |
| `crates/roko-core/src/agent.rs` | ProviderKind enum, resolve_model() |
| `crates/roko-learn/src/cascade_router.rs` | CascadeRouter LinUCB bandit |
