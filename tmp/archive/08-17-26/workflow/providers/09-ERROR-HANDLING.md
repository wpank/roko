# 09 — Error Handling, Stderr & Provider Health

## The Problem

Mori has error classification, stderr filtering, DOA detection, and simple retry.
Roko has basic pattern matching, discards stderr, no DOA detection, and CascadeRouter
health tracking that's never consulted.

---

## Mori's Error Handling

### Stderr Classification
**File**: `connection.rs:826-984` — `benign_stderr_summary()`

Categorizes ~30 stderr patterns as "benign" (logged once, then suppressed):
- `"apply_patch verification failed"` — migration issue
- `"failed to open state db at"` — db migration
- `"Failed to delete shell snapshot"` — ENOENT
- `"timed out waiting for thread"` — thread join timeout
- `"Failed to kill process group"` — process cleanup
- Cursor race condition on `.cursor/cli-config.json`
- Codex rollout channel closures
- sqlx slow statement warnings
- Unknown model metadata fallbacks

Non-benign stderr → error event. Prevents TUI flooding from known harmless messages.

### Error Classification in Result Events
```rust
ClaudeStreamEvent::Result(r) => {
    if r.is_error && r.subtype != "success" {
        tx.send(AgentEvent::Error {
            error: format!("claude turn error: subtype={}", r.subtype),
        });
    }
}
```

### Process Exit Handling
```rust
// Claude: read stdout/stderr until EOF, wait 5s for exit code
// Codex: thread/start has 75s timeout with 3 retries
// Cursor: 1.2s graceful + 0.8s SIGTERM + 0.8s SIGKILL
```

### Fallback on Spawn Failure
```rust
match self.try_spawn(role, effort, model).await {
    Ok(()) => Ok(()),
    Err(primary_err) => {
        if let Some(fb) = self.fallback_model.clone() {
            return self.try_spawn(role, effort, Some(&fb)).await;
        }
        Err(primary_err)
    }
}
```

One retry with fallback model if primary spawn fails.

---

## Roko's Error Handling

### dispatch_direct.rs (chat)
```rust
// Claude CLI: check exit code, capture stderr
if !status.success() {
    let stderr_text = /* read stderr */;
    bail!("claude CLI exited with {status}: {stderr_text}");
}

// Anthropic API: HTTP status check
if !resp.status().is_success() {
    bail!("Anthropic API {status}: {body_text}");
}

// OpenAI-compat: HTTP status check
if !resp.status().is_success() {
    bail!("OpenAI API {status}: {body_text}");
}
```

### chat_inline.rs error recovery
```rust
Phase::Error { prompt, error } => {
    // Show error with suggestions
    // [r]etry / [q]uit
}
```

Error suggestions pattern matching:
- "429" / "rate limit" → "Wait a moment"
- "401" / "403" / "authentication" → "Check API key"
- "timeout" / "timed out" → "Try again"
- "connection" / "network" → "Check connectivity"

### agent_stream.rs stderr handling
```rust
// run.rs agent_stream: stderr logged at debug! and discarded
// Lines 277-285
debug!("agent stderr: {}", line);
// No benign classification, no filtering, no error events
```

### Provider retry (roko-agent)
```rust
// roko-agent/src/retry.rs
// Has RetryPolicy with max_retries, classify errors as:
// - Retryable (rate limit, server error, timeout)
// - Fatal (auth failure, invalid request, context overflow)
// - Unknown
```

This exists but is only used by the roko-agent provider adapters,
not by the CLI dispatch paths.

---

## Provider Health Tracking

### Mori: Simple
No explicit health system. Relies on:
- Exit codes + stderr classification
- One retry with fallback model
- Timeout-based health (different per backend)

### Roko: CascadeRouter (built, unused)
```rust
// CascadeRouter in roko-learn/
// LinUCB bandit-based routing with:
// - Context features (tier, frequency, budget bias)
// - Per-model performance history
// - Persists to .roko/learn/cascade-router.json
//
// But: NEVER CONSULTED AT RUNTIME
// Model selection is static across all dispatch paths
```

The CascadeRouter is comprehensive but:
1. No feature vector computed at dispatch time
2. No routing observations recorded from actual runs
3. No fallback chain triggered on health degradation
4. Not wired into any of the 6+ dispatch paths

---

## What's Missing in Roko

### 1. Stderr classification
No benign/important classification. All stderr either logged at debug level
or shown raw to the user. Floods chat with unhelpful messages.

### 2. DOA detection
No detection of agent process exiting within 2 seconds of spawn.
If the binary doesn't exist or auth fails immediately, the user gets
a generic "exit code 1" error.

### 3. Error event classification
dispatch_direct.rs shows raw error strings. No structured classification
into rate_limit/auth/timeout/context_overflow for different recovery actions.

### 4. Health-based routing
CascadeRouter exists but is never consulted. No automatic downgrade from
Opus to Sonnet on repeated failures. No automatic fallback to different
providers when one is rate-limited.

### 5. Retry at dispatch level
roko-agent has retry logic but CLI dispatch paths don't use it.
chat_inline.rs has manual retry (Phase::Error) but no automatic retry.

---

## What Needs to Change

1. **Wire CascadeRouter** into dispatch for model selection with health feedback
2. **Add stderr classification** — port mori's benign_stderr_summary concept
3. **Add DOA detection** — check if process exits within 2s
4. **Structured error classification** — rate_limit, auth, timeout, context_overflow
5. **Automatic retry with fallback** — retry once with fallback model on spawn failure
6. **Record routing observations** — feed success/failure back to CascadeRouter
