# 04 — Terminal & Serve Anti-Patterns

## MEDIUM: Session ID remapping allows arbitrary IDs

**File:** `crates/roko-serve/src/terminal.rs:323-341`

The WebSocket handler auto-creates a session and then reassigns its ID to whatever the client requested:

```rust
let (new_id, reader, sess_gen) = create_session(...)?;
if new_id != id {
    info_map.remove(&new_id);
    sessions.remove(&new_id);
    info_map.insert(id.clone());
    sessions.insert(id.clone());
}
```

No validation on the requested ID. Client can request any string, including path-like IDs. No collision checking — if the ID already exists, the old session is silently overwritten.

---

## MEDIUM: No per-auth session isolation

Terminal sessions are stored in a global `HashMap<String, PtySession>`. No auth context is attached to sessions. Any client that can reach the terminal endpoint can:
- List all sessions (GET `/api/terminal/sessions`)
- Write to any session by ID (via WebSocket)
- Destroy any session (DELETE `/api/terminal/sessions/{id}`)

On loopback, terminal auth is disabled entirely.

---

## MEDIUM: WebSocket hardcodes 80x24 terminal

**File:** `crates/roko-serve/src/terminal.rs:325`

```rust
let (reader, sess_gen) = match state.terminal_sessions.create_session(80, 24, None, None)
```

WebSocket creates fixed 80x24 terminal. The REST API respects `cols`/`rows` from the request body, but the WebSocket upgrade ignores query params or initial messages for dimensions.

The demo app sends resize messages after connection, but there's a race — output before the resize arrives renders in 80x24.

---

## MEDIUM: ZDOTDIR temp directories never cleaned up

**File:** `crates/roko-serve/src/terminal.rs:141-144`

```rust
let zdotdir = std::env::temp_dir().join(format!("roko-zdot-{}", Uuid::new_v4()));
let _ = std::fs::create_dir_all(&zdotdir);
let _ = std::fs::write(zdotdir.join(".zshrc"), "PS1='%~ %# '\n");
```

Every terminal session creates a unique temp directory for a custom `.zshrc`. These are never cleaned up. On a long-running server spawning many terminals, `/tmp/roko-zdot-*` accumulates indefinitely.

---

## MEDIUM: No rate limiting on session creation

No limit on:
- Number of PTY sessions per client/IP
- Rate of input keystrokes via WebSocket
- Rate of resize requests

A single client can exhaust file descriptors by creating hundreds of PTY sessions.

---

## MEDIUM: JSON parse for resize uses string prefix matching

**File:** `crates/roko-serve/src/terminal.rs:393-402`

```rust
if text.starts_with("{\"type\":\"resize\"") || text.starts_with("{\"type\": \"resize\"") {
```

Fragile string prefix matching instead of proper JSON parsing. Accepts any payload that starts with the right prefix, even if the rest is malformed JSON.

---

## LOW: Generation counter resets on process restart

**File:** `crates/roko-serve/src/terminal.rs:237-252`

`sess_generation` is an `AtomicU64` starting at 0. If the server restarts, the counter resets. A stale client holding generation 0 could match a new session's generation 0 and kill it via `destroy_session_if_sess_generation`.

---

## LOW: No CSRF protection for WebSocket terminal

Terminal WebSocket at `/ws/terminal/{id}` accepts connections from any origin when `unsafe_public_cors` is enabled. A malicious website could open the WebSocket and execute commands.

---

## LOW: Terminal sessions not cleaned up on ACP disconnect

When the ACP client disconnects (Zed closes), terminal sessions spawned during the ACP session continue running. No cleanup hook ties ACP session lifecycle to terminal session lifecycle.
