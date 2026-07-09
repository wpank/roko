# ⚠️ SUPERSEDED — See [MASTER-PLAN.md](../MASTER-PLAN.md) Tier 1C + Tier 2D
>
> Content absorbed into MASTER-PLAN.md. This file retained for historical reference.

---

# 07 — MCP & Tool Registry Wiring

> **Priority**: 🟡 P1 — Code intelligence requires MCP; tool restrictions require registry
> **Parity sections**: §22 (MCP server), §36 (Tool registry)
> **Checklist ref**: `MORI-PARITY-CHECKLIST.md` §22, §36

## Problem statement

Two separate issues:

### 1. MCP (Model Context Protocol) — exists but not passed to agents

Roko has an MCP client (`/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/mcp/`) with JSON-RPC,
tool converter, dedup, and config walk-up. But:
- `ClaudeCliAgent` doesn't pass `--mcp-config`
- The MCP server binary isn't spawned as part of the agent lifecycle
- Mori conditionally skips MCP for AutoFixer/Conductor roles

### 2. Tool registry — exists but not connected to agent spawn

Roko has a `ToolRegistry` with per-role allowlists (`/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/tool/`)
including `role_allowlist`, `ToolRelevanceScorer`, `ProfileBandit`, `FormatNegotiator`.
None of this feeds into the `--tools` flag.

Mori's per-role tool matrix (connection.rs:2483-2535):
```
Implementer: Read,Glob,Grep,Edit,Write,Bash(+MCP)
Reviewer:    Read,Glob,Grep,Bash,WebFetch(+MCP)
Architect:   Read,Glob,Grep,Edit,Write,Bash,WebFetch,WebSearch(+MCP+permissions)
AutoFixer:   Read,Glob,Grep,Edit,Write,Bash (NO MCP)
Conductor:   Read,Glob,Grep,Bash (NO MCP)
```

## Checklist

### Phase A: Tool registry → --tools flag

- [ ] **7.1** `fn tools_for_role(role: AgentRole) -> Vec<String>` using existing `role_allowlist`
- [ ] **7.2** ClaudeCliAgent passes `--tools <comma-separated-list>`
- [ ] **7.3** Test: each role gets exactly the tools mori gives it

### Phase B: MCP config injection

- [ ] **7.4** ClaudeCliAgent does config walk-up for `.mcp.json` (matching mori's logic)
- [ ] **7.5** Skip MCP for AutoFixer and Conductor roles
- [ ] **7.6** Pass `--mcp-config <path>` when found
- [ ] **7.7** Pass `--strict-mcp-config` when MCP config is active
- [ ] **7.8** MCP server lifecycle: spawn on agent start, kill on agent end

### Phase C: Tool-loop integration (non-Claude backends)

- [ ] **7.9** ToolLoop uses `ToolRegistry::for_call(role, task_ctx, limit)` to select tools
- [ ] **7.10** Tool format negotiation via `FormatNegotiator` is active
- [ ] **7.11** Progressive tool discovery (bandit-based subset selection) is active
- [ ] **7.12** Tool result compression for large outputs

> Maps to checklist: §22.1-22.10, §36.k, §36.l, §36.57-36.61
