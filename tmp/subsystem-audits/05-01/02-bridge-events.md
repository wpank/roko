# 02 — Bridge Events Anti-Patterns

## HIGH: Unreachable panic in `map_event_to_update`

**File:** `crates/roko-acp/src/bridge_events.rs:2940`

```rust
fn map_event_to_update(event: CognitiveEvent) -> SessionUpdate {
    match event {
        // ... handled cases ...
        CognitiveEvent::Complete { .. } | CognitiveEvent::MaxTokens => {
            unreachable!("terminal cognitive events are handled before update mapping")
        }
    }
}
```

Terminal events (`Complete`, `MaxTokens`) are handled by a separate match arm in `stream_events_to_editor` before `map_event_to_update` is called. However, the catch-all `other =>` branch calls `map_event_to_update(other)`, which will **panic** if a terminal event somehow reaches it due to refactoring or new event variants.

**Fix:** Return an error or a no-op update instead of panicking:
```rust
CognitiveEvent::Complete { .. } | CognitiveEvent::MaxTokens => {
    // Should not reach here; caller handles terminal events separately.
    return SessionUpdate::Plan { entries: vec![] }; // no-op
}
```

---

## CRITICAL: Dead `_history_context` variable

**File:** `crates/roko-acp/src/bridge_events.rs:993-997`

```rust
let _history_context = if should_resolve_context {
    session.build_history_context_for_cli()
} else {
    String::new()
};
```

The `_history_context` was used by the old `run_claude_cognitive_task` ClaudeCli path to prepend conversation history to the prompt. After the refactor to `run_anthropic_cognitive_task` (which takes structured `messages` instead), this variable is computed but never used.

**Impact:** The Anthropic API path uses structured `messages` array which already includes conversation history. But if `build_history_context_for_cli()` is the *only* place history gets included, and the `messages` array doesn't include prior turns, then **conversation history is silently dropped**.

**Must verify:** Does the `messages` array built at line 998+ include prior conversation turns from `session.conversation_history`?

---

## MEDIUM: Dead Claude CLI stream types

**File:** `crates/roko-acp/src/bridge_events.rs:55-135`

Lines 55-135 define `ClaudeStreamEvent`, `ClaudeSystemEvent`, `ClaudeAssistantEvent`, `ClaudeToolEvent`, `ClaudeResultEvent`, `ClaudeUsage` — all marked `#[allow(dead_code)]`.

These were for the Claude CLI subprocess fallback path which has been replaced by the Anthropic API dispatch. Dead code adds maintenance burden and hides the fact that this path is gone.

**Fix:** Delete these types.

---

## MEDIUM: Missing error recovery in permission flow

**File:** `crates/roko-acp/src/bridge_events.rs:617-679`

When `request_permission` gets an error response or a transport error, it returns `PermissionDecision::Reject` silently. The editor can't distinguish between:
- User explicitly rejected the permission
- Network/transport error prevented the request
- Timeout expired

**Fix:** Return a `PermissionDecision::Error(reason)` variant, or at minimum log the distinction.

---

## MEDIUM: Knowledge query has no timeout

**File:** `crates/roko-acp/src/bridge_events.rs:960-967`

```rust
let knowledge = if is_slash_command {
    DispatchKnowledge::default()
} else {
    query_dispatch_knowledge(workdir, &prompt_text).await
};
```

If the neuro store is slow or deadlocked, this blocks the entire prompt dispatch. No `tokio::time::timeout` wrapper.

**Fix:** Wrap in a timeout:
```rust
let knowledge = match tokio::time::timeout(
    Duration::from_secs(5),
    query_dispatch_knowledge(workdir, &prompt_text)
).await {
    Ok(k) => k,
    Err(_) => DispatchKnowledge::default(),
};
```

---

## MEDIUM: Hardcoded `max_tokens: 8192` in Anthropic dispatch

**File:** `crates/roko-acp/src/bridge_events.rs` (new code in working tree)

```rust
let mut body = serde_json::json!({
    "model": slug,
    "messages": api_messages,
    "max_tokens": 8192,
    "stream": true
});
```

8192 tokens is low for complex tasks. Should derive from the effort config:
- `low` → 4096
- `medium` → 8192
- `high` → 16384
- `max` → 65536 (or model max)

---

## LOW: `expect()` panics on stdout unwrap

**File:** `crates/roko-acp/src/bridge_events.rs:2731, 2828`

```rust
let stdout = child.stdout.take().expect("stdout was piped");
```

Will panic if the process wasn't spawned with `.stdout(Stdio::piped())`. Should use `ok_or_else` + `?` instead.

---

## LOW: Cascade router IO lock is global

**File:** `crates/roko-acp/src/bridge_events.rs:178, 300`

```rust
static CASCADE_ROUTER_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
```

All ACP sessions share this lock for cascade router file I/O. Under load with multiple sessions, this serializes all routing decisions. No timeout, so deadlock risk if holder panics.
