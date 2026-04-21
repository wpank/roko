# 18 - Model Dispatch Redesign

Scope: `crates/roko-cli/src/model_selection.rs`, `crates/roko-cli/src/unified.rs`, `crates/roko-cli/src/dispatch_direct.rs`, `crates/roko-agent/src/model_call_service.rs`, `crates/roko-agent/src/provider/mod.rs`, `crates/roko-core/src/foundation.rs`

This pass looks past the immediate ACP/chat regressions and asks why provider bugs keep reappearing. The answer is that Roko still does not have one authoritative model dispatch contract. Recent work added more "resolved selection" structs and fallbacks, but surface code can still bypass the shared provider layer, synthesize providers at call time, or reinterpret provider kinds.

## Findings

### CRITICAL: model selection is not the same thing as dispatch resolution

`model_selection.rs:55-70` defines `EffectiveModelSelection` with `effective_model_key`, `provider_key`, `provider_kind`, and `backend_slug`, but it is only a reporting/selection value. It does not prove that the selected provider is authenticated, supported by the caller surface, capable of streaming/tools/MCP, or the transport that will actually be used.

That gap shows up immediately in `unified.rs:250-285`: one-shot chat resolves a model, then rejects anything except `ProviderKind::ClaudeCli`. If that fails, `unified.rs:111-129` falls back first to `dispatch_v2::dispatch_via_model_call_service` and then to deprecated raw dispatch. The selection object says "model/provider chosen"; the command still has to decide whether it can really execute it.

Expected design: dispatch resolution should return a single typed `DispatchPlan`, not just display metadata. That plan should contain requested model, actual model, provider id, provider kind, concrete transport, auth status, caller capability requirements, fallback policy, and the reason the plan is valid for the current surface.

### HIGH: deterministic fallbacks hide ambiguous configuration

`model_selection.rs:304-317` handles `--provider` by picking the alphabetically first model attached to that provider. `model_selection.rs:362-381` picks the exact provider key if present, otherwise the alphabetically first provider of a matching kind. `provider/mod.rs:449-464` has a similar "first provider of kind" helper.

These rules are deterministic, but they are not semantic. When multiple providers of the same kind exist, or a provider supports multiple models, the user gets a stable but arbitrary selection. That is a duct-tape fix for "we need any provider" rather than a design for "we know which backend this task should use."

Expected design: provider override should either name a default model for that provider or require the caller to provide a model. Provider-kind fallback should be an explicit config rule with priority, not a sorted map side effect.

### HIGH: the cascade router is invoked without task context

`model_selection.rs:250-263` calls `router.select(Vec::new()).model.slug` and the comment says the selector has no richer feature context. That means this path asks the learning router for a deterministic raw-context choice, then treats the result as a meaningful routing decision.

This is a design smell from the runner batches: routing was "wired" by calling the router, but the context that makes it a router was never modeled at the CLI boundary.

Expected design: the model resolver should accept a structured `RoutingContext` assembled once from caller, task, role, repo, budget, gate stage, history, and user override. If a surface cannot build that context, it should record `routing_context_status = unavailable` and use declared defaults instead of presenting a learned decision.

### HIGH: deprecated raw dispatch is still on a live fallback path

`unified.rs:111-129` says ChatAgentSession has the full prompt/tools/MCP/safety path, but falls back to raw dispatch when initialization fails. `dispatch_direct.rs` is marked deprecated, yet it remains reachable. That raw path spawns `claude --print --output-format stream-json` without the same system prompt/tool contract, and its API branches create provider-specific HTTP requests outside the adapter layer.

This means a configuration or init bug can silently downgrade the command to a less safe and less capable execution mode. The visible symptom is "the prompt returned text"; the hidden regression is "the request skipped the actual orchestration contract."

Expected design: raw dispatch should be deleted from production paths or kept only behind a test/dev diagnostic flag. A failed session build should return a typed initialization error that names the missing capability, not retry through a weaker transport.

### HIGH: ModelCallService still synthesizes providers during execution

`model_call_service.rs:107-131` constructs its own runtime with default config, cache, budget, fallback models, env, OpenAI base URL, and routers. `model_call_service.rs:321-380` then mutates a cloned config to insert OpenAI-compatible and Anthropic providers based on request-time fields and environment presence.

`provider/mod.rs:131-210` also synthesizes provider configs from `agent.command` and known CLI names. These heuristics are useful for bootstrapping, but they should not live inside the execution hot path. They make it hard to tell whether a provider came from project config, migration defaults, env inference, or command-name inference.

Expected design: config migration/defaulting should produce a validated provider registry before execution starts. `ModelCallService` should receive an immutable registry plus a `DispatchPlan`; it should not invent providers while handling a request.

### MEDIUM: the provider interface does not support the streaming surfaces that need it

`foundation.rs:159-164` defines `ModelCaller` with only `call()`. ACP and chat both need streaming, but because the shared boundary lacks a streaming method, ACP added a raw Anthropic SSE parser in `bridge_events.rs` and chat retains raw dispatch fallbacks. The missing abstraction is pushing provider protocol code up into surface crates.

Expected design: add a provider-owned streaming API such as `stream(req) -> Stream<ModelDelta/Event>`. ACP, chat, serve, and CLI one-shot should all consume the same event stream and map it to their wire protocols.

## Redesign Direction

1. Introduce `DispatchPlan` as the only object that can authorize execution. It should be produced by one resolver and consumed by `ModelCallService`.
2. Move provider synthesis to config loading/migration. Execution should only use validated providers and models.
3. Replace sorted-map provider/model fallback with explicit config priority and clear ambiguity errors.
4. Require structured `RoutingContext` for learned routing; otherwise mark routing unavailable and use declared defaults.
5. Add streaming to the shared model-call/provider API, then remove ACP/chat raw provider clients.
6. Delete or quarantine `dispatch_direct` so init failures cannot downgrade safety, tools, MCP, or prompt contracts.
