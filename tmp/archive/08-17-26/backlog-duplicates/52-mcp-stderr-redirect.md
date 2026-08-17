# MCP Server Stderr Redirect

**Priority**: P2 — MCP subprocess stderr pollutes terminal output
**Size**: XS (½ day)
**Crate**: `crates/roko-agent/`

---

## Problem

`crates/roko-agent/src/mcp/client.rs` spawns MCP server subprocesses with
`.stderr(Stdio::inherit())` (line ~229). This means all MCP server stderr output
(errors, warnings, debug logs, progress messages) goes directly to the parent
process's stderr.

When running `roko serve` or `roko plan run`, MCP server noise is interleaved with
roko's own output, making it difficult to distinguish roko errors from MCP server
errors. If multiple MCP servers are running concurrently, their stderr output is
interleaved without any source attribution.

---

## Where to look

- `crates/roko-agent/src/mcp/client.rs` — line ~229, `.stderr(Stdio::inherit())`

---

## What to do

**Step 1.** Change `.stderr(Stdio::inherit())` to `.stderr(Stdio::piped())`.

**Step 2.** Spawn a background task to read the piped stderr and forward lines through
`tracing` with the MCP server name as context:

```rust
let stderr = child.stderr.take().expect("piped stderr");
let server_name = server_name.clone();
tokio::spawn(async move {
    let reader = BufReader::new(stderr);
    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        tracing::debug!(mcp_server = %server_name, "{}", line);
    }
});
```

**Step 3.** Log at `debug` level by default. MCP server errors that indicate a crash
or protocol violation should be logged at `warn`.

---

## Acceptance criteria

- [ ] MCP server stderr is piped, not inherited
- [ ] Stderr lines are forwarded through `tracing` with the server name as context
- [ ] Default log level for MCP stderr is `debug`
- [ ] MCP server crash/exit is logged at `warn`
- [ ] All existing tests pass (`cargo test -p roko-agent`)

---

**Origin**: productionizing audit (2026-08-13)
