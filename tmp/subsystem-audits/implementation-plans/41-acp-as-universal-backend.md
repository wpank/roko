# 41 — ACP As Universal Backend

Doc 42 idea E: today there are three separate session implementations
(`ChatAgentSession`, `AcpSession`, TUI's `App` state). They each
manage their own history, model selection, and tool policy. The
proposal: make `AcpSession` the universal backend for all surfaces.

This is the **deepest architectural refactor** in the forward-looking
plans. It's foundational for plan 42 (work items) but optional for plan
40.

---

## Today's State

Three session implementations:

- `ChatAgentSession` (`crates/roko-cli/src/chat_session.rs`): chat REPL
  primary state; conversation history (limited), model switching, slash
  commands.
- `AcpSession` (`crates/roko-acp/src/session.rs`): ACP protocol session;
  conversation history (40 turns / 64K chars), config state, approval
  flow, workflow integration, trust state, cancel token.
- TUI's `App` (`crates/roko-cli/src/tui/app.rs`): TUI per-tab state;
  no real session abstraction.

`AcpSession` has the richest feature set, so it becomes the substrate.

---

## Anti-Patterns

1. **No copy-pasting `AcpSession` into chat / TUI.** Migrate to a
   shared crate.
2. **No "thin wrapper around AcpSession that adds three fields."** If
   chat needs more state, add it to `AcpSession`.
3. **No JSON-RPC envelope for in-process consumers.** ACP's JSON-RPC
   layer is a transport; the session machinery should be reachable
   directly in-process.

---

## Plan

### Phase 1: Extract `AcpSession` core to a shared crate

**Files**: Move from `crates/roko-acp/src/session.rs` to
`crates/roko-session/src/lib.rs` (new crate).

```
crates/roko-session/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── session.rs          // the core Session type
│   ├── history.rs          // conversation history with FIFO trim
│   ├── approval.rs         // request/grant/deny flow
│   └── trust.rs            // workspace trust state
```

`roko-acp` becomes a thin transport wrapper around `roko-session`.

### Phase 2: Migrate `ChatAgentSession` to `Session`

`ChatAgentSession` becomes:

```rust
pub struct ChatAgentSession {
    inner: Session,
    // chat-specific state: slash command palette, completion suggestions
}

impl ChatAgentSession {
    pub async fn send_turn_api(&mut self, prompt: &str) -> Result<...> {
        self.inner.dispatch_user_message(prompt).await
    }
    // ...
}
```

The `inner` is the universal `Session`. Anything chat-specific (slash
commands, palette) wraps it.

### Phase 3: TUI consumes `Session`

The TUI's per-tab state holds an `Arc<RwLock<Session>>` and reads its
DashboardSnapshot. Approval flows go through the session's
`request_permission` method.

### Phase 4: Cross-surface session continuity

Once all surfaces use `Session`:

- A session has a stable ID. `roko serve` exposes
  `GET /api/sessions/:id`.
- Work started in CLI continues in TUI: same session ID.
- Work started in editor (ACP) visible in TUI.

Persist session state under `.roko/sessions/<id>.json`.

---

## Plan

- [ ] Phase 1 — Extract to `roko-session` crate
- [ ] Phase 2 — `ChatAgentSession` wraps `Session`
- [ ] Phase 3 — TUI consumes `Session`
- [ ] Phase 4 — Cross-surface continuity

**Estimated effort**: 30-50 hours. **Don't start before plan 22
finishes** (dispatch consolidation must be done first; otherwise the
shared session has three different dispatch paths to support).
