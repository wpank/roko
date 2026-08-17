# Fix: Inject ROKO_SERVE_URL in PTY Terminal Sessions

## Summary

When `roko serve` spawns PTY sessions (for the terminal tab in the demo IDE), the
spawned shell has no knowledge of the running server. Commands like `roko plan run`
executed inside the PTY cannot forward events back to the serve layer.

## Current State

`crates/roko-serve/src/terminal.rs` spawns shells with minimal env:

```rust
// terminal.rs line 261
cmd.cwd(&wd);
cmd.env("TERM", "xterm-256color");
cmd.env("COLORTERM", "truecolor");
// Sets ZDOTDIR for custom .zshrc — but NO server connection env vars
```

The only place `ROKO_SERVE_URL` is read today is in the TUI's agent stream
connection (`tui/app.rs` line 3121):

```rust
fn resolve_agent_stream_server_url() -> String {
    std::env::var("ROKO_SERVE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("ROKO_SERVER_URL").ok().filter(|v| !v.trim().is_empty()))
        .unwrap_or_else(|| roko_cli::DEFAULT_SERVE_URL.to_string())
}
```

## The Fix

### Step 1: Inject env vars when spawning PTY

In `terminal.rs`, where `CommandBuilder` is configured:

```rust
// Add alongside existing env() calls:
cmd.env("ROKO_SERVE_URL", format!("http://127.0.0.1:{}", server_port));
cmd.env("ROKO_SERVER_AUTH_TOKEN", &auth_token);  // if auth is configured
cmd.env("ROKO_SESSION_ID", &session_id);          // for event attribution
```

The `server_port` and `auth_token` should come from `AppState` (or the serve config).

### Step 2: CLI respects ROKO_SERVE_URL for event forwarding

With the HTTP event sink from `03-CLI-EVENT-SINK.md`, any `roko` command run inside
the PTY will automatically detect `ROKO_SERVE_URL` and forward events. No additional
CLI code needed — the sink activates on env var presence.

### Step 3: Non-roko commands (optional, future)

For arbitrary commands in the PTY (e.g., `cargo test`, `git push`), we can't inject
event forwarding. However, we can:

1. **Parse PTY output** for patterns (compile errors, test results) and emit events
   server-side from the `CommandEventReader`
2. **Wrap commands** via a shell function in the custom `.zshrc`:

```zsh
# In the ZDOTDIR/.zshrc that terminal.rs creates:
roko_wrap() {
    local cmd="$*"
    # Notify server that a command started
    curl -s -X POST "$ROKO_SERVE_URL/api/events/ingest" \
        -H "Content-Type: application/json" \
        -d "{\"type\":\"operation_started\",\"opId\":\"$RANDOM\",\"kind\":\"terminal_command\"}" &>/dev/null &
    eval "$cmd"
    local exit_code=$?
    # Notify server of completion
    curl -s -X POST "$ROKO_SERVE_URL/api/events/ingest" \
        -H "Content-Type: application/json" \
        -d "{\"type\":\"operation_completed\",\"opId\":\"$RANDOM\",\"kind\":\"terminal_command\",\"success\":$([ $exit_code -eq 0 ] && echo true || echo false)}" &>/dev/null &
    return $exit_code
}
```

This is optional and lower priority — the main win is roko CLI commands forwarding.

## Env Vars to Inject

| Variable | Value | Purpose |
|----------|-------|---------|
| `ROKO_SERVE_URL` | `http://127.0.0.1:{port}` | Event sink target |
| `ROKO_SERVER_AUTH_TOKEN` | From config | Authentication |
| `ROKO_SESSION_ID` | PTY session UUID | Event attribution |
| `ROKO_WORKSPACE` | Working directory | Context for commands |

## Integration with SessionManager

The `SessionManager` in `terminal.rs` already tracks sessions by ID. Add the serve
URL to its `create_session` parameters:

```rust
pub(crate) fn create_session(
    &mut self,
    working_dir: &Path,
    size: PtySize,
    serve_url: &str,       // <-- add
    auth_token: Option<&str>, // <-- add
) -> Result<SessionId, TerminalError> {
    // ... existing code ...
    cmd.env("ROKO_SERVE_URL", serve_url);
    if let Some(token) = auth_token {
        cmd.env("ROKO_SERVER_AUTH_TOKEN", token);
    }
    cmd.env("ROKO_SESSION_ID", session_id.to_string());
    // ...
}
```

## Verification

1. Start `roko serve` on :6677
2. Open terminal via WebSocket (`/api/terminal/create`)
3. In the PTY, run: `echo $ROKO_SERVE_URL` → should show `http://127.0.0.1:6677`
4. Run: `roko plan run plans/test/` in the PTY
5. Observe events appearing in the SSE stream at `/api/events/stream`
6. Confirm events are attributed to the correct session via `ROKO_SESSION_ID`
