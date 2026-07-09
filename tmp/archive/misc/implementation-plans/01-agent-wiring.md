# 01 — Agent Wiring: The Critical Gap

> **Priority**: 🔴 P0 — Nothing works correctly without this
> **Parity sections**: §7.1 (Claude backend), §7.2-7.5 (other backends), §8 (process mgmt)
> **Checklist ref**: `MORI-PARITY-CHECKLIST.md` §7, §8, I.2

## Problem statement

Roko has two agent paths, both incomplete:

1. **`ClaudeAgent`** (`/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_agent.rs`)
   — Calls Anthropic HTTPS API directly. Has NO system prompt field. `MessagesRequest` is just
   `{model, max_tokens, messages}`. No tools, no streaming, no session resume.

2. **`ExecAgent`** (`/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/exec.rs`)
   — Generic stdin→stdout subprocess. Used by both `run.rs` AND `orchestrate.rs` to spawn `claude` CLI.
   Passes NO Claude-specific flags (`--tools`, `--settings`, `--bare`, `--append-system-prompt`,
   `--fallback-model`, `--effort`, `--mcp-config`, `--resume`).

3. **`orchestrate.rs`** (`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`)
   — NEW: runtime harness that dispatches ExecutorActions. Has inline `role_system_prompt()`
   (1-sentence per role) but passes them as user-message sections, not `--append-system-prompt`.
   Still uses ExecAgent.

Meanwhile, **Mori** (`/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:2444-2620`)
passes 15+ flags with role-specific configuration.

## What mori does (line-by-line from connection.rs)

```
Line 2453: model_slug = model.unwrap_or("claude-opus-4-6")
Line 2455: system_prompt = claude_system_prompt(self.role)     ← role-specific ~2K token prompt
Line 2457: if self.bare_mode { cmd.arg("--bare") }
Line 2460: --print --verbose --output-format stream-json
Line 2464: --model <slug>
Line 2466: --effort <level>
Line 2468: --append-system-prompt <system_prompt>
Line 2472: --fallback-model claude-haiku-4-5
Line 2480: --settings <safety_hooks_json>
Line 2497: --tools <role-specific allowlist>                    ← per-role least-privilege
Line 2534: --dangerously-skip-permissions                       ← for Architect role
Line 2547: --mcp-config <path>                                  ← conditional per role
Line 2570: --strict-mcp-config
Line 2574: --resume <session_id>                                ← multi-turn
Line 2578: current_dir(working_dir)
Line 2583: env CARGO_INCREMENTAL=0
Line 2586: env CARGO_BUILD_JOBS=2
```

## Checklist

### Phase A: ClaudeAgent HTTPS path — add system prompt

- [ ] **A.1** Add `system: Option<String>` field to `MessagesRequest` in `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_agent.rs`
- [ ] **A.2** Add `.with_system_prompt(prompt: impl Into<String>)` builder method on `ClaudeAgent`
- [ ] **A.3** Wire `SystemPromptBuilder` output into `ClaudeAgent::run()` — compose from role + conventions + domain + task + tool instructions + anti-patterns
- [ ] **A.4** Add `tools` field to `MessagesRequest` for native Anthropic tool_use
- [ ] **A.5** Add `tool_choice` field support
- [ ] **A.6** Parse `tool_use` content blocks in response (currently only handles `text` blocks)
- [ ] **A.7** Add streaming support (SSE parsing for `/v1/messages` with `stream: true`)
- [ ] **A.8** Add `max_tokens` configurability (currently hardcoded at `self.max_tokens`)

> Maps to checklist: §7.1.1, §7.1.5, §7.1.7

### Phase B: Claude CLI spawn path — new `ClaudeCliAgent`

Roko needs a **dedicated Claude CLI agent** (not ExecAgent) that mirrors mori's spawn logic.

- [ ] **B.1** Create `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_cli_agent.rs`
- [ ] **B.2** `--print --verbose --output-format stream-json` (suppress TUI, get NDJSON)
- [ ] **B.3** `--bare` flag (configurable via `config.agent.bare_mode`)
- [ ] **B.4** `--model <slug>` from config or per-invocation override
- [ ] **B.5** `--effort <level>` from `config.agent.default_effort`
- [ ] **B.6** `--append-system-prompt <prompt>` — wire in `SystemPromptBuilder` output
- [ ] **B.7** `--fallback-model <slug>` from `config.agent.fallback_model`
- [ ] **B.8** `--settings <json>` — safety hooks (see plan 03)
- [ ] **B.9** `--tools <role-allowlist>` — per-role tool restrictions (see plan 07)
- [ ] **B.10** `--dangerously-skip-permissions` for specific roles (Architect)
- [ ] **B.11** `--mcp-config <path>` with config walk-up (skip for AutoFixer/Conductor)
- [ ] **B.12** `--strict-mcp-config` when MCP config found
- [ ] **B.13** `--resume <session_id>` for multi-turn conversations
- [ ] **B.14** `current_dir(working_dir)` — per-agent worktree
- [ ] **B.15** Environment: `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=2`, sccache wrapper
- [ ] **B.16** Stream-json NDJSON parser for stdout events
- [ ] **B.17** Stderr monitoring with known-warning classification (mori-ref: `connection.rs:2630-2655`)
- [ ] **B.18** PID registration for orphan reaping

> Maps to checklist: §7.1.2, §7.1.3, §7.1.4, §7.1.6, §7.1.8, §7.1.9, §7.1.10, §8.1-8.6

### Phase C: Wire into roko-cli

- [ ] **C.1** `orchestrate.rs` detects backend type from config: `"claude"` → ClaudeCliAgent, `"claude-api"` → ClaudeAgent, `"ollama"` → ExecAgent etc.
- [ ] **C.2** Pass `config.agent.bare_mode`, `config.agent.default_effort`, `config.agent.fallback_model` through
- [ ] **C.3** Replace inline `role_system_prompt()` in orchestrate.rs with SystemPromptBuilder from roko-compose
- [ ] **C.4** Integration test: spawn claude CLI via ClaudeCliAgent, verify flags match mori's

### Phase D: Other backends (parity with mori)

- [ ] **D.1** Codex backend (`/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/codex_agent.rs`) — verify flags match mori's AppServerConnection
- [ ] **D.2** Cursor backend — verify agent protocol match
- [ ] **D.3** Ollama backend — verify tool_loop integration
- [ ] **D.4** OpenAI backend — verify streaming + function calling
