# 13 — ACP Provider Regression

Scope: `crates/roko-acp/src/bridge_events.rs`, `crates/roko-acp/src/session.rs`, `crates/roko-acp/src/types.rs`

This round checked the current ACP single-agent path against the runner architecture packs from `arch`, `converge`, `converge-followup`, `mega-parity`, and `post-parity`. The latest edits fix the visible symptom that "Claude CLI dispatch is disabled", but they do it by adding another provider dispatch path inside ACP instead of routing through the existing provider adapter layer.

## Findings

### CRITICAL: Claude CLI configuration now silently requires `ANTHROPIC_API_KEY`

`bridge_events.rs:1202-1215` maps both `ProviderKind::ClaudeCli` and `ProviderKind::AnthropicApi` to `run_anthropic_cognitive_task`. The project default in `roko.toml:12-15` still points to `default_backend = "anthropic"` and `default_model = "claude-sonnet"`, while `roko.toml:40-45` defines that backend as `kind = "claude_cli"`.

The result is a broken default path for users who have Claude CLI configured but not `ANTHROPIC_API_KEY`. The code comment says Claude CLI cannot run as a subprocess inside ACP, then falls through to Anthropic API. That is not provider resolution; it is a hidden provider substitution.

Expected design: ACP should dispatch through the same provider abstraction used elsewhere, or fail with an explicit "ACP does not support claude_cli" error that names the configured provider and the required fix. It should not reinterpret a CLI provider as an API provider.

### HIGH: ACP now owns a raw Anthropic streaming client

`bridge_events.rs:1403-1600` implements its own Anthropic Messages API streaming loop, including:

- environment lookup at `bridge_events.rs:1410`;
- request shape at `bridge_events.rs:1449-1457`;
- `reqwest::Client::new()` at `bridge_events.rs:1459`;
- SSE parsing at `bridge_events.rs:1512-1588`;
- usage extraction with zero defaults at `bridge_events.rs:1571-1584`.

This violates the runner rules from `mega-parity` and `post-parity`: no second provider resolution chain, no raw provider HTTP in surface code, and no new `reqwest::Client` per request. It also bypasses provider health, retry policy, shared client pooling, cost normalization, model capability handling, and adapter-specific compatibility fixes.

Expected design: ACP should build a typed model-call request and hand it to `ModelCallService` / provider adapters. If streaming is missing from that service, add streaming support at the adapter/service boundary, not inside `roko-acp`.

### HIGH: Error handling sends "complete" for failed dispatches

When `ANTHROPIC_API_KEY` is missing, the function sends a user-visible error chunk and then `CognitiveEvent::Complete { stop_reason: EndTurn, usage: None }` before returning an error (`bridge_events.rs:1411-1423`). The same pattern occurs on connection and HTTP errors (`bridge_events.rs:1471-1503`).

That makes the stream look like a completed model turn even though the dispatch failed. Downstream state can interpret the completion as a normal assistant turn or successful terminal event, depending on which branch observes first.

Expected design: dispatch failure should be a typed failure event, not a normal completion with error text hidden inside a token chunk.

### MEDIUM: Unknown usage is still collapsed to absent or zero

`bridge_events.rs:1509-1510` initializes token counts to zero, and `bridge_events.rs:1571-1584` uses `unwrap_or(0)` when usage fields are missing. It avoids emitting `Some(UsageInfo)` if both counts remain zero, but internally it still cannot distinguish an actual zero from an unknown/missing field.

This repeats the telemetry anti-pattern called out by `mega-parity` and `post-parity`: unknown is not zero. Usage should be represented as optional fields at the collection boundary, then normalized once in the shared telemetry layer.

### MEDIUM: ACP ContentBlock remains wire-incompatible with likely clients

`types.rs:356-365` serializes text blocks as `"type": "content"` and only accepts `"text"` as a deserialization alias. `bridge_events.rs:3349-3361` and `types.rs:950-965` now assert that outbound session updates use `"content"`.

The previous audit identified this as likely incompatible with ACP/Zed clients expecting `"type": "text"`. This second pass did not find a protocol-source justification for the rename. The tests currently lock in the local implementation, not the external contract.

Expected design: prove the wire contract from the ACP spec/client fixture before changing serialization. If compatibility is required, serialize the canonical external type and only use aliases for inbound tolerance.

## Root Cause

The patch optimized for "make ACP stream Claude output" without first deciding where provider ownership lives. That produced an ad-hoc Anthropic implementation in a protocol bridge. It solves one visible error message while creating a second dispatch stack, a second streaming parser, a second auth lookup, and a second usage-normalization path.

## Fix Direction

1. Remove `run_anthropic_cognitive_task` from `roko-acp` and route single-agent ACP through the shared provider/model-call layer.
2. Treat `ProviderKind::ClaudeCli` as either a real CLI adapter call or an explicit unsupported-provider error, not as Anthropic API.
3. Emit typed failure events instead of `Complete { EndTurn }` on failed dispatch.
4. Preserve unknown usage as `None` until the shared telemetry layer records it.
5. Re-verify `ContentBlock` serialization against ACP client fixtures before keeping `"content"`.
