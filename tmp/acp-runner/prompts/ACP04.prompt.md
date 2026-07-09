# Batch ACP04 — Handler dispatch loop

## Goal

Implement the main ACP handler that reads messages and dispatches by method name.

## Target files

- `crates/roko-acp/src/handler.rs` — Main dispatch loop

## Implementation details

### Main entry point

```rust
pub async fn run_acp_server(config: AcpConfig) -> Result<()>
```

This is the function called by the CLI. It:

1. Sets up file-based logging (stdout is the protocol channel)
2. Creates a `StdioTransport`
3. Creates session storage (`HashMap<String, AcpSession>`)
4. Enters the main loop:
   - Read message from transport
   - Match on message type:
     - **Request**: dispatch by method name
     - **Response**: route to pending request via `transport.handle_incoming_response()`
     - **Notification**: dispatch by method name
   - On EOF: clean shutdown

### Method dispatch table

| Method | Type | Handler |
|--------|------|---------|
| `initialize` | Request | Return `InitializeResult` with agent capabilities |
| `session/new` | Request | Create session, return config options |
| `session/list` | Request | List active sessions |
| `session/load` | Request | Load/resume a session |
| `session/prompt` | Request | Handle prompt (streaming) — delegates to `bridge_events` |
| `session/cancel` | Notification | Cancel active prompt in session |
| `session/config/update` | Request | Update config option |
| `session/set_mode` | Request | Legacy mode switch (translate to config update) |

### Error handling

- Unknown method -> `METHOD_NOT_FOUND` error
- Malformed params -> `INVALID_PARAMS` error
- Invalid session ID -> `SESSION_NOT_FOUND` error
- Session busy (prompt in flight) -> `SESSION_BUSY` error

### Logging setup

```rust
fn setup_file_logging(log_file: &Path) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let file_appender = tracing_appender::rolling::never(
        log_file.parent().unwrap_or(Path::new(".")),
        log_file.file_name().unwrap_or_default(),
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter("roko_acp=debug")
        .init();
    Ok(guard)
}
```

### Stubs for later batches

For methods that depend on later batches, implement minimal stubs:
- `session/prompt` -> return empty result with `stop_reason: end_turn` (wired in ACP06)
- Config options -> return empty list (wired in ACP15)
- Slash commands -> return empty list (wired in ACP16)

## Verification

```bash
cargo check -p roko-acp
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- `run_acp_server()` compiles and handles all method dispatch
- File logging is set up (no stdout logging)
- All error cases return proper JSON-RPC errors
- Stubs are in place for not-yet-implemented features
