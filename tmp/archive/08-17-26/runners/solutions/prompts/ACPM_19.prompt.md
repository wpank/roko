# ACPM_19: Add Cross-Server Tool Call Client

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-19`](../ISSUE-TRACKER.md#acpm-19)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.19
- Priority: **P1**
- Effort: 5 hours
- Depends on: `ACPM_18` (source 9.18)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

With the registry in place, MCP servers need a client to make cross-server tool calls. The client discovers a server via registry, connects via stdio subprocess (spawning the server binary), sends a JSON-RPC request, and returns the result.

## Exact Changes

1. Define `FederatedClient`:
   ```rust
   pub struct FederatedClient {
       registry: McpRegistry,
       timeout: Duration,
       circuit_breaker: CircuitBreaker,
   }
   ```
2. Define `CircuitBreaker`:
   ```rust
   struct CircuitBreaker {
       failures: HashMap<String, u32>,
       threshold: u32,  // default 3
   }
   ```
3. Implement `call_tool(server_name: &str, tool: &str, args: Value) -> Result<Value>`:
   - Discover server via registry
   - Check circuit breaker (open = immediate error)
   - Spawn the server binary as a subprocess with stdio
   - Send `tools/call` JSON-RPC request with the tool name and args
   - Apply timeout (default 30s)
   - On success: reset circuit breaker for this server, return result
   - On failure: increment circuit breaker counter, return error
4. Implement `call_tool_by_name(tool: &str, args: Value) -> Result<Value>` that auto-discovers which server provides the tool.
5. Add `pub mod federation;` to `lib.rs`.

## Design Guidance

Each cross-server call spawns a fresh subprocess (the MCP server binary), sends one request, reads one response, and terminates. This is simple and avoids connection lifecycle management. For high-throughput scenarios, a persistent connection pool could be added later but is not needed now.

## Write Scope

- `crates/roko-mcp-stdio/src/lib.rs`
- `crates/roko-mcp-stdio/Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Integration test: server A calls tool on server B via federation
- [ ] Timeout triggers after configured duration with clear error message
- [ ] Circuit breaker opens after threshold consecutive failures
- [ ] Circuit breaker resets on successful call

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Integration test: server A calls tool on server B via federation
- Timeout triggers after configured duration with clear error message
- Circuit breaker opens after threshold consecutive failures
- Circuit breaker resets on successful call
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
