# roko-std — Test Coverage

> 96 tests for the 19 built-in tools: file ops, shell execution, search, and MCP dispatch.

**Status**: Shipping
**Crate**: `roko-std`
**Section**: 18 — Tools
**Last reviewed**: 2026-04-19

---

## Test Count: 96

Source: implementation status audit, 2026-04-17.

| Tool category | Approx. tests | Focus |
|---|---|---|
| File tools | ~25 | read_file, write_file, list_dir, move_file, delete_file |
| Shell tools | ~20 | run_command, timeout, stdout/stderr capture |
| Search tools | ~15 | web_search, code_search, symbol_search |
| MCP dispatch | ~20 | MCP client integration, tool selection, result parsing |
| Content tools | ~10 | fetch_url, extract_content, diff |
| Utility tools | ~6 | sleep, log, checkpoint |

---

## Key Test Focus Areas

### File Tools

- `read_file`: reads a file in a temp dir; returns `Err(NotFound)` for missing files.
- `write_file`: creates or overwrites a file; parent directories are created automatically.
- `list_dir`: returns all entries in a directory; supports recursive listing.
- `move_file`: moves a file atomically within the same filesystem.
- `delete_file`: deletes a file; returns `Ok(())` for non-existent files (idempotent).

Key property: [../by-property/tool-file-ops-idempotence.md](../by-property/tool-file-ops-idempotence.md).

### Shell Tools

- `run_command`: captures stdout and stderr; returns exit code.
- Timeout: a command that exceeds timeout is killed and returns `Err(Timeout)`.
- Shell injection: arguments are passed as a list, never via shell interpolation (no shell injection).

### MCP Dispatch

- Tools registered via MCP are discoverable by the agent.
- An MCP tool call dispatches to the correct server.
- A missing MCP server returns `Err(ServerNotFound)` rather than hanging.
- MCP result is parsed and converted to the standard tool result format.

### Tool Dispatch Determinism

- Given the same task context and tool registry, the tool selector always chooses the same tool.

Key property: [../by-property/tool-dispatch-determinism.md](../by-property/tool-dispatch-determinism.md).

---

## Known Gaps

- `web_search` tool tests use a mock HTTP server; no tests against real search APIs.
- `fetch_url` has no tests for HTTP redirects or authentication challenges.
- MCP dispatch integration tests cover only 2 of the planned MCP server types.

## See also

- [subsystem-agent.md](subsystem-agent.md) — agent dispatches tools via roko-std
- [../by-property/tool-file-ops-idempotence.md](../by-property/tool-file-ops-idempotence.md)
