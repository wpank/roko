# S-term-5: Per-session auth + generation counter for reconnect safety

## Task
Two related fixes:
(a) `TerminalSession` carries an `owner` field; reconnects that don't match the owner are rejected.
(b) `TerminalSession` carries a `generation: u64` counter; reconnect requests with a stale generation are rejected.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-term-1. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/26-terminal-demo-truth.md` § Phase 5 C & D.

## Why
Today, anyone with a valid API key can attach to any `session_id`. And reconnecting can mix old and new terminal output if the session was reset between connects.

## Exact changes

### A. Per-session owner

```rust
pub struct TerminalSession {
    pub session_id: String,
    pub owner: SessionOwner,    // identifies the API key / user
    pub generation: u64,
    // ...
}

pub enum SessionOwner {
    ApiKey(String),    // hashed
    Anonymous,         // local dev with auth disabled
    User(String),      // Privy / authenticated user
}
```

The `create_session` route stamps the owner from the request's auth context. The `attach`/`io`/`events` WS handlers verify:

```rust
async fn handle_io(ws: WebSocket, state: Arc<AppState>, session_id: String, requester: SessionOwner) {
    let session = state.terminal_sessions.get(&session_id).await
        .ok_or_else(|| TerminalError::NotFound)?;
    if session.owner != requester {
        send_error(ws, "session not owned by requester").await;
        return;
    }
    // ... proceed
}
```

Compare via constant-time on the hash if `ApiKey` is hashed.

### B. Generation counter

```rust
pub struct TerminalSession {
    pub generation: u64,
    // ...
}

impl TerminalSession {
    pub fn bump_generation(&mut self) {
        self.generation += 1;
    }
}

// In the WS upgrade handlers, accept `?gen=<u64>` query param:
async fn handle_io(
    ws: WebSocket,
    state: Arc<AppState>,
    Path(session_id): Path<String>,
    Query(params): Query<ConnectParams>,
) {
    if let Some(expected) = params.gen {
        if expected != session.generation {
            send_error(ws, "stale generation; reconnect not allowed").await;
            return;
        }
    }
    // ...
}

#[derive(Deserialize)]
struct ConnectParams { gen: Option<u64> }
```

The frontend tracks the generation it received at session-create and includes it in reconnect URLs. If the session was destroyed and recreated with the same id (shouldn't happen normally; rejected by `create_session` if the id exists), the generation differs and reconnect fails fast.

### C. Tests

```rust
#[tokio::test]
async fn rejects_attach_from_different_owner() {
    let session = create_test_session_for("apikey-A").await;
    let resp = attach_io(&session.id, /* requester */ "apikey-B").await;
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn rejects_attach_with_stale_generation() {
    let session = create_test_session().await;
    // session.generation == 1
    session.bump_generation().await;  // now 2
    let resp = attach_io_with_gen(&session.id, 1).await;
    assert_eq!(resp.status(), 410);   // Gone
}
```

## Write Scope
- `crates/roko-serve/src/terminal.rs`
- `crates/roko-serve/src/terminal/session.rs` (if split)
- `crates/roko-serve/src/state.rs` (only if `TerminalSession` lives outside `terminal.rs`)

## Verify

```bash
rg 'SessionOwner|generation: u64|owner: SessionOwner' crates/roko-serve/src/terminal*.rs
# Expect: at least 3 hits

rg 'rejects_attach_from_different_owner|rejects_attach_with_stale_generation' crates/roko-serve/
# Expect: 2 hits
```

## Do NOT

- Do NOT use plain-text API keys in `SessionOwner`. Hash.
- Do NOT remove existing auth middleware (`require_api_key`) — owner check is in addition, not replacement.
- Do NOT bundle with S-term-4.
- Do NOT include rate-limit logic here (T3-23 owns).
