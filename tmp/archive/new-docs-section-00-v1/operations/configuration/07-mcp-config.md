# MCP Configuration

> `.mcp.json` controls which MCP (Model Context Protocol) tool servers are available to
> Roko agents. Roko discovers tool servers via this file and forwards their tools to agents.

**Status**: Shipping
**Crate**: `roko-agent`, `roko-std`
**Depends on**: [02-agent-config.md](02-agent-config.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Create `.mcp.json` in your project root. List one entry per tool server. Roko starts the
servers on agent boot and forwards their tools to every agent.

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/project"]
    }
  }
}
```

---

## What MCP Is

The Model Context Protocol is a JSON-RPC 2.0 protocol (over stdio) that standardises how
AI agents discover and call external tools. An MCP server is a lightweight process that
exposes one or more tools (file I/O, shell commands, database queries, API calls, etc.).
Roko acts as an MCP client: it starts the server processes, enumerates their tools, and
makes those tools available to agents.

Roko ships 19 built-in tools in `roko-std`. MCP tool servers extend this with
project-specific or domain-specific tooling.

---

## File Location

Roko looks for `.mcp.json` at the path specified by `agent.mcp_config` in `roko.toml`.
The default is `.mcp.json` in the project root. Override:

```toml
[agent]
mcp_config = "config/mcp.json"
```

Set `agent.mcp_config = ""` to disable MCP discovery entirely.

---

## File Format

```json
{
  "mcpServers": {
    "<server-name>": {
      "command": "<executable>",
      "args":    ["<arg1>", "<arg2>"],
      "env":     { "KEY": "value" }
    }
  }
}
```

Fields:

| Field | Required | Description |
|-------|----------|-------------|
| `command` | Yes | Executable to run (on `PATH` or absolute path) |
| `args` | No | Command-line arguments passed to the process |
| `env` | No | Environment variables injected into the server process |

The `<server-name>` key is an arbitrary label used in logs and error messages. It has no
semantic effect on tool routing.

---

## Tool Namespacing

When an MCP server exposes a tool, Roko qualifies its name as `<server-name>__<tool-name>`.
For example, a server named `"filesystem"` exposing `read_file` becomes `filesystem__read_file`.

Agents see the qualified names in their tool list. The namespace prefix prevents collisions
when multiple MCP servers expose tools with the same name.

---

## Built-in Tools (No MCP Config Needed)

`roko-std` ships 19 tools available to all agents without any MCP configuration:

| Tool | Category | What it does |
|------|----------|-------------|
| `read_file` | File | Read file contents with optional line range |
| `write_file` | File | Write or overwrite a file |
| `list_directory` | File | List directory contents |
| `search_files` | File | Grep-based content search |
| `shell` | Shell | Run a shell command (constrained by safety policy) |
| `cargo_check` | Rust | Run `cargo check` |
| `cargo_test` | Rust | Run `cargo nextest run` |
| `cargo_clippy` | Rust | Run `cargo clippy` |
| `search_symbols` | Index | Query the workspace symbol index |
| `get_context` | Index | Assemble context for a set of symbols |
| `find_references` | Index | Find cross-file references to a symbol |
| `get_workspace_map` | Index | Return the full workspace symbol tree |
| `web_search` | Research | Web search with citation extraction |
| `fetch_url` | Research | Fetch and extract content from a URL |
| `create_engram` | Memory | Create a new Engram in the Substrate |
| `query_engrams` | Memory | Query the Substrate by score, kind, or HDC similarity |
| `emit_pulse` | Events | Emit an ephemeral event on the Bus |
| `mcp_tool_call` | Meta | Call a tool on any connected MCP server (meta-tool) |
| `think` | Meta | Extended reasoning scratch space (no output side effects) |

---

## Common MCP Servers

### Official MCP Servers (via `npx`)

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/project"]
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": { "GITHUB_TOKEN": "${GITHUB_TOKEN}" }
    },
    "postgres": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-postgres", "${DATABASE_URL}"]
    }
  }
}
```

### Custom Rust MCP Server

Roko includes `roko-mcp-serve` as a batteries-included MCP server exposing the workspace
index tools (`search_symbols`, `get_context`, `find_references`, `get_workspace_map`):

```json
{
  "mcpServers": {
    "roko-index": {
      "command": "cargo",
      "args": ["run", "-p", "roko-mcp-serve", "--", "--workspace", "/path/to/project"]
    }
  }
}
```

---

## Environment Variable Interpolation

Values in the `env` map support `${VAR_NAME}` interpolation from the shell environment:

```json
{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_TOKEN": "${GITHUB_TOKEN}"
      }
    }
  }
}
```

Roko resolves `${GITHUB_TOKEN}` at startup time from the environment. If the variable
is not set, the server is still started (the variable becomes an empty string), but the
tool will fail at call time with an auth error.

**Do not hardcode secrets in `.mcp.json`.** The file is committed to version control.
Use environment variable interpolation for all API keys and tokens.

---

## Health Checking

At agent startup, Roko:

1. Starts each configured MCP server as a subprocess.
2. Sends an `initialize` request over stdio.
3. Calls `tools/list` to enumerate available tools.
4. If a server fails to respond within 10 seconds, it is marked as unhealthy and its
   tools are excluded from the agent's tool list.
5. A `MCP_SERVER_UNHEALTHY` warning is logged (see [operations/error-handling/08-observability.md](../error-handling/08-observability.md)).

Unhealthy MCP servers do not prevent agent startup. Agents proceed with the tools that
are available.

---

## Two Full Examples

**Coding agent (filesystem + GitHub):**

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."],
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": { "GITHUB_TOKEN": "${GITHUB_TOKEN}" }
    }
  }
}
```

**Research agent (web + databases):**

```json
{
  "mcpServers": {
    "brave-search": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-brave-search"],
      "env": { "BRAVE_API_KEY": "${BRAVE_API_KEY}" }
    },
    "postgres": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-postgres", "${DATABASE_URL}"]
    }
  }
}
```

---

## See Also

- [02-agent-config.md](02-agent-config.md) — `agent.mcp_config` key
- [13-security-considerations.md](13-security-considerations.md) — secrets in `.mcp.json` vs environment
- [reference/05-operators/](../../reference/05-operators/README.md) — how Roko's operators interact with MCP tools

## Open Questions

- Inline MCP server definition in `roko.toml` (instead of a separate `.mcp.json`) is being considered.
- Per-server retry and reconnect policy is not yet configurable.
- Tool inclusion/exclusion filters (e.g. only expose `read_file`, not `write_file`) are not yet implemented.
