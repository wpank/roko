# Agent Configuration

> The `[agent]` table controls the LLM model selection, turn limits, timeouts, and
> connection settings for every agent that Roko spawns.

**Status**: Shipping
**Crate**: `roko-agent`, `roko-orchestrator`
**Depends on**: [01-roko-toml-schema.md](01-roko-toml-schema.md)
**Used by**: [operations/performance/01-latency-budgets.md](../performance/01-latency-budgets.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The `[agent]` table has three critical keys for most deployments: `model` (which LLM),
`max_turns` (when to give up), and `timeout_seconds` (wall-clock limit). Everything else
has sensible defaults.

```toml
[agent]
model           = "claude-sonnet-4-5"
max_turns       = 25
timeout_seconds = 600
```

---

## Model Selection

The `model` key accepts any model slug supported by the chosen `backend`. Roko ships
five LLM backends:

| Backend | Env var required | Common model slugs |
|---------|----------------|--------------------|
| `anthropic` | `ANTHROPIC_API_KEY` | `claude-opus-4-6`, `claude-sonnet-4-5`, `claude-haiku-4-5` |
| `openai` | `OPENAI_API_KEY` | `gpt-4o`, `gpt-4o-mini`, `o3`, `o4-mini` |
| `openrouter` | `OPENROUTER_API_KEY` | Any of 400+ models via a single key |
| `ollama` | none | `llama3.2`, `mistral`, any locally-pulled model |
| `bedrock` | AWS credentials | `anthropic.claude-opus-4-6-v1` etc. |

Set `backend` to match your API key. The `model` slug must match what the backend
accepts — Roko does not translate slugs across backends.

**Example: switching to OpenAI:**

```toml
[agent]
backend = "openai"
model   = "gpt-4o"
```

**Example: using Ollama for a local, air-gapped deployment:**

```toml
[agent]
backend  = "ollama"
base_url = "http://localhost:11434/v1/"
model    = "llama3.2:70b"
```

---

## Turn Limits and Timeouts

### `max_turns`

Each LLM interaction (prompt → response) counts as one turn. Tool calls may or may
not count as turns depending on the backend (Anthropic does not count tool calls;
OpenAI counts each tool-call round-trip). The `max_turns` limit exists to prevent
runaway agents that loop indefinitely.

When `max_turns` is reached, the agent receives a final "maximum turns reached" message
and must produce a concluding response. The task is then evaluated by the gate pipeline
against whatever the agent produced. If the gate passes, the task is considered complete;
if it fails, the task is marked failed with the reason "max_turns".

Recommended values:

| Workload | Recommended `max_turns` |
|----------|------------------------|
| Simple single-file edits | 10–15 |
| Multi-file refactors | 20–30 |
| Complex architectural changes | 35–50 |
| Research-heavy tasks | 25–40 |

### `timeout_seconds`

The wall-clock timeout for the entire agent task (from the first LLM call to the gate
completing). When the timeout fires:

1. The agent subprocess is sent SIGTERM.
2. After a 10-second grace period, SIGKILL is sent if still running.
3. A partial state snapshot is written to `.roko/state/`.
4. The task is marked as `TimedOut` in the executor.

The task can be resumed after fixing the root cause (usually a hanging tool call or
an external service timeout). See [operations/error-handling/04-crash-recovery.md](../error-handling/04-crash-recovery.md).

---

## MCP Configuration

`mcp_config` points to the `.mcp.json` file that controls which MCP tool servers are
available to agents. Setting it to an empty string (`""`) disables MCP tool discovery
entirely, which is useful for sandboxed or restricted environments.

See [07-mcp-config.md](07-mcp-config.md) for the `.mcp.json` file format.

---

## Gateway / Proxy

To route all agent inference through a local gateway (for caching, cost tracking, or
provider failover), set `base_url` to the gateway's base URL:

```toml
[agent]
backend  = "anthropic"
model    = "claude-sonnet-4-5"
base_url = "http://localhost:4000/"
```

The gateway must present an Anthropic-compatible API (or OpenAI-compatible if `backend
= "openai"`). The gateway is responsible for forwarding to the real provider; Roko does
not know or care about what is behind `base_url`.

---

## Extended Thinking

Extended thinking allows Claude 3.7+ to reason through problems before producing a
response. It is expensive (10K+ thinking tokens per call) and increases latency
significantly. Use it for tasks that require multi-step planning or architectural
decisions.

```toml
[agent]
model                  = "claude-sonnet-4-7"
thinking               = true
thinking_budget_tokens = 15000
```

Thinking tokens are billed at the same input rate as prompt tokens but are discarded
after the response is generated (they do not appear in the response to the user).

---

## Custom System Prompts

The default system prompt teaches agents Roko's conventions (how to use tools, coding
standards, MCP patterns, etc.). Override it with a project-specific file:

```toml
[agent]
system_prompt_path = "AGENTS.md"
```

Convention: name the file `AGENTS.md` in the project root. This file should explain:

- The project's directory layout.
- Which tools to prefer (e.g. always use `search_symbols` before `read_file`).
- Coding standards (error handling patterns, naming, dependency rules).
- Known gotchas for this codebase.

If `system_prompt_path` is set, the built-in system prompt is completely replaced.
The custom file is read once at startup; changes require a restart.

---

## Two Full Examples

**Laptop developer profile:**

```toml
[agent]
model              = "claude-sonnet-4-5"
mcp_config         = ".mcp.json"
max_turns          = 20
timeout_seconds    = 300
backend            = "anthropic"
system_prompt_path = "AGENTS.md"
```

**Server/team profile with gateway:**

```toml
[agent]
model              = "claude-opus-4-6"
mcp_config         = ".mcp.json"
max_turns          = 40
timeout_seconds    = 900
backend            = "anthropic"
base_url           = "http://roko-gateway.internal:4000/"
thinking           = false
```

---

## See Also

- [07-mcp-config.md](07-mcp-config.md) — `.mcp.json` format
- [08-environment-variables.md](08-environment-variables.md) — `ANTHROPIC_API_KEY` and peers
- [12-examples.md](12-examples.md) — complete per-persona profiles
- [operations/performance/01-latency-budgets.md](../performance/01-latency-budgets.md) — how `timeout_seconds` fits into the latency budget

## Open Questions

- `agent.concurrency` key (maximum concurrent agents for the orchestrator) is not yet in the schema — controlled only by the CLI `--concurrency` flag today.
- Whether `thinking_budget_tokens` should be per-task-category (so high-complexity tasks get more budget) is under discussion.
