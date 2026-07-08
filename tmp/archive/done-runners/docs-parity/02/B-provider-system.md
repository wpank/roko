# B — Provider System

Refresh target: `docs/02-agents/01-provider-registry.md`, `02-provider-adapters.md`, `14-provider-integrations.md`

Verdict: `rewrite`

---

## Current Parity Summary

| Topic | Current state | Notes |
|---|---|---|
| Provider registry | Shipping | `ProviderConfig`, `ModelProfile`, and `resolve_model()` are live |
| Provider kinds | Shipping | 6 `ProviderKind` variants are in code today |
| Adapters | Shipping | 6 adapters are registered in `adapter_for_kind()` |
| Runtime families | Shipping | Claude CLI/API, Codex, Cursor, OpenAI, Ollama, Gemini, and Perplexity are all represented |
| Integrations doc scope | Needs narrowing | some older provider narratives lag the current adapter map |

---

## Provider Story To Use In Refreshed Docs

Describe the live provider surface in two layers.

### 1. Registered provider kinds

`ProviderKind` currently includes:

- `AnthropicApi`
- `ClaudeCli`
- `OpenAiCompat`
- `CursorAcp`
- `PerplexityApi`
- `GeminiApi`

That is the actual adapter dispatch surface today.

### 2. Runtime families users actually see

The human-facing provider/backend list is slightly broader:

- Claude CLI
- Anthropic API
- Codex
- Cursor
- OpenAI-compatible HTTP
- Ollama
- Gemini
- Perplexity

Important nuance:

- Codex and many OpenAI/OpenRouter/GLM/Kimi-style integrations ride through `OpenAiCompat`.
- Ollama still exists as its own agent/backend family, even though some config paths resolve through OpenAI-compatible conventions.
- Gemini and Perplexity now have dedicated provider kinds and adapters; they should no longer be documented as “just OpenAI-compatible aliases.”

---

## What The Old Parity Copy Overstated Or Missed

### Old understatement

The earlier parity pack made the provider system sound like a partially landed migration. That is no longer accurate.

What is already true:

- `adapter_for_kind()` is exhaustive for the 6 current provider kinds.
- `create_agent_for_model()` is the main config-driven construction seam.
- provider-specific integrations for Gemini and Perplexity are live, not target-state notes.

### Old simplification that now misleads

The old copy treated Perplexity and Gemini as plain `OpenAiCompat` consumers. That is stale.

Use the refreshed language instead:

- OpenAI-compatible remains the broad compatibility bucket.
- Gemini and Perplexity have dedicated kinds and adapters.
- Ollama belongs in the runtime-family story even when the config path is compatibility-shaped.

---

## Narrow Remaining Gaps

These are the only provider-system gaps worth carrying in this parity pack:

1. Some docs and source anchors still use stale counts or stale variant lists.
2. Tool-loop coverage is not uniform across every backend family, so provider docs must distinguish shared-path coverage from provider-specific paths.
3. The parity copy should stop mixing runtime families, provider kinds, and agent classes as if they were the same layer.

---

## Explicit Non-Goals

- Do not introduce new provider SPI layers here.
- Do not propose new optimization matrices unless they already exist in code.
- Do not expand this batch into a roadmap for additional provider families.

---

## Verification Anchors

```bash
rg -n "pub enum ProviderKind" crates/roko-core/src/agent.rs
rg -n "pub fn adapter_for_kind|pub fn create_agent_for_model|pub trait ProviderAdapter" crates/roko-agent/src/provider/mod.rs
rg -n "struct .*Adapter|impl ProviderAdapter" crates/roko-agent/src/provider crates/roko-agent/src/gemini crates/roko-agent/src/perplexity
rg -n "AgentBackend" crates/roko-core/src/agent.rs
```
