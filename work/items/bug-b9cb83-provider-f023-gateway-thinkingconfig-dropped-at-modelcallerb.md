+++
id = "bug-b9cb83"
kind = "bug"
title = "Gateway ThinkingConfig dropped at ModelCallerBackend boundary"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
subsystem = ["roko-gateway/provider"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F023"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F023"
anchors = ["crates/roko-gateway/src/provider.rs::ModelCallerBackend::request", "crates/roko-gateway/src/types.rs:173", "crates/roko-core/src/foundation.rs::ModelCallRequest", "crates/roko-gateway/src/gateway.rs:127"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/fn request(&self, request: &InferenceRequest)/,/^    }/p' crates/roko-gateway/src/provider.rs | grep -q thinking"
+++
`InferenceRequest.thinking` is populated by the gateway's `ThinkingCap` pipeline stage but is never forwarded to the underlying `ModelCallRequest` through `ModelCallerBackend::request()`. Thinking configuration requested via the gateway is silently dropped before reaching any provider.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F023`
- `tmp/archive/provider-audit/18-thinking-reasoning.md`

How to verify: Confirm in crates/roko-gateway/src/provider.rs whether still true: Gateway `ThinkingConfig` dropped at `ModelCallerBackend` boundary

Verified 2026-09-28: still true. ModelCallerBackend::request (crates/roko-gateway/src/provider.rs:123-136) copies model, messages, max_tokens, temperature and tools, but never InferenceRequest.thinking (types.rs:173). ModelCallRequest (crates/roko-core/src/foundation.rs) also has no thinking or reasoning field to carry it, and neither does GenerationSettings. A fix therefore has to add that field and map it for each provider. The gateway builds these backends in production (gateway.rs:127-129).
