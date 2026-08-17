# Inference & Dispatch: Goals

## End State Vision

Every LLM call in roko -- regardless of entry point (CLI one-shot, plan runner, ACP
bridge, serve routes, agent sidecar, dream consolidation, neuro distillation, web search
tool) -- flows through a single dispatch abstraction that provides: model selection via
CascadeRouter, cost tracking and budget enforcement, episode and efficiency logging,
provider health monitoring with auto-failover, prompt assembly with context injection,
and feedback recording for continuous learning.

---

## 1. Core Properties

### 1A. One Dispatch Path

All inference call sites route through `ModelCallService` -> provider adapter -> concrete
backend. No more:
- Direct `Command::new("claude")` outside `ClaudeCliAdapter`
- Direct `reqwest::Client::post("api.anthropic.com")` outside `AnthropicApiAdapter`
- Direct `std::env::var("ANTHROPIC_API_KEY")` outside provider config resolution
- Manual HTTP request construction outside the provider system

**Measure:** `rg 'Command::new\("claude"\)' crates/ --type rust | grep -v test | grep -v provider/` returns zero results.

### 1B. Cascade Router Learning From Every Call

Every model call feeds back success/failure/latency/cost/token-usage to CascadeRouter.
The router currently has zero live callers because `PlanRunner` (its only consumer) is
dead code. The goal is:
- `resolve_effective_model()` always receives a `CascadeRouter` reference
- After each call, `ModelCallService` records the observation
- CascadeRouter persists updated state to `.roko/learn/cascade-router.json`
- Dashboard and TUI display routing decisions and model confidence

**Current gap:** `resolve_effective_model_key()` always passes `None` for cascade_router.
The `ModelCallService` has `ForceBackendOverrideRecorder` wired but the full observation
loop is missing from most paths.

### 1C. Budget Enforcement (Mandatory)

Per-task and per-session cost limits are mandatory, not optional `None`:
- `ModelCallService` has `BudgetCell` with cumulative tracking (wired)
- Need default budgets: per-turn $0.50, per-session $10.00, per-plan $50.00
- Budget exceeded -> graceful degradation (try cheaper model first, then fail)
- Budget warnings at 50%, 75%, 90% thresholds
- Real-time cost visible in TUI CostPanel and API `/status`

### 1D. Provider Health Monitoring

Circuit breaker per provider with auto-failover:
- Track error rate, latency percentiles, rate-limit frequency per provider
- Provider health states: Healthy -> Degraded -> Open -> Half-Open -> Healthy
- When a provider enters Open state, route to next-best provider automatically
- Health state visible in TUI and via `roko config providers health`
- Exponential backoff with jitter for rate-limited providers

**What exists:** `ProviderError` enum in `provider/mod.rs` has the error classification
(`RateLimit`, `AuthFailure`, `ModelNotFound`, `Timeout`, `ContextOverflow`,
`ServerError`, `Other`). `ProviderRateLimiter` exists in the OpenAI-compat backend.
Missing: circuit breaker state machine, health aggregation, auto-failover routing.

### 1E. Multi-Model Racing (Parallel Dispatch)

Dispatch the same prompt to N providers in parallel, take first/best result:
- Useful for latency-sensitive paths (interactive chat, TUI)
- Cancel slower providers when first result arrives
- Track which provider wins for CascadeRouter learning
- Budget accounts for all attempted calls, not just the winner
- Configurable: `race_models = ["claude-haiku-4-5", "gemini-2.0-flash"]`

---

## 2. Model Selection Goals

### 2A. Smart Model Selection by Task Complexity

Match model capability to task complexity automatically:

| Task Signal | Cheap Model (Haiku/Flash) | Mid Model (Sonnet) | Premium Model (Opus) |
|---|---|---|---|
| File rename, simple grep | Yes | Overkill | Wasteful |
| Implement function from spec | Maybe | Yes | If complex |
| Architecture decision | No | Maybe | Yes |
| Code review (small diff) | Yes | Yes | Overkill |
| Multi-file refactor | No | Yes | Yes |
| Debug subtle race condition | No | No | Yes |

Complexity signals available today:
- Task category (from plan TOML: implement, review, test, refactor, docs)
- LOC estimate (from task definition or file analysis)
- Domain (from roko.toml agent config)
- Prior failure count (gate failures on this task)
- Role (implementer, reviewer, architect, etc.)

### 2B. Escalation and De-escalation

Cascade routing with automatic escalation:
1. Start with cheapest viable model for the task
2. If gate fails -> escalate to next tier
3. If 3 consecutive gate passes at current tier -> consider de-escalation
4. Track escalation/de-escalation decisions for learning
5. Never escalate past budget ceiling

De-escalation signals:
- Task similarity to previously successful cheap-model tasks
- Knowledge store entries about task-model fit
- Time-of-day cost optimization (batch cheaper models during off-peak)

### 2C. Role-Based Model Defaults

Configure per-role model preferences in `roko.toml`:
```toml
[agent.roles.implementer]
model = "claude-sonnet-4-6"

[agent.roles.architect]
model = "claude-opus-4-6"

[agent.roles.reviewer]
model = "claude-haiku-4-5"

[agent.roles.researcher]
model = "perplexity/sonar-pro"
```

This is already supported via `resolve_effective_model()` precedence tier 3 (RoleConfig).
The gap is that most callers don't pass a role string.

---

## 3. Provider Landscape Goals (2025-2026)

### 3A. Models to Support

| Provider | Model Family | Why | Priority |
|---|---|---|---|
| Anthropic | Claude Opus 4, Sonnet 4, Haiku 4 | Primary backend, best tool calling | P0 (done) |
| Anthropic | Claude Opus 4.6, Sonnet 4.6 | Latest generation | P0 (done) |
| OpenAI | GPT-5, GPT-5-mini, o3, o4-mini | Alternative, strong reasoning | P1 |
| Google | Gemini 2.5 Pro, Flash | Free tier available, multimodal | P1 (done) |
| Cerebras | Llama 3.1 8B/70B | Ultra-fast inference (2000+ tok/s) | P1 (done) |
| Perplexity | Sonar Pro, Deep Research | Web search, research | P1 (done) |
| OpenRouter | Any model | Aggregator, fallback, cost arbitrage | P2 (metadata done) |
| Deepseek | R1, V3 | Strong reasoning, cheap | P2 |
| Moonshot | Kimi K2.5 | Long context (128K), thinking | P2 (compat done) |
| xAI | Grok 3 | Fast, competitive pricing | P3 |
| Meta | Llama 4 | Open weights, self-hosted | P3 |

### 3B. Pricing Awareness

ModelProfile already carries cost fields:
```rust
pub cost_input_per_m: Option<f64>,     // $/M input tokens
pub cost_output_per_m: Option<f64>,    // $/M output tokens
pub cost_input_per_m_high: Option<f64>,  // premium tier pricing
pub cost_output_per_m_high: Option<f64>,
pub cost_cache_read_per_m: Option<f64>,
pub cost_cache_write_per_m: Option<f64>,
```

Goals:
- Auto-populate pricing from OpenRouter metadata API
- Periodic refresh of pricing data (daily or on startup)
- Cost-aware routing: factor price into CascadeRouter reward signal
- Cache utilization tracking: measure cache hit rates per model
- Budget dashboard showing cost breakdown by model/role/task

### 3C. Capability Matrix

ModelProfile capability flags that exist:
```rust
pub supports_tools: bool,
pub supports_thinking: bool,
pub supports_vision: bool,
pub supports_web_search: bool,
pub supports_mcp_tools: bool,
pub supports_partial: bool,
pub supports_grounding: bool,
pub supports_code_execution: bool,
pub supports_caching: bool,
```

Goals:
- Use capabilities for hard constraints (don't route tool tasks to non-tool models)
- Use capabilities for soft preferences (prefer vision models for image tasks)
- Auto-detect capabilities from OpenRouter metadata
- Warn when task requires capability that selected model lacks

---

## 4. UX Goals (From v2 UX Showcase)

### 4A. RouterTrace Card

Visual display of cascade router decisions:
- Policy mode: `auto - cost-optimized` / `auto - learning` / `post-replay update`
- Candidate list with score bars and reasons (e.g. "haiku 94% trivial, sonnet 62% overkill")
- Active chosen candidate highlighted
- Escalation state visible (e.g. "haiku 32% failed gate -> sonnet 91% escalated")

### 4B. CostPanel

Right-rail panel showing:
- This-turn cost vs turn budget (progress bar)
- Session cost vs session budget (progress bar)
- 4-cell token breakdown (input/output/cached/thought)
- Cost-per-turn sparkline with trend indicator
- Total session cost in dollars

### 4C. Tier Confidence Panel

Per-model confidence bars with cost:
- haiku 74% confidence, $0.001/call average
- sonnet 18% confidence, $0.012/call average
- opus 8% confidence, $0.084/call average

### 4D. Data Feeds Required

| Feed | Shape | Source |
|---|---|---|
| `RouterDecision` | policy, candidates[], chosen flag | CascadeRouter.select() |
| `EscalationEvent` | from_model, to_model, reason | Gate failure -> re-route |
| `CostEstimate` | model, estimated_cost, actual_cost | ModelCallService.cost_predict() |
| `TierConfidence` | per-model: name, confidence, cost/call | CascadeRouter.confidence_stats |
| `DecisionHistory` | per-task: model, success, escalated_to | routing-log.jsonl |
| `CostSummary` | turn/session costs, tokens, sparkline | BudgetCell + GatewayEvents |

---

## 5. What Exists Today

### 5A. Provider Infrastructure (Done)

- 7 provider adapters in `roko-agent/src/provider/`
- `create_agent_for_model()` config-driven agent factory
- `ModelCallService` (2,143 LOC) with cache, budget, convergence, gateway events
- `resolve_effective_model()` 6-tier precedence chain
- `dispatch_via_model_call_service()` v2 entry point
- Per-turn usage info (input/output/thought/cached tokens)
- `UsageObservation` canonical telemetry type
- `FeedbackService` implementing `FeedbackSink`
- `GatewayEventWriter` for durable event logging
- `ProviderRateLimiter` shared rate limiter
- Provider error classification (`ProviderError` enum)
- OpenRouter metadata fetcher for auto-discovery

### 5B. CascadeRouter (Built, Not Wired to Live Paths)

- LinUCB contextual bandit with 4-stage pipeline
- Persistence to `.roko/learn/cascade-router.json`
- Per-model reward tracking with Bayesian updates
- Pareto frontier for cost-quality tradeoff
- Knowledge-informed routing boost
- Cost spike detection
- Free-tier shadow evaluation
- `ForceBackendOverrideRecorder` trait for learning from overrides

### 5C. Prompt Assembly (Partial)

- 9-layer `SystemPromptBuilder` in `roko-compose`
- 10+ role templates
- VCG auction for context allocation (built, greedy path dominates)
- `AttentionBidder` variants for context competition

---

## 6. Gaps (What's Missing)

### Priority 1 (Blocks self-hosting quality)

| Gap | Impact | Fix |
|---|---|---|
| CascadeRouter zero live callers | No adaptive routing | Thread router through all dispatch paths |
| One-shot paths skip episode logging | No learning from interactive use | Add episode recording to ModelCallService |
| ACP bridge bypasses providers | Inconsistent model resolution | Route through ClaudeCliAdapter |
| Budget optional (None default) | No cost control | Set mandatory defaults |

### Priority 2 (Improves efficiency)

| Gap | Impact | Fix |
|---|---|---|
| No provider health monitoring | No auto-failover | Add circuit breaker state machine |
| No cost-aware routing | Wastes money on trivial tasks | Factor pricing into CascadeRouter reward |
| No escalation tracking | No learning from gate failures | Record escalation events |
| Knowledge store not consulted | Misses routing intelligence | Wire neuro store query |

### Priority 3 (Feature completeness)

| Gap | Impact | Fix |
|---|---|---|
| No multi-model racing | Higher latency | Implement parallel dispatch |
| No prompt caching metrics | Can't optimize cache | Track cache hit rates |
| No cost dashboard | No visibility | Wire CostSummary to TUI |
| OpenRouter auto-discovery | Manual config | Periodic metadata refresh |

---

## Sources

| File | Purpose |
|---|---|
| `crates/roko-agent/src/provider/mod.rs` | Provider adapter registry (1,148 LOC) |
| `crates/roko-agent/src/model_call_service.rs` | ModelCallService (2,143 LOC) |
| `crates/roko-cli/src/model_selection.rs` | 6-tier model selection (581 LOC) |
| `crates/roko-cli/src/dispatch_v2.rs` | v2 dispatch entry point (946 LOC) |
| `crates/roko-learn/src/cascade_router.rs` | CascadeRouter LinUCB bandit |
| `crates/roko-agent/src/usage.rs` | UsageObservation telemetry (74 LOC) |
| `crates/roko-agent/src/provider/cerebras.rs` | Cerebras adapter (198 LOC) |
| `crates/roko-agent/src/provider/openrouter_meta.rs` | OpenRouter metadata (388 LOC) |
| `crates/roko-core/src/agent.rs` | ProviderKind enum (7 variants) |
