# MCP Integration Issues

## Critical

### Config format mismatch — Claude CLI can't read roko's MCP config
- `orchestrate.rs:4265`: `resolve_mcp_config_path` writes `{"servers": [...]}` (roko format).
- Claude CLI expects `{"mcpServers": {"name": {...}}}` (Claude format).
- `process/mcp.rs:185-202` writes correct Claude format — but is dead code.
- Result: agents get `--mcp-config .roko/mcp-config.json` but Claude ignores it.

### Server `env` vars silently dropped
- `orchestrate.rs:4172`: `StdioTransport::spawn(&server.command, &server.args)` — no env.
- `StdioTransport::spawn_with_env` exists but not used.
- Servers requiring `GITHUB_TOKEN`, `SLACK_BOT_TOKEN` via env field → fail at first tool call.

## High

### HTTP transport not handled in `setup_mcp`
- `orchestrate.rs:4172`: Always calls `StdioTransport::spawn` without checking `server.transport`.
- HTTP-configured server → tries to spawn empty command → crash.

### `roko-mcp-code` crashes on startup without pre-built index
- `roko-mcp-code/lib.rs:196-198`: `WorkspaceIndex::load(root)` is synchronous at startup. Canonicalize failure → exit before init handshake.

### `McpHandlerResolver` built but never wired
- `mcp/handler.rs:23`, `mcp/error_accumulator.rs:58`: Built, tested, exported.
- Never instantiated in `setup_mcp` or `dispatch_task`.
- Non-CLI backends (Gemini, Ollama) list MCP tools in system prompt but have no handler to execute them.

## Medium

### `roko-mcp-slack` and `roko-mcp-scripts` not in `default-members`
- Not built by `cargo build`. Binary may not exist when configured in `.mcp.json`.

### `roko-mcp-github` blocking retry loop causes timeout
- `roko-mcp-github/main.rs:1268-1290`: 5 retries × 30s max with `thread::sleep`.
- Client has 30s response timeout. Client times out; server keeps retrying as zombie.

### `roko-mcp-scripts` silently empty when env vars missing
- `main.rs:520-531`: No `ROKO_SCRIPTS_DIR` → empty tool list. No startup warning.

### Three MCP config discovery paths, all inconsistent
- `orchestrate.rs:4265` writes roko format to `.roko/mcp-config.json`
- `process/mcp.rs:148-178` searches for Claude format (dead code)
- `mcp/config.rs` searches for `.mcp.json` (roko multi-server format)
- `workspace.rs:249-253` defines `.roko/mcp.json` (never written or read)
