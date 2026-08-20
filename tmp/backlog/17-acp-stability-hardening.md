# 17 — ACP Stability Hardening

**Priority**: P0 — crashes in production, concurrency bugs, silent failures
**Size**: L (5–7 days)
**Crates**: `crates/roko-acp/`
**Depends on**: None (roko-agent and roko-acp both compile cleanly as of 2026-08-19)

---

## Background

Roko includes a crate called `roko-acp` that implements the Agent Client Protocol (ACP),
a JSON-RPC protocol that allows code editors (Zed, Cursor, JetBrains) to send prompts to
roko agents. When you configure Zed to use roko as its AI assistant, it communicates over
ACP. The crate handles session lifecycle, model routing, gate pipelines (compile, test,
clippy), and streaming LLM responses back to the editor.

The `roko-acp` crate is 19,915 LOC across 15 modules. It has 180 passing tests and runs
as a subprocess that the editor spawns. If `roko-acp` crashes, the editor's AI assistant
goes silent — there is no reconnect; the user has to restart. Silent failures (data silently
discarded, wrong state recorded) are almost as bad: they cause cost budgets to drift,
learning data to be lost, or code reviews to be auto-approved.

This item fixes all of the P0 crashes and the most important P1 silent failures found in
a 2026-08-15 audit. It does not restructure the code (that is item 18).

## Current State

### Verified P0 crashes (will terminate the ACP server process)

1. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` line 1698** — `session.provider_health_registry.as_ref().expect("ACP provider health initialized before prompt")`. This is called from `handle_session_prompt` (line 1657). If provider runtime initialization races or fails silently, the process dies mid-prompt.

2. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` line 1704** — `session.provider_rate_limiter.as_ref().expect("ACP provider rate limiter initialized before prompt")`. Same function, same pattern, same risk.

3. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` line 4993** — `child.stdout.take().expect("stdout was piped")` in slash-command dispatch. If the `Command` builder ever omits `Stdio::piped()`, this panics mid-prompt.

4. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` line 5311** — `child.stdout.take().expect("stdout was piped")` in a second shell-command path. Identical pattern.

5. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` line 5945** — `text[end..].chars().next().expect("valid char boundary")` in the at-mention parser. User-provided text that is not valid UTF-8 at the expected boundary panics.

6. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` line 3762** — `unreachable!("suffix search should always find a unique tool name")`. A malicious or buggy MCP server that registers many identically-named tools can hit this.

7. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` line 5439** — `unreachable!("terminal/async cognitive events are handled before update mapping")`. Adding a new `CognitiveEvent` variant without updating the match arm panics at runtime.

### Verified P1 silent failures

8. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs` line 1841** — On reviewer agent error: `warn!(error = %e, "reviewer failed, treating as approved")` then `run.pipeline.step(PipelineEvent::ReviewApproved { ... })`. A crashed review agent silently auto-approves the submission.

9. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs` lines 1910–1911 and 1937–1938** — In `run_multi_role_review` (the "thorough" review mode), `all_approved` initializes to `true` (line 1886). If the architect reviewer errors (line 1910), `all_approved` is not set to `false` — it stays `true`. Same for the auditor (line 1937). Both reviewer errors silently approve.

10. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/acp_adapter.rs` line 164** — `let _ = self.sender.try_send(cognitive_event)`. Agent output, gate results, and completion signals are silently dropped when the channel buffer is full, leaving the editor showing stale/incomplete progress.

11. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` line 1910** — The cognitive event channel at line 1910 is created with capacity 64. The `AcpWorkflowEventConsumer::publish()` at `acp_adapter.rs:164` uses `try_send`, which drops events silently on full buffer.

### Verified concurrency issues

12. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` lines 1657–1662** — The busy check and set are not atomic:
    ```rust
    if session.is_busy() { ... }         // line 1657: LOAD
    session.ensure_provider_runtime(...); // line 1661: gap
    session.begin_prompt();              // line 1662: STORE
    ```
    The `session.cancel()` path (called from notification handlers) sets `busy = false` via `self.busy.store(false, Ordering::Release)` in `session.rs:649`. This can race with the gap above.

13. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs` lines 647–660** — `cancel()` and `begin_prompt()` both assign `self.cancel_token`. If `cancel()` fires between `begin_prompt()` creating the new token (line 654) and the cognitive task capturing it, the new prompt may be immediately cancelled or the old cancellation may be lost.

14. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` line 788** — `assign_acp_experiment()` acquires `EXPERIMENT_STORE_IO_LOCK` (a `std::sync::Mutex`) and reads a JSON file from disk, running on a tokio worker thread. This blocks the tokio worker, violating the async runtime contract. The `record_cascade_observation` function at line 1147 correctly uses `task::spawn_blocking`; the experiment assignment should do the same.

## Implementation Plan

### Fix A1–A2: Replace provider registry expects with error returns

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` lines 1694–1705:

```rust
// Before:
let provider_health = Arc::clone(
    session
        .provider_health_registry
        .as_ref()
        .expect("ACP provider health initialized before prompt"),
);
let provider_rate_limiter = Arc::clone(
    session
        .provider_rate_limiter
        .as_ref()
        .expect("ACP provider rate limiter initialized before prompt"),
);

// After:
let provider_health = session
    .provider_health_registry
    .as_ref()
    .ok_or_else(|| BridgeEventsError::Pipeline(anyhow::anyhow!(
        "provider health registry not initialized before prompt"
    )))?;
let provider_health = Arc::clone(provider_health);

let provider_rate_limiter = session
    .provider_rate_limiter
    .as_ref()
    .ok_or_else(|| BridgeEventsError::Pipeline(anyhow::anyhow!(
        "provider rate limiter not initialized before prompt"
    )))?;
let provider_rate_limiter = Arc::clone(provider_rate_limiter);
```

### Fix A3–A4: Replace stdout/stderr expects in subprocess paths

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` at line 4993 and line 5311, replace each `.expect("stdout was piped")` with:

```rust
// Before:
let stdout = child.stdout.take().expect("stdout was piped");
let stderr = child.stderr.take().expect("stderr was piped");

// After:
let stdout = child.stdout.take().ok_or_else(|| {
    anyhow::anyhow!("subprocess stdout was not piped; check Command::stdout(Stdio::piped())")
})?;
let stderr = child.stderr.take().ok_or_else(|| {
    anyhow::anyhow!("subprocess stderr was not piped; check Command::stderr(Stdio::piped())")
})?;
```

The enclosing functions return `Result<...>` or `anyhow::Result<...>`, so `?` works.

### Fix A5: At-mention char boundary

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` at line 5945:

```rust
// Before:
let ch = text[end..].chars().next().expect("valid char boundary");

// After:
let Some(ch) = text[end..].chars().next() else { break; };
```

### Fix A6: Unique tool name exhaustion

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` at line 3762:

```rust
// Before:
unreachable!("suffix search should always find a unique tool name")

// After:
return Err(BridgeEventsError::Pipeline(anyhow::anyhow!(
    "could not find unique tool name for '{base}' after {MAX_SUFFIX} suffixes"
)));
```

The enclosing function (`make_unique_tool_name` or similar) must also be updated to return `Result<String, BridgeEventsError>`. Update all 1–2 call sites to handle the `Result`.

### Fix A7: Unhandled CognitiveEvent variant

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` at lines 5435–5440:

```rust
// Before:
CognitiveEvent::Complete { .. }
| CognitiveEvent::Failure { .. }
| CognitiveEvent::MaxTokens
| CognitiveEvent::PermissionRequest { .. } => {
    unreachable!("terminal/async cognitive events are handled before update mapping")
}

// After:
CognitiveEvent::Complete { .. }
| CognitiveEvent::Failure { .. }
| CognitiveEvent::MaxTokens
| CognitiveEvent::PermissionRequest { .. } => {
    warn!("unexpected terminal/async cognitive event reached update mapping; skipping");
    return None;  // Caller must handle Option<SessionUpdate>
}
```

Update the enclosing function signature from `-> SessionUpdate` to `-> Option<SessionUpdate>`. Update the call site to skip `None` returns.

### Fix B1: Reviewer failure auto-approves

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs` lines 1840–1845:

```rust
// Before:
Err(e) => {
    warn!(error = %e, "reviewer failed, treating as approved");
    run.pipeline.step(PipelineEvent::ReviewApproved {
        summary: "Review skipped (agent error)".into(),
    })
}

// After:
Err(e) => {
    warn!(error = %e, "reviewer agent failed; treating as revision-required");
    run.pipeline.step(PipelineEvent::ReviewRevise {
        findings: vec![format!(
            "Reviewer agent failed and could not complete the review: {e}. \
             Manual review required before merging."
        )],
    })
}
```

### Fix B2–B3: Multi-role reviewer errors silently approve

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs` at lines 1910–1912 and 1937–1939:

```rust
// Before (architect):
Err(e) => {
    warn!(error = %e, "architect reviewer failed, continuing");
}

// After:
Err(e) => {
    warn!(error = %e, "architect reviewer failed");
    all_approved = false;
    all_findings.push(format!("[architect] agent failed: {e}"));
}

// Before (auditor):
Err(e) => {
    warn!(error = %e, "auditor reviewer failed, continuing");
}

// After:
Err(e) => {
    warn!(error = %e, "auditor reviewer failed");
    all_approved = false;
    all_findings.push(format!("[auditor] agent failed: {e}"));
}
```

### Fix B9: Silent event drops in acp_adapter.rs

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/acp_adapter.rs` line 164:

```rust
// Before:
let _ = self.sender.try_send(cognitive_event);

// After:
if self.sender.try_send(cognitive_event).is_err() {
    warn!("acp_adapter: cognitive event channel full; event dropped");
}
```

Also increase the channel buffer capacity in `bridge_events.rs` at line 1910 from 64 to 256:

```rust
// Before:
let (event_sender, event_receiver) = mpsc::channel(64);

// After:
let (event_sender, event_receiver) = mpsc::channel(256);
```

### Fix C1: TOCTOU busy check

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs`, replace the separate `is_busy()` + `begin_prompt()` pattern with an atomic `try_begin_prompt()`:

```rust
// Add to AcpSession impl:
/// Atomically transition from idle to busy. Returns false if already busy.
pub fn try_begin_prompt(&mut self) -> bool {
    // The &mut self receiver ensures we hold exclusive access to the session
    // from the sequential handler loop. The atomic is still needed because
    // cancel() can be called from a concurrent notification handler path.
    if self.busy.compare_exchange(
        false, true, std::sync::atomic::Ordering::AcqRel,
        std::sync::atomic::Ordering::Acquire,
    ).is_err() {
        return false;
    }
    self.cancel_token = CancelToken::new();
    true
}
```

In `bridge_events.rs` lines 1657–1662, replace:

```rust
// Before:
if session.is_busy() {
    return Err(BridgeEventsError::SessionBusy(session.session_id.clone()));
}
session.ensure_provider_runtime(workdir, roko_config);
session.begin_prompt();

// After:
session.ensure_provider_runtime(workdir, roko_config);
if !session.try_begin_prompt() {
    return Err(BridgeEventsError::SessionBusy(session.session_id.clone()));
}
```

### Fix D1: Blocking I/O on tokio thread in assign_acp_experiment

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs`, `assign_acp_experiment` at line 798 acquires `EXPERIMENT_STORE_IO_LOCK` and reads disk while on a tokio thread. Wrap the entire blocking call in `spawn_blocking`:

```rust
// In the caller (handle_session_prompt_inner, line ~1710):
// Before:
let experiment_assignment = if is_slash_command {
    None
} else {
    assign_acp_experiment(&experiment_path, &session.config_state.agent_mode)
};

// After:
let experiment_assignment = if is_slash_command {
    None
} else {
    let path = experiment_path.clone();
    let mode = session.config_state.agent_mode.clone();
    tokio::task::spawn_blocking(move || assign_acp_experiment(&path, &mode))
        .await
        .unwrap_or(None)
};
```

## Acceptance Criteria

1. All 7 `.expect()` / `unreachable!()` calls in production paths of `bridge_events.rs` are replaced with `Result` returns or `Option`-safe patterns. Verified by: `grep -n '\.expect\|unreachable!' crates/roko-acp/src/bridge_events.rs | grep -v '#\[cfg(test)\]\|test\|//.*safe'` returns zero production hits.

2. `run_multi_role_review` does not auto-approve when any reviewer errors. Verified by: a unit test in `runner.rs` where both reviewer legs return `Err(...)` and the function produces `PipelineEvent::ReviewRevise` with non-empty findings.

3. `try_begin_prompt()` uses `compare_exchange` — the atomic check-and-set is in a single operation. Verified by code review of `session.rs`.

4. The cognitive event channel capacity is 256 (not 64). `acp_adapter.rs:164` logs on drop instead of silently discarding.

5. `assign_acp_experiment` runs inside `spawn_blocking`. Verified by: no `std::sync::Mutex` guard is held across a `.await` point in any async function in `bridge_events.rs`.

6. All 180 existing ACP tests continue to pass: `cargo test -p roko-acp`.

7. `cargo clippy -p roko-acp --no-deps -- -D warnings` passes clean.

## Verification Checklist

- [ ] `cargo test -p roko-acp` — 180 tests pass, zero failures
- [ ] `cargo clippy -p roko-acp --no-deps -- -D warnings` — clean
- [ ] `cargo build -p roko-acp` — zero errors
- [ ] `grep -n '\.expect\|unreachable!' crates/roko-acp/src/bridge_events.rs | grep -v test | grep -v '//'` — no production panics
- [ ] Simulate a reviewer failure in a unit test and verify `ReviewRevise` (not `ReviewApproved`) is emitted
- [ ] Check `bridge_events.rs` for any `std::sync::Mutex` guard held across `.await` — should be zero after D1 fix
- [ ] Run `roko acp` with a test editor session and verify no crashes on malformed input

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` | Fix A1–A7 (expects/unreachables), Fix B9 (event drop), Fix C1 (TOCTOU), Fix D1 (blocking I/O) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs` | Fix B1 (reviewer auto-approve), Fix B2–B3 (multi-role reviewer silent approve) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs` | Add `try_begin_prompt()` with compare_exchange |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/acp_adapter.rs` | Log on try_send failure (Fix B9) |
