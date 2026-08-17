# 03: Inference Dispatch Implementation Plan

> 42 tasks for unifying, fixing, and extending the inference dispatch subsystem.
> Covers: CascadeRouter wiring, dispatch unification, provider health, budget
> enforcement, stream parser consolidation, model selection, env key elimination,
> episode logging, and novel dispatch strategies.

**Source audits:**
- `tmp/solutions/roko/03-PROVIDER-AND-AGENT-AUDIT.md`
- `tmp/solutions/roko/19-DISPATCH-AUDIT.md`
- `tmp/solutions/roko/19-DISPATCH-GOALS.md`
- `tmp/solutions/roko/19-DISPATCH-ISSUES.md`
- `tmp/solutions/roko/19-DISPATCH-PLAN.md`

**Date:** 2026-04-29

---

## Phase 0: Wire CascadeRouter to Live Paths

### TASK-D01: Load CascadeRouter at CLI Startup
**Priority**: P0
**Category**: wiring
**Files**: `crates/roko-cli/src/model_selection.rs`, `crates/roko-learn/src/cascade_router.rs`
**What**: Add `load_cascade_router(workdir, config)` function to `model_selection.rs` that loads from `.roko/learn/cascade-router.json` or initializes a fresh router with model slugs from config. Currently `CascadeRouter` is fully built (LinUCB 4-stage pipeline, persistence, Pareto frontier) but has zero live callers because the dead `PlanRunner` in `orchestrate.rs` was its only consumer.
**Steps**:
1. Add `pub fn load_cascade_router(workdir: &Path, config: &RokoConfig) -> CascadeRouter` that calls `CascadeRouter::load_or_new()` with the router path and model slugs extracted from `config.effective_models()`
2. Add `pub fn save_cascade_router(workdir: &Path, router: &CascadeRouter) -> Result<()>` companion function
3. Verify the serve runtime's existing router load at `serve_runtime.rs:522` uses the same path convention
4. Add unit test for load/save roundtrip
**Acceptance**: `rg 'load_cascade_router' crates/roko-cli/src/model_selection.rs | wc -l` >= 1
**Depends on**: none
**Effort**: S

### TASK-D02: Thread CascadeRouter Through resolve_effective_model_key
**Priority**: P0
**Category**: wiring
**Files**: `crates/roko-cli/src/model_selection.rs`
**What**: `resolve_effective_model_key()` is the convenience wrapper called by `plan.rs`, `prd.rs`, and other CLI commands. It currently hard-codes `None` for the cascade_router parameter. Change it to accept `Option<&CascadeRouter>` and forward to `resolve_effective_model()`.
**Steps**:
1. Change `resolve_effective_model_key()` signature to accept `Option<&CascadeRouter>`
2. Pass through to `resolve_effective_model()` instead of hard-coded `None`
3. Update all call sites (`commands/plan.rs:559`, `commands/plan.rs:608`, `commands/prd.rs:351`, `commands/prd.rs:672`) to pass the router
4. Verify `SelectionSource::CascadeRouter` variant is now reachable
**Acceptance**: `rg 'resolve_effective_model_key' crates/roko-cli/src/ --type rust | grep -v test` shows router parameter at all call sites
**Depends on**: TASK-D01
**Effort**: S

### TASK-D03: Wire CascadeRouter into `roko run` Entry Point
**Priority**: P0
**Category**: wiring
**Files**: `crates/roko-cli/src/run.rs`
**What**: The `roko run "<prompt>"` universal loop is the primary dispatch entry point. It should load a CascadeRouter, use it for model selection, and persist after execution.
**Steps**:
1. At the top of the `roko run` handler, call `load_cascade_router(&workdir, &config)`
2. Pass the router to model selection where `build_role_system_prompt_validated()` currently uses static model selection
3. After execution completes, call `save_cascade_router(&workdir, &router)`
4. Ensure the router is persisted even on early return / error (use a defer-like pattern or explicit save in error paths)
**Acceptance**: Run `cargo run -p roko-cli -- run "echo hello"` and verify `.roko/learn/cascade-router.json` is updated
**Depends on**: TASK-D01
**Effort**: M

### TASK-D04: Wire CascadeRouter into `roko chat` / chat_inline
**Priority**: P0
**Category**: wiring
**Files**: `crates/roko-cli/src/chat_inline.rs`
**What**: The interactive chat REPL (`roko chat`, `roko <prompt>`) should use adaptive routing. Load router at session start, record observations per turn, persist on session end.
**Steps**:
1. Load CascadeRouter at chat session initialization
2. Pass router reference through to `dispatch_via_model_call_service()` calls
3. After each turn, the `ModelCallService` feedback path should record an observation
4. Persist router on session exit (both clean exit and Ctrl-C handler)
5. Verify the in-memory `CostMeter` data also feeds into router observations
**Acceptance**: Interactive chat session produces router observations; `.roko/learn/cascade-router.json` grows after a multi-turn session
**Depends on**: TASK-D01, TASK-D06
**Effort**: M

### TASK-D05: Wire CascadeRouter into `roko plan run`
**Priority**: P0
**Category**: wiring
**Files**: `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/runner/event_loop.rs`
**What**: Plan execution dispatches many sequential agent calls. The runner should use the CascadeRouter for per-task model selection and learn from each task's outcome.
**Steps**:
1. Load CascadeRouter in the `plan run` command handler
2. Pass to the runner's `CliProviderConfig` or `ModelCallService` builder
3. After each task gate pass/fail, record the observation with the model used and the gate verdict as reward signal
4. Persist router after each batch of tasks (not after every single task, to avoid I/O overhead)
5. On plan completion, do a final persist
**Acceptance**: `rg 'CascadeRouter\|cascade_router' crates/roko-cli/src/commands/plan.rs | wc -l` >= 2
**Depends on**: TASK-D01, TASK-D02
**Effort**: M

### TASK-D06: Record Router Observations in ModelCallService
**Priority**: P0
**Category**: wiring
**Files**: `crates/roko-agent/src/model_call_service.rs`
**What**: After each `call()`, `ModelCallService` should feed success/failure/latency/cost back to the CascadeRouter. The `ForceBackendOverrideRecorder` trait exists but the full observation loop is missing.
**Steps**:
1. Add `cascade_router: Option<Arc<Mutex<CascadeRouter>>>` field to `ModelCallService`
2. Add builder method `with_cascade_router(router: Arc<Mutex<CascadeRouter>>) -> Self`
3. After `Agent::run()` completes, call `router.lock().observe()` with model slug, success boolean, latency_ms, cost_usd, and a context feature vector derived from the request
4. Build the 17-dimensional context vector from available request metadata (role, complexity hints, prior failure flag)
5. Ensure the observation is recorded even on failure (with success=false)
**Acceptance**: `rg 'cascade_router.*observe\|observe.*cascade' crates/roko-agent/src/model_call_service.rs | wc -l` >= 1
**Depends on**: none
**Effort**: M

---

## Phase 1: Episode Logging for All Paths

### TASK-D07: Add Episode Emission to ModelCallService
**Priority**: P0
**Category**: wiring
**Files**: `crates/roko-agent/src/model_call_service.rs`, `crates/roko-core/src/runtime_event.rs`
**What**: Every LLM call through `ModelCallService` should produce a durable episode record. The `FeedbackSink` records `FeedbackEvent::ModelCall` but does not write to `episodes.jsonl`. Add an episode emission path.
**Steps**:
1. After each successful or failed `call()`, emit a `RuntimeEvent::Episode` variant (add to `RuntimeEvent` if missing)
2. Include: timestamp, run_id, model, role, input_tokens, output_tokens, cost_usd, latency_ms, success, entry_point
3. Route through the existing `event_consumers` vector
4. Ensure the episode format matches what `runner/event_loop.rs` already writes for consistency
**Acceptance**: `rg 'RuntimeEvent::Episode\|emit.*episode' crates/roko-agent/src/model_call_service.rs | wc -l` >= 1
**Depends on**: none
**Effort**: S

### TASK-D08: Wire Episode Logger to One-Shot CLI Paths
**Priority**: P0
**Category**: wiring
**Files**: `crates/roko-cli/src/run_inline.rs`, `crates/roko-cli/src/chat_inline.rs`, `crates/roko-runtime/src/jsonl_logger.rs`
**What**: The one-shot paths (`roko "prompt"`, inline dispatch) currently produce no durable episode data. Wire a JSONL episode logger as an event consumer on the `ModelCallService` used by these paths.
**Steps**:
1. Create or reuse a `JsonlEpisodeLogger` implementing `EventConsumer` that appends episode events to `.roko/episodes.jsonl`
2. In `run_inline.rs` and `chat_inline.rs`, when constructing `ModelCallService`, add `.with_event_consumer(Arc::new(episode_logger))`
3. Ensure the file is created with append mode and flushed per-event (not buffered)
4. Verify with an integration test: run a one-shot prompt, check `episodes.jsonl` has a new entry
**Acceptance**: `cargo run -p roko-cli -- run "echo hello" && test -f .roko/episodes.jsonl` succeeds
**Depends on**: TASK-D07
**Effort**: M

### TASK-D09: Wire Efficiency Events to One-Shot Paths
**Priority**: P1
**Category**: wiring
**Files**: `crates/roko-cli/src/chat_inline.rs`, `crates/roko-cli/src/run_inline.rs`
**What**: Efficiency events (tokens/sec, cost/token, cache hit rate) are written by `runner/event_loop.rs` but not by one-shot dispatch paths. The learning subsystem needs these for optimizing model selection.
**Steps**:
1. After each model call in one-shot paths, compute efficiency metrics from `UsageObservation`
2. Emit `RuntimeEvent::Efficiency` with tokens_per_second, cost_per_1k_tokens, cache_fraction
3. Route through the same JSONL logger to `.roko/learn/efficiency.jsonl`
4. Verify the efficiency format matches the runner's output
**Acceptance**: `rg 'RuntimeEvent::Efficiency\|efficiency' crates/roko-cli/src/run_inline.rs | wc -l` >= 1
**Depends on**: TASK-D07
**Effort**: S

### TASK-D10: Add Entry Point Tag to Episode Records
**Priority**: P1
**Category**: fix
**Files**: `crates/roko-core/src/runtime_event.rs`, `crates/roko-agent/src/model_call_service.rs`
**What**: Episode records should include which CLI entry point generated them (run, chat, plan, prd, research, serve, agent-sidecar) so the learning subsystem can segment data by usage pattern.
**Steps**:
1. Add `entry_point: Option<String>` field to the Episode event payload
2. Add `with_entry_point(tag: impl Into<String>)` builder method to `ModelCallService`
3. Set entry_point in each CLI command's ModelCallService construction
4. Include entry_point in the JSONL episode record
**Acceptance**: Episode records in `episodes.jsonl` include `"entry_point": "roko_run"` or similar
**Depends on**: TASK-D07
**Effort**: S

---

## Phase 2: Unify Dispatch Paths

### TASK-D11: Migrate chat_inline to ModelCallService
**Priority**: P0
**Category**: wiring
**Files**: `crates/roko-cli/src/chat_inline.rs`, `crates/roko-cli/src/dispatch_v2.rs`
**What**: `chat_inline.rs` uses `dispatch_via_model_call_service()` for some paths but still has fallback paths that bypass the provider system. Ensure all inference calls in the chat REPL go through `ModelCallService`.
**Steps**:
1. Audit all LLM call sites in `chat_inline.rs` (grep for `dispatch_`, `create_agent`, `Command::new`)
2. Replace any remaining direct dispatch calls with `dispatch_via_model_call_service()` or direct `ModelCallService::call()`
3. Remove the in-memory-only `CostMeter` and use `BudgetCell` from `ModelCallService` instead
4. Ensure cost data is persisted, not lost on exit
5. Verify `extract_clean_text()` calls are replaced with typed parsing (or deferred to TASK-D25)
**Acceptance**: `rg 'dispatch_direct\|dispatch_prompt' crates/roko-cli/src/chat_inline.rs | grep -v test | wc -l` == 0
**Depends on**: TASK-D07
**Effort**: M

### TASK-D12: Migrate ACP Runner to Provider System
**Priority**: P0
**Category**: wiring
**Files**: `crates/roko-acp/src/runner.rs`, `crates/roko-acp/Cargo.toml`
**What**: `runner.rs:run_claude_cli()` spawns `claude --print --dangerously-skip-permissions` as a bare subprocess with no model flag, no streaming, no system prompt, and no feedback. Replace with `create_agent_for_model()`.
**Steps**:
1. Add `roko-agent` dependency to `roko-acp/Cargo.toml` if not present
2. Thread `RokoConfig` through the ACP pipeline context (via `PipelineContext` or similar)
3. Replace `Command::new("claude")` with `create_agent_for_model(&config, model_key, &options)`
4. Use the provider-resolved model instead of relying on the CLI default
5. Return `AgentResult` and extract usage for cost tracking
**Acceptance**: `rg 'Command::new\("claude"\)' crates/roko-acp/src/runner.rs | grep -v test | wc -l` == 0
**Depends on**: none
**Effort**: M

### TASK-D13: Migrate ACP Bridge Events to Provider System
**Priority**: P0
**Category**: wiring
**Files**: `crates/roko-acp/src/bridge_events.rs`
**What**: Two functions bypass providers: `run_claude_cognitive_task()` spawns `claude` directly, `run_openai_compat_cognitive_task()` builds its own HTTP client. Both should route through provider adapters.
**Steps**:
1. Replace `run_claude_cognitive_task()` subprocess spawn with `create_agent_for_model()` using `ClaudeCliAdapter`
2. Replace `run_openai_compat_cognitive_task()` manual HTTP construction with `create_agent_for_model()` using `OpenAiCompatAdapter`
3. Thread `RokoConfig` into the bridge event context
4. Preserve streaming behavior by using the agent's streaming mode
5. Ensure the cognitive task results maintain the same output format for downstream consumers
**Acceptance**: `rg 'Command::new\("claude"\)' crates/roko-acp/src/bridge_events.rs | grep -v test | wc -l` == 0
**Depends on**: TASK-D12
**Effort**: M

### TASK-D14: Gate dispatch_direct.rs Behind Feature Flag (Verify)
**Priority**: P1
**Category**: fix
**Files**: `crates/roko-cli/src/dispatch_direct.rs`
**What**: The audit says `dispatch_direct.rs` is gated behind `legacy-orchestrate`. Verify that all public symbols are indeed feature-gated and that no live code path can reach them without the feature.
**Steps**:
1. Verify `#[cfg(feature = "legacy-orchestrate")]` covers all public functions in `dispatch_direct.rs`
2. Check that `roko-cli/Cargo.toml` does not enable `legacy-orchestrate` by default
3. Grep for any `dispatch_direct::` imports that are NOT behind cfg gates
4. If any live imports exist, add the feature gate or migrate the caller
5. Add a comment documenting the planned removal timeline
**Acceptance**: `rg 'dispatch_direct::' crates/ --type rust | grep -v 'cfg.*legacy' | grep -v test | wc -l` == 0
**Depends on**: none
**Effort**: S

### TASK-D15: Remove Hardcoded Model Strings from auth_detect.rs
**Priority**: P1
**Category**: fix
**Files**: `crates/roko-cli/src/auth_detect.rs`
**What**: `auth_detect.rs` contains `Command::new("claude")` with hardcoded model strings. Auth detection should probe provider availability without hardcoding specific models.
**Steps**:
1. Replace hardcoded `"claude-sonnet-4-6"` with the config's `default_model` or a lightweight probe model
2. Use `create_agent_for_model()` instead of direct `Command::new("claude")`
3. If auth detection needs a minimal probe, use a zero-token request or provider health check endpoint instead of a full model invocation
4. Remove direct `std::env::var("ZAI_API_KEY")`, `std::env::var("ANTHROPIC_API_KEY")`, `std::env::var("OPENAI_API_KEY")` reads; use `config.providers` to check which providers have credentials
**Acceptance**: `rg 'Command::new\("claude"\)' crates/roko-cli/src/auth_detect.rs | wc -l` == 0
**Depends on**: none
**Effort**: M

---

## Phase 3: Provider Health and Circuit Breaker

### TASK-D16: Implement ProviderHealthTracker
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-agent/src/provider/health.rs` (new), `crates/roko-agent/src/provider/mod.rs`
**What**: Build a circuit breaker state machine for provider health. States: Healthy -> Degraded (>10% error rate in 5min window) -> Open (>50% error rate or 5 consecutive failures) -> HalfOpen (probe after cooldown) -> Healthy.
**Steps**:
1. Create `provider/health.rs` with `ProviderHealthTracker` struct
2. Implement `ProviderState` enum: `Healthy`, `Degraded`, `Open`, `HalfOpen`
3. Use a sliding time window (5min default) for error rate calculation
4. Track per-provider: success count, failure count, consecutive failures, last state transition time
5. Implement `record_success(provider_key)`, `record_failure(provider_key, error_kind)`, `is_healthy(provider_key)`, `state(provider_key)`
6. Add cooldown period (30s default) before transitioning Open -> HalfOpen
7. HalfOpen allows a single probe request; success -> Healthy, failure -> Open
8. Export from `provider/mod.rs`
9. Add unit tests for all state transitions
**Acceptance**: `cargo test -p roko-agent -- provider::health | wc -l` shows passing tests for all 5 state transitions
**Depends on**: none
**Effort**: M

### TASK-D17: Wire Health Checks into ModelCallService
**Priority**: P1
**Category**: wiring
**Files**: `crates/roko-agent/src/model_call_service.rs`
**What**: Before dispatching a call, `ModelCallService` should check provider health. If unhealthy, try fallback models on healthy providers. After each call, record success/failure on the health tracker.
**Steps**:
1. Add `health_tracker: Option<Arc<ProviderHealthTracker>>` field to `ModelCallService`
2. Add builder method `with_health_tracker(tracker: Arc<ProviderHealthTracker>)`
3. In `call()`, before `create_agent_for_model()`: check `health_tracker.is_healthy(provider_key)`
4. If unhealthy: iterate `fallback_models`, find first with healthy provider, use that instead
5. After `Agent::run()` succeeds: `health_tracker.record_success(provider_key)`
6. After `Agent::run()` fails: `health_tracker.record_failure(provider_key, classify_error(err))`
7. Emit `RuntimeEvent::ProviderHealthChange` when state transitions occur
**Acceptance**: `rg 'health_tracker\|ProviderHealthTracker' crates/roko-agent/src/model_call_service.rs | wc -l` >= 3
**Depends on**: TASK-D16
**Effort**: M

### TASK-D18: Add Retry Logic to ModelCallService
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-agent/src/model_call_service.rs`
**What**: `ModelCallService` has `fallback_models` for model-level failover but no retry logic for transient errors. Network timeouts, 500 errors, and rate-limit-with-retry-after should trigger automatic retry before falling back to a different model.
**Steps**:
1. Add `RetryPolicy` struct: `max_retries: u32` (default 2), `backoff_base_ms: u64` (default 1000), `backoff_max_ms: u64` (default 30000), `retryable_errors: HashSet<ErrorKind>`
2. Add builder method `with_retry_policy(policy: RetryPolicy)`
3. Default retryable: `RateLimit`, `ServerError`, `Timeout`
4. Never retry: `AuthFailure`, `ModelNotFound`, `ContextOverflow`, `ContentPolicy`
5. Implement full-jitter exponential backoff (matches the existing `retry.rs` pattern in `crates/roko-agent/src/retry.rs`)
6. Honor `retry_after_ms` from `ProviderError::RateLimit` when present
7. After max retries exhausted, fall through to fallback model logic
**Acceptance**: Unit test demonstrates: transient 500 -> retry succeeds on attempt 2
**Depends on**: none
**Effort**: M

### TASK-D19: Expose Provider Health via Serve API
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-serve/src/routes/providers.rs`, `crates/roko-serve/src/state.rs`
**What**: Add `GET /api/providers/health` endpoint returning per-provider health state, error rate, latency percentiles, and rate limit utilization. The route stubs already exist in `providers.rs`.
**Steps**:
1. Add `ProviderHealthTracker` to the serve `AppState`
2. Wire the shared tracker through `ModelCallService` instances created by the serve runtime
3. Implement handler for `GET /api/providers/health` returning JSON with per-provider state, error_rate, latency_p50_ms, latency_p99_ms, rate_limit_utilization
4. Add `GET /api/providers/{key}/health` for single-provider detail
5. Wire to the existing `roko config providers health` CLI command
**Acceptance**: `curl http://localhost:6677/api/providers/health` returns valid JSON with provider states
**Depends on**: TASK-D16, TASK-D17
**Effort**: M

### TASK-D20: Per-Provider Rate Limiter from Config
**Priority**: P1
**Category**: fix
**Files**: `crates/roko-agent/src/openai_compat_backend.rs`, `crates/roko-agent/src/rate_limit.rs`
**What**: The global `shared_rate_limiter()` uses a `OnceLock` with fixed 60 RPM. Different providers have different limits (Anthropic: 1000 RPM tier 4, Cerebras: 30 RPM free). Move to per-provider rate limits from config.
**Steps**:
1. Add `rate_limit_rpm: Option<u32>` to `ProviderConfig` schema in `roko-core`
2. In `create_agent_for_model()`, construct a per-provider `ProviderRateLimiter` from config instead of sharing the global singleton
3. Store per-provider limiters in a `HashMap<String, Arc<ProviderRateLimiter>>` keyed by provider name
4. Use the global 60 RPM limiter as fallback when config doesn't specify a limit
5. Pass the correct limiter to `OpenAiCompatLlmBackend::with_rate_limiter()`
**Acceptance**: Two providers with different configured RPM limits are throttled independently
**Depends on**: none
**Effort**: M

---

## Phase 4: Budget Enforcement

### TASK-D21: Add Budget Configuration to roko.toml Schema
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-core/src/config/schema.rs`
**What**: Add a `[budget]` section to the config schema with per-turn, per-session, and per-plan cost limits plus warning thresholds and exceeded-action policy.
**Steps**:
1. Add `BudgetConfig` struct: `per_turn_usd: Option<f64>`, `per_session_usd: Option<f64>`, `per_plan_usd: Option<f64>`, `warn_at: Vec<f64>` (default [0.5, 0.75, 0.9]), `on_exceeded: BudgetAction` (Downgrade/Fail/Warn)
2. Add `budget: Option<BudgetConfig>` field to `RokoConfig`
3. Implement `Default` with sensible values: per_turn=$0.50, per_session=$10.00, per_plan=$100.00
4. Add deserialization from TOML
5. Validate: all values > 0, warn_at values in 0..1 range, per_turn < per_session < per_plan
**Acceptance**: `rg 'BudgetConfig\|per_turn_usd\|per_session_usd' crates/roko-core/src/config/ --type rust | wc -l` >= 3
**Depends on**: none
**Effort**: S

### TASK-D22: Set Default Budget in ModelCallService
**Priority**: P1
**Category**: fix
**Files**: `crates/roko-agent/src/model_call_service.rs`
**What**: `ModelCallService::new()` creates `BudgetCell::new(None)` -- no cost limit by default. Change to read from `BudgetConfig` or apply a sensible default.
**Steps**:
1. Add `with_budget_config(config: &BudgetConfig)` builder method
2. Change default from `BudgetCell::new(None)` to `BudgetCell::new(Some(10.0))` for a $10 session safety net
3. When `BudgetConfig` is provided, use `per_session_usd` for the budget cell limit
4. Update all `ModelCallService` construction sites to pass budget config from `RokoConfig`
5. Add a log warning when the implicit default is used (config has no budget section)
**Acceptance**: `rg 'BudgetCell::new\(None\)' crates/roko-agent/src/model_call_service.rs | wc -l` == 0 (outside tests)
**Depends on**: TASK-D21
**Effort**: S

### TASK-D23: Implement Budget Exceeded Graceful Degradation
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-agent/src/model_call_service.rs`
**What**: When budget is near exhaustion, try a cheaper model before failing. When budget is exceeded, act according to the `on_exceeded` policy: Downgrade (try cheapest model), Fail (return error), Warn (proceed with warning).
**Steps**:
1. Before dispatch, check `budget.remaining_fraction()`
2. If < 10% remaining: find cheapest viable model from config, switch to it, emit warning
3. If budget exceeded: match on `BudgetAction` -- Downgrade tries cheapest, Fail returns `BudgetExceeded` error, Warn logs and proceeds
4. Emit `RuntimeEvent::BudgetWarning` at configured thresholds (50%, 75%, 90%)
5. Track which threshold warnings have been emitted to avoid duplicate warnings per session
6. Include cheapest-model selection in CascadeRouter observations (learning from forced downgrades)
**Acceptance**: Integration test: set $0.01 budget, make call, verify downgrade or failure according to policy
**Depends on**: TASK-D22
**Effort**: M

### TASK-D24: Expose Budget Status via API and TUI Events
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-serve/src/routes/status/mod.rs`, `crates/roko-core/src/runtime_event.rs`
**What**: Budget consumption should be visible in real-time through the serve API and TUI dashboard events.
**Steps**:
1. Add `GET /api/status/budget` endpoint returning current session budget, consumed, remaining, fraction_used
2. Emit `RuntimeEvent::BudgetStatus` periodically (after each model call) through event consumers
3. Include per-model cost breakdown in the status response
4. Wire to the TUI cost panel if the TUI infrastructure supports it (otherwise note as follow-up)
**Acceptance**: `curl http://localhost:6677/api/status/budget` returns JSON with budget fields
**Depends on**: TASK-D21, TASK-D22
**Effort**: S

---

## Phase 5: Stream Parser Consolidation

### TASK-D25: Replace extract_clean_text with Typed Parsing
**Priority**: P1
**Category**: fix
**Files**: `crates/roko-cli/src/chat.rs`, `crates/roko-agent/src/provider/claude_cli/stream.rs`
**What**: `extract_clean_text()` at `chat.rs:386-515` is a 130-line format guesser handling 10 different response shapes. Replace with typed deserialization using the canonical `parse_stream_line()` from `provider/claude_cli/stream.rs`.
**Steps**:
1. Review `parse_stream_line()` in `provider/claude_cli/stream.rs` to confirm it handles all needed event types (System, Assistant, Tool, Result, ContentBlockDelta)
2. Add any missing event types to the canonical parser (e.g., sidecar string content format)
3. Create `fn extract_text_typed(raw: &str) -> String` that first tries `parse_stream_line()`, then falls back to the `BackendResponse` enum deserialization
4. Replace all callers of `extract_clean_text()` with `extract_text_typed()`
5. Remove `extract_clean_text()` function entirely
6. Verify chat output formatting remains identical
**Acceptance**: `rg 'extract_clean_text' crates/ --type rust | grep -v test | wc -l` == 0
**Depends on**: none
**Effort**: M

### TASK-D26: Consolidate translate/mod.rs Stream Parsing
**Priority**: P1
**Category**: fix
**Files**: `crates/roko-agent/src/translate/mod.rs`, `crates/roko-agent/src/provider/claude_cli/stream.rs`
**What**: `translate/mod.rs` has two inline parsers: `extract_text()` and `extract_tool_outputs()` that duplicate stream-json parsing with 4096-byte truncation. Replace with calls to the canonical parser.
**Steps**:
1. In `extract_text()`: replace inline JSON parsing with `parse_stream_line()` calls, map `ClaudeStreamEvent` variants to the current output format
2. In `extract_tool_outputs()`: replace inline parsing with `parse_stream_line()`, extract tool name and content from `ClaudeStreamEvent::Tool` variants
3. Use the shared truncation utility (TASK-D27) instead of inline `4096` truncation
4. Verify all existing tests pass with the new parsing path
5. Remove dead inline parsing code
**Acceptance**: `rg 'serde_json::from_str.*result\|serde_json::from_str.*assistant' crates/roko-agent/src/translate/mod.rs | wc -l` == 0 (no more inline format guessing)
**Depends on**: TASK-D27
**Effort**: M

### TASK-D27: Extract Shared Truncation Utility
**Priority**: P1
**Category**: fix
**Files**: `crates/roko-agent/src/provider/claude_cli/stream.rs` (or new utility module)
**What**: Four copies of the 4096-byte char-boundary-safe truncation logic exist. Extract to a single utility function.
**Steps**:
1. Add `pub fn truncate_output(content: &str, max_bytes: usize) -> String` to the stream module or a shared utility
2. Implement char-boundary-safe truncation with `...[truncated {n} bytes]` suffix
3. Replace all 4 inline copies: `dispatch_direct.rs`, `translate/mod.rs` (x2), `chat.rs`
4. Make the default max configurable but default to 4096
5. Add unit tests for ASCII, UTF-8 multi-byte, and boundary-edge cases
**Acceptance**: `rg '\.is_char_boundary\(' crates/ --type rust | grep -v test | grep -v 'stream\|truncat' | wc -l` == 0 (all callers use the utility)
**Depends on**: none
**Effort**: S

---

## Phase 6: Environment Key Elimination

### TASK-D28: Migrate episode_completion.rs Off Direct Env Reads
**Priority**: P1
**Category**: fix
**Files**: `crates/roko-neuro/src/episode_completion.rs`
**What**: `episode_completion.rs` reads `ANTHROPIC_API_KEY` directly and builds its own `reqwest::Client` for distillation calls. Should receive a pre-configured agent through dependency injection.
**Steps**:
1. Add an `agent: Option<Box<dyn Agent>>` or `model_caller: Option<Arc<dyn ModelCaller>>` field to the `EpisodeCompleter` (or equivalent struct)
2. Provide a constructor that takes a `RokoConfig` and creates the agent via `create_agent_for_model()`
3. Replace the direct HTTP request with `agent.run(prompt, context).await`
4. Remove the `std::env::var("ANTHROPIC_API_KEY")` read
5. Use a cheap model (haiku) for distillation to minimize cost
6. Ensure the distillation call goes through the same cost tracking as other calls
**Acceptance**: `rg 'std::env::var.*ANTHROPIC_API_KEY' crates/roko-neuro/src/episode_completion.rs | wc -l` == 0
**Depends on**: none
**Effort**: M

### TASK-D29: Migrate web_search.rs Off Direct Env Reads
**Priority**: P1
**Category**: fix
**Files**: `crates/roko-std/src/tool/builtin/web_search.rs`
**What**: The web search builtin tool reads `PERPLEXITY_API_KEY` directly. Should receive the API key through `ToolContext` or a pre-configured provider client.
**Steps**:
1. Add a `perplexity_api_key: Option<String>` field to `ToolContext` (or add a `secrets` map if one doesn't exist)
2. In the web search handler, read from `ToolContext` instead of `std::env::var()`
3. Populate `ToolContext` from `RokoConfig.providers.perplexity.api_key_env` at tool dispatch setup time
4. Fall back to env var only if context key is missing (with deprecation warning)
5. Track the web search API call cost if possible
**Acceptance**: `rg 'std::env::var.*PERPLEXITY_API_KEY' crates/roko-std/src/tool/builtin/web_search.rs | wc -l` == 0 (excluding fallback)
**Depends on**: none
**Effort**: M

### TASK-D30: Audit and Migrate Remaining Direct Env Key Reads
**Priority**: P2
**Category**: fix
**Files**: `crates/roko-serve/src/routes/templates.rs`, `crates/roko-serve/src/routes/deployments.rs`, `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/learning_helpers.rs`
**What**: Multiple files read API keys directly from env for convenience. Audit all `std::env::var.*API_KEY` sites outside provider adapters and migrate to config-driven resolution.
**Steps**:
1. Run `rg 'std::env::var.*API_KEY' crates/ --type rust | grep -v test | grep -v provider/` to enumerate all sites
2. Categorize: (a) provider credential reads that should use config, (b) feature detection that should use config, (c) legitimate env reads (e.g., test setup)
3. For category (a): replace with `config.providers.get(name).and_then(|p| p.resolve_api_key())`
4. For category (b): replace with `config.providers.contains_key(name)` or similar
5. Document any remaining legitimate env reads with comments
**Acceptance**: `rg 'std::env::var.*API_KEY' crates/ --type rust | grep -v test | grep -v 'provider/\|adapter\|auth\|config'` count decreases by >= 50%
**Depends on**: TASK-D28, TASK-D29
**Effort**: M

---

## Phase 7: Model Selection Improvements

### TASK-D31: Add Thinking Token Tracking to UsageObservation
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-agent/src/usage.rs`, `crates/roko-agent/src/model_call_service.rs`
**What**: `UsageObservation` has no field for thinking/reasoning tokens. Models with thinking (Claude `--effort`, OpenAI o3/o4-mini, Gemini with reasoning) produce internal reasoning tokens that cost money but are invisible in usage reports.
**Steps**:
1. Add `thinking_tokens: Option<u64>` field to `UsageObservation`
2. Update `From<Usage> for UsageObservation` conversion (map from a new `Usage.thinking_tokens` field or leave as None)
3. Update the Claude CLI stream parser to extract thinking tokens from the Result event
4. Update the OpenAI-compat backend to extract `reasoning_tokens` from response usage
5. Update `CostTable` to use thinking-specific pricing (often cheaper than output tokens)
6. Include thinking tokens in episode records and cost calculations
**Acceptance**: `rg 'thinking_tokens' crates/roko-agent/src/usage.rs | wc -l` >= 1
**Depends on**: none
**Effort**: M

### TASK-D32: Replace ProviderQuirks Booleans with Struct
**Priority**: P2
**Category**: rearchitecture
**Files**: `crates/roko-agent/src/openai_compat_backend.rs`
**What**: The OpenAI-compat backend has accumulated per-provider boolean flags (`skip_session_fields`, `disable_parallel_tool_calls`, `normalize_tool_call_content`). Replace with a `ProviderQuirks` struct for cleaner extensibility.
**Steps**:
1. Define `ProviderQuirks` struct with all existing boolean fields plus future-proofing fields (max_tools, timeout_budget, image_format, content_type_constraints)
2. Add `ProviderQuirks::default()` with permissive defaults (all features enabled)
3. Add `ProviderQuirks::cerebras()`, `ProviderQuirks::deepseek()` etc. constructors for known strict providers
4. Replace individual boolean fields on `OpenAiCompatLlmBackend` with a single `quirks: ProviderQuirks` field
5. Replace individual `with_*` builder methods with `with_quirks(quirks: ProviderQuirks)`
6. Optionally: derive quirks from `ProviderConfig` or `ModelProfile` fields
**Acceptance**: `rg 'skip_session_fields|disable_parallel_tool_calls|normalize_tool_call_content' crates/roko-agent/src/openai_compat_backend.rs` shows references to `quirks.` instead of standalone fields
**Depends on**: none
**Effort**: M

### TASK-D33: Configurable Tool Loop Max Iterations from ModelProfile
**Priority**: P2
**Category**: fix
**Files**: `crates/roko-agent/src/provider/cerebras.rs`, `crates/roko-agent/src/provider/openai_compat.rs`, `crates/roko-core/src/config/schema.rs`
**What**: Tool loop iteration limits are hardcoded per-adapter (Cerebras: 50, OpenAI-compat: 30) instead of configurable from `ModelProfile` or `ProviderConfig`.
**Steps**:
1. Add `max_tool_iterations: Option<u32>` to `ModelProfile` in the config schema
2. Add `max_tool_iterations: Option<u32>` to `ProviderConfig` as a provider-level default
3. In each adapter's `create_agent()`, read max_iterations from: model profile -> provider config -> adapter default
4. Remove hardcoded `50` and `30` constants from adapter code
5. Document recommended values: 30 for large models, 50 for small models, 100 for autonomous agents
**Acceptance**: Changing `max_tool_iterations` in roko.toml model profile changes the actual iteration limit
**Depends on**: none
**Effort**: S

### TASK-D34: Auto-Populate CostTable from OpenRouter Metadata
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-agent/src/task_runner.rs`, `crates/roko-agent/src/provider/openrouter_meta.rs`
**What**: `CostTable` pricing is manually maintained. The OpenRouter metadata helper can fetch live pricing but is not wired to auto-populate. On startup, fetch pricing for configured models.
**Steps**:
1. Add `CostTable::merge_from_openrouter(profiles: &[OpenRouterModelProfile])` method
2. In `ModelCallService` construction, if an OpenRouter API key is configured, call `fetch_model_metadata()` for all configured models
3. Merge fetched pricing into `CostTable`, preferring locally-configured prices over fetched ones
4. Cache fetched pricing to `.roko/cache/pricing.json` with 24-hour TTL
5. On subsequent startups, use cached pricing if TTL hasn't expired
**Acceptance**: Model cost predictions use live pricing data after initial fetch
**Depends on**: none
**Effort**: M

### TASK-D35: Wire Knowledge Store Query into CascadeRouter
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-learn/src/cascade_router.rs`, `crates/roko-neuro/src/knowledge_store.rs`
**What**: The CascadeRouter does not query the neuro knowledge store for model selection. Historical knowledge about which models performed well for similar tasks should bias routing decisions.
**Steps**:
1. Add `knowledge_query: Option<Arc<dyn Fn(&str, usize) -> Vec<KnowledgeEntry>>>` field to `CascadeRouter`
2. In `select()`, before scoring candidates, query knowledge store for task-similar episodes
3. Extract model performance data from matched episodes (model_slug, success, latency, cost)
4. Use matched performance as a Bayesian prior that boosts or penalizes models
5. Weight knowledge prior by recency (newer episodes have more weight)
6. Add a config option `knowledge_routing_weight: f64` (default 0.3) to control how much knowledge influences routing
**Acceptance**: CascadeRouter with knowledge store prefers models that succeeded on similar tasks in history
**Depends on**: none
**Effort**: L

---

## Phase 8: orchestrate.rs Decomposition

### TASK-D36: Identify and Catalog Live Exports from orchestrate.rs
**Priority**: P1
**Category**: rearchitecture
**Files**: `crates/roko-cli/src/orchestrate.rs`, `crates/roko-cli/src/lib.rs`
**What**: Before decomposing the 21,577-line God object, enumerate exactly which symbols are imported by live (non-test, non-dead) code paths.
**Steps**:
1. Run `rg 'orchestrate::' crates/ --type rust | grep -v orchestrate.rs | grep -v test` to find all external imports
2. For each import, determine if the caller is live or dead code
3. Categorize live exports: (a) functions that should move to proper modules, (b) type definitions that should move, (c) functions that can be deleted
4. Document the dependency graph in a comment block at the top of orchestrate.rs
5. Tag each section with its target destination module
**Acceptance**: Documented catalog of live exports with target modules
**Depends on**: none
**Effort**: S

### TASK-D37: Extract Gate Failure Replan Logic
**Priority**: P1
**Category**: rearchitecture
**Files**: `crates/roko-cli/src/orchestrate.rs`, `crates/roko-orchestrator/src/replan.rs` (new or existing)
**What**: `build_gate_failure_plan_revision()` and related gate failure handling should move from the dead PlanRunner to `roko-orchestrator` where the live runner can use it.
**Steps**:
1. Extract `build_gate_failure_plan_revision()` function to `roko-orchestrator/src/replan.rs`
2. Extract any supporting types (gate failure context, revision plan format)
3. Make the function standalone (no dependency on PlanRunner struct)
4. Wire the extracted function into the live runner in `commands/plan.rs` or `runner/event_loop.rs`
5. Add tests using the extracted function directly
6. Remove the original from orchestrate.rs (or mark as delegating to the extracted version)
**Acceptance**: `rg 'build_gate_failure_plan_revision' crates/roko-orchestrator/ --type rust | wc -l` >= 1
**Depends on**: TASK-D36
**Effort**: M

### TASK-D38: Extract Context Bidding to roko-compose
**Priority**: P2
**Category**: rearchitecture
**Files**: `crates/roko-cli/src/orchestrate.rs`, `crates/roko-compose/src/context_bidding.rs` (new)
**What**: The VCG auction and `AttentionBidder` variants for context allocation in prompt assembly should live in `roko-compose` alongside `SystemPromptBuilder`.
**Steps**:
1. Extract `AttentionBidder` enum and variants (Neuro, Task, Research) to `roko-compose/src/context_bidding.rs`
2. Extract `vcg_allocate()` function and supporting types
3. Extract context budget calculation logic
4. Wire into `SystemPromptBuilder` as an optional composition step
5. The greedy path that currently dominates should remain as the default, with VCG as opt-in
6. Add unit tests for the extracted bidding logic
**Acceptance**: `rg 'AttentionBidder\|vcg_allocate' crates/roko-compose/ --type rust | wc -l` >= 2
**Depends on**: TASK-D36
**Effort**: M

### TASK-D39: Delete Dead PlanRunner and Unused Helpers
**Priority**: P2
**Category**: rearchitecture
**Files**: `crates/roko-cli/src/orchestrate.rs`
**What**: After extracting all valuable patterns (TASK-D37, TASK-D38), delete the dead `PlanRunner` struct and all methods/helpers that exist only to serve it.
**Steps**:
1. After TASK-D37 and TASK-D38 are complete, re-run the live export catalog
2. Delete `PlanRunner` struct definition and all `impl PlanRunner` blocks
3. Delete `dispatch_agent_with()` and all private callers
4. Delete `run_task_plans()` and all private callers
5. Delete private helper functions that have no external callers
6. Keep any still-imported symbols (move them to their proper module if they haven't been moved yet)
7. Verify `cargo build --workspace` succeeds
8. Target: reduce orchestrate.rs from 21,577 lines to < 3,000
**Acceptance**: `wc -l crates/roko-cli/src/orchestrate.rs` < 5000
**Depends on**: TASK-D37, TASK-D38
**Effort**: L

---

## Phase 9: Novel Dispatch Strategies

### TASK-D40: Implement Speculative Dispatch
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-agent/src/model_call_service.rs`
**What**: For latency-sensitive paths (interactive chat), dispatch to a fast model (Haiku/Flash) while simultaneously starting a premium model. If the fast result passes a quality threshold, cancel the slow model and return immediately.
**Steps**:
1. Add `pub async fn call_speculative(&self, req: ModelCallRequest, quality_threshold: f64) -> Result<ModelCallResponse>`
2. Determine fast_model (cheapest/fastest from config) and premium_model (resolved by cascade router)
3. If fast_model == premium_model, just call once
4. Spawn both calls via `tokio::spawn`, await the fast one first
5. Define quality_score heuristic: output length > minimum, no error markers, model confidence if available
6. If fast result quality >= threshold, abort premium `JoinHandle` and return fast result
7. Otherwise await premium result, abort fast if still running
8. Account for cost of both calls in budget (both consume tokens even if one is cancelled)
9. Record both calls as CascadeRouter observations (fast model gets reward proportional to quality, premium gets full reward)
**Acceptance**: Unit test: speculative dispatch returns fast model result when quality is high, premium result when quality is low
**Depends on**: TASK-D06, TASK-D22
**Effort**: L

### TASK-D41: Implement Ensemble Dispatch
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-agent/src/model_call_service.rs`, `crates/roko-agent/src/composition.rs`
**What**: For high-stakes decisions (architecture, security review), dispatch the same prompt to N models in parallel and aggregate results. Leverage the existing `MergeStrategy` from `composition.rs`.
**Steps**:
1. Add `pub async fn call_ensemble(&self, req: ModelCallRequest, models: &[String]) -> Result<ModelCallResponse>`
2. Fan out to all models via `futures::future::join_all`
3. Collect all successful results (tolerate individual failures if >= 2 succeed)
4. Apply `MergeStrategy` from `composition.rs`: `Vote` for structured outputs, `BestOfN` for text generation
5. Return merged result with usage aggregated across all models
6. Budget accounts for all N calls
7. Record each model's individual result as a CascadeRouter observation
8. Add a config option `ensemble_models: Vec<String>` for pre-configured ensemble sets
**Acceptance**: Ensemble with 3 models returns a merged result; individual model failures don't break the ensemble
**Depends on**: TASK-D06
**Effort**: M

### TASK-D42: Implement Cost-Optimized Batch Routing
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-agent/src/model_call_service.rs`, `crates/roko-orchestrator/src/lib.rs`
**What**: For plan execution with many tasks, pre-compute an optimal model assignment that minimizes total cost while meeting quality requirements within budget.
**Steps**:
1. Add `pub fn plan_batch_routing(&self, tasks: &[TaskSpec], total_budget: f64) -> Vec<(TaskId, String)>`
2. Sort tasks by estimated complexity (from task definition metadata or heuristic)
3. Assign cheapest viable model to each task (viability = model supports required capabilities)
4. Calculate total estimated cost using `CostTable`
5. If total > budget: further downgrade low-complexity tasks
6. If total < budget * 0.7: consider upgrading high-complexity tasks to premium models
7. Apply CascadeRouter confidence scores as soft constraints (prefer models with high confidence for the task type)
8. Return the assignment map; the plan runner uses it for model selection per task
9. Log the batch routing plan for observability
**Acceptance**: Given 10 tasks and a $5 budget, produces a valid model assignment where all tasks have a model and total estimated cost <= $5
**Depends on**: TASK-D06, TASK-D22
**Effort**: L

---

## Phase 10: Observability and Dashboard Integration

### TASK-D43: Emit RouterDecision Events
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-agent/src/model_call_service.rs`, `crates/roko-core/src/runtime_event.rs`
**What**: When the CascadeRouter selects a model, emit a structured event with the routing decision details (policy mode, candidate scores, chosen model, reason).
**Steps**:
1. Add `RuntimeEvent::RouterDecision` variant with fields: policy, candidates (Vec of model/score/reason), chosen model, estimated_cost
2. In `ModelCallService::call()`, after model resolution, emit the decision event
3. Include the `SelectionSource` from `EffectiveModelSelection` in the event
4. Include CascadeRouter confidence scores for top-3 candidates
5. Wire through event_consumers for JSONL logging and TUI consumption
**Acceptance**: `rg 'RuntimeEvent::RouterDecision' crates/ --type rust | wc -l` >= 2
**Depends on**: TASK-D06, TASK-D07
**Effort**: S

### TASK-D44: Add Cache Metrics to ModelCallService
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-agent/src/model_call_service.rs`
**What**: The L1 response cache (`CacheCell`, 128 entries) has no metrics. Add hit rate tracking, eviction counts, and size monitoring. Separately, track Anthropic server-side cache utilization.
**Steps**:
1. Add `CacheMetrics` struct: hits, misses, evictions, size_entries, estimated_savings_usd
2. Increment counters in `CacheCell::get()` (hit/miss) and `CacheCell::insert()` (eviction if at capacity)
3. Calculate `estimated_savings_usd` as: hits * average_call_cost
4. Track Anthropic prompt cache utilization from `cache_read_tokens` / `cache_creation_tokens` in usage
5. Expose via `pub fn cache_metrics(&self) -> &CacheMetrics`
6. Emit periodic `RuntimeEvent::CacheMetrics` for dashboard consumption
**Acceptance**: After 10 model calls, `cache_metrics()` returns non-zero hit or miss counts
**Depends on**: none
**Effort**: S

---

## Dependency Graph

```
Phase 0 (CascadeRouter)             Phase 2 (Dispatch Unification)
  D01 ──> D02 ──> D05                D12 ──> D13
  D01 ──> D03                        D14 (independent)
  D01 ──> D04 ──> D06                D15 (independent)
         D06 ──> D04                  D11 ──> D07

Phase 1 (Episode Logging)           Phase 3 (Health)
  D07 ──> D08                        D16 ──> D17
  D07 ──> D09                        D18 (independent)
  D07 ──> D10                        D17 ──> D19
                                     D20 (independent)

Phase 4 (Budget)                    Phase 5 (Parser)
  D21 ──> D22 ──> D23                D27 ──> D26
  D21 ──> D24                        D25 (independent)
  D22 ──> D24

Phase 6 (Env Keys)                  Phase 7 (Model Selection)
  D28 (independent)                  D31 (independent)
  D29 (independent)                  D32 (independent)
  D28,D29 ──> D30                    D33 (independent)
                                     D34 (independent)
                                     D35 (independent)

Phase 8 (orchestrate.rs)            Phase 9 (Novel Strategies)
  D36 ──> D37                        D06,D22 ──> D40
  D36 ──> D38                        D06 ──> D41
  D37,D38 ──> D39                    D06,D22 ──> D42

Phase 10 (Observability)
  D06,D07 ──> D43
  D44 (independent)
```

---

## Critical Path (P0 tasks, must-do order)

1. **D01** Load CascadeRouter at startup
2. **D06** Record observations in ModelCallService
3. **D07** Add episode emission to ModelCallService
4. **D02** Thread router through resolve_effective_model_key
5. **D03** Wire into `roko run`
6. **D04** Wire into `roko chat`
7. **D05** Wire into `roko plan run`
8. **D08** Wire episode logger to one-shot paths
9. **D11** Migrate chat_inline to ModelCallService
10. **D12** Migrate ACP runner to provider system
11. **D13** Migrate ACP bridge events to provider system

These 11 tasks take CascadeRouter from zero live callers to fully wired with learning
feedback from all major paths, and eliminate the two remaining bare subprocess spawns.

---

## Estimated Effort Summary

| Phase | Tasks | Effort | Priority |
|---|---|---|---|
| 0: CascadeRouter wiring | D01-D06 | 3-5 days | P0 |
| 1: Episode logging | D07-D10 | 2-3 days | P0-P1 |
| 2: Dispatch unification | D11-D15 | 3-5 days | P0-P1 |
| 3: Provider health | D16-D20 | 3-5 days | P1-P2 |
| 4: Budget enforcement | D21-D24 | 2-3 days | P1-P2 |
| 5: Parser consolidation | D25-D27 | 2-3 days | P1 |
| 6: Env key elimination | D28-D30 | 2-3 days | P1-P2 |
| 7: Model selection | D31-D35 | 3-5 days | P1-P2 |
| 8: orchestrate.rs decomp | D36-D39 | 4-6 days | P1-P2 |
| 9: Novel strategies | D40-D42 | 3-5 days | P2 |
| 10: Observability | D43-D44 | 1-2 days | P2 |
| **Total** | **44 tasks** | **28-45 days** | |

---

## Grep Gates (Final Acceptance)

After all tasks complete, these commands should produce the indicated results:

```bash
# No bare claude spawns outside provider adapters
rg 'Command::new\("claude"\)' crates/ --type rust | grep -v test | grep -v 'provider/\|adapter'
# Expected: 0 results

# No direct API_KEY env reads outside provider adapters and config
rg 'std::env::var.*API_KEY' crates/ --type rust \
  | grep -v test | grep -v 'provider/\|adapter\|config/schema\|auth\.rs'
# Expected: 0 results

# extract_clean_text removed
rg 'extract_clean_text' crates/ --type rust | grep -v test
# Expected: 0 results

# CascadeRouter has live callers
rg 'load_cascade_router\|CascadeRouter::load' crates/roko-cli/src/ --type rust | grep -v test
# Expected: >= 3 results

# ModelCallService records router observations
rg 'cascade_router.*observe\|observe.*cascade' crates/roko-agent/src/model_call_service.rs
# Expected: >= 1 result

# Budget default is non-None
rg 'BudgetCell::new\(None\)' crates/roko-agent/src/model_call_service.rs
# Expected: 0 results (outside test code)

# orchestrate.rs is significantly smaller
wc -l crates/roko-cli/src/orchestrate.rs
# Expected: < 5000 lines

# Episode logging from all paths
ls -la .roko/episodes.jsonl .roko/learn/efficiency.jsonl
# Expected: both files exist and grow during normal use
```
