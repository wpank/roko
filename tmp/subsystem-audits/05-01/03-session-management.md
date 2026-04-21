# 03 — Session Management Issues

## HIGH: TOCTOU race on session busy flag

**File:** `crates/roko-acp/src/bridge_events.rs:905-915`

```rust
if session.is_busy() {
    return Err(BridgeEventsError::SessionBusy(session.session_id.clone()));
}
session.begin_prompt();  // Sets busy = true
```

Between the `is_busy()` check (Acquire load) and `begin_prompt()` (Release store), another concurrent task could also see `false` and proceed. The atomic bool doesn't protect the transition.

**Fix:** Use `compare_exchange`:
```rust
if session.busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
    return Err(BridgeEventsError::SessionBusy(...));
}
// busy is now exclusively true, proceed
```

---

## MEDIUM: Conversation history trimmed silently

**File:** `crates/roko-acp/src/session.rs`

```rust
fn trim_history(&mut self) {
    while self.conversation_history.len() > MAX_HISTORY_TURNS {
        self.conversation_history.remove(0);
    }
}
```

When history exceeds 40 turns or 64KB, old turns are silently removed. The client never knows context was dropped. This causes:
- "I told you earlier to..." referencing a turn that no longer exists
- Inconsistent behavior as conversations get longer

**Fix:** Send a notification when history is trimmed:
```rust
// session/notification: { "type": "history_trimmed", "turnsTrimmed": N }
```

---

## MEDIUM: Session persistence not synchronized

**File:** `crates/roko-acp/src/handler.rs:189`

```rust
sessions.persist_session(&session_id_for_persist);
```

Sessions are persisted after each prompt completes, but:
- No file lock prevents concurrent writes from multiple processes
- If two prompts complete simultaneously, the second write may include stale state
- No fsync guarantee — crash between write and flush loses session

---

## MEDIUM: Config update has no validation

**File:** `crates/roko-acp/src/session.rs:562+`

`apply_config_option` accepts arbitrary `serde_json::Value` and applies it without checking:
- Is the value type correct? (e.g., string for "model", bool for "clippy_enabled")
- Is the value in the allowed set? (e.g., effort must be "low"/"medium"/"high"/"max")
- Will this model/provider combination work?

A bad config update silently corrupts session state.

---

## LOW: Hardcoded defaults don't reflect roko.toml

**File:** `crates/roko-acp/src/session.rs:153-175`

```rust
impl Default for SessionConfigState {
    fn default() -> Self {
        Self {
            provider: "anthropic".to_owned(),
            model: FALLBACK_MODEL.to_owned(), // "sonnet"
            ...
        }
    }
}
```

The `Default` impl is used when no roko.toml is found. But `FALLBACK_MODEL = "sonnet"` is a roko-internal key, not an Anthropic model ID. If this default is used without being resolved via the model table, the Anthropic API dispatch will send `"model": "sonnet"` which Anthropic will reject.

---

## LOW: Session GC uses 7-day hardcoded TTL

**File:** `crates/roko-acp/src/session.rs`

Session garbage collection runs at startup with a 7-day cutoff. Not configurable. For rapid development with many sessions, this can accumulate hundreds of session files.
