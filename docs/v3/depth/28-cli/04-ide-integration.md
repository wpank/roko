# 28.04 -- IDE Integration

> Depth file for [28-CLI.md](../../28-CLI.md) -- v1/12/20.

---

## Integration Points

Roko integrates with IDEs through three mechanisms:

1. **ACP server** (`roko acp`) -- Primary Cursor integration via stdio JSON-RPC
2. **MCP servers** -- Code intelligence and tool serving
3. **CLI commands** -- Direct invocation from IDE terminals

## ACP Integration

The ACP server is the primary integration path for Cursor and compatible
editors. See `27-acp/` depth files for full protocol details.

### Editor Configuration

Cursor's `settings.json` configures the ACP server:

```json
{
  "roko.acp.command": "roko",
  "roko.acp.args": ["acp", "--config", "roko.toml"],
  "roko.acp.globalConfig": "~/.roko/config.toml"
}
```

### Settings JSON Generation

The ACP runner generates a `.claude/settings.json` for Claude CLI sessions:

```rust
pub fn build_settings_json(
    workdir: &Path,
    sandbox_level: RunnerSandboxLevel,
    mcp_config: Option<&Path>,
) -> serde_json::Value {
    // Generates allowedTools, mcpServers, etc.
}
```

This enables MCP tool availability and sandbox policy within Claude CLI
sessions spawned by the ACP pipeline.

## MCP Code Intelligence

The `roko-mcp-code` crate provides a Model Context Protocol server for
code intelligence:

```bash
# Start the MCP code intelligence server
roko mcp code-intel
```

This serves code analysis tools (search, parse, index) over the MCP stdio
protocol, making them available to any MCP-compatible editor.

### MCP Server Configuration

```bash
# List configured MCP servers
roko config mcp list

# Test MCP server connectivity
roko config mcp test <name>

# Add an MCP server
roko config mcp add <name> --command <cmd>
```

MCP servers are configured in `roko.toml`:

```toml
[[mcp_servers]]
name = "code-intel"
command = "roko"
args = ["mcp", "code-intel"]
```

## Tool Definitions

Roko exposes 52 tool definitions through its tool registry (`roko-std`):

- 16 executable local tools
- 19 GitHub MCP tools
- 17 typed optional-chain placeholders

Tools are defined with JSON Schema input specifications and made available
to agents through the MCP protocol or direct API dispatch.

### Tool Policy

The `AgentContract` system controls which tools are available per role:

```rust
pub struct AgentContract {
    pub allowed_tools: Vec<String>,  // Allowlist
    pub denied_tools: Vec<String>,   // Blocklist (wins over allow)
    pub role: AgentRole,
}
```

Denials always win. Unknown roles deny all tools. Unsupported policy-bearing
dispatches are rejected.

## CLI Terminal Integration

IDE terminals can run any `roko` subcommand directly:

### Common Terminal Workflows

```bash
# Quick research from terminal
roko think "How does the routing system work?"

# Capture a note without LLM
roko note "TODO: fix the flaky test" --tags "bugs,tests"

# Check workspace health
roko doctor

# Interactive dashboard in terminal
roko dashboard
```

### JSON Output Mode

The `--json` flag enables machine-readable output for IDE extension
consumption:

```bash
roko status --json | jq '.agents'
roko plan status plans/my-plan --json
roko learn gates --json
```

### Quiet Mode

`--quiet` suppresses non-essential output, useful for IDE-driven automation:

```bash
roko run "Fix the import" --quiet --json
```

## Config Provider Discovery

The IDE integration leverages provider auto-discovery:

```bash
# Discover available providers
roko config providers discover

# List configured providers with health status
roko config providers list

# Test provider connectivity
roko config providers test anthropic
```

This helps editors surface which providers are available and functional.

## Vision Loop

The vision loop provides iterative visual refinement, useful for UI
development in editors:

```bash
roko vision-loop screenshot.png
```

The loop:
1. Reads the screenshot
2. Identifies UI issues via vision-capable model
3. Generates fixes
4. Takes new screenshot
5. Repeats until satisfactory

Vision capabilities are advertised in the ACP `initialize` response based
on the default model's `supports_vision` flag.

## Shell Completions

IDE terminal integration benefits from shell completions:

```bash
# Generate completions for your shell
roko completions zsh > ~/.zfunc/_roko
roko completions bash > /etc/bash_completion.d/roko
roko completions fish > ~/.config/fish/completions/roko.fish
```

See `completions-and-shell.md` for details.

## Source

- `crates/roko-acp/` -- ACP server for editor integration
- `crates/roko-mcp-code/` -- Code intelligence MCP server
- `crates/roko-std/` -- Tool definitions and registry
- `crates/roko-agent/src/safety/contract.rs` -- AgentContract tool policy
