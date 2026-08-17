# MCP Passthrough Gaps: ACP Chat Drops Session MCP, AgentConfig Missing Field

## Problem

MCP config passthrough works for `roko plan run` (Claude CLI backend) but has gaps:

1. ACP chat with Claude CLI backend drops session-level MCP servers
2. `roko-core::AgentConfig` missing `mcp_config` field — can't pass MCP config through
   the agent dispatch layer cleanly
3. MCP servers discovered by auto-discovery aren't forwarded to ACP agents

## Root Cause

### A. ACP chat drops MCP

**File:** `crates/roko-acp/src/bridge_events.rs`

When ACP dispatches a chat message via Claude CLI, it builds the command:
```rust
let mut args = vec!["--print", "--model", &model];
// MCP config is NOT added here
// In contrast, orchestrate.rs adds:
// args.push("--mcp-config");
// args.push(&mcp_config_path);
```

The session may have MCP servers configured (either from `roko.toml` or auto-discovered),
but they're not passed to the Claude CLI subprocess for chat interactions.

### B. AgentConfig missing mcp_config

**File:** `crates/roko-core/src/config/agent.rs`

```rust
pub struct AgentConfig {
    pub model: String,
    pub role: Option<String>,
    pub system_prompt: Option<String>,
    pub allowed_tools: Option<String>,
    pub max_turns: Option<u32>,
    // NO mcp_config field
}
```

The `roko.toml` top-level `[agent]` section has `mcp_config`, but the per-agent `AgentConfig`
struct doesn't. This means you can set MCP globally but not per-agent.

### C. Auto-discovered MCP not forwarded

**File:** `crates/roko-cli/src/mcp_discovery.rs`

MCP auto-discovery finds `.mcp.json` files in the workspace and parent directories. But
the discovered servers are only used by the CLI path (orchestrate.rs). The ACP bridge
doesn't run auto-discovery.

## Fix

### Fix 1: Add MCP to ACP Claude CLI dispatch (~10 min)

**File:** `crates/roko-acp/src/bridge_events.rs`

```rust
// When building Claude CLI args for chat:
if let Some(mcp_config) = &session.mcp_config_path {
    args.push("--mcp-config");
    args.push(mcp_config);
}
```

### Fix 2: Add mcp_config to AgentConfig (~5 min)

**File:** `crates/roko-core/src/config/agent.rs`

```rust
pub struct AgentConfig {
    // existing fields...
    #[serde(default)]
    pub mcp_config: Option<String>,
}
```

### Fix 3: Run MCP discovery in ACP session init (~10 min)

**File:** `crates/roko-acp/src/session.rs`

On session initialization, run MCP auto-discovery and merge with config:
```rust
let discovered = mcp_discovery::discover(workdir)?;
let mcp_servers = merge_mcp_config(&config.agent.mcp_config, &discovered);
session.mcp_config_path = write_merged_mcp_config(&mcp_servers)?;
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-acp/src/bridge_events.rs` | Pass MCP config to Claude CLI chat |
| `crates/roko-core/src/config/agent.rs` | Add `mcp_config` field |
| `crates/roko-acp/src/session.rs` | Run MCP auto-discovery |

## Priority

**P2** — MCP servers provide agents with external tools (GitHub, Slack, code intelligence).
Without passthrough in ACP, agents in Zed don't get these capabilities.
