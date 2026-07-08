# A — Core Abstractions

Refresh target: `docs/02-agents/00-agent-trait.md`, `03-chat-types.md`, `04-agent-roles.md`

Verdict: `rewrite`

---

## Current Parity Summary

| Topic | Current state | Notes |
|---|---|---|
| `Agent` + `AgentResult` | Shipping | Stable contract in `crates/roko-agent/src/agent.rs` |
| Roles | Shipping | `AgentRole` has 28 variants with backend/tier defaults |
| Backend families | Shipping | Live surface spans Claude CLI/API, Codex, Cursor, OpenAI, Ollama, Gemini, and Perplexity |
| MCP handoff | Shipping | `AgentOptions.mcp_config` is already threaded into provider-backed creation paths |
| Shared response ownership | Partial | response types still live on the agent side and remain a layering seam |
| Event taxonomy | Partial | duplicate `AgentEvent` enums remain in `roko-agent` and `roko-learn` |

---

## What Is Definitely Live

### Agent contract

- `AgentResult` is defined at `crates/roko-agent/src/agent.rs:9`.
- `Agent` is defined at `crates/roko-agent/src/agent.rs:120`.
- The runtime surface is bigger than the old parity notes implied: `rg -n "impl Agent for" crates/roko-agent/src` currently finds 19 implementations.

### Runtime families

The parity copy should describe the agent surface by runtime family, not by the original seven implementations list:

- Claude CLI: `ClaudeCliAgent`
- Anthropic API: `ClaudeAgent`
- Codex / OpenAI-style HTTP: `CodexAgent`, `OpenAiAgent`
- Cursor ACP: `CursorAgent`
- Ollama: `OllamaAgent`
- Gemini: `GeminiCompatAgent`, `GeminiNativeAgent`, `GeminiEmbedAgent`
- Perplexity: chat, deep-research, embed, and tool-loop agents

This is enough to support the audit conclusion that agent dispatch is already wired, not hypothetical.

### Role system

- `AgentRole` defaults are live in `crates/roko-core/src/agent.rs`.
- The docs should keep the role taxonomy, but stop implying that the remaining work is to invent a new role framework.
- The real near-term question is parity and ownership, not role invention.

### MCP already passes through the agent surface

- `SpawnAgentSpec.mcp_config` is present in `crates/roko-cli/src/agent_spawn.rs:25`.
- CLI config exposes `agent.mcp_config` in `crates/roko-cli/src/config.rs:194`.
- Provider-backed creation paths already accept `AgentOptions.mcp_config`, and Claude CLI plus OpenAI-compatible paths consume it.

This means parity copy should say MCP is passed through today, not planned.

---

## Narrow Remaining Gaps

### 1. Shared response ownership is still uneven

The docs can keep calling out the response-type seam, but the claim should be narrow:

- there are still duplicated/agent-owned response surfaces
- this is a layering cleanup issue
- it is not evidence that the agent stack is missing its core abstractions

### 2. `AgentEvent` duplication is real

This is the cross-crate abstraction issue worth keeping visible in the parity pack:

- `crates/roko-agent/src/task_runner.rs:73`
- `crates/roko-learn/src/events.rs:15`

The docs should point to unification as a concrete follow-on, not inflate it into a large event-framework redesign.

---

## What To Stop Claiming

- Do not describe the agent layer as missing its backend surface.
- Do not imply MCP still needs a new core abstraction to flow through providers.
- Do not frame batch `02` as the place to invent new agent categories, new role systems, or speculative memory-sharing primitives.

---

## Recommended Refresh Language

- Keep: `Agent`, `AgentResult`, role taxonomy, and the separation from the six core traits.
- Rewrite: the implementation examples and counts so they match the current runtime surface.
- Keep but narrow: the response-type ownership seam.
- Keep: duplicate `AgentEvent` enums as a real integration issue.

---

## Verification Anchors

```bash
rg -n "pub struct AgentResult|pub trait Agent" crates/roko-agent/src/agent.rs
rg -n "impl Agent for" crates/roko-agent/src
rg -n "pub enum AgentRole" crates/roko-core/src/agent.rs
rg -n "mcp_config" crates/roko-cli/src/config.rs crates/roko-cli/src/agent_spawn.rs crates/roko-agent/src/provider
rg -n "pub enum AgentEvent" crates/roko-agent/src/task_runner.rs crates/roko-learn/src/events.rs
```
