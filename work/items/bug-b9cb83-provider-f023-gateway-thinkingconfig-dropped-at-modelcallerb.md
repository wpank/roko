+++
id = "bug-b9cb83"
kind = "bug"
title = "Gateway ThinkingConfig dropped at ModelCallerBackend boundary"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "core"
subsystem = ["roko-gateway/provider"]
created = 2026-09-01
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F023"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F023"
anchors = ["crates/roko-gateway/src/provider.rs::ModelCallerBackend::request", "crates/roko-gateway/src/types.rs::ThinkingConfig", "crates/roko-core/src/foundation.rs::ModelCallRequest", "crates/roko-gateway/src/gateway.rs::GatewayConfig::from_model_caller", "crates/roko-agent/src/provider/mod.rs::AgentOptions", "crates/roko-agent/src/provider/anthropic_api/tool_loop.rs::create_tool_loop_backend_with_api_key"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/fn request(&self, request: &InferenceRequest)/,/^    }/p' crates/roko-gateway/src/provider.rs | grep -q thinking && grep -rqw 'fn model_caller_backend_forwards_thinking_config' crates/roko-gateway/ && cargo test -p roko-gateway model_caller_backend_forwards_thinking_config"
+++

## Problem

A caller can ask the inference gateway for extended thinking, but the request never reaches a provider.

`POST /api/gateway/inference` (route in `crates/roko-gateway/src/http.rs:61`, served by `roko serve`) deserializes
an `InferenceRequest`. That request has a `thinking` field, for example
`"thinking": {"type": "enabled", "budget_tokens": 8000}`. The gateway's ThinkingCap pipeline stage even fills in a
default budget when none is given. `ModelCallerBackend::request()` then converts the request into a
`roko_core::foundation::ModelCallRequest` and leaves `thinking` out, because `ModelCallRequest` has no field for it.

- Actual: the setting is dropped with no error or log. `"type": "disabled"` is dropped too. Whether the provider
  thinks depends only on the model profile (see Current state).
- Expected: the provider call runs with thinking at the requested budget, or with thinking off when disabled.
  If a provider cannot honour the setting, the gateway says so and does not drop it silently.

## Why it matters

- Goal `core` ("Plan runs work reliably"). The gateway is the serve-side inference path. When a request parameter
  is silently ignored, cost and answer quality differ from what the caller asked for, and nothing records it.
- Telemetry is wrong. `ThinkingCapStats.thinking_budgets_applied` (`/api/gateway/stats`) counts budgets that no
  provider ever received.
- Related: audit finding F024 ("Anthropic extended thinking not wired in any provider path", partly fixed since;
  see Current state). `find-af6b7f` (Gemini thinking tokens missing from usage). `gap-13bbbd` (sampling
  parameters are never set; it touches the same request types).

## Where

- `crates/roko-gateway/src/provider.rs::ModelCallerBackend::request` (line 123): maps `InferenceRequest` to
  `ModelCallRequest`. Both `ProviderBackend::complete` and `ProviderBackend::stream` use it. This is where
  `thinking` is dropped.
- `crates/roko-gateway/src/types.rs::ThinkingConfig` (line 142) and `ThinkingMode` (line 132): the gateway-side
  type. Its serde field `type` takes `enabled` or `disabled`, and `budget_tokens` is `Option<u32>`.
  `InferenceRequest.thinking` is at line 173.
- `crates/roko-gateway/src/thinking_cap.rs::apply_thinking_cap` (line 49): fills a family default only when thinking
  is enabled and no budget is given (opus 32768, haiku 4096, anything else 16384). It runs in the ThinkingCap stage
  of `crates/roko-gateway/src/gateway.rs` (around line 470).
- `crates/roko-gateway/src/gateway.rs::GatewayConfig::from_model_caller` (around line 121): wraps one live
  `ModelCaller` in three backends: `anthropic` (`claude-`), `openai` (`gpt-`, `o1`, `o3`, `o4`) and a catch-all.
- `crates/roko-core/src/foundation.rs::ModelCallRequest` (line 254) and `GenerationSettings` (line 329): neither has
  a thinking or reasoning field.
- The entry point is `crates/roko-serve/src/state.rs:1044`, which builds the gateway from the shared
  `ModelCallService` (`crates/roko-agent/src/model_call_service.rs`, `impl ModelCaller for ModelCallService` at
  line 2231).
- Provider side: `ModelCallService` turns the request into `AgentOptions` (`crates/roko-agent/src/provider/mod.rs:759`).
  `AgentOptions` has `effort: Option<String>` but no thinking field.
  `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs::create_tool_loop_backend_with_api_key` (around
  line 162) enables thinking with `default_thinking_budget(slug)` when `model.supports_thinking` is set.

## Current state

- Re-checked 2026-09-29 at HEAD. `ModelCallerBackend::request` copies `model`, `messages`, `max_tokens`,
  `temperature`, `role`, `caller`, `run_id`, `budget_remaining` and `tools`. It does not copy `thinking`. No commit
  has touched this since `085c34c25` (a clippy cleanup).
- F024 is partly fixed. `AnthropicMessagesBackend` now sends `"thinking": {"type": "enabled", "budget_tokens": N}`
  in both the plain and the streaming body (`tool_loop.rs:476` and `tool_loop.rs:535`). The budget comes only from
  the model profile (`supports_thinking`), so a per-request setting has no path to it.
- `ModelCallService` has its own `ThinkingCapCell` (`model_call_service.rs:1738`). It treats `req.max_tokens` as a
  thinking budget and overwrites the `max_tokens=` extra argument for slugs that match a substring heuristic
  (`supports_thinking` at line 1791: opus, o1, o3, o4, deepseek-r1). This is a separate oddity next to this bug.
  Do not rely on it, and do not make it worse.
- A test harness is ready. `gateway_from_model_caller_uses_existing_live_dispatch_boundary` (`gateway.rs:1172`)
  captures the `ModelCallRequest` that the live caller receives. `provider.rs` tests have a `LiveCaller` stub
  (line 318).

## Plan

1. Give roko-core a provider-neutral thinking type. `roko-core` cannot depend on `roko-gateway`, so the type must
   live in core. Two options:
   - Recommended: move `ThinkingConfig` and `ThinkingMode` from `roko-gateway/src/types.rs` into
     `roko-core/src/foundation.rs` unchanged, keeping the same serde attributes so the HTTP wire format does not
     change. Re-export them from `roko-gateway` (`types.rs`, and the `pub use` list in `lib.rs:52`). This leaves
     one type and needs no mapping.
   - Alternative: add a new core struct and convert to it in the gateway. This means two types to keep in sync.
2. Add `pub thinking: Option<ThinkingConfig>` to `ModelCallRequest` with
   `#[serde(default, skip_serializing_if = "Option::is_none")]`, next to `tools`. Serde stays backward compatible.
   Struct literals without `..Default::default()` will fail to compile, so fix them. There are about 30
   `ModelCallRequest {` literals in 13 files, and most already use `..Default::default()`.
3. In `ModelCallerBackend::request`, set `thinking: request.thinking.clone()`.
4. In `ModelCallService` (both `call` and `stream`), carry `req.thinking` into a new `AgentOptions.thinking` field.
   In `create_tool_loop_backend_with_api_key`:
   - `Some(Disabled)`: no thinking block, even if the profile supports thinking.
   - `Some(Enabled)`: call `with_thinking_budget(budget.unwrap_or(default_thinking_budget(slug)))`.
   - `None`: keep today's behaviour, where the profile decides.

   Respect the Anthropic API rules (check the current docs): `budget_tokens` must be at least 1024 and below
   `max_tokens`, and thinking does not combine with a custom temperature or a forced `tool_choice`. Clamp the budget
   or raise `max_tokens`, and log it with `tracing::debug!`.
5. For backends that cannot honour the setting (CLI backends, OpenAI-compatible), log once at debug level that it
   was ignored. Mapping it to `effort` for Claude CLI and Codex, or to `reasoning_effort` for o-series, is optional
   here: audit finding F081 covers `reasoning_effort` separately. Do not fail the request.
6. Add tests:
   - In `roko-gateway`, `model_caller_backend_forwards_thinking_config`: build an `InferenceRequest` with thinking
     enabled at budget 2048, run it through `ModelCallerBackend::complete` and `ModelCallerBackend::stream` with a
     capturing `ModelCaller`, and assert that `ModelCallRequest.thinking` equals the input. Add a second case that
     goes through `InferenceGateway::process_request` for an `opus` slug with no budget and assert that 32768
     arrives.
   - In `roko-agent`: a request-level `Disabled` removes the thinking block for a `supports_thinking` profile, and
     an explicit budget overrides the default. Use the mock-poster tests in `anthropic_api/tool_loop.rs`.

## Done when

- `ModelCallRequest` carries `thinking`, and `ModelCallerBackend::request` forwards it on both the complete path
  and the stream path.
- For an Anthropic API model, `thinking: {"type": "enabled", "budget_tokens": N}` produces `thinking.budget_tokens == N`
  in the outgoing HTTP body, and `"type": "disabled"` produces no thinking block.
- A provider that cannot honour the setting logs that it was ignored. Nothing drops it silently at the gateway
  boundary.
- `cargo test -p roko-gateway model_caller_backend_forwards_thinking_config` passes, and so does the item's
  `[[verify]]` command (static check that `thinking` appears in `ModelCallerBackend::request`).

## Notes

- `ModelCallRequest` and `AgentOptions` are used across the workspace. The change is additive, but it touches
  many files at compile time. Do not run it in parallel with other items that edit `foundation.rs::ModelCallRequest`
  or `AgentOptions`, such as `gap-13bbbd`.
- Keep the default budgets in `roko-gateway/src/thinking_cap.rs::default_budget` and
  `anthropic_api/tool_loop.rs::default_thinking_budget` equal. The doc comment on `default_thinking_budget` (around `tool_loop.rs:949`) says they must match.
- Do not change the HTTP wire shape of `ThinkingConfig`: the `type` field and lowercase `enabled`/`disabled`.
- This item has no dependencies. Live verification needs an Anthropic key, so the unit tests above are the gate.

## Original notes

`InferenceRequest.thinking` is populated by the gateway's `ThinkingCap` pipeline stage but is never forwarded to the underlying `ModelCallRequest` through `ModelCallerBackend::request()`. Thinking configuration requested via the gateway is silently dropped before reaching any provider.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F023`
- `tmp/archive/provider-audit/18-thinking-reasoning.md`

How to verify: Confirm in crates/roko-gateway/src/provider.rs whether still true: Gateway `ThinkingConfig` dropped at `ModelCallerBackend` boundary

Verified 2026-09-28: still true. ModelCallerBackend::request (crates/roko-gateway/src/provider.rs:123-136) copies model, messages, max_tokens, temperature and tools, but never InferenceRequest.thinking (types.rs:173). ModelCallRequest (crates/roko-core/src/foundation.rs) also has no thinking or reasoning field to carry it, and neither does GenerationSettings. A fix therefore has to add that field and map it for each provider. The gateway builds these backends in production (gateway.rs:127-129).
