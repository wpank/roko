# 14 — Chat Session Model Regression

Scope: `crates/roko-cli/src/auth_detect.rs`, `crates/roko-cli/src/chat_inline.rs`, `crates/roko-cli/src/chat_session.rs`, `crates/roko-core/src/agent.rs`

The post-parity runner said chat should have one session owner and one dispatch path. The current work moved in that direction, but the latest patches still leave mixed ownership and fallback behavior that can desynchronize what the UI says from what dispatch actually sends.

## Findings

### HIGH: `/model` failure path corrupts session state

In session mode, `/model` tries to resolve the new model at `chat_inline.rs:2846-2855`. On success, it updates `model`, `model_selection`, `provider_base_url`, and `provider_api_key_env` together (`chat_inline.rs:2856-2867`).

On failure, it falls back to only setting `agent_session.model = arg.to_string()` (`chat_inline.rs:2874-2888`). The actual dispatch code ignores that field for Claude CLI command construction and uses `model_selection.backend_slug` instead (`chat_session.rs:775`, `chat_session.rs:989`, `chat_session.rs:995`). This means the UI can report a switch that dispatch does not honor.

Expected design: a model switch should be atomic. If resolution fails, leave the previous model selection unchanged and show a failure. Do not update one redundant field and leave the canonical selection untouched.

### HIGH: `detect_auth` treats `claude --version` as login/auth proof

`auth_detect.rs:69-75` returns `AuthMethod::ClaudeCli` when `claude --version` exits successfully. The file comment says "installed and reachable", but the enum still documents `ClaudeCli` as "installed and logged in" (`auth_detect.rs:15-16`).

This reintroduces the exact problem the previous ordering tried to avoid: a machine with a Claude binary but expired/no auth will select the CLI path ahead of a valid API key. The failure moves from setup detection to the first dispatch.

Expected design: auth detection must distinguish "binary installed" from "can perform a real authenticated call", or it must prefer configured API keys when present. If probing a real call is too expensive, the status should be "Claude CLI installed, auth unknown" and should not outrank a valid API key.

### MEDIUM: API chat path hand-rolls provider HTTP instead of using adapters

`chat_session.rs:501-625` contains direct Anthropic and OpenAI-compatible HTTP request construction, response parsing, and usage extraction. This is inside the CLI chat session rather than the provider adapter or `ModelCallService`.

This violates the same architecture target used by the runners: chat dispatch should delegate to existing adapters, not manually build provider JSON in the CLI layer. The immediate symptom is feature drift: API chat does not share the adapter layer's tool conversion, MCP handling, provider quirks, retries, telemetry, and streaming behavior.

Expected design: `ChatAgentSession` should own chat state, but dispatch should call a shared adapter/service with a typed request. It should not own provider protocol details.

2026-05-01 update: the raw HTTP construction in `ChatAgentSession::send_turn_api`
has been removed. The chat API path now builds a `ModelCallRequest`, synthesizes
only missing provider/model config for the selected model, and consumes
`ModelCallService::stream`; `ChatAgentSession` no longer owns a `reqwest::Client`.
Verified with:

```bash
cargo check -p roko-cli --lib
rg 'POST /v1/messages|POST /chat/completions|send_turn_api: Anthropic|send_turn_api: OpenAI|x-api-key|bearer_auth|anthropic-version' crates/roko-cli/src/chat_session.rs
```

Follow-up update: shared `ModelCallService` prompt rendering now preserves
`User:`/`Assistant:` role boundaries for multi-turn history and keeps single-user
turns unchanged. Remaining design debt: some adapters still receive rendered
prompt text rather than provider-native structured chat messages. That must be
fixed in `roko-agent`/provider ownership, not by restoring surface-local provider
HTTP.

### MEDIUM: Anthropic API messages include `system` as a message

`chat_session.rs:512-522` pushes a `"role": "system"` message into the `messages` array when history is empty. Then `chat_session.rs:555-559` sends that array directly to Anthropic `/v1/messages`.

Anthropic Messages API expects system content in the top-level `system` field, not as a message role. The ACP patch implements that extraction separately in `bridge_events.rs:1426-1457`, which proves the codebase now has two different Anthropic request shapes.

Expected design: one Anthropic adapter should own this translation. Chat should not need to know where system text goes.

2026-05-01 update: chat no longer builds Anthropic request JSON, so this exact
surface bug is removed. The remaining acceptance criterion is a provider-layer
test proving chat/ACP/serve all route system text through the same Anthropic
adapter translation.

### MEDIUM: Fallback model is hardcoded outside model-selection policy

`chat_session.rs:995-997` adds `--fallback-model claude-haiku-4-5` for every Claude CLI model except haiku. This fallback is independent of `roko.toml`, `EffectiveModelSelection`, budget policy, or user override semantics.

For an explicit `/model opus` or CLI override, a silent fallback to haiku can violate the "CLI --model is a hard override" runner rule. If fallback is allowed, it should be an explicit model-selection policy decision with user-visible source/reason.

## Root Cause

The implementation kept `ChatAgentSession` as the state owner but did not remove the duplicate provider/protocol ownership. Fields such as `model` and `model_selection` still compete, and error paths update only one of them. The design needs a canonical selected model object and a provider adapter call boundary.

## Fix Direction

1. Make `model_selection` the only dispatch source and remove or strictly derive `model`.
2. Make `/model` all-or-nothing: failed resolution leaves the previous selection intact.
3. Replace API request construction in `chat_session.rs` with adapter/model-call service calls. **Done for chat API dispatch on 2026-05-01.**
4. Move Anthropic system-message translation into a single Anthropic adapter path. **Surface-local chat construction removed; add provider-layer parity tests for every consumer.**
5. Replace hardcoded Claude fallback with explicit, typed model-selection policy.
