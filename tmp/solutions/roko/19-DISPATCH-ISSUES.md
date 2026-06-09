# Inference & Dispatch: Issues

Catalogued issues in the inference dispatch subsystem, ordered by severity.

---

## Critical Issues (Block Self-Hosting Quality)

### ISS-01: CascadeRouter Has Zero Live Callers

**Severity:** Critical
**Files:** `crates/roko-cli/src/model_selection.rs`, `crates/roko-learn/src/cascade_router.rs`
**Status:** Open

The CascadeRouter is a sophisticated LinUCB contextual bandit with 4-stage routing
(role table, confidence stats, LinUCB, Pareto frontier), persistence, cost spike
detection, and knowledge-informed routing. It is fully built and persists to
`.roko/learn/cascade-router.json`.

However, it has zero live callers:
- `resolve_effective_model()` accepts `Option<&CascadeRouter>` but every caller
  passes `None`
- `resolve_effective_model_key()` (convenience wrapper used by CLI commands) hard-codes
  `None` for the cascade_router parameter
- The dead `PlanRunner` in `orchestrate.rs` was the only code that constructed and
  used a CascadeRouter

**Impact:** All model selection is static -- either CLI override, role config, or project
default. No adaptive routing, no learning from success/failure, no cost optimization.
The most expensive feature in the codebase provides zero value at runtime.

**Fix:** Load CascadeRouter at startup in `roko plan run`, `roko run`, and `roko chat`.
Pass it through `resolve_effective_model()`. After each call, record the observation.
Persist on shutdown.

---

### ISS-02: One-Shot Paths Skip All Feedback Recording

**Severity:** Critical
**Files:** `crates/roko-cli/src/dispatch_direct.rs`, `crates/roko-cli/src/chat_inline.rs`
**Status:** Partially fixed (v2 path has feedback)

The one-shot paths (`roko "prompt"`, `roko chat`) write no durable feedback:
- No episodes to `.roko/episodes.jsonl`
- No efficiency events to `.roko/learn/efficiency.jsonl`
- No routing decisions to `.roko/learn/routing-log.jsonl`
- `CostMeter` is in-memory only, lost on exit

The v2 dispatch path (`dispatch_via_model_call_service()`) does record feedback via
`FeedbackSink`, but the legacy paths do not.

**Impact:** Interactive use generates no learning data. CascadeRouter can never learn
from the most common user interaction pattern. Cost tracking for interactive sessions
is invisible.

**Fix:** Ensure all CLI entry points route through `ModelCallService` (which records
feedback automatically). The `dispatch_direct.rs` legacy path is already feature-gated
behind `legacy-orchestrate`, but the migration of `chat_inline.rs` is incomplete.

---

### ISS-03: orchestrate.rs God Object (21,577 LOC)

**Severity:** Critical
**Files:** `crates/roko-cli/src/orchestrate.rs`
**Status:** Open (dead code, blocking cleanup)

The `orchestrate.rs` file is 21,577 lines containing:
- `PlanRunner` struct with the most sophisticated dispatch logic in the codebase
- CascadeRouter integration, 9-layer prompt assembly, HDC fingerprints, daimon affect,
  conductor intervention, budget guardrails, anomaly detection
- `dispatch_agent_with()` at line 13,906
- `run_task_plans()` at line 7,514
- `save_snapshot_atomic()` (live, used in tests)
- 50+ helper functions for gate failure replan, enrichment, context bidding

`PlanRunner` itself is dead code -- never instantiated from any CLI entry point. The
actual `roko plan run` uses `runner/event_loop.rs` which is a completely separate
implementation.

**Impact:** The best dispatch logic is unreachable. The file is too large to maintain.
Valuable patterns (cascade routing, escalation, knowledge-informed routing) are
trapped in dead code rather than available in live paths.

**Fix:** Extract the valuable patterns into proper modules:
1. CascadeRouter integration -> already in `model_selection.rs` (just needs wiring)
2. Prompt assembly -> already in `roko-compose` (just needs calling)
3. Gate failure replan -> extract to `roko-gate` or `roko-orchestrator`
4. Budget/anomaly/conductor -> extract to `roko-runtime`
5. Delete the dead `PlanRunner` struct and unused helpers

---

### ISS-04: ACP Bridge Bypasses Provider System

**Severity:** High
**Files:** `crates/roko-acp/src/runner.rs`, `crates/roko-acp/src/bridge_events.rs`
**Status:** Open

Two ACP paths bypass the provider system entirely:

1. `runner.rs:run_claude_cli()` -- spawns `claude --print --dangerously-skip-permissions`
   with no model flag, no streaming, no system prompt, no feedback. This is the most
   bare-bones invocation in the codebase.

2. `bridge_events.rs:run_claude_cognitive_task()` -- spawns `claude --print
   --output-format stream-json --model <m> --system-prompt <sp>` as a direct subprocess.
   Better than #1 (has model and system prompt) but still bypasses provider adapters,
   credential resolution, cost tracking, and feedback.

The OpenAI-compat path in `bridge_events.rs` (`run_openai_compat_cognitive_task()`)
is better -- it uses `resolve_model()` from RokoConfig -- but still builds its own
HTTP client instead of going through the provider adapter.

**Impact:** ACP pipeline tasks get no model routing, no cost tracking, no feedback,
and no provider health monitoring. Credential management is inconsistent.

**Fix:** Replace direct subprocess spawns with `create_agent_for_model()` via the
provider adapter system. The `ClaudeCliAdapter` already handles all the subprocess
construction logic properly.

---

## High-Severity Issues

### ISS-05: No Provider Health Monitoring or Circuit Breaker

**Severity:** High
**Files:** `crates/roko-agent/src/provider/mod.rs`
**Status:** Open

The provider system has error classification (`ProviderError` with `RateLimit`,
`AuthFailure`, `ModelNotFound`, `Timeout`, `ContextOverflow`, `ServerError`) and a
per-provider rate limiter (`ProviderRateLimiter`), but no circuit breaker or health
tracking.

When a provider fails:
- No state machine tracks failure rate over time
- No automatic failover to alternative providers
- Rate limit errors are classified but not used for adaptive backoff
- Timeout errors don't trigger provider demotion
- Repeated failures don't trigger alerts or fallback routing

**Impact:** A provider outage or rate-limit event causes cascading failures rather than
graceful degradation. Users see raw errors instead of transparent model switching.

**Fix:** Implement `ProviderHealthTracker`:
- States: Healthy -> Degraded (>10% error rate) -> Open (>50% or 5 consecutive) ->
  Half-Open (probe after cooldown) -> Healthy
- Wire into `ModelCallService` for pre-dispatch health checks
- Add `with_fallback_models()` for auto-failover when primary is unhealthy
- `ModelCallService` already has `fallback_models` field (just needs health integration)

---

### ISS-06: Budget Enforcement Optional (None Default)

**Severity:** High
**Files:** `crates/roko-agent/src/model_call_service.rs`
**Status:** Partial (BudgetCell exists, default is None)

`ModelCallService::new()` creates a `BudgetCell::new(None)` -- no cost limit by default.
The builder method `with_cost_budget(max_cost_usd)` exists but callers must explicitly
set it. Most callers don't.

The `BudgetCell` implementation tracks cumulative cost and rejects calls when budget is
exceeded, but without a default budget there's no safety net.

**Impact:** Runaway loops or expensive model selections can burn unlimited API credits.
The `roko plan run` path is especially dangerous because it can execute hundreds of
tasks sequentially with no cumulative cost limit.

**Fix:** Set sensible defaults:
- Per-turn: $0.50 (overridable via config)
- Per-session: $10.00 for interactive, $50.00 for plan execution
- Per-plan: $100.00 hard ceiling
- Budget exceeded -> try cheaper model first, then fail with clear message
- Add `[budget]` section to `roko.toml` schema

---

### ISS-07: Direct Environment Variable Reads for API Keys

**Severity:** High
**Files:** `crates/roko-neuro/src/episode_completion.rs`,
  `crates/roko-std/src/tool/builtin/web_search.rs`
**Status:** Open

Two live code paths read API keys directly from environment variables instead of going
through the provider configuration system:

1. `episode_completion.rs` reads `ANTHROPIC_API_KEY` for neuro distillation calls
2. `web_search.rs` reads `PERPLEXITY_API_KEY` for the web search builtin tool

**Impact:**
- Keys are not rotated through the credential management system
- Calls are not tracked in cost accounting
- No fallback if the env var is missing (raw panic or silent failure)
- Inconsistent with the provider system's `ProviderConfig.api_key_env` pattern

**Fix:** Both should receive a configured `Agent` (or `ModelCallService`) through
dependency injection rather than constructing their own HTTP clients from env vars.

---

## Medium-Severity Issues

### ISS-08: 4 Copies of Claude stream-json Parsing Logic

**Severity:** Medium
**Files:** `crates/roko-cli/src/dispatch_direct.rs`,
  `crates/roko-agent/src/translate/mod.rs` (x2),
  `crates/roko-cli/src/chat.rs`
**Status:** Partially mitigated (dispatch_direct.rs feature-gated)

The stream-json parsing logic is duplicated 4 times with inconsistent output formats:
- `dispatch_direct.rs` stores `ToolOutput { tool_name, content }` struct
- `translate/mod.rs:extract_text()` embeds `"\n[{tool_name}]\n{content}\n"` inline
- `translate/mod.rs:extract_tool_outputs()` returns `(Option<String>, String)` tuple
- `chat.rs:extract_clean_text()` formats `"[{tool_name}] {content}"` or truncated

All 4 copies independently implement the same 4096-byte truncation with char_boundary
checks.

**Canonical parser exists:** `provider/claude_cli/stream.rs` exports `parse_stream_line()`
returning typed `ClaudeStreamEvent` variants. The 4 duplicates should all delegate to
this parser.

**Impact:** Bug fixes to one parser don't propagate. Format inconsistencies cause
display bugs. Maintenance burden is 4x.

**Fix:** Replace inline parsing in #2, #3, #4 with calls to `parse_stream_line()`.
#1 is already behind a feature gate and will be removed when legacy-orchestrate is
dropped.

---

### ISS-09: extract_clean_text() -- 130-Line Format Guesser

**Severity:** Medium
**Files:** `crates/roko-cli/src/chat.rs` (lines 386-515)
**Status:** Open

This function handles 10 different response formats by guessing the shape of the input:
1. Plain text passthrough
2. `{"result":"text"}` -- Claude CLI result wrapper
3. `{"content":"text"}` -- sidecar string content
4. `{"content":[{"type":"text","text":"..."}]}` -- content blocks
5. JSON array of content blocks
6. JSONL `result` event
7. JSONL `assistant` event with message.content blocks
8. JSONL `tool` event (4096 truncation)
9. JSONL generic result/content string fallback
10. Final raw text fallback

**Impact:** Brittle format detection. New response shapes require adding another
branch. The function is called from both `chat.rs` and `dispatch_direct.rs`.

**Fix:** Replace with typed deserialization per backend. Each backend adapter should
return a `BackendResponse` that the caller can inspect without guessing formats.
The `BackendResponse` enum (`Json`, `StreamJson`, `Text`) already exists in
`translate/mod.rs`.

---

### ISS-10: Missing Thinking/Reasoning Token Accounting

**Severity:** Medium
**Files:** `crates/roko-agent/src/usage.rs`
**Status:** Partial

`UsageObservation` tracks `input_tokens`, `output_tokens`, `cache_creation_tokens`,
and `cache_read_tokens` but has no field for thinking/reasoning tokens. The
`ThinkingCapCell` in `ModelCallService` caps reasoning budgets but doesn't track
actual thinking token consumption.

Models with thinking (Claude with `--effort`, OpenAI o3/o4-mini, Gemini with
reasoning) produce internal reasoning tokens that cost money but aren't visible
in the standard output. These should be:
- Tracked separately from output tokens
- Included in cost calculations (often cheaper per-token than output)
- Visible in usage reports and dashboards
- Subject to the thinking cap budget

**Impact:** Cost accounting is inaccurate for thinking-capable models. Users can't
see how much they're paying for reasoning vs output.

**Fix:** Add `thinking_tokens: Option<u64>` to `UsageObservation`. Update provider
adapters to extract reasoning token counts from responses. Update `CostTable` to
use thinking-specific pricing.

---

### ISS-11: No Prompt Caching Metrics

**Severity:** Medium
**Files:** `crates/roko-agent/src/model_call_service.rs`
**Status:** Open

`ModelCallService` has an L1 response cache (`CacheCell`, 128 entries, exact match)
but no metrics on cache performance:
- No cache hit rate tracking
- No cache size monitoring
- No cache eviction statistics
- No cache key collision detection
- No cache value freshness tracking

Separately, Anthropic's server-side prompt caching (`cache_read_tokens`,
`cache_creation_tokens`) is tracked in usage but not analyzed:
- No reporting on cache utilization rates
- No optimization of system prompt ordering for better cache hits
- No tracking of cache savings ($) over time

**Impact:** Cannot optimize cache configuration. Cannot measure the cost savings
from caching. Cannot detect cache thrashing.

**Fix:**
- Add `CacheMetrics` to `CacheCell`: hits, misses, evictions, size_bytes
- Expose metrics via gateway events and TUI
- Add Anthropic cache utilization analysis to learning subsystem
- Report cache savings in CostPanel

---

### ISS-12: OpenAI-Compat Provider Strictness Fragmentation

**Severity:** Medium
**Files:** `crates/roko-agent/src/openai_compat_backend.rs`
**Status:** Open

The `OpenAiCompatLlmBackend` has accumulated per-provider workarounds:
- `skip_session_fields`: for Cerebras (rejects unknown fields)
- `disable_parallel_tool_calls`: for Cerebras (small models can't handle parallel calls)
- `normalize_tool_call_content`: for Cerebras (rejects empty content with tool_calls)

These boolean flags are a code smell -- they'll multiply as more strict providers
are added (Deepseek, xAI, etc.). The Kimi K2.5 documentation in the module header
lists 7 additional constraints that aren't yet implemented as flags.

**Impact:** Adding each new strict provider requires new boolean flags and test
coverage. The number of flag combinations grows exponentially.

**Fix:** Replace boolean flags with a `ProviderQuirks` struct or `ProviderProfile`
that captures all compatibility requirements in one place:
```rust
struct ProviderQuirks {
    session_fields: bool,
    parallel_tool_calls: bool,
    normalize_tool_call_content: bool,
    max_tools: Option<usize>,
    timeout_budget: Option<Duration>,
    image_format: ImageFormat,
    // ...
}
```

---

## Low-Severity Issues

### ISS-13: ToolLoop Max Iterations Inconsistent

**Severity:** Low
**Files:** `crates/roko-agent/src/provider/cerebras.rs`,
  `crates/roko-agent/src/provider/openai_compat.rs`
**Status:** Open

The Cerebras adapter sets `tool_loop_max_iterations(50)` while the OpenAI-compat
adapter uses `tool_loop_max_iterations(30)` (or provider-configured). These limits
are per-adapter constants rather than configurable from `roko.toml` or the
`ModelProfile`.

**Fix:** Add `max_tool_iterations` to `ModelProfile` or `ProviderConfig` schema.
Default to 30 for API providers, 50 for Cerebras (small models need more turns).

---

### ISS-14: CostTable Pricing Not Auto-Populated

**Severity:** Low
**Files:** `crates/roko-agent/src/task_runner.rs`
**Status:** Open

`CostTable` for pricing is constructed manually. The `OpenRouter` metadata helper
can fetch live pricing but it's not wired to auto-populate the cost table.

**Fix:** On startup, if an OpenRouter API key is configured, fetch pricing for all
configured models and merge into `CostTable`. Cache locally with a 24-hour TTL.

---

### ISS-15: ProviderKind Enum Missing DeepSeek and xAI

**Severity:** Low
**Files:** `crates/roko-core/src/agent.rs`
**Status:** Open

The `ProviderKind` enum has 7 variants: `AnthropicApi`, `ClaudeCli`, `OpenAiCompat`,
`CursorAcp`, `PerplexityApi`, `GeminiApi`, `CerebrasApi`. Models from DeepSeek, xAI,
Mistral, and Cohere currently route through `OpenAiCompat` which works (they all
support the OpenAI chat completions protocol) but prevents provider-specific
optimizations and error handling.

**Fix:** Only add dedicated `ProviderKind` variants when a provider needs specific
adapter behavior (like Cerebras does for strict mode). For now, `OpenAiCompat` is
fine for protocol-compatible providers. Document which providers are covered by
`OpenAiCompat` in the adapter registry.

---

### ISS-16: Rate Limiter is Singleton

**Severity:** Low
**Files:** `crates/roko-agent/src/openai_compat_backend.rs`
**Status:** Open

`shared_rate_limiter()` uses a `OnceLock` to create a single global
`ProviderRateLimiter` with a fixed 60 RPM default. All `OpenAiCompatLlmBackend`
instances share this limiter unless explicitly overridden with `with_rate_limiter()`.

This means:
- Different providers with different rate limits share the same limiter
- A provider with high limits (1000 RPM) is throttled to 60 RPM
- A provider with low limits (10 RPM) may exceed its actual limit if the global
  limiter allows 60

**Fix:** Move rate limiter configuration to `ProviderConfig` with per-provider
settings. The `with_rate_limiter()` escape hatch exists but should be wired
automatically from config.

---

### ISS-17: No Retry Logic for Transient Failures

**Severity:** Low
**Files:** `crates/roko-agent/src/model_call_service.rs`
**Status:** Open

`ModelCallService` has `fallback_models` for model-level failover but no retry
logic for transient errors (network timeouts, 500 errors, rate limit with
retry-after). When a call fails transiently:
- No automatic retry with backoff
- No rate-limit retry-after header honoring
- Immediate failover to fallback model (may not be necessary)

**Fix:** Add configurable retry policy to `ModelCallService`:
- Retry on `ProviderError::RateLimit` (honor retry_after_ms)
- Retry on `ProviderError::ServerError` with exponential backoff
- Retry on `ProviderError::Timeout` (once, with increased timeout)
- Never retry on `AuthFailure`, `ModelNotFound`, `ContextOverflow`
- Max retries configurable (default 2)

---

## Sources

| File | Purpose |
|---|---|
| `crates/roko-agent/src/provider/mod.rs` | Provider adapter registry (1,148 LOC) |
| `crates/roko-agent/src/model_call_service.rs` | ModelCallService (2,143 LOC) |
| `crates/roko-cli/src/model_selection.rs` | Model selection precedence (581 LOC) |
| `crates/roko-cli/src/orchestrate.rs` | Dead PlanRunner God object (21,577 LOC) |
| `crates/roko-cli/src/dispatch_direct.rs` | Legacy dispatch (404 LOC, feature-gated) |
| `crates/roko-cli/src/dispatch_v2.rs` | v2 dispatch (946 LOC) |
| `crates/roko-cli/src/chat.rs` | REPL + extract_clean_text() |
| `crates/roko-acp/src/runner.rs` | ACP bare subprocess (969 LOC) |
| `crates/roko-acp/src/bridge_events.rs` | ACP event bridge (1,855 LOC) |
| `crates/roko-agent/src/openai_compat_backend.rs` | OpenAI-compat backend (1,120 LOC) |
| `crates/roko-agent/src/provider/cerebras.rs` | Cerebras adapter (198 LOC) |
| `crates/roko-agent/src/usage.rs` | UsageObservation (74 LOC) |
| `crates/roko-neuro/src/episode_completion.rs` | Direct env key read |
| `crates/roko-std/src/tool/builtin/web_search.rs` | Direct env key read |
| `crates/roko-learn/src/cascade_router.rs` | CascadeRouter LinUCB bandit |
