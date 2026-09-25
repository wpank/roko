# 05 -- Dispatcher Architecture

> **Implementation status (2026-09):** Shipping. The ToolDispatcher's
> 7-step pipeline, batch dispatch with concurrency partitioning,
> ingress validation, immune screening, and 12 submodules are all live.

---

## Overview

The `ToolDispatcher` at `crates/roko-agent/src/dispatcher/mod.rs`
processes every tool call through a 7-step pipeline. It is the
enforcement boundary between the LLM provider's tool-call output and
the execution of host-side tool handlers.

The dispatcher does not cache results internally. Every call must reach
current authorization, durable immune control, screening,
finalization, and terminal audit state. This is a deliberate design
decision: caching at this layer would allow stale authorization or
expired safety decisions to leak through.

---

## The 7-Step Pipeline

Each tool call passes through these stages in order. The first failure
at any stage short-circuits and the error is returned to the caller.

```
1. VALIDATE   -- identity + args against JSON schema from registry
2. AUTHORIZE  -- profile/task filters and role capabilities
3. SAFETY     -- hooks, policy, durable immune controls
4. EXECUTE    -- handler under timeout/cancellation, panic-catching
5. BOUND      -- recursively scrub, recover, re-bound results
6. SCREEN     -- finalized result through fixed immune Graph
7. FINALIZE   -- emit one sanitized terminal audit signal
```

### Step 1: VALIDATE

Validates the tool call's identity and arguments:

- `validate_tool_call_identity` checks that the call's tool name
  matches a registered `ToolDef` in the registry.
- Argument validation checks against the JSON schema defined in the
  `ToolDef`.
- Calls `validate_tool_call_ingress` for frame-level size checks.

### Step 2: AUTHORIZE

Applies profile and task filters:

- Role capabilities from `AgentContract` determine which tools the
  current role may invoke.
- Task-specific allowlists from the plan definition further restrict
  the available tools.
- Unknown roles deny all tools (fail-closed).

### Step 3: SAFETY

The SafetyLayer composes six policy families (see
`depth/05-agent/safety-layer.md`). Additionally:

- Pre-execution hooks from `hook_chain` run in order.
- The `production_safety_chain` submodule applies the mandatory
  9-stage production checks when enabled.
- Durable immune controls from `check_tool_control` are evaluated.
- Untrusted sources are checked via `is_untrusted_source`.

### Step 4: EXECUTE

The handler is invoked under timeout and cancellation:

```rust
// Simplified execution path
let result = with_timeout(
    timeout,
    wait_cancelled(cancel_token, handler.execute(&call, &ctx)),
).await;
```

**Panic catching:** The dispatcher installs a custom panic hook that
suppresses handler panic payloads. A thread-local
`HANDLER_PANIC_POLL_DEPTH` counter tracks whether the current poll is
inside a handler future. When a panic occurs during handler execution,
the payload is suppressed (never formatted) to prevent
attacker-controlled panic payloads from leaking into logs. Unrelated
panics are forwarded to the previous hook.

```rust
struct HandlerPanicPollGuard;

impl HandlerPanicPollGuard {
    fn enter() -> Self {
        HANDLER_PANIC_POLL_DEPTH.with(|d| d.set(d.get() + 1));
        Self
    }
}

impl Drop for HandlerPanicPollGuard {
    fn drop(&mut self) {
        HANDLER_PANIC_POLL_DEPTH.with(|d| d.set(d.get() - 1));
    }
}
```

The `HandlerFutureLifecycle<F>` wrapper owns the handler future so
both polling and destruction occur under the payload-suppressing scope.
This ensures that even if a timeout or cancellation drops the future,
the destructor unwind does not leak sensitive information.

### Step 5: BOUND

Post-execution result processing:

- Truncate oversized results via `truncate_result` to
  `DEFAULT_MAX_RESULT_BYTES`.
- Recursively scrub sensitive content from result payloads.
- Recover from partial results when the handler produced output before
  failing.
- Re-bound the result to enforce size limits after scrubbing.

### Step 6: SCREEN

The finalized result is screened through the fixed immune Graph:

- `screen_tool_result` runs the result through the five-stage immune
  decision Graph.
- Results that fail screening are replaced with a sanitized error.

### Step 7: FINALIZE

Emit one sanitized terminal audit signal:

- The audit signal contains the tool name, arguments (scrubbed),
  result summary, timing, and success/failure status.
- The signal is emitted as a `Kind::ToolAudit` Signal via the
  event bus.

---

## Batch Dispatch

`dispatch_batch` processes a turn's worth of tool calls. It groups
calls by `ToolConcurrency`:

### Concurrency partitioning

```rust
pub fn partition_by_concurrency(
    calls: &[ToolCall],
    registry: &ToolRegistry,
) -> (Vec<ToolCall>, Vec<ToolCall>)
// Returns: (parallel_calls, serial_calls)
```

- **Parallel** tools (read_file, grep, glob) run through a bounded
  unordered stream via `futures::future::join_all`. Concurrency is
  limited to `DEFAULT_MAX_CONCURRENT_TOOLS`.
- **Serial** tools (bash, write_file) run sequentially to preserve
  ordering and avoid write-write races.

Results contain the parallel bucket first and the serial bucket last.

### Ingress validation

Before dispatch, the batch is validated:

```rust
pub fn validate_tool_call_ingress(calls: &[ToolCall])
    -> Result<(), &'static str>
{
    if calls.len() > MAX_TOOL_CALLS_PER_BATCH {
        return Err("call_count");
    }
    for call in calls {
        validate_tool_call_identity(call)?;
        bounded_serialized_bytes(call, MAX_TOOL_CALL_INGRESS_BYTES)?;
    }
    bounded_serialized_bytes(calls, MAX_TOOL_CALL_FRAME_BYTES)?;
    Ok(())
}
```

Error reasons are deliberately fixed reason codes: rejected identities
and arguments must never be reflected into any host-visible diagnostic.

---

## Limits

| Constant | Value | Purpose |
|----------|-------|---------|
| `MAX_TOOL_CALLS_PER_BATCH` | 16 | Per-turn call cap |
| `MAX_TOOL_CALL_INGRESS_BYTES` | 256 KB | Per-call size limit |
| `MAX_TOOL_CALL_FRAME_BYTES` | 1 MB | Per-frame size limit |
| `MAX_TOOL_BATCH_RESULT_BYTES` | 8 MB | Aggregate result payload cap |
| `DEFAULT_MAX_RESULT_BYTES` | (from roko-core) | Per-result truncation limit |
| `DEFAULT_MAX_CONCURRENT_TOOLS` | (from roko-core) | Parallel tool concurrency cap |

---

## HandlerResolver

The dispatcher does not depend on `roko-std` (the crate that provides
the 16 built-in tool handlers). Instead, callers pass a
`HandlerResolver` trait implementation:

```rust
pub trait HandlerResolver: Send + Sync {
    fn resolve(&self, tool_name: &str)
        -> Option<Arc<dyn ToolHandler>>;
}
```

Typically the resolver closes over `roko_std::tool::handler_for`,
keeping the `roko-agent` crate free of the `roko-std` dependency. This
layering decision is documented as M19 in MISTAKES-LEARNED.md: the
mistake was originally inverting the dependency so that backends pulled
in the entire standard tool library. The `HandlerResolver` trait
corrects this.

A blanket implementation is provided for closures:

```rust
impl<F> HandlerResolver for F
where
    F: Fn(&str) -> Option<Arc<dyn ToolHandler>> + Send + Sync,
{
    fn resolve(&self, name: &str) -> Option<Arc<dyn ToolHandler>> {
        (self)(name)
    }
}
```

---

## ToolDispatcher Structure

```rust
pub struct ToolDispatcher {
    registry: Arc<dyn ToolRegistry>,
    resolver: Arc<dyn HandlerResolver>,
    max_result_bytes: usize,
    safety: SafetyLayer,
    hook_chain: Option<SafetyHookChain>,
    production_safety_chain:
        Option<ProductionSafetyChain>,
    tool_selector: Option<ToolSelector>,
    safety_denial_callback: Option<SafetyDenialCallback>,
    file_audit: Option<Arc<ScrubAuditAdapter>>,
}
```

The `production_safety_chain` is mandatory for production constructors.
It contains stages 5-7 of the 9-stage enforcement (hallucination
detector, taint ceiling, corrigibility) and stage 9 as the
post-handler result filter. Stages 1-4 are inline in `SafetyLayer`.
It is kept separate from the extension hook chain so callers cannot
replace production safety hooks by attaching a custom chain.

### EffectiveCatalogSnapshot

```rust
pub struct EffectiveCatalogSnapshot {
    pub tool_count: usize,
    pub execution_owner: String,
    pub policy_owner: String,
    pub selector_active: bool,
    pub hook_chain_active: bool,
    pub production_hooks_active: bool,
}
```

Records the exact authorization state at dispatch time so that audit
and replay can reconstruct what applied.

---

## Submodules

| Module | Purpose |
|--------|---------|
| `alert` | Alert emission for safety-critical tool events |
| `cancel` | Cancellation primitives; `wait_cancelled` wraps futures with a cancel token |
| `dedup_cache` | Dispatch-level dedup for idempotent agent dispatch (DEPLOY-09) |
| `emit_metric` | Metric emission for telemetry |
| `hook_chain` | Pre/post execution hook chains |
| `parallel` | Concurrency partitioning by `ToolConcurrency` |
| `production_safety_chain` | Mandatory production safety hooks (stages 5-7, 9) |
| `result_cache` | Explicit cache primitives (dispatcher does NOT cache internally) |
| `timeout` | Timeout enforcement via `with_timeout` |
| `tool_selector` | Tool selection logic for context-aware tool filtering |
| `truncate` | Result truncation and bounding; `bounded_json_bytes`, `bounded_serialized_bytes` |
| `validate` | Input validation against ToolDef JSON schemas |

---

## Interaction with ToolLoop

The ToolLoop calls the dispatcher once per turn:

```
ToolLoop turn:
  1. LlmBackend.send_turn(messages, tools) -> BackendResponse
  2. Translator.parse_calls(response) -> Vec<ToolCall>
  3. ToolDispatcher.dispatch_batch(calls, ctx) -> Vec<ToolResult>
  4. Translator.render_results(calls_and_results) -> RenderedResults
  5. Append results to messages
  6. Repeat until stop condition
```

The dispatcher is stateless between turns. The ToolLoop maintains
conversation state; the dispatcher processes each batch independently.

---

## Interaction with SafetyLayer

The dispatcher holds a reference to a `SafetyLayer` configured at
construction time. The SafetyLayer's `check_pre_execution` method is
called in Step 3 (SAFETY) and applies all six policy families:

1. **BashPolicy** -- command allowlist/denylist for bash tool calls.
2. **PathPolicy** -- worktree escape prevention for file tools.
3. **NetworkPolicy** -- destination allowlist for network tools.
4. **GitPolicy** -- branch protection rules.
5. **ScrubPolicy** -- secrets removal from outputs (applied in Step 5).
6. **RateLimiter** -- per-tool/per-role rate limits.

Additionally, the `AgentContract` and `AgentWarrant` from the
SafetyLayer enforce role-level tool access control.

---

## Implementation Sources

| File | Purpose |
|------|---------|
| `crates/roko-agent/src/dispatcher/mod.rs` | ToolDispatcher, 7-step pipeline, batch dispatch |
| `crates/roko-agent/src/dispatcher/parallel.rs` | partition_by_concurrency |
| `crates/roko-agent/src/dispatcher/validate.rs` | Input validation |
| `crates/roko-agent/src/dispatcher/truncate.rs` | Result truncation and bounding |
| `crates/roko-agent/src/dispatcher/timeout.rs` | Timeout enforcement |
| `crates/roko-agent/src/dispatcher/cancel.rs` | Cancellation primitives |
| `crates/roko-agent/src/dispatcher/hook_chain.rs` | Pre/post execution hooks |
| `crates/roko-agent/src/dispatcher/production_safety_chain.rs` | Production safety hooks |
| `crates/roko-agent/src/tool_immune.rs` | check_tool_control, screen_tool_result, validate_tool_call_identity |

---

## Citations

1. `crates/roko-agent/src/dispatcher/mod.rs` -- ToolDispatcher, 7-step
   pipeline, batch dispatch, ingress validation.
2. `crates/roko-agent/src/tool_immune.rs` -- Immune boundary functions.
3. MISTAKES-LEARNED.md M19 -- HandlerResolver layering decision.
4. `crates/roko-core/src/tool/` -- ToolDef, ToolCall, ToolResult,
   ToolHandler, ToolRegistry, ToolConcurrency.
