# MCP Server Patterns

## Observed patterns in the ecosystem
1. **Stdio transport** — most common, simplest, used by Claude Desktop
2. **HTTP+SSE** — for remote servers, better for multi-client
3. **WebSocket** — lowest latency, complex lifecycle

## Our approach
- Stdio for local tools (code intelligence, file ops)
- HTTP+SSE for the control plane sidecar
- Config passthrough via `agent.mcp_config` in roko.toml

## Code intelligence MCP (roko-mcp-code)
- Tools: search, goto-definition, find-references, diagnostics
- Uses tree-sitter for parsing
- HDC vectors for semantic similarity
