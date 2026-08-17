# 23 - Dispatch and Streaming Migration Plan

Goal: one provider/model dispatch contract across ACP, chat, CLI one-shot, runner, and serve. Surface crates may assemble prompts and map events to their wire protocols, but they must not resolve provider transports, synthesize providers, read provider auth env vars directly, build provider HTTP payloads, parse provider SSE, or silently substitute providers.

## Sequencing Blockers

Do these first as a small ACP compatibility patch; they are wire-contract bugs, not redesign work.

1. `crates/roko-acp/src/types.rs`: make `ContentBlock::Text` serialize as `{ "type": "text", ... }`; keep inbound aliases only if needed.
2. `crates/roko-acp/src/bridge_events.rs`: fix `send_session_update` to merge `sessionId` into the tagged `SessionUpdate` object instead of nesting under `update`.
3. `crates/roko-acp/src/bridge_events.rs`: replace terminal-event `unreachable!()` with a typed no-op/error path and add `CognitiveEvent::Failed { message }`; failed dispatches must not emit `Complete { EndTurn }`.
4. `crates/roko-acp/src/bridge_events.rs` + `session.rs`: preserve conversation history in structured messages and make `begin_prompt` an atomic compare-and-set before larger dispatch changes land.

## Target Ownership

- `roko-core`: owns dispatch request/plan/result/event types, provider capability/auth enums, optional usage types, and validated config shapes.
- `roko-agent`: owns provider adapters, streaming parsers, HTTP clients, subprocess launchers, retries, usage normalization, and `ModelCallService`.
- `roko-cli`: owns terminal/chat UI state and prompt assembly only; consumes shared dispatch/streaming APIs.
- `roko-acp`: owns ACP protocol mapping only; consumes shared dispatch/streaming APIs and maps stream events to `SessionUpdate`.
- `roko-serve`: owns HTTP/WebSocket API projection and server state only; consumes shared dispatch/streaming APIs for template dispatch, provider tests, and agent messaging.

## New Shared Contract

Add `crates/roko-core/src/dispatch_plan.rs` and re-export from `roko-core`.

Core types:

- `DispatchRequest`: caller surface (`Acp`, `CliChat`, `CliOneShot`, `Runner`, `Serve`), workdir, role, prompt/messages, model override, provider override, routing context, required capabilities, generation options, cache policy, budget, permission policy, and fallback policy.
- `DispatchRequirement`: `streaming`, `tools`, `mcp_tools`, `vision`, `thinking`, `web_search`, `resume`, `editor_mediated_auth`, `side_effects`.
- `DispatchPlan`: the only execution-authorizing object. Contains requested model/provider, effective model key, concrete model slug, provider id, provider kind, provider config snapshot, model profile snapshot, transport plan, validated auth status, validated capabilities, routing trace, config provenance, fallback candidates, and ambiguity diagnostics.
- `TransportPlan`: `Cli { command, args, protocol }`, `Http { base_url, auth, protocol }`, `Acp { command_or_endpoint, protocol }`, `Unsupported { reason }`.
- `FallbackPolicy`: `Disabled`, `ConfigOrdered(Vec<ModelKey>)`, `SameProviderOnly`, `AllowCrossProvider { reason }`. CLI/user hard overrides default to `Disabled`.
- `DispatchAttempt`: per-attempt model/provider/transport plus `primary` or `fallback`.
- `DispatchError`: typed `MissingAuth`, `UnsupportedProvider`, `CapabilityMismatch`, `AmbiguousProvider`, `AmbiguousModel`, `ProviderFailure`, `Cancelled`, `BudgetExceeded`, `ConfigInvalid`.

Migration note: `crates/roko-cli/src/dispatch/mod.rs::DispatchPlan` is runner-local and prompt-specific. Rename it to `RunnerDispatchPlan` or embed the new `roko_core::DispatchPlan` inside it so the name `DispatchPlan` means the shared execution contract everywhere.

## Shared Streaming API

Extend `crates/roko-core/src/foundation.rs`:

- Change `TokenUsage` to optional fields or replace it with promoted `UsageObservation`; unknown provider usage must remain unknown until UI display.
- Add `ModelStreamEvent`: `Started`, `ContentDelta`, `ReasoningDelta`, `ToolCallStart`, `ToolCallDelta`, `ToolCallComplete`, `Usage(UsageObservation)`, `AttemptFailed`, `Completed { stop_reason, final_model, final_provider }`, `Failed { error }`, `Cancelled`.
- Add `type ModelStream = Pin<Box<dyn Stream<Item = Result<ModelStreamEvent>> + Send>>`.
- Extend `ModelCaller` with `plan(req) -> DispatchPlan`, `call(plan, req) -> ModelCallResponse`, and `stream(plan, req) -> ModelStream`.

Implement in `crates/roko-agent`:

- Move `ProviderDispatchResolver` out of `crates/roko-cli/src/dispatch_v2.rs` into `roko-core` or `roko-agent` so ACP/serve can use it without depending on CLI.
- Extend `ProviderAdapter` or add `StreamingProviderAdapter` with `stream(...)`. One-shot providers synthesize a stream from `AgentResult`; native providers expose true streaming.
- Reuse `crates/roko-agent/src/runtime_events.rs` and `streaming.rs`, but make `ModelStreamEvent` the service boundary. `AgentRuntimeEvent`, ACP `CognitiveEvent`, CLI rendering, and serve SSE become adapters from that boundary.
- Provider HTTP/SSE parsing belongs only in `roko-agent/src/provider/*`, using shared HTTP clients from `roko-agent/src/http.rs`.

## Phase Plan

### Phase 1 - Resolver and Validation

Owned modules: `roko-core/src/dispatch_plan.rs`, `roko-core/src/config/provider.rs`, `roko-core/src/config/schema.rs`, `roko-agent/src/model_call_service.rs`, `roko-cli/src/dispatch_v2.rs`, `roko-cli/src/model_selection.rs`.

1. Build `DispatchResolver::resolve(request, ResolvedConfig) -> Result<DispatchPlan>`.
2. Move runtime provider synthesis out of `ModelCallService::config_for_model`; config load/migration must produce all providers/models once with provenance.
3. Replace sorted-map fallback rules with explicit config priority or ambiguity errors.
4. Validate auth before execution. Env var lookup is allowed in config/auth/provider boundaries only; surfaces receive `MissingAuth { provider_id, auth_method }`.
5. Validate capabilities before execution. Examples: ACP/chat require streaming; Cursor ACP must not become OpenAI HTTP; MCP/tool requests require model/provider support.
6. Keep `model_selection::EffectiveModelSelection` as display/selection metadata only, derived from `DispatchPlan`, never used as execution proof.

### Phase 2 - Streaming Service

Owned modules: `roko-core/src/foundation.rs`, `roko-agent/src/model_call_service.rs`, `roko-agent/src/provider/mod.rs`, `roko-agent/src/provider/anthropic_api.rs`, `openai_compat.rs`, `cerebras.rs`, `claude_cli.rs`, `cursor_acp.rs`, `runtime_events.rs`, `streaming.rs`, `usage.rs`.

1. Add `ModelCaller::stream(plan, req)`.
2. Implement Anthropic Messages SSE, OpenAI-compatible SSE, Cerebras/OpenAI-compatible SSE, Claude CLI stream-json, and Cursor ACP event streams behind provider adapters.
3. Normalize stop reasons, tool events, reasoning deltas, usage, retries, and errors into `ModelStreamEvent`.
4. Emit `AttemptFailed` for retryable failures and `Failed` for terminal failures. Never encode dispatch failure as content text plus normal completion.
5. Preserve requested vs attempted vs final model/provider in every response/event.

### Phase 3 - ACP Thin Consumer

Owned modules: `crates/roko-acp/src/bridge_events.rs`, `session.rs`, `acp_adapter.rs`, `runner.rs`.

1. Replace `run_anthropic_cognitive_task` and `run_openai_compat_cognitive_task` with `ModelCallService::stream`.
2. Build `DispatchRequest { caller: Acp, requirements: streaming + tools/mcp as configured, routing_context: acp_routing_context(...) }`.
3. Map `ModelStreamEvent` to `CognitiveEvent`, then to flat ACP `SessionUpdate`.
4. On `MissingAuth`, `UnsupportedProvider`, or `CapabilityMismatch`, emit an ACP error/failure result, not `Complete`.
5. Keep workflow engine path separate initially, but make workflow phases use the same resolver/service after the single-agent ACP path is stable.

### Phase 4 - Chat and CLI

Owned modules: `crates/roko-cli/src/chat_session.rs`, `chat_inline.rs`, `auth_detect.rs`, `unified.rs`, `dispatch_direct.rs`, `dispatch_v2.rs`, `dispatch/mod.rs`.

1. Replace `ChatAgentSession::send_turn_api` raw HTTP with `ModelCallService::call/stream`. **Changed 2026-05-01:** chat API dispatch now builds `ModelCallRequest` and consumes `ModelCallService::stream`.
2. Replace `build_streaming_command` hardcoded Claude CLI construction with `DispatchPlan.transport`.
3. Make `/model` atomic: failed resolution leaves `model_selection` and display state unchanged.
4. Change auth detection from "which binary/env exists" to "which configured `DispatchPlan` is executable"; `claude --version` is installation evidence, not login proof.
5. Remove `unified.rs` and `chat_inline.rs` production fallback to `dispatch_direct`.
6. Keep any raw diagnostic dispatch behind an explicit dev-only feature, not reachable from normal commands.

### Phase 5 - Serve

Owned modules: `crates/roko-serve/src/dispatch.rs`, `state.rs`, `routes/providers.rs`, `routes/agents.rs`.

1. Make `AppState` own a shared `DispatchResolver`/`ModelCallService` bundle built from validated config.
2. Replace template `create_agent_for_model` calls with `DispatchPlan` + service execution.
3. Replace provider test route with a minimal `DispatchRequest`/`ModelCallRequest` so auth, capability validation, health, latency, and usage flow through the same path. **Changed 2026-05-01:** `POST /api/providers/{id}/test` now uses `ModelCallService::call`; capability/auth validation is still not fully typed.
4. For agent message streaming, map `ModelStreamEvent` to serve SSE/WebSocket output; sidecar proxying can remain separate because it is not provider dispatch.

### Phase 6 - Deletion and Quarantine

Delete or quarantine after all consumers are migrated:

- `crates/roko-acp/src/bridge_events.rs`: ACP-local Anthropic/OpenAI streaming functions, local Claude stream structs, direct provider env lookups.
- `crates/roko-cli/src/chat_session.rs`: hardcoded `claude-haiku-4-5` fallback, provider-key preflight outside provider ownership, and any remaining compatibility state duplication. Direct API provider HTTP was removed from `send_turn_api` on 2026-05-01.
- `crates/roko-cli/src/dispatch_direct.rs`: remove from production imports and `lib.rs`; keep only test/dev diagnostic if still needed.
- `crates/roko-agent/src/model_call_service.rs`: runtime provider/model synthesis based on env/model prefix.
- `crates/roko-cli/src/dispatch_v2.rs`: resolver types after they move to core/agent; leave only CLI invocation helpers if still needed.
- Any surface use of provider API env vars except config/auth diagnostics.

## Fallback Semantics

- User hard override (`--model`, `/model`, explicit ACP model config) means no fallback unless the user/config explicitly opts in.
- Config fallback order is declared, not sorted-map derived.
- Fallback candidates must satisfy the same `DispatchRequirement`s as the primary.
- Cross-provider fallback requires an explicit reason and must emit an attempt event; `ClaudeCli -> AnthropicApi` is never implicit.
- Construction/auth errors are terminal unless the fallback policy explicitly allows trying a different already-authenticated provider.
- Final responses and telemetry record requested, primary attempted, every fallback attempted, and final model/provider separately.

## Recurrence Checks

Add CI or pre-merge scripts:

- Fail on provider HTTP construction in surfaces: `rg 'https://api\\.anthropic\\.com|/v1/messages|/chat/completions|anthropic-version|x-api-key|bearer_auth' crates/roko-cli crates/roko-acp crates/roko-serve`.
- Fail on provider auth env reads outside config/auth/provider modules: `rg 'std::env::var\\(\"(ANTHROPIC_API_KEY|OPENAI_API_KEY|ZAI_API_KEY)' crates/roko-cli crates/roko-acp crates/roko-serve`.
- Fail on production imports of `dispatch_direct`.
- Fail on `ProviderKind::ClaudeCli` branches that call Anthropic API or OpenAI-compatible API helpers.
- Fail on `unwrap_or(0)` in provider usage parsers unless the type is a UI display type.
- Golden ACP serialization tests for `ContentBlock` and flat `session/update`.
- Config validation test: provider names, provider kinds, transports, auth, and model references cannot be ambiguous.

## Acceptance Tests

1. ACP golden fixture: `session/update` is flat and text chunks serialize as `{ "type": "text" }`.
2. ACP with `claude_cli` configured and no `ANTHROPIC_API_KEY`: returns typed unsupported/missing auth for the configured transport, not an Anthropic API error and not normal completion.
3. ACP Anthropic/OpenAI-compatible mock SSE streams produce identical `ModelStreamEvent` sequences and editor chunks through the shared service.
4. CLI chat dispatch recurrence check proves `chat_session.rs` contains no provider endpoint/header/body construction; provider-layer parity tests verify system prompt placement for Anthropic and native message-array handling where adapters support it. Shared fallback rendering must preserve user/assistant role boundaries.
5. `/model bad-name` leaves the prior dispatch plan and displayed model intact.
6. CLI with valid API key and unauthenticated Claude binary chooses the executable configured plan, not `claude --version`.
7. `CursorAcp` in chat/ACP resolves to ACP transport or fails capability validation; it is never sent to OpenAI-compatible HTTP.
8. Hard model override does not fallback; configured fallback emits `AttemptFailed` and records final model separately.
9. Serve provider test and template dispatch both pass through `DispatchPlan` and update provider health/latency through the shared path.
10. Recurrence checks above pass with no allowlist entries in ACP/chat/provider-dispatch surfaces.
