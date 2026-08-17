# 05 — Session Resume & Conversation Continuity

## The Problem

Mori captures `session_id` from Claude's Result event and passes it back via `--resume`
on every subsequent turn. This gives Claude full conversation history without re-sending
the entire context. Roko has the infrastructure but applies it inconsistently.

---

## How Mori Does It

### Capture
**File**: `connection.rs:3184-3210`

```rust
ClaudeStreamEvent::Result(r) => {
    // ...
    let _ = tx.send(AgentEvent::TurnCompleted {
        role,
        instance: instance.clone(),
        thread_id: Some(r.session_id),  // session_id → thread_id
    });
}
```

### Storage
```rust
// connection.rs:2343-2344
current_session_id: Option<String>,

// Set when TurnCompleted event received by pool:
pool.set_thread_id(role, Some(session_id));
```

### Resume
**File**: `connection.rs:2574-2576`

```rust
// On turn_start():
if let Some(ref sid) = self.current_session_id {
    cmd.arg("--resume").arg(sid);
}
```

### Result
Every subsequent turn in the same conversation resumes the Claude session.
Claude Code has the full message history server-side. No need to re-send context.
This is critical for multi-turn agent workflows.

---

## How Roko Does It

### dispatch_direct.rs (chat path) — Partially done
```rust
// session_id captured from Result event (line 128):
if let Some(sid) = event.get("session_id").and_then(Value::as_str) {
    session_id = Some(sid.to_string());
}

// Returned in DispatchResult:
DispatchResult { ..., session_id }
```

But: `session_id` is returned to the caller and then **ignored**.
`chat_inline.rs` receives `DispatchResult` but never stores or passes
back the session_id on the next turn.

### run.rs (plan execution) — Partial
```rust
// run.rs:536-539
if let Some(session_id) = optional_resume_session_id(&config, prev_session_id.as_deref()) {
    cmd.arg("--resume").arg(&session_id);
}
```

This works but only in the Claude CLI subprocess path (Path 3).
The other 5 paths don't have resume support.

### orchestrate.rs — Partial
```rust
// orchestrate.rs:1546-1547, 1607-1608, 15113-15114
// --resume passed when session_id available
```

This works for plan execution but session_id must be captured from the
agent's stream output first.

---

## What's Broken

1. **Chat has no multi-turn resume** — Each chat message is a fresh prompt.
   `session_id` captured but not stored or passed back on next turn.

2. **No session_id storage** — `ChatSession` struct has no `last_session_id` field.
   Even if captured, nowhere to put it.

3. **Only Claude CLI path has resume** — Anthropic API, OpenAI-compat, Ollama
   paths don't have any resume/conversation mechanism.

4. **HTTP chat path can't resume** — When chat goes through HTTP (sidecar/serve),
   the session_id is lost in the HTTP response processing.

---

## What Needs to Change

1. Add `session_id: Option<String>` to `ChatSession`
2. Store `result.session_id` after each turn
3. Pass session_id to `dispatch_prompt()` for Claude CLI path
4. `dispatch_claude_cli()` accepts optional session_id and passes `--resume`
5. For HTTP backends: implement multi-turn via message history instead
